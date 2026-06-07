//! Interchangeability and communication with VPNExtension (written in Swift).

use std::{
    collections::HashMap,
    hint::spin_loop,
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};

use block2::RcBlock;
use defguard_client_core::database::models::tunnel_configuration::{
    id_from_manager, manager_for_key_and_value, LOCATION_ID, OBSERVER_COMMS, PLUGIN_BUNDLE_ID,
    TUNNEL_ID,
};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::{
    ns_string, NSArray, NSData, NSDate, NSError, NSNotification, NSNotificationCenter,
    NSObjectProtocol, NSOperationQueue, NSRunLoop, NSString,
};
use objc2_network_extension::{
    NETunnelProviderManager, NETunnelProviderProtocol, NETunnelProviderSession, NEVPNConnection,
    NEVPNStatus, NEVPNStatusDidChangeNotification,
};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tracing::Level;

use crate::{
    active_connections::find_connection,
    appstate::AppState,
    database::{
        models::{location::Location, tunnel::Tunnel, Id},
        DB_POOL,
    },
    events::EventKey,
    log_watcher::service_log_watcher::spawn_log_watcher_task,
    tray::{configure_tray_icon, reload_tray_menu, show_main_window},
    ConnectionType,
};

const SYSTEM_SYNC_DELAY: Duration = Duration::from_millis(500);

type VpnStateSender = Mutex<Sender<()>>;
type VpnStateReceiver = Mutex<Option<Receiver<()>>>;

static VPN_STATE_UPDATE_COMMS: LazyLock<(VpnStateSender, VpnStateReceiver)> = LazyLock::new(|| {
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
        tokio::time::sleep(SYSTEM_SYNC_DELAY).await;
        sync_connections_with_system(app_handle).await;
        reload_tray_menu(app_handle).await;
        let _ = configure_tray_icon(app_handle).await;
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
            let status = location.status();
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
                            location.stop_vpn_tunnel();
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
                            app_handle,
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
            let status = tunnel.status();
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
                            app_handle,
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

/// Retrieve VPN tunnel statistics from VPNExtension.
pub(crate) fn tunnel_stats(id: Id, connection_type: &ConnectionType) -> Option<Stats> {
    let new_stats = Arc::new(Mutex::new(None));
    let plugin_bundle_id = ns_string!(PLUGIN_BUNDLE_ID);

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
        if &*bundle_id != plugin_bundle_id {
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
