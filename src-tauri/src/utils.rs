use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    process::Command,
    str::FromStr,
};
use tauri::AppHandle;

use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration};
use sqlx::query;
use tauri::Manager;
use tonic::{codegen::tokio_stream::StreamExt, transport::Channel};

use crate::{
    appstate::AppState,
    commands::{LocationInterfaceDetails, Payload},
    database::{
        models::location::peer_to_location_stats, models::tunnel::peer_to_tunnel_stats,
        ActiveConnection, Connection, DbPool, Location, Tunnel, TunnelConnection, WireguardKeys,
    },
    error::Error,
    service::{
        log_watcher::spawn_log_watcher_task,
        proto::{
            desktop_daemon_service_client::DesktopDaemonServiceClient, CreateInterfaceRequest,
            ReadInterfaceDataRequest, RemoveInterfaceRequest,
        },
    },
    ConnectionType,
};
use local_ip_address::local_ip;
use tracing::Level;

pub static IS_MACOS: bool = cfg!(target_os = "macos");
pub static STATS_PERIOD: u64 = 60;
pub static DEFAULT_ROUTE: &str = "0.0.0.0/0";

/// Setup client interface
pub async fn setup_interface(
    location: &Location,
    interface_name: String,
    preshared_key: Option<String>,
    pool: &DbPool,
    mut client: DesktopDaemonServiceClient<Channel>,
) -> Result<(), Error> {
    if let Some(keys) = WireguardKeys::find_by_instance_id(pool, location.instance_id).await? {
        // prepare peer config
        debug!("Decoding location public key: {}.", location.pubkey);
        let peer_key: Key = Key::from_str(&location.pubkey)?;
        let mut peer = Peer::new(peer_key);

        debug!("Parsing location endpoint: {}", location.endpoint);
        let endpoint: SocketAddr = location.endpoint.parse()?;
        peer.endpoint = Some(endpoint);
        peer.persistent_keepalive_interval = Some(25);

        if let Some(psk) = preshared_key {
            let peer_psk = Key::from_str(&psk)?;
            peer.preshared_key = Some(peer_psk);
        }

        debug!("Parsing location allowed ips: {}", location.allowed_ips);
        let allowed_ips: Vec<String> = if location.route_all_traffic {
            debug!("Using all traffic routing: {DEFAULT_ROUTE}");
            vec![DEFAULT_ROUTE.into()]
        } else {
            debug!("Using predefined location traffic");
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
                    error!("Error parsing IP address {allowed_ip}: {err}");
                    // Continue to the next iteration of the loop
                    continue;
                }
            }
        }

        // request interface configuration
        if let Some(port) = find_random_free_port() {
            let interface_config = InterfaceConfiguration {
                name: interface_name,
                prvkey: keys.prvkey,
                address: location.address.clone(),
                port: port.into(),
                peers: vec![peer.clone()],
            };
            debug!("Creating interface {interface_config:#?}");
            let request = CreateInterfaceRequest {
                config: Some(interface_config.clone().into()),
                allowed_ips,
                dns: location.dns.clone(),
                pre_up: None,
                post_up: None,
            };
            if let Err(error) = client.create_interface(request).await {
                error!("Failed to create interface: {error}");
                Err(Error::InternalError)
            } else {
                info!("Created interface {interface_config:#?}");
                Ok(())
            }
        } else {
            error!("Error finding free port");
            Err(Error::InternalError)
        }
    } else {
        error!("No keys found for instance: {}", location.instance_id);
        Err(Error::InternalError)
    }
}

/// Helper function to remove whitespace from location name
#[must_use]
pub fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
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

