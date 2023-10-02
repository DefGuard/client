use std::{net::SocketAddr, str::FromStr};

use wireguard_rs::{
    wgapi::WGApi, InterfaceConfiguration, IpAddrMask, Key, Peer, WireguardInterfaceApi,
};

use crate::{
    database::{DbPool, Location, WireguardKeys},
    error::Error,
};

/// Setup client interface
pub async fn setup_interface(location: &Location, pool: &DbPool) -> Result<(), Error> {
    let interface_name = remove_whitespace(&location.name);
    debug!("Creating interface: {}", interface_name);
    let api = WGApi::new(interface_name.clone(), false)?;

    if let Some(keys) = WireguardKeys::find_by_instance_id(pool, location.instance_id).await? {
        // TODO: handle unwrap
        let peer_key: Key = Key::from_str(&location.pubkey).unwrap();
        let mut peer = Peer::new(peer_key);
        debug!("Creating interface for location: {:#?}", location);
        let endpoint: SocketAddr = location.endpoint.parse().unwrap();
        peer.endpoint = Some(endpoint);
        peer.persistent_keepalive_interval = Some(25);
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
                    // TODO: Handle windows later
                    // Add a route for the allowed IP using the `ip -4 route add` command
                    if let Err(err) = add_route(&allowed_ip, &interface_name) {
                        error!("Error adding route for {}: {}", allowed_ip, err);
                    } else {
                        info!("Added route for {}", allowed_ip);
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
        if let Some((address, port)) = location.endpoint.split_once(':') {
            let interface_config = InterfaceConfiguration {
                name: interface_name.clone(),
                prvkey: keys.prvkey,
                address: address.into(),
                port: port.parse().unwrap(),
                peers: vec![peer],
            };
            debug!("Creating interface {:#?}", interface_config);
            api.configure_interface(&interface_config)?;
            info!("created interface {:#?}", interface_config);
            Ok(())
        } else {
            error!("Failed to parse location endpoint: {}", location.endpoint);
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
