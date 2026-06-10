//! Resolve the MFA proof across three input sources.
//!
//! Priority order: `--code` > `--code-command` > interactive TTY prompt.
//! Non-TTY + no code or command → `MfaInputRequired` error.
//!
//! The returned value is wrapped in [`secrecy::SecretString`] so it never
//! appears in logs, debug output, or error messages.

use std::{
    fmt,
    io::{self, IsTerminal, Write},
    process::Command,
};

use secrecy::SecretString;

use crate::state::CliError;

/// Describes where to source the MFA code from.
///
/// Manual [`Debug`] impl redacts the `Literal` variant so `--code <value>`
/// never leaks into logs or error output.
#[derive(Clone)]
pub enum CodeSource {
    /// Literal value from `--code <6-digit>`.
    Literal(String),
    /// Shell command whose stdout yields the code (`--code-command`).
    Command(String),
    /// Read interactively from the terminal.
    Interactive,
}

impl fmt::Debug for CodeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(_) => f.debug_tuple("Literal").field(&"<redacted>").finish(),
            Self::Command(cmd) => f.debug_tuple("Command").field(cmd).finish(),
            Self::Interactive => f.debug_tuple("Interactive").finish(),
        }
    }
}

/// Context passed to `--code-command` via environment variables.
pub struct MfaContext {
    /// `DG_INSTANCE` — the instance name.
    pub instance: String,
    /// `DG_LOCATION` — the location name.
    pub location: String,
}

/// Obtain a TOTP/email code from the configured source.
///
/// The returned [`SecretString`] prevents accidental exposure through
/// [`Debug`], logs, or error messages.  Callers that need the raw value
/// must explicitly call `.expose_secret()`.
pub fn obtain_code(source: &CodeSource, ctx: &MfaContext) -> Result<SecretString, CliError> {
    match source {
        CodeSource::Literal(code) => {
            tracing::debug!("Using --code value");
            Ok(SecretString::from(code.trim()))
        }
        CodeSource::Command(cmd) => {
            tracing::debug!("Running --code-command");
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
            Ok(SecretString::from(code.as_str()))
        }
        CodeSource::Interactive => {
            if !std::io::stdin().is_terminal() {
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

            Ok(SecretString::from(code.trim()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    fn ctx() -> MfaContext {
        MfaContext {
            instance: "test-inst".into(),
            location: "test-loc".into(),
        }
    }

    #[test]
    fn test_literal_code_returns_trimmed_secret() {
        let source = CodeSource::Literal("  123456  ".into());
        let secret = obtain_code(&source, &ctx()).unwrap();
        assert_eq!(secret.expose_secret(), "123456");
    }

    #[test]
    fn test_command_produces_stdout() {
        let source = CodeSource::Command("echo -n 654321".into());
        let secret = obtain_code(&source, &ctx()).unwrap();
        assert_eq!(secret.expose_secret(), "654321");
    }

    #[test]
    fn test_command_failure_is_mfa_failed() {
        let source = CodeSource::Command("exit 2".into());
        let err = obtain_code(&source, &ctx()).unwrap_err();
        assert!(matches!(err, CliError::MfaFailed(_)));
        assert!(err.to_string().contains("exited"));
    }

    #[test]
    fn test_command_empty_output_is_mfa_failed() {
        let source = CodeSource::Command("true".into()); // produces no stdout
        let err = obtain_code(&source, &ctx()).unwrap_err();
        assert!(matches!(err, CliError::MfaFailed(_)));
        assert!(err.to_string().contains("no output"));
    }

    #[test]
    fn test_command_receives_env_vars() {
        // Print the env vars to stdout so we can assert they're set.
        let source = CodeSource::Command("echo -n $DG_INSTANCE/$DG_LOCATION".into());
        let secret = obtain_code(&source, &ctx()).unwrap();
        assert_eq!(secret.expose_secret(), "test-inst/test-loc");
    }

    #[test]
    fn test_literal_is_redacted_in_debug() {
        let source = CodeSource::Literal("secret123".into());
        let debug = format!("{source:?}");
        assert!(!debug.contains("secret123"));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn test_command_is_not_redacted_in_debug() {
        let source = CodeSource::Command("echo code".into());
        let debug = format!("{source:?}");
        assert!(debug.contains("echo code"));
    }

    #[test]
    fn test_interactive_shows_in_debug() {
        let source = CodeSource::Interactive;
        let debug = format!("{source:?}");
        assert!(debug.contains("Interactive"));
    }
}
