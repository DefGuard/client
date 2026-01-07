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

use block2::RcBlock;
use common::dns_owned;
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_foundation::{
    ns_string, NSArray, NSData, NSDate, NSDictionary, NSError, NSMutableArray, NSMutableDictionary,
    NSNotification, NSNotificationCenter, NSNotificationName, NSNumber, NSObjectProtocol,
    NSOperationQueue, NSRunLoop, NSString,
};
use objc2_network_extension::{
    NETunnelProviderManager, NETunnelProviderProtocol, NETunnelProviderSession, NEVPNStatus,
};
use serde::Deserialize;
use sqlx::SqliteExecutor;
use tauri::{AppHandle, Emitter, Manager};
use tracing::Level;

use crate::{
    active_connections::find_connection,
    appstate::AppState,
    database::{
        models::{location::Location, tunnel::Tunnel, wireguard_keys::WireguardKeys, Id},
        DB_POOL,
    },
    error::Error,
    events::EventKey,
    log_watcher::service_log_watcher::spawn_log_watcher_task,
    tray::show_main_window,
    utils::{DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6},
    ConnectionType,
};

const PLUGIN_BUNDLE_ID: &str = "net.defguard.VPNExtension";
const SYSTEM_SYNC_DELAY_MS: u64 = 500;
const LOCATION_ID: &str = "locationId";
const TUNNEL_ID: &str = "tunnelId";

static OBSERVER_COMMS: LazyLock<(
    Mutex<Sender<(String, Id)>>,
    Mutex<Option<Receiver<(String, Id)>>>,
)> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel();
    (Mutex::new(tx), Mutex::new(Some(rx)))
});
static VPN_STATE_UPDATE_COMMS: LazyLock<(Mutex<Sender<()>>, Mutex<Option<Receiver<()>>>)> =
    LazyLock::new(|| {
        let (tx, rx) = mpsc::channel();
        (Mutex::new(tx), Mutex::new(Some(rx)))
    });

/// Thread responsible for handling VPN status update requests.
/// This is an async function.
/// It has access to the `AppHandle` to be able to emit events.
pub async fn connection_state_update_thread(app_handle: &AppHandle) {
    let receiver = {
        let mut rx_opt = VPN_STATE_UPDATE_COMMS
            .1
            .lock()
            .expect("Failed to lock state update receiver");
        rx_opt.take().expect("Receiver already taken")
    };

    debug!("Waiting for status update message from channel...");
    while receiver.recv().is_ok() {
        debug!("Status update message received, synchronizing state...");
        tokio::time::sleep(Duration::from_millis(SYSTEM_SYNC_DELAY_MS)).await;
        sync_connections_with_system(app_handle).await;
        debug!("Processed status update message.");
    }
}

