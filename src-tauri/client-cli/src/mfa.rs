//! Connect-time VPN MFA over `core::proxy` (HTTP).
//!
//! Flow: `start` → `obtain_code` → `finish` → preshared_key.
//! Supports TOTP, email, and OIDC methods.  Mobile-approve is
//! not yet supported by the CLI.

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
            location::{infer_mfa_method, Location, LocationMfaMethod, LocationMfaMode},
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

/// Resolve the effective MFA method for a location and validate CLI flags
/// that are incompatible with certain methods.
///
/// When `method_override` is `Some`, parses it into [`MfaMethod`]; otherwise
/// delegates to [`infer_method`] which respects the location's
/// [`LocationMfaMode`].
///
/// Rejects:
/// * `--mfa-method oidc` on Internal-mode locations
/// * `--code` / `--code-command` when the resolved method is OIDC
pub(crate) fn resolve_method(
    location: &Location<Id>,
    method_override: Option<&str>,
    code: Option<&str>,
    code_command: Option<&str>,
) -> Result<MfaMethod, CliError> {
    let method = if let Some(raw) = method_override {
        let method = parse_method(raw)?;
        // OIDC override on an Internal-mode location will be rejected by the
        // server.  Fail early to give the user a clear error before I/O.
        if method == MfaMethod::Oidc && location.location_mfa_mode == LocationMfaMode::Internal {
            return Err(CliError::InvalidInput(
                "--mfa-method oidc is only valid for locations that use external (OIDC) MFA."
                    .into(),
            ));
        }
        method
    } else {
        infer_method(location)
    };

    // OIDC MFA does not use codes; --code / --code-command are incompatible.
    if method == MfaMethod::Oidc && (code.is_some() || code_command.is_some()) {
        return Err(CliError::InvalidInput(format!(
            "location '{}' cannot use --code / --code-command with external (OIDC) MFA",
            location.name
        )));
    }

    Ok(method)
}

