//! Connect-time VPN MFA over `core::proxy` (HTTP).
//!
//! Flow: `start` → `obtain_code` → `finish` → preshared_key.
//! Supports TOTP and email methods.  OIDC and mobile-approve are Phase 6.

use defguard_client_proto::defguard::client_types::{
    ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest, ClientMfaStartResponse,
    MfaMethod,
};
use defguard_core::{
    database::models::{instance::Instance, location::Location, wireguard_keys::WireguardKeys, Id},
    proxy::post_with_headers,
};
use reqwest::{StatusCode, Url};
use serde::Deserialize;

use crate::{
    mfa_code::{obtain_code, CodeSource, MfaContext},
    state::CliError,
};

/// Run the VPN MFA handshake for a location.
///
/// * `location` — the target location.
/// * `source`  — how to obtain the code.
/// * `instance` — the instance this location belongs to (for proxy URL + pubkey).
///
/// Returns the WireGuard preshared key that must be passed to `bring_up`.
pub async fn authorize(
    location: &Location<Id>,
    source: &CodeSource,
    instance: &Instance<Id>,
) -> Result<String, CliError> {
    let wireguard_keys =
        WireguardKeys::find_by_instance_id(&*defguard_core::database::DB_POOL, instance.id)
            .await
            .map_err(|e| CliError::Other(e.to_string()))?
            .ok_or_else(|| {
                CliError::Other(format!(
                    "WireGuard keys not found for instance {}",
                    instance.name
                ))
            })?;

    let method = infer_method(location);

    // Step 1: Start the MFA session.
    let start_req = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey.clone(),
        method: method as i32,
        posture_data: None, // Phase 6: wire posture data into MFA start when both required.
    };

    let proxy_url = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?
        .join("api/v1/client-mfa/start")
        .map_err(|e| CliError::Other(format!("Failed to build MFA start URL: {e}")))?;

    tracing::debug!("Starting MFA session at {proxy_url}");
    let response = post_with_headers(proxy_url, &start_req)
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
    tracing::debug!("MFA session started, token obtained");

    // Step 2: Obtain the code.
    let ctx = MfaContext {
        instance: instance.name.clone(),
        location: location.name.clone(),
    };
    let code = obtain_code(source, &ctx)?;

    // Step 3: Finish the MFA session.
    let finish_req = ClientMfaFinishRequest {
        token,
        code: Some(code),
        auth_pub_key: None,
    };

    let finish_url = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?
        .join("api/v1/client-mfa/finish")
        .map_err(|e| CliError::Other(format!("Failed to build MFA finish URL: {e}")))?;

    tracing::debug!("Finishing MFA session at {finish_url}");
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

    tracing::info!("MFA session completed, preshared key obtained");
    Ok(finish_resp.preshared_key)
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

/// Determine the MFA method to use for a location.
fn infer_method(location: &Location<Id>) -> MfaMethod {
    match location.mfa_method {
        Some(defguard_core::database::models::location::LocationMfaMethod::Totp) => MfaMethod::Totp,
        Some(defguard_core::database::models::location::LocationMfaMethod::Email) => {
            MfaMethod::Email
        }
        Some(defguard_core::database::models::location::LocationMfaMethod::Oidc) => MfaMethod::Oidc,
        Some(defguard_core::database::models::location::LocationMfaMethod::Biometric) => {
            MfaMethod::Biometric
        }
        Some(defguard_core::database::models::location::LocationMfaMethod::MobileApprove) => {
            MfaMethod::MobileApprove
        }
        None => {
            // Default: if MFA is enabled but no method is stored, assume TOTP.
            MfaMethod::Totp
        }
    }
}
