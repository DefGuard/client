//! Interchangeability and communication with VPNExtension (written in Swift).

use std::{
    collections::HashMap,
    hint::spin_loop,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use block2::RcBlock;
use defguard_client_core::connection::{
    active_connections::find_connection,
    apple::{
        manager_for_key_and_value, LOCATION_ID, PLUGIN_BUNDLE_ID, TUNNEL_ID, VPN_STATE_UPDATE_COMMS,
    },
};
use objc2::rc::Retained;
use objc2_foundation::{ns_string, NSData};
use objc2_network_extension::{
    NETunnelProviderManager, NETunnelProviderProtocol, NETunnelProviderSession, NEVPNStatus,
};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;
use tracing::Level;

use crate::{
    appstate::AppState,
    database::models::{get_all_tunnels_locations, location::Location, tunnel::Tunnel, Id},
    events::EventKey,
    log_watcher::service_log_watcher::spawn_log_watcher_task,
    tray::{configure_tray_icon, reload_tray_menu, show_main_window},
    ConnectionType,
};

const SYSTEM_SYNC_DELAY: Duration = Duration::from_millis(500);

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
        sleep(SYSTEM_SYNC_DELAY).await;
        sync_connections_with_system(app_handle).await;
        reload_tray_menu(app_handle).await;
        let _ = configure_tray_icon(app_handle).await;
        debug!("Processed status update message.");
    }
}

/// Synchronize the app's connection state with the system's VPN state.
/// This checks all locations and tunnels and updates the app state to match
/// what's actually running in the system.
async fn sync_connections_with_system(app_handle: &AppHandle) {
    let app_state = app_handle.state::<AppState>();
    let (tunnels, locations) = get_all_tunnels_locations().await;

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
                        let _ = location.stop_vpn_tunnel();
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
