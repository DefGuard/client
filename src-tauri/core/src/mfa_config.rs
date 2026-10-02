//! Post-enrollment MFA factor configuration over HTTP. The proxy mints a short-lived session
//! from the device's polling token, which stands in for the enrollment cookie.

use std::{fmt, time::Duration};

use chrono::Utc;
use defguard_client_proto::defguard::client_types::{
    CodeMfaSetupFinishRequest, CodeMfaSetupFinishResponse, CodeMfaSetupStartRequest,
    CodeMfaSetupStartResponse, MfaConfigAuthorizeRequest, MfaConfigAuthorizeResponse,
    MfaConfigEndRequest, MfaConfigFido2ChallengeRequest, MfaConfigFido2ChallengeResponse,
    MfaConfigSendCodeRequest, MfaConfigStartRequest, MfaConfigStartResponse, MfaMethod,
};
use reqwest::{Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tokio::{
    select,
    time::{sleep, Instant},
};
use tokio_util::sync::CancellationToken;

use crate::{
    database::models::{instance::MfaCapabilities, Id},
    mfa::{OIDC_POLL_INTERVAL, OIDC_POLL_TIMEOUT},
    proxy::{post_with_headers, read_error_message},
};

// No leading slash: it would make `Url::join` discard a proxy base path (`https://host/defguard/`).
const START: &str = "api/v1/mfa-config/start";
const SEND_CODE: &str = "api/v1/mfa-config/send-code";
const AUTHORIZE: &str = "api/v1/mfa-config/authorize";
const FIDO2_CHALLENGE: &str = "api/v1/mfa-config/fido2-challenge";
const SETUP_START: &str = "api/v1/mfa-config/setup/start";
const SETUP_FINISH: &str = "api/v1/mfa-config/setup/finish";
const END: &str = "api/v1/mfa-config/end";

// what this client can authorize with, narrowed per instance by `MfaCapabilities`
pub const AUTHORIZING_METHODS: &[MfaMethod] = &[
    MfaMethod::Totp,
    MfaMethod::Email,
    MfaMethod::Fido2,
    MfaMethod::Oidc,
];

// FIDO2 sends an assertion and OIDC completes in the browser, so only these send a code
const CODE_METHODS: &[MfaMethod] = &[MfaMethod::Totp, MfaMethod::Email];

// Core reuses 401, 403 and 428 for several errors, only the message tells these apart
const INVALID_CODE_MESSAGE: &str = "invalid code";
const METHOD_NOT_CONFIGURED_MESSAGE: &str = "method not configured";
const ALREADY_AUTHORIZED_MESSAGE: &str = "session already authorized";
// a prefix, Core's login flow words it "OIDC authentication not completed yet"
const OIDC_PENDING_MESSAGE: &str = "OIDC authentication not completed";

// what this client can set up, narrowed per instance by `MfaCapabilities`
pub const CONFIGURABLE_METHODS: &[MfaMethod] =
    &[MfaMethod::Totp, MfaMethod::Email, MfaMethod::Fido2];

/// What proves a factor was set up. Split so the FIDO2-only fields of
/// `CodeMfaSetupFinishRequest` are unrepresentable for a code factor, and vice versa.
pub enum SetupProof {
    Code(String),
    Fido2 { name: String, attestation: String },
}

pub enum AuthorizeProof {
    Code {
        method: MfaMethod,
        code: String,
    },
    Fido2 {
        signature: Vec<u8>,
        auth_data: Vec<u8>,
        credential_id: Vec<u8>,
    },
    /// Core has already seen the browser login, so there is nothing to send
    Oidc,
}

impl AuthorizeProof {
    fn method(&self) -> MfaMethod {
        match self {
            Self::Code { method, .. } => *method,
            Self::Fido2 { .. } => MfaMethod::Fido2,
            Self::Oidc => MfaMethod::Oidc,
        }
    }
}

/// One authorized session configures several factors, so it outlives a single setup.
#[derive(Clone)]
pub struct MfaConfigSession {
    pub instance_id: Id,
    pub proxy_url: Url,
    pub session_token: String,
    pub deadline_timestamp: i64,
    pub capabilities: MfaCapabilities,
}

impl MfaConfigSession {
    #[must_use]
    pub fn is_expired(&self, now: i64) -> bool {
        self.deadline_timestamp <= now
    }
}

// `session_token` is a proxy bearer token, never let it reach a log line.
impl fmt::Debug for MfaConfigSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MfaConfigSession")
            .field("instance_id", &self.instance_id)
            .field("proxy_url", &self.proxy_url)
            .field("session_token", &"<redacted>")
            .field("deadline_timestamp", &self.deadline_timestamp)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MfaConfigError {
    #[error("This Defguard instance does not support configuring MFA from the client")]
    Unsupported,

    #[error("MFA configuration session expired or was rejected")]
    SessionExpired,

    #[error("{message}")]
    InvalidCode { message: String },

    #[error("This device has no polling token; update the instance and try again")]
    NoToken,

    #[error("{message}")]
    UnsupportedMethod { message: String },

    /// Always user-fixable, so the message is written to be shown as it is.
    #[error("{message}")]
    SecurityKey { message: String },

    /// The user backed out of a ceremony, so there is nothing to tell them about it.
    #[error("MFA configuration was cancelled")]
    Cancelled,

    /// e.g. a FIDO2 challenge for a user with no security key
    #[error("{message}")]
    MethodNotConfigured { message: String },

    /// e.g. an inactive user or too many attempts, Core words these for the user
    #[error("{message}")]
    Forbidden { message: String },

    /// never leaves the OIDC poll loop, which retries until the browser login completes
    #[error("OpenID authentication is not completed yet")]
    OidcPending,

    /// the response that authorized it was lost, so the session cannot be resumed
    #[error("MFA configuration session is already authorized")]
    AlreadyAuthorized,

    /// e.g. no FIDO2 challenge pending
    #[error("{message}")]
    FailedPrecondition { message: String },

    #[error("Timed out waiting for authentication")]
    Timeout,

    #[error("{message}")]
    NetworkError { message: String },

    #[error("Proxy error (HTTP {status}): {message}")]
    ProxyError { status: u16, message: String },

    #[error("{message}")]
    Other { message: String },
}

fn method_name(method: MfaMethod) -> &'static str {
    match method {
        MfaMethod::Totp => "authenticator app",
        MfaMethod::Email => "email",
        MfaMethod::Oidc => "OpenID",
        MfaMethod::Biometric => "mobile biometric authentication",
        MfaMethod::MobileApprove => "mobile app approval",
        MfaMethod::Fido2 => "security key",
    }
}

