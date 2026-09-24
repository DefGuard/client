//! Post-enrollment MFA factor configuration over HTTP. The proxy mints a short-lived session
//! from the device's polling token, which stands in for the enrollment cookie.

use std::fmt;

use defguard_client_proto::defguard::client_types::{
    CodeMfaSetupFinishRequest, CodeMfaSetupFinishResponse, CodeMfaSetupStartRequest,
    CodeMfaSetupStartResponse, MfaConfigAuthorizeRequest, MfaConfigAuthorizeResponse,
    MfaConfigSendCodeRequest, MfaConfigStartRequest, MfaConfigStartResponse, MfaMethod,
};
use reqwest::{Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

use crate::{
    database::models::Id,
    proxy::{post_with_headers, read_error_message},
};

// No leading slash: it would make `Url::join` discard a proxy base path (`https://host/defguard/`).
const START: &str = "api/v1/mfa-config/start";
const SEND_CODE: &str = "api/v1/mfa-config/send-code";
const AUTHORIZE: &str = "api/v1/mfa-config/authorize";
const SETUP_START: &str = "api/v1/mfa-config/setup/start";
const SETUP_FINISH: &str = "api/v1/mfa-config/setup/finish";

// `MfaConfigAuthorizeRequest` carries only a `code`, so a non-code factor cannot authorize here.
pub const AUTHORIZING_METHODS: &[MfaMethod] = &[MfaMethod::Totp, MfaMethod::Email];

// Mirrors the methods Core accepts in `mfa_setup_start` / `mfa_setup_finish`, keep in step.
pub const CONFIGURABLE_METHODS: &[MfaMethod] =
    &[MfaMethod::Totp, MfaMethod::Email, MfaMethod::Fido2];

/// What proves a factor was set up. Split so the FIDO2-only fields of
/// `CodeMfaSetupFinishRequest` are unrepresentable for a code factor, and vice versa.
pub enum SetupProof {
    Code(String),
    Fido2 { name: String, attestation: String },
}

/// One authorized session configures several factors, so it outlives a single setup.
#[derive(Clone)]
pub struct MfaConfigSession {
    pub instance_id: Id,
    pub proxy_url: Url,
    pub session_token: String,
    pub deadline_timestamp: i64,
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

fn ensure_can_authorize(method: MfaMethod) -> Result<(), MfaConfigError> {
    if AUTHORIZING_METHODS.contains(&method) {
        Ok(())
    } else {
        Err(MfaConfigError::UnsupportedMethod {
            message: format!(
                "A {} cannot authorize MFA configuration; use a one-time code instead.",
                method_name(method)
            ),
        })
    }
}

fn ensure_can_configure(method: MfaMethod) -> Result<(), MfaConfigError> {
    if CONFIGURABLE_METHODS.contains(&method) {
        Ok(())
    } else {
        Err(MfaConfigError::UnsupportedMethod {
            message: format!(
                "Configuring a {} from the desktop client is not supported.",
                method_name(method)
            ),
        })
    }
}

/// Unknown method numbers are dropped so a newer Core cannot break an older client.
#[must_use]
pub fn authorizing_methods(response: &MfaConfigStartResponse) -> Vec<MfaMethod> {
    response
        .available_methods
        .iter()
        .filter_map(|value| MfaMethod::try_from(*value).ok())
        .filter(|method| AUTHORIZING_METHODS.contains(method))
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
        StatusCode::UNAUTHORIZED => Err(MfaConfigError::SessionExpired),
        StatusCode::BAD_REQUEST => Err(MfaConfigError::InvalidCode { message }),
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

pub async fn mfa_config_authorize(
    proxy_url: Url,
    session_token: String,
    method: MfaMethod,
    code: String,
) -> Result<MfaConfigAuthorizeResponse, MfaConfigError> {
    ensure_can_authorize(method)?;
    debug!("Authorizing MFA configuration session");
    let request = MfaConfigAuthorizeRequest {
        session_token,
        method: method as i32,
        code,
    };
    parse(post(&proxy_url, AUTHORIZE, &request).await?).await
}

pub async fn mfa_config_setup_start(
    proxy_url: Url,
    session_token: String,
    method: MfaMethod,
) -> Result<CodeMfaSetupStartResponse, MfaConfigError> {
    ensure_can_configure(method)?;
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
) -> Result<CodeMfaSetupFinishResponse, MfaConfigError> {
    ensure_can_configure(method)?;
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

#[cfg(test)]
mod tests;