#[cfg(not(target_os = "macos"))]
/// Returns interface name for location
#[must_use]
pub fn get_interface_name(name: &str) -> String {
    remove_whitespace(name)
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

pub async fn spawn_stats_thread(
    handle: tauri::AppHandle,
    interface_name: String,
    connection_type: ConnectionType,
) {
    tokio::spawn(async move {
        let state = handle.state::<AppState>();
        let mut client = state.client.clone();
        let request = ReadInterfaceDataRequest {
            interface_name: interface_name.clone(),
        };
        let mut stream = client
            .read_interface_data(request)
            .await
            .expect("Failed to connect to interface stats stream")
            .into_inner();

        while let Some(item) = stream.next().await {
            match item {
                Ok(interface_data) => {
                    debug!("Received interface data update: {interface_data:?}");
                    let peers: Vec<Peer> =
                        interface_data.peers.into_iter().map(Into::into).collect();
                    for peer in peers {
                        if connection_type.eq(&ConnectionType::Location) {
                            let mut location_stats = peer_to_location_stats(
                                &peer,
                                interface_data.listen_port,
                                &state.get_pool(),
                            )
                            .await
                            .unwrap();
                            debug!("Saving location stats: {location_stats:#?}");
                            let _ = location_stats.save(&state.get_pool()).await;
                            debug!("Saved location stats: {location_stats:#?}");
                        } else {
                            let mut tunnel_stats = peer_to_tunnel_stats(
                                &peer,
                                interface_data.listen_port,
                                &state.get_pool(),
                            )
                            .await
                            .unwrap();
                            debug!("Saving tunnel stats: {tunnel_stats:#?}");
                            let _ = tunnel_stats.save(&state.get_pool()).await;
                            debug!("Saved location stats: {tunnel_stats:#?}");
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to receive interface data update: {err}");
                }
            }
        }
        warn!("Interface data stream disconnected");
    });
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
pub fn get_service_log_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    let path = PathBuf::from("/Logs/defguard-service");

    #[cfg(not(target_os = "windows"))]
    let path = PathBuf::from("/var/log/defguard-service");

    path
}
/// Setup client interface
pub async fn setup_interface_tunnel(
    tunnel: &Tunnel,
    interface_name: String,
    mut client: DesktopDaemonServiceClient<Channel>,
) -> Result<(), Error> {
    // prepare peer config
    debug!("Decoding location public key: {}.", tunnel.server_pubkey);
    let peer_key: Key = Key::from_str(&tunnel.server_pubkey)?;
    let mut peer = Peer::new(peer_key);

    debug!("Parsing location endpoint: {}", tunnel.endpoint);
    let endpoint: SocketAddr = tunnel.endpoint.parse()?;
    peer.endpoint = Some(endpoint);
    peer.persistent_keepalive_interval = Some(
        tunnel
            .persistent_keep_alive
            .try_into()
            .expect("Failed to parse persistent keep alive"),
    );

    if let Some(psk) = &tunnel.preshared_key {
        let peer_psk = Key::from_str(psk)?;
        peer.preshared_key = Some(peer_psk);
    }

    debug!("Parsing location allowed ips: {:?}", tunnel.allowed_ips);
    let allowed_ips: Vec<String> = if tunnel.route_all_traffic {
        debug!("Using all traffic routing: {DEFAULT_ROUTE}");
        vec![DEFAULT_ROUTE.into()]
    } else {
        debug!("Using predefined location traffic");
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

    // request interface configuration
    if let Some(port) = find_random_free_port() {
        let interface_config = InterfaceConfiguration {
            name: interface_name,
            prvkey: tunnel.prvkey.clone(),
            address: tunnel.address.clone(),
            port: port.into(),
            peers: vec![peer.clone()],
        };
        debug!("Creating interface {interface_config:#?}");
        let request = CreateInterfaceRequest {
            config: Some(interface_config.clone().into()),
            allowed_ips,
            dns: tunnel.dns.clone(),
            pre_up: tunnel.pre_up.clone(),
            post_up: tunnel.post_up.clone(),
        };
        if let Err(error) = client.create_interface(request).await {
            error!("Failed to create interface: {error}");
            Err(Error::InternalError)
        } else {
            info!("Created interface {interface_config:#?}");
            Ok(())
        }
    } else {
        error!("Error finding free port");
        Err(Error::InternalError)
    }
}

pub async fn get_tunnel_interface_details(
    tunnel_id: i64,
    pool: &DbPool,
) -> Result<LocationInterfaceDetails, Error> {
    debug!("Fetching tunnel details for tunnel ID {tunnel_id}");
    if let Some(tunnel) = Tunnel::find_by_id(pool, tunnel_id).await? {
        debug!("Fetching WireGuard keys for location {}", tunnel.name);
        let peer_pubkey = tunnel.pubkey;

        // generate interface name
        #[cfg(target_os = "macos")]
        let interface_name = get_interface_name();
        #[cfg(not(target_os = "macos"))]
        let interface_name = get_interface_name(&tunnel.name);

        let result = query!(
            r#"
            SELECT last_handshake, listen_port as "listen_port!: u32",
              persistent_keepalive_interval as "persistent_keepalive_interval?: u16"
            FROM tunnel_stats
            WHERE tunnel_id = $1 ORDER BY collected_at DESC LIMIT 1
            "#,
            tunnel_id
        )
        .fetch_optional(pool)
        .await?;

        let (listen_port, persistent_keepalive_interval, last_handshake) = match result {
            Some(record) => (
                Some(record.listen_port),
                record.persistent_keepalive_interval,
                Some(record.last_handshake),
            ),
            None => (None, None, None),
        };

        Ok(LocationInterfaceDetails {
            location_id: tunnel_id,
            name: interface_name,
            pubkey: tunnel.server_pubkey,
            address: tunnel.address,
            dns: tunnel.dns,
            listen_port,
            peer_pubkey,
            peer_endpoint: tunnel.endpoint,
            allowed_ips: tunnel.allowed_ips.unwrap_or_default(),
            persistent_keepalive_interval,
            last_handshake,
        })
    } else {
        error!("Tunnel ID {tunnel_id} not found");
        Err(Error::NotFound)
    }
}
pub async fn get_location_interface_details(
    location_id: i64,
    pool: &DbPool,
) -> Result<LocationInterfaceDetails, Error> {
    debug!("Fetching location details for location ID {location_id}");
    if let Some(location) = Location::find_by_id(pool, location_id).await? {
        debug!("Fetching WireGuard keys for location {}", location.name);
        let keys = WireguardKeys::find_by_instance_id(pool, location.instance_id)
            .await?
            .ok_or(Error::NotFound)?;
        let peer_pubkey = keys.pubkey;

        // generate interface name
        #[cfg(target_os = "macos")]
        let interface_name = get_interface_name();
        #[cfg(not(target_os = "macos"))]
        let interface_name = get_interface_name(&location.name);

        let result = query!(
            r#"
            SELECT last_handshake, listen_port as "listen_port!: u32",
              persistent_keepalive_interval as "persistent_keepalive_interval?: u16"
            FROM location_stats
            WHERE location_id = $1 ORDER BY collected_at DESC LIMIT 1
            "#,
            location_id
        )
        .fetch_optional(pool)
        .await?;

        let (listen_port, persistent_keepalive_interval, last_handshake) = match result {
            Some(record) => (
                Some(record.listen_port),
                record.persistent_keepalive_interval,
                Some(record.last_handshake),
            ),
            None => (None, None, None),
        };

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
        error!("Location ID {location_id} not found");
        Err(Error::NotFound)
    }
}

/// Setup new connection for location
pub async fn handle_connection_for_location(
    location: &Location,
    preshared_key: Option<String>,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!(
        "Creating new interface connection for location: {}",
        location.name
    );
    let state = handle.state::<AppState>();
    #[cfg(target_os = "macos")]
    let interface_name = get_interface_name();
    #[cfg(not(target_os = "macos"))]
    let interface_name = get_interface_name(&location.name);
    setup_interface(
        location,
        interface_name.clone(),
        preshared_key,
        &state.get_pool(),
        state.client.clone(),
    )
    .await?;
    let address = local_ip()?;
    let connection = ActiveConnection::new(
        location.id.expect("Missing Location ID"),
        address.to_string(),
        interface_name.clone(),
        ConnectionType::Location,
    );
    state
        .active_connections
        .lock()
        .map_err(|_| Error::MutexError)?
        .push(connection);
    debug!(
        "Active connections: {:#?}",
        state
            .active_connections
            .lock()
            .map_err(|_| Error::MutexError)?
    );
    debug!("Sending event connection-changed.");
    handle.emit_all(
        "connection-changed",
        Payload {
            message: "Created new connection".into(),
        },
    )?;

    // Spawn stats threads
    debug!("Spawning stats thread");
    spawn_stats_thread(
        handle.clone(),
        interface_name.clone(),
        ConnectionType::Location,
    )
    .await;

    // spawn log watcher
    spawn_log_watcher_task(
        handle,
        location.id.expect("Missing Location ID"),
        interface_name,
        ConnectionType::Location,
        Level::DEBUG,
        None,
    )
    .await?;
    Ok(())
}

/// Setup new connection for tunnel
pub async fn handle_connection_for_tunnel(tunnel: &Tunnel, handle: AppHandle) -> Result<(), Error> {
    debug!(
        "Creating new interface connection for tunnel: {}",
        tunnel.name
    );
    let state = handle.state::<AppState>();
    #[cfg(target_os = "macos")]
    let interface_name = get_interface_name();
    #[cfg(not(target_os = "macos"))]
    let interface_name = get_interface_name(&tunnel.name);
    setup_interface_tunnel(tunnel, interface_name.clone(), state.client.clone()).await?;
    let address = local_ip()?;
    let connection = ActiveConnection::new(
        tunnel.id.expect("Missing Tunnel ID"),
        address.to_string(),
        interface_name.clone(),
        ConnectionType::Tunnel,
    );
    state
        .active_connections
        .lock()
        .map_err(|_| Error::MutexError)?
        .push(connection);
    debug!(
        "Active connections: {:#?}",
        state
            .active_connections
            .lock()
            .map_err(|_| Error::MutexError)?
    );
    debug!("Sending event connection-changed.");
    handle.emit_all(
        "connection-changed",
        Payload {
            message: "Created new connection".into(),
        },
    )?;

    // Spawn stats threads
    info!("Spawning stats thread");
    spawn_stats_thread(
        handle.clone(),
        interface_name.clone(),
        ConnectionType::Tunnel,
    )
    .await;

    //spawn log watcher
    spawn_log_watcher_task(
        handle,
        tunnel.id.expect("Missing Tunnel ID"),
        interface_name,
        ConnectionType::Tunnel,
        Level::DEBUG,
        None,
    )
    .await?;
    Ok(())
}
/// Execute command passed as argument.
pub fn execute_command(command: &str) -> Result<(), Error> {
    let mut command_parts = command.split_whitespace();

    if let Some(command) = command_parts.next() {
        let output = Command::new(command).args(command_parts).output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            info!("Command executed successfully. Stdout:\n{}", stdout);
            if !stderr.is_empty() {
                error!("Stderr:\n{stderr}");
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Error executing command. Stderr:\n{stderr}");
        }
    }
    Ok(())
}
/// Helper function to remove interface and close connection
pub async fn disconnect_interface(
    active_connection: ActiveConnection,
    state: &AppState,
) -> Result<(), Error> {
    debug!("Removing interface");
    let mut client = state.client.clone();
    let interface_name = active_connection.interface_name.clone();
    let (id, connection_type) = (
        active_connection.location_id,
        active_connection.connection_type.clone(),
    );
    match active_connection.connection_type {
        ConnectionType::Location => {
            let request = RemoveInterfaceRequest {
                interface_name: interface_name.clone(),
                pre_down: None,
                post_down: None,
            };
            if let Err(error) = client.remove_interface(request).await {
                error!("Failed to remove interface: {error}");
                return Err(Error::InternalError);
            }
            let mut connection: Connection = active_connection.into();
            connection.save(&state.get_pool()).await?;
            trace!("Saved connection: {connection:#?}");
            debug!("Removed interface");
            debug!("Saving connection");
            trace!("Connection: {:#?}", connection);
        }
        ConnectionType::Tunnel => {
            if let Some(tunnel) =
                Tunnel::find_by_id(&state.get_pool(), active_connection.location_id).await?
            {
                let request = RemoveInterfaceRequest {
                    interface_name: interface_name.clone(),
                    pre_down: tunnel.pre_down,
                    post_down: tunnel.post_down,
                };
                if let Err(error) = client.remove_interface(request).await {
                    error!("Failed to remove interface: {error}");
                    return Err(Error::InternalError);
                }
                let mut connection: TunnelConnection = active_connection.into();
                connection.save(&state.get_pool()).await?;
                trace!("Saved connection: {connection:#?}");
            } else {
                error!("Tunnel with ID {} not found", active_connection.location_id);
                return Err(Error::NotFound);
            }
        }
    }

    info!("Location {} {:?} disconnected", id, connection_type);
    Ok(())
}
