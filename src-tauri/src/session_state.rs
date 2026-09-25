use std::{collections::HashMap, fs, path::Path};

use defguard_client_core::events::EventKey;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::SqlitePool;
use struct_patch::Patch;
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(unix)]
use crate::set_perms;
use crate::{
    appstate::AppState,
    database::models::{instance::Instance, tunnel::Tunnel},
};

static WINDOW_SESSION_FILE_NAME: &str = "window-session.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStateMfaMethod {
    Totp,
    Email,
    Oidc,
    Biometric,
    MobileApprove,
}

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
    // needed to display properly the method tile between windows as connection doesn't hold this
    pub connection_mfa_method: HashMap<String, Option<SessionStateMfaMethod>>,
}

impl SessionState {
    #[must_use]
    pub fn persisted(&self) -> PersistedSessionState {
        PersistedSessionState {
            view_selection: self.view_selection.clone(),
        }
    }
}

impl From<PersistedSessionState> for SessionState {
    fn from(persisted: PersistedSessionState) -> Self {
        Self {
            view_selection: persisted.view_selection,
            ..Default::default()
        }
    }
}

/// Subset of [`SessionState`] persisted to window-session.json; restoring is best-effort.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PersistedSessionState {
    #[serde(default)]
    pub view_selection: Option<OverviewViewSelection>,
}

/// Per-field fallback, so one malformed field doesn't discard the others.
fn field_or_default<T: Default + DeserializeOwned>(
    fields: &mut Map<String, Value>,
    name: &str,
) -> T {
    let Some(value) = fields.remove(name) else {
        return T::default();
    };
    serde_json::from_value(value).unwrap_or_else(|err| {
        warn!("Ignoring malformed window session field \"{name}\": {err}");
        T::default()
    })
}

