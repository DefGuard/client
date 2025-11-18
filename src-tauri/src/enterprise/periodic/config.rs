use std::{
    cmp::Ordering,
    collections::HashSet,
    str::FromStr,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use reqwest::{Client, StatusCode};
use serde::Serialize;
use sqlx::{Sqlite, Transaction};
use tauri::{AppHandle, Emitter, Url};
use tokio::time::sleep;

use crate::{
    active_connections::active_connections,
    commands::{do_update_instance, locations_changed},
    database::{
        models::{instance::Instance, Id},
        DB_POOL,
    },
    error::Error,
    events::EventKey,
    proto::{DeviceConfigResponse, InstanceInfoRequest, InstanceInfoResponse},
    utils::construct_platform_header,
    CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, MIN_CORE_VERSION, MIN_PROXY_VERSION,
    PKG_VERSION,
};

const INTERVAL_SECONDS: Duration = Duration::from_secs(30);
const HTTP_REQ_TIMEOUT: Duration = Duration::from_secs(5);
static POLLING_ENDPOINT: &str = "/api/v1/poll";

/// Periodically retrieves and updates configuration for all [`Instance`]s.
/// Updates are only performed if no connections are established to the [`Instance`],
/// otherwise event is emmited and UI message is displayed.
pub async fn poll_config(handle: AppHandle) {
    debug!("Starting the configuration polling loop.");
    // Polling starts sooner than app's frontend may load in dev builds, causing events (toasts) to be lost,
    // you may want to wait here before starting if you want to debug it.
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
                "Failed to retireve instances for config polling, retrying in {}s",
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
                if let Err(err) = poll_instance(&mut transaction, instance, &handle).await {
                    match err {
                        Error::CoreNotEnterprise => {
                            debug!(
                                "Tried to contact core for instance {instance} config but it's not \
                                enterprise, can't retrieve config"
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

/// Retrieves configuration for given [`Instance`].
/// Updates the instance if there aren't any active connections, otherwise displays UI message.
pub async fn poll_instance(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
    handle: &AppHandle,
) -> Result<(), Error> {
    debug!("Getting config from core for instance {}", instance.name);
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
    let response = Client::new()
        .post(url)
        .json(&request)
        .header(CLIENT_VERSION_HEADER, PKG_VERSION)
        .header(CLIENT_PLATFORM_HEADER, construct_platform_header())
        .timeout(HTTP_REQ_TIMEOUT)
        .send()
        .await;
    let response = response.map_err(|err| {
        Error::InternalError(format!(
            "HTTP request failed for instance {}({}), url: {}, {err}",
            instance.name, instance.id, instance.proxy_url
        ))
    })?;
    debug!(
        "Got the following config response for instance {} from core: {response:?}",
        instance.name
    );

    check_min_version(&response, instance, handle)?;

    // Return early if the enterprise features are disabled in the core
    if response.status() == StatusCode::PAYMENT_REQUIRED {
        debug!(
            "Instance {}({}) has enterprise features disabled, checking if this state is reflected \
            on our end.",
            instance.name, instance.id
        );
        if instance.enterprise_enabled {
            info!(
                "Instance {}({}) has enterprise features disabled, but we have them enabled, \
                disabling.",
                instance.name, instance.id
            );
            instance
                .disable_enterprise_features(transaction.as_mut())
                .await?;
        } else {
            debug!(
                "Instance {}({}) has enterprise features disabled, and we have them disabled as \
                well, no action needed",
                instance.name, instance.id
            );
        }
        return Err(Error::CoreNotEnterprise);
    }

    // Parse the response
    debug!(
        "Parsing the config response for instance {}.",
        instance.name
    );
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
    debug!("Parsed the config for instance {}", instance.name);
    trace!("Parsed config: {device_config:?}");

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
    //
    if active_connections(instance).await?.is_empty() {
        debug!(
            "Updating instance {}({}) configuration: {device_config:?}",
            instance.name, instance.id,
        );
        do_update_instance(transaction, instance, device_config.clone()).await?;
        info!(
            "Updated instance {}({}) configuration based on core's response",
            instance.name, instance.id
        );
    } else {
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

    Ok(())
}

async fn config_changed(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &Instance<Id>,
    device_config: &DeviceConfigResponse,
) -> Result<bool, Error> {
    debug!(
        "Checking if config and any of the locations changed for instance {}({})",
        instance.name, instance.id
    );
    let locations_changed = locations_changed(transaction, instance, device_config).await?;
    let info_changed = match &device_config.instance {
        Some(info) => instance != info,
        None => false,
    };
    debug!(
        "Did the locations change?: {locations_changed}. Did the instance information change?: \
        {info_changed}"
    );
    Ok(locations_changed || info_changed)
}

/// Retrieves token to build InstanceInfoRequest
fn build_request(instance: &Instance<Id>) -> Result<InstanceInfoRequest, Error> {
    let token = instance.token.as_ref().ok_or_else(|| Error::NoToken)?;

    Ok(InstanceInfoRequest {
        token: (*token).clone(),
    })
}

/// Tracks instance IDs that for which we already sent notification about version mismatches
/// to prevent duplicate notifications in the app's lifetime.
static NOTIFIED_INSTANCES: LazyLock<Mutex<HashSet<Id>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

const CORE_VERSION_HEADER: &str = "defguard-core-version";
const CORE_CONNECTED_HEADER: &str = "defguard-core-connected";
const PROXY_VERSION_HEADER: &str = "defguard-component-version";

#[derive(Clone, Serialize)]
struct VersionMismatchPayload {
    instance_name: String,
    instance_id: Id,
    core_version: String,
    proxy_version: String,
    core_required_version: String,
    proxy_required_version: String,
    core_compatible: bool,
    proxy_compatible: bool,
}

fn check_min_version(
    response: &reqwest::Response,
    instance: &Instance<Id>,
    handle: &AppHandle,
) -> Result<(), Error> {
    let mut notified_instances = NOTIFIED_INSTANCES.lock().unwrap();
    if notified_instances.contains(&instance.id) {
        debug!(
            "Instance {}({}) already notified about version mismatch, skipping",
            instance.name, instance.id
        );
        return Ok(());
    }

    let detected_core_version: String;
    let detected_proxy_version: String;
    let defguard_core_connected: Option<bool> = response
        .headers()
        .get(CORE_CONNECTED_HEADER)
        .and_then(|v| {
            debug!(
                "Defguard core connection status header for instance {}({}): {v:?}",
                instance.name, instance.id
            );
            v.to_str().ok()
        })
        .and_then(|s| s.parse().ok());

    let core_compatible = if let Some(core_version) = response.headers().get(CORE_VERSION_HEADER) {
        if let Ok(core_version) = core_version.to_str() {
            if let Ok(core_version) = semver::Version::from_str(core_version) {
                detected_core_version = core_version.to_string();
                core_version.cmp_precedence(&MIN_CORE_VERSION) != Ordering::Less
            } else {
                warn!(
                    "Core version header not a valid semver string in response for instance {}({}): \
                    '{core_version}'",
                    instance.name, instance.id
                );
                detected_core_version = core_version.to_string();
                false
            }
        } else {
            warn!(
                "Core version header not a valid string in response for instance {}({}): \
                '{core_version:?}'",
                instance.name, instance.id
            );
            detected_core_version = "unknown".to_string();
            false
        }
    } else {
        warn!(
            "Core version header not present in response for instance {}({})",
            instance.name, instance.id
        );
        detected_core_version = "unknown".to_string();
        false
    };

    let proxy_compatible = if let Some(proxy_version) = response.headers().get(PROXY_VERSION_HEADER)
    {
        if let Ok(proxy_version) = proxy_version.to_str() {
            if let Ok(proxy_version) = semver::Version::from_str(proxy_version) {
                detected_proxy_version = proxy_version.to_string();
                proxy_version.cmp_precedence(&MIN_PROXY_VERSION) != Ordering::Less
            } else {
                warn!(
                    "Proxy version header not a valid semver string in response for instance {}({}): \
                    '{proxy_version}'",
                    instance.name, instance.id
                );
                detected_proxy_version = proxy_version.to_string();
                false
            }
        } else {
            warn!(
                "Proxy version header not a valid string in response for instance {}({}): \
                '{proxy_version:?}'",
                instance.name, instance.id
            );
            detected_proxy_version = "unknown".to_string();
            false
        }
    } else {
        warn!(
            "Proxy version header not present in response for instance {}({})",
            instance.name, instance.id
        );
        detected_proxy_version = "unknown".to_string();
        false
    };

    let should_inform = match defguard_core_connected {
        Some(true) => {
            debug!(
                "Defguard core is connected for instance {}({})",
                instance.name, instance.id
            );
            true
        }
        Some(false) => {
            info!(
                "Defguard core is not connected for instance {}({})",
                instance.name, instance.id
            );
            false
        }
        None => {
            debug!(
                "Defguard core connection status unknown for instance {}({})",
                instance.name, instance.id
            );
            true
        }
    };

    if should_inform && (!core_compatible || !proxy_compatible) {
        warn!(
                "Instance {} is running incompatible versions: core {detected_core_version}, proxy {detected_proxy_version}. Required \
                versions: core >= {MIN_CORE_VERSION}, proxy >= {MIN_PROXY_VERSION}",
                instance.name,
            );

        let payload = VersionMismatchPayload {
            instance_name: instance.name.clone(),
            instance_id: instance.id,
            core_version: detected_core_version,
            proxy_version: detected_proxy_version,
            core_required_version: MIN_CORE_VERSION.to_string(),
            proxy_required_version: MIN_PROXY_VERSION.to_string(),
            core_compatible,
            proxy_compatible,
        };
        if let Err(err) = handle.emit(EventKey::VersionMismatch.into(), payload) {
            error!("Failed to emit version mismatch event to the frontend: {err}");
        } else {
            notified_instances.insert(instance.id);
        }
    }

    Ok(())
}
