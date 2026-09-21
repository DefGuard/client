//! Connect-time VPN MFA over HTTP.
//!
//! Synchronous (request/response) MFA functions for TOTP and email methods,
//! plus long-running flows for OpenID (poll loop) and mobile approve (WebSocket).

use std::time::Duration;

use defguard_client_proto::defguard::client_types::{
    mfa_step_result, ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
    ClientMfaStartResponse, ClientMfaStepStartRequest, ClientMfaStepStartResponse, MfaMethod,
    MfaStartRejectionReason, MfaStepRejection, MfaStepResult,
};
use futures_util::{SinkExt, StreamExt};
use reqwest::{Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
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
    proxy::{construct_platform_header, http_client, read_error_message},
    version::{
        is_version_at_least, Version, CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER,
        CORE_VERSION_HEADER, MIN_MULTI_STEP_MFA_VERSION, PKG_VERSION, PROXY_VERSION_HEADER,
    },
};

const ATTEMPT_LIMIT_MESSAGE: &str = "Too many failed MFA attempts. Please try connecting again.";

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

    #[error("{message}")]
    AttemptLimit { message: String },

    #[error("MFA operation timed out")]
    Timeout,

    #[error("MFA operation cancelled")]
    Cancelled,

    #[error("{message}")]
    Other { message: String },
}

#[derive(Debug)]
pub struct MfaStartResult {
    pub response: ClientMfaStartResponse,
    pub multi_step_mfa_capable: bool,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum MobileMfaResponse {
    #[serde(rename = "mfa_success")]
    Legacy { preshared_key: String },
    #[serde(rename = "mfa_result")]
    Result { result: MfaStepResult },
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

    let message = read_error_message(response).await;

    match status {
        // A 403 means either a posture failure or an attempt-limit error. The message tells them apart.
        StatusCode::FORBIDDEN if message == ATTEMPT_LIMIT_MESSAGE => {
            Err(MfaError::AttemptLimit { message })
        }
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
    Ok(mfa_start_with_capability(proxy_url, request)
        .await?
        .response)
}

pub async fn mfa_start_with_capability(
    proxy_url: Url,
    request: ClientMfaStartRequest,
) -> Result<MfaStartResult, MfaError> {
    let client = http_client();

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
    let multi_step_mfa_capable = is_multi_step_mfa_capable(response.headers());
    let start_response: ClientMfaStartResponse =
        response.json().await.map_err(|e| MfaError::Other {
            message: format!("Invalid MFA start response: {e}"),
        })?;

    if !start_response.rejections.is_empty() {
        let messages: Vec<String> = start_response
            .rejections
            .iter()
            .map(rejection_message)
            .collect();
        return Err(MfaError::MfaRejected {
            message: messages.join(" "),
        });
    }

    Ok(MfaStartResult {
        response: start_response,
        multi_step_mfa_capable,
    })
}

fn is_multi_step_mfa_capable(headers: &reqwest::header::HeaderMap) -> bool {
    [CORE_VERSION_HEADER, PROXY_VERSION_HEADER]
        .into_iter()
        .all(|header| {
            headers
                .get(header)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<Version>().ok())
                .is_some_and(|version| is_version_at_least(&version, &MIN_MULTI_STEP_MFA_VERSION))
        })
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
    let client = http_client();

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
    let client = http_client();

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

/// Keepalive period for the mobile-approve WebSocket.
const MOBILE_APPROVE_PING_INTERVAL: Duration = Duration::from_secs(20);

/// Polls Edge until OIDC MFA advances, completes, times out, or is cancelled.
/// The browser must already be open. `AwaitingExternal` and legacy 428 responses keep polling.
pub async fn poll_openid_mfa(
    proxy_url: Url,
    token: String,
    step_attempt_id: Option<String>,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let client = http_client();
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
        step_attempt_id,
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
                    let response = response.json::<ClientMfaFinishResponse>().await.map_err(|e| {
                        MfaError::Other {
                            message: format!("Invalid MFA finish response: {e}"),
                        }
                    })?;

                    match response.result.as_ref() {
                        None => return Ok(response),
                        Some(result) => match result.outcome.as_ref() {
                            Some(mfa_step_result::Outcome::AwaitingExternal(_)) => {}
                            Some(
                                mfa_step_result::Outcome::Advanced(_)
                                | mfa_step_result::Outcome::Completed(_),
                            ) => return Ok(response),
                            None => {
                                return Err(MfaError::Other {
                                    message: "The server returned an unexpected verification state"
                                        .to_string(),
                                });
                            }
                        },
                    }
                } else if status != StatusCode::PRECONDITION_REQUIRED {
                    return Err(check_mfa_response(response).await.err().unwrap_or(
                        MfaError::Other {
                            message: format!("Unexpected status: {status}"),
                        },
                    ));
                }
                // 428: not complete yet - fall through to sleep.
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

/// Return the preshared key only when the MFA session completed.
///
/// Intermediate responses contain no key. For legacy responses without a step
/// result, use the top-level key.
#[must_use]
pub fn completed_preshared_key(response: &ClientMfaFinishResponse) -> Option<String> {
    let key = match response
        .result
        .as_ref()
        .and_then(|result| result.outcome.as_ref())
    {
        Some(mfa_step_result::Outcome::Completed(completed)) => &completed.preshared_key,
        Some(_) => return None,
        // Older Edge responses and mobile-approve frames have no step outcome.
        #[allow(deprecated)]
        None => &response.preshared_key,
    };
    (!key.is_empty()).then(|| key.clone())
}

/// Waits for mobile approval after the QR code is shown. Returns cancellation or
/// timeout errors when applicable.
pub async fn connect_mobile_approve(
    ws_url: &str,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let (ws_stream, _response) =
        connect_async(ws_url)
            .await
            .map_err(|err| MfaError::NetworkError {
                // Avoid logging the URL: it contains the MFA token.
                message: match &err {
                    WsError::Io(io_err) => {
                        format!("Failed to connect to Edge ({})", io_err.kind())
                    }
                    _ => "Failed to connect to Edge".to_string(),
                },
            })?;

    wait_for_mfa_outcome(ws_stream, cancel).await
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

fn mobile_approve_closed(detail: Option<String>) -> MfaError {
    let message = match detail {
        Some(detail) => {
            format!("mobile approval failed: connection closed by Edge ({detail})")
        }
        None => "mobile approval failed: connection closed by Edge".to_string(),
    };
    MfaError::MfaRejected { message }
}

fn read_error_label(err: &WsError) -> String {
    match err {
        WsError::Io(io_err) => format!("I/O error: {}", io_err.kind()),
        WsError::Protocol(protocol_err) => format!("protocol error: {protocol_err}"),
        WsError::Capacity(_) | WsError::Utf8(_) => "malformed frame from Edge".to_string(),
        _ => "stream error".to_string(),
    }
}

/// Wait on the WebSocket for an MFA outcome frame.
async fn wait_for_mfa_outcome(
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let (mut write, mut read) = ws_stream.split();
    let deadline = Instant::now() + MOBILE_APPROVE_TIMEOUT;
    // Preserve Edge's close reason for the user-facing error.
    let mut close_detail: Option<String> = None;

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
            // Keep the socket alive while the user approves; proxies may drop
            // idle connections before the MFA timeout.
            () = sleep(MOBILE_APPROVE_PING_INTERVAL) => {
                if write.send(Message::Ping(Vec::new().into())).await.is_err() {
                    return Err(mobile_approve_closed(close_detail));
                }
                continue;
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(msg)) => msg,
                    Some(Err(err)) => {
                        return Err(mobile_approve_closed(
                            close_detail.or_else(|| Some(read_error_label(&err))),
                        ));
                    }
                    None => return Err(mobile_approve_closed(close_detail)),
                }
            }
        };

