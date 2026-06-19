use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use struct_patch::Patch;
use tauri::{AppHandle, Emitter, Manager, State};

use defguard_client_core::{
    database::models::location::{LocationMfaMethod, LocationMfaMode},
    events::EventKey,
};

use crate::{
    appstate::AppState,
    database::{models::location::Location, DbPool, DB_POOL},
    error::Error,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewSelectionKind {
    Instance,
    Tunnel,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OverviewViewSelection {
    pub kind: ViewSelectionKind,
    pub id: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Patch, Serialize)]
#[patch(attribute(derive(Debug, Deserialize, Serialize)))]
pub struct SessionState {
    pub view_selection: Option<OverviewViewSelection>,
    pub location_mfa_preference: HashMap<String, LocationMfaMethod>,
}

// verifies integrity of the mfa preference in session, this needs to be ran on location update so polling doesn't break the session store
pub async fn validate_location_mfa_preference(
    pool: &DbPool,
    mut preference: HashMap<String, LocationMfaMethod>,
) -> Result<HashMap<String, LocationMfaMethod>, Error> {
    if preference.is_empty() {
        return Ok(preference);
    }

    let ids: Vec<i64> = preference.keys().filter_map(|k| k.parse().ok()).collect();

    if ids.is_empty() {
        preference.clear();
        return Ok(preference);
    }

    let placeholders = (0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT id, location_mfa_mode FROM location WHERE id IN ({placeholders})");
    let mut q = sqlx::query_as::<_, (i64, LocationMfaMode)>(&sql);
    for id in &ids {
        q = q.bind(*id);
    }
    let rows = q.fetch_all(pool).await?;

    let mut found = std::collections::HashSet::with_capacity(rows.len());
    for (id, mfa_mode) in rows {
        let key = id.to_string();
        found.insert(key.clone());
        match mfa_mode {
            LocationMfaMode::Disabled => {
                preference.remove(&key);
            }
            LocationMfaMode::External => {
                preference.insert(key, LocationMfaMethod::Oidc);
            }
            LocationMfaMode::Internal => {
                if let Some(m) = preference.get(&key) {
                    match m {
                        LocationMfaMethod::Totp
                        | LocationMfaMethod::Email
                        | LocationMfaMethod::MobileApprove => {}
                        _ => {
                            preference.insert(key, LocationMfaMethod::Totp);
                        }
                    }
                }
            }
        }
    }
    preference.retain(|k, _| found.contains(k));
    Ok(preference)
}

pub async fn initialize_session_state() -> Result<SessionState, Error> {
    let locations = Location::all(&*DB_POOL, false).await?;
    let location_mfa_preference = locations
        .into_iter()
        .filter_map(|loc| loc.mfa_method.map(|method| (loc.id.to_string(), method)))
        .collect();
    Ok(SessionState {
        location_mfa_preference,
        ..SessionState::default()
    })
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
