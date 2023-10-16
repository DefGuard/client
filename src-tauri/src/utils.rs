use std::{
    net::{SocketAddr, TcpListener},
    str::FromStr,
};

use defguard_wireguard_rs::{
    host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};

use crate::{
    appstate::AppState,
    database::{
        models::location::peer_to_location_stats, ActiveConnection, DbPool, Location, WireguardKeys,
    },
    error::Error,
};
use tauri::Manager;
use tokio::time::{sleep, Duration};

pub static IS_MACOS: bool = cfg!(target_os = "macos");
pub static STATS_PERIOD: u64 = 60;

/// Setup client interface
pub async fn setup_interface(
    location: &Location,
    interface_name: &str,
    pool: &DbPool,
) -> Result<WGApi, Error> {
    if let Some(keys) = WireguardKeys::find_by_instance_id(pool, location.instance_id).await? {
        debug!("Creating api for interface: '{}'", interface_name);
        let api = create_api(interface_name)?;

        api.create_interface()?;
        debug!("Decoding location public key: {}.", location.pubkey);
        let peer_key: Key = Key::from_str(&location.pubkey)?;
        let mut peer = Peer::new(peer_key);
        debug!("Parsing location endpoint: {}", location.endpoint);
        let endpoint: SocketAddr = location.endpoint.parse()?;
        peer.endpoint = Some(endpoint);
        peer.persistent_keepalive_interval = Some(25);

        debug!("Parsing location allowed ips: {}", location.allowed_ips);
        let allowed_ips: Vec<String> = location
            .allowed_ips
            .split(',')
            .map(str::to_string)
            .collect();
        debug!("Routing allowed ips");
        for allowed_ip in allowed_ips {
            match IpAddrMask::from_str(&allowed_ip) {
                Ok(addr) => {
                    peer.allowed_ips.push(addr);
                    // TODO: Handle windows when wireguard_rs adds support
                    // Add a route for the allowed IP using the `ip -4 route add` command
                    if let Err(err) = add_route(&allowed_ip, interface_name) {
                        error!("Error adding route for {}: {}", allowed_ip, err);
                    } else {
                        debug!("Added route for {}", allowed_ip);
                    }
                }
                Err(err) => {
                    // Handle the error from IpAddrMask::from_str, if needed
                    error!("Error parsing IP address {}: {}", allowed_ip, err);
                    // Continue to the next iteration of the loop
                    continue;
                }
            }
        }
        if let Some(port) = find_random_free_port() {
            let interface_config = InterfaceConfiguration {
                name: interface_name.into(),
                prvkey: keys.prvkey,
                address: location.address.clone(),
                port: port.into(),
                peers: vec![peer.clone()],
            };
            debug!("Creating interface {:#?}", interface_config);
            if let Err(err) = api.configure_interface(&interface_config) {
                error!("Failed to configure interface: {}", err.to_string());
                Err(Error::InternalError)
            } else {
                if let Err(err) = api.configure_peer(&peer) {
                    error!("Failed to configure peer: {}", err.to_string());
                    return Err(Error::InternalError);
                }
                info!("created interface {:#?}", interface_config);
                Ok(api)
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
pub fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[cfg(target_os = "linux")]
fn add_route(allowed_ip: &str, interface_name: &str) -> Result<(), std::io::Error> {
    std::process::Command::new("ip")
        .args(["-4", "route", "add", allowed_ip, "dev", interface_name])
        .output()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn add_route(allowed_ip: &str, interface_name: &str) -> Result<(), std::io::Error> {
    std::process::Command::new("route")
        .args([
            "-n",
            "add",
            "-net",
            allowed_ip,
            "-interface",
            interface_name,
        ])
        .output()?;
    Ok(())
}

fn find_random_free_port() -> Option<u16> {
    const MAX_PORT: u16 = 65535;
    const MIN_PORT: u16 = 6000;

    // Create a TcpListener to check for port availability
    for _ in 0..(MAX_PORT - MIN_PORT + 1) {
        let port = rand::random::<u16>() % (MAX_PORT - MIN_PORT) + MIN_PORT;
        if is_port_free(port) {
            return Some(port);
        }
    }

    None // No free port found in the specified range
}

/// Returns interface name for location
pub fn get_interface_name(
    location: &Location,
    active_connections: Vec<ActiveConnection>,
) -> String {
    let active_interfaces: Vec<String> = active_connections
        .into_iter()
        .map(|con| con.interface_name)
        .collect();
    match IS_MACOS {
        true => {
            let mut counter = 3;
            let mut interface_name = format!("utun{}", counter);

            while active_interfaces.contains(&interface_name) {
                counter += 1;
                interface_name = format!("utun{}", counter);
            }

            interface_name
        }
        false => remove_whitespace(&location.name),
    }
}

fn is_port_free(port: u16) -> bool {
    if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", port)) {
        // Port is available; close the listener
        drop(listener);
        true
    } else {
        false
    }
}

/// Create new api object
pub fn create_api(interface_name: &str) -> Result<WGApi, Error> {
    Ok(WGApi::new(interface_name.to_string(), IS_MACOS)?)
}

pub async fn spawn_stats_thread(
    handle: tauri::AppHandle,
    location: Location,
    api: WGApi, // Replace with your actual type
) {
    tokio::spawn(async move {
        let state = handle.state::<AppState>();
        loop {
            debug!("Reading interface data");
            match api.read_interface_data() {
                Ok(host) => {
                    let peers = host.peers;
                    for (_, peer) in peers {
                        let mut location_stats = peer_to_location_stats(&peer, &state.get_pool())
                            .await
                            .unwrap();
                        debug!("Saving location stats: {:#?}", location_stats);
                        let _ = location_stats.save(&state.get_pool()).await;
                        debug!("Saved location stats: {:#?}", location_stats);
                    }
                }
                Err(e) => {
                    error!(
                        "Error {} while reading data for interface: {}",
                        e, location.name
                    );
                    debug!(
                        "Stopped stats thread for location: {}. Error: {}",
                        location.name,
                        e.to_string()
                    );
                    break;
                }
            }
            sleep(Duration::from_secs(STATS_PERIOD)).await;
        }
    });
}
