use defguard_core::{
    connection::{active_state::active_state, tear_down},
    ConnectionType,
};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::{
    output::CommandOutput,
    resolve::{resolve_disconnect_target, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

pub async fn handle(
    state: &State,
    name: Option<&str>,
    tunnel: bool,
    id: Option<i64>,
    instance: Option<&str>,
    all: bool,
) -> Result<DisconnectResult, CliError> {
    if all {
        // Disconnect all currently-active connections.
        let active = active_state(&state.pool).await?;

        if active.is_empty() {
            return Ok(DisconnectResult::NoneActive);
        }

        let mut disconnected = Vec::with_capacity(active.len());
        let mut errors = Vec::new();

        for connection in &active {
            let name = connection.name.clone();
            info!(
                "Disconnecting {name} on interface {}...",
                connection.interface_name
            );
            match tear_down(connection).await {
                Ok(()) => {
                    info!("Disconnected {name} ({})", connection.interface_name);
                    disconnected.push(name);
                }
                Err(e) => {
                    let msg = format!("Failed to disconnect {name}: {e}");
                    error!("{msg}");
                    errors.push(msg);
                }
            }
        }

        Ok(DisconnectResult::All {
            disconnected,
            errors,
        })
    } else {
        // No-arg disconnect: if exactly one connection is active, disconnect it.
        if name.is_none() && !tunnel && id.is_none() && instance.is_none() {
            let active = active_state(&state.pool).await?;

            match active.len() {
                0 => {
                    return Ok(DisconnectResult::NoneActive);
                }
                1 => {
                    let connection = &active[0];
                    let ifname = connection.interface_name.clone();
                    let name = connection.name.clone();
                    info!("Disconnecting sole active connection {name} on interface {ifname}...");

                    #[cfg(not(target_os = "macos"))]
                    tear_down(connection).await?;

                    #[cfg(target_os = "macos")]
                    {
                        use std::sync::{
                            atomic::{AtomicBool, Ordering},
                            Arc,
                        };

                        use defguard_core::connection::apple::spawn_runloop_and_wait_for;

                        let semaphore = Arc::new(AtomicBool::new(false));
                        let semaphore_clone = Arc::clone(&semaphore);
                        let connection_clone = connection.clone();
                        let handle = tokio::spawn(async move {
                            let result = tear_down(&connection_clone).await;
                            semaphore_clone.store(true, Ordering::Release);
                            result
                        });
                        spawn_runloop_and_wait_for(&semaphore);
                        handle.await.unwrap()?;
                    }

                    return Ok(DisconnectResult::Single {
                        name,
                        interface: ifname,
                    });
                }
                _ => {
                    let names = active.iter().map(|c| c.name.as_str()).collect::<Vec<_>>();
                    return Err(CliError::Usage(format!(
                        "Multiple active connections ({}). Specify which to disconnect, --all to \
                        disconnect all.",
                        names.join(", ")
                    )));
                }
            }
        }

        let spec = TargetSpec {
            name: name.map(String::from),
            tunnel,
            id,
            instance: instance.map(String::from),
        };

        let target = resolve_disconnect_target(&spec, &state.pool).await?;

        let (target_id, target_connection_type, target_name) = match &target {
            ResolvedTarget::Location(loc) => (loc.id, ConnectionType::Location, loc.name.clone()),
            ResolvedTarget::Tunnel(tun) => (tun.id, ConnectionType::Tunnel, tun.name.clone()),
        };

        // Look up the actual interface name from active_state.
        let active = active_state(&state.pool).await?;

        let connection = active
            .iter()
            .find(|c| c.connection_type == target_connection_type && c.target_id == target_id)
            .ok_or_else(|| {
                CliError::NotFound(format!("'{target_name}' is not currently connected"))
            })?;

        let ifname = connection.interface_name.clone();

        info!("Disconnecting {target_name} on interface {ifname}...");

        tear_down(connection).await?;

        Ok(DisconnectResult::Single {
            name: target_name,
            interface: ifname,
        })
    }
}

pub enum DisconnectResult {
    Single {
        name: String,
        interface: String,
    },
    All {
        disconnected: Vec<String>,
        errors: Vec<String>,
    },
    NoneActive,
}

impl CommandOutput for DisconnectResult {
    fn human(&self) -> String {
        match self {
            DisconnectResult::Single { name, interface } => {
                format!("Disconnected from {name} ({interface})")
            }
            DisconnectResult::All {
                disconnected,
                errors,
            } => {
                let mut parts = Vec::new();
                if !disconnected.is_empty() {
                    parts.push(format!("disconnected: {}", disconnected.join(", ")));
                }
                if !errors.is_empty() {
                    parts.push(format!("errors: {}", errors.join(", ")));
                }
                if parts.is_empty() {
                    "No active connections.".to_string()
                } else {
                    parts.join("\n")
                }
            }
            DisconnectResult::NoneActive => "No active connections.".to_string(),
        }
    }

    fn json(&self) -> Value {
        match self {
            DisconnectResult::Single { name, interface } => json!({
                "disconnected": name,
                "interface": interface,
            }),
            DisconnectResult::All {
                disconnected,
                errors,
            } => json!({
                "disconnected": disconnected,
                "errors": errors,
            }),
            DisconnectResult::NoneActive => json!({}),
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            DisconnectResult::All { errors, .. } if !errors.is_empty() => 1,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_human() {
        let result = DisconnectResult::Single {
            name: "office".to_string(),
            interface: "wg0".to_string(),
        };
        assert_eq!(result.human(), "Disconnected from office (wg0)");
    }

    #[test]
    fn test_none_active_human() {
        let result = DisconnectResult::NoneActive;
        assert_eq!(result.human(), "No active connections.");
    }

    #[test]
    fn test_all_success_human() {
        let result = DisconnectResult::All {
            disconnected: vec!["office".to_string(), "home".to_string()],
            errors: Vec::new(),
        };
        assert_eq!(result.human(), "disconnected: office, home");
    }

    #[test]
    fn test_all_with_errors_human() {
        let result = DisconnectResult::All {
            disconnected: vec!["office".to_string()],
            errors: vec!["Failed to disconnect home: timeout".to_string()],
        };
        let s = result.human();
        assert!(s.contains("disconnected: office"));
        assert!(s.contains("errors: Failed to disconnect home: timeout"));
    }

    #[test]
    fn test_single_json() {
        let result = DisconnectResult::Single {
            name: "office".to_string(),
            interface: "wg0".to_string(),
        };
        let json = result.json();
        assert_eq!(json["disconnected"], "office");
        assert_eq!(json["interface"], "wg0");
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_all_json() {
        let result = DisconnectResult::All {
            disconnected: vec!["office".to_string()],
            errors: vec!["err".to_string()],
        };
        let json = result.json();
        assert_eq!(json["disconnected"].as_array().unwrap().len(), 1);
        assert_eq!(json["errors"].as_array().unwrap().len(), 1);
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_none_active_json() {
        let result = DisconnectResult::NoneActive;
        let json = result.json();
        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn test_exit_code_zero_on_success() {
        assert_eq!(
            DisconnectResult::Single {
                name: "x".to_string(),
                interface: "y".to_string(),
            }
            .exit_code(),
            0
        );
        assert_eq!(
            DisconnectResult::All {
                disconnected: vec!["x".to_string()],
                errors: Vec::new(),
            }
            .exit_code(),
            0
        );
        assert_eq!(DisconnectResult::NoneActive.exit_code(), 0);
    }

    #[test]
    fn test_exit_code_one_on_partial_failure() {
        assert_eq!(
            DisconnectResult::All {
                disconnected: vec!["x".to_string()],
                errors: vec!["e".to_string()],
            }
            .exit_code(),
            1
        );
    }
}
