#[macro_use]
extern crate log;

use std::{cmp::Ordering, collections::HashSet, str::FromStr};

pub mod commands;

use defguard_client_core::{
    database::{
        models::{
            instance::{mfa_configured_methods, Instance},
            Id,
        },
        DbPool,
    },
    error::Error,
    proxy::post_with_headers,
    version::{CORE_VERSION_HEADER, MIN_CORE_VERSION, MIN_PROXY_VERSION, PROXY_VERSION_HEADER},
};
use defguard_client_proto::defguard::client_types::{InstanceInfoRequest, InstanceInfoResponse};
use futures_util::future::join_all;
use reqwest::{StatusCode, Url};
use semver::Version;
use serde::Serialize;
use sqlx::{types::Json, Sqlite, Transaction};

use crate::commands::{
    disable_enterprise_features, do_update_instance, sync_service_locations_best_effort,
};

static POLLING_ENDPOINT: &str = "/api/v1/poll";

const CORE_CONNECTED_HEADER: &str = "defguard-core-connected";

/// Result of a successful config fetch from the proxy.
#[derive(Debug)]
pub struct FetchedConfig {
    pub response: InstanceInfoResponse,
    pub version_mismatch: Option<VersionMismatchPayload>,
}

/// Result of polling a single instance once.
#[derive(Debug)]
pub enum PollInstanceResult {
    Unchanged {
        version_mismatch: Option<VersionMismatchPayload>,
    },
    Updated {
        locations_changed: bool,
        version_mismatch: Option<VersionMismatchPayload>,
    },
    ChangedWhileActive {
        version_mismatch: Option<VersionMismatchPayload>,
        /// Part of the config was safe to write mid-connection, so the frontend's copy of the
        /// instance is stale even though the rest of the update was deferred.
        instance_updated: bool,
    },
}

/// Outcome of polling a single instance in a batch.
#[derive(Debug)]
pub struct PollInstanceOutcome {
    pub instance_id: Id,
    pub instance_name: String,
    pub result: Result<PollInstanceResult, Error>,
}

/// Payload emitted when a version mismatch is detected.
#[derive(Clone, Debug, Serialize)]
pub struct VersionMismatchPayload {
    pub instance_name: String,
    pub instance_id: Id,
    pub core_version: String,
    pub proxy_version: String,
    pub core_required_version: String,
    pub proxy_required_version: String,
    pub core_compatible: bool,
    pub proxy_compatible: bool,
}

/// Talks to the proxy for a single instance: builds the request, POSTs it,
/// handles 402 PAYMENT_REQUIRED as [`Error::CoreNotEnterprise`], parses the
/// response, and checks the version headers.
///
/// Pure network fetch: touches no database, so callers can run it concurrently.
/// Persisting the 402 state is the caller's job (see [`poll_instance`]).
///
/// Does **not** apply config changes or emit events - those are the caller's
/// responsibility.
pub async fn fetch_instance_config(instance: &Instance<Id>) -> Result<FetchedConfig, Error> {
    debug!("Getting config from core for instance {}", instance.name);

    let request = build_request(instance)?;
    let url = Url::from_str(&instance.proxy_url)
        .and_then(|url| url.join(POLLING_ENDPOINT))
        .map_err(|_| {
            Error::InternalError(format!(
                "Can't build polling url: {}/{POLLING_ENDPOINT}",
                instance.proxy_url
            ))
        })?;
    let response = post_with_headers(url, &request).await.map_err(|err| {
        Error::InternalError(format!(
            "HTTP request failed for instance {}({}), url: {}, {err}",
            instance.name, instance.id, instance.proxy_url
        ))
    })?;
    debug!(
        "Got the following config response for instance {} from core: {response:?}",
        instance.name
    );

    // Enterprise features disabled in core; the caller persists that state
    // serially (it owns the write transaction, this fetch does none).
    if response.status() == StatusCode::PAYMENT_REQUIRED {
        debug!(
            "Instance {}({}) has enterprise features disabled in core.",
            instance.name, instance.id
        );
        return Err(Error::CoreNotEnterprise);
    }

    if !response.status().is_success() {
        return Err(Error::InternalError(format!(
            "Config polling failed for instance {}({}) with status {}",
            instance.name,
            instance.id,
            response.status(),
        )));
    }

    let version_mismatch = check_min_version(&response, instance);

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

    if response.device_config.is_none() {
        return Err(Error::InternalError(
            "Device config not present in response".to_string(),
        ));
    }

    debug!("Parsed the config for instance {}", instance.name);
    trace!("Parsed config: {:?}", response.device_config);

    Ok(FetchedConfig {
        response,
        version_mismatch,
    })
}

/// Polls one instance once and applies changed configuration only when safe.
///
/// The caller owns scheduling, active-connection detection, and user-facing notifications.
pub async fn poll_instance(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
    has_active_connections: bool,
) -> Result<PollInstanceResult, Error> {
    let fetched = fetch_instance_config(instance).await;
    apply_fetched_config(transaction, instance, has_active_connections, fetched).await
}

