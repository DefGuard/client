use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::Path,
    process::Command,
    str::FromStr,
    time::Duration,
};

use chrono::{DateTime, NaiveDateTime, Utc};
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration};
use sqlx::query;
use tauri::{AppHandle, Manager, State};
use tokio::time::sleep;
use tonic::{transport::Channel, Code};
use tracing::Level;

use crate::{
    appstate::AppState,
    commands::{disconnect, LocationInterfaceDetails, Payload},
    database::{
        models::{
            connection::{ActiveConnection, Connection},
            location::Location,
            location_stats::{peer_to_location_stats, LocationStats},
            tunnel::{peer_to_tunnel_stats, Tunnel, TunnelConnection, TunnelStats},
            wireguard_keys::WireguardKeys,
            Id,
        },
        DbPool,
    },
    error::Error,
    events::{DeadConDroppedOutReason, DeadConnDroppedOut, CONNECTION_CHANGED},
    log_watcher::service_log_watcher::spawn_log_watcher_task,
    service::proto::{
        desktop_daemon_service_client::DesktopDaemonServiceClient, CreateInterfaceRequest,
        ReadInterfaceDataRequest, RemoveInterfaceRequest,
    },
    ConnectionType,
};
#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use widestring::U16CString;
#[cfg(target_os = "windows")]
use winapi::{
    shared::{minwindef::DWORD, winerror::ERROR_SERVICE_DOES_NOT_EXIST},
    um::{
        errhandlingapi::GetLastError,
        winsvc::{
            CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_HANDLE__,
            SC_MANAGER_CONNECT, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
        },
    },
};

pub const IS_MACOS: bool = cfg!(target_os = "macos");
static DEFAULT_ROUTE_IPV4: &str = "0.0.0.0/0";
static DEFAULT_ROUTE_IPV6: &str = "::/0";

/// Setup client interface
pub async fn setup_interface(
    location: &Location<Id>,
    interface_name: String,
    preshared_key: Option<String>,
    pool: &DbPool,
    mut client: DesktopDaemonServiceClient<Channel>,
) -> Result<(), Error> {
    debug!("Setting up interface for location: {location}");

    debug!("Looking for wireguard keys for location {location} instance");
    let Some(keys) = WireguardKeys::find_by_instance_id(pool, location.instance_id).await? else {
        error!("No keys found for instance: {}", location.instance_id);
        return Err(Error::InternalError(
            "No keys found for instance".to_string(),
        ));
    };
    debug!("Wireguard keys found for location {location} instance");

    // prepare peer config
    debug!(
        "Decoding location {location} public key: {}.",
        location.pubkey
    );
    let peer_key: Key = Key::from_str(&location.pubkey)?;
    debug!("Location {location} public key decoded: {peer_key}");
    let mut peer = Peer::new(peer_key);

    debug!(
        "Parsing location {location} endpoint: {}",
        location.endpoint
    );
    peer.set_endpoint(&location.endpoint)?;
    peer.persistent_keepalive_interval = Some(25);
    debug!("Parsed location {location} endpoint: {}", location.endpoint);

    if let Some(psk) = preshared_key {
        debug!("Decoding location {location} preshared key.");
        let peer_psk = Key::from_str(&psk)?;
        info!("Location {location} preshared key decoded.");
        peer.preshared_key = Some(peer_psk);
    }

    debug!(
        "Parsing location {location} allowed ips: {}",
        location.allowed_ips
    );
    let allowed_ips = if location.route_all_traffic {
        debug!("Using all traffic routing for location {location}: {DEFAULT_ROUTE_IPV4} {DEFAULT_ROUTE_IPV6}");
        vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
    } else {
        debug!(
            "Using predefined location {location} traffic: {}",
            location.allowed_ips
        );
        location
            .allowed_ips
            .split(',')
            .map(str::to_string)
            .collect()
    };
    for allowed_ip in &allowed_ips {
        match IpAddrMask::from_str(allowed_ip) {
            Ok(addr) => {
                peer.allowed_ips.push(addr);
            }
            Err(err) => {
                // Handle the error from IpAddrMask::from_str, if needed
                error!("Error parsing IP address {allowed_ip} while setting up interface for location {location}, error details: {err}");
                continue;
            }
        }
    }
    debug!(
        "Parsed allowed IPs for location {location}: {:?}",
        peer.allowed_ips
    );

    // request interface configuration
    debug!("Looking for a free port for interface {interface_name}...");
    if let Some(port) = find_random_free_port() {
        debug!("Found free port: {port} for interface {interface_name}.");
        let interface_config = InterfaceConfiguration {
            name: interface_name,
            prvkey: keys.prvkey,
            address: location.address.clone(),
            port: port.into(),
            peers: vec![peer.clone()],
            mtu: None,
        };
        debug!(
            "Creating interface for location {location} with configuration {interface_config:?}"
        );
        let request = CreateInterfaceRequest {
            config: Some(interface_config.clone().into()),
            allowed_ips,
            dns: location.dns.clone(),
        };
        if let Err(error) = client.create_interface(request).await {
            if error.code() == Code::Unavailable {
                error!("Failed to set up connection for location {location}; background service is unavailable. Make sure the service is running. Error: {error}, Interface configuration: {interface_config:?}");
                Err(Error::InternalError(
                    "Background service is unavailable. Make sure the service is running.".into(),
                ))
            } else {
                error!("Failed to send a request to the background service to create an interface for location {location} with the following configuration: {interface_config:?}. Error: {error}");
                Err(Error::InternalError(
                        format!("Failed to send a request to the background service to create an interface for location {location}. Error: {error}. Check logs for details.")
                    ))
            }
        } else {
            info!("The interface for location {location} has been created successfully, interface name: {}.", interface_config.name);
            Ok(())
        }
    } else {
        let msg = format!(
            "Couldn't find free port during interface {interface_name} setup for location {location}"
        );
        error!("{msg}");
        Err(Error::InternalError(msg))
    }
}

