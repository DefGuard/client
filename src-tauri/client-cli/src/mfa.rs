//! Connect-time VPN MFA over `core::proxy` (HTTP).
//!
//! Flow: `start` → `obtain_code` → `finish` → preshared_key.
//! Supports TOTP and email methods.  OIDC and mobile-approve are WIP.

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
use tracing::{debug, info, warn};

use crate::{
    mfa_code::{obtain_code, CodeSource, MfaContext},
    state::CliError,
};

/// Run the VPN MFA handshake for a location.
///
/// * `location` — the target location.
/// * `source`  — how to obtain the code.
/// * `instance` — the instance this location belongs to (for proxy URL + pubkey).
/// * `method_override` — optional `--mfa-method` flag; if set, uses this instead of the
///   location's stored preference.
/// * `posture_data` — device posture data; must be provided when the location also
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
    let wireguard_keys = WireguardKeys::find_by_instance_id(pool, instance.id)
        .await
        .map_err(|e| CliError::Other(e.to_string()))?
        .ok_or_else(|| {
            CliError::Other(format!(
                "WireGuard keys not found for instance {}",
                instance.name
            ))
        })?;

    let method = if let Some(raw) = method_override {
        parse_method(raw)?
    } else {
        infer_method(location)
    };

    // Reject methods not yet supported by the CLI.
    match method {
        MfaMethod::Oidc | MfaMethod::Biometric | MfaMethod::MobileApprove => {
            return Err(CliError::MfaFailed(format!(
                "MFA method {:?} is not yet supported by the CLI. Use the desktop client.",
                method
            )));
        }
        _ => {}
    }

    // Parse the proxy base URL once and reuse it for both MFA requests.
    let proxy_base = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?;

    // The one-time MFA code and the returned preshared key are sensitive; warn (but do
    // not block) if the proxy is not using HTTPS, since they would travel in cleartext.
    if proxy_base.scheme() != "https" {
        warn!(
            "Proxy URL '{}' is not HTTPS; the MFA code and preshared key will be sent in cleartext.",
            instance.proxy_url
        );
    }

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
