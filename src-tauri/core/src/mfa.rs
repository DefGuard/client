//! Connect-time VPN MFA over HTTP.
//!
//! Synchronous (request/response) MFA functions for TOTP and email methods,
//! plus long-running flows for OpenID (poll loop) and mobile approve (WebSocket).

use std::time::Duration;

use defguard_client_proto::defguard::{
    client_types::{
        mfa_flow_start_response, mfa_flow_step_finish_request, mfa_step_result, mfa_step_started,
        ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
        ClientMfaStartResponse, MfaAdvanced, MfaCodeCredential, MfaCompleted, MfaFido2Assertion,
        MfaFlowStartRequest, MfaFlowStartResponse, MfaFlowStepFinishRequest,
        MfaFlowStepFinishResponse, MfaFlowStepStartRequest, MfaFlowStepStartResponse, MfaMethod,
        MfaStartRejectionReason, MfaStepRejection, MfaStepResult, MfaStepStarted,
    },
    enterprise::posture::v2::DevicePostureData,
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
    database::models::Id,
    mfa_contract::MfaContract,
    proxy::{construct_platform_header, http_client, read_error_message},
    version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION},
};

const ATTEMPT_LIMIT_MESSAGE: &str = "Too many failed MFA attempts. Please try connecting again.";

/// Registration guidance for an unavailable mobile-approve step.
///
/// Both MFA start paths use this message when they report that case.
const MOBILE_NOT_REGISTERED_MESSAGE: &str =
    "No mobile authenticator is registered for your account. \
     Register one in the Defguard mobile app, then retry.";

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

pub struct MfaStartRequest {
    pub location_id: i64,
    pub pubkey: String,
    pub posture_data: Option<DevicePostureData>,
    pub selected_methods: Vec<MfaMethod>,
}

#[derive(Debug)]
pub struct MfaStepStartResponse {
    pub step_attempt_id: Option<String>,
    pub challenge: Option<String>,
    pub credential_ids: Vec<String>,
}

#[derive(Debug)]
pub struct MfaStartResponse {
    pub token: String,
    pub first_step: MfaStepStartResponse,
}

pub enum MfaSubmission {
    Code(String),
    Fido2(MfaFido2Assertion),
}

pub struct MfaFinishRequest {
    pub token: String,
    pub step_attempt_id: Option<String>,
    pub submission: Option<MfaSubmission>,
}

#[derive(Debug)]
pub struct MfaFinishResponse {
    pub preshared_key: String,
    pub result: Option<MfaStepResult>,
}

