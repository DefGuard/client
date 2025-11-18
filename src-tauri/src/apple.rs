//! Structures used for interchangeability with the Swift code.

use std::{
    hint::spin_loop,
    net::IpAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc,
    },
};

use block2::RcBlock;
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
use objc2::{rc::Retained, runtime::AnyObject};
use objc2_foundation::{
    ns_string, NSArray, NSDictionary, NSError, NSMutableArray, NSMutableDictionary, NSNumber,
    NSString,
};
use objc2_network_extension::{NETunnelProviderManager, NETunnelProviderProtocol};
use serde::Serialize;
use sqlx::SqliteExecutor;

use crate::{
    database::models::{location::Location, tunnel::Tunnel, wireguard_keys::WireguardKeys, Id},
    error::Error,
    utils::{DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6},
};

// const BUNDLE_ID: &str = "net.defguard";
const PLUGIN_BUNDLE_ID: &str = "net.defguard.VPNExtension";

// Should match the declaration in Swift.
#[repr(C)]
pub(crate) struct Stats {
    pub(crate) location_id: Option<Id>,
    pub(crate) tunnel_id: Option<Id>,
    pub(crate) tx_bytes: u64,
    pub(crate) rx_bytes: u64,
    pub(crate) last_handshake: u64,
}

/// Find `NETunnelProviderManager` in system preferences.
fn manager_for_name(name: &str) -> Option<Retained<NETunnelProviderManager>> {
    let name_string = NSString::from_str(name);
    let plugin_bundle_id = NSString::from_str(PLUGIN_BUNDLE_ID);
    let (tx, rx) = channel();

    let handler = RcBlock::new(
        move |managers_ptr: *mut NSArray<NETunnelProviderManager>, error_ptr: *mut NSError| {
            if !error_ptr.is_null() {
                error!("Failed to load tunnel provider managers.");
                return;
            }

            let Some(managers) = (unsafe { managers_ptr.as_ref() }) else {
                error!("No managers");
                return;
            };

            for manager in managers {
                let Some(vpn_protocol) = (unsafe { manager.protocolConfiguration() }) else {
                    continue;
                };
                let Ok(tunnel_protocol) = vpn_protocol.downcast::<NETunnelProviderProtocol>()
                else {
                    error!("Failed to downcast to NETunnelProviderProtocol");
                    continue;
                };
                // Sometimes all managers from all apps come through, so filter by bundle ID.
                if let Some(bundle_id) = unsafe { tunnel_protocol.providerBundleIdentifier() } {
                    if bundle_id != plugin_bundle_id {
                        continue;
                    }
                }
                if let Some(descr) = unsafe { manager.localizedDescription() } {
                    error!("Descripion {descr}");
                    if descr == name_string {
                        tx.send(Some(manager)).unwrap();
                        return;
                    }
                }
            }

            tx.send(None).unwrap();
        },
    );
    unsafe {
        NETunnelProviderManager::loadAllFromPreferencesWithCompletionHandler(&handler);
    }

    rx.recv().unwrap()
}

