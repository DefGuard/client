use serde::Serialize;

use crate::state::CliError;

/// Render a typed result as either a human-readable table or JSON.
pub fn emit<T: Serialize + std::fmt::Debug>(value: &T, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{error: \"{e}\"}}"))
        );
    } else {
        // For now, use debug formatting; later phases introduce table rendering.
        println!("{value:#?}");
    }
}

/// Render an error.  Under `--json`, prints a `{ "kind", "message" }` object.
pub fn emit_error(err: &CliError, json: bool) {
    if json {
        #[derive(Serialize)]
        struct JsonError {
            kind: String,
            message: String,
        }
        let je = JsonError {
            kind: error_kind(err),
            message: err.to_string(),
        };
        println!(
            "{}",
            serde_json::to_string(&je).unwrap_or_else(|e| format!("{{error: \"{e}\"}}"))
        );
    } else {
        eprintln!("Error: {err}");
    }
}

fn error_kind(err: &CliError) -> String {
    match err {
        CliError::Usage(_) => "usage".into(),
        CliError::NotFound(_) => "notFound".into(),
        CliError::DaemonUnavailable(_) => "unavailable".into(),
        CliError::MfaFailed(_) => "mfaFailed".into(),
        CliError::MfaInputRequired(_) => "mfaInputRequired".into(),
        CliError::NotEnrolled(_) => "notEnrolled".into(),
        CliError::Database(_) => "database".into(),
        CliError::Other(_) => "other".into(),
    }
}