#[derive(Clone)]
pub struct MfaAuthSession {
    pub contract: MfaContract,
    pub step_attempt_id: Option<String>,
    pub instance_id: Id,
    pub location_id: Id,
    pub proxy_url: Url,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum LegacyMobileMfaResponse {
    #[serde(rename = "mfa_success")]
    Success { preshared_key: String },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum MultiStepMobileMfaResponse {
    #[serde(rename = "mfa_result")]
    Result { result: MultiStepMobileMfaResult },
}

#[derive(Deserialize)]
struct MultiStepMobileMfaResult {
    outcome: MultiStepMobileMfaOutcome,
}

#[derive(Deserialize)]
enum MultiStepMobileMfaOutcome {
    Advanced(MfaAdvanced),
    Completed(MfaCompleted),
}

fn decode_multi_step_mobile_mfa_frame(text: &str) -> Result<MfaFinishResponse, MfaError> {
    let response: MultiStepMobileMfaResponse =
        serde_json::from_str(text).map_err(|_| MfaError::Other {
            message: "Invalid multi-step mobile MFA response".into(),
        })?;
    let MultiStepMobileMfaResponse::Result { result } = response;
    let outcome = match result.outcome {
        MultiStepMobileMfaOutcome::Advanced(advanced) => {
            mfa_step_result::Outcome::Advanced(advanced)
        }
        MultiStepMobileMfaOutcome::Completed(completed) if completed.preshared_key.is_empty() => {
            return Err(MfaError::MfaRejected {
                message: "mobile approval failed: Edge returned an empty preshared key".into(),
            });
        }
        MultiStepMobileMfaOutcome::Completed(completed) => {
            mfa_step_result::Outcome::Completed(completed)
        }
    };

    Ok(MfaFinishResponse {
        preshared_key: String::new(),
        result: Some(MfaStepResult {
            outcome: Some(outcome),
        }),
    })
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

#[derive(Clone, Copy)]
enum MfaRoute {
    Start,
    StepStart,
    Finish,
    Remote,
}

fn route_path(contract: MfaContract, route: MfaRoute) -> Option<&'static str> {
    match (contract, route) {
        (MfaContract::Legacy, MfaRoute::Start) => Some("api/v1/client-mfa/start"),
        (MfaContract::Legacy, MfaRoute::Finish) => Some("api/v1/client-mfa/finish"),
        (MfaContract::Legacy, MfaRoute::Remote) => Some("api/v1/client-mfa/remote"),
        (MfaContract::MultiStep, MfaRoute::Start) => Some("api/v1/mfa-flow/start"),
        (MfaContract::MultiStep, MfaRoute::StepStart) => Some("api/v1/mfa-flow/step-start"),
        (MfaContract::MultiStep, MfaRoute::Finish) => Some("api/v1/mfa-flow/step-finish"),
        (MfaContract::MultiStep, MfaRoute::Remote) => Some("api/v1/mfa-flow/remote"),
        (MfaContract::Legacy, MfaRoute::StepStart) => None,
    }
}

fn route_url(proxy_url: &Url, contract: MfaContract, route: MfaRoute) -> Result<Url, MfaError> {
    let name = match route {
        MfaRoute::Start => "MFA start",
        MfaRoute::StepStart => "MFA step start",
        MfaRoute::Finish => "MFA finish",
        MfaRoute::Remote => "MFA remote",
    };
    let path = route_path(contract, route).ok_or_else(|| MfaError::Other {
        message: "The legacy MFA contract does not support step start".into(),
    })?;
    proxy_url.join(path).map_err(|e| MfaError::Other {
        message: format!("Failed to build {name} URL: {e}"),
    })
}

fn started_step(step: MfaStepStarted) -> Result<MfaStepStartResponse, MfaError> {
    if step.step_attempt_id.is_empty() {
        return Err(MfaError::Other {
            message: "MFA flow response did not include a step attempt ID".into(),
        });
    }
    let (challenge, credential_ids) = match step.challenge {
        Some(mfa_step_started::Challenge::Signature(challenge)) => {
            (Some(challenge.challenge), Vec::new())
        }
        Some(mfa_step_started::Challenge::Fido2(challenge)) => {
            (Some(challenge.challenge), challenge.credential_ids)
        }
        None => (None, Vec::new()),
    };
    Ok(MfaStepStartResponse {
        step_attempt_id: Some(step.step_attempt_id),
        challenge,
        credential_ids,
    })
}

/// Start an MFA session using the persisted contract selected by the caller.
pub async fn mfa_start(
    contract: MfaContract,
    proxy_url: Url,
    request: MfaStartRequest,
) -> Result<MfaStartResponse, MfaError> {
    let Some(first_method) = request.selected_methods.first().copied() else {
        return Err(MfaError::Other {
            message: "MFA method plan is empty".into(),
        });
    };
    let selected_methods = request
        .selected_methods
        .iter()
        .map(|method| *method as i32)
        .collect::<Vec<_>>();
    let url = route_url(&proxy_url, contract, MfaRoute::Start)?;
    let client = http_client();
    let mut builder = match contract {
        MfaContract::Legacy => {
            #[allow(deprecated)]
            let request = ClientMfaStartRequest {
                location_id: request.location_id,
                pubkey: request.pubkey,
                method: first_method as i32,
                posture_data: request.posture_data,
            };
            client.post(url).json(&request)
        }
        MfaContract::MultiStep => {
            let request = MfaFlowStartRequest {
                location_id: request.location_id,
                pubkey: request.pubkey,
                posture_data: request.posture_data,
                selected_methods: selected_methods.clone(),
            };
            client.post(url).json(&request)
        }
    };
    for (key, value) in standard_headers() {
        builder = builder.header(key, value);
    }
    let response = builder.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;
    let response = match check_mfa_response(response).await {
        Ok(response) => response,
        Err(err) => return Err(rewrap_mobile_start_error(first_method as i32, err)),
    };

    match contract {
        MfaContract::Legacy => {
            let response: ClientMfaStartResponse =
                response.json().await.map_err(|e| MfaError::Other {
                    message: format!("Invalid MFA start response: {e}"),
                })?;
            if response.token.is_empty() {
                return Err(MfaError::Other {
                    message: "MFA start response did not include a token".into(),
                });
            }
            Ok(MfaStartResponse {
                token: response.token,
                first_step: MfaStepStartResponse {
                    step_attempt_id: None,
                    challenge: response.challenge,
                    credential_ids: Vec::new(),
                },
            })
        }
        MfaContract::MultiStep => {
            let response: MfaFlowStartResponse =
                response.json().await.map_err(|e| MfaError::Other {
                    message: format!("Invalid MFA flow start response: {e}"),
                })?;
            let accepted = match response.outcome {
                Some(mfa_flow_start_response::Outcome::Accepted(accepted)) => accepted,
                Some(mfa_flow_start_response::Outcome::Rejected(rejected)) => {
                    let messages = rejected
                        .rejections
                        .iter()
                        .map(|rejection| {
                            rejection_message(
                                rejection,
                                selected_methods.get(rejection.step as usize).copied(),
                            )
                        })
                        .collect::<Vec<_>>();
                    return Err(MfaError::MfaRejected {
                        message: messages.join(" "),
                    });
                }
                None => {
                    return Err(MfaError::Other {
                        message: "MFA flow start response did not include an outcome".into(),
                    });
                }
            };
            if accepted.token.is_empty() {
                return Err(MfaError::Other {
                    message: "MFA flow start response did not include a token".into(),
                });
            }
            let first_step = accepted.first_step.ok_or_else(|| MfaError::Other {
                message: "MFA flow start response did not include the first step".into(),
            })?;
            Ok(MfaStartResponse {
                token: accepted.token,
                first_step: started_step(first_step)?,
            })
        }
    }
}

fn rejection_message(rejection: &MfaStepRejection, selected_method: Option<i32>) -> String {
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
        MfaStartRejectionReason::MfaStartRejectionStepUnavailable
            if selected_method == Some(MfaMethod::MobileApprove as i32) =>
        {
            MOBILE_NOT_REGISTERED_MESSAGE.to_string()
        }
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
    contract: MfaContract,
    proxy_url: Url,
    token: String,
    method: MfaMethod,
) -> Result<MfaStepStartResponse, MfaError> {
    let request = MfaFlowStepStartRequest {
        token,
        method: method as i32,
    };
    let url = route_url(&proxy_url, contract, MfaRoute::StepStart)?;
    let mut builder = http_client().post(url).json(&request);
    for (key, value) in standard_headers() {
        builder = builder.header(key, value);
    }
    let response = builder.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;
    let response = check_mfa_response(response).await?;
    let response: MfaFlowStepStartResponse =
        response.json().await.map_err(|e| MfaError::Other {
            message: format!("Invalid MFA step start response: {e}"),
        })?;
    let started = response.started.ok_or_else(|| MfaError::Other {
        message: "MFA step start response did not include a step".into(),
    })?;
    started_step(started)
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
                    message: MOBILE_NOT_REGISTERED_MESSAGE.into(),
                };
            }
        }
    }
    err
}

