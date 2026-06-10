use std::{
    hint::spin_loop,
    net::IpAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, channel, Receiver, Sender},
        Arc, LazyLock, Mutex,
    },
};

use block2::RcBlock;
use defguard_client_common::dns_owned;
use defguard_wireguard_rs::{key::Key, net::IpAddrMask, peer::Peer};
use objc2::{rc::Retained, runtime::AnyObject};
use objc2_foundation::{
    ns_string, NSArray, NSDictionary, NSError, NSMutableArray, NSMutableDictionary, NSNumber,
    NSString,
};
use objc2_network_extension::{NETunnelProviderManager, NETunnelProviderProtocol, NEVPNStatus};

use crate::{
    database::{
        models::{
            instance::{ClientTrafficPolicy, Instance},
            location::Location,
            tunnel::Tunnel,
            wireguard_keys::WireguardKeys,
            Id,
        },
        DB_POOL,
    },
    error::Error,
    DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6,
};

pub const LOCATION_ID: &str = "locationId";
pub const TUNNEL_ID: &str = "tunnelId";

type ObserverSender = Mutex<Sender<(&'static str, Id)>>;
type ObserverReceiver = Mutex<Option<Receiver<(&'static str, Id)>>>;

pub static OBSERVER_COMMS: LazyLock<(ObserverSender, ObserverReceiver)> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel();
    (Mutex::new(tx), Mutex::new(Some(rx)))
});

pub const PLUGIN_BUNDLE_ID: &str = "net.defguard.VPNExtension";

/// Try to get `Id` out of manager. ID is embedded in configuration dictionary under `key`.
pub fn id_from_manager(manager: &NETunnelProviderManager, key: &NSString) -> Option<Id> {
    let plugin_bundle_id = ns_string!(PLUGIN_BUNDLE_ID);

    let vpn_protocol = (unsafe { manager.protocolConfiguration() })?;
    let Ok(tunnel_protocol) = vpn_protocol.downcast::<NETunnelProviderProtocol>() else {
        error!("Failed to downcast to NETunnelProviderProtocol");
        return None;
    };
    // Sometimes all managers from all apps come through, so filter by bundle ID.
    if let Some(bundle_id) = unsafe { tunnel_protocol.providerBundleIdentifier() } {
        if &*bundle_id != plugin_bundle_id {
            return None;
        }
    }

    if let Some(config_dict) = unsafe { tunnel_protocol.providerConfiguration() } {
        if let Some(any_object) = config_dict.objectForKey(key) {
            let Ok(id) = any_object.downcast::<NSNumber>() else {
                warn!("Failed to downcast ID to NSNumber");
                return None;
            };
            return Some(id.as_i64());
        }
    }

    None
}

/// Try to find [`NETunnelProviderManager`] in system settings that matches key and value.
/// Key is usually `locationId` or `tunnelId`.
pub fn manager_for_key_and_value(
    key: &str,
    value: Id,
) -> Option<Retained<NETunnelProviderManager>> {
    let key_string = NSString::from_str(key);
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
                if let Some(id) = id_from_manager(&manager, &key_string) {
                    if id == value {
                        // This is the manager we were looking for.
                        tx.send(Some(manager)).expect("Sender is dead");
                        return;
                    }
                }
            }

            tx.send(None).expect("Sender is dead");
        },
    );
    unsafe {
        NETunnelProviderManager::loadAllFromPreferencesWithCompletionHandler(&handler);
    }

    rx.recv().expect("Receiver is dead")
}

/// Tunnel configuration shared with VPNExtension (written in Swift).
pub struct TunnelConfiguration {
    location_id: Option<Id>,
    tunnel_id: Option<Id>,
    name: String,
    private_key: String,
    addresses: Vec<IpAddrMask>,
    listen_port: Option<u16>,
    peers: Vec<Peer>,
    mtu: Option<u32>,
    dns: Vec<IpAddr>,
    dns_search: Vec<String>,
}

