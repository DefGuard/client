//! Connect-time VPN MFA over HTTP.
//!
//! Synchronous (request/response) MFA functions for TOTP and email methods,
//! plus long-running flows for OpenID (poll loop) and mobile approve (WebSocket).

use std::time::Duration;

use defguard_client_proto::defguard::client_types::{
    ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest, ClientMfaStartResponse,
    ClientMfaStepStartRequest, ClientMfaStepStartResponse, MfaMethod, MfaStartRejectionReason,
    MfaStepRejection,
};
use futures_util::StreamExt;
use reqwest::{Client, Response, StatusCode, Url};
use serde::Serialize;
use thiserror::Error;
use tokio::{
    net::TcpStream,
    select,
    time::{sleep, Instant},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message},
    MaybeTlsStream, WebSocketStream,
};
use tokio_util::sync::CancellationToken;

use crate::{
    proxy::construct_platform_header,
    version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION},
};

/// Error type returned by MFA operations.
///
/// Serialized as a tagged JSON union so the TypeScript frontend can
/// match on the `type` field to show context-specific messages.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MfaError {
    #[error("{message}")]
    NetworkError { message: String },

    #[error("Proxy error (HTTP {status}): {message}")]
    ProxyError { status: u16, message: String },

    #[error("MFA rejected: {message}")]
    MfaRejected { message: String },

    #[error("Posture check failed: {message}")]
    PostureRejected { message: String },

    #[error("MFA operation timed out")]
    Timeout,

    #[error("MFA operation cancelled")]
    Cancelled,

    #[error("{message}")]
    Other { message: String },
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
        // The proxy returns 403 only for a failed device posture check
        // (ApiError::PostureRejected); 401 and other 4xx are ordinary MFA
        // rejections. Keeping them distinct lets the frontend route posture
        // failures to the dedicated posture-check-failed view.
        StatusCode::FORBIDDEN => Err(MfaError::PostureRejected { message }),
        StatusCode::UNAUTHORIZED => Err(MfaError::MfaRejected { message }),
        _ if status.is_client_error() => Err(MfaError::MfaRejected { message }),
        _ => Err(MfaError::ProxyError {
            status: status.as_u16(),
            message,
        }),
    }
}

/// Start an MFA handshake for a VPN location.
///
/// POSTs a `ClientMfaStartRequest` (proto JSON) to
/// `/api/v1/client-mfa/start` and returns the session token (and
/// optionally the biometric challenge).
pub async fn mfa_start(
    proxy_url: Url,
    request: ClientMfaStartRequest,
) -> Result<ClientMfaStartResponse, MfaError> {
    let client = Client::new();

    let url = proxy_url
        .join("api/v1/client-mfa/start")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA start URL: {e}"),
        })?;

    let mut req = client.post(url).json(&request);

    for (k, v) in standard_headers() {
        req = req.header(k, v);
    }

    let response = req.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;

    #[allow(deprecated)]
    let response = match check_mfa_response(response).await {
        Ok(response) => response,
        Err(err) => return Err(rewrap_mobile_start_error(request.method, err)),
    };
    let start_response: ClientMfaStartResponse =
        response.json().await.map_err(|e| MfaError::Other {
            message: format!("Invalid MFA start response: {e}"),
        })?;

    if let Some(rejection) = start_response.rejections.first() {
        return Err(MfaError::MfaRejected {
            message: rejection_message(rejection),
        });
    }

    Ok(start_response)
}

fn rejection_message(rejection: &MfaStepRejection) -> String {
    let step = rejection.step + 1;
    match rejection.reason() {
        MfaStartRejectionReason::MfaStartRejectionMethodNotInStep => format!(
            "The method chosen for verification step {step} is not allowed. \
             The location's MFA settings have changed, so pick a method again."
        ),
        MfaStartRejectionReason::MfaStartRejectionStepEmptyAfterLicense => format!(
            "Verification step {step} has no method available on this server. \
             Contact your administrator."
        ),
        MfaStartRejectionReason::MfaStartRejectionStepUnavailable => format!(
            "The method chosen for verification step {step} cannot be used. \
             Set it up first, or pick a different one."
        ),
        MfaStartRejectionReason::MfaStartRejectionUnspecified => {
            format!("The server rejected verification step {step}.")
        }
    }
}