fn find_random_free_port() -> Option<u16> {
    const MAX_PORT: u16 = 65535;
    const MIN_PORT: u16 = 6000;

    // Create a TcpListener to check for port availability
    for _ in 0..=(MAX_PORT - MIN_PORT) {
        let port = rand::random::<u16>() % (MAX_PORT - MIN_PORT) + MIN_PORT;
        if is_port_free(port) {
            return Some(port);
        }
    }

    None // No free port found in the specified range
}

#[cfg(target_os = "macos")]
/// Find next available `utun` interface.
#[must_use]
pub fn get_interface_name() -> String {
    let mut index = 0;
    if let Ok(interfaces) = nix::net::if_::if_nameindex() {
        while index < u32::MAX {
            let ifname = format!("utun{index}");
            if interfaces
                .iter()
                .any(|interface| interface.name().to_string_lossy() == ifname)
            {
                index += 1;
            } else {
                return ifname;
            }
        }
    }

    "utun0".into()
}

/// Strips location name of all non-alphanumeric characters returning usable interface name.
#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn get_interface_name(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).collect()
}

fn is_port_free(port: u16) -> bool {
    if let Ok(listener) = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
    {
        // Port is available; close the listener
        drop(listener);
        true
    } else {
        false
    }
}

pub(crate) async fn stats_handler(
    pool: DbPool,
    interface_name: String,
    connection_type: ConnectionType,
    mut client: DesktopDaemonServiceClient<Channel>,
) {
    let request = ReadInterfaceDataRequest {
        interface_name: interface_name.clone(),
    };
    let mut stream = client
        .read_interface_data(request)
        .await
        .expect("Failed to connect to interface stats stream for interface {interface_name}")
        .into_inner();

    loop {
        match stream.message().await {
            Ok(Some(interface_data)) => {
                debug!("Received new network usage statistics for interface {interface_name}.");
                trace!("Received interface data: {interface_data:?}");
                let peers: Vec<Peer> = interface_data.peers.into_iter().map(Into::into).collect();
                for peer in peers {
                    if connection_type.eq(&ConnectionType::Location) {
                        let location_stats =
                            peer_to_location_stats(&peer, interface_data.listen_port, &pool)
                                .await
                                .unwrap();
                        let location_name = location_stats
                            .get_name(&pool)
                            .await
                            .unwrap_or("UNKNOWN".to_string());

                        debug!("Saving network usage stats related to location {location_name} (interface {interface_name}).");
                        trace!("Stats: {location_stats:?}");
                        match location_stats.save(&pool).await {
                            Ok(_) => {
                                debug!("Saved network usage stats for location {location_name}");
                            }
                            Err(err) => {
                                error!(
                                        "Failed to save network usage stats for location {location_name}: {err}"
                                    );
                            }
                        }
                    } else {
                        let tunnel_stats =
                            peer_to_tunnel_stats(&peer, interface_data.listen_port, &pool)
                                .await
                                .unwrap();
                        let tunnel_name = tunnel_stats
                            .get_name(&pool)
                            .await
                            .unwrap_or("UNKNOWN".to_string());
                        debug!("Saving network usage stats related to tunnel {tunnel_name} (interface {interface_name}): {tunnel_stats:?}");
                        match tunnel_stats.save(&pool).await {
                            Ok(_) => {
                                debug!("Saved stats for tunnel {tunnel_name}");
                            }
                            Err(err) => {
                                error!("Failed to save stats for tunnel {tunnel_name}: {err}");
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                debug!("gRPC stream to the defguard-service managing connections has been closed");
                break;
            }
            Err(err) => {
                error!("gRPC stream to the defguard-service managing connections error: {err}");
                break;
            }
        }
    }
    debug!("Network usage stats thread for interface {interface_name} has been terminated");
}

// gets targets that will be allowed by logger, this will be empty if not provided
#[must_use]
pub fn load_log_targets() -> Vec<String> {
    match std::env::var("DEFGUARD_CLIENT_LOG_INCLUDE") {
        Ok(targets) => {
            if !targets.is_empty() {
                return targets
                    .split(',')
                    .filter(|t| !t.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            Vec::new()
        }
        Err(_) => Vec::new(),
    }
}

// helper function to get log file directory for the defguard-service daemon
#[must_use]
pub fn get_service_log_dir() -> &'static Path {
    #[cfg(target_os = "windows")]
    let path = "/Logs/defguard-service";

    #[cfg(not(target_os = "windows"))]
    let path = "/var/log/defguard-service";

    Path::new(path)
}

/// Setup client interface
pub async fn setup_interface_tunnel(
    tunnel: &Tunnel<Id>,
    interface_name: String,
    mut client: DesktopDaemonServiceClient<Channel>,
) -> Result<(), Error> {
    debug!("Setting up interface for tunnel {tunnel}");
    // prepare peer config
    debug!(
        "Decoding tunnel {tunnel} public key: {}.",
        tunnel.server_pubkey
    );
    let peer_key = Key::from_str(&tunnel.server_pubkey)?;
    debug!("Tunnel {tunnel} public key decoded.");
    let mut peer = Peer::new(peer_key);

    debug!("Parsing tunnel {tunnel} endpoint: {}", tunnel.endpoint);
    peer.set_endpoint(&tunnel.endpoint)?;
    peer.persistent_keepalive_interval = Some(
        tunnel
            .persistent_keep_alive
            .try_into()
            .expect("Failed to parse persistent keep alive"),
    );
    debug!("Parsed tunnel {tunnel} endpoint: {}", tunnel.endpoint);

    if let Some(psk) = &tunnel.preshared_key {
        debug!("Decoding tunnel {tunnel} preshared key.");
        let peer_psk = Key::from_str(psk)?;
        debug!("Preshared key for tunnel {tunnel} decoded.");
        peer.preshared_key = Some(peer_psk);
    }

    debug!(
        "Parsing tunnel {tunnel} allowed ips: {:?}",
        tunnel.allowed_ips
    );
    let allowed_ips = if tunnel.route_all_traffic {
        debug!("Using all traffic routing for tunnel {tunnel}: {DEFAULT_ROUTE_IPV4} {DEFAULT_ROUTE_IPV6}");
        vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
    } else {
        let msg = match &tunnel.allowed_ips {
            Some(ips) => format!("Using predefined location traffic for tunnel {tunnel}: {ips}"),
            None => "No allowed IPs found in tunnel {tunnel} configuration".to_string(),
        };
        debug!("{msg}");
        tunnel
            .allowed_ips
            .as_ref()
            .map(|ips| ips.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    };
    for allowed_ip in &allowed_ips {
        match IpAddrMask::from_str(allowed_ip.trim()) {
            Ok(addr) => {
                peer.allowed_ips.push(addr);
            }
            Err(err) => {
                // Handle the error from IpAddrMask::from_str, if needed
                error!("Error parsing IP address {allowed_ip}: {err}");
                // Continue to the next iteration of the loop
                continue;
            }
        }
    }
    debug!("Parsed tunnel {tunnel} allowed IPs: {:?}", peer.allowed_ips);

    // request interface configuration
    debug!("Looking for a free port for interface {interface_name}...");
    if let Some(port) = find_random_free_port() {
        debug!("Found free port: {port} for interface {interface_name}.");
        let interface_config = InterfaceConfiguration {
            name: interface_name,
            prvkey: tunnel.prvkey.clone(),
            address: tunnel.address.clone(),
            port: port.into(),
            peers: vec![peer.clone()],
            mtu: None,
        };
        debug!("Creating interface {interface_config:?}");
        let request = CreateInterfaceRequest {
            config: Some(interface_config.clone().into()),
            allowed_ips,
            dns: tunnel.dns.clone(),
        };
        if let Some(pre_up) = &tunnel.pre_up {
            debug!("Executing defined PreUp command before setting up the interface {} for the tunnel {tunnel}: {pre_up}", interface_config.name);
            let _ = execute_command(pre_up);
            info!(
                "Executed defined PreUp command before setting up the interface {} for the tunnel {tunnel}: {pre_up}", interface_config.name
            );
        }
        if let Err(error) = client.create_interface(request).await {
            error!(
                "Failed to create a network interface ({}) for tunnel {tunnel}: {error}",
                interface_config.name
            );
            Err(Error::InternalError(format!(
                "Failed to create a network interface ({}) for tunnel {tunnel}, error message: {}. Check logs for more details.",
                interface_config.name, error.message()
            )))
        } else {
            info!(
                "Network interface {} for tunnel {tunnel} created successfully.",
                interface_config.name
            );
            if let Some(post_up) = &tunnel.post_up {
                debug!("Executing defined PostUp command after setting up the interface {} for the tunnel {tunnel}: {post_up}", interface_config.name);
                let _ = execute_command(post_up);
                info!("Executed defined PostUp command after setting up the interface {} for the tunnel {tunnel}: {post_up}", interface_config.name);
            }
            debug!(
                "Created interface {} with config: {interface_config:?}",
                interface_config.name
            );
            Ok(())
        }
    } else {
        let msg = format!(
            "Couldn't find free port for interface {interface_name} while setting up tunnel {tunnel}"
        );
        error!("{msg}");
        Err(Error::InternalError(msg))
    }
}

pub async fn get_tunnel_interface_details(
    tunnel_id: Id,
    pool: &DbPool,
) -> Result<LocationInterfaceDetails, Error> {
    debug!("Fetching tunnel details for tunnel ID {tunnel_id}");
    if let Some(tunnel) = Tunnel::find_by_id(pool, tunnel_id).await? {
        debug!("The tunnel with ID {tunnel_id} has been found and identified as {tunnel}.");
        let peer_pubkey = &tunnel.pubkey;

        // generate interface name
        #[cfg(target_os = "macos")]
        let interface_name = get_interface_name();
        #[cfg(not(target_os = "macos"))]
        let interface_name = get_interface_name(&tunnel.name);

        debug!("Fetching tunnel stats for tunnel ID {tunnel_id}");
        let result = query!(
            "SELECT last_handshake, listen_port \"listen_port!: u32\", \
            persistent_keepalive_interval \"persistent_keepalive_interval?: u16\" \
            FROM tunnel_stats WHERE tunnel_id = $1 ORDER BY collected_at DESC LIMIT 1",
            tunnel_id
        )
        .fetch_optional(pool)
        .await?;
        debug!("Fetched tunnel connection statistics for tunnel {tunnel}");

        let (listen_port, persistent_keepalive_interval, last_handshake) = match result {
            Some(record) => (
                Some(record.listen_port),
                record.persistent_keepalive_interval,
                Some(record.last_handshake),
            ),
            None => (None, None, None),
        };

        debug!("Fetched tunnel configuration details for tunnel {tunnel}.");

        Ok(LocationInterfaceDetails {
            location_id: tunnel_id,
            name: interface_name,
            pubkey: tunnel.server_pubkey,
            address: tunnel.address,
            dns: tunnel.dns,
            listen_port,
            peer_pubkey: peer_pubkey.to_string(),
            peer_endpoint: tunnel.endpoint,
            allowed_ips: tunnel.allowed_ips.unwrap_or_default(),
            persistent_keepalive_interval,
            last_handshake,
        })
    } else {
        error!("Error while fetching tunnel details for ID {tunnel_id}: tunnel not found");
        Err(Error::NotFound)
    }
}

pub async fn get_location_interface_details(
    location_id: Id,
    pool: &DbPool,
) -> Result<LocationInterfaceDetails, Error> {
    debug!("Fetching location details for location ID {location_id}");
    if let Some(location) = Location::find_by_id(pool, location_id).await? {
        debug!("Fetching WireGuard keys for location {}", location.name);
        let keys = WireguardKeys::find_by_instance_id(pool, location.instance_id)
            .await?
            .ok_or(Error::NotFound)?;
        debug!(
            "Successfully fetched WireGuard keys for location {}",
            location.name
        );
        let peer_pubkey = keys.pubkey;

        // generate interface name
        #[cfg(target_os = "macos")]
        let interface_name = get_interface_name();
        #[cfg(not(target_os = "macos"))]
        let interface_name = get_interface_name(&location.name);

        debug!("Fetching location stats for location ID {location_id}");
        let result = query!(
            "SELECT last_handshake, listen_port \"listen_port!: u32\", \
            persistent_keepalive_interval \"persistent_keepalive_interval?: u16\" \
            FROM location_stats \
            WHERE location_id = $1 ORDER BY collected_at DESC LIMIT 1",
            location_id
        )
        .fetch_optional(pool)
        .await?;
        debug!("Fetched location stats for location ID {location_id}");

        let (listen_port, persistent_keepalive_interval, last_handshake) = match result {
            Some(record) => (
                Some(record.listen_port),
                record.persistent_keepalive_interval,
                Some(record.last_handshake),
            ),
            None => (None, None, None),
        };

        debug!("Fetched location details for location ID {location_id}");

        Ok(LocationInterfaceDetails {
            location_id,
            name: interface_name,
            pubkey: location.pubkey,
            address: location.address,
            dns: location.dns,
            listen_port,
            peer_pubkey,
            peer_endpoint: location.endpoint,
            allowed_ips: location.allowed_ips,
            persistent_keepalive_interval,
            last_handshake,
        })
    } else {
        error!("Error while fetching location details for ID {location_id}: location not found");
        Err(Error::NotFound)
    }
}

/// Setup new connection for location
pub(crate) async fn handle_connection_for_location(
    location: &Location<Id>,
    preshared_key: Option<String>,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Setting up the connection for location {}", location.name);
    let state = handle.state::<AppState>();
    #[cfg(target_os = "macos")]
    let interface_name = get_interface_name();
    #[cfg(not(target_os = "macos"))]
    let interface_name = get_interface_name(&location.name);
    setup_interface(
        location,
        interface_name.clone(),
        preshared_key,
        &state.db,
        state.client.clone(),
    )
    .await?;
    state
        .add_connection(location.id, &interface_name, ConnectionType::Location)
        .await;

    debug!("Sending event informing the frontend that a new connection has been created.");
    handle.emit_all(
        CONNECTION_CHANGED,
        Payload {
            message: "Created new connection".into(),
        },
    )?;
    debug!("Event informing the frontend that a new connection has been created sent.");

    // spawn log watcher
    debug!("Spawning service log watcher for location {location}...");
    spawn_log_watcher_task(
        handle,
        location.id,
        interface_name,
        ConnectionType::Location,
        Level::DEBUG,
        None,
    )
    .await?;
    debug!("Service log watcher for location {location} spawned.");
    Ok(())
}

/// Setup new connection for tunnel
pub(crate) async fn handle_connection_for_tunnel(
    tunnel: &Tunnel<Id>,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Setting up the connection for tunnel: {}", tunnel.name);
    let state = handle.state::<AppState>();
    #[cfg(target_os = "macos")]
    let interface_name = get_interface_name();
    #[cfg(not(target_os = "macos"))]
    let interface_name = get_interface_name(&tunnel.name);
    setup_interface_tunnel(tunnel, interface_name.clone(), state.client.clone()).await?;
    state
        .add_connection(tunnel.id, &interface_name, ConnectionType::Tunnel)
        .await;

    debug!("Sending event informing the frontend that a new connection has been created.");
    handle.emit_all(
        CONNECTION_CHANGED,
        Payload {
            message: "Created new connection".into(),
        },
    )?;
    debug!("Event informing the frontend that a new connection has been created sent.");

    // spawn log watcher
    debug!("Spawning log watcher for tunnel {}", tunnel.name);
    spawn_log_watcher_task(
        handle,
        tunnel.id,
        interface_name,
        ConnectionType::Tunnel,
        Level::DEBUG,
        None,
    )
    .await?;
    debug!("Log watcher for tunnel {} spawned", tunnel.name);
    Ok(())
}

/// Execute command passed as argument.
pub fn execute_command(command: &str) -> Result<(), Error> {
    debug!("Executing command: {command}");
    let mut command_parts = command.split_whitespace();

    if let Some(command) = command_parts.next() {
        let output = Command::new(command).args(command_parts).output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            debug!(
                "Command {command} executed successfully. Stdout: {}",
                stdout
            );
            if !stderr.is_empty() {
                error!("Command produced the following output on stderr: {stderr}");
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Error while executing command: {command}. Stderr: {stderr}");
        }
    }
    Ok(())
}

/// Helper function to remove interface and close connection
pub(crate) async fn disconnect_interface(
    active_connection: &ActiveConnection,
    state: &AppState,
) -> Result<(), Error> {
    debug!(
        "Disconnecting interface {}...",
        active_connection.interface_name
    );
    let mut client = state.client.clone();
    let location_id = active_connection.location_id;
    let interface_name = active_connection.interface_name.clone();

    match active_connection.connection_type {
        ConnectionType::Location => {
            let Some(location) = Location::find_by_id(&state.db, location_id).await? else {
                error!("Error while disconnecting interface {interface_name}, location with ID {location_id} not found");
                return Err(Error::NotFound);
            };
            let request = RemoveInterfaceRequest {
                interface_name,
                endpoint: location.endpoint.clone(),
            };
            debug!("Sending request to the background service to remove interface {} for location {}...", active_connection.interface_name, location.name);
            if let Err(error) = client.remove_interface(request).await {
                let msg = if error.code() == Code::Unavailable {
                    format!("Couldn't remove interface {}. Background service is unavailable. Please make sure the service is running. Error: {error}.", active_connection.interface_name)
                } else {
                    format!("Failed to send a request to the background service to remove interface {}. Error: {error}.", active_connection.interface_name)
                };
                error!("{msg}");
                return Err(Error::InternalError(msg));
            }
            let connection: Connection = active_connection.into();
            let connection = connection.save(&state.db).await?;
            debug!(
                "Saved location {} new connection status in the database",
                location.name
            );
            trace!("Saved connection: {connection:?}");
            info!(
                "Network interface {} for location {location} has been removed",
                active_connection.interface_name
            );
            debug!("Finished disconnecting from location {}", location.name);
        }
        ConnectionType::Tunnel => {
            let Some(tunnel) = Tunnel::find_by_id(&state.db, location_id).await? else {
                error!("Error while disconnecting interface {interface_name}, tunnel with ID {location_id} not found");
                return Err(Error::NotFound);
            };
            if let Some(pre_down) = &tunnel.pre_down {
                debug!("Executing defined PreDown command before setting up the interface {} for the tunnel {tunnel}: {pre_down}", active_connection.interface_name);
                let _ = execute_command(pre_down);
                info!("Executed defined PreDown command before setting up the interface {} for the tunnel {tunnel}: {pre_down}", active_connection.interface_name);
            }
            let request = RemoveInterfaceRequest {
                interface_name,
                endpoint: tunnel.endpoint.clone(),
            };
            if let Err(error) = client.remove_interface(request).await {
                error!(
                    "Error while removing interface {}, error details: {:?}",
                    active_connection.interface_name, error
                );
                return Err(Error::InternalError(format!(
                    "Failed to remove interface, error message: {}",
                    error.message()
                )));
            }
            if let Some(post_down) = &tunnel.post_down {
                debug!("Executing defined PostDown command after removing the interface {} for the tunnel {tunnel}: {post_down}", active_connection.interface_name);
                let _ = execute_command(post_down);
                info!("Executed defined PostDown command after removing the interface {} for the tunnel {tunnel}: {post_down}", active_connection.interface_name);
            }
            let connection: TunnelConnection = active_connection.into();
            let connection = connection.save(&state.db).await?;
            debug!(
                "Saved new tunnel {} connection status in the database",
                tunnel.name
            );
            trace!("Saved connection: {connection:#?}");
            info!(
                "Network interface {} for tunnel {tunnel} has been removed",
                active_connection.interface_name
            );
            debug!("Finished disconnecting from tunnel {}", tunnel.name);
        }
    }

    Ok(())
}

/// Helper function to get the name of a tunnel or location by its ID
/// Returns the name of the tunnel or location if it exists, otherwise "UNKNOWN"
/// This is for logging purposes.
pub async fn get_tunnel_or_location_name(
    id: Id,
    connection_type: ConnectionType,
    appstate: &AppState,
) -> String {
    let name = match connection_type {
        ConnectionType::Location => Location::find_by_id(&appstate.db, id)
            .await
            .ok()
            .and_then(|l| l.map(|l| l.name)),
        ConnectionType::Tunnel => Tunnel::find_by_id(&appstate.db, id)
            .await
            .ok()
            .and_then(|t| t.map(|t| t.name)),
    };

    if let Some(name) = name {
        name
    } else {
        debug!(
                "Couldn't identify {connection_type}'s name for logging purposes, it will be referred to as UNKNOWN",
            );
        "UNKNOWN".to_string()
    }
}

#[cfg(target_os = "windows")]
fn open_service_manager() -> Result<*mut SC_HANDLE__, DWORD> {
    let sc_manager_handle = unsafe { OpenSCManagerW(null_mut(), null_mut(), SC_MANAGER_CONNECT) };
    if sc_manager_handle.is_null() {
        Err(unsafe { GetLastError() })
    } else {
        Ok(sc_manager_handle)
    }
}

#[cfg(target_os = "windows")]
fn open_service(
    sc_manager_handle: *mut SC_HANDLE__,
    service_name: &str,
    desired_access: DWORD,
) -> Result<*mut SC_HANDLE__, DWORD> {
    let service_name_wstr = match U16CString::from_str(service_name) {
        Ok(service_name_wstr) => service_name_wstr,
        Err(err) => {
            error!(
                "Failed to convert service name {} to a wide string: {err}",
                service_name
            );
            return Err(1);
        }
    };
    let service_handle = unsafe {
        OpenServiceW(
            sc_manager_handle,
            service_name_wstr.as_ptr(),
            desired_access,
        )
    };
    if service_handle.is_null() {
        Err(unsafe { GetLastError() })
    } else {
        Ok(service_handle)
    }
}

#[cfg(target_os = "windows")]
fn get_service_status(service_handle: *mut SC_HANDLE__) -> Result<DWORD, DWORD> {
    let mut service_status = unsafe { std::mem::zeroed() };
    let result = unsafe { QueryServiceStatus(service_handle, &mut service_status) };
    if result == 0 {
        Err(unsafe { GetLastError() })
    } else {
        Ok(service_status.dwCurrentState)
    }
}

#[cfg(target_os = "windows")]
fn close_service_handle(
    service_handle: *mut SC_HANDLE__,
    service_name: &str,
) -> Result<i32, Error> {
    let result = unsafe { CloseServiceHandle(service_handle) };
    if result == 0 {
        let error = unsafe { GetLastError() };
        Err(Error::InternalError(format!(
            "Failed to close service handle for service {service_name}, error code: {error}",
        )))
    } else {
        info!("Service handle closed successfully");
        Ok(result)
    }
}

// TODO: Move the connection handling to a seperate, common function,
// so `handle_connection_for_location` and `handle_connection_for_tunnel` are not
// partially duplicated here.
#[cfg(target_os = "windows")]
pub(crate) async fn sync_connections(app_handle: &AppHandle) -> Result<(), Error> {
    debug!("Synchronizing active connections with the systems' state...");
    let appstate = app_handle.state::<AppState>();
    let all_locations = Location::all(&appstate.db).await?;
    let service_control_manager = open_service_manager().map_err(|err| {
        error!("Failed to open service control manager while trying to sync client's connections with the host state: {}", err);
        Error::InternalError("Failed to open service control manager while trying to sync client's connections with the host state".to_string())
    })?;

    debug!("Opened service control manager, starting to synchronize active connections for locations...");
    // Go through all locations and check if they are connected (if the windows service is running)
    // If we encounter any errors, continue with the next iteration of the loop, it's not a big deal
    // if we skip some locations, as the user can always reconnect to them manually
    for location in all_locations {
        let interface_name = get_interface_name(&location.name);
        let service_name = format!("WireGuardTunnel${}", interface_name);
        let service = match open_service(
            service_control_manager,
            &service_name,
            SERVICE_QUERY_STATUS,
        ) {
            Ok(service) => service,
            Err(err) => match err {
                ERROR_SERVICE_DOES_NOT_EXIST => {
                    debug!(
                        "WireGuard tunnel {} is not installed, nothing to synchronize",
                        interface_name
                    );
                    continue;
                }
                _ => {
                    warn!(
                            "Failed to open service {service_name} for interface {interface_name} while synchronizing active connections. \
                            This may cause the location {} state to display incorrectly in the client. Reconnect to it manually to fix it. Error: {err}", location.name
                        );
                    continue;
                }
            },
        };
        match get_service_status(service) {
            Ok(status) => {
                // Only point where we don't jump to the next iteration of the loop and continue with the rest of the code below the match
                close_service_handle(service, &service_name)?;
                if status == SERVICE_RUNNING {
                    debug!("WireGuard tunnel {} is running, ", interface_name);
                } else {
                    debug!(
                        "WireGuard tunnel {} is not running, status code: {status}. Refer to Windows documentation for more information about the code.",
                        interface_name
                    );
                    continue;
                }
            }
            Err(err) => {
                close_service_handle(service, &service_name)?;
                warn!(
                    "Failed to query service status for interface {} while synchronizing active connections. \
                    This may cause the location {} state to display incorrectly in the client. Reconnect to it manually to fix it. Error: {err}",
                    interface_name, location.name
                );
                continue;
            }
        }

        if appstate
            .find_connection(location.id, ConnectionType::Location)
            .await
            .is_some()
        {
            debug!(
                "Location {} has already a connected state, skipping synchronization",
                location.name
            );
            continue;
        }

        appstate.add_connection(location.id, &interface_name, ConnectionType::Location);

        debug!("Sending event informing the frontend that a new connection has been created.");
        app_handle.emit_all(
            CONNECTION_CHANGED,
            Payload {
                message: "Created new connection".into(),
            },
        )?;
        debug!("Event informing the frontend that a new connection has been created sent.");

        debug!("Spawning service log watcher for location {}...", location);
        spawn_log_watcher_task(
            app_handle.clone(),
            location.id,
            interface_name,
            ConnectionType::Location,
            Level::DEBUG,
            None,
        )
        .await?;
        debug!("Service log watcher for location {} spawned.", location);
    }

    debug!("Synchronizing active connections for tunnels...");
    // Do the same for tunnels
    for tunnel in Tunnel::all(&appstate.db).await? {
        let interface_name = get_interface_name(&tunnel.name);
        let service_name = format!("WireGuardTunnel${}", interface_name);
        let service = match open_service(
            service_control_manager,
            &service_name,
            SERVICE_QUERY_STATUS,
        ) {
            Ok(service) => service,
            Err(err) => match err {
                ERROR_SERVICE_DOES_NOT_EXIST => {
                    debug!(
                        "WireGuard tunnel {} is not installed, nothing to synchronize",
                        interface_name
                    );
                    continue;
                }
                _ => {
                    error!(
                            "Failed to open service {service_name} for interface {interface_name}. \
                            This may cause the tunnel {} state to display incorrectly in the client. Reconnect to it manually to fix it. Error: {err}", tunnel.name
                        );
                    continue;
                }
            },
        };
        match get_service_status(service) {
            Ok(status) => {
                // Only point where we don't jump to the next iteration of the loop and continue with the rest of the code below the match
                close_service_handle(service, &service_name)?;
                if status == SERVICE_RUNNING {
                    debug!("WireGuard tunnel {} is running", interface_name);
                } else {
                    debug!(
                        "WireGuard tunnel {} is not running, status code: {status}. Refer to Windows documentation for more information about the code.",
                        interface_name
                    );
                    continue;
                }
            }
            Err(err) => {
                close_service_handle(service, &service_name)?;
                warn!(
                    "Failed to query service status for interface {}. \
                    This may cause the tunnel {} state to display incorrectly in the client. Reconnect to it manually to fix it. Error: {err}",
                    interface_name, tunnel.name
                );
                continue;
            }
        }

        if appstate
            .find_connection(tunnel.id, ConnectionType::Tunnel)
            .await
            .is_some()
        {
            debug!(
                "Tunnel {} has already a connected state, skipping synchronization",
                tunnel.name
            );
            continue;
        }

        appstate.add_connection(tunnel.id, &interface_name, ConnectionType::Tunnel);

        debug!("Sending event informing the frontend that a new connection has been created.");
        app_handle.emit_all(
            CONNECTION_CHANGED,
            Payload {
                message: "Created new connection".into(),
            },
        )?;
        debug!("Event informing the frontend that a new connection has been created sent.");

        //spawn log watcher
        debug!("Spawning log watcher for tunnel {}", tunnel.name);
        spawn_log_watcher_task(
            app_handle.clone(),
            tunnel.id,
            interface_name,
            ConnectionType::Tunnel,
            Level::DEBUG,
            None,
        )
        .await?;
        debug!("Log watcher for tunnel {} spawned", tunnel.name);
    }

    close_service_handle(service_control_manager, "SERVICE_CONTROL_MANAGER")?;

    debug!("Active connections synchronized with the system state");

    Ok(())
}

pub(crate) enum ConnectionToVerify {
    Location(Location<Id>),
    Tunnel(Tunnel<Id>),
}

#[must_use]
fn is_connection_alive(connection_start: NaiveDateTime, last_activity: NaiveDateTime) -> bool {
    let start = DateTime::<Utc>::from_naive_utc_and_offset(connection_start, Utc);
    let activity = DateTime::<Utc>::from_naive_utc_and_offset(last_activity, Utc);
    let result = activity >= start;
    trace!(
        "Check for connection, start: {start}, last activity: {activity}, check result: {result}"
    );
    result
}

/// Verify if made connection is actually alive after being optimistically connected.
/// This works by checking if any activity was made after connecting, within specified time window.
// TODO: put the verification time into UI Settings
pub(crate) async fn verify_connection(app_handle: AppHandle, connection: ConnectionToVerify) {
    let state: State<AppState> = app_handle.state();
    let wait_time = Duration::from_secs(
        state
            .app_config
            .lock()
            .unwrap()
            .connection_verification_time
            .into(),
    );
    debug!("Connection verification task is sleeping for {wait_time:?}");
    sleep(wait_time).await;
    debug!("Connection verification task finished sleeping");
    let db_pool = &state.db;
    let active_connections = state.active_connections.lock().await;

    match connection {
        ConnectionToVerify::Location(location) => {
            match active_connections.iter().find(|&x| {
                x.location_id == location.id && x.connection_type == ConnectionType::Location
            }) {
                Some(active_connection) => {
                    debug!("Verifying connection to location {location}");
                    trace!("Verifying connection {active_connection:?}");
                    let payload = DeadConnDroppedOut {
                        con_type: ConnectionType::Location,
                        name: location.name.to_string(),
                        reason: DeadConDroppedOutReason::ConnectionVerification,
                    };
                    let connection_start = active_connection.start;
                    drop(active_connections); // release Mutex lock

                    match LocationStats::latest_by_location_id(db_pool, location.id).await {
                        Ok(Some(latest_stat)) => {
                            if is_connection_alive(connection_start, latest_stat.collected_at) {
                                info!("Active connection for location {location} verified successfully.");
                            } else {
                                info!("Location {location} will be disconnected due to lack of activity within {wait_time:?}.");
                                match disconnect(
                                    location.id,
                                    ConnectionType::Location,
                                    app_handle.clone(),
                                )
                                .await
                                {
                                    Ok(()) => {
                                        payload.emit(&app_handle);
                                    }
                                    Err(err) => {
                                        error!(
                                            "Failed to disconnect location {location}. Error: {err}"
                                        );
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            info!("Location {location} will be disconnected due to lack of statistics within {wait_time:?}.");
                            match disconnect(
                                location.id,
                                ConnectionType::Location,
                                app_handle.clone(),
                            )
                            .await
                            {
                                Ok(()) => {
                                    payload.emit(&app_handle);
                                }
                                Err(err) => {
                                    error!(
                                        "Failed to disconnect location {location}. Error: {err}"
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            error!("Connection verification for location {location} Failed during retrieval of stats. Error: {err}");
                        }
                    }
                }
                None => {
                    error!("Connection verification for location {location} failed. Active connection in appstate not found.");
                }
            }
        }
        ConnectionToVerify::Tunnel(tunnel) => {
            debug!("Verifying connection to tunnel {tunnel}");
            match active_connections.iter().find(|&x| {
                x.location_id == tunnel.id && x.connection_type == ConnectionType::Tunnel
            }) {
                Some(active_connection) => {
                    trace!("Verifying connection {active_connection:?}");
                    let payload = DeadConnDroppedOut {
                        con_type: ConnectionType::Tunnel,
                        name: tunnel.name.to_string(),
                        reason: DeadConDroppedOutReason::ConnectionVerification,
                    };
                    let connection_start = active_connection.start;
                    drop(active_connections); // release Mutex lock

                    match TunnelStats::latest_by_tunnel_id(db_pool, tunnel.id).await {
                        Ok(Some(latest_stat)) => {
                            if is_connection_alive(connection_start, latest_stat.collected_at) {
                                info!("Tunnel {tunnel} connection verified successfully.");
                            } else {
                                info!("Tunnel {tunnel} will be disconnected due to lack of activity within specified time.");
                                match disconnect(
                                    tunnel.id,
                                    ConnectionType::Tunnel,
                                    app_handle.clone(),
                                )
                                .await
                                {
                                    Ok(()) => {
                                        payload.emit(&app_handle);
                                    }
                                    Err(err) => {
                                        error!("Connection for tunnel {tunnel} could not be disconnected. Reason: {err}");
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            info!("Tunnel {tunnel} will be disconnected due to lack of stats in specified time.");
                            match disconnect(tunnel.id, ConnectionType::Tunnel, app_handle.clone())
                                .await
                            {
                                Ok(()) => {
                                    payload.emit(&app_handle);
                                }
                                Err(err) => {
                                    error!("Connection for tunnel {tunnel} could not be disconnected. Reason: {err}");
                                }
                            }
                        }
                        Err(err) => {
                            error!("Connection verification for tunnel {tunnel} failed during retrieval of stats. Reason: {err}");
                        }
                    }
                }
                None => {
                    error!("Connection verification for tunnel {tunnel} failed. Active connection not found.");
                }
            }
        }
    }
}
