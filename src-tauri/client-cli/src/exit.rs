use crate::state::CliError;

/// Map a `CliError` variant to the corresponding process exit code.
///
/// | Code | Meaning            |
/// |------|--------------------|
/// | 0    | ok                 |
/// | 1    | other              |
/// | 2    | usage              |
/// | 3    | not-found          |
/// | 4    | daemon-unavailable |
/// | 5    | mfa-failed         |
/// | 6    | not-enrolled       |
/// | 7    | invalid-input      |
pub fn exit_code_for(err: &CliError) -> u8 {
    match err {
        CliError::Usage(_) => 2,
        CliError::NotFound(_) => 3,
        CliError::DaemonUnavailable(_) => 4,
        CliError::MfaFailed(_) | CliError::MfaInputRequired(_) => 5,
        CliError::NotEnrolled(_) => 6,
        CliError::InvalidInput(_) => 7,
        CliError::Database(_) => 1,
        CliError::Other(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_mapping() {
        let cases: &[(&CliError, u8)] = &[
            (&CliError::Usage("bad flag".into()), 2),
            (&CliError::NotFound("no such location".into()), 3),
            (&CliError::DaemonUnavailable("daemon down".into()), 4),
            (&CliError::MfaFailed("wrong code".into()), 5),
            (&CliError::MfaInputRequired("no TTY".into()), 5),
            (&CliError::NotEnrolled("no instances".into()), 6),
            (
                &CliError::InvalidInput("route_all_traffic enforced".into()),
                7,
            ),
            (
                &CliError::Database(sqlx::Error::Protocol("bad schema".into())),
                1,
            ),
            (&CliError::Other("something broke".into()), 1),
        ];
        for (err, expected) in cases {
            assert_eq!(
                exit_code_for(err),
                *expected,
                "{:?} should map to {expected}",
                err
            );
        }
    }
}
