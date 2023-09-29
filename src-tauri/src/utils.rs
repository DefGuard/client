use std::{net::SocketAddr, str::FromStr};

use wireguard_rs::{
    wgapi::WGApi, InterfaceConfiguration, IpAddrMask, Key, Peer, WireguardInterfaceApi,
};

use crate::{
    database::{DbPool, Location, WireguardKeys},
    error::Error,
};

// TODO: Learn how to run tauri app with sudo permissions to setup interface
/// Setup client interface
pub async fn setup_interface(location: &Location, pool: &DbPool) -> Result<(), Error> {
    let interface_name = remove_whitespace(&location.name);
    debug!("Creating interface: {}", interface_name);
    let api = WGApi::new(interface_name.clone(), false)?;

    if let Some(keys) = WireguardKeys::find_by_instance_id(pool, location.instance_id).await? {
        // TODO: handle unwrap
        let peer_key: Key = Key::from_str(&location.pubkey).unwrap();
        let mut peer = Peer::new(peer_key);
        println!("{}", location.endpoint);
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
                    // TODO: Handle other OS than linux
                    // Add a route for the allowed IP using the `ip -4 route add` command
                    if let Err(err) = std::process::Command::new("ip")
                        .args(["-4", "route", "add", &allowed_ip, "dev", &interface_name])
                        .output()
                    {
                        // Handle the error if the ip command fails
                        eprintln!("Error adding route for {}: {}", allowed_ip, err);
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
        if let Some((address, port)) = location.endpoint.split_once(":") {
            let interface_config = InterfaceConfiguration {
                name: interface_name.clone(),
                prvkey: keys.prvkey,
                address: address.into(),
                port: port.parse().unwrap(),
                peers: vec![peer],
            };
            info!("created interface {:#?}", interface_config);
        };

        return Ok(());
    } else {
        return Err(Error::IpAddrMask());
    }
}
pub fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}
