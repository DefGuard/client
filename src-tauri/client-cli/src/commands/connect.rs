use std::io::IsTerminal;

use secrecy::ExposeSecret;

use defguard_core::connection::{active_state::active_state, bring_up, ConnectionTarget};
use defguard_core::ConnectionType;

use crate::{
    mfa,
    mfa_code::CodeSource,
    output::CommandOutput,
    resolve::{self, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

#[allow(clippy::too_many_arguments)]
pub async fn handle(
    state: &State,
    name: Option<&str>,
    tunnel: bool,
    id: Option<i64>,
    instance: Option<&str>,
    code: Option<&str>,
    code_command: Option<&str>,
    mfa_method: Option<&str>,
    all_traffic: bool,
    predefined_traffic: bool,
) -> Result<ConnectResult, CliError> {
    #[cfg(target_os = "macos")]
    {
        return Err(CliError::Other(
            "VPN connection management is not yet supported on macOS from the CLI. \
             Use the desktop client."
                .into(),
        ));
    }

    // Per-call routing override: --all-traffic = true, --predefined-traffic = false,
    // neither = None (use the location/tunnel default).
    let routing_override: Option<bool> = if all_traffic {
        Some(true)
    } else if predefined_traffic {
        Some(false)
    } else {
        None
    };

    let spec = TargetSpec {
        name: name.map(String::from),
        tunnel,
        id,
        instance: instance.map(String::from),
    };

    let target = resolve::resolve_connect_target(&spec, &state.pool).await?;

    // Idempotency: if the target is already connected, report and exit 0.
    let (target_id, target_connection_type, target_name) = match &target {
        ResolvedTarget::Location(loc) => (loc.id, ConnectionType::Location, loc.name.as_str()),
        ResolvedTarget::Tunnel(tun) => (tun.id, ConnectionType::Tunnel, tun.name.as_str()),
    };
    let active = active_state(&state.pool).await?;
    if active
        .iter()
        .any(|c| c.connection_type == target_connection_type && c.target_id == target_id)
    {
        return Ok(ConnectResult::AlreadyConnected {
            name: target_name.to_string(),
        });
    }

    let (target_name, psk, mtu) = match &target {
        ResolvedTarget::Location(loc) => {
            if loc.mfa_enabled() {
                // Determine the MFA code source from CLI flags.
                let code_source = code
                    .map(|c| CodeSource::Literal(c.to_string()))
                    .or_else(|| code_command.map(|cmd| CodeSource::Command(cmd.to_string())));

                use defguard_core::database::models::instance::Instance;

                let source = if let Some(s) = code_source {
                    s
                } else if std::io::stdin().is_terminal() {
                    CodeSource::Interactive
                } else {
                    return Err(CliError::MfaInputRequired(format!(
                        "Location '{}' requires MFA but no --code, --code-command, or TTY is available.",
                        loc.name
                    )));
                };

                let inst = Instance::find_by_id(&state.pool, loc.instance_id)
                    .await
                    .map_err(|e| CliError::Other(format!("Failed to load instance: {e}")))?
                    .ok_or_else(|| {
                        CliError::Other(format!("Instance {} not found", loc.instance_id))
                    })?;

                // When posture is also required, collect posture data and pass it
                // into the MFA start request so the server can validate both together.
                let posture_data = if loc.posture_check_required {
                    Some(
                        defguard_client_posture::get_posture_data()
                            .await
                            .map_err(|e| CliError::Other(e.to_string()))?,
                    )
                } else {
                    None
                };

                let psk =
                    mfa::authorize(loc, &source, &inst, mfa_method, posture_data, &state.pool)
                        .await?;
                (
                    loc.name.clone(),
                    Some(psk.expose_secret().to_string()),
                    state.app_config.mtu(),
                )
            } else if loc.posture_check_required {
                // Posture only (no MFA).
                let psk = defguard_client_posture::authorize_posture_session(loc)
                    .await
                    .map_err(|e| CliError::Other(e.to_string()))?;
                (loc.name.clone(), Some(psk), state.app_config.mtu())
            } else {
                (loc.name.clone(), None, state.app_config.mtu())
            }
        }
        ResolvedTarget::Tunnel(tun) => (
            tun.name.clone(),
            tun.preshared_key.clone(),
            state.app_config.mtu(),
        ),
    };

    tracing::info!("Connecting to {target_name}...");
    let conn_target = match &target {
        ResolvedTarget::Location(loc) => ConnectionTarget::Location(loc),
        ResolvedTarget::Tunnel(tun) => ConnectionTarget::Tunnel(tun),
    };
    bring_up(conn_target, psk, mtu, &state.pool, routing_override).await?;

    Ok(ConnectResult::Connected { name: target_name })
}

pub enum ConnectResult {
    /// A new connection was established.
    Connected { name: String },
    /// The target was already connected (idempotent).
    AlreadyConnected { name: String },
}

impl CommandOutput for ConnectResult {
    fn human(&self) -> String {
        match self {
            ConnectResult::Connected { name } => format!("Connected to {name}"),
            ConnectResult::AlreadyConnected { name } => {
                format!("Already connected to {name}")
            }
        }
    }

    fn json(&self) -> serde_json::Value {
        match self {
            ConnectResult::Connected { name } => serde_json::json!({
                "connected": name,
            }),
            ConnectResult::AlreadyConnected { name } => serde_json::json!({
                "connected": name,
                "already": true,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connected_human() {
        let result = ConnectResult::Connected {
            name: "office".to_string(),
        };
        assert_eq!(result.human(), "Connected to office");
    }

    #[test]
    fn test_already_connected_human() {
        let result = ConnectResult::AlreadyConnected {
            name: "office".to_string(),
        };
        assert_eq!(result.human(), "Already connected to office");
    }

    #[test]
    fn test_connected_json() {
        let result = ConnectResult::Connected {
            name: "office".to_string(),
        };
        let json = result.json();
        assert_eq!(json["connected"], "office");
        assert!(json["already"].is_null());
    }

    #[test]
    fn test_already_connected_json() {
        let result = ConnectResult::AlreadyConnected {
            name: "office".to_string(),
        };
        let json = result.json();
        assert_eq!(json["connected"], "office");
        assert_eq!(json["already"], true);
    }

    #[test]
    fn test_json_no_message_field() {
        let result = ConnectResult::Connected {
            name: "office".to_string(),
        };
        let json = result.json();
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_exit_code_zero() {
        let result = ConnectResult::Connected {
            name: "office".to_string(),
        };
        assert_eq!(result.exit_code(), 0);
    }
}