pub async fn mfa_step_start(
    proxy_url: Url,
    request: ClientMfaStepStartRequest,
) -> Result<ClientMfaStepStartResponse, MfaError> {
    let client = Client::new();

    let url = proxy_url
        .join("api/v1/client-mfa/step-start")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA step start URL: {e}"),
        })?;

    let mut request_builder = client.post(url).json(&request);

    for (header_name, header_value) in standard_headers() {
        request_builder = request_builder.header(header_name, header_value);
    }

    let response = request_builder
        .send()
        .await
        .map_err(|e| MfaError::NetworkError {
            message: format!("Failed to reach proxy: {e}"),
        })?;

    let response = check_mfa_response(response).await?;
    response.json().await.map_err(|e| MfaError::Other {
        message: format!("Invalid MFA step start response: {e}"),
    })
}

/// Turn the proxy's generic "selected MFA method is not available" rejection
/// into actionable guidance for mobile-approve MFA (the user has no registered
/// mobile authenticator). Restores the CLI behavior that was lost when this
/// logic moved into core; benefits the desktop client too. Non-mobile methods
/// keep the original message.
fn rewrap_mobile_start_error(method: i32, err: MfaError) -> MfaError {
    if method == MfaMethod::MobileApprove as i32 {
        if let MfaError::MfaRejected { message } = &err {
            if message.contains("selected MFA method is not available") {
                return MfaError::MfaRejected {
                    message: "No mobile authenticator is registered for your account. \
                              Register one in the Defguard mobile app, then retry."
                        .into(),
                };
            }
        }
    }
    err
}

/// Finish an MFA handshake using a one-time code (TOTP or email).
///
/// POSTs a `ClientMfaFinishRequest` to `/api/v1/client-mfa/finish`
/// and returns the preshared key.
pub async fn mfa_finish_code(
    proxy_url: Url,
    request: ClientMfaFinishRequest,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let client = Client::new();

    let url = proxy_url
        .join("api/v1/client-mfa/finish")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA finish URL: {e}"),
        })?;

    let mut req = client.post(url).json(&request);

    for (k, v) in standard_headers() {
        req = req.header(k, v);
    }

    let response = req.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;

    let response = check_mfa_response(response).await?;
    response.json().await.map_err(|e| MfaError::Other {
        message: format!("Invalid MFA finish response: {e}"),
    })
}

#[cfg(not(test))]
const OIDC_POLL_INTERVAL: Duration = Duration::from_secs(5);
#[cfg(test)]
const OIDC_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[cfg(not(test))]
const OIDC_POLL_TIMEOUT: Duration = Duration::from_mins(5);
#[cfg(test)]
const OIDC_POLL_TIMEOUT: Duration = Duration::from_millis(200);

#[cfg(not(test))]
const MOBILE_APPROVE_TIMEOUT: Duration = Duration::from_mins(2);
#[cfg(test)]
const MOBILE_APPROVE_TIMEOUT: Duration = Duration::from_secs(5);