/// Run the VPN MFA handshake for a location.
///
/// * `location` - the target location.
/// * `source`  - how to obtain the code.
/// * `instance` - the instance this location belongs to (for proxy URL + pubkey).
/// * `method` - the resolved [`MfaMethod`] (use [`resolve_method`] to obtain it).
///   **Must not be [`MfaMethod::Oidc`]**; OIDC uses [`authorize_oidc`] instead.
/// * `posture_data` - device posture data; must be provided when the location also
///   requires posture checks.
///
/// Returns the WireGuard preshared key (as a [`SecretString`]) that must
/// be passed to `bring_up`.
pub async fn authorize(
    location: &Location<Id>,
    source: &CodeSource,
    instance: &Instance<Id>,
    method: MfaMethod,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
) -> Result<SecretString, CliError> {
    // Reject methods not yet supported by the CLI before doing any I/O.
    // OIDC is not rejected as "not yet supported" because it IS supported -
    // but it has its own dedicated code path (authorize_oidc).  If callers
    // accidentally invoke authorize() with OIDC, this catch-all is a
    // defense-in-depth barrier that emits a clear error.
    match method {
        MfaMethod::Biometric | MfaMethod::MobileApprove => {
            return Err(CliError::MfaFailed(format!(
                "MFA method {method:?} is not yet supported by the CLI. Use the desktop client."
            )));
        }
        MfaMethod::Oidc => {
            return Err(CliError::Other(
                "Internal error: OIDC MFA must use authorize_oidc, not authorize".into(),
            ));
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

#[derive(Deserialize)]
struct ErrorBody {
    error: Option<String>,
}

/// Map a non-2xx MFA response to a `CliError`.
async fn handle_mfa_error(response: reqwest::Response) -> CliError {
    let status = response.status();

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
        warn!("Proxy URL '{proxy_url}' is not HTTPS; secrets will be sent in cleartext.");
    }
}

/// Open a URL in the system browser.
///
/// Production: calls [`webbrowser::open`]; prints a hint to stderr on failure.
/// When `json_mode` is true, the fallback message includes the URL itself since
/// it wasn't already printed above.
/// Tests: no-op (never spawn a browser).
#[cfg(not(test))]
fn open_url(url: &str, json_mode: bool) {
    if webbrowser::open(url).is_err() {
        if json_mode {
            eprintln!("Could not open browser. Open this URL manually: {url}");
        } else {
            eprintln!("Could not open browser. Open the URL above manually.");
        }
    }
}

#[cfg(test)]
fn open_url(_url: &str, _json_mode: bool) {
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

/// Run the OIDC MFA flow for an external-IdP location.
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
    let mut browser_url = proxy_base
        .join("openid/mfa")
        .map_err(|e| CliError::Other(format!("Failed to build OIDC MFA URL: {e}")))?;
    browser_url
        .query_pairs_mut()
        .append_pair("token", &start_resp.token);

    if !json_mode {
        eprintln!("Open this URL to authenticate:");
        eprintln!("  {browser_url}");
        eprintln!("Waiting for authentication... (Ctrl-C to cancel)");
    }
    open_url(browser_url.as_ref(), json_mode);

    // Step 3: Poll until the user completes authentication or the deadline expires.
    poll_finish(&proxy_base, &start_resp.token, json_mode).await
}

/// Poll [`client-mfa/finish`] until the OIDC session is completed or the
/// deadline expires.
///
/// Returns the preshared key on success.
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

        // Poll the proxy (first iteration is immediate, subsequent ones wait
        // the poll interval at the end of each loop body).
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

        // 428: OIDC not complete yet.  Wait the poll interval, yielding to
        // Ctrl-C, then loop around for another check.
        select! {
            _ = ctrl_c() => {
                if !json_mode {
                    eprintln!("MFA login cancelled.");
                }
                return Err(CliError::Cancelled("MFA login cancelled.".into()));
            }
            () = sleep(OIDC_POLL_INTERVAL) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use defguard_core::database::models::location::ServiceLocationMode;

    fn location(name: &str, mode: LocationMfaMode) -> Location<Id> {
        Location {
            id: 1,
            instance_id: 1,
            network_id: 1,
            name: name.into(),
            address: "10.0.0.0/24".into(),
            pubkey: "pk".into(),
            endpoint: "1.2.3.4:51820".into(),
            allowed_ips: "0.0.0.0/0".into(),
            dns: None,
            route_all_traffic: false,
            keepalive_interval: 25,
            location_mfa_mode: mode,
            service_location_mode: ServiceLocationMode::Disabled,
            mfa_method: None,
            posture_check_required: false,
        }
    }

    #[test]
    fn test_oidc_location_resolves_to_oidc() {
        let l = location("office", LocationMfaMode::External);
        let method = resolve_method(&l, None, None, None).unwrap();
        assert_eq!(method, MfaMethod::Oidc);
    }

    #[test]
    fn test_internal_location_resolves_to_totp() {
        let l = location("office", LocationMfaMode::Internal);
        let method = resolve_method(&l, None, None, None).unwrap();
        assert_eq!(method, MfaMethod::Totp);
    }

    #[test]
    fn test_code_with_oidc_rejected() {
        let l = location("office", LocationMfaMode::External);
        let err = resolve_method(&l, None, Some("123456"), None).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_code_command_with_oidc_rejected() {
        let l = location("office", LocationMfaMode::External);
        let err = resolve_method(&l, None, None, Some("pass otp")).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code-command"));
    }

    #[test]
    fn test_code_with_totp_passes() {
        let l = location("office", LocationMfaMode::Internal);
        let method = resolve_method(&l, None, Some("123456"), None).unwrap();
        assert_eq!(method, MfaMethod::Totp);
    }

    #[test]
    fn test_no_code_with_oidc_passes() {
        let l = location("office", LocationMfaMode::External);
        let method = resolve_method(&l, None, None, None).unwrap();
        assert_eq!(method, MfaMethod::Oidc);
    }

    #[test]
    fn test_mfa_method_oidc_on_internal_rejected() {
        let l = location("office", LocationMfaMode::Internal);
        let err = resolve_method(&l, Some("oidc"), None, None).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("oidc"));
    }
}