/// Applies an already-fetched config to the database.
async fn apply_fetched_config(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
    has_active_connections: bool,
    fetch_result: Result<FetchedConfig, Error>,
) -> Result<PollInstanceResult, Error> {
    let fetched = match fetch_result {
        Err(Error::CoreNotEnterprise) if instance.enterprise_enabled => {
            info!(
                "Instance {}({}) has enterprise features disabled, but we have them enabled, \
                disabling.",
                instance.name, instance.id
            );
            disable_enterprise_features(instance, transaction.as_mut()).await?;
            return Err(Error::CoreNotEnterprise);
        }
        fetched => fetched?,
    };
    let version_mismatch = fetched.version_mismatch;

    let device_config =
        fetched.response.device_config.as_ref().ok_or_else(|| {
            Error::InternalError("Device config not present in response".to_string())
        })?;
    if !config_changed(transaction, instance, device_config).await? {
        debug!(
            "Config for instance {}({}) didn't change",
            instance.name, instance.id
        );
        return Ok(PollInstanceResult::Unchanged { version_mismatch });
    }

    debug!(
        "Config for instance {}({}) changed",
        instance.name, instance.id
    );

    if has_active_connections {
        let mut instance_updated = false;
        if let Some(ref info) = device_config.instance {
            // add dedicated override to disable tunnels without waiting for a disconnect
            let new_tunnels_disabled = info.disable_tunnels.unwrap_or(false);
            if new_tunnels_disabled && !instance.disable_tunnels {
                debug!(
                    "Tunnels were disabled for instance {}({}) while a connection is active, \
                    persisting the flag immediately.",
                    instance.name, instance.id
                );
                instance.disable_tunnels = true;
                instance_updated = true;
            }
            // Says nothing about the tunnel, and deferring it would keep the instance unable to
            // configure MFA for as long as the VPN stayed up.
            let configured_methods = mfa_configured_methods(info).map(Json);
            if instance.mfa_configured_methods.as_ref().map(|json| &json.0)
                != configured_methods.as_ref().map(|json| &json.0)
            {
                debug!(
                    "MFA state changed for instance {}({}) while a connection is active, \
                    persisting the snapshot immediately.",
                    instance.name, instance.id
                );
                instance.mfa_configured_methods = configured_methods;
                instance_updated = true;
            }
            if instance_updated {
                instance.save(transaction.as_mut()).await?;
            }
        }
        return Ok(PollInstanceResult::ChangedWhileActive {
            version_mismatch,
            instance_updated,
        });
    }

    debug!(
        "Updating instance {}({}) configuration: {device_config:?}",
        instance.name, instance.id,
    );
    let locations_changed =
        do_update_instance(transaction, instance, device_config.clone()).await?;
    info!(
        "Updated instance {}({}) configuration based on core's response",
        instance.name, instance.id
    );

    Ok(PollInstanceResult::Updated {
        locations_changed,
        version_mismatch,
    })
}

/// Polls all instances that have a polling token and commits any safe configuration updates.
///
/// Fetches run concurrently. Configuration updates commit in one transaction,
/// in instance order. The caller handles active-connection checks and
/// user-facing effects.
pub async fn poll_instances(
    pool: &DbPool,
    active_instance_ids: &HashSet<Id>,
) -> Result<Vec<PollInstanceOutcome>, Error> {
    let mut instances = Instance::all_with_token(pool).await?;
    let fetch_results = join_all(instances.iter().map(fetch_instance_config)).await;

    let mut transaction = pool.begin().await?;
    let mut outcomes = Vec::with_capacity(instances.len());

    for (instance, fetch_result) in instances.iter_mut().zip(fetch_results) {
        let has_active_connections = active_instance_ids.contains(&instance.id);
        let instance_id = instance.id;
        let result = apply_fetched_config(
            &mut transaction,
            instance,
            has_active_connections,
            fetch_result,
        )
        .await;
        outcomes.push(PollInstanceOutcome {
            instance_id,
            instance_name: instance.name.clone(),
            result,
        });
    }

    transaction.commit().await?;

    // Push to the daemon only after committing to avoid hanging transactions across grpc calls.
    for instance in &instances {
        sync_service_locations_best_effort(pool, instance).await;
    }

    Ok(outcomes)
}

/// Checks if config has changed compared to what's in the database.
pub async fn config_changed(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &Instance<Id>,
    device_config: &defguard_client_proto::defguard::client_types::DeviceConfigResponse,
) -> Result<bool, Error> {
    debug!(
        "Checking if config and any of the locations changed for instance {}({})",
        instance.name, instance.id
    );
    let locations_changed =
        commands::locations_changed(transaction, instance, device_config).await?;
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

/// Checks response headers for version compatibility.
/// Returns `Some(payload)` when versions are incompatible, `None` when
/// everything is compatible or headers are missing.
fn check_min_version(
    response: &reqwest::Response,
    instance: &Instance<Id>,
) -> Option<VersionMismatchPayload> {
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
            if let Ok(core_version) = Version::from_str(core_version) {
                detected_core_version = core_version.to_string();
                core_version.cmp_precedence(&MIN_CORE_VERSION) != Ordering::Less
            } else {
                warn!(
                    "Core version header: invalid semver string in response for instance {}({}): \
                    '{core_version}'",
                    instance.name, instance.id
                );
                detected_core_version = core_version.to_string();
                false
            }
        } else {
            warn!(
                "Core version header: invalid string in response for instance {}({}): \
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
            if let Ok(proxy_version) = Version::from_str(proxy_version) {
                detected_proxy_version = proxy_version.to_string();
                proxy_version.cmp_precedence(&MIN_PROXY_VERSION) != Ordering::Less
            } else {
                warn!(
                    "Proxy version header not a valid semver string in response for instance \
                        {}({}): '{proxy_version}'",
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
            "Instance {} is running incompatible versions: core {detected_core_version}, proxy \
            {detected_proxy_version}. Required versions: core >= {MIN_CORE_VERSION}, proxy >= \
            {MIN_PROXY_VERSION}",
            instance.name,
        );

        Some(VersionMismatchPayload {
            instance_name: instance.name.clone(),
            instance_id: instance.id,
            core_version: detected_core_version,
            proxy_version: detected_proxy_version,
            core_required_version: MIN_CORE_VERSION.to_string(),
            proxy_required_version: MIN_PROXY_VERSION.to_string(),
            core_compatible,
            proxy_compatible,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
