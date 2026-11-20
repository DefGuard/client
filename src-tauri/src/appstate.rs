use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use defguard_client_core::{
    connection::active_connections::ACTIVE_CONNECTIONS, enrollment::EnrollmentSession,
    mfa::MfaAuthSession, mfa_config::MfaConfigSession,
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
    events::AddInstancePayload,
    session_state::SessionState,
    utils::stats_handler,
    ConnectionType,
};

/// One security key ceremony. The id tells two ceremonies for the same session apart, so a
/// finished one never removes the entry a newer one left in its place.
#[derive(Clone)]
pub struct Ceremony {
    pub id: Uuid,
    pub token: CancellationToken,
}

pub struct AppState {
    pub enrollment_sessions: Mutex<HashMap<Uuid, EnrollmentSession>>,
    pub mfa_config_sessions: Mutex<HashMap<Uuid, MfaConfigSession>>,
    pub mfa_auth_sessions: Mutex<HashMap<String, Arc<MfaAuthSession>>>,
    /// Keyed by configuration session, so abandoning one dismisses the prompt it left on screen.
    pub mfa_config_ceremonies: Mutex<HashMap<Uuid, Ceremony>>,
    pub log_watchers: Mutex<HashMap<String, CancellationToken>>,
    pub mfa_tasks: Mutex<HashMap<String, CancellationToken>>,
    pub app_config: Mutex<AppConfig>,
    pub tray_click_position: Mutex<Option<PhysicalPosition<f64>>>,
    stat_threads: Mutex<HashMap<(Id, ConnectionType), JoinHandle<()>>>,
    pub provisioning_config: Mutex<Option<ProvisioningConfig>>,
    pub provisioning_instance_id: Mutex<Option<String>>,
    pub pending_add_instance: Mutex<Option<AddInstancePayload>>,
    pub session_state: Mutex<SessionState>,
}

impl AppState {
    #[must_use]
    pub fn mtu(&self) -> Option<u32> {
        self.app_config
            .lock()
            .expect("failed to lock app state")
            .mtu()
    }

    #[must_use]
    pub fn new(config: AppConfig, provisioning_config: Option<ProvisioningConfig>) -> Self {
        Self {
            enrollment_sessions: Mutex::new(HashMap::new()),
            mfa_config_sessions: Mutex::new(HashMap::new()),
            mfa_auth_sessions: Mutex::new(HashMap::new()),
            mfa_config_ceremonies: Mutex::new(HashMap::new()),
            log_watchers: Mutex::new(HashMap::new()),
            mfa_tasks: Mutex::new(HashMap::new()),
            app_config: Mutex::new(config),
            tray_click_position: Mutex::new(None),
            stat_threads: Mutex::new(HashMap::new()),
            provisioning_config: Mutex::new(provisioning_config),
            provisioning_instance_id: Mutex::new(None),
            pending_add_instance: Mutex::new(None),
            session_state: Mutex::new(SessionState::default()),
        }
    }

    #[must_use]
    pub fn with_session_state(mut self, session_state: SessionState) -> Self {
        self.session_state = Mutex::new(session_state);
        self
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

        debug!("Spawning thread for network statistics for {connection_type} ID {location_id}");
        #[cfg(target_os = "macos")]
        let handle = spawn(stats_handler(location_id, connection_type));
        #[cfg(not(target_os = "macos"))]
        let handle = spawn(stats_handler(ifname, connection_type));
        let Some(old_handle) = self.replace_stat_thread((location_id, connection_type), handle)
        else {
            debug!("Added new network statistics thread for {connection_type} ID {location_id}");
            return;
        };
        warn!(
            "Something went wrong: old network statistics thread for {connection_type} ID \
            {location_id} still exists"
        );
        old_handle.abort();
        if let Err(err) = old_handle.await {
            debug!(
                "Old network statistics thread for {connection_type} ID {location_id} returned \
                {err}"
            );
        }
    }

    fn replace_stat_thread(
        &self,
        key: (Id, ConnectionType),
        handle: JoinHandle<()>,
    ) -> Option<JoinHandle<()>> {
        self.stat_threads.lock().unwrap().insert(key, handle)
    }

    fn take_stat_thread(&self, key: (Id, ConnectionType)) -> Option<JoinHandle<()>> {
        self.stat_threads.lock().unwrap().remove(&key)
    }

    /// Try to remove a connection from the list of active connections.
    /// Return removed connection, or `None` if not found.
    pub(crate) async fn remove_connection(
        &self,
        location_id: Id,
        connection_type: ConnectionType,
    ) -> Option<ActiveConnection> {
        debug!("Removing active connection for {connection_type} ID: {location_id}");

        if let Some(handle) = self.take_stat_thread((location_id, connection_type)) {
            debug!("Stopping network statistics thread for {connection_type} ID {location_id}");
            handle.abort();
            if let Err(err) = handle.await {
                debug!(
                    "Network statistics thread for {connection_type} ID {location_id} returned \
                    {err}"
                );
            }
        }

        let mut connections = ACTIVE_CONNECTIONS.lock().await;
        if let Some(index) = connections.iter().position(|conn| {
            conn.location_id == location_id && conn.connection_type == connection_type
        }) {
            let removed_connection = connections.remove(index);
            debug!("Active connection has been removed from the active connections list.");
            Some(removed_connection)
        } else {
            debug!("No active connection found with {connection_type} ID: {location_id}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use super::*;

    #[test]
    fn location_and_tunnel_with_same_id_keep_separate_stat_threads() {
        let state = AppState::new(AppConfig::default(), None);
        let location_key = (1, ConnectionType::Location);
        let tunnel_key = (1, ConnectionType::Tunnel);

        assert!(state
            .replace_stat_thread(location_key, spawn(pending()))
            .is_none());
        assert!(state
            .replace_stat_thread(tunnel_key, spawn(pending()))
            .is_none());

        state
            .take_stat_thread(tunnel_key)
            .expect("tunnel stats thread should be tracked")
            .abort();

        let location_handle = state
            .take_stat_thread(location_key)
            .expect("location stats thread should survive tunnel removal");
        assert!(!location_handle.inner().is_finished());
        location_handle.abort();
    }
}