impl TunnelConfiguration {
    /// Convert to [`NSDictionary`].
    fn as_nsdict(&self) -> Retained<NSDictionary<NSString, AnyObject>> {
        let dict = NSMutableDictionary::new();

        if let Some(location_id) = self.location_id {
            dict.insert(
                ns_string!(LOCATION_ID),
                NSNumber::new_i64(location_id).as_ref(),
            );
        }

        if let Some(tunnel_id) = self.tunnel_id {
            dict.insert(ns_string!(TUNNEL_ID), NSNumber::new_i64(tunnel_id).as_ref());
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
                ns_string!("publicKey"),
                NSString::from_str(&peer.public_key.to_string()).as_ref(),
            );

            if let Some(preshared_key) = &peer.preshared_key {
                peer_dict.insert(
                    ns_string!("preSharedKey"),
                    NSString::from_str(&preshared_key.to_string()).as_ref(),
                );
            }

            if let Some(endpoint) = &peer.endpoint {
                peer_dict.insert(
                    ns_string!("endpoint"),
                    NSString::from_str(&endpoint.to_string()).as_ref(),
                );
            }

            // Skipping: lastHandshake, txBytes, rxBytes.

            if let Some(persistent_keep_alive) = peer.persistent_keepalive_interval {
                peer_dict.insert(
                    ns_string!("persistentKeepAlive"),
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
            peer_dict.insert(ns_string!("allowedIPs"), allowed_ips.as_ref());

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

    /// Try to find `NETunnelProviderManager` for this configuration, based on location ID or
    /// tunnel ID.
    pub fn tunnel_provider_manager(&self) -> Option<Retained<NETunnelProviderManager>> {
        let (key, value) = match (self.location_id, self.tunnel_id) {
            (Some(location_id), None) => (LOCATION_ID, location_id),
            (None, Some(tunnel_id)) => (TUNNEL_ID, tunnel_id),
            _ => return None,
        };

        manager_for_key_and_value(key, value)
    }

    /// Create or update system VPN settings with this configuration.
    pub fn save(&self) {
        let spinlock = Arc::new(AtomicBool::new(false));
        let spinlock_clone = Arc::clone(&spinlock);
        let plugin_bundle_id = ns_string!(PLUGIN_BUNDLE_ID);

        let provider_manager = self
            .tunnel_provider_manager()
            .unwrap_or_else(|| unsafe { NETunnelProviderManager::new() });

        unsafe {
            let tunnel_protocol = NETunnelProviderProtocol::new();
            tunnel_protocol.setProviderBundleIdentifier(Some(plugin_bundle_id));
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

            // Save to system settings.
            let handler = RcBlock::new(move |error_ptr: *mut NSError| {
                if error_ptr.is_null() {
                    debug!("Saved tunnel configuration for {name} to system settings");
                } else {
                    error!("Failed to save tunnel configuration for: {name} to system settings");
                }
                spinlock_clone.store(true, Ordering::Release);
            });
            provider_manager.saveToPreferencesWithCompletionHandler(Some(&*handler));
        }

        while !spinlock.load(Ordering::Acquire) {
            spin_loop();
        }
    }

    /// Start tunnel for this configuration.
    pub fn start_tunnel(&self) {
        if let Some(provider_manager) = self.tunnel_provider_manager() {
            if let Err(err) =
                unsafe { provider_manager.connection().startVPNTunnelAndReturnError() }
            {
                error!("Failed to start VPN: {err}");
            } else {
                OBSERVER_COMMS
                    .0
                    .lock()
                    .expect("Failed to lock observer sender")
                    .send((
                        self.location_id
                            .map_or_else(|| TUNNEL_ID, |_location_id| LOCATION_ID),
                        self.location_id.or(self.tunnel_id).unwrap(),
                    ))
                    .expect("Failed to send to observer channel");
                info!("VPN started");
            }
        } else {
            debug!(
                "Couldn't find configuration from system settings for {}",
                self.name
            );
        }
    }
}

impl Location<Id> {
    /// Build [`TunnelConfiguration`] from [`Location`].
    pub async fn tunnel_configuration(
        &self,
        preshared_key: Option<String>,
        mtu: Option<u32>,
    ) -> Result<TunnelConfiguration, Error> {
        debug!("Looking for WireGuard keys for location {self} instance");
        let Some(keys) = WireguardKeys::find_by_instance_id(&*DB_POOL, self.instance_id).await?
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
        let Some(instance) = Instance::find_by_id(&*DB_POOL, self.instance_id).await? else {
            error!("Instance {} not found", self.instance_id);
            return Err(Error::InternalError(format!(
                "Instance {} not found",
                self.instance_id
            )));
        };
        let route_all_traffic = match instance.client_traffic_policy {
            ClientTrafficPolicy::ForceAllTraffic => true,
            ClientTrafficPolicy::DisableAllTraffic => false,
            ClientTrafficPolicy::None => self.route_all_traffic,
        };
        let allowed_ips = if route_all_traffic {
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
        let (dns, dns_search) = dns_owned(&self.dns);
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

    /// Check whether VPN tunnel is running for [`Location`].
    pub fn status(&self) -> Option<NEVPNStatus> {
        manager_for_key_and_value(LOCATION_ID, self.id).map_or_else(
            || {
                debug!(
                    "Couldn't find configuration in system settings for location {}",
                    self.name
                );
                None
            },
            |provider_manager| unsafe {
                let connection = provider_manager.connection();
                Some(connection.status())
            },
        )
    }

    /// Remove configuration from system settings for [`Location`].
    pub fn remove_config(&self) {
        if let Some(provider_manager) = manager_for_key_and_value(LOCATION_ID, self.id) {
            unsafe {
                provider_manager.removeFromPreferencesWithCompletionHandler(None);
            }
        } else {
            debug!(
                "Couldn't find configuration in system settings for location {}",
                self.name
            );
        }
    }

    /// Stop VPN tunnel for [`Location`].
    pub fn stop_vpn_tunnel(&self) -> bool {
        manager_for_key_and_value(LOCATION_ID, self.id).map_or_else(
            || {
                debug!(
                    "Couldn't find configuration in system settings for location {}",
                    self.name
                );
                false
            },
            |provider_manager| {
                unsafe {
                    provider_manager.connection().stopVPNTunnel();
                }
                info!("VPN stopped");
                true
            },
        )
    }
}

impl Tunnel<Id> {
    /// Build [`TunnelConfiguration`] from [`Tunnel`].
    pub fn tunnel_configuration(&self, mtu: Option<u32>) -> Result<TunnelConfiguration, Error> {
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
            let msg = self.allowed_ips.as_ref().map_or_else(
                || "No allowed IP addresses found in tunnel {self} configuration".to_string(),
                |ips| format!("Using predefined location traffic for tunnel {self}: {ips}"),
            );
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
        let (dns, dns_search) = dns_owned(&self.dns);
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

    /// Check whether VPN tunnel is running for [`Tunnel`].
    pub fn status(&self) -> Option<NEVPNStatus> {
        manager_for_key_and_value(TUNNEL_ID, self.id).map_or_else(
            || {
                debug!(
                    "Couldn't find configuration in system settings for tunnel {}",
                    self.name
                );
                None
            },
            |provider_manager| unsafe {
                let connection = provider_manager.connection();
                Some(connection.status())
            },
        )
    }

    /// Remove configuration from system settings for [`Tunnel`].
    pub fn remove_config(&self) {
        if let Some(provider_manager) = manager_for_key_and_value(TUNNEL_ID, self.id) {
            unsafe {
                provider_manager.removeFromPreferencesWithCompletionHandler(None);
            }
        } else {
            debug!(
                "Couldn't find configuration in system settings for tunnel {}",
                self.name
            );
        }
    }

    /// Stop tunnel for [`Tunnel`].
    pub fn stop_vpn_tunnel(&self) -> bool {
        manager_for_key_and_value(TUNNEL_ID, self.id).map_or_else(
            || {
                debug!(
                    "Couldn't find configuration in system settings for location {}",
                    self.name
                );
                false
            },
            |provider_manager| {
                unsafe {
                    provider_manager.connection().stopVPNTunnel();
                }
                info!("VPN stopped");
                true
            },
        )
    }
}
