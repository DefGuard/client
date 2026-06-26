use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
    time::Duration,
};

pub use defguard_client_config_sync::commands::{
    disable_enterprise_features, do_update_instance, locations_changed,
};
use defguard_client_config_sync::{poll_instance, PollInstanceResult};
use defguard_client_core::{
    connection::active_connections::active_connections,
    database::{
        models::{instance::Instance, Id},
        DB_POOL,
    },
    error::Error,
    events::EventKey,
};
use log::{debug, error, info};
use sqlx::{Sqlite, Transaction};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

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
        let Ok(mut transaction) = DB_POOL.begin().await else {
            error!(
                "Failed to begin database transaction for config polling, retrying in {}s",
                INTERVAL_SECONDS.as_secs()
            );
            sleep(INTERVAL_SECONDS).await;
            continue;
        };
        let Ok(mut instances) = Instance::all_with_token(&mut *transaction).await else {
            error!(
                "Failed to retrieve instances for config polling, retrying in {}s",
                INTERVAL_SECONDS.as_secs()
            );
            let _ = transaction.rollback().await;
            sleep(INTERVAL_SECONDS).await;
            continue;
        };
        debug!(
            "Found {} instances with a config polling token, proceeding with polling their \
            configuration.",
            instances.len()
        );
        let mut config_retrieved = 0;
        for instance in &mut instances {
            if instance.token.is_some() {
                if let Err(err) =
                    poll_instance_with_events(&mut transaction, instance, &handle).await
                {
                    match err {
                        Error::CoreNotEnterprise => {
                            debug!(
                                "Tried to contact core for instance {instance} config but it's \
                                not enterprise, can't retrieve config"
                            );
                        }
                        Error::NoToken => {
                            debug!(
                                "Instance {instance} has no token, can't retrieve its config from \
                                the core",
                            );
                        }
                        _ => {
                            error!(
                                "Failed to retrieve instance {instance} config from core: {err}"
                            );
                        }
                    }
                } else {
                    config_retrieved += 1;
                    debug!(
                        "Finished processing configuration polling request for instance {instance}"
                    );
                }
            }
        }
        if let Err(err) = transaction.commit().await {
            error!(
                "Failed to commit config polling transaction, configuration won't be updated: \
                {err}"
            );
        }
        if let Err(err) = handle.emit(EventKey::InstanceUpdate.into(), ()) {
            error!("Failed to emit instance update event to the frontend: {err}");
        }
        if config_retrieved > 0 {
            info!(
                "Automatically retrieved the newest instance configuration from core for \
                {config_retrieved} instances, sleeping for {}s",
                INTERVAL_SECONDS.as_secs(),
            );
            debug!("Instances for which configuration was retrieved from core: {instances:?}");
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

    let version_mismatch = match &result {
        PollInstanceResult::Unchanged { version_mismatch }
        | PollInstanceResult::Updated {
            version_mismatch, ..
        }
        | PollInstanceResult::ChangedWhileActive { version_mismatch } => version_mismatch,
    };

    // Emit version-mismatch event if applicable and not already notified
    if let Some(payload) = version_mismatch {
        let mut notified_instances = NOTIFIED_INSTANCES.lock().unwrap();
        if notified_instances.insert(instance.id) {
            if let Err(err) = handle.emit(EventKey::VersionMismatch.into(), payload.clone()) {
                error!("Failed to emit version mismatch event to the frontend: {err}");
                // Remove so we can retry next cycle
                notified_instances.remove(&instance.id);
            }
        }
    }

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
                instance.name, instance.id,
            );
            let _ = handle.emit(EventKey::ConfigChanged.into(), &instance.name);
            info!(
                "Emitted config-changed event for instance {}({})",
                instance.name, instance.id,
            );
        }
    }

    Ok(())
}
