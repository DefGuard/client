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
pub fn exit_code_for(err: &CliError) -> u8 {
    match err {
        CliError::Usage(_) => 2,
        CliError::NotFound(_) => 3,
        CliError::Unavailable(_) => 4,
        CliError::MfaFailed(_) | CliError::MfaInputRequired(_) => 5,
        CliError::NotEnrolled(_) => 6,
        CliError::Database(_) => 1,
        CliError::Other(_) => 1,
    }
}
