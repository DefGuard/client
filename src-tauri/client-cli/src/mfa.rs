//! Connect-time VPN MFA over `core::proxy` (HTTP).
//!
//! Flow: `start` → `obtain_code` → `finish` → preshared_key.
//! Supports TOTP and email methods.  OIDC and mobile-approve are WIP.

use std::time::Duration;

use defguard_client_proto::defguard::{
    client_types::{
        ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
        ClientMfaStartResponse, MfaMethod,
    },
    enterprise::posture::v2::DevicePostureData,
};
use defguard_core::{
    database::{
        models::{
            instance::Instance,
            location::{infer_mfa_method, Location, LocationMfaMethod},
            wireguard_keys::WireguardKeys,
            Id,
        },
        DbPool,
    },
    proxy::post_with_headers,
};
use reqwest::{StatusCode, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use tokio::{
    select,
    signal::ctrl_c,
    time::{sleep, Instant},
};
use tracing::{debug, info, warn};

use crate::{
    mfa_code::{obtain_code, CodeSource, MfaContext},
    state::CliError,
};

/// Resolve the effective MFA method for a location.
///
/// When `method_override` is `Some`, parses it into [`MfaMethod`]; otherwise
/// delegates to [`infer_method`] which respects the location's
/// [`LocationMfaMode`].
pub(crate) fn resolve_method(
    location: &Location<Id>,
    method_override: Option<&str>,
) -> Result<MfaMethod, CliError> {
    if let Some(raw) = method_override {
        parse_method(raw)
    } else {
        Ok(infer_method(location))
    }
}

/// Run the VPN MFA handshake for a location.
///
/// * `location` - the target location.
/// * `source`  - how to obtain the code.
/// * `instance` - the instance this location belongs to (for proxy URL + pubkey).
/// * `method_override` - optional `--mfa-method` flag; if set, uses this instead of the
///   location's stored preference.
/// * `posture_data` - device posture data; must be provided when the location also
///   requires posture checks.
///
/// Returns the WireGuard preshared key (as a [`SecretString`]) that must
/// be passed to `bring_up`.
pub async fn authorize(
    location: &Location<Id>,
    source: &CodeSource,
    instance: &Instance<Id>,
    method_override: Option<&str>,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
) -> Result<SecretString, CliError> {
    let method = resolve_method(location, method_override)?;

    // Reject methods not yet supported by the CLI before doing any I/O.
    match method {
        MfaMethod::Biometric | MfaMethod::MobileApprove => {
            return Err(CliError::MfaFailed(format!(
                "MFA method {:?} is not yet supported by the CLI. Use the desktop client.",
                method
            )));
        }
        _ => {}
    }

    let wireguard_keys = WireguardKeys::find_by_instance_id(pool, instance.id)
        .await
        .map_err(|e| CliError::Other(e.to_string()))?
        .ok_or_else(|| {
            CliError::Other(format!(
                "WireGuard keys not found for instance {}",
                instance.name
            ))
        })?;

    // Parse the proxy base URL once and reuse it for both MFA requests.
    let proxy_base = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?;

    // The one-time MFA code and the returned preshared key are sensitive; warn (but do
    // not block) if the proxy is not using HTTPS, since they would travel in cleartext.
    check_proxy_scheme(&proxy_base, &instance.proxy_url);

    // Step 1: Start the MFA session.
    let start_req = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey.clone(),
        method: method as i32,
        posture_data,
    };

    let start_url = proxy_base
        .join("api/v1/client-mfa/start")
        .map_err(|e| CliError::Other(format!("Failed to build MFA start URL: {e}")))?;

    debug!("Starting MFA session at {start_url}");
    let response = post_with_headers(start_url, &start_req)
        .await
        .map_err(|e| CliError::Other(format!("Failed to reach proxy: {e}")))?;

    if !response.status().is_success() {
        return Err(handle_mfa_error(response).await);
    }

    let start_resp: ClientMfaStartResponse = response
        .json()
        .await
        .map_err(|e| CliError::Other(format!("Invalid MFA start response: {e}")))?;

    let token = start_resp.token.clone();
    debug!("MFA session started, token obtained");

    // Step 2: Obtain the code.
    let ctx = MfaContext {
        instance: instance.name.clone(),
        location: location.name.clone(),
    };
    let code = obtain_code(source, &ctx)?;

    // Step 3: Finish the MFA session.
    let finish_req = ClientMfaFinishRequest {
        token,
        code: Some(code.expose_secret().to_string()),
        auth_pub_key: None,
    };

    let finish_url = proxy_base
        .join("api/v1/client-mfa/finish")
        .map_err(|e| CliError::Other(format!("Failed to build MFA finish URL: {e}")))?;

    debug!("Finishing MFA session at {finish_url}");
    let response = post_with_headers(finish_url, &finish_req)
        .await
        .map_err(|e| CliError::Other(format!("Failed to reach proxy: {e}")))?;

    if !response.status().is_success() {
        return Err(handle_mfa_error(response).await);
    }

    let finish_resp: ClientMfaFinishResponse = response
        .json()
        .await
        .map_err(|e| CliError::Other(format!("Invalid MFA finish response: {e}")))?;

    info!("MFA session completed, preshared key obtained");
    Ok(SecretString::from(finish_resp.preshared_key))
}

