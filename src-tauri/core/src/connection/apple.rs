//! Interchangeability and communication with VPNExtension (written in Swift).

use std::{
    collections::HashMap,
    hint::spin_loop,
    net::IpAddr,
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, channel, Receiver, RecvTimeoutError, Sender},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};

const OBSERVER_CLEANUP_INTERVAL: Duration = Duration::from_secs(30);

use block2::RcBlock;
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_foundation::{
    ns_string, NSArray, NSDate, NSDictionary, NSError, NSNotification, NSNotificationCenter,
    NSNumber, NSObjectProtocol, NSOperationQueue, NSRunLoop, NSString,
};
use objc2_network_extension::{
    NETunnelProviderManager, NETunnelProviderProtocol, NEVPNConnection,
    NEVPNStatusDidChangeNotification,
};

use crate::database::{
    models::{location::Location, tunnel::Tunnel, Id},
    DB_POOL,
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
