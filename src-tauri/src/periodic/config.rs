use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex},
    time::Duration,
};

use defguard_client_config_sync::{
    poll_instance, poll_instances, PollInstanceResult, VersionMismatchPayload,
};
use defguard_client_core::{
    connection::active_connections::{active_connections, ACTIVE_CONNECTIONS},
    database::{
        models::{instance::Instance, location::Location, Id},
        DB_POOL,
    },
    error::Error,
    events::EventKey,
    ConnectionType,
};
use log::{debug, error, info};
use sqlx::{Sqlite, Transaction};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

pub use defguard_client_config_sync::commands::{
    disable_enterprise_features, do_update_instance, locations_changed,
};

const INTERVAL_SECONDS: Duration = Duration::from_secs(30);

/// Tracks instance IDs for which we already sent a version-mismatch notification,
/// to prevent duplicate notifications in the app's lifetime.
static NOTIFIED_INSTANCES: LazyLock<Mutex<HashSet<Id>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Periodically retrieves and updates configuration for all [`Instance`]s.
/// Updates are only performed if no connections are established to the [`Instance`],
/// otherwise event is emitted and UI message is displayed.
pub async fn poll_config(handle: AppHandle) {
    debug!("Starting the configuration polling loop.");
    // Polling starts sooner than app's frontend may load in dev builds, causing events (toasts)
    // to be lost; you may want to wait here before starting if you want to debug it.
    loop {
        let active_instance_ids = match active_instance_ids().await {
            Ok(ids) => ids,
            Err(err) => {
                error!(
                    "Failed to detect active instances for config polling, retrying in {}s: {err}",
                    INTERVAL_SECONDS.as_secs()
                );
                sleep(INTERVAL_SECONDS).await;
                continue;
            }
        };

        let outcomes = match poll_instances(&DB_POOL, &active_instance_ids).await {
            Ok(outcomes) => outcomes,
            Err(err) => {
                error!(
                    "Failed to poll instance configuration, retrying in {}s: {err}",
                    INTERVAL_SECONDS.as_secs()
                );
                sleep(INTERVAL_SECONDS).await;
                continue;
            }
        };

        debug!(
            "Found {} instances with a config polling token, processed configuration polling.",
            outcomes.len()
        );

        let mut config_retrieved = 0;
        for outcome in outcomes {
            let instance_name = outcome.instance_name;
            let instance_id = outcome.instance_id;
            match outcome.result {
                Ok(result) => {
                    config_retrieved += 1;
                    emit_version_mismatch(&handle, instance_id, version_mismatch(&result));
                    emit_poll_result_events(&handle, instance_id, &instance_name, result);
                    debug!(
                        "Finished processing configuration polling request for instance {}(ID: {})",
                        instance_name, instance_id
                    );
                }
                Err(Error::CoreNotEnterprise) => {
                    debug!(
                        "Tried to contact core for instance {}(ID: {}) config but it's not enterprise, can't retrieve config",
                        instance_name, instance_id
                    );
                }
                Err(Error::NoToken) => {
                    debug!(
                        "Instance {}(ID: {}) has no token, can't retrieve its config from the core",
                        instance_name, instance_id,
                    );
                }
                Err(err) => {
                    error!(
                        "Failed to retrieve instance {}(ID: {}) config from core: {err}",
                        instance_name, instance_id
                    );
                }
            }
        }

        if let Err(err) = handle.emit(EventKey::InstanceUpdate.into(), ()) {
            error!("Failed to emit instance update event to the frontend: {err}");
        }
        if config_retrieved > 0 {
            info!(
                "Automatically retrieved the newest instance configuration from core for {config_retrieved} instances, sleeping for {}s",
                INTERVAL_SECONDS.as_secs(),
            );
        } else {
            debug!(
                "No configuration updates retrieved, sleeping {}s",
                INTERVAL_SECONDS.as_secs(),
            );
        }
        sleep(INTERVAL_SECONDS).await;
    }
}

/// Retrieves configuration for a given [`Instance`].
/// Updates the instance if there aren't any active connections, otherwise emits
/// a ConfigChanged event so the frontend can prompt the user to reconnect.
pub async fn poll_instance_with_events(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
    handle: &AppHandle,
) -> Result<(), Error> {
    let has_active_connections = !active_connections(instance).await?.is_empty();
    let result = poll_instance(transaction, instance, has_active_connections).await?;

    emit_version_mismatch(handle, instance.id, version_mismatch(&result));
    emit_poll_result_events(handle, instance.id, &instance.name, result);

    Ok(())
}

fn emit_version_mismatch(
    handle: &AppHandle,
    instance_id: Id,
    payload: Option<&VersionMismatchPayload>,
) {
    if let Some(payload) = payload {
        let mut notified_instances = NOTIFIED_INSTANCES.lock().unwrap();
        if notified_instances.insert(instance_id) {
            if let Err(err) = handle.emit(EventKey::VersionMismatch.into(), payload.clone()) {
                error!("Failed to emit version mismatch event to the frontend: {err}");
                // Remove so we can retry next cycle.
                notified_instances.remove(&instance_id);
            }
        }
    }
}

fn emit_poll_result_events(
    handle: &AppHandle,
    instance_id: Id,
    instance_name: &str,
    result: PollInstanceResult,
) {
    match result {
        PollInstanceResult::Unchanged { .. } => {}
        PollInstanceResult::Updated {
            locations_changed, ..
        } => {
            if locations_changed {
                if let Err(err) = handle.emit(EventKey::InstanceUpdated.into(), ()) {
                    error!("Failed to emit instance-updated event: {err}");
                }
            }
        }
        PollInstanceResult::ChangedWhileActive { .. } => {
            debug!(
                "Emitting config-changed event for instance {}({})",
                instance_name, instance_id,
            );
            let _ = handle.emit(EventKey::ConfigChanged.into(), instance_name);
            info!(
                "Emitted config-changed event for instance {}({})",
                instance_name, instance_id,
            );
        }
    }
}

fn version_mismatch(result: &PollInstanceResult) -> Option<&VersionMismatchPayload> {
    match result {
        PollInstanceResult::Unchanged { version_mismatch }
        | PollInstanceResult::Updated {
            version_mismatch, ..
        }
        | PollInstanceResult::ChangedWhileActive { version_mismatch } => version_mismatch.as_ref(),
    }
}

async fn active_instance_ids() -> Result<HashSet<Id>, Error> {
    let active_location_ids = ACTIVE_CONNECTIONS
        .lock()
        .await
        .iter()
        .filter(|connection| connection.connection_type == ConnectionType::Location)
        .map(|connection| connection.location_id)
        .collect::<HashSet<_>>();

    if active_location_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let location_instances = Location::all(&*DB_POOL, false)
        .await?
        .into_iter()
        .map(|location| (location.id, location.instance_id))
        .collect::<HashMap<_, _>>();

    Ok(active_location_ids
        .into_iter()
        .filter_map(|location_id| location_instances.get(&location_id).copied())
        .collect())
}
