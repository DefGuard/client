use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(crate) struct StartRequest<'a> {
    pub token: &'a str,
}

#[derive(Serialize)]
pub(crate) struct StatusRequest<'a> {
    pub token: &'a str,
    pub session_id: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Algorithm {
    Sha1,
    /// An algorithm from a newer Edge, rejected when the session is built.
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
pub(crate) struct StartResponse {
    pub session_id: i64,
    /// Raw RFC 6238 secret in base64, not the base32 used by authenticator apps.
    pub secret: String,
    pub algorithm: Algorithm,
    pub digits: u32,
    /// Seconds.
    pub period: u32,
    /// Unix timestamp in seconds.
    pub expires_at: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Verified,
    Blocked,
    Cancelled,
    Expired,
    /// An outcome from a newer Core, waited on like pending.
    #[serde(other)]
    Unknown,
}

impl Outcome {
    #[must_use]
    pub fn is_final(self) -> bool {
        !matches!(self, Self::Pending | Self::Unknown)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct VerificationStatus {
    pub session_id: i64,
    pub outcome: Outcome,
    /// Unix timestamp in seconds.
    pub expires_at: i64,
    /// Unix timestamp in seconds, set only for an administrator decision.
    #[serde(default)]
    pub resolved_at: Option<i64>,
}

/// The single frame Edge sends over the await socket before closing it.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum SocketMessage {
    Status(VerificationStatus),
    Error {
        error: Option<String>,
        code: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_start_response_matches_edge() {
        let response: StartResponse = serde_json::from_value(json!({
            "session_id": 7,
            "secret": "AAEC",
            "algorithm": "sha1",
            "digits": 6,
            "period": 60,
            "expires_at": 1_700_000_000,
        }))
        .unwrap();
        assert_eq!(response.session_id, 7);
        assert_eq!(response.algorithm, Algorithm::Sha1);
        assert_eq!(response.digits, 6);
        assert_eq!(response.period, 60);

        let response: StartResponse = serde_json::from_value(json!({
            "session_id": 7,
            "secret": "AAEC",
            "algorithm": "sha512",
            "digits": 6,
            "period": 60,
            "expires_at": 1_700_000_000,
        }))
        .unwrap();
        assert_eq!(response.algorithm, Algorithm::Unknown);
    }

    #[test]
    fn test_status_names_every_outcome() {
        for (name, outcome) in [
            ("pending", Outcome::Pending),
            ("verified", Outcome::Verified),
            ("blocked", Outcome::Blocked),
            ("cancelled", Outcome::Cancelled),
            ("expired", Outcome::Expired),
            ("from_the_future", Outcome::Unknown),
        ] {
            let status: VerificationStatus = serde_json::from_value(json!({
                "session_id": 3,
                "outcome": name,
                "expires_at": 100,
            }))
            .unwrap();
            assert_eq!(status.outcome, outcome);
            assert_eq!(status.resolved_at, None);
        }
        assert!(!Outcome::Pending.is_final());
        assert!(!Outcome::Unknown.is_final());
        assert!(Outcome::Verified.is_final());
        assert!(Outcome::Blocked.is_final());
        assert!(Outcome::Cancelled.is_final());
        assert!(Outcome::Expired.is_final());
    }

    #[test]
    fn test_socket_messages_match_edge() {
        let message: SocketMessage = serde_json::from_value(json!({
            "type": "status",
            "session_id": 3,
            "outcome": "blocked",
            "expires_at": 100,
            "resolved_at": 50,
        }))
        .unwrap();
        let SocketMessage::Status(status) = message else {
            panic!("expected a status, got {message:?}");
        };
        assert_eq!(
            status,
            VerificationStatus {
                session_id: 3,
                outcome: Outcome::Blocked,
                expires_at: 100,
                resolved_at: Some(50),
            }
        );

        let message: SocketMessage = serde_json::from_value(json!({
            "type": "error",
            "error": "invalid token",
            "code": null,
        }))
        .unwrap();
        let SocketMessage::Error { error, code } = message else {
            panic!("expected an error, got {message:?}");
        };
        assert_eq!(error.as_deref(), Some("invalid token"));
        assert_eq!(code, None);
    }
}