fn ensure_can_authorize(
    proof: &AuthorizeProof,
    capabilities: &MfaCapabilities,
) -> Result<(), MfaConfigError> {
    let method = proof.method();
    if !AUTHORIZING_METHODS.contains(&method) {
        return Err(MfaConfigError::UnsupportedMethod {
            message: format!(
                "A {} cannot authorize MFA configuration.",
                method_name(method)
            ),
        });
    }
    if !capabilities.can_authorize(method.into()) {
        return Err(MfaConfigError::UnsupportedMethod {
            message: format!(
                "This Defguard instance does not accept a {} to authorize MFA configuration.",
                method_name(method)
            ),
        });
    }
    if matches!(proof, AuthorizeProof::Code { .. }) && !CODE_METHODS.contains(&method) {
        return Err(MfaConfigError::UnsupportedMethod {
            message: format!(
                "A {} does not authorize with a one-time code.",
                method_name(method)
            ),
        });
    }
    Ok(())
}

fn ensure_can_configure(
    method: MfaMethod,
    capabilities: &MfaCapabilities,
) -> Result<(), MfaConfigError> {
    if !CONFIGURABLE_METHODS.contains(&method) {
        return Err(MfaConfigError::UnsupportedMethod {
            message: format!(
                "Configuring a {} from the desktop client is not supported.",
                method_name(method)
            ),
        });
    }
    if !capabilities.can_set_up(method.into()) {
        return Err(MfaConfigError::UnsupportedMethod {
            message: format!(
                "This Defguard instance does not support configuring a {} from the desktop client.",
                method_name(method)
            ),
        });
    }
    Ok(())
}

