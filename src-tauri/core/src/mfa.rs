//! Connect-time VPN MFA over HTTP.
//!
//! Synchronous (request/response) MFA functions for TOTP and email methods.
//! Long-running flows (OpenID poll, mobile approve WebSocket) are in
//! separate functions and are covered in later implementation steps.

use reqwest::{Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    proxy::construct_platform_header,
    version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION},
};

/// Error type returned by MFA operations.
///
/// Serialized as a tagged JSON union so the TypeScript frontend can
/// match on the `type` field to show context-specific messages.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MfaError {
    #[error("{message}")]
    NetworkError { message: String },

    #[error("Proxy error (HTTP {status}): {message}")]
    ProxyError { status: u16, message: String },

    #[error("MFA rejected: {message}")]
    MfaRejected { message: String },

    #[error("{message}")]
    Other { message: String },
}

/// WireGuard preshared key returned after a successful MFA handshake.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresharedKey(pub String);

/// Information returned by the MFA start endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MfaInfo {
    pub token: String,
    #[serde(default)]
    pub challenge: Option<String>,
}

/// Deserialization-only payload for the MFA finish response.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MfaFinishBody {
    preshared_key: String,
}

fn build_client() -> Client {
    Client::new()
}

fn standard_headers() -> Vec<(&'static str, String)> {
    vec![
        (CLIENT_VERSION_HEADER, PKG_VERSION.to_string()),
        (CLIENT_PLATFORM_HEADER, construct_platform_header()),
    ]
}

/// Check an MFA response status and map it to `MfaError`.
async fn check_mfa_response(response: Response) -> Result<Response, MfaError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let message = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or_else(|| format!("HTTP {status}"));

    match status {
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => Err(MfaError::MfaRejected { message }),
        _ if status.is_client_error() => Err(MfaError::MfaRejected { message }),
        _ => Err(MfaError::ProxyError {
            status: status.as_u16(),
            message,
        }),
    }
}

/// Start an MFA handshake for a VPN location.
///
/// POSTs `{ "locationId": ..., "pubkey": "...", "method": "..." }` to
/// `/api/v1/client-mfa/start` and returns the session token (and
/// optionally the biometric challenge).
pub async fn mfa_start(
    proxy_url: Url,
    location_id: i64,
    pubkey: String,
    method: String,
) -> Result<MfaInfo, MfaError> {
    let client = build_client();

    let url = proxy_url
        .join("api/v1/client-mfa/start")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA start URL: {e}"),
        })?;

    let mut req = client.post(url).json(&serde_json::json!({
        "locationId": location_id,
        "pubkey": pubkey,
        "method": method,
    }));

    for (k, v) in standard_headers() {
        req = req.header(k, v);
    }

    let response = req.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;

    let response = check_mfa_response(response).await?;
    response.json().await.map_err(|e| MfaError::Other {
        message: format!("Invalid MFA start response: {e}"),
    })
}

/// Finish an MFA handshake using a one-time code (TOTP or email).
///
/// POSTs `{ "token": "...", "code": "..." }` to
/// `/api/v1/client-mfa/finish` and returns the WireGuard preshared key.
pub async fn mfa_finish_code(
    proxy_url: Url,
    token: String,
    code: String,
) -> Result<PresharedKey, MfaError> {
    let client = build_client();

    let url = proxy_url
        .join("api/v1/client-mfa/finish")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA finish URL: {e}"),
        })?;

    let mut req = client.post(url).json(&serde_json::json!({
        "token": token,
        "code": code,
    }));

    for (k, v) in standard_headers() {
        req = req.header(k, v);
    }

    let response = req.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;

    let response = check_mfa_response(response).await?;
    let body: MfaFinishBody = response.json().await.map_err(|e| MfaError::Other {
        message: format!("Invalid MFA finish response: {e}"),
    })?;

    Ok(PresharedKey(body.preshared_key))
}
