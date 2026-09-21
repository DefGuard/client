//! Resolve the MFA proof across three input sources.
//!
//! Priority order: `--code` > `--code-command` > interactive TTY prompt.
//! Non-TTY + no code or command → `MfaInputRequired` error.
//!
//! The returned value is wrapped in [`secrecy::SecretString`] so it never
//! appears in logs, debug output, or error messages.

use std::{
    fmt,
    io::{stderr, stdin, IsTerminal, Write},
    process::Command,
};

use defguard_core::database::models::location::LocationMfaMethod;
use secrecy::SecretString;
use tracing::debug;

use crate::{
    mfa::{opt_step_badge, step_method_label},
    state::CliError,
};

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
    /// `DG_INSTANCE` - the instance name.
    pub instance: String,
    /// `DG_LOCATION` - the location name.
    pub location: String,
    /// Present when the code belongs to one step of a multi-step location.
    pub step: Option<MfaStepContext>,
}

/// Additional context displayed to the user when entering an MFA code.
#[derive(Clone, Copy)]
pub struct MfaStepContext {
    /// Zero-based step index.
    pub index: usize,
    /// Total number of steps.
    pub total: usize,
    /// The method this step verifies.
    pub method: LocationMfaMethod,
}

/// Interactive prompt text.
fn code_prompt(ctx: &MfaContext) -> String {
    match &ctx.step {
        Some(step) => format!(
            "{}Enter the {} code for '{}': ",
            opt_step_badge(Some(step)),
            step_method_label(step.method),
            ctx.location
        ),
        None => format!("Enter MFA code for {}: ", ctx.location),
    }
}

/// Obtain a TOTP/email code from the configured source.
pub fn obtain_code(source: &CodeSource, ctx: &MfaContext) -> Result<SecretString, CliError> {
    match source {
        CodeSource::Literal(code) => {
            debug!("Using --code value");
            Ok(SecretString::from(code.trim()))
        }
        CodeSource::Command(cmd) => {
            debug!("Running --code-command");
            let mut command = Command::new("sh");
            command
                .arg("-c")
                .arg(cmd)
                .env("DG_INSTANCE", &ctx.instance)
                .env("DG_LOCATION", &ctx.location);
            // Add step context for multi-step code commands.
            if let Some(step) = &ctx.step {
                command
                    .env("DG_MFA_STEP", (step.index + 1).to_string())
                    .env("DG_MFA_STEP_COUNT", step.total.to_string())
                    .env("DG_MFA_METHOD", step.method.as_str());
            }
            let output = command
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
            if !stdin().is_terminal() {
                return Err(CliError::MfaInputRequired(
                    "No TTY available for interactive MFA code entry. Provide --code or --code-command."
                        .into(),
                ));
            }

            // N.B. stderr - stdout is reserved for data.
            eprint!("{}", code_prompt(ctx));
            stderr().flush().ok();

            let mut code = String::new();
            stdin()
                .read_line(&mut code)
                .map_err(|e| CliError::MfaFailed(format!("Failed to read code: {e}")))?;

            Ok(SecretString::from(code.trim()))
        }
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;

    fn ctx() -> MfaContext {
        MfaContext {
            instance: "test-inst".into(),
            location: "test-loc".into(),
            step: None,
        }
    }

    #[test]
    fn test_code_prompt_legacy_without_step() {
        assert_eq!(code_prompt(&ctx()), "Enter MFA code for test-loc: ");
    }

    #[test]
    fn test_code_prompt_step_aware() {
        let mut ctx = ctx();
        ctx.step = Some(MfaStepContext {
            index: 1,
            total: 2,
            method: LocationMfaMethod::Totp,
        });
        assert_eq!(
            code_prompt(&ctx),
            "[2/2] Enter the Authenticator app code for 'test-loc': "
        );
    }

    #[test]
    fn test_literal_code_returns_trimmed_secret() {
        let source = CodeSource::Literal("  123456  ".into());
        let secret = obtain_code(&source, &ctx()).unwrap();
        assert_eq!(secret.expose_secret(), "123456");
    }

    #[test]
    #[ignore = "`echo -n` is not portable"]
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
    fn test_command_receives_step_env_vars() {
        let mut ctx = ctx();
        ctx.step = Some(MfaStepContext {
            index: 0,
            total: 2,
            method: LocationMfaMethod::Email,
        });
        let source = CodeSource::Command(
            r#"printf '%s/%s/%s/%s/%s' "$DG_INSTANCE" "$DG_LOCATION" "$DG_MFA_STEP" "$DG_MFA_STEP_COUNT" "$DG_MFA_METHOD""#.into(),
        );
        let secret = obtain_code(&source, &ctx).unwrap();
        assert_eq!(secret.expose_secret(), "test-inst/test-loc/1/2/email");
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