/// Map a non-2xx MFA response to a `CliError`.
async fn handle_mfa_error(response: reqwest::Response) -> CliError {
    let status = response.status();
    #[derive(Deserialize)]
    struct ErrorBody {
        error: Option<String>,
    }

    let message = response
        .json::<ErrorBody>()
        .await
        .ok()
        .and_then(|b| b.error)
        .unwrap_or_else(|| format!("HTTP {status}"));

    match status {
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
            CliError::MfaFailed(format!("MFA rejected: {message}"))
        }
        _ if status.is_client_error() => CliError::MfaFailed(format!("MFA error: {message}")),
        _ => CliError::Other(format!("Proxy error (HTTP {status}): {message}")),
    }
}

/// Parse a `--mfa-method` flag string into the proto [`MfaMethod`] enum.
fn parse_method(raw: &str) -> Result<MfaMethod, CliError> {
    match raw.to_lowercase().as_str() {
        "totp" => Ok(MfaMethod::Totp),
        "email" => Ok(MfaMethod::Email),
        "oidc" => Ok(MfaMethod::Oidc),
        "biometric" => Ok(MfaMethod::Biometric),
        "mobile" | "mobile_approve" => Ok(MfaMethod::MobileApprove),
        _ => Err(CliError::Usage(format!(
            "Invalid --mfa-method '{raw}'. Valid: totp, email, oidc, biometric, mobile."
        ))),
    }
}

/// Determine the MFA method to use for a location.
///
/// Delegates to the core's [`infer_mfa_method`] so that
/// [`LocationMfaMode`] is respected - an External-mode location always
/// uses OIDC, while an Internal-mode location respects the stored
/// preference (defaulting to TOTP when unset).
fn infer_method(location: &Location<Id>) -> MfaMethod {
    let method = infer_mfa_method(location.location_mfa_mode, location.mfa_method);
    match method {
        Some(LocationMfaMethod::Totp) => MfaMethod::Totp,
        Some(LocationMfaMethod::Email) => MfaMethod::Email,
        Some(LocationMfaMethod::Oidc) => MfaMethod::Oidc,
        Some(LocationMfaMethod::Biometric) => MfaMethod::Biometric,
        Some(LocationMfaMethod::MobileApprove) => MfaMethod::MobileApprove,
        None => {
            // `infer_mfa_method` only returns None for Disabled mode, but
            // this function is only called when `mfa_enabled()` is true.
            // Default to TOTP as a safe fallback.
            MfaMethod::Totp
        }
    }
}

/// Warn if the proxy is not using HTTPS.
///
/// The one-time MFA code and the returned preshared key are sensitive and would
/// travel in cleartext over plain HTTP.
fn check_proxy_scheme(proxy_base: &Url, proxy_url: &str) {
    if proxy_base.scheme() != "https" {
        warn!(
            "Proxy URL '{}' is not HTTPS; secrets will be sent in cleartext.",
            proxy_url
        );
    }
}

/// Open a URL in the system browser.
///
/// Production: calls [`webbrowser::open`]; prints a hint to stderr on failure.
/// Tests: no-op (never spawn a browser).
#[cfg(not(test))]
fn open_url(url: &str) {
    if webbrowser::open(url).is_err() {
        eprintln!("Could not open browser. Open the URL above manually.");
    }
}

#[cfg(test)]
fn open_url(_url: &str) {
    // no-op: tests must not spawn a browser
}

/// Fixed interval between OIDC MFA finish polls (shortened for tests).
#[cfg(not(test))]
const OIDC_POLL_INTERVAL: Duration = Duration::from_secs(5);
#[cfg(test)]
const OIDC_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Total time the CLI will wait for the user to complete OIDC authentication
/// before giving up (shortened for tests).
#[cfg(not(test))]
const OIDC_POLL_TIMEOUT: Duration = Duration::from_secs(300);
#[cfg(test)]
const OIDC_POLL_TIMEOUT: Duration = Duration::from_millis(200);

