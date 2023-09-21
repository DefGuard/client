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
    create_interface(&location.name)?;
    address_interface(&location.name, &IpAddrMask::from_str(&location.address)?)?;
    let api = WGApi::new(location.name.clone(), false);
    let mut host = api.read_host()?;
    if let Some(keys) = WireguardKeys::find_by_location_id(pool, location.instance_id).await? {
        // TODO: handle unwrap
        let private_key: Key = Key::from_str(&keys.prvkey).unwrap();
        host.private_key = Some(private_key);
        let peer_key: Key = Key::from_str(&location.pubkey).unwrap();
        let mut peer = Peer::new(peer_key);
        let endpoint: SocketAddr = location.endpoint.parse()?;
        peer.endpoint = Some(endpoint);
        peer.persistent_keepalive_interval = Some(25);
        let allowed_ips: Vec<String> = location
            .allowed_ips
            .split(',')
            .map(str::to_string)
            .collect();
        for allowed_ip in allowed_ips {
            let addr = IpAddrMask::from_str(&allowed_ip)?;
            peer.allowed_ips.push(addr);
            // TODO: Handle other OS than linux
            // Add a route for the allowed IP using the `ip -4 route add` command
            std::process::Command::new("ip")
                .args(["-4", "route", "add", &allowed_ip, "dev", &location.name])
                .output()?;
        }
    };

    Ok(())
}


