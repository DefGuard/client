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
pub fn command_get_session_state(app_state: State<'_, AppState>) -> SessionState {
    app_state.session_state.lock().unwrap().clone()
}

#[tauri::command(async)]
pub async fn command_patch_session_state(
    patch: SessionStatePatch,
    app_handle: AppHandle,
) -> Result<SessionState, ()> {
    let app_state = app_handle.state::<AppState>();
    let updated = {
        let mut session_state = app_state.session_state.lock().unwrap();
        session_state.apply(patch);
        session_state.clone()
    };
    if let Err(err) = app_handle.emit(EventKey::SessionStateChanged.into(), ()) {
        error!("Failed to emit session-state-changed event: {err}");
    }
    Ok(updated)
}
