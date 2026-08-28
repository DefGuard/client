use std::{
    str::FromStr,
    time::{Duration, UNIX_EPOCH},
};

use defguard_wireguard_rs::{
    host::Host, key::Key, net::IpAddrMask, peer::Peer, InterfaceConfiguration,
};

use tonic::Status;

use crate::defguard::client::v1::{InterfaceConfig, InterfaceData, Peer as ProtoPeer};

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

impl TryFrom<InterfaceConfig> for InterfaceConfiguration {
    type Error = Status;

    fn try_from(config: InterfaceConfig) -> Result<Self, Self::Error> {
        let addresses = config
            .address
            .split(',')
            .filter_map(|ip| IpAddrMask::from_str(ip.trim()).ok())
            .collect();
        let peers = config
            .peers
            .into_iter()
            .map(Peer::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            name: config.name,
            prvkey: config.prvkey,
            addresses,
            port: config.port as u16,
            peers,
            mtu: config.mtu,
            fwmark: None, // TODO: add to config
        })
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
                    .unwrap_or_default()
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

impl TryFrom<ProtoPeer> for Peer {
    type Error = Status;

    fn try_from(peer: ProtoPeer) -> Result<Self, Self::Error> {
        let public_key = Key::decode(peer.public_key)
            .map_err(|err| Status::invalid_argument(format!("Invalid peer public key: {err}")))?;
        let preshared_key = peer
            .preshared_key
            .map(|key| {
                Key::decode(key).map_err(|err| {
                    Status::invalid_argument(format!("Invalid preshared key: {err}"))
                })
            })
            .transpose()?;
        let endpoint = peer
            .endpoint
            .map(|addr| {
                addr.parse().map_err(|err| {
                    Status::invalid_argument(format!("Invalid endpoint {addr}: {err}"))
                })
            })
            .transpose()?;
        let allowed_ips = peer
            .allowed_ips
            .into_iter()
            .map(|addr| {
                addr.parse().map_err(|err| {
                    Status::invalid_argument(format!("Invalid allowed IP {addr}: {err}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            public_key,
            preshared_key,
            protocol_version: peer.protocol_version,
            endpoint,
            last_handshake: peer
                .last_handshake
                .map(|timestamp| UNIX_EPOCH + Duration::from_secs(timestamp)),
            tx_bytes: peer.tx_bytes,
            rx_bytes: peer.rx_bytes,
            persistent_keepalive_interval: peer
                .persistent_keepalive_interval
                .and_then(|interval| u16::try_from(interval).ok()),
            allowed_ips,
        })
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

        let converted_peer: Peer = proto_peer.try_into().unwrap();

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
        let converted: Peer = proto.try_into().unwrap();
        assert_eq!(peer, converted);
    }

    #[test]
    fn test_invalid_peer_fields_are_rejected() {
        let base: ProtoPeer = sample_peer().into();
        let invalid = [
            ProtoPeer {
                public_key: "NOT-A-VALID-WIREGUARD-KEY".to_string(),
                ..base.clone()
            },
            ProtoPeer {
                preshared_key: Some("not-a-key".to_string()),
                ..base.clone()
            },
            ProtoPeer {
                endpoint: Some("not-an-endpoint".to_string()),
                ..base.clone()
            },
            ProtoPeer {
                allowed_ips: vec!["999.999.999.999/32".to_string()],
                ..base
            },
        ];

        for peer in invalid {
            let config = InterfaceConfig {
                name: "dg0".to_string(),
                prvkey: String::new(),
                address: "10.20.30.1/24".to_string(),
                port: 51820,
                peers: vec![peer],
                mtu: None,
            };
            let err = InterfaceConfiguration::try_from(config).unwrap_err();
            assert_eq!(err.code(), tonic::Code::InvalidArgument);
        }
    }

    #[test]
    fn test_keepalive_overflow_maps_to_none() {
        let mut proto: ProtoPeer = sample_peer().into();
        // A value exceeding u16::MAX can't be represented as a keepalive interval.
        proto.persistent_keepalive_interval = Some(u32::from(u16::MAX) + 1);
        let converted: Peer = proto.try_into().unwrap();
        assert_eq!(converted.persistent_keepalive_interval, None);
    }
}
