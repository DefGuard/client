#[macro_use]
extern crate log;

use std::{cmp::Ordering, str::FromStr};

pub mod commands;

use defguard_client_core::{
    database::models::{instance::Instance, Id},
    error::Error,
    proxy::post_with_headers,
    version::{MIN_CORE_VERSION, MIN_PROXY_VERSION},
};
use defguard_client_proto::defguard::client_types::{InstanceInfoRequest, InstanceInfoResponse};
use reqwest::{StatusCode, Url};
use semver::Version;
use serde::Serialize;
use sqlx::{Sqlite, Transaction};

use crate::commands::disable_enterprise_features;

static POLLING_ENDPOINT: &str = "/api/v1/poll";

const CORE_VERSION_HEADER: &str = "defguard-core-version";
const CORE_CONNECTED_HEADER: &str = "defguard-core-connected";
const PROXY_VERSION_HEADER: &str = "defguard-component-version";

/// Result of a successful config fetch from the proxy.
#[derive(Debug)]
pub struct FetchedConfig {
    pub response: InstanceInfoResponse,
    pub version_mismatch: Option<VersionMismatchPayload>,
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
/// handles 402 PAYMENT_REQUIRED by disabling enterprise features, parses the
/// response, and checks the version headers.
///
/// Does **not** apply config changes or emit events - those are the caller's
/// responsibility.
pub async fn fetch_instance_config(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
) -> Result<FetchedConfig, Error> {
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
            disable_enterprise_features(instance, transaction.as_mut()).await?;
        } else {
            debug!(
                "Instance {}({}) has enterprise features disabled, and we have them disabled as \
                well, no action needed",
                instance.name, instance.id
            );
        }
        return Err(Error::CoreNotEnterprise);
    }

    ensure_success_status(&response, instance)?;

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

fn ensure_success_status(
    response: &reqwest::Response,
    instance: &Instance<Id>,
) -> Result<(), Error> {
    if response.status().is_success() {
        return Ok(());
    }

    Err(Error::HttpError(format!(
        "Config polling failed for instance {}({}) with status {}",
        instance.name,
        instance.id,
        response.status(),
    )))
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
mod tests {
    use defguard_client_core::database::models::instance::ClientTrafficPolicy;

    use super::*;

    fn instance_with_token(token: Option<&str>) -> Instance<Id> {
        Instance {
            id: 1,
            name: "inst".into(),
            uuid: "uuid".into(),
            url: "https://core".into(),
            proxy_url: "https://proxy".into(),
            username: "alice".into(),
            token: token.map(str::to_string),
            client_traffic_policy: ClientTrafficPolicy::None,
            enterprise_enabled: false,
            openid_display_name: None,
        }
    }

    fn response_with_headers(headers: &[(&str, &str)]) -> reqwest::Response {
        response_with_status_and_headers(StatusCode::OK, headers)
    }

    fn response_with_status_and_headers(
        status: StatusCode,
        headers: &[(&str, &str)],
    ) -> reqwest::Response {
        let mut builder = http::Response::builder();
        builder = builder.status(status);
        for (key, value) in headers {
            builder = builder.header(*key, *value);
        }
        reqwest::Response::from(builder.body(String::new()).unwrap())
    }

    #[test]
    fn test_build_request_no_token_errors() {
        let instance = instance_with_token(None);
        assert!(matches!(build_request(&instance), Err(Error::NoToken)));
    }

    #[test]
    fn test_build_request_includes_token() {
        let instance = instance_with_token(Some("tok"));
        let request = build_request(&instance).unwrap();
        assert_eq!(request.token, "tok");
    }

    #[test]
    fn test_check_min_version_compatible_returns_none() {
        let response = response_with_headers(&[
            (CORE_VERSION_HEADER, "1.6.0"),
            (PROXY_VERSION_HEADER, "1.6.0"),
        ]);
        let instance = instance_with_token(Some("tok"));
        assert!(check_min_version(&response, &instance).is_none());
    }

    #[test]
    fn test_check_min_version_incompatible_core() {
        let response = response_with_headers(&[
            (CORE_VERSION_HEADER, "1.0.0"),
            (PROXY_VERSION_HEADER, "1.6.0"),
        ]);
        let instance = instance_with_token(Some("tok"));
        let payload = check_min_version(&response, &instance).expect("mismatch expected");
        assert!(!payload.core_compatible);
        assert!(payload.proxy_compatible);
        assert_eq!(payload.core_version, "1.0.0");
    }

    #[test]
    fn test_check_min_version_missing_headers_returns_mismatch() {
        let response = response_with_headers(&[]);
        let instance = instance_with_token(Some("tok"));
        let payload = check_min_version(&response, &instance).expect("mismatch expected");
        assert!(!payload.core_compatible);
        assert!(!payload.proxy_compatible);
        assert_eq!(payload.core_version, "unknown");
        assert_eq!(payload.proxy_version, "unknown");
    }

    #[test]
    fn test_non_success_status_returns_http_error_before_version_check() {
        let response = response_with_status_and_headers(StatusCode::BAD_GATEWAY, &[]);
        let instance = instance_with_token(Some("tok"));
        let err = ensure_success_status(&response, &instance).expect_err("HTTP error expected");

        assert!(
            matches!(err, Error::HttpError(message) if message.contains("status 502 Bad Gateway"))
        );
    }

    #[test]
    fn test_check_min_version_core_not_connected_suppresses() {
        // Core reports it is not connected, so an incompatible version is not flagged.
        let response = response_with_headers(&[
            (CORE_CONNECTED_HEADER, "false"),
            (CORE_VERSION_HEADER, "1.0.0"),
            (PROXY_VERSION_HEADER, "1.6.0"),
        ]);
        let instance = instance_with_token(Some("tok"));
        assert!(check_min_version(&response, &instance).is_none());
    }
}