/// Poll Defguard Edge for OpenID MFA completion.
///
/// The caller must already have opened the browser to the OIDC provider
/// URL (the token from `mfa_start` encodes the redirect).  This function
/// POSTs a `ClientMfaFinishRequest` to `/api/v1/client-mfa/finish` every
/// [`OIDC_POLL_INTERVAL`] until the server returns a 200 (success),
/// the deadline expires, or the [`CancellationToken`] is fired.
pub async fn poll_openid_mfa(
    proxy_url: Url,
    token: String,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let client = Client::new();
    let url = proxy_url
        .join("api/v1/client-mfa/finish")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA finish URL: {e}"),
        })?;

    let deadline = Instant::now() + OIDC_POLL_TIMEOUT;

    let request = ClientMfaFinishRequest {
        token,
        code: None,
        auth_pub_key: None,
        step_attempt_id: None,
        auth_data: None,
        credential_id: None,
    };

    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            return Err(MfaError::Timeout);
        }

        let mut req = client.post(url.clone()).json(&request);
        for (k, v) in standard_headers() {
            req = req.header(k, v);
        }

        select! {
            () = cancel.cancelled() => {
                return Err(MfaError::Cancelled);
            }
            result = req.send() => {
                let response = result.map_err(|err| MfaError::NetworkError {
                    message: format!("Failed to reach Edge: {err}"),
                })?;

                let status = response.status();
                if status == StatusCode::OK {
                    return response.json().await.map_err(|e| MfaError::Other {
                        message: format!("Invalid MFA finish response: {e}"),
                    });
                }
                if status != StatusCode::PRECONDITION_REQUIRED {
                    return Err(check_mfa_response(response).await.err().unwrap_or(
                        MfaError::Other {
                            message: format!("Unexpected status: {status}"),
                        },
                    ));
                }
                // 428: not complete yet — fall through to sleep.
            }
        }

        select! {
            () = cancel.cancelled() => {
                return Err(MfaError::Cancelled);
            }
            () = sleep(OIDC_POLL_INTERVAL) => {}
        }
    }
}

/// Connect to a WebSocket endpoint and wait for mobile-approve MFA
/// completion.
///
/// The caller must have already displayed the QR code to the user
/// (the token from `mfa_start` encodes the challenge).  This function
/// opens a WebSocket to `ws_url` and waits for a
/// `{"type":"mfa_success","preshared_key":"..."}` text frame.
/// Returns [`MfaError::Cancelled`] if the token fires or
/// [`MfaError::Timeout`] if the deadline expires.
pub async fn connect_mobile_approve(
    ws_url: &str,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let (ws_stream, _response) =
        connect_async(ws_url)
            .await
            .map_err(|err| MfaError::NetworkError {
                // Never interpolate the raw error: `ws_url` carries the MFA
                // token as a query parameter and can appear in the error's
                // Display, which is surfaced to the frontend and logs.
                message: match &err {
                    WsError::Io(io_err) => {
                        format!("Failed to connect to Edge ({})", io_err.kind())
                    }
                    _ => "Failed to connect to Edge".to_string(),
                },
            })?;

    wait_for_mfa_success(ws_stream, cancel).await
}

/// Derive the WebSocket URL from the proxy's base URL and MFA token.
pub fn derive_ws_url(proxy_base: &Url, token: &str) -> Result<String, MfaError> {
    let mut ws_url = proxy_base
        .join("api/v1/client-mfa/remote")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build WebSocket URL: {e}"),
        })?;

    let ws_scheme = match proxy_base.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            return Err(MfaError::Other {
                message: format!("Invalid Edge URL scheme '{other}'; expected http or https"),
            });
        }
    };

    ws_url.set_scheme(ws_scheme).map_err(|()| MfaError::Other {
        message: "Failed to set WebSocket URL scheme".into(),
    })?;
    ws_url.query_pairs_mut().append_pair("token", token);

    Ok(ws_url.to_string())
}

/// Wait on the WebSocket for an `mfa_success` frame.
async fn wait_for_mfa_success(
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let (_write, mut read) = ws_stream.split();
    let deadline = Instant::now() + MOBILE_APPROVE_TIMEOUT;

    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            return Err(MfaError::Timeout);
        }

        let msg = select! {
            () = sleep(remaining) => {
                return Err(MfaError::Timeout);
            }
            () = cancel.cancelled() => {
                return Err(MfaError::Cancelled);
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(msg)) => msg,
                    Some(Err(_)) | None => {
                        return Err(MfaError::MfaRejected {
                            message: "mobile approval failed: connection closed by Edge"
                                .into(),
                        });
                    }
                }
            }
        };

        if let Message::Text(text) = msg {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                if parsed.get("type").and_then(|v| v.as_str()) == Some("mfa_success") {
                    if let Some(key) = parsed["preshared_key"].as_str() {
                        #[allow(deprecated)]
                        return Ok(ClientMfaFinishResponse {
                            preshared_key: key.to_string(),
                            token: None,
                            result: None,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests;
