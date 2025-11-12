#[cfg(target_os = "macos")]
use std::time::Duration;
use std::{env, path::Path, process::Command, str::FromStr};

use common::{find_free_tcp_port, get_interface_name};
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration};
use sqlx::query;
use tauri::{AppHandle, Emitter, Manager};
#[cfg(not(target_os = "macos"))]
use tonic::Code;
use tracing::Level;
#[cfg(windows)]
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};
#[cfg(windows)]
use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;

#[cfg(windows)]
use crate::active_connections::find_connection;
#[cfg(target_os = "macos")]
use crate::apple::{all_tunnel_stats, start_tunnel, stop_tunnel};
#[cfg(not(target_os = "macos"))]
use crate::service::{
    proto::{CreateInterfaceRequest, ReadInterfaceDataRequest, RemoveInterfaceRequest},
    utils::DAEMON_CLIENT,
};
use crate::{
    appstate::AppState,
    commands::LocationInterfaceDetails,
    database::{
        models::{
            connection::{ActiveConnection, Connection},
            location::Location,
            location_stats::peer_to_location_stats,
            tunnel::{peer_to_tunnel_stats, Tunnel, TunnelConnection},
            wireguard_keys::WireguardKeys,
            Id,
        },
        DbPool, DB_POOL,
    },
    error::Error,
    events::EventKey,
    log_watcher::service_log_watcher::spawn_log_watcher_task,
    ConnectionType,
};

pub(crate) static DEFAULT_ROUTE_IPV4: &str = "0.0.0.0/0";
pub(crate) static DEFAULT_ROUTE_IPV6: &str = "::/0";
// Work-around MFA propagation delay. FIXME: remove once Core API is corrected.
static TUNNEL_START_DELAY: Duration = Duration::from_secs(1);

/// Setup client interface for `Instance`.
#[cfg(not(target_os = "macos"))]
pub(crate) async fn setup_interface(
    location: &Location<Id>,
    name: &str,
    preshared_key: Option<String>,
    pool: &DbPool,
) -> Result<String, Error> {
    debug!("Setting up interface for location: {location}");
    let interface_name = get_interface_name(name);

    // request interface configuration
    debug!("Looking for a free port for interface {interface_name}.");
    let Some(port) = find_free_tcp_port() else {
        let msg = format!(
            "Couldn't find free port during interface {interface_name} setup for location \
            {location}"
        );
        error!("{msg}");
        return Err(Error::InternalError(msg));
    };
    debug!("Found free port: {port} for interface {interface_name}.");

    let interface_config = location
        .interface_configurarion(pool, interface_name.clone(), preshared_key)
        .await?;
    debug!("Creating interface for location {location} with configuration {interface_config:?}");
    let request = CreateInterfaceRequest {
        config: Some(interface_config.clone().into()),
        dns: location.dns.clone(),
    };
    if let Err(error) = DAEMON_CLIENT.clone().create_interface(request).await {
        if error.code() == Code::Unavailable {
            error!(
                "Failed to set up connection for location {location}; background service is \
                unavailable. Make sure the service is running. Error: {error}, Interface \
                configuration: {interface_config:?}"
            );
            Err(Error::InternalError(
                "Background service is unavailable. Make sure the service is running.".into(),
            ))
        } else {
            error!(
                "Failed to send a request to the background service to create an interface for \
                location {location} with the following configuration: {interface_config:?}. \
                Error: {error}"
            );
            Err(Error::InternalError(format!(
                "Failed to send a request to the background service to create an interface for \
                location {location}. Error: {error}. Check logs for details."
            )))
        }
    } else {
        info!(
            "The interface for location {location} has been created successfully, interface \
            name: {}.",
            interface_config.name
        );
        Ok(interface_name)
    }
}

#[cfg(target_os = "macos")]
pub(crate) async fn setup_interface(
    location: &Location<Id>,
    name: &str,
    preshared_key: Option<String>,
    pool: &DbPool,
) -> Result<String, Error> {
    debug!("Setting up interface for location: {location}");
    let interface_name = get_interface_name(name);

    let (dns, dns_search) = location.dns();
    let tunnel_config = location
        .tunnel_configurarion(pool, preshared_key, dns, dns_search)
        .await?;

    tunnel_config.save();
    tokio::time::sleep(TUNNEL_START_DELAY).await;
    start_tunnel(&location.name);

    Ok(interface_name)
}

