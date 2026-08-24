use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
    time::{Duration, UNIX_EPOCH},
};

use defguard_wireguard_rs::{
    host::Host, key::Key, net::IpAddrMask, peer::Peer, InterfaceConfiguration,
};

use crate::defguard::client::v1::{InterfaceConfig, InterfaceData, Peer as ProtoPeer};

/// Clears host bits from a peer allowed IP.
///
/// This runs before `WGApi` classifies default routes. In particular, a non-canonical `/0` must
/// become an unspecified address so it takes the default-route loop-prevention path.
#[must_use]
pub fn mask_allowed_ip(mut allowed_ip: IpAddrMask) -> IpAddrMask {
    allowed_ip.address = match allowed_ip.address {
        IpAddr::V4(address) => {
            let mask = if allowed_ip.cidr == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(allowed_ip.cidr))
            };
            IpAddr::V4(Ipv4Addr::from(u32::from(address) & mask))
        }
        IpAddr::V6(address) => {
            let mask = if allowed_ip.cidr == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(allowed_ip.cidr))
            };
            IpAddr::V6(Ipv6Addr::from(u128::from(address) & mask))
        }
    };
    allowed_ip
}

/// Normalizes and deduplicates peer allowed IPs before they reach `WGApi`.
pub fn normalize_allowed_ips(config: &mut InterfaceConfiguration) {
    for peer in &mut config.peers {
        let mut seen = HashSet::new();
        peer.allowed_ips = std::mem::take(&mut peer.allowed_ips)
            .into_iter()
            .map(mask_allowed_ip)
            .filter(|allowed_ip| seen.insert(allowed_ip.clone()))
            .collect();
    }
}

impl From<InterfaceConfiguration> for InterfaceConfig {
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

impl From<InterfaceConfig> for InterfaceConfiguration {
    fn from(config: InterfaceConfig) -> Self {
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
            fwmark: None, // TODO: add to config
        }
    }
}

impl From<Peer> for ProtoPeer {
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

impl From<ProtoPeer> for Peer {
    fn from(peer: ProtoPeer) -> Self {
        Self {
            public_key: Key::decode(peer.public_key).expect("Failed to parse public key"),
            preshared_key: peer.preshared_key.map(|key| {
                Key::decode(&key).unwrap_or_else(|_| panic!("Failed to parse preshared key: {key}"))
            }),
            protocol_version: peer.protocol_version,
            endpoint: peer.endpoint.map(|addr| {
                addr.parse()
                    .unwrap_or_else(|_| panic!("Failed to parse endpoint address: {addr}"))
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
                .map(|addr| {
                    addr.parse()
                        .unwrap_or_else(|_| panic!("Failed to parse allowed IP: {addr}"))
                })
                .collect(),
        }
    }
}

impl From<Host> for InterfaceData {
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

    use defguard_wireguard_rs::{key::Key, net::IpAddrMask, peer::Peer};
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

        let proto_peer: ProtoPeer = base_peer.clone().into();

        let converted_peer: Peer = proto_peer.into();

        assert_eq!(base_peer, converted_peer);
    }

    fn sample_peer() -> Peer {
        let secret = EphemeralSecret::random();
        let peer_key: Key = PublicKey::from(&secret).as_ref().try_into().unwrap();
        let mut peer = Peer::new(peer_key);
        peer.allowed_ips
            .push(IpAddrMask::from_str("10.20.30.2/32").unwrap());
        peer.endpoint = Some("127.0.0.1:8080".parse().unwrap());
        peer.persistent_keepalive_interval = Some(25);
        peer
    }

    #[test]
    fn test_mask_allowed_ip_clears_ipv4_host_bits() {
        let allowed_ip = "172.16.0.1/24".parse::<IpAddrMask>().unwrap();

        assert_eq!(
            mask_allowed_ip(allowed_ip),
            "172.16.0.0/24".parse::<IpAddrMask>().unwrap()
        );
    }

    #[test]
    fn test_mask_allowed_ip_keeps_ipv4_host_route() {
        let allowed_ip = "172.16.0.1/32".parse::<IpAddrMask>().unwrap();

        assert_eq!(mask_allowed_ip(allowed_ip.clone()), allowed_ip);
    }

    #[test]
    fn test_mask_allowed_ip_keeps_canonical_address() {
        let allowed_ip = "172.16.0.0/24".parse::<IpAddrMask>().unwrap();

        assert_eq!(mask_allowed_ip(allowed_ip.clone()), allowed_ip);
    }

    #[test]
    fn test_mask_allowed_ip_handles_ipv4_default_route() {
        let allowed_ip = "10.0.0.1/0".parse::<IpAddrMask>().unwrap();

        assert_eq!(
            mask_allowed_ip(allowed_ip),
            "0.0.0.0/0".parse::<IpAddrMask>().unwrap()
        );
    }

    #[test]
    fn test_mask_allowed_ip_clears_ipv6_host_bits() {
        let allowed_ip = "2001:db8::1/96".parse::<IpAddrMask>().unwrap();

        assert_eq!(
            mask_allowed_ip(allowed_ip),
            "2001:db8::/96".parse::<IpAddrMask>().unwrap()
        );
    }

    #[test]
    fn test_normalize_allowed_ips_deduplicates_after_masking() {
        let mut peer = sample_peer();
        peer.allowed_ips = ["172.16.0.1/24", "172.16.0.2/24", "10.0.0.0/24"]
            .into_iter()
            .map(|allowed_ip| allowed_ip.parse().unwrap())
            .collect();
        let mut config = InterfaceConfiguration {
            name: "wg0".into(),
            prvkey: String::new(),
            addresses: vec!["10.0.0.1/24".parse().unwrap()],
            port: 0,
            peers: vec![peer],
            mtu: None,
            fwmark: None,
        };

        normalize_allowed_ips(&mut config);

        assert_eq!(
            config.peers[0].allowed_ips,
            ["172.16.0.0/24", "10.0.0.0/24"]
                .into_iter()
                .map(|allowed_ip| allowed_ip.parse().unwrap())
                .collect::<Vec<_>>()
        );
        assert_eq!(config.addresses, vec!["10.0.0.1/24".parse().unwrap()]);
    }

    #[test]
    fn test_host_to_interface_data() {
        let secret = EphemeralSecret::random();
        let host_key: Key = PublicKey::from(&secret).as_ref().try_into().unwrap();
        let mut host = Host::new(51820, host_key);
        let peer = sample_peer();
        host.peers.insert(peer.public_key.clone(), peer.clone());

        let data: InterfaceData = host.into();

        assert_eq!(data.listen_port, 51820);
        assert_eq!(data.peers.len(), 1);
        assert_eq!(data.peers[0].public_key, peer.public_key.to_lower_hex());
    }

    #[test]
    fn test_proto_peer_to_peer_roundtrip() {
        let peer = sample_peer();
        let proto: ProtoPeer = peer.clone().into();
        let converted: Peer = proto.into();
        assert_eq!(peer, converted);
    }

    #[test]
    fn test_keepalive_overflow_maps_to_none() {
        let mut proto: ProtoPeer = sample_peer().into();
        // A value exceeding u16::MAX can't be represented as a keepalive interval.
        proto.persistent_keepalive_interval = Some(u32::from(u16::MAX) + 1);
        let converted: Peer = proto.into();
        assert_eq!(converted.persistent_keepalive_interval, None);
    }
}
