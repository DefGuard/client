use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{prelude::BASE64_STANDARD, Engine};
use serde::Serialize;
use totp_lite::{totp_custom, Sha1};

use crate::{
    types::{Algorithm, StartResponse},
    Error,
};

/// Longest code totp_lite computes without overflowing.
const MAX_DIGITS: u32 = 9;

/// Holds the shared secret, so it must never leave the backend.
#[derive(Clone, PartialEq, Eq)]
pub struct VerificationSession {
    pub session_id: i64,
    secret: Vec<u8>,
    pub digits: u32,
    /// Seconds.
    pub period: u64,
    /// Unix timestamp in seconds.
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerificationCode {
    pub code: String,
    /// Seconds until the next code.
    pub expires_in: u64,
}

impl VerificationSession {
    /// Mirrors how Core computes the code it shows to the administrator.
    #[must_use]
    pub fn code_at(&self, unix_secs: u64) -> VerificationCode {
        VerificationCode {
            code: totp_custom::<Sha1>(self.period, self.digits, &self.secret, unix_secs),
            expires_in: self.period - unix_secs % self.period,
        }
    }

    #[must_use]
    pub fn current_code(&self) -> VerificationCode {
        self.code_at(unix_now())
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        i64::try_from(unix_now()).unwrap_or(i64::MAX) >= self.expires_at
    }
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl TryFrom<StartResponse> for VerificationSession {
    type Error = Error;

    fn try_from(response: StartResponse) -> Result<Self, Self::Error> {
        if response.algorithm != Algorithm::Sha1 {
            return Err(Error::UnsupportedAlgorithm);
        }
        if response.period == 0 || response.digits == 0 || response.digits > MAX_DIGITS {
            return Err(Error::InvalidResponse {
                message: format!(
                    "unsupported code parameters, {} digits every {} seconds",
                    response.digits, response.period
                ),
            });
        }
        let secret =
            BASE64_STANDARD
                .decode(&response.secret)
                .map_err(|err| Error::InvalidResponse {
                    message: format!("invalid verification secret: {err}"),
                })?;
        if secret.is_empty() {
            return Err(Error::InvalidResponse {
                message: "empty verification secret".into(),
            });
        }

        Ok(Self {
            session_id: response.session_id,
            secret,
            digits: response.digits,
            period: response.period.into(),
            expires_at: response.expires_at,
        })
    }
}

impl fmt::Debug for VerificationSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerificationSession")
            .field("session_id", &self.session_id)
            .field("secret", &"***")
            .field("digits", &self.digits)
            .field("period", &self.period)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SHA1 secret from RFC 6238 appendix B.
    const RFC_SECRET: &[u8] = b"12345678901234567890";

    fn start_response(secret: &[u8], digits: u32, period: u32) -> StartResponse {
        StartResponse {
            session_id: 1,
            secret: BASE64_STANDARD.encode(secret),
            algorithm: Algorithm::Sha1,
            digits,
            period,
            expires_at: 1_000,
        }
    }

    #[test]
    fn test_rfc_6238_vectors() {
        let session = VerificationSession::try_from(start_response(RFC_SECRET, 8, 30)).unwrap();
        for (time, code) in [
            (59, "94287082"),
            (1_111_111_109, "07081804"),
            (1_234_567_890, "89005924"),
            (20_000_000_000, "65353130"),
        ] {
            assert_eq!(session.code_at(time).code, code, "time {time}");
        }
    }

    #[test]
    fn test_core_parameters() {
        // Core uses 6 digits every 60 seconds. Counters 0 and 1 give the RFC 4226 HOTP values.
        let session = VerificationSession::try_from(start_response(RFC_SECRET, 6, 60)).unwrap();
        assert_eq!(session.code_at(0).code, "755224");
        assert_eq!(session.code_at(59).code, "755224");
        assert_eq!(session.code_at(60).code, "287082");
        assert_eq!(session.code_at(0).expires_in, 60);
        assert_eq!(session.code_at(59).expires_in, 1);
        assert_eq!(session.code_at(60).expires_in, 60);
    }

    #[test]
    fn test_rejects_bad_parameters() {
        let mut response = start_response(RFC_SECRET, 6, 60);
        response.algorithm = Algorithm::Unknown;
        assert_eq!(
            VerificationSession::try_from(response),
            Err(Error::UnsupportedAlgorithm)
        );
        for (digits, period) in [(0, 60), (10, 60), (6, 0)] {
            assert!(matches!(
                VerificationSession::try_from(start_response(RFC_SECRET, digits, period)),
                Err(Error::InvalidResponse { .. })
            ));
        }
        assert!(matches!(
            VerificationSession::try_from(start_response(b"", 6, 60)),
            Err(Error::InvalidResponse { .. })
        ));
        let mut response = start_response(RFC_SECRET, 6, 60);
        response.secret = "not base64!".into();
        assert!(matches!(
            VerificationSession::try_from(response),
            Err(Error::InvalidResponse { .. })
        ));
    }

    #[test]
    fn test_expiry() {
        let mut session = VerificationSession::try_from(start_response(RFC_SECRET, 6, 60)).unwrap();
        assert!(session.is_expired());
        session.expires_at = i64::try_from(unix_now()).unwrap() + 60;
        assert!(!session.is_expired());
    }

    #[test]
    fn test_debug_hides_secret() {
        let session = VerificationSession::try_from(start_response(RFC_SECRET, 6, 60)).unwrap();
        let debug = format!("{session:?}");
        assert!(!debug.contains("12345678901234567890"));
        assert!(!debug.contains("49, 50"));
        assert!(debug.contains("***"));
    }
}