/// Unknown method numbers are dropped so a newer Core cannot break an older client.
#[must_use]
pub fn authorizing_methods(
    response: &MfaConfigStartResponse,
    capabilities: &MfaCapabilities,
) -> Vec<MfaMethod> {
    response
        .available_methods
        .iter()
        .filter_map(|value| MfaMethod::try_from(*value).ok())
        .filter(|method| {
            AUTHORIZING_METHODS.contains(method) && capabilities.can_authorize((*method).into())
        })
        .collect()
}

fn build_url(proxy_url: &Url, endpoint: &str) -> Result<Url, MfaConfigError> {
    proxy_url
        .join(endpoint)
        .map_err(|err| MfaConfigError::Other {
            message: format!("Failed to build MFA configuration URL: {err}"),
        })
}

async fn check_response(response: Response, endpoint: &str) -> Result<Response, MfaConfigError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    // Only `START` can 404 on an older proxy, elsewhere it means a wrong base path.
    if status == StatusCode::NOT_FOUND && endpoint == START {
        return Err(MfaConfigError::Unsupported);
    }

    let message = read_error_message(response).await;
    match status {
        // a wrong code or FIDO2 assertion, any other 401 is a session Core no longer knows
        StatusCode::UNAUTHORIZED if message == INVALID_CODE_MESSAGE => {
            Err(MfaConfigError::InvalidCode { message })
        }
        StatusCode::UNAUTHORIZED => Err(MfaConfigError::SessionExpired),
        StatusCode::BAD_REQUEST => Err(MfaConfigError::InvalidCode { message }),
        StatusCode::FORBIDDEN if message == METHOD_NOT_CONFIGURED_MESSAGE => {
            Err(MfaConfigError::MethodNotConfigured { message })
        }
        StatusCode::FORBIDDEN => Err(MfaConfigError::Forbidden { message }),
        // other 428s end a poll, so the status alone cannot mean keep polling
        StatusCode::PRECONDITION_REQUIRED if message.starts_with(OIDC_PENDING_MESSAGE) => {
            Err(MfaConfigError::OidcPending)
        }
        StatusCode::PRECONDITION_REQUIRED if message == ALREADY_AUTHORIZED_MESSAGE => {
            Err(MfaConfigError::AlreadyAuthorized)
        }
        StatusCode::PRECONDITION_REQUIRED => Err(MfaConfigError::FailedPrecondition { message }),
        _ => Err(MfaConfigError::ProxyError {
            status: status.as_u16(),
            message,
        }),
    }
}

async fn post<T: Serialize + ?Sized>(
    proxy_url: &Url,
    endpoint: &str,
    body: &T,
) -> Result<Response, MfaConfigError> {
    let url = build_url(proxy_url, endpoint)?;
    let response =
        post_with_headers(url, body)
            .await
            .map_err(|err| MfaConfigError::NetworkError {
                message: format!("Failed to reach proxy: {err}"),
            })?;
    check_response(response, endpoint).await
}

async fn parse<T: DeserializeOwned>(response: Response) -> Result<T, MfaConfigError> {
    response.json().await.map_err(|err| MfaConfigError::Other {
        message: format!("Invalid MFA configuration response: {err}"),
    })
}

pub async fn mfa_config_start(
    proxy_url: Url,
    token: String,
    pubkey: String,
) -> Result<MfaConfigStartResponse, MfaConfigError> {
    debug!("Starting MFA configuration session");
    let request = MfaConfigStartRequest { token, pubkey };
    parse(post(&proxy_url, START, &request).await?).await
}

/// The proxy answers with an empty 200, so the body is discarded rather than parsed.
pub async fn mfa_config_send_code(
    proxy_url: Url,
    session_token: String,
) -> Result<(), MfaConfigError> {
    debug!("Requesting MFA configuration email code");
    let request = MfaConfigSendCodeRequest { session_token };
    post(&proxy_url, SEND_CODE, &request).await?;
    Ok(())
}

/// single-use, every authorize call that reaches verification consumes it
pub async fn mfa_config_fido2_challenge(
    proxy_url: Url,
    session_token: String,
) -> Result<MfaConfigFido2ChallengeResponse, MfaConfigError> {
    debug!("Requesting MFA configuration FIDO2 challenge");
    let request = MfaConfigFido2ChallengeRequest { session_token };
    parse(post(&proxy_url, FIDO2_CHALLENGE, &request).await?).await
}