/// Synchronize the app's connection state with the system's VPN state.
/// This checks all locations and tunnels and updates the app state to match
/// what's actually running in the system.
pub async fn sync_connections_with_system(app_handle: &AppHandle) {
    let pool = DB_POOL.clone();
    let app_state = app_handle.state::<AppState>();

    if let Ok(locations) = Location::all(&pool, false).await {
        for location in locations {
            debug!(
                "Synchronizing VPN status for location with system status: {}. Querying status...",
                location.name
            );
            let status = get_location_status(&location);
            debug!(
                "Location {} (ID {}) status: {status:?}",
                location.name, location.id
            );

            match status {
                Some(NEVPNStatus::Connected) => {
                    debug!("Location {} is connected", location.name);
                    if find_connection(location.id, crate::ConnectionType::Location)
                        .await
                        .is_some()
                    {
                        debug!(
                            "Location {} has already a connected state, skipping synchronization",
                            location.name
                        );
                    } else {
                        // Check if location requires MFA - if so, we need to cancel this connection
                        // and trigger MFA flow through the app
                        if location.mfa_enabled() {
                            info!(
                                "Location {} requires MFA but was started from system settings, \
                                canceling system connection and triggering MFA flow",
                                location.name
                            );
                            stop_tunnel_for_location(&location);
                            show_main_window(app_handle);
                            let _ = app_handle.emit(EventKey::MfaTrigger.into(), &location);
                            continue;
                        }

                        debug!("Adding connection for location {}", location.name);

                        app_state
                            .add_connection(
                                location.id,
                                &location.name,
                                crate::ConnectionType::Location,
                            )
                            .await;
                        app_handle
                            .emit(EventKey::ConnectionChanged.into(), ())
                            .unwrap();

                        debug!(
                            "Spawning log watcher for location {} (started from system settings)",
                            location.name
                        );
                        if let Err(e) = spawn_log_watcher_task(
                            app_handle.clone(),
                            location.id,
                            location.name.clone(),
                            ConnectionType::Location,
                            Level::DEBUG,
                            None,
                        )
                        .await
                        {
                            warn!(
                                "Failed to spawn log watcher for location {}: {e}",
                                location.name
                            );
                        }
                    }
                }
                Some(NEVPNStatus::Disconnected) => {
                    debug!("Location {} is disconnected", location.name);
                    if find_connection(location.id, crate::ConnectionType::Location)
                        .await
                        .is_some()
                    {
                        debug!("Removing connection for location {}", location.name);
                        app_state
                            .remove_connection(location.id, crate::ConnectionType::Location)
                            .await;
                        app_handle
                            .emit(EventKey::ConnectionChanged.into(), ())
                            .unwrap();
                    } else {
                        debug!(
                            "Location {} has no active connection, skipping removal",
                            location.name
                        );
                    }
                }
                Some(unknown_status) => {
                    debug!(
                    "Location {} has unknown status {unknown_status:?}, skipping synchronization",
                    location.name
                );
                }
                None => {
                    debug!(
                        "Couldn't find configuration for tunnel {}, skipping synchronization",
                        location.name
                    );
                }
            }
        }
    }

    if let Ok(tunnels) = Tunnel::all(&pool).await {
        for tunnel in tunnels {
            debug!(
                "Synchronizing VPN status for tunnel with system status: {}. Querying status...",
                tunnel.name
            );
            let status = get_tunnel_status(&tunnel);
            debug!(
                "Location {} (ID {}) status: {status:?}",
                tunnel.name, tunnel.id
            );

            match status {
                Some(NEVPNStatus::Connected) => {
                    debug!("Location {} is connected", tunnel.name);
                    if find_connection(tunnel.id, crate::ConnectionType::Tunnel)
                        .await
                        .is_some()
                    {
                        debug!(
                            "Location {} has already a connected state, skipping synchronization",
                            tunnel.name
                        );
                    } else {
                        debug!("Adding connection for location {}", tunnel.name);

                        app_state
                            .add_connection(tunnel.id, &tunnel.name, crate::ConnectionType::Tunnel)
                            .await;

                        app_handle
                            .emit(EventKey::ConnectionChanged.into(), ())
                            .unwrap();

                        // Spawn log watcher for this tunnel (VPN was started from system settings)
                        debug!(
                            "Spawning log watcher for tunnel {} (started from system settings)",
                            tunnel.name
                        );
                        if let Err(e) = spawn_log_watcher_task(
                            app_handle.clone(),
                            tunnel.id,
                            tunnel.name.clone(),
                            ConnectionType::Tunnel,
                            Level::DEBUG,
                            None,
                        )
                        .await
                        {
                            warn!(
                                "Failed to spawn log watcher for tunnel {}: {e}",
                                tunnel.name
                            );
                        }
                    }
                }
                Some(NEVPNStatus::Disconnected) => {
                    debug!("Location {} is disconnected", tunnel.name);
                    if find_connection(tunnel.id, crate::ConnectionType::Tunnel)
                        .await
                        .is_some()
                    {
                        debug!("Removing connection for location {}", tunnel.name);
                        app_state
                            .remove_connection(tunnel.id, crate::ConnectionType::Tunnel)
                            .await;
                        app_handle
                            .emit(EventKey::ConnectionChanged.into(), ())
                            .unwrap();
                    } else {
                        debug!(
                            "Location {} has no active connection, skipping removal",
                            tunnel.name
                        );
                    }
                }
                Some(unknown_status) => {
                    debug!(
                        "Location {} has unknown status {:?}, skipping synchronization",
                        tunnel.name, unknown_status
                    );
                }
                None => {
                    debug!(
                        "Couldn't find configuration for tunnel {}, skipping synchronization",
                        tunnel.name
                    );
                }
            }
        }
    }
}