#[derive(Serialize)]
pub(crate) struct TunnelConfiguration {
    #[serde(rename = "locationId")]
    location_id: Option<Id>,
    #[serde(rename = "tunnelId")]
    tunnel_id: Option<Id>,
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

impl TunnelConfiguration {
    /// Convert to `NSDictionary`.
    fn as_nsdict(&self) -> Retained<NSDictionary<NSString, AnyObject>> {
        let dict = NSMutableDictionary::new();

        if let Some(location_id) = self.location_id {
            dict.insert(
                ns_string!("locationId"),
                NSNumber::new_i64(location_id).as_ref(),
            );
        }

        if let Some(tunnel_id) = self.tunnel_id {
            dict.insert(
                ns_string!("tunnelId"),
                NSNumber::new_i64(tunnel_id).as_ref(),
            );
        }

        dict.insert(ns_string!("name"), NSString::from_str(&self.name).as_ref());

        dict.insert(
            ns_string!("privateKey"),
            NSString::from_str(&self.private_key).as_ref(),
        );

        // IpAddrMask
        let addresses = NSMutableArray::<NSDictionary<NSString, AnyObject>>::new();
        for addr in &self.addresses {
            let addr_dict = NSMutableDictionary::<NSString, AnyObject>::new();
            addr_dict.insert(
                ns_string!("address"),
                NSString::from_str(&addr.address.to_string()).as_ref(),
            );
            addr_dict.insert(ns_string!("cidr"), NSNumber::new_u8(addr.cidr).as_ref());
            addresses.addObject(addr_dict.into_super().as_ref());
        }
        dict.insert(ns_string!("addresses"), addresses.as_ref());

        if let Some(listen_port) = self.listen_port {
            dict.insert(
                ns_string!("listenPort"),
                NSNumber::new_u16(listen_port).as_ref(),
            );
        }

        // Peer
        let peers = NSMutableArray::<NSDictionary<NSString, AnyObject>>::new();
        for peer in &self.peers {
            let peer_dict = NSMutableDictionary::<NSString, AnyObject>::new();
            peer_dict.insert(
                ns_string!("public_key"),
                NSString::from_str(&peer.public_key.to_string()).as_ref(),
            );

            if let Some(preshared_key) = &peer.preshared_key {
                peer_dict.insert(
                    ns_string!("preshared_key"),
                    NSString::from_str(&preshared_key.to_string()).as_ref(),
                );
            }

            if let Some(endpoint) = &peer.endpoint {
                peer_dict.insert(
                    ns_string!("endpoint"),
                    NSString::from_str(&endpoint.to_string()).as_ref(),
                );
            }

            // Skipping: last_handshake

            peer_dict.insert(
                ns_string!("tx_bytes"),
                NSNumber::new_u64(peer.tx_bytes).as_ref(),
            );
            peer_dict.insert(
                ns_string!("rx_bytes"),
                NSNumber::new_u64(peer.rx_bytes).as_ref(),
            );

            if let Some(persistent_keep_alive) = peer.persistent_keepalive_interval {
                peer_dict.insert(
                    ns_string!("persistent_keepalive_interval"),
                    NSNumber::new_u16(persistent_keep_alive).as_ref(),
                );
            }

            // IpAddrMask
            let allowed_ips = NSMutableArray::<NSDictionary<NSString, AnyObject>>::new();
            for addr in &peer.allowed_ips {
                let addr_dict = NSMutableDictionary::<NSString, AnyObject>::new();
                addr_dict.insert(
                    ns_string!("address"),
                    NSString::from_str(&addr.address.to_string()).as_ref(),
                );
                addr_dict.insert(ns_string!("cidr"), NSNumber::new_u8(addr.cidr).as_ref());
                allowed_ips.addObject(addr_dict.into_super().as_ref());
            }
            peer_dict.insert(ns_string!("allowed_ips"), allowed_ips.as_ref());

            peers.addObject(peer_dict.into_super().as_ref());
        }
        dict.insert(ns_string!("peers"), peers.into_super().as_ref());

        if let Some(mtu) = self.mtu {
            dict.insert(ns_string!("mtu"), NSNumber::new_u32(mtu).as_ref());
        }

        let dns = NSMutableArray::<NSString>::new();
        for entry in &self.dns {
            dns.addObject(NSString::from_str(&entry.to_string()).as_ref());
        }
        dict.insert(ns_string!("dns"), dns.as_ref());

        let dns_search = NSMutableArray::<NSString>::new();
        for entry in &self.dns_search {
            dns_search.addObject(NSString::from_str(entry).as_ref());
        }
        dict.insert(ns_string!("dnsSearch"), dns_search.as_ref());

        dict.into_super()
    }

