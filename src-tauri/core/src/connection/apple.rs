//! Interchangeability and communication with VPNExtension (written in Swift).

use std::{
    collections::HashMap,
    hint::spin_loop,
    net::IpAddr,
    ptr::NonNull,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, channel, Receiver, RecvTimeoutError, Sender},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};

const OBSERVER_CLEANUP_INTERVAL: Duration = Duration::from_secs(30);

use block2::RcBlock;
use defguard_client_common::dns_owned;
use defguard_wireguard_rs::{key::Key, net::IpAddrMask, peer::Peer};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_foundation::{
    ns_string, NSArray, NSData, NSDate, NSDictionary, NSError, NSMutableArray, NSMutableDictionary,
    NSNotification, NSNotificationCenter, NSNumber, NSObjectProtocol, NSOperationQueue, NSRunLoop,
    NSString,
};
use objc2_network_extension::{
    NETunnelProviderManager, NETunnelProviderProtocol, NETunnelProviderSession, NEVPNConnection,
    NEVPNStatus, NEVPNStatusDidChangeNotification,
};

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
    ConnectionType, DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6,
};

const PLUGIN_BUNDLE_ID: &str = "net.defguard.VPNExtension";
const LOCATION_ID: &str = "locationId";
const TUNNEL_ID: &str = "tunnelId";

type ObserverSender = Mutex<Sender<(&'static str, Id)>>;
type ObserverReceiver = Mutex<Option<Receiver<(&'static str, Id)>>>;

static OBSERVER_COMMS: LazyLock<(ObserverSender, ObserverReceiver)> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel();
    (Mutex::new(tx), Mutex::new(Some(rx)))
});

type VpnStateSender = Mutex<Sender<()>>;
type VpnStateReceiver = Mutex<Option<Receiver<()>>>;

static VPN_STATE_UPDATE_COMMS: LazyLock<(VpnStateSender, VpnStateReceiver)> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel();
    (Mutex::new(tx), Mutex::new(Some(rx)))
});

/// Thread responsible for observing VPN status changes.
/// This is intentionally a blocking function, as it uses the Objective-C objects which are not
/// thread safe.
pub fn observer_thread(
    initial_managers: HashMap<(&'static str, Id), Retained<NETunnelProviderManager>>,
) {
    debug!("Starting VPN connection observer thread");
    let receiver = {
        let mut rx_opt = OBSERVER_COMMS
            .1
            .lock()
            .expect("Failed to lock observer receiver");
        rx_opt.take().expect("Receiver already taken")
    };

    let mut observers = HashMap::new();

    // spawn initial observers for existing managers
    for ((key, value), manager) in initial_managers {
        debug!("Spawning initial observer for manager with key: {key}, value: {value}");
        let connection = unsafe { manager.connection() };
        let observer = create_observer(&connection);
        debug!("Registered initial observer for manager with key: {key}, value: {value}");
        observers.insert((key, value), observer);
    }

    loop {
        match receiver.recv_timeout(OBSERVER_CLEANUP_INTERVAL) {
            Ok(message) => {
                debug!("Received message to observe the following connection: {message:?}");

                let (key, value) = message;

                if observers.contains_key(&(key, value)) {
                    debug!(
                        "Observer for manager with key: {key}, value: {value} already exists,
                        skipping",
                    );
                    continue;
                }

                let manager = manager_for_key_and_value(key, value).unwrap();
                let connection = unsafe { manager.connection() };
                let observer = create_observer(&connection);

                observers.insert((key, value), observer);
                debug!("Registered observer for manager with key: {key}, value: {value}");
            }
            Err(RecvTimeoutError::Timeout) => {
                debug!("Performing periodic cleanup of dead observers");
                let mut dead_keys = Vec::new();

                for (key, value) in observers.keys() {
                    if manager_for_key_and_value(key, *value).is_none() {
                        debug!(
                            "Manager for key: {key}, value: {value} no longer exists, marking for
                            removal"
                        );
                        dead_keys.push((*key, *value));
                    }
                }

                for dead_key in dead_keys {
                    if let Some(_observer) = observers.remove(&dead_key) {
                        debug!(
                            "Removed dead VPN connection observer for key: {}, value: {}",
                            dead_key.0, dead_key.1
                        );
                    }
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                error!("Observer receiver channel disconnected, exiting observer thread");
                break;
            }
        }
    }

    debug!("Exiting VPN connection observer thread");
}

/// Tunnel statistics shared with VPNExtension (written in Swift).
/// Run [`NSRunLoop`] until semaphore becomes `true`.
pub fn spawn_runloop_and_wait_for(semaphore: &Arc<AtomicBool>) {
    const ONE_SECOND: f64 = 1.;
    let run_loop = NSRunLoop::currentRunLoop();
    let mut date = NSDate::dateWithTimeIntervalSinceNow(ONE_SECOND);
    loop {
        run_loop.runUntilDate(&date);
        if semaphore.load(Ordering::Acquire) {
            break;
        }
        date = date.dateByAddingTimeInterval(ONE_SECOND);
    }
}

/// Handle VPN status change.
fn vpn_status_change_handler(notification: &NSNotification) {
    let name = notification.name();
    debug!("Received VPN status change notification: {name:?}");
    VPN_STATE_UPDATE_COMMS
        .0
        .lock()
        .expect("Failed to lock state update sender")
        .send(())
        .expect("Failed to send to state update channel");
    debug!("Sent status update request to channel");
}

/// Observe VPN status change.
fn create_observer(object: &NEVPNConnection) -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    let center = NSNotificationCenter::defaultCenter();
    let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
        vpn_status_change_handler(unsafe { notification.as_ref() });
    });
    let queue = NSOperationQueue::mainQueue();
    unsafe {
        let name = NEVPNStatusDidChangeNotification;
        center.addObserverForName_object_queue_usingBlock(
            Some(name),
            Some(object),
            Some(&queue),
            &block,
        )
    }
}