impl PersistedSessionState {
    /// Never fails; errors are logged and defaults returned.
    #[must_use]
    pub fn load(dir: &Path) -> Self {
        let path = dir.join(WINDOW_SESSION_FILE_NAME);
        let contents = match fs::read(&path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                debug!("Window session file doesn't exist; using defaults.");
                return Self::default();
            }
            Err(err) => {
                error!(
                    "Failed to read window session file {}: {err}",
                    path.display()
                );
                return Self::default();
            }
        };
        match serde_json::from_slice::<Value>(&contents) {
            Ok(Value::Object(mut fields)) => Self {
                view_selection: field_or_default(&mut fields, "view_selection"),
            },
            Ok(_) => {
                error!("Window session file is not a JSON object; using defaults.");
                Self::default()
            }
            Err(err) => {
                error!("Failed to parse window session file: {err}. Using defaults.");
                Self::default()
            }
        }
    }

    /// Failures are only logged.
    pub fn save(&self, dir: &Path) {
        let contents = match serde_json::to_vec(self) {
            Ok(contents) => contents,
            Err(err) => {
                error!("Failed to serialize window session: {err}");
                return;
            }
        };
        if let Err(err) = fs::create_dir_all(dir) {
            error!("Failed to create app data dir {}: {err}", dir.display());
            return;
        }
        let path = dir.join(WINDOW_SESSION_FILE_NAME);
        match fs::write(&path, contents) {
            Ok(()) => {
                #[cfg(unix)]
                set_perms(&path);
                debug!("Window session file has been saved.");
            }
            Err(err) => error!(
                "Failed to write window session file {}: {err}",
                path.display()
            ),
        }
    }

    /// Replaces a selection missing from the DB with the latest instance, or none.
    /// Returns whether anything changed.
    pub async fn resolve(&mut self, pool: &SqlitePool) -> bool {
        let Some(selection) = &self.view_selection else {
            return false;
        };
        let exists = match selection.kind {
            ViewSelectionKind::Instance => Instance::find_by_id(pool, selection.id)
                .await
                .map(|instance| instance.is_some()),
            ViewSelectionKind::Tunnel => Tunnel::find_by_id(pool, selection.id)
                .await
                .map(|tunnel| tunnel.is_some()),
        };
        let fallback = match exists {
            Ok(true) => return false,
            Ok(false) => match Instance::all(pool).await {
                Ok(instances) => instances
                    .iter()
                    .map(|instance| instance.id)
                    .max()
                    .map(|id| OverviewViewSelection {
                        kind: ViewSelectionKind::Instance,
                        id,
                    }),
                Err(err) => {
                    error!("Failed to load instances while restoring window session: {err}");
                    None
                }
            },
            Err(err) => {
                error!("Failed to validate restored window session selection: {err}");
                None
            }
        };
        info!(
            "Restored window session selection {:?} no longer exists; replacing with {fallback:?}.",
            self.view_selection
        );
        self.view_selection = fallback;
        true
    }
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
    let (updated, persisted_changed) = app_state
        .session_state
        .lock()
        .map_err(|err| format!("Session state mutex poisoned: {err}"))
        .map(|mut s| {
            let before = s.persisted();
            s.apply(patch);
            let changed = before != s.persisted();
            (s.clone(), changed)
        })?;
    if persisted_changed {
        match app_handle.path().app_data_dir() {
            Ok(dir) => updated.persisted().save(&dir),
            Err(err) => error!("Failed to access app data dir to save window session: {err}"),
        }
    }
    if let Err(err) = app_handle.emit(EventKey::SessionStateChanged.into(), ()) {
        error!("Failed to emit session-state-changed event: {err}");
    }
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use sqlx::SqlitePool;
    use tempfile::tempdir;

    use super::*;
    use crate::database::models::{
        instance::{ClientTrafficPolicy, Instance},
        tunnel::Tunnel,
        Id, NoId,
    };

    fn instance_selection(id: Id) -> Option<OverviewViewSelection> {
        Some(OverviewViewSelection {
            kind: ViewSelectionKind::Instance,
            id,
        })
    }

    async fn seed_instance(pool: &SqlitePool, name: &str) -> Id {
        Instance {
            id: NoId,
            name: name.into(),
            uuid: format!("uuid-{name}"),
            url: "https://core.example".into(),
            proxy_url: "https://proxy.example".into(),
            username: "alice".into(),
            token: None,
            client_traffic_policy: ClientTrafficPolicy::None,
            enterprise_enabled: false,
            disable_tunnels: false,
            openid_display_name: None,
            mfa_configured_methods: None,
        }
        .save(pool)
        .await
        .unwrap()
        .id
    }

    async fn seed_tunnel(pool: &SqlitePool) -> Id {
        Tunnel::new(
            "tunnel".into(),
            "pubkey".into(),
            "prvkey".into(),
            "10.0.0.2/24".into(),
            "server-pubkey".into(),
            None,
            None,
            "1.2.3.4:51820".into(),
            None,
            25,
            false,
            None,
            None,
            None,
            None,
        )
        .save(pool)
        .await
        .unwrap()
        .id
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = tempdir().unwrap();
        assert_eq!(
            PersistedSessionState::load(dir.path()),
            PersistedSessionState::default()
        );
    }

    #[test]
    fn test_load_corrupt_json_returns_default() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(WINDOW_SESSION_FILE_NAME),
            b"{ not valid json",
        )
        .unwrap();
        assert_eq!(
            PersistedSessionState::load(dir.path()),
            PersistedSessionState::default()
        );
    }

    #[test]
    fn test_load_malformed_field_returns_default() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(WINDOW_SESSION_FILE_NAME),
            br#"{"view_selection": {"kind": "bogus", "id": 1}}"#,
        )
        .unwrap();
        assert_eq!(PersistedSessionState::load(dir.path()).view_selection, None);
    }

    #[test]
    fn test_save_round_trip() {
        let dir = tempdir().unwrap();
        let persisted = PersistedSessionState {
            view_selection: instance_selection(7),
        };
        persisted.save(dir.path());
        assert_eq!(PersistedSessionState::load(dir.path()), persisted);
    }

    #[test]
    fn test_persisted_excludes_mfa_methods() {
        let mut state = SessionState::from(PersistedSessionState {
            view_selection: instance_selection(3),
        });
        assert!(state.connection_mfa_method.is_empty());
        state
            .connection_mfa_method
            .insert("location-1".into(), Some(SessionStateMfaMethod::Totp));
        assert_eq!(state.persisted().view_selection, instance_selection(3));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_resolve_keeps_existing_instance(pool: SqlitePool) {
        let id = seed_instance(&pool, "a").await;
        seed_instance(&pool, "b").await;
        let mut persisted = PersistedSessionState {
            view_selection: instance_selection(id),
        };
        assert!(!persisted.resolve(&pool).await);
        assert_eq!(persisted.view_selection, instance_selection(id));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_resolve_keeps_existing_tunnel(pool: SqlitePool) {
        let id = seed_tunnel(&pool).await;
        let selection = Some(OverviewViewSelection {
            kind: ViewSelectionKind::Tunnel,
            id,
        });
        let mut persisted = PersistedSessionState {
            view_selection: selection.clone(),
        };
        assert!(!persisted.resolve(&pool).await);
        assert_eq!(persisted.view_selection, selection);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_resolve_replaces_stale_with_latest_instance(pool: SqlitePool) {
        seed_instance(&pool, "z").await;
        let latest = seed_instance(&pool, "a").await;
        let mut persisted = PersistedSessionState {
            view_selection: instance_selection(latest + 100),
        };
        assert!(persisted.resolve(&pool).await);
        assert_eq!(persisted.view_selection, instance_selection(latest));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_resolve_clears_stale_without_instances(pool: SqlitePool) {
        let mut persisted = PersistedSessionState {
            view_selection: Some(OverviewViewSelection {
                kind: ViewSelectionKind::Tunnel,
                id: 42,
            }),
        };
        assert!(persisted.resolve(&pool).await);
        assert_eq!(persisted.view_selection, None);
    }
}
