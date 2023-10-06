use std::{
    net::{SocketAddr, TcpListener},
    panic::Location as ErrorLocation,
    str::FromStr,
};

use defguard_wireguard_rs::{
    host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};

use crate::{
    database::{ActiveConnection, DbPool, Location, WireguardKeys},
    error::Error,
};

pub static IS_MACOS: bool = true;

/// Setup client interface
pub async fn setup_interface(
    location: &Location,
    interface_name: &str,
    pool: &DbPool,
) -> Result<WGApi, Error> {
    debug!(
        "Creating new interface: {} for location: {:#?}",
        interface_name, location
    );

    if let Some(keys) = WireguardKeys::find_by_instance_id(pool, location.instance_id).await? {
        let api = create_api(&interface_name).log()?;

        api.create_interface().log()?;
        // TODO: handle unwrap
        debug!("Decoding location public key: {}.", location.pubkey);
        let peer_key: Key = Key::from_str(&location.pubkey).unwrap();
        let mut peer = Peer::new(peer_key);
        debug!("Parsing location endpoint: {}", location.endpoint);
        let endpoint: SocketAddr = location.endpoint.parse().log()?;
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
                    if let Err(err) = add_route(&allowed_ip, &interface_name) {
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
        .iter()
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

            return interface_name;
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

pub trait LogError {
    fn log(self) -> Self;
}
/// Trait to know when mapped error failed and how
/// example use failing_function().log()?;
impl<T, E> LogError for Result<T, E>
where
    E: std::fmt::Display,
{
    #[track_caller]
    fn log(self) -> Self {
        if let Err(e) = &self {
            error!("Error '{e}' originated in :{}", &ErrorLocation::caller());
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Location;
    use chrono::Utc;

    #[test]
    fn test_macos_existing_interfaces() {
        let location = Location {
            id: None,
            instance_id: 1,
            network_id: 2,
            name: "Example Location".to_string(),
            address: "123 Main St".to_string(),
            pubkey: "public_key_here".to_string(),
            endpoint: "endpoint_here".to_string(),
            allowed_ips: "allowed_ips_here".to_string(),
        };
        let active_connections = vec![ActiveConnection {
            location_id: 1,
            start: Utc::now().naive_utc(),
            connected_from: "Test".to_string(),
            interface_name: "utun3".to_string(),
        }];

        assert_eq!(get_interface_name(&location, active_connections), "utun4");
    }
}
