use std::{collections::HashSet, str::FromStr, time::Duration};

use sqlx::{Sqlite, Transaction};
use tauri::{AppHandle, Manager, State, Url};
use tokio::time::sleep;

use crate::{
    appstate::AppState,
    commands::{device_config_to_location, do_update_instance},
    database::{
        models::{Id, NoId},
        Instance, Location,
    },
    error::Error,
    events::{CONFIG_CHANGED, INSTANCE_UPDATE},
    proto::{DeviceConfigResponse, InstanceInfoRequest, InstanceInfoResponse},
};

const INTERVAL_SECONDS: Duration = Duration::from_secs(30);
const POLLING_ENDPOINT: &str = "/api/v1/poll";

/// Periodically retrieves and updates configuration for all [`Instance`]s.
/// Updates are only performed if no connections are established to the [`Instance`],
/// otherwise event is emmited and UI message is displayed.
pub async fn poll_config(handle: AppHandle) {
    let state: State<AppState> = handle.state();
    let pool = state.get_pool();
    loop {
        let Ok(mut transaction) = pool.begin().await else {
            error!(
                "Failed to begin db transaction, retrying in {}s",
                INTERVAL_SECONDS.as_secs()
            );
            sleep(INTERVAL_SECONDS).await;
            continue;
        };
        let Ok(mut instances) = Instance::all(&mut *transaction).await else {
            error!(
                "Failed to retireve instances, retrying in {}s",
                INTERVAL_SECONDS.as_secs()
            );
            let _ = transaction.rollback().await;
            sleep(INTERVAL_SECONDS).await;
            continue;
        };
        debug!(
            "Polling configuration updates for {} instances",
            instances.len(),
        );
        for instance in &mut instances {
            if let Err(err) = poll_instance(&mut transaction, instance, &handle).await {
                error!(
                    "Failed to retrieve instance {}({}) config: {err}",
                    instance.name, instance.id,
                );
            } else {
                debug!(
                    "Retrieved config for instance {}({})",
                    instance.name, instance.id,
                );
            }
        }
        if let Err(err) = transaction.commit().await {
            error!("Failed to commit config polling transaction, configuration won't be updated: {err}");
        }
        info!(
            "Retrieved configuration for {} instances, sleeping {}s",
            instances.len(),
            INTERVAL_SECONDS.as_secs(),
        );
        sleep(INTERVAL_SECONDS).await;
    }
}

/// Retrieves configuration for given [`Instance`].
/// Updates the instance if there aren't any active connections, otherwise displays UI message.
pub async fn poll_instance(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
    handle: &AppHandle,
) -> Result<(), Error> {
    // Query proxy api
    let request = build_request(instance)?;
    let url = Url::from_str(&instance.proxy_url)
        .and_then(|url| url.join(POLLING_ENDPOINT))
        .map_err(|_| {
            Error::InternalError(format!(
                "Can't build polling url: {}/{POLLING_ENDPOINT}",
                instance.proxy_url
            ))
        })?;
    let response = reqwest::Client::new()
        .post(url)
        .json(&request)
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    let response = response.map_err(|err| {
        Error::InternalError(format!(
            "HTTP request failed for instance {}({}), url: {}, {err}",
            instance.name, instance.id, instance.proxy_url
        ))
    })?;
    debug!("InstanceInfoResponse: {response:?}");

    // Parse the response
    let response: InstanceInfoResponse = response.json().await.map_err(|err| {
        Error::InternalError(format!(
            "Failed to parse InstanceInfoResponse for instance {}({}): {err}",
            instance.name, instance.id,
        ))
    })?;
    let device_config = response
        .device_config
        .as_ref()
        .ok_or_else(|| Error::InternalError("Device config not present in response".to_string()))?;
    let instance_config = response.instance_config.as_ref().ok_or_else(|| {
        Error::InternalError("Instance config not present in response".to_string())
    })?;

    if instance.disable_all_traffic != instance_config.disable_all_traffic {
        debug!(
            "Instance {}({}) disable_all_traffic changed from {} to {}",
            instance.name,
            instance.id,
            instance.disable_all_traffic,
            instance_config.disable_all_traffic
        );
        instance.disable_all_traffic = instance_config.disable_all_traffic;
        instance.save(&mut **transaction).await?;
        handle.emit_all(INSTANCE_UPDATE, ())?;
    }

    // Early return if config didn't change
    if !config_changed(transaction, instance, device_config).await? {
        debug!(
            "Config for instance {}({}) didn't change",
            instance.name, instance.id
        );
        return Ok(());
    }

    debug!(
        "Config for instance {}({}) changed",
        instance.name, instance.id
    );

    // Config changed. If there are no active connections for this instance, update the database.
    // Otherwise just display a message to reconnect.
    let state: State<'_, AppState> = handle.state();
    if state.active_connections(instance).await?.is_empty() {
        debug!(
            "Updating instance {}({}) configuration: {device_config:?}",
            instance.name, instance.id,
        );
        do_update_instance(transaction, instance, device_config.clone()).await?;
        handle.emit_all(INSTANCE_UPDATE, ())?;
        info!(
            "Updated instance {}({}) configuration",
            instance.name, instance.id
        );
    } else {
        debug!(
            "Emitting config-changed event for instance {}({})",
            instance.name, instance.id,
        );
        let _ = handle.emit_all(CONFIG_CHANGED, &instance.name);
        info!(
            "Emitted config-changed event for instance {}({})",
            instance.name, instance.id,
        );
    }

    Ok(())
}

/// Returns true if configuration in instance_info differs from current configuration
async fn config_changed(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &Instance<Id>,
    device_config: &DeviceConfigResponse,
) -> Result<bool, Error> {
    let db_locations: Vec<Location<NoId>> =
        Location::find_by_instance_id(transaction.as_mut(), instance.id)
            .await?
            .into_iter()
            .map(Location::<NoId>::from)
            .collect();
    let db_locations: HashSet<Location<NoId>> = HashSet::from_iter(db_locations);
    let core_locations: Vec<Location<NoId>> = device_config
        .configs
        .iter()
        .map(|config| device_config_to_location(config.clone(), instance.id))
        .map(Location::<NoId>::from)
        .collect();
    let core_locations: HashSet<Location<NoId>> = HashSet::from_iter(core_locations);

    Ok(db_locations != core_locations)
}

/// Retrieves pubkey & token to build InstanceInfoRequest
fn build_request(instance: &Instance<Id>) -> Result<InstanceInfoRequest, Error> {
    let token = &instance.token.as_ref().ok_or_else(|| {
        Error::InternalError(format!(
            "Instance {}({}) missing token",
            instance.name, instance.id
        ))
    })?;

    Ok(InstanceInfoRequest {
        token: (*token).to_string(),
    })
}