pub async fn mfa_config_authorize(
    proxy_url: Url,
    session_token: String,
    proof: AuthorizeProof,
    capabilities: &MfaCapabilities,
) -> Result<MfaConfigAuthorizeResponse, MfaConfigError> {
    ensure_can_authorize(&proof, capabilities)?;
    debug!("Authorizing MFA configuration session");
    let method = proof.method() as i32;
    let (code, signature, auth_data, credential_id) = match proof {
        AuthorizeProof::Code { code, .. } => (code, None, None, None),
        AuthorizeProof::Fido2 {
            signature,
            auth_data,
            credential_id,
        } => (
            String::new(),
            Some(signature),
            Some(auth_data),
            Some(credential_id),
        ),
        AuthorizeProof::Oidc => (String::new(), None, None, None),
    };
    let request = MfaConfigAuthorizeRequest {
        session_token,
        method,
        code,
        signature,
        auth_data,
        credential_id,
    };
    parse(post(&proxy_url, AUTHORIZE, &request).await?).await
}

/// the browser login must already be open, this only polls for its result.
/// the first poll waits an interval, Core knows of the attempt only once the browser loads it
pub async fn mfa_config_poll_oidc(
    proxy_url: Url,
    session_token: String,
    deadline_timestamp: i64,
    capabilities: &MfaCapabilities,
    cancel: CancellationToken,
) -> Result<MfaConfigAuthorizeResponse, MfaConfigError> {
    let session_left = u64::try_from(deadline_timestamp - Utc::now().timestamp()).unwrap_or(0);
    let started = Instant::now();
    let session_deadline = started + Duration::from_secs(session_left);
    let poll_deadline = started + OIDC_POLL_TIMEOUT;

    loop {
        select! {
            () = cancel.cancelled() => return Err(MfaConfigError::Cancelled),
            () = sleep(OIDC_POLL_INTERVAL) => {}
        }

        let now = Instant::now();
        if now >= session_deadline {
            return Err(MfaConfigError::SessionExpired);
        }
        if now >= poll_deadline {
            return Err(MfaConfigError::Timeout);
        }

        // not raced against the cancel, Core may authorize even if the answer is dropped
        match mfa_config_authorize(
            proxy_url.clone(),
            session_token.clone(),
            AuthorizeProof::Oidc,
            capabilities,
        )
        .await
        {
            Err(MfaConfigError::OidcPending) => {}
            other => return other,
        }
    }
}

pub async fn mfa_config_setup_start(
    proxy_url: Url,
    session_token: String,
    method: MfaMethod,
    capabilities: &MfaCapabilities,
) -> Result<CodeMfaSetupStartResponse, MfaConfigError> {
    ensure_can_configure(method, capabilities)?;
    debug!("Starting MFA factor setup");
    let request = CodeMfaSetupStartRequest {
        method: method as i32,
        token: session_token,
    };
    parse(post(&proxy_url, SETUP_START, &request).await?).await
}

/// `recovery_codes` is empty unless this was the first factor, Core issues them only once.
pub async fn mfa_config_setup_finish(
    proxy_url: Url,
    session_token: String,
    method: MfaMethod,
    proof: SetupProof,
    capabilities: &MfaCapabilities,
) -> Result<CodeMfaSetupFinishResponse, MfaConfigError> {
    ensure_can_configure(method, capabilities)?;
    debug!("Finishing MFA factor setup");
    // The proto spells the unused half as empty rather than absent.
    let (code, name, fido2_attestation) = match proof {
        SetupProof::Code(code) => (code, None, None),
        SetupProof::Fido2 { name, attestation } => (String::new(), Some(name), Some(attestation)),
    };
    let request = CodeMfaSetupFinishRequest {
        code,
        token: session_token,
        method: method as i32,
        name,
        fido2_attestation,
    };
    parse(post(&proxy_url, SETUP_FINISH, &request).await?).await
}

/// Lets Core drop the session before its deadline. The proxy answers with an empty 200.
pub async fn mfa_config_end(proxy_url: Url, session_token: String) -> Result<(), MfaConfigError> {
    debug!("Ending MFA configuration session");
    let request = MfaConfigEndRequest { session_token };
    post(&proxy_url, END, &request).await?;
    Ok(())
}

#[cfg(test)]
mod tests;
