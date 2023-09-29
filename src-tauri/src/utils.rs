use std::{net::SocketAddr, str::FromStr};

use wireguard_rs::{
    netlink::{address_interface, create_interface},
    wgapi::WGApi,
    IpAddrMask, Key, Peer,
};

use crate::{
    database::{DbPool, Location, WireguardKeys},
    error::Error,
};

// TODO: Learn how to run tauri app with sudo permissions to setup interface
/// Setup client interface
pub async fn setup_interface(location: &Location, pool: &DbPool) -> Result<(), Error> {
    let interface_name = remove_whitespace(&location.name);
    create_interface(&interface_name)?;
    address_interface(&interface_name, &IpAddrMask::from_str(&location.address)?)?;
    let api = WGApi::new(interface_name.clone(), false);

    let mut host = api.read_host()?;
    if let Some(keys) = WireguardKeys::find_by_instance_id(pool, location.instance_id).await? {
        // TODO: handle unwrap
        let private_key: Key = Key::from_str(&keys.prvkey).unwrap();
        host.private_key = Some(private_key);
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
                    println!("here2");
                    // Handle the error from IpAddrMask::from_str, if needed
                    eprintln!("Error parsing IP address {}: {}", allowed_ip, err);
                    // Continue to the next iteration of the loop
                    continue;
                }
            }
        }
        println!("{:#?}", peer);
        api.write_host(&host)?;
        api.write_peer(&peer)?;
    };

    Ok(())
}
pub fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}