    /// Create or update system VPN settings with this configuration.
    pub(crate) fn save(&self) {
        unsafe {
            let provider_manager =
                manager_for_name(&self.name).unwrap_or_else(|| NETunnelProviderManager::new());

            let tunnel_protocol = NETunnelProviderProtocol::new();
            let plugin_bundle_id = NSString::from_str(PLUGIN_BUNDLE_ID);
            tunnel_protocol.setProviderBundleIdentifier(Some(&plugin_bundle_id));
            let server_address = self.peers.first().map_or(String::new(), |peer| {
                peer.endpoint.map_or(String::new(), |sa| sa.to_string())
            });
            let server_address = NSString::from_str(&server_address);
            // `serverAddress` must have a non-nil string value for the protocol configuration to be
            // valid.
            tunnel_protocol.setServerAddress(Some(&server_address));

            let provider_config = self.as_nsdict();
            tunnel_protocol.setProviderConfiguration(Some(&*provider_config));

            provider_manager.setProtocolConfiguration(Some(&tunnel_protocol));
            let name = NSString::from_str(&self.name);
            provider_manager.setLocalizedDescription(Some(&name));
            provider_manager.setEnabled(true);

            // Save to preferences.
            let spinlock = Arc::new(AtomicBool::new(false));
            let spinlock_clone = Arc::clone(&spinlock);
            let name = self.name.clone();
            let handler = RcBlock::new(move |error_ptr: *mut NSError| {
                if error_ptr.is_null() {
                    info!("Saved tunnel configuration for {name}");
                } else {
                    error!("Failed to save tunnel configuration for: {name}");
                }
                spinlock_clone.store(true, Ordering::Release);
            });
            provider_manager.saveToPreferencesWithCompletionHandler(Some(&*handler));
            while !spinlock.load(Ordering::Acquire) {
                spin_loop();
            }
        }
    }
}

/// IMPORTANT: This is currently for testing. Assume the config has been saved.
pub(crate) fn start_tunnel(name: &str) {
    if let Some(provider_manager) = manager_for_name(name) {
        if let Err(err) = unsafe { provider_manager.connection().startVPNTunnelAndReturnError() } {
            error!("Failed to start VPN: {err}");
        } else {
            info!("VPN started");
        }
    } else {
        error!("Couldn't find configuration from preferences for {name}");
    }
}

/// IMPORTANT: This is currently for testing. Assume the config has been saved.
pub(crate) fn stop_tunnel(name: &str) -> bool {
    if let Some(provider_manager) = manager_for_name(name) {
        unsafe {
            provider_manager.connection().stopVPNTunnel();
        }
        info!("VPN stopped");
        true
    } else {
        error!("Couldn't find configuration from preferences for {name}");
        false
    }
}

/// IMPORTANT: This is currently for testing. Assume the config has been saved.
pub(crate) fn all_tunnel_stats() -> Vec<Stats> {
    Vec::<Stats>::new()
}

impl Location<Id> {
    pub(crate) async fn tunnel_configurarion<'e, E>(
        &self,
        executor: E,
        preshared_key: Option<String>,
        dns: Vec<IpAddr>,
        dns_search: Vec<String>,
        mtu: Option<u32>,
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
        Ok(TunnelConfiguration {
            location_id: Some(self.id),
            tunnel_id: None,
            name: self.name.clone(),
            private_key: keys.prvkey,
            addresses,
            listen_port: Some(0),
            peers: vec![peer],
            mtu,
            dns,
            dns_search,
        })
    }
}

impl Tunnel<Id> {
    pub(crate) async fn tunnel_configurarion<'e, E>(
        &self,
        executor: E,
        dns: Vec<IpAddr>,
        dns_search: Vec<String>,
        mtu: Option<u32>,
    ) -> Result<TunnelConfiguration, Error>
    where
        E: SqliteExecutor<'e>,
    {
        // prepare peer config
        debug!("Decoding tunnel {self} public key: {}.", self.server_pubkey);
        let peer_key = Key::from_str(&self.server_pubkey)?;
        debug!("Tunnel {self} public key decoded.");
        let mut peer = Peer::new(peer_key);

        debug!("Parsing tunnel {self} endpoint: {}", self.endpoint);
        peer.set_endpoint(&self.endpoint)?;
        peer.persistent_keepalive_interval = Some(
            self.persistent_keep_alive
                .try_into()
                .expect("Failed to parse persistent keep alive"),
        );
        debug!("Parsed tunnel {self} endpoint: {}", self.endpoint);

        if let Some(psk) = &self.preshared_key {
            debug!("Decoding tunnel {self} preshared key.");
            let peer_psk = Key::from_str(psk)?;
            debug!("Preshared key for tunnel {self} decoded.");
            peer.preshared_key = Some(peer_psk);
        }

        debug!("Parsing tunnel {self} allowed ips: {:?}", self.allowed_ips);
        let allowed_ips = if self.route_all_traffic {
            debug!("Using all traffic routing for tunnel {self}");
            vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
        } else {
            let msg = match &self.allowed_ips {
                Some(ips) => {
                    format!("Using predefined location traffic for tunnel {self}: {ips}")
                }
                None => "No allowed IP addresses found in tunnel {self} configuration".to_string(),
            };
            debug!("{msg}");
            self.allowed_ips
                .as_ref()
                .map(|ips| ips.split(',').map(str::to_string).collect())
                .unwrap_or_default()
        };
        for allowed_ip in &allowed_ips {
            match IpAddrMask::from_str(allowed_ip.trim()) {
                Ok(addr) => {
                    peer.allowed_ips.push(addr);
                }
                Err(err) => {
                    // Handle the error from IpAddrMask::from_str, if needed
                    error!("Error parsing IP address {allowed_ip}: {err}");
                    // Continue to the next iteration of the loop
                }
            }
        }
        debug!("Parsed tunnel {self} allowed IPs: {:?}", peer.allowed_ips);

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
        Ok(TunnelConfiguration {
            location_id: None,
            tunnel_id: Some(self.id),
            name: self.name.clone(),
            private_key: self.prvkey.clone(),
            addresses,
            listen_port: Some(0),
            peers: vec![peer],
            mtu,
            dns,
            dns_search,
        })
    }
}
