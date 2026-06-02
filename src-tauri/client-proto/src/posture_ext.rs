use std::fmt;

use crate::defguard::enterprise::posture::v2::{
    bool_check, int32_check, string_check, BoolCheck, Int32Check, StringCheck, UnavailableReason,
};

impl fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => f.write_str("unspecified"),
            Self::DetectionFailed => f.write_str("detection failed"),
            Self::NotApplicable => f.write_str("not applicable on this platform"),
            Self::InsufficientPermissions => f.write_str("insufficient permissions"),
        }
    }
}

/// Convert `Result` to `BoolCheck`.
impl From<Result<bool, UnavailableReason>> for BoolCheck {
    fn from(value: Result<bool, UnavailableReason>) -> Self {
        Self {
            result: Some(match value {
                Ok(inner) => bool_check::Result::Value(inner),
                Err(err) => bool_check::Result::Unavailable(err as i32),
            }),
        }
    }
}

/// Convert `Result` to `Int32Check`.
impl From<Result<i32, UnavailableReason>> for Int32Check {
    fn from(value: Result<i32, UnavailableReason>) -> Self {
        Self {
            result: Some(match value {
                Ok(inner) => int32_check::Result::Value(inner),
                Err(err) => int32_check::Result::Unavailable(err as i32),
            }),
        }
    }
}

/// Convert `Result` to `StringCheck`.
impl From<Result<String, UnavailableReason>> for StringCheck {
    fn from(value: Result<String, UnavailableReason>) -> Self {
        Self {
            result: Some(match value {
                Ok(inner) => string_check::Result::Value(inner),
                Err(err) => string_check::Result::Unavailable(err as i32),
            }),
        }
    }
}