fn finish_request_body(
    contract: MfaContract,
    request: MfaFinishRequest,
) -> Result<serde_json::Value, MfaError> {
    let value = match contract {
        MfaContract::Legacy => {
            if request.step_attempt_id.is_some() {
                return Err(MfaError::Other {
                    message: "Legacy MFA does not accept a step attempt ID".into(),
                });
            }
            let (code, auth_pub_key) = match request.submission {
                Some(MfaSubmission::Code(code)) => (Some(code), None),
                None => (None, None),
                Some(MfaSubmission::Fido2(_)) => {
                    return Err(MfaError::Other {
                        message: "FIDO2 requires the multi-step MFA contract".into(),
                    });
                }
            };
            #[allow(deprecated)]
            serde_json::to_value(ClientMfaFinishRequest {
                token: request.token,
                code,
                auth_pub_key,
            })
        }
        MfaContract::MultiStep => {
            let step_attempt_id = request
                .step_attempt_id
                .filter(|id| !id.is_empty())
                .ok_or_else(|| MfaError::Other {
                    message: "MFA flow request did not include a step attempt ID".into(),
                })?;
            let submission = request.submission.map(|submission| match submission {
                MfaSubmission::Code(code) => {
                    mfa_flow_step_finish_request::Submission::Code(MfaCodeCredential { code })
                }
                MfaSubmission::Fido2(assertion) => {
                    mfa_flow_step_finish_request::Submission::Fido2(assertion)
                }
            });
            serde_json::to_value(MfaFlowStepFinishRequest {
                token: request.token,
                step_attempt_id,
                submission,
            })
        }
    };
    value.map_err(|e| MfaError::Other {
        message: format!("Failed to encode MFA finish request: {e}"),
    })
}

