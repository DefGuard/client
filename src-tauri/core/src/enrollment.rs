//! Enrollment flow for adding a new Defguard instance.
//!
//! Handles the client-side enrollment protocol against the Edge proxy:
//! starting enrollment, creating a device, activating the user, registering
//! MFA, and finishing the enrollment.

use reqwest::{Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    proxy::construct_platform_header,
    version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION},
};

/// Error type returned by enrollment operations.
///
/// Serialized as a tagged JSON union so the TypeScript frontend can
/// match on the `type` field to show context-specific messages.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EnrollmentError {
    #[error("Enrollment token has expired or is invalid")]
    TokenExpired,

    #[error("Session cookie not found in enrollment response")]
    MissingCookie,

    #[error("{message}")]
    NetworkError { message: String },

    #[error("Proxy error (HTTP {status}): {message}")]
    ProxyError { status: u16, message: String },

    #[error("{message}")]
    Other { message: String },
}

/// Holds the enrollment session state.
#[derive(Clone)]
pub struct EnrollmentSession {
    pub cookie: String,
    pub proxy_url: Url,
    pub client: Client,
}

/// User information returned by the enrollment process.
///
/// Mirrors `InitialUserInfo` from `client_types.proto`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub first_name: String,
    pub last_name: String,
    pub login: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub is_active: bool,
    pub device_names: Vec<String>,
    pub enrolled: bool,
    pub is_admin: bool,
    pub password_management_disabled: bool,
}

/// Deserialization-only wrapper for the `EnrollmentStartResponse` JSON from
/// the proxy, so we only pull out the fields we need without a full proto
/// dependency.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProxyEnrollmentStartResponse {
    user: UserInfo,
}

/// Extract the `defguard_proxy` session cookie from a response's `Set-Cookie`
/// headers.
fn extract_defguard_cookie(response: &Response) -> Result<String, EnrollmentError> {
    for value in response.headers().get_all(reqwest::header::SET_COOKIE) {
        let raw = value.to_str().unwrap_or_default();
        if raw.starts_with("defguard_proxy=") {
            // Take everything up to the first `;`.
            return Ok(raw.split(';').next().unwrap_or(raw).to_string());
        }
    }
    Err(EnrollmentError::MissingCookie)
}

/// Read an error body from a non-2xx response.
async fn read_error_body(response: Response) -> String {
    let status = response.status();
    response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or_else(|| format!("HTTP {status}"))
}

/// Start the enrollment process.
///
/// POSTs `{ "token": "<token>" }` to `/api/v1/enrollment/start`, extracts
/// the `defguard_proxy` session cookie from the `Set-Cookie` response
/// header, and returns the session together with the enrolling user's
/// information.
pub async fn enrollment_start(
    proxy_url: Url,
    token: String,
) -> Result<(EnrollmentSession, UserInfo), EnrollmentError> {
    let client = Client::new();

    let url = proxy_url
        .join("api/v1/enrollment/start")
        .map_err(|e| EnrollmentError::Other {
            message: format!("Failed to build enrollment start URL: {e}"),
        })?;

    let response = client
        .post(url)
        .json(&serde_json::json!({ "token": token }))
        .header(CLIENT_VERSION_HEADER, PKG_VERSION)
        .header(CLIENT_PLATFORM_HEADER, construct_platform_header())
        .send()
        .await
        .map_err(|e| EnrollmentError::NetworkError {
            message: format!("Failed to reach proxy: {e}"),
        })?;

    let status = response.status();

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(EnrollmentError::TokenExpired);
    }

    if !status.is_success() {
        let message = read_error_body(response).await;
        return Err(EnrollmentError::ProxyError {
            status: status.as_u16(),
            message,
        });
    }

    let cookie = extract_defguard_cookie(&response)?;

    let body: ProxyEnrollmentStartResponse =
        response.json().await.map_err(|e| EnrollmentError::Other {
            message: format!("Failed to parse enrollment response: {e}"),
        })?;

    let session = EnrollmentSession {
        cookie,
        proxy_url,
        client,
    };

    Ok((session, body.user))
}
