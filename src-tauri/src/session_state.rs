use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use struct_patch::Patch;
use tauri::{AppHandle, Emitter, Manager, State};

use defguard_client_core::{
    database::models::{instance::InstanceInfo, location::LocationMfaMethod, Id},
    events::EventKey,
};

use crate::{appstate::AppState, commands::LocationInfo};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum OverviewViewSelection {
    Instance(InstanceInfo<Id>),
    Tunnel(LocationInfo),
}

#[derive(Clone, Debug, Default, Deserialize, Patch, Serialize)]
#[patch(attribute(derive(Debug, Deserialize, Serialize)))]
pub struct SessionState {
    pub view_selection: Option<OverviewViewSelection>,
    pub location_mfa_preference: HashMap<String, LocationMfaMethod>,
}

#[tauri::command]
pub fn get_session_state(app_state: State<'_, AppState>) -> Result<SessionState, String> {
    app_state
        .session_state
        .lock()
        .map(|s| s.clone())
        .map_err(|err| format!("Session state mutex poisoned: {err}"))
}

#[tauri::command(async)]
pub async fn patch_session_state(
    patch: SessionStatePatch,
    app_handle: AppHandle,
) -> Result<SessionState, String> {
    let app_state = app_handle.state::<AppState>();
    let updated = app_state
        .session_state
        .lock()
        .map_err(|err| format!("Session state mutex poisoned: {err}"))
        .map(|mut s| {
            s.apply(patch);
            s.clone()
        })?;
    if let Err(err) = app_handle.emit(EventKey::SessionStateChanged.into(), ()) {
        error!("Failed to emit session-state-changed event: {err}");
    }
    Ok(updated)
}
