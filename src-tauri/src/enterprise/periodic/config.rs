use std::{str::FromStr, time::Duration};

use reqwest::StatusCode;
use sqlx::{Sqlite, Transaction};
use tauri::{AppHandle, Manager, State, Url};
use tokio::time::sleep;

use crate::{
    appstate::AppState,
    commands::{do_update_instance, locations_changed},
    database::{models::Id, Instance},
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
        if let Err(err) = handle.emit_all(INSTANCE_UPDATE, ()) {
            error!("Failed to emit instance update event: {err}");
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

    // Return early if the enterprise features are disabled in the core
    if response.status() == StatusCode::PAYMENT_REQUIRED {
        debug!(
            "Instance {}({}) has enterprise features disabled, checking if this state is reflected on our end...",
            instance.name, instance.id
        );
        if instance.enterprise_enabled {
            info!(
                "Instance {}({}) has enterprise features disabled, but we have them enabled, disabling...",
                instance.name, instance.id
            );
            instance
                .disable_enterprise_features(transaction.as_mut())
                .await?;
        } else {
            debug!(
                "Instance {}({}) has enterprise features disabled, and we have them disabled as well, no action needed",
                instance.name, instance.id
            );
        }
        return Ok(());
    }

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

async fn config_changed(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &Instance<Id>,
    device_config: &DeviceConfigResponse,
) -> Result<bool, Error> {
    debug!(
        "Checking if config changed for instance {}({})",
        instance.name, instance.id
    );
    let locations_changed = locations_changed(transaction, instance, device_config).await?;
    let info_changed = match &device_config.instance {
        Some(info) => instance != info,
        None => false,
    };
    debug!(
        "Did the locations change: {}. Did the instance information change: {}",
        locations_changed, info_changed
    );
    Ok(locations_changed || info_changed)
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
