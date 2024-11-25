use serde::Serialize;
use strum::Display;
use tauri::{api::notification::Notification, AppHandle, Manager};

use crate::ConnectionType;

// Keep list of events on top
pub static SINGLE_INSTANCE: &str = "single-instance";
pub static CONNECTION_CHANGED: &str = "connection-changed";
pub static INSTANCE_UPDATE: &str = "instance-update";
pub static LOCATION_UPDATE: &str = "location-update";
pub static APP_VERSION_FETCH: &str = "app-version-fetch";
pub static CONFIG_CHANGED: &str = "config-changed";
pub static DEAD_CONNECTION_DROPPED: &str = "dead-connection-dropped";
pub static APPLICATION_CONFIG_CHANGED: &str = "application-config-changed";

/// Why [`DEAD_CONNECTION_DROPPED`] event was emitted
#[derive(Display, Clone, Debug, Serialize)]
pub enum DeadConDroppedOutReason {
    /// Connections verification periodic loop
    PeriodicVerification,
    /// Verification after connection establishment
    ConnectionVerification,
}

/// Used as payload for [`DEAD_CONNECTION_DROPPED`] event
#[derive(Serialize, Clone, Debug)]
pub struct DeadConnDroppedOut {
    pub(crate) name: String,
    pub(crate) con_type: ConnectionType,
    pub(crate) reason: DeadConDroppedOutReason,
}

impl DeadConnDroppedOut {
    /// Emits [`DEAD_CONNECTION_DROPPED`] event with corresponding side effects.
    pub(crate) fn emit(self, app_handle: &AppHandle) {
        let body = match self.reason {
            DeadConDroppedOutReason::ConnectionVerification => "No handshake.",
            DeadConDroppedOutReason::PeriodicVerification => "Handshake timeout.",
        };
        if let Err(err) = Notification::new(&app_handle.config().tauri.bundle.identifier)
            .title(format!("{} {} disconnected", self.con_type, self.name))
            .body(body)
            .show()
        {
            warn!("Dead connection dropped notification not shown. Reason: {err}");
        }
        if let Err(err) = app_handle.emit_all(DEAD_CONNECTION_DROPPED, self) {
            error!("Event Dead Connection Dropped was not emitted. Reason: {err}");
        }
    }
}