#[cfg(target_os = "macos")]
pub(crate) async fn stats_handler(
    pool: DbPool,
    _interface_name: String,
    _connection_type: ConnectionType,
) {
    const CHECK_INTERVAL: Duration = Duration::from_secs(5);
    let mut interval = tokio::time::interval(CHECK_INTERVAL);

    loop {
        info!("Stats loop");
        interval.tick().await;

        let all_stats = all_tunnel_stats();
        if all_stats.len() == 0 {
            continue;
        }
        // Let `all_stats` be `Send`.
        let all_stats = all_stats.as_slice().to_owned();

        // let mut transaction = match pool.begin().await {
        //     Ok(transactions) => transactions,
        //     Err(err) => {
        //         error!(
        //             "Failed to begin database transaction for saving location/tunnel stats: {err}",
        //         );
        //         continue;
        //     }
        // };

        for stats in all_stats {
            info!(
                "==> Stats: {} {} {}",
                stats.last_handshake, stats.tx_bytes, stats.rx_bytes
            );

            if let Some(location_id) = stats.location_id {
                //let location_stats = LocationStats {
                //};
                /*match location_stats.save(&mut *transaction).await {
                    Ok(_) => {
                        debug!("Saved network usage stats for location ID {location_id}");
                    }
                    Err(err) => {
                        error!(
                            "Failed to save network usage stats for location ID {location_id}: \
                            {err}"
                        );
                    }
                }*/
            }
            if let Some(tunnel_id) = stats.tunnel_id {
                //let tunnel_stats = TunnelStats {
                //};
                /*match tunnel_stats.save(&mut *transaction).await {
                    Ok(_) => {
                        debug!("Saved network usage stats for tunnel ID {tunnel_id}");
                    }
                    Err(err) => {
                        error!(
                            "Failed to save network usage stats for tunnel ID {tunnel_id}: \
                            {err}"
                        );
                    }
                }*/
            }
        }

        // if let Err(err) = transaction.commit().await {
        //     error!("Failed to commit database transaction for saving location/tunnel stats: {err}");
        // }
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) async fn stats_handler(
    pool: DbPool,
    interface_name: String,
    connection_type: ConnectionType,
) {
    let request = ReadInterfaceDataRequest {
        interface_name: interface_name.clone(),
    };
    let mut stream = DAEMON_CLIENT
        .clone()
        .read_interface_data(request)
        .await
        .expect("Failed to connect to interface stats stream for interface {interface_name}")
        .into_inner();

    loop {
        match stream.message().await {
            Ok(Some(interface_data)) => {
                debug!("Received new network usage statistics for interface {interface_name}.");
                trace!("Received interface data: {interface_data:?}");

                // begin transaction
                let mut transaction = match pool.begin().await {
                    Ok(transactions) => transactions,
                    Err(err) => {
                        error!(
                            "Failed to begin database transaction for saving location/tunnel stats: {err}",
                        );
                        continue;
                    }
                };

                let peers: Vec<Peer> = interface_data.peers.into_iter().map(Into::into).collect();
                for peer in peers {
                    if connection_type.eq(&ConnectionType::Location) {
                        let location_stats = match peer_to_location_stats(
                            &peer,
                            interface_data.listen_port,
                            &mut *transaction,
                        )
                        .await
                        {
                            Ok(stats) => stats,
                            Err(err) => {
                                error!("Failed to convert peer data to location stats: {err}");
                                continue;
                            }
                        };
                        let location_name = location_stats
                            .get_name(&mut *transaction)
                            .await
                            .unwrap_or("UNKNOWN".to_string());

                        debug!(
                            "Saving network usage stats related to location {location_name} \
                            (interface {interface_name})."
                        );
                        trace!("Stats: {location_stats:?}");
                        match location_stats.save(&mut *transaction).await {
                            Ok(_) => {
                                debug!("Saved network usage stats for location {location_name}");
                            }
                            Err(err) => {
                                error!(
                                    "Failed to save network usage stats for location \
                                    {location_name}: {err}"
                                );
                            }
                        }
                    } else {
                        let tunnel_stats = match peer_to_tunnel_stats(
                            &peer,
                            interface_data.listen_port,
                            &mut *transaction,
                        )
                        .await
                        {
                            Ok(stats) => stats,
                            Err(err) => {
                                error!("Failed to convert peer data to tunnel stats: {err}");
                                continue;
                            }
                        };
                        let tunnel_name = tunnel_stats
                            .get_name(&mut *transaction)
                            .await
                            .unwrap_or("UNKNOWN".to_string());
                        debug!(
                            "Saving network usage stats related to tunnel {tunnel_name} \
                            (interface {interface_name}): {tunnel_stats:?}"
                        );
                        match tunnel_stats.save(&mut *transaction).await {
                            Ok(_) => {
                                debug!("Saved stats for tunnel {tunnel_name}");
                            }
                            Err(err) => {
                                error!("Failed to save stats for tunnel {tunnel_name}: {err}");
                            }
                        }
                    }
                }

                // commit transaction
                if let Err(err) = transaction.commit().await {
                    error!(
                        "Failed to commit database transaction for saving location/tunnel stats: \
                        {err}",
                    );
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
    if let Ok(targets) = env::var("DEFGUARD_CLIENT_LOG_INCLUDE") {
        if !targets.is_empty() {
            return targets
                .split(',')
                .filter(|t| !t.is_empty())
                .map(ToString::to_string)
                .collect();
        }
    }
    Vec::new()
}

/// Helper function to get log file directory for `defguard-service` daemon.
#[must_use]
pub fn get_service_log_dir() -> &'static Path {
    #[cfg(windows)]
    let path = "/Logs/defguard-service";

    #[cfg(not(windows))]
    let path = "/var/log/defguard-service";

    Path::new(path)
}

/// Setup client interface
pub async fn setup_interface_tunnel(tunnel: &Tunnel<Id>, name: &str) -> Result<String, Error> {
    debug!("Setting up interface for tunnel {tunnel}");
    let interface_name = get_interface_name(name);
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
        debug!("Using all traffic routing for tunnel {tunnel}");
        vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
    } else {
        let msg = match &tunnel.allowed_ips {
            Some(ips) => format!("Using predefined location traffic for tunnel {tunnel}: {ips}"),
            None => "No allowed IP addresses found in tunnel {tunnel} configuration".to_string(),
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
            }
        }
    }
    debug!("Parsed tunnel {tunnel} allowed IPs: {:?}", peer.allowed_ips);

    // request interface configuration
    debug!("Looking for a free port for interface {interface_name}.");
    let Some(port) = find_free_tcp_port() else {
        let msg = format!(
            "Couldn't find free port for interface {interface_name} while setting up tunnel {tunnel}"
        );
        error!("{msg}");
        return Err(Error::InternalError(msg));
    };
    debug!("Found free port: {port} for interface {interface_name}.");

    let addresses = tunnel
        .address
        .split(',')
        .map(str::trim)
        .map(IpAddrMask::from_str)
        .collect::<Result<_, _>>()
        .map_err(|err| {
            let msg = format!("Failed to parse IP addresses '{}': {err}", tunnel.address);
            error!("{msg}");
            Error::InternalError(msg)
        })?;
    let interface_config = InterfaceConfiguration {
        name: interface_name.clone(),
        prvkey: tunnel.prvkey.clone(),
        addresses,
        port,
        peers: vec![peer.clone()],
        mtu: None,
    };

    #[cfg(not(target_os = "macos"))]
    {
        debug!("Creating interface {interface_config:?}");
        let request = CreateInterfaceRequest {
            config: Some(interface_config.clone().into()),
            dns: tunnel.dns.clone(),
        };
        if let Some(pre_up) = &tunnel.pre_up {
            debug!(
                "Executing defined PreUp command before setting up the interface {} for the \
            tunnel {tunnel}: {pre_up}",
                interface_config.name
            );
            let _ = execute_command(pre_up);
            info!(
                "Executed defined PreUp command before setting up the interface {} for the \
            tunnel {tunnel}: {pre_up}",
                interface_config.name
            );
        }
        if let Err(error) = DAEMON_CLIENT.clone().create_interface(request).await {
            error!(
                "Failed to create a network interface ({}) for tunnel {tunnel}: {error}",
                interface_config.name
            );
            return Err(Error::InternalError(format!(
            "Failed to create a network interface ({}) for tunnel {tunnel}, error message: {}. \
            Check logs for more details.",
            interface_config.name,
            error.message()
        )));
        } else {
            info!(
                "Network interface {} for tunnel {tunnel} created successfully.",
                interface_config.name
            );
            if let Some(post_up) = &tunnel.post_up {
                debug!(
                    "Executing defined PostUp command after setting up the interface {} for the \
                tunnel {tunnel}: {post_up}",
                    interface_config.name
                );
                let _ = execute_command(post_up);
                info!(
                    "Executed defined PostUp command after setting up the interface {} for the \
                tunnel {tunnel}: {post_up}",
                    interface_config.name
                );
            }
            debug!(
                "Created interface {} with config: {interface_config:?}",
                interface_config.name
            );
        }
    }

    Ok(interface_name)
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
    let interface_name = setup_interface(location, &location.name, preshared_key, &DB_POOL).await?;
    state
        .add_connection(location.id, &interface_name, ConnectionType::Location)
        .await;

    debug!("Sending event informing the frontend that a new connection has been created.");
    handle.emit(EventKey::ConnectionChanged.into(), ())?;
    debug!("Event informing the frontend that a new connection has been created sent.");

    // spawn log watcher
    debug!("Spawning service log watcher for location {location}.");
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
    let interface_name = setup_interface_tunnel(tunnel, &tunnel.name).await?;
    state
        .add_connection(tunnel.id, &interface_name, ConnectionType::Tunnel)
        .await;

    debug!("Sending event informing the frontend that a new connection has been created.");
    handle.emit(EventKey::ConnectionChanged.into(), ())?;
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

            debug!("Command {command} executed successfully. Stdout: {stdout}");
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
) -> Result<(), Error> {
    debug!(
        "Disconnecting interface {}.",
        active_connection.interface_name
    );
    let location_id = active_connection.location_id;
    let interface_name = active_connection.interface_name.clone();

    match active_connection.connection_type {
        ConnectionType::Location => {
            let Some(location) = Location::find_by_id(&*DB_POOL, location_id).await? else {
                error!(
                    "Error while disconnecting interface {interface_name}, location with ID \
                    {location_id} not found"
                );
                return Err(Error::NotFound);
            };

            #[cfg(target_os = "macos")]
            {
                let result = stop_tunnel(&location.name);
                error!("stop_tunnel() for location returned {result:?}");
                if !result {
                    return Err(Error::InternalError("Error from tunnel".into()));
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                let mut client = DAEMON_CLIENT.clone();
                let request = RemoveInterfaceRequest {
                    interface_name,
                    endpoint: location.endpoint.clone(),
                };
                debug!(
                    "Sending request to the background service to remove interface {} for location \
                    {}...",
                    active_connection.interface_name, location.name
                );
                if let Err(error) = client.remove_interface(request).await {
                    let msg = if error.code() == Code::Unavailable {
                        format!(
                            "Couldn't remove interface {}. Background service is unavailable. \
                            Please make sure the service is running. Error: {error}.",
                            active_connection.interface_name
                        )
                    } else {
                        format!(
                            "Failed to send a request to the background service to remove interface \
                            {}. Error: {error}.",
                            active_connection.interface_name
                        )
                    };
                    error!("{msg}");
                }
            }

            let connection: Connection = active_connection.into();
            let connection = connection.save(&*DB_POOL).await?;
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
            let Some(tunnel) = Tunnel::find_by_id(&*DB_POOL, location_id).await? else {
                error!(
                    "Error while disconnecting interface {interface_name}, tunnel with ID \
                    {location_id} not found"
                );
                return Err(Error::NotFound);
            };
            if let Some(pre_down) = &tunnel.pre_down {
                debug!(
                    "Executing defined PreDown command before setting up the interface {} for the \
                    tunnel {tunnel}: {pre_down}",
                    active_connection.interface_name
                );
                let _ = execute_command(pre_down);
                info!(
                    "Executed defined PreDown command before setting up the interface {} for the \
                    tunnel {tunnel}: {pre_down}",
                    active_connection.interface_name
                );
            }

            #[cfg(target_os = "macos")]
            {
                let result = stop_tunnel(&tunnel.name);
                error!("stop_tunnel() for location returned {result:?}");
                if !result {
                    return Err(Error::InternalError("Error from tunnel".into()));
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                let request = RemoveInterfaceRequest {
                    interface_name,
                    endpoint: tunnel.endpoint.clone(),
                };
                if let Err(error) = client.remove_interface(request).await {
                    error!(
                        "Error while removing interface {}, error details: {error:?}",
                        active_connection.interface_name
                    );
                    return Err(Error::InternalError(format!(
                        "Failed to remove interface, error message: {}",
                        error.message()
                    )));
                }
            }
            if let Some(post_down) = &tunnel.post_down {
                debug!(
                    "Executing defined PostDown command after removing the interface {} for the \
                    tunnel {tunnel}: {post_down}",
                    active_connection.interface_name
                );
                let _ = execute_command(post_down);
                info!(
                    "Executed defined PostDown command after removing the interface {} for the \
                    tunnel {tunnel}: {post_down}",
                    active_connection.interface_name
                );
            }
            let connection: TunnelConnection = active_connection.into();
            let connection = connection.save(&*DB_POOL).await?;
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
pub async fn get_tunnel_or_location_name(id: Id, connection_type: ConnectionType) -> String {
    let name = match connection_type {
        ConnectionType::Location => Location::find_by_id(&*DB_POOL, id)
            .await
            .ok()
            .and_then(|l| l.map(|l| l.name)),
        ConnectionType::Tunnel => Tunnel::find_by_id(&*DB_POOL, id)
            .await
            .ok()
            .and_then(|t| t.map(|t| t.name)),
    };

    if let Some(name) = name {
        name
    } else {
        debug!(
            "Couldn't identify {connection_type}'s name for logging purposes, \
            it will be referred to as UNKNOWN",
        );
        "UNKNOWN".to_string()
    }
}

// Check if location/tunnel is connected and WireGuard Windows service is running.
// `id`: location or tunnel Id
// `name`: location or tunnel name
#[cfg(windows)]
async fn check_connection(
    service_manager: &ServiceManager,
    id: Id,
    name: &str,
    connection_type: ConnectionType,
    app_handle: &AppHandle,
) -> Result<(), Error> {
    let appstate = app_handle.state::<AppState>();
    let interface_name = get_interface_name(name);
    let service_name = format!("WireGuardTunnel${interface_name}");
    let service = match service_manager.open_service(&service_name, ServiceAccess::QUERY_STATUS) {
        Ok(service) => service,
        Err(windows_service::Error::Winapi(err))
            if err.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) =>
        {
            debug!("WireGuard tunnel {interface_name} is not installed, nothing to synchronize");
            return Ok(());
        }
        Err(err) => {
            warn!(
                "Failed to open service {service_name} for interface {interface_name} while \
                synchronizing active connections. This may cause the {connection_type} {name} \
                state to display incorrectly in the client. Reconnect to it manually to fix it. \
                Error: {err}"
            );
            return Ok(());
        }
    };
    match service.query_status() {
        Ok(status) => {
            // Only point where we don't return and continue with the rest of the code below.
            if status.current_state == ServiceState::Running {
                debug!("WireGuard tunnel {interface_name} is running.");
            } else {
                debug!(
                    "WireGuard tunnel {interface_name} is not running, status code: {:?}. Refer to \
                    Windows documentation for more information about the code.",
                    status.current_state
                );
                return Ok(());
            }
        }
        Err(err) => {
            warn!(
              "Failed to query service status for interface {interface_name} while synchronizing \
              active connections. This may cause the {connection_type} {name} state to display \
              incorrectly in the client. Reconnect to it manually to fix it. Error: {err}",
            );
            return Ok(());
        }
    }

    if find_connection(id, connection_type).await.is_some() {
        debug!("{connection_type} {name} has already a connected state, skipping synchronization");
        return Ok(());
    }

    appstate
        .add_connection(id, &interface_name, connection_type)
        .await;

    debug!("Sending event informing the frontend that a new connection has been created.");
    app_handle.emit(EventKey::ConnectionChanged.into(), ())?;
    debug!("Event informing the frontend that a new connection has been created sent.");

    debug!("Spawning service log watcher for {connection_type} {name}...");
    spawn_log_watcher_task(
        app_handle.clone(),
        id,
        interface_name,
        connection_type,
        Level::DEBUG,
        None,
    )
    .await?;
    debug!("Service log watcher for {connection_type} {name} spawned.");

    Ok(())
}

// TODO: Move the connection handling to a seperate, common function,
// so `handle_connection_for_location` and `handle_connection_for_tunnel` are not
// partially duplicated here.
#[cfg(windows)]
pub async fn sync_connections(app_handle: &AppHandle) -> Result<(), Error> {
    debug!("Synchronizing active connections with the systems' state...");
    let all_locations = Location::all(&*DB_POOL, false).await?;
    let service_manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT).map_err(
            |err| {
                error!(
                    "Failed to open service control manager while trying to sync client's \
                    connections with the host state: {err}"
                );
                Error::InternalError(
                    "Failed to open service control manager while trying to sync client's \
                    connections with the host state"
                        .to_string(),
                )
            },
        )?;

    debug!("Opened service control manager. Synchronizing active connections for locations...");
    // Go through all locations and check if they are connected and Windows service is running.
    // If we encounter any errors, continue with the next iteration of the loop, it's not a big deal
    // if we skip some locations, as the user can always reconnect to them manually.
    for location in all_locations {
        check_connection(
            &service_manager,
            location.id,
            &location.name,
            ConnectionType::Location,
            app_handle,
        )
        .await?;
    }

    debug!("Synchronizing active connections for tunnels...");
    // Do the same for tunnels
    for tunnel in Tunnel::all(&*DB_POOL).await? {
        check_connection(
            &service_manager,
            tunnel.id,
            &tunnel.name,
            ConnectionType::Tunnel,
            app_handle,
        )
        .await?;
    }

    debug!("Active connections synchronized with the system state");

    Ok(())
}
