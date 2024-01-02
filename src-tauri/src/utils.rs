use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    str::FromStr,
};

use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration};
use tauri::Manager;
use tonic::{codegen::tokio_stream::StreamExt, transport::Channel};

use crate::{
    appstate::AppState,
    database::{models::location::peer_to_location_stats, DbPool, Location, WireguardKeys, Tunnel},
    error::Error,
    service::proto::{
        desktop_daemon_service_client::DesktopDaemonServiceClient, CreateInterfaceRequest,
        ReadInterfaceDataRequest,
    },
};

pub static IS_MACOS: bool = cfg!(target_os = "macos");
pub static STATS_PERIOD: u64 = 60;
pub static DEFAULT_ROUTE: &str = "0.0.0.0/0";

/// Setup client interface
pub async fn setup_interface(
    location: &Location,
    interface_name: String,
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

pub async fn spawn_stats_thread(handle: tauri::AppHandle, interface_name: String) {
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
    // FIXME: find out what's a shared log dir on Windows
    #[cfg(target_os = "windows")]
    unimplemented!();

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
        peer.persistent_keepalive_interval = Some(tunnel.persistent_keep_alive.try_into().expect("Failed to parse persistent keep alive"));

        debug!("Parsing location allowed ips: {:?}", tunnel.allowed_ips);
        let allowed_ips: Vec<String> = if tunnel.route_all_traffic {
            debug!("Using all traffic routing: {DEFAULT_ROUTE}");
            vec![DEFAULT_ROUTE.into()]
        } else {
            debug!("Using predefined location traffic");
            tunnel.allowed_ips
              .as_ref()
                    .map(|ips| ips.split(',').map(str::to_string).collect())
                    .unwrap_or(Vec::new())
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
