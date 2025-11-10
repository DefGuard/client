//! Structures used for interchangeability with the Swift code.

use std::{net::IpAddr, str::FromStr};

use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
use serde::Serialize;
use sqlx::SqliteExecutor;
use swift_rs::{swift, SRObject, SRObjectArray, SRString};

#[repr(C)]
// Should match the declaration in Swift.
pub(crate) struct Stats {
    pub(crate) tx_bytes: u64,
    pub(crate) rx_bytes: u64,
    pub(crate) last_handshake: u64,
}

swift!(pub(crate) fn start_tunnel(json: &SRString) -> bool);
swift!(pub(crate) fn stop_tunnel(name: &SRString) -> bool);
swift!(pub(crate) fn tunnel_stats(name: &SRString) -> Option<SRObject<Stats>>);
swift!(pub(crate) fn all_tunnel_stats() -> SRObjectArray<Stats>);

use crate::{
    database::models::{location::Location, wireguard_keys::WireguardKeys, Id},
    error::Error,
    utils::{DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6},
};

#[derive(Serialize)]
pub(crate) struct TunnelConfiguration {
    name: String,
    #[serde(rename = "privateKey")]
    private_key: String,
    addresses: Vec<IpAddrMask>,
    #[serde(rename = "listenPort")]
    listen_port: Option<u16>,
    peers: Vec<Peer>,
    mtu: Option<u32>,
    dns: Vec<IpAddr>,
    #[serde(rename = "dnsSearch")]
    dns_search: Vec<String>,
}

impl Location<Id> {
    pub(crate) async fn tunnel_configurarion<'e, E>(
        &self,
        executor: E,
        preshared_key: Option<String>,
        dns: Vec<IpAddr>,
        dns_search: Vec<String>,
    ) -> Result<TunnelConfiguration, Error>
    where
        E: SqliteExecutor<'e>,
    {
        debug!("Looking for WireGuard keys for location {self} instance");
        let Some(keys) = WireguardKeys::find_by_instance_id(executor, self.instance_id).await?
        else {
            error!("No keys found for instance: {}", self.instance_id);
            return Err(Error::InternalError(
                "No keys found for instance".to_string(),
            ));
        };
        debug!("WireGuard keys found for location {self} instance");

        // prepare peer config
        debug!("Decoding location {self} public key: {}.", self.pubkey);
        let peer_key = Key::from_str(&self.pubkey)?;
        debug!("Location {self} public key decoded: {peer_key}");
        let mut peer = Peer::new(peer_key);

        debug!("Parsing location {self} endpoint: {}", self.endpoint);
        peer.set_endpoint(&self.endpoint)?;
        peer.persistent_keepalive_interval = Some(25);
        debug!("Parsed location {self} endpoint: {}", self.endpoint);

        if let Some(psk) = preshared_key {
            debug!("Decoding location {self} preshared key.");
            let peer_psk = Key::from_str(&psk)?;
            info!("Location {self} preshared key decoded.");
            peer.preshared_key = Some(peer_psk);
        }

        debug!("Parsing location {self} allowed IPs: {}", self.allowed_ips);
        let allowed_ips = if self.route_all_traffic {
            debug!("Using all traffic routing for location {self}");
            vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
        } else {
            debug!(
                "Using predefined location {self} traffic: {}",
                self.allowed_ips
            );
            self.allowed_ips.split(',').map(str::to_string).collect()
        };
        for allowed_ip in &allowed_ips {
            match IpAddrMask::from_str(allowed_ip) {
                Ok(addr) => {
                    peer.allowed_ips.push(addr);
                }
                Err(err) => {
                    // Handle the error from IpAddrMask::from_str, if needed
                    error!(
                        "Error parsing IP address {allowed_ip} while setting up interface for \
                        location {self}, error details: {err}"
                    );
                }
            }
        }
        debug!(
            "Parsed allowed IPs for location {self}: {:?}",
            peer.allowed_ips
        );

        let addresses = self
            .address
            .split(',')
            .map(str::trim)
            .map(IpAddrMask::from_str)
            .collect::<Result<_, _>>()
            .map_err(|err| {
                let msg = format!("Failed to parse IP addresses '{}': {err}", self.address);
                error!("{msg}");
                Error::InternalError(msg)
            })?;
        let interface_config = TunnelConfiguration {
            name: self.name.clone(),
            private_key: keys.prvkey,
            addresses,
            listen_port: Some(0),
            peers: vec![peer],
            mtu: None,
            dns,
            dns_search,
        };

        Ok(interface_config)
    }
}
