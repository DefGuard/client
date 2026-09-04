use std::{collections::HashMap, sync::Mutex};

use defguard_client_core::{
    connection::active_connections::ACTIVE_CONNECTIONS, enrollment::EnrollmentSession,
};
use defguard_client_provisioning::ProvisioningConfig;
use tauri::{
    async_runtime::{spawn, JoinHandle},
    PhysicalPosition,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    app_config::AppConfig,
    database::models::{connection::ActiveConnection, Id},
    session_state::SessionState,
    utils::stats_handler,
    ConnectionType,
};

pub struct AppState {
    pub enrollment_sessions: Mutex<HashMap<Uuid, EnrollmentSession>>,
    pub log_watchers: Mutex<HashMap<String, CancellationToken>>,
    pub mfa_tasks: Mutex<HashMap<String, CancellationToken>>,
    multi_step_mfa_capabilities: Mutex<HashMap<Id, bool>>,
    pub app_config: Mutex<AppConfig>,
    pub tray_click_position: Mutex<Option<PhysicalPosition<f64>>>,
    stat_threads: Mutex<HashMap<Id, JoinHandle<()>>>, // location ID is the key
    pub provisioning_config: Mutex<Option<ProvisioningConfig>>,
    pub session_state: Mutex<SessionState>,
}

impl AppState {
    #[must_use]
    pub fn new(config: AppConfig, provisioning_config: Option<ProvisioningConfig>) -> Self {
        Self {
            enrollment_sessions: Mutex::new(HashMap::new()),
            log_watchers: Mutex::new(HashMap::new()),
            mfa_tasks: Mutex::new(HashMap::new()),
            multi_step_mfa_capabilities: Mutex::new(HashMap::new()),
            app_config: Mutex::new(config),
            tray_click_position: Mutex::new(None),
            stat_threads: Mutex::new(HashMap::new()),
            provisioning_config: Mutex::new(provisioning_config),
            session_state: Mutex::new(SessionState::default()),
        }
    }

    pub(crate) fn set_multi_step_mfa_capable(&self, instance_id: Id, capable: bool) {
        self.multi_step_mfa_capabilities
            .lock()
            .expect("multi_step_mfa_capabilities mutex poisoned")
            .insert(instance_id, capable);
    }

    #[must_use]
    pub fn is_multi_step_mfa_capable(&self, instance_id: Id) -> bool {
        self.multi_step_mfa_capabilities
            .lock()
            .expect("multi_step_mfa_capabilities mutex poisoned")
            .get(&instance_id)
            .copied()
            .unwrap_or(false)
    }

    pub(crate) async fn add_connection<S: Into<String>>(
        &self,
        location_id: Id,
        interface_name: S,
        connection_type: ConnectionType,
    ) {
        let ifname = interface_name.into();
        let connection = ActiveConnection::new(location_id, ifname.clone(), connection_type);
        debug!("Adding active connection for location ID: {location_id}");
        let mut connections = ACTIVE_CONNECTIONS.lock().await;
        connections.push(connection);
        trace!("Current active connections: {connections:?}");
        drop(connections);

        debug!("Spawning thread for network statistics for location ID {location_id}");
        #[cfg(target_os = "macos")]
        let handle = spawn(stats_handler(location_id, connection_type));
        #[cfg(not(target_os = "macos"))]
        let handle = spawn(stats_handler(ifname, connection_type));
        let Some(old_handle) = self
            .stat_threads
            .lock()
            .unwrap()
            .insert(location_id, handle)
        else {
            debug!("Added new network statistics thread for location ID {location_id}");
            return;
        };
        warn!("Something went wrong: old network statistics thread still exists");
        old_handle.abort();
        if let Err(err) = old_handle.await {
            debug!("Old network statistics thread for location ID {location_id} returned {err}");
        }
    }

    /// Try to remove a connection from the list of active connections.
    /// Return removed connection, or `None` if not found.
    pub(crate) async fn remove_connection(
        &self,
        location_id: Id,
        connection_type: ConnectionType,
    ) -> Option<ActiveConnection> {
        debug!("Removing active connection for location ID: {location_id}");

        // Stop statistics thread
        {
            let handle = self.stat_threads.lock().unwrap().remove(&location_id);
            if let Some(handle) = handle {
                debug!("Stopping network statistics thread for location ID {location_id}");
                handle.abort();
                if let Err(err) = handle.await {
                    debug!(
                        "Network statistics thread for location ID {location_id} returned {err}"
                    );
                }
            }
        }

        let mut connections = ACTIVE_CONNECTIONS.lock().await;
        if let Some(index) = connections.iter().position(|conn| {
            conn.location_id == location_id && conn.connection_type == connection_type
        }) {
            // Found a connection with the specified location_id
            let removed_connection = connections.remove(index);
            debug!("Active connection has been removed from the active connections list.");
            Some(removed_connection)
        } else {
            debug!("No active connection found with location ID: {location_id}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_step_mfa_capability_defaults_to_false() {
        let state = AppState::new(AppConfig::default(), None);

        assert!(!state.is_multi_step_mfa_capable(1));
        state.set_multi_step_mfa_capable(1, true);
        assert!(state.is_multi_step_mfa_capable(1));
        state.set_multi_step_mfa_capable(1, false);
        assert!(!state.is_multi_step_mfa_capable(1));
    }
}