        match msg {
            Message::Text(text) => match serde_json::from_str::<MobileMfaResponse>(&text) {
                Ok(MobileMfaResponse::Legacy { preshared_key }) => {
                    if preshared_key.is_empty() {
                        return Err(MfaError::MfaRejected {
                            message: "mobile approval failed: Edge returned an empty preshared key"
                                .into(),
                        });
                    }

                    #[allow(deprecated)]
                    return Ok(ClientMfaFinishResponse {
                        preshared_key,
                        token: None,
                        result: None,
                    });
                }
                // An intermediate result has no key; the caller checks its outcome.
                Ok(MobileMfaResponse::Result { result }) => {
                    #[allow(deprecated)]
                    return Ok(ClientMfaFinishResponse {
                        preshared_key: String::new(),
                        token: None,
                        result: Some(result),
                    });
                }
                // Ignore unknown frames and keep waiting; they may contain a preshared key.
                Err(err) => {
                    debug!("Ignoring unrecognized mobile MFA frame: {err}");
                }
            },
            Message::Close(frame) => {
                close_detail = Some(match frame {
                    Some(frame) if frame.reason.is_empty() => {
                        format!("code {}", u16::from(frame.code))
                    }
                    Some(frame) => format!("code {}: {}", u16::from(frame.code), frame.reason),
                    None => "no close reason".to_string(),
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests;