const OBSERVER_CLEANUP_INTERVAL: Duration = Duration::from_secs(30);

/// Thread responsible for observing VPN status changes.
/// This is intentionally a blocking function, as it uses the objective-c objects which are not thread safe.
pub fn observer_thread(initial_managers: HashMap<(String, Id), Retained<NETunnelProviderManager>>) {
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

        let observer = create_observer(
            &NSNotificationCenter::defaultCenter(),
            unsafe { objc2_network_extension::NEVPNStatusDidChangeNotification },
            vpn_status_change_handler,
            Some(connection.as_ref()),
        );
        debug!("Registered initial observer for manager with key: {key}, value: {value}");
        observers.insert((key, value), observer);
    }

    loop {
        match receiver.recv_timeout(OBSERVER_CLEANUP_INTERVAL) {
            Ok(message) => {
                debug!("Received message to observe the following connection: {message:?}");

                let (key, value) = message;

                if observers.contains_key(&(key.clone(), value)) {
                    debug!(
                        "Observer for manager with key: {key}, value: {value} already exists,
                        skipping",
                    );
                    continue;
                }

                let manager = manager_for_key_and_value(&key, value).unwrap();
                let connection = unsafe { manager.connection() };
                let observer = create_observer(
                    &NSNotificationCenter::defaultCenter(),
                    unsafe { objc2_network_extension::NEVPNStatusDidChangeNotification },
                    vpn_status_change_handler,
                    Some(connection.as_ref()),
                );

                observers.insert((key.clone(), value), observer);
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
                        dead_keys.push((key.clone(), *value));
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
#[derive(Deserialize)]
#[repr(C)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Stats {
    pub(crate) location_id: Option<Id>,
    pub(crate) tunnel_id: Option<Id>,
    pub(crate) tx_bytes: u64,
    pub(crate) rx_bytes: u64,
    pub(crate) last_handshake: u64,
}

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

fn create_observer(
    center: &NSNotificationCenter,
    name: &NSNotificationName,
    handler: impl Fn(&NSNotification) + 'static,
    object: Option<&AnyObject>,
) -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
        handler(unsafe { notification.as_ref() });
    });
    let queue = NSOperationQueue::mainQueue();
    unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(name), object, Some(&queue), &block)
    }
}

#[must_use]
pub fn get_managers_for_tunnels_and_locations(
    tunnels: &[Tunnel<Id>],
    locations: &[Location<Id>],
) -> HashMap<(String, Id), Retained<NETunnelProviderManager>> {
    let mut managers = HashMap::new();

    for location in locations {
        if let Some(manager) = manager_for_key_and_value(LOCATION_ID, location.id) {
            managers.insert((LOCATION_ID.to_string(), location.id), manager);
        }
    }

    for tunnel in tunnels {
        if let Some(manager) = manager_for_key_and_value(TUNNEL_ID, tunnel.id) {
            managers.insert((TUNNEL_ID.to_string(), tunnel.id), manager);
        }
    }

    managers
}

