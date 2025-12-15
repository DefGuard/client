#[cfg(not(target_os = "macos"))]
pub mod client;
pub mod config;
pub mod proto {
    tonic::include_proto!("client");
}
#[cfg(not(target_os = "macos"))]
pub mod daemon;
#[cfg(windows)]
pub mod named_pipe;
pub mod utils;
#[cfg(windows)]
pub mod windows;

use std::{
    str::FromStr,
    time::{Duration, UNIX_EPOCH},
};

use defguard_wireguard_rs::{
    host::{Host, Peer},
    key::Key,
    net::IpAddrMask,
    InterfaceConfiguration,
};

impl From<InterfaceConfiguration> for proto::InterfaceConfig {
    fn from(config: InterfaceConfiguration) -> Self {
        Self {
            name: config.name,
            prvkey: config.prvkey,
            address: config
                .addresses
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            port: u32::from(config.port),
            peers: config.peers.into_iter().map(Into::into).collect(),
            mtu: config.mtu,
        }
    }
}

impl From<proto::InterfaceConfig> for InterfaceConfiguration {
    fn from(config: proto::InterfaceConfig) -> Self {
        let addresses = config
            .address
            .split(',')
            .filter_map(|ip| IpAddrMask::from_str(ip.trim()).ok())
            .collect();
        Self {
            name: config.name,
            prvkey: config.prvkey,
            addresses,
            port: config.port as u16,
            peers: config.peers.into_iter().map(Into::into).collect(),
            mtu: config.mtu,
        }
    }
}

impl From<Peer> for proto::Peer {
    fn from(peer: Peer) -> Self {
        Self {
            public_key: peer.public_key.to_lower_hex(),
            preshared_key: peer.preshared_key.map(|key| key.to_lower_hex()),
            protocol_version: peer.protocol_version,
            endpoint: peer.endpoint.map(|addr| addr.to_string()),
            last_handshake: peer.last_handshake.map(|time| {
                time.duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs()
            }),
            tx_bytes: peer.tx_bytes,
            rx_bytes: peer.rx_bytes,
            persistent_keepalive_interval: peer.persistent_keepalive_interval.map(u32::from),
            allowed_ips: peer
                .allowed_ips
                .into_iter()
                .map(|addr| addr.to_string())
                .collect(),
        }
    }
}

impl From<proto::Peer> for Peer {
    fn from(peer: proto::Peer) -> Self {
        Self {
            public_key: Key::decode(peer.public_key).expect("Failed to parse public key"),
            preshared_key: peer
                .preshared_key
                .map(|key| Key::decode(key).expect("Failed to parse preshared key: {key}")),
            protocol_version: peer.protocol_version,
            endpoint: peer.endpoint.map(|addr| {
                addr.parse()
                    .expect("Failed to parse endpoint address: {addr}")
            }),
            last_handshake: peer
                .last_handshake
                .map(|timestamp| UNIX_EPOCH + Duration::from_secs(timestamp)),
            tx_bytes: peer.tx_bytes,
            rx_bytes: peer.rx_bytes,
            persistent_keepalive_interval: peer
                .persistent_keepalive_interval
                .and_then(|interval| u16::try_from(interval).ok()),
            allowed_ips: peer
                .allowed_ips
                .into_iter()
                .map(|addr| addr.parse().expect("Failed to parse allowed IP: {addr}"))
                .collect(),
        }
    }
}

impl From<Host> for proto::InterfaceData {
    fn from(host: Host) -> Self {
        Self {
            listen_port: u32::from(host.listen_port),
            peers: host.peers.into_values().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use x25519_dalek::{EphemeralSecret, PublicKey};

    use super::*;

    #[test]
    fn convert_peer() {
        let secret = EphemeralSecret::random();
        let key = PublicKey::from(&secret);
        let peer_key: Key = key.as_ref().try_into().unwrap();
        let mut base_peer = Peer::new(peer_key);
        let addr = IpAddrMask::from_str("10.20.30.2/32").unwrap();
        base_peer.allowed_ips.push(addr);
        // Workaround since nanoseconds are lost in conversion.
        base_peer.last_handshake = Some(SystemTime::UNIX_EPOCH);
        base_peer.protocol_version = Some(3);
        base_peer.endpoint = Some("127.0.0.1:8080".parse().unwrap());
        base_peer.tx_bytes = 100;
        base_peer.rx_bytes = 200;

        let proto_peer: proto::Peer = base_peer.clone().into();

        let converted_peer: Peer = proto_peer.into();

        assert_eq!(base_peer, converted_peer);
    }
}
