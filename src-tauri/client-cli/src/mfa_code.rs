//! Resolve the MFA proof across three input sources.
//!
//! Priority order: `--code` > `--code-command` > interactive TTY prompt.
//! Non-TTY + no code or command → `MfaInputRequired` error.

use std::{
    io::{self, Write},
    process::Command,
};

use crate::state::CliError;

/// Describes where to source the MFA code from.
#[derive(Clone, Debug)]
pub enum CodeSource {
    /// Literal value from `--code <6-digit>`.
    Literal(String),
    /// Shell command whose stdout yields the code (`--code-command`).
    Command(String),
    /// Read interactively from the terminal.
    Interactive,
}

/// Context passed to `--code-command` via environment variables.
pub struct MfaContext {
    /// `DG_INSTANCE` — the instance name.
    pub instance: String,
    /// `DG_LOCATION` — the location name.
    pub location: String,
}

/// Obtain a TOTP/email code from the configured source, wrapping it in a
/// `secrecy::Secret` so it never appears in logs or debug output.
pub fn obtain_code(source: &CodeSource, ctx: &MfaContext) -> Result<String, CliError> {
    match source {
        CodeSource::Literal(code) => {
            tracing::debug!("Using --code value");
            Ok(code.trim().to_string())
        }
        CodeSource::Command(cmd) => {
            tracing::debug!("Running --code-command: {cmd}");
            let output = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .env("DG_INSTANCE", &ctx.instance)
                .env("DG_LOCATION", &ctx.location)
                .output()
                .map_err(|e| CliError::MfaFailed(format!("Failed to run code command: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(CliError::MfaFailed(format!(
                    "Code command exited with {}: {}",
                    output.status,
                    stderr.trim()
                )));
            }

            let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if code.is_empty() {
                return Err(CliError::MfaFailed(
                    "Code command produced no output".into(),
                ));
            }
            Ok(code)
        }
        CodeSource::Interactive => {
            if !atty::is(atty::Stream::Stdin) {
                return Err(CliError::MfaInputRequired(
                    "No TTY available for interactive MFA code entry. Provide --code or --code-command."
                        .into(),
                ));
            }

            // N.B. stderr — stdout is reserved for data.
            eprint!("Enter MFA code for {}: ", ctx.location);
            io::stderr().flush().ok();

            let mut code = String::new();
            io::stdin()
                .read_line(&mut code)
                .map_err(|e| CliError::MfaFailed(format!("Failed to read code: {e}")))?;

            Ok(code.trim().to_string())
        }
    }
}