/// Run the OIDC MFA handshake for an external-IdP location.
///
/// Opens the system browser to `{proxy_url}openid/mfa?token=...` and polls
/// the proxy until the user completes authentication with the external
/// identity provider. Returns the WireGuard preshared key.
///
/// When `json_mode` is true, progress messages on stderr are suppressed so
/// that `--json` output consumers only see the final result/error.
pub(crate) async fn authorize_oidc(
    location: &Location<Id>,
    instance: &Instance<Id>,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
    json_mode: bool,
) -> Result<SecretString, CliError> {
    let wireguard_keys = WireguardKeys::find_by_instance_id(pool, instance.id)
        .await
        .map_err(|e| CliError::Other(e.to_string()))?
        .ok_or_else(|| {
            CliError::Other(format!(
                "WireGuard keys not found for instance {}",
                instance.name
            ))
        })?;

    let proxy_base = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?;
    check_proxy_scheme(&proxy_base, &instance.proxy_url);

    // Step 1: Start the OIDC MFA session.
    let start_req = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey.clone(),
        method: MfaMethod::Oidc as i32,
        posture_data,
    };

    let start_url = proxy_base
        .join("api/v1/client-mfa/start")
        .map_err(|e| CliError::Other(format!("Failed to build MFA start URL: {e}")))?;

    debug!("Starting OIDC MFA session at {start_url}");
    let response = post_with_headers(start_url, &start_req)
        .await
        .map_err(|e| CliError::Other(format!("Failed to reach proxy: {e}")))?;

    if !response.status().is_success() {
        return Err(handle_mfa_error(response).await);
    }

    let start_resp: ClientMfaStartResponse = response
        .json()
        .await
        .map_err(|e| CliError::Other(format!("Invalid MFA start response: {e}")))?;

    // Step 2: Open the browser for the user to authenticate.
    // Never log the token-bearing URL via tracing.
    let browser_url = proxy_base
        .join(&format!("openid/mfa?token={}", start_resp.token))
        .map_err(|e| CliError::Other(format!("Failed to build OIDC MFA URL: {e}")))?;

    if !json_mode {
        eprintln!("Open this URL to authenticate:");
        eprintln!("  {browser_url}");
        eprintln!("Waiting for authentication… (Ctrl-C to cancel)");
    }
    open_url(browser_url.as_ref());

    // Step 3: Poll until the user completes authentication or the deadline expires.
    poll_finish(&proxy_base, &start_resp.token, json_mode).await
}

/// Poll [`client-mfa/finish`] until the OIDC session is completed or the
/// deadline expires.
///
/// Returns the preshared key on success.
///
/// [`client-mfa/finish`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/428
async fn poll_finish(
    proxy_base: &Url,
    token: &str,
    json_mode: bool,
) -> Result<SecretString, CliError> {
    let finish_url = proxy_base
        .join("api/v1/client-mfa/finish")
        .map_err(|e| CliError::Other(format!("Failed to build MFA finish URL: {e}")))?;

    let finish_req = ClientMfaFinishRequest {
        token: token.to_string(),
        code: None,
        auth_pub_key: None,
    };

    let deadline = Instant::now() + OIDC_POLL_TIMEOUT;

    loop {
        // Check if we've exceeded the overall deadline.
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            if !json_mode {
                eprintln!("MFA login timed out.");
            }
            return Err(CliError::MfaFailed("MFA login timed out".into()));
        }

        // Wait the poll interval, yielding to Ctrl-C.
        let sleep = sleep(OIDC_POLL_INTERVAL);
        select! {
            _ = ctrl_c() => {
                if !json_mode {
                    eprintln!("MFA login cancelled.");
                }
                return Err(CliError::Cancelled("MFA login cancelled.".into()));
            }
            () = sleep => {}
        }

        // Poll the proxy, yielding to Ctrl-C during the request.
        let (status, body) = select! {
            _ = ctrl_c() => {
                if !json_mode {
                    eprintln!("MFA login cancelled.");
                }
                return Err(CliError::Cancelled("MFA login cancelled.".into()));
            }
            result = post_with_headers(finish_url.clone(), &finish_req) => {
                let response = result
                    .map_err(|e| CliError::Other(format!("Failed to reach proxy: {e}")))?;
                let status = response.status();
                if status == StatusCode::OK {
                    let finish_resp: ClientMfaFinishResponse = response
                        .json()
                        .await
                        .map_err(|e| CliError::Other(format!("Invalid MFA finish response: {e}")))?;
                    info!("OIDC MFA session completed, preshared key obtained");
                    return Ok(SecretString::from(finish_resp.preshared_key));
                }
                (status, response)
            }
        };

        // Non-OK, non-428: report the error.
        if status != StatusCode::PRECONDITION_REQUIRED {
            return Err(handle_mfa_error(body).await);
        }

        // 428: OIDC not complete yet, loop around.
    }
}
