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

/// Convert `WMIError` to `UnavailableReason`.
#[cfg(windows)]
impl From<wmi::WMIError> for UnavailableReason {
    fn from(err: wmi::WMIError) -> Self {
        if let wmi::WMIError::HResultError { .. } = err {
            UnavailableReason::InsufficientPermissions
        } else {
            UnavailableReason::DetectionFailed
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::defguard::enterprise::posture::v2::{
        bool_check, int32_check, string_check, BoolCheck, Int32Check, StringCheck,
        UnavailableReason,
    };

    #[test]
    fn test_bool_check_ok() {
        let check = BoolCheck::from(Ok(true));
        assert_eq!(check.result, Some(bool_check::Result::Value(true)));
    }

    #[test]
    fn test_bool_check_unavailable() {
        let check = BoolCheck::from(Err(UnavailableReason::DetectionFailed));
        assert_eq!(
            check.result,
            Some(bool_check::Result::Unavailable(
                UnavailableReason::DetectionFailed as i32
            ))
        );
    }

    #[test]
    fn test_int32_check_ok() {
        let check = Int32Check::from(Ok(42));
        assert_eq!(check.result, Some(int32_check::Result::Value(42)));
    }

    #[test]
    fn test_int32_check_unavailable() {
        let check = Int32Check::from(Err(UnavailableReason::NotApplicable));
        assert_eq!(
            check.result,
            Some(int32_check::Result::Unavailable(
                UnavailableReason::NotApplicable as i32
            ))
        );
    }

    #[test]
    fn test_string_check_ok() {
        let check = StringCheck::from(Ok("1.2.3".to_string()));
        assert_eq!(
            check.result,
            Some(string_check::Result::Value("1.2.3".to_string()))
        );
    }

    #[test]
    fn test_string_check_unavailable() {
        let check = StringCheck::from(Err(UnavailableReason::InsufficientPermissions));
        assert_eq!(
            check.result,
            Some(string_check::Result::Unavailable(
                UnavailableReason::InsufficientPermissions as i32
            ))
        );
    }

    #[test]
    fn test_unavailable_reason_display() {
        assert_eq!(UnavailableReason::Unspecified.to_string(), "unspecified");
        assert_eq!(
            UnavailableReason::DetectionFailed.to_string(),
            "detection failed"
        );
        assert_eq!(
            UnavailableReason::NotApplicable.to_string(),
            "not applicable on this platform"
        );
        assert_eq!(
            UnavailableReason::InsufficientPermissions.to_string(),
            "insufficient permissions"
        );
    }
}