async fn decode_finish_response(
    contract: MfaContract,
    response: Response,
) -> Result<MfaFinishResponse, MfaError> {
    match contract {
        MfaContract::Legacy => {
            #[allow(deprecated)]
            let response: ClientMfaFinishResponse =
                response.json().await.map_err(|e| MfaError::Other {
                    message: format!("Invalid MFA finish response: {e}"),
                })?;
            Ok(MfaFinishResponse {
                preshared_key: response.preshared_key,
                result: None,
            })
        }
        MfaContract::MultiStep => {
            let response: MfaFlowStepFinishResponse =
                response.json().await.map_err(|e| MfaError::Other {
                    message: format!("Invalid MFA flow finish response: {e}"),
                })?;
            let result = response.result.ok_or_else(|| MfaError::Other {
                message: "MFA flow finish response did not include a result".into(),
            })?;
            Ok(MfaFinishResponse {
                preshared_key: String::new(),
                result: Some(result),
            })
        }
    }
}

pub async fn mfa_finish(
    contract: MfaContract,
    proxy_url: Url,
    request: MfaFinishRequest,
) -> Result<MfaFinishResponse, MfaError> {
    let url = route_url(&proxy_url, contract, MfaRoute::Finish)?;
    let body = finish_request_body(contract, request)?;
    let mut builder = http_client().post(url).json(&body);
    for (key, value) in standard_headers() {
        builder = builder.header(key, value);
    }
    let response = builder.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;
    let response = check_mfa_response(response).await?;
    decode_finish_response(contract, response).await
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
    contract: MfaContract,
    proxy_url: Url,
    token: String,
    step_attempt_id: Option<String>,
    cancel: CancellationToken,
) -> Result<MfaFinishResponse, MfaError> {
    let client = http_client();
    let url = route_url(&proxy_url, contract, MfaRoute::Finish)?;
    let body = finish_request_body(
        contract,
        MfaFinishRequest {
            token,
            step_attempt_id,
            submission: None,
        },
    )?;
    let deadline = Instant::now() + OIDC_POLL_TIMEOUT;

    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            return Err(MfaError::Timeout);
        }

        let mut request = client.post(url.clone()).json(&body);
        for (key, value) in standard_headers() {
            request = request.header(key, value);
        }

        select! {
            () = cancel.cancelled() => return Err(MfaError::Cancelled),
            result = request.send() => {
                let response = result.map_err(|err| MfaError::NetworkError {
                    message: format!("Failed to reach Edge: {err}"),
                })?;
                let status = response.status();
                if status == StatusCode::OK {
                    let response = decode_finish_response(contract, response).await?;
                    match response.result.as_ref().and_then(|result| result.outcome.as_ref()) {
                        None if contract == MfaContract::Legacy => return Ok(response),
                        Some(mfa_step_result::Outcome::AwaitingExternal(_)) => {}
                        Some(mfa_step_result::Outcome::Advanced(_) | mfa_step_result::Outcome::Completed(_)) => {
                            return Ok(response);
                        }
                        None => {
                            return Err(MfaError::Other {
                                message: "The server returned an unexpected verification state".into(),
                            });
                        }
                    }
                } else if status != StatusCode::PRECONDITION_REQUIRED {
                    return Err(check_mfa_response(response).await.err().unwrap_or(
                        MfaError::Other { message: format!("Unexpected status: {status}") },
                    ));
                }
            }
        }

        select! {
            () = cancel.cancelled() => return Err(MfaError::Cancelled),
            () = sleep(OIDC_POLL_INTERVAL) => {}
        }
    }
}

