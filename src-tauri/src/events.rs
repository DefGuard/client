use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::ConnectionType;

// Match src/page/client/types.ts.
#[non_exhaustive]
pub enum EventKey {
    SingleInstance,
    ConnectionChanged,
    InstanceUpdate,
    LocationUpdate,
    AppVersionFetch,
    ConfigChanged,
    DeadConnectionDropped,
    DeadConnectionReconnected,
    ApplicationConfigChanged,
    DoEntrollment,
}

impl From<EventKey> for &'static str {
    fn from(key: EventKey) -> &'static str {
        match key {
            EventKey::SingleInstance => "single-instance",
            EventKey::ConnectionChanged => "connection-changed",
            EventKey::InstanceUpdate => "instance-update",
            EventKey::LocationUpdate => "location-update",
            EventKey::AppVersionFetch => "app-version-fetch",
            EventKey::ConfigChanged => "config-changed",
            EventKey::DeadConnectionDropped => "dead-connection-dropped",
            EventKey::DeadConnectionReconnected => "dead-connection-reconnected",
            EventKey::ApplicationConfigChanged => "application-config-changed",
            EventKey::DoEntrollment => "do-enrollment",
        }
    }
}

/// Used as payload for [`DEAD_CONNECTION_DROPPED`] event
#[derive(Serialize, Clone, Debug)]
pub struct DeadConnDroppedOut {
    pub(crate) name: String,
    pub(crate) con_type: ConnectionType,
    pub(crate) peer_alive_period: i64,
}

impl DeadConnDroppedOut {
    /// Emits [`DEAD_CONNECTION_DROPPED`] event with corresponding side effects.
    pub(crate) fn emit(self, app_handle: &AppHandle) {
        if let Err(err) = app_handle
            .notification()
            .builder()
            // .id(&app_handle.config().identifier)
            .title(format!("{} {} disconnected", self.con_type, self.name))
            .body("Connection activity timeout.")
            .show()
        {
            warn!("Dead connection dropped notification not shown. Reason: {err}");
        }
        if let Err(err) = app_handle.emit(EventKey::DeadConnectionDropped.into(), self) {
            error!("Event Dead Connection Dropped was not emitted. Reason: {err}");
        }
    }
}

/// Used as payload for [`DEAD_CONNECTION_RECONNECTED`] event
#[derive(Serialize, Clone, Debug)]
pub struct DeadConnReconnected {
    pub(crate) name: String,
    pub(crate) con_type: ConnectionType,
    pub(crate) peer_alive_period: i64,
}

impl DeadConnReconnected {
    /// Emits [`DEAD_CONNECTION_RECONNECTED`] event with corresponding side effects.
    pub(crate) fn emit(self, app_handle: &AppHandle) {
        if let Err(err) = app_handle
            .notification()
            .builder()
            // .id(&app_handle.config().identifier)
            .title(format!("{} {} reconnected", self.con_type, self.name))
            .body("Connection activity timeout.")
            .show()
        {
            warn!("Dead connection reconnected notification not shown. Reason: {err}");
        }
        if let Err(err) = app_handle.emit(EventKey::DeadConnectionReconnected.into(), self) {
            error!("Event Dead Connection Reconnected was not emitted. Reason: {err}");
        }
    }
}
