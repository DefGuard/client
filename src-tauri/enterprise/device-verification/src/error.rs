use serde::Serialize;
use thiserror::Error;

use crate::{Outcome, VerificationStatus};

/// Serialized tagged by type so the frontend can match on it.
#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Error {
    #[error("{message}")]
    Network { message: String },

    #[error("Device verification requires an Enterprise license")]
    LicenseRequired,

    /// The device token is missing or unknown: the device was removed or enrolled again.
    #[error("The device is no longer known to Defguard")]
    DeviceTokenInvalid,

    #[error("The user account is inactive")]
    UserInactive,

    #[error("The device is blocked")]
    DeviceBlocked,

    /// A new session has to be started.
    #[error("The verification session no longer exists")]
    SessionNotFound,

    #[error("Edge sent a verification code algorithm this client does not support")]
    UnsupportedAlgorithm,

    #[error("Invalid response from Edge: {message}")]
    InvalidResponse { message: String },

    #[error("Edge error (HTTP {status}): {message}")]
    Proxy { status: u16, message: String },

    #[error("Device verification cancelled")]
    Cancelled,

    /// No more codes, the session ended with this outcome.
    #[error("Device verification ended with outcome {outcome:?}")]
    Resolved {
        outcome: Outcome,
        resolved_at: Option<i64>,
    },

    /// A failure in this client rather than in Edge or Core.
    #[error("{message}")]
    Other { message: String },
}

impl Error {
    /// Maps an Edge error response. The machine readable code wins over the bare status.
    pub(crate) fn from_response(status: u16, body: Option<&serde_json::Value>) -> Self {
        let field = |name| {
            body.and_then(|body| body.get(name))
                .and_then(serde_json::Value::as_str)
        };
        if let Some(error) = Self::from_code(field("code")) {
            return error;
        }
        if status == 402 {
            return Self::LicenseRequired;
        }
        Self::Proxy {
            status,
            message: field("error").map_or_else(|| format!("HTTP {status}"), str::to_string),
        }
    }

    pub(crate) fn from_code(code: Option<&str>) -> Option<Self> {
        match code? {
            "device_token_invalid" => Some(Self::DeviceTokenInvalid),
            "user_inactive" => Some(Self::UserInactive),
            "device_blocked" => Some(Self::DeviceBlocked),
            "verification_session_not_found" => Some(Self::SessionNotFound),
            "license_required" => Some(Self::LicenseRequired),
            _ => None,
        }
    }

    #[must_use]
    pub fn resolved(status: &VerificationStatus) -> Self {
        Self::Resolved {
            outcome: status.outcome,
            resolved_at: status.resolved_at,
        }
    }

    /// Edge or Core being away or slow, worth retrying.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Network { .. } => true,
            Self::Proxy { status, .. } => *status >= 500,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_codes_win_over_status() {
        for (status, code, expected) in [
            (401, "device_token_invalid", Error::DeviceTokenInvalid),
            (403, "user_inactive", Error::UserInactive),
            (403, "device_blocked", Error::DeviceBlocked),
            (
                404,
                "verification_session_not_found",
                Error::SessionNotFound,
            ),
        ] {
            let body = json!({"error": "whatever", "code": code});
            assert_eq!(Error::from_response(status, Some(&body)), expected);
        }
    }

    #[test]
    fn test_status_without_code() {
        let body = json!({"error": "no valid license", "code": null});
        assert_eq!(
            Error::from_response(402, Some(&body)),
            Error::LicenseRequired
        );
        assert_eq!(
            Error::from_response(502, Some(&json!({"error": "core timeout", "code": null}))),
            Error::Proxy {
                status: 502,
                message: "core timeout".into()
            }
        );
        assert_eq!(
            Error::from_response(500, None),
            Error::Proxy {
                status: 500,
                message: "HTTP 500".into()
            }
        );
    }

    #[test]
    fn test_transient_errors() {
        assert!(Error::Network {
            message: String::new()
        }
        .is_transient());
        assert!(Error::from_response(503, None).is_transient());
        assert!(!Error::from_response(400, None).is_transient());
        assert!(!Error::DeviceBlocked.is_transient());
        assert!(!Error::SessionNotFound.is_transient());
        assert!(!Error::Cancelled.is_transient());
    }

    #[test]
    fn test_serializes_tagged() {
        assert_eq!(
            serde_json::to_value(Error::DeviceBlocked).unwrap(),
            json!({"type": "device_blocked"})
        );
        assert_eq!(
            serde_json::to_value(Error::Proxy {
                status: 500,
                message: "boom".into()
            })
            .unwrap(),
            json!({"type": "proxy", "status": 500, "message": "boom"})
        );
        assert_eq!(
            serde_json::to_value(Error::Resolved {
                outcome: Outcome::Expired,
                resolved_at: None,
            })
            .unwrap(),
            json!({"type": "resolved", "outcome": "expired", "resolved_at": null})
        );
    }
}
