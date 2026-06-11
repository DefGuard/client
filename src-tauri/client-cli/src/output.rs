use std::process::ExitCode;

use serde::Serialize;

use crate::{exit, state::CliError};

/// Typed command output that owns both human and JSON representations.
pub trait CommandOutput {
    /// Produce a human-readable string (no trailing newline required).
    fn human(&self) -> String;
    /// Produce a structured JSON value.
    fn json(&self) -> serde_json::Value;
    /// Exit code override; defaults to 0 (success).
    fn exit_code(&self) -> u8 {
        0
    }
}

/// Render a `CommandOutput` value as either JSON or human-readable output.
pub fn emit<T: CommandOutput>(value: &T, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value.json())
                .unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
        );
    } else {
        println!("{}", value.human());
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
        eprintln!(
            "{}",
            serde_json::to_string(&je).unwrap_or_else(|e| format!("{{error: \"{e}\"}}"))
        );
    } else {
        eprintln!("Error: {err}");
    }
}

#[derive(Serialize)]
pub struct InstanceEntry {
    pub name: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct LocationEntry {
    pub name: String,
    pub instance: Option<String>,
    pub address: String,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_all_traffic: Option<bool>,
}

#[derive(Serialize)]
pub struct TunnelEntry {
    pub name: String,
    pub address: String,
    pub endpoint: String,
}

#[derive(Serialize)]
pub struct ActiveEntry {
    pub connection_type: String,
    pub name: String,
    pub interface: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_handshake_secs: Option<u64>,
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

/// Finalize a `CommandOutput` result, emit output, and return the exit code.
pub fn finish<T: CommandOutput>(result: Result<T, CliError>, json: bool) -> ExitCode {
    match result {
        Ok(output) => {
            let code = output.exit_code();
            emit(&output, json);
            ExitCode::from(code)
        }
        Err(err) => {
            let code = exit::exit_code_for(&err);
            emit_error(&err, json);
            ExitCode::from(code)
        }
    }
}
