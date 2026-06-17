pub use defguard_client_core::events::EventKey;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Url};
use tauri_plugin_notification::NotificationExt;

use crate::{
    window_manager::{WindowManager, COMPACT_WINDOW_ID},
    ConnectionType,
};

/// Used as payload for [`DEAD_CONNECTION_DROPPED`] event
#[derive(Clone, Serialize)]
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
#[derive(Clone, Serialize)]
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

#[derive(Clone, Serialize)]
pub struct AddInstancePayload<'a> {
    pub token: &'a str,
    pub url: &'a str,
}

/// Handle deep-link URLs.
pub fn handle_deep_link(app_handle: &AppHandle, urls: &[Url]) {
    debug!("Deep link received.");
    for link in urls {
        if link.host_str() == Some("addinstance") {
            let mut token = None;
            let mut url = None;
            for (key, value) in link.query_pairs() {
                if key == "token" {
                    token = Some(value.clone());
                }
                if key == "url" {
                    url = Some(value.clone());
                }
            }
            if let (Some(token), Some(url)) = (token, url) {
                info!("Valid Deep link received.");
                if let Some(tray_win) = app_handle.get_webview_window(COMPACT_WINDOW_ID) {
                    let _ = tray_win.hide();
                }
                if let Err(e) = WindowManager::open_full_view(app_handle) {
                    warn!("Deep link: failed to open main window: {e}");
                }
                let _ = app_handle.emit(
                    EventKey::AddInstance.into(),
                    AddInstancePayload {
                        token: &token,
                        url: &url,
                    },
                );
            }
        }
    }
}