/// Return the preshared key only when the MFA session completed.
#[must_use]
pub fn completed_preshared_key(response: &MfaFinishResponse) -> Option<String> {
    let key = match response
        .result
        .as_ref()
        .and_then(|result| result.outcome.as_ref())
    {
        Some(mfa_step_result::Outcome::Completed(completed)) => &completed.preshared_key,
        Some(_) => return None,
        None => &response.preshared_key,
    };
    (!key.is_empty()).then(|| key.clone())
}

/// Waits for mobile approval after the QR code is shown. Returns cancellation or
/// timeout errors when applicable.
pub async fn connect_mobile_approve(
    contract: MfaContract,
    ws_url: &str,
    cancel: CancellationToken,
) -> Result<MfaFinishResponse, MfaError> {
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

    wait_for_mfa_outcome(contract, ws_stream, cancel).await
}

/// Derive the contract-specific WebSocket URL from the proxy base and session identifiers.
pub fn derive_ws_url(
    contract: MfaContract,
    proxy_base: &Url,
    token: &str,
    step_attempt_id: Option<&str>,
) -> Result<String, MfaError> {
    let mut ws_url = route_url(proxy_base, contract, MfaRoute::Remote)?;
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
    let mut query = ws_url.query_pairs_mut();
    query.append_pair("token", token);
    if contract == MfaContract::MultiStep {
        let step_attempt_id =
            step_attempt_id
                .filter(|id| !id.is_empty())
                .ok_or_else(|| MfaError::Other {
                    message: "MFA flow remote request did not include a step attempt ID".into(),
                })?;
        query.append_pair("step_attempt_id", step_attempt_id);
    }
    drop(query);

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
    contract: MfaContract,
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    cancel: CancellationToken,
) -> Result<MfaFinishResponse, MfaError> {
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
            Message::Text(text) => match contract {
                MfaContract::Legacy => {
                    match serde_json::from_str::<LegacyMobileMfaResponse>(&text) {
                        Ok(LegacyMobileMfaResponse::Success { preshared_key }) => {
                            if preshared_key.is_empty() {
                                return Err(MfaError::MfaRejected {
                                    message: "mobile approval failed: Edge returned an empty preshared key"
                                        .into(),
                                });
                            }

                            return Ok(MfaFinishResponse {
                                preshared_key,
                                result: None,
                            });
                        }
                        // Preserve legacy handling of frames that are not mfa_success.
                        Err(err) => debug!("Ignoring unrecognized mobile MFA frame: {err}"),
                    }
                }
                MfaContract::MultiStep => {
                    return decode_multi_step_mobile_mfa_frame(&text);
                }
            },
            Message::Close(_) if contract == MfaContract::MultiStep => {
                return Err(mobile_approve_closed(None));
            }
            Message::Close(frame) => {
                close_detail = Some(match frame {
                    Some(frame) if frame.reason.is_empty() => {
                        format!("code {}", u16::from(frame.code))
                    }
                    Some(frame) => format!("code {}: {}", u16::from(frame.code), frame.reason),
                    None => "no close reason".to_string(),
                });
            }
            Message::Binary(_) if contract == MfaContract::MultiStep => {
                return Err(MfaError::Other {
                    message: "Multi-step mobile MFA returned an unexpected frame".into(),
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests;
