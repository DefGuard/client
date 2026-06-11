use std::process::ExitCode;

use serde::Serialize;

use crate::{exit, state::CliError};

/// Types that can render themselves for human-readable terminal output.
pub trait HumanRender {
    /// Produce a human-readable string (no trailing newline required).
    fn render(&self) -> String;
}

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

/// Render a typed result as either JSON or human-readable output.
pub fn emit<T: Serialize + HumanRender>(value: &T, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{error: \"{e}\"}}"))
        );
    } else {
        println!("{}", value.render());
    }
}

/// Render a `CommandOutput` value as either JSON or human-readable output.
pub fn emit_typed<T: CommandOutput>(value: &T, json: bool) {
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

/// Output for the `disconnect` command.
#[derive(Serialize)]
pub struct DisconnectOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disconnected: Option<DisconnectedResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl HumanRender for DisconnectOutput {
    fn render(&self) -> String {
        if let Some(ref msg) = self.message {
            return msg.clone();
        }
        // Fallback for --all without message.
        let mut parts = Vec::new();
        match &self.disconnected {
            Some(DisconnectedResult::Single(name)) => {
                parts.push(format!("disconnected: {name}"));
            }
            Some(DisconnectedResult::List(names)) => {
                parts.push(format!("disconnected: {}", names.join(", ")));
            }
            None => {}
        }
        if !self.errors.is_empty() {
            parts.push(format!("errors: {}", self.errors.join(", ")));
        }
        parts.join("\n")
    }
}

/// The `disconnected` field value - either a single name, a list, or absent.
#[derive(Serialize)]
#[serde(untagged)]
pub enum DisconnectedResult {
    Single(String),
    List(Vec<String>),
}

/// Output for the `location list` command.
#[derive(Serialize)]
pub struct LocationListOutput {
    pub locations: Vec<LocationEntry>,
    pub message: String,
}

impl HumanRender for LocationListOutput {
    fn render(&self) -> String {
        self.message.clone()
    }
}

/// Output for the `location set` command.
#[derive(Serialize)]
pub struct LocationSetOutput {
    pub location: String,
    pub changes: Vec<String>,
    pub message: String,
}

impl HumanRender for LocationSetOutput {
    fn render(&self) -> String {
        self.message.clone()
    }
}

/// Output for the `location show` command.
#[derive(Serialize)]
pub struct LocationShowOutput {
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub pubkey: String,
    pub allowed_ips: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<String>,
    pub mfa_method: String,
    pub route_all_traffic: bool,
    pub keepalive_interval: i64,
    pub message: String,
}

impl HumanRender for LocationShowOutput {
    fn render(&self) -> String {
        self.message.clone()
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
            emit_typed(&output, json);
            ExitCode::from(code)
        }
        Err(err) => {
            let code = exit::exit_code_for(&err);
            emit_error(&err, json);
            ExitCode::from(code)
        }
    }
}

/// Finalize a legacy `Result<(), CliError>`, emit errors if needed, and return
/// the exit code.  Used for commands that have not yet been migrated to
/// `CommandOutput`.
pub fn finish_legacy(result: Result<(), CliError>, json: bool) -> ExitCode {
    match result {
        Ok(()) => ExitCode::from(0),
        Err(err) => {
            let code = exit::exit_code_for(&err);
            emit_error(&err, json);
            ExitCode::from(code)
        }
    }
}