#[must_use]
pub fn get_managers_for_tunnels_and_locations(
    tunnels: &[Tunnel<Id>],
    locations: &[Location<Id>],
) -> HashMap<(&'static str, Id), Retained<NETunnelProviderManager>> {
    let mut managers = HashMap::new();

    for location in locations {
        if let Some(manager) = manager_for_key_and_value(LOCATION_ID, location.id) {
            managers.insert((LOCATION_ID, location.id), manager);
        }
    }

    for tunnel in tunnels {
        if let Some(manager) = manager_for_key_and_value(TUNNEL_ID, tunnel.id) {
            managers.insert((TUNNEL_ID, tunnel.id), manager);
        }
    }

    managers
}

/// Try to get `Id` out of manager. ID is embedded in configuration dictionary under `key`.
fn id_from_manager(manager: &NETunnelProviderManager, key: &NSString) -> Option<Id> {
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
fn manager_for_key_and_value(key: &str, value: Id) -> Option<Retained<NETunnelProviderManager>> {
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
pub(crate) struct TunnelConfiguration {
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

            if let Some(persistent_keep_alive) = peer.persistent_keepalive_interval {
                peer_dict.insert(
                    ns_string!("persistentKeepAlive"),
                    NSNumber::new_u16(persistent_keep_alive).as_ref(),
                );
            }

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

    pub(crate) fn tunnel_provider_manager(&self) -> Option<Retained<NETunnelProviderManager>> {
        let (key, value) = match (self.location_id, self.tunnel_id) {
            (Some(location_id), None) => (LOCATION_ID, location_id),
            (None, Some(tunnel_id)) => (TUNNEL_ID, tunnel_id),
            _ => return None,
        };

        manager_for_key_and_value(key, value)
    }

    pub(crate) fn save(&self) {
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
            tunnel_protocol.setServerAddress(Some(&server_address));

            let provider_config = self.as_nsdict();
            tunnel_protocol.setProviderConfiguration(Some(&*provider_config));

            provider_manager.setProtocolConfiguration(Some(&tunnel_protocol));
            let name = NSString::from_str(&self.name);
            provider_manager.setLocalizedDescription(Some(&name));
            provider_manager.setEnabled(true);

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

    pub(crate) fn start_tunnel(&self) {
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
    pub(crate) async fn tunnel_configuration(
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

    pub(crate) fn status(&self) -> Option<NEVPNStatus> {
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

    pub(crate) fn remove_config(&self) {
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

    pub(crate) fn stop_vpn_tunnel(&self) -> bool {
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
    pub(crate) fn tunnel_configuration(
        &self,
        mtu: Option<u32>,
    ) -> Result<TunnelConfiguration, Error> {
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
            debug!("Using predefined location traffic for tunnel {self}");
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
                    error!("Error parsing IP address {allowed_ip}: {err}");
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

    pub(crate) fn status(&self) -> Option<NEVPNStatus> {
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

    pub(crate) fn remove_config(&self) {
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

    pub(crate) fn stop_vpn_tunnel(&self) -> bool {
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

/// Synchronize locations and tunnels with system settings.
pub async fn sync_locations_and_tunnels(mtu: Option<u32>) -> Result<(), sqlx::Error> {
    // Update location settings.
    let all_locations = Location::all(&*DB_POOL, false).await?;
    for location in &all_locations {
        // For syncing, set `preshred_key` to `None`.
        let Ok(tunnel_config) = location.tunnel_configuration(None, mtu).await else {
            error!(
                "Failed to convert location {} to tunnel configuration.",
                location.name
            );
            continue;
        };
        tunnel_config.save();
    }

    // Update tunnel settings.
    let all_tunnels = Tunnel::all(&*DB_POOL).await?;
    for tunnel in &all_tunnels {
        let Ok(tunnel_config) = tunnel.tunnel_configuration(mtu) else {
            error!(
                "Failed to convert tunnel {} to tunnel configuration.",
                tunnel.name
            );
            continue;
        };
        tunnel_config.save();
    }

    debug!("Saved all configurations with system settings.");

    // Convert to Vec<Id>.
    let mut all_location_ids = all_locations
        .into_iter()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    let mut all_tunnel_ids = all_tunnels
        .into_iter()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    // For faster lookup using binary search (see below).
    all_location_ids.sort_unstable();
    all_tunnel_ids.sort_unstable();

    let spinlock = Arc::new(AtomicBool::new(false));
    let spinlock_clone = Arc::clone(&spinlock);
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

            let location_key = NSString::from_str(LOCATION_ID);
            let tunnel_key = NSString::from_str(TUNNEL_ID);
            for manager in managers {
                if let Some(id) = id_from_manager(&manager, &location_key) {
                    if all_location_ids.binary_search(&id).is_ok() {
                        // Known location - skip.
                        continue;
                    }
                }
                if let Some(id) = id_from_manager(&manager, &tunnel_key) {
                    if all_tunnel_ids.binary_search(&id).is_ok() {
                        // Known tunnel - skip.
                        continue;
                    }
                }
                unsafe { manager.removeFromPreferencesWithCompletionHandler(None) };
            }

            spinlock_clone.store(true, Ordering::Release);
        },
    );
    unsafe {
        NETunnelProviderManager::loadAllFromPreferencesWithCompletionHandler(&handler);
    }

    while !spinlock.load(Ordering::Acquire) {
        spin_loop();
    }

    debug!("Removed unknown configurations from system settings.");

    Ok(())
}