/// Try to get `Id` out of manager. ID is embedded in configuration dictionary under `key`.
fn id_from_manager(manager: &NETunnelProviderManager, key: &NSString) -> Option<Id> {
    // TODO: This variable should be static.
    let plugin_bundle_id = NSString::from_str(PLUGIN_BUNDLE_ID);

    let Some(vpn_protocol) = (unsafe { manager.protocolConfiguration() }) else {
        return None;
    };
    let Ok(tunnel_protocol) = vpn_protocol.downcast::<NETunnelProviderProtocol>() else {
        error!("Failed to downcast to NETunnelProviderProtocol");
        return None;
    };
    // Sometimes all managers from all apps come through, so filter by bundle ID.
    if let Some(bundle_id) = unsafe { tunnel_protocol.providerBundleIdentifier() } {
        if bundle_id != plugin_bundle_id {
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
    pub(crate) fn tunnel_provider_manager(&self) -> Option<Retained<NETunnelProviderManager>> {
        let (key, value) = match (self.location_id, self.tunnel_id) {
            (Some(location_id), None) => (LOCATION_ID, location_id),
            (None, Some(tunnel_id)) => (TUNNEL_ID, tunnel_id),
            _ => return None,
        };

        manager_for_key_and_value(key, value)
    }

    /// Create or update system VPN settings with this configuration.
    pub(crate) fn save(&self) {
        let spinlock = Arc::new(AtomicBool::new(false));
        let spinlock_clone = Arc::clone(&spinlock);
        let plugin_bundle_id = NSString::from_str(PLUGIN_BUNDLE_ID);

        let provider_manager = self
            .tunnel_provider_manager()
            .unwrap_or_else(|| unsafe { NETunnelProviderManager::new() });

        unsafe {
            let tunnel_protocol = NETunnelProviderProtocol::new();
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
                        self.location_id.map_or_else(
                            || TUNNEL_ID.to_string(),
                            |_location_id| LOCATION_ID.to_string(),
                        ),
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

/// Remove configuration from system settings for [`Location`].
pub(crate) fn remove_config_for_location(location: &Location<Id>) {
    if let Some(provider_manager) = manager_for_key_and_value(LOCATION_ID, location.id) {
        unsafe {
            provider_manager.removeFromPreferencesWithCompletionHandler(None);
        }
    } else {
        debug!(
            "Couldn't find configuration in system settings for location {}",
            location.name
        );
    }
}

/// Remove configuration from system settings for [`Tunnel`].
pub(crate) fn remove_config_for_tunnel(tunnel: &Tunnel<Id>) {
    if let Some(provider_manager) = manager_for_key_and_value(TUNNEL_ID, tunnel.id) {
        unsafe {
            provider_manager.removeFromPreferencesWithCompletionHandler(None);
        }
    } else {
        debug!(
            "Couldn't find configuration in system settings for tunnel {}",
            tunnel.name
        );
    }
}

/// Stop tunnel for [`Location`].
pub(crate) fn stop_tunnel_for_location(location: &Location<Id>) -> bool {
    manager_for_key_and_value(LOCATION_ID, location.id).map_or_else(
        || {
            debug!(
                "Couldn't find configuration in system settings for location {}",
                location.name
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

/// Stop tunnel for [`Tunnel`].
pub(crate) fn stop_tunnel_for_tunnel(tunnel: &Tunnel<Id>) -> bool {
    manager_for_key_and_value(TUNNEL_ID, tunnel.id).map_or_else(
        || {
            debug!(
                "Couldn't find configuration in system settings for location {}",
                tunnel.name
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

/// Check whether tunnel is running for [`Location`].
pub(crate) fn get_location_status(location: &Location<Id>) -> Option<NEVPNStatus> {
    manager_for_key_and_value(LOCATION_ID, location.id).map_or_else(
        || {
            debug!(
                "Couldn't find configuration in system settings for location {}",
                location.name
            );
            None
        },
        |provider_manager| unsafe {
            let connection = provider_manager.connection();
            Some(connection.status())
        },
    )
}

/// Check whether tunnel is running for [`Tunnel`].
pub(crate) fn get_tunnel_status(tunnel: &Tunnel<Id>) -> Option<NEVPNStatus> {
    manager_for_key_and_value(TUNNEL_ID, tunnel.id).map_or_else(
        || {
            debug!(
                "Couldn't find configuration in system settings for tunnel {}",
                tunnel.name
            );
            None
        },
        |provider_manager| unsafe {
            let connection = provider_manager.connection();
            Some(connection.status())
        },
    )
}

pub(crate) fn tunnel_stats(id: Id, connection_type: &ConnectionType) -> Option<Stats> {
    let new_stats = Arc::new(Mutex::new(None));
    let plugin_bundle_id = NSString::from_str(PLUGIN_BUNDLE_ID);

    let new_stats_clone = Arc::clone(&new_stats);

    let finished = Arc::new(AtomicBool::new(false));
    let finished_clone = Arc::clone(&finished);

    let response_handler = RcBlock::new(move |data_ptr: *mut NSData| {
        if let Some(data) = unsafe { data_ptr.as_ref() } {
            if let Ok(stats) = serde_json::from_slice(data.to_vec().as_slice()) {
                if let Ok(mut new_stats_locked) = new_stats_clone.lock() {
                    *new_stats_locked = Some(stats);
                }
            } else {
                warn!("Failed to deserialize tunnel stats");
            }
        } else {
            debug!("No data received in tunnel stats response, skipping");
        }
        finished_clone.store(true, Ordering::Release);
    });

    let manager = manager_for_key_and_value(
        match connection_type {
            ConnectionType::Location => LOCATION_ID,
            ConnectionType::Tunnel => TUNNEL_ID,
        },
        id,
    )?;

    let vpn_protocol = (unsafe { manager.protocolConfiguration() })?;
    let Ok(tunnel_protocol) = vpn_protocol.downcast::<NETunnelProviderProtocol>() else {
        error!("Failed to downcast to NETunnelProviderProtocol");
        return None;
    };

    // Sometimes all managers from all apps come through, so filter by bundle ID.
    if let Some(bundle_id) = unsafe { tunnel_protocol.providerBundleIdentifier() } {
        if bundle_id != plugin_bundle_id {
            return None;
        }
    }

    let Ok(session) = unsafe { manager.connection() }.downcast::<NETunnelProviderSession>() else {
        error!("Failed to downcast to NETunnelProviderSession");
        return None;
    };

    let message_data = NSData::new();
    if unsafe {
        session.sendProviderMessage_returnError_responseHandler(
            &message_data,
            None,
            Some(&response_handler),
        )
    } {
        debug!("Message sent to NETunnelProviderSession");
    } else {
        error!("Failed to send to NETunnelProviderSession while requesting stats");
    }

    // Wait for all handlers to complete.
    while !finished.load(Ordering::Acquire) {
        spin_loop();
    }

    let stats = new_stats
        .lock()
        .map_or(None, |mut new_stats_locked| new_stats_locked.take());

    stats
}

/// Synchronize locations and tunnels with system settings.
pub async fn sync_locations_and_tunnels(mtu: Option<u32>) -> Result<(), sqlx::Error> {
    // Update location settings.
    let all_locations = Location::all(&*DB_POOL, false).await?;
    for location in &all_locations {
        // For syncing, set `preshred_key` to `None`.
        let Ok(tunnel_config) = location.tunnel_configurarion(&*DB_POOL, None, mtu).await else {
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
        let Ok(tunnel_config) = tunnel.tunnel_configurarion(mtu) else {
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

impl Location<Id> {
    /// Build [`TunnelConfiguration`] from [`Location`].
    pub(crate) async fn tunnel_configurarion<'e, E>(
        &self,
        executor: E,
        preshared_key: Option<String>,
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
}

impl Tunnel<Id> {
    /// Build [`TunnelConfiguration`] from [`Tunnel`].
    pub(crate) fn tunnel_configurarion(
        &self,
        mtu: Option<u32>,
    ) -> Result<TunnelConfiguration, Error> {
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
}
