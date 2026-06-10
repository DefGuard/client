use defguard_core::connection::{active_state::active_state, tear_down};
use defguard_core::ConnectionType;

use crate::{
    output,
    resolve::{self, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

pub async fn handle(
    state: &State,
    json: bool,
    name: Option<&str>,
    tunnel: bool,
    id: Option<i64>,
    instance: Option<&str>,
    all: bool,
) -> Result<(), CliError> {
    if all {
        // Disconnect all currently-active connections.
        let active = active_state(&state.pool).await?;

        if active.is_empty() {
            if json {
                output::emit(&serde_json::json!({ "disconnected": [] }), json);
            } else {
                println!("No active connections.");
            }
            return Ok(());
        }

        let mut disconnected = Vec::with_capacity(active.len());
        let mut errors = Vec::new();

        for conn in &active {
            let name = conn.name.clone();
            tracing::info!(
                "Disconnecting {name} on interface {}...",
                conn.interface_name
            );
            match tear_down(conn, &state.pool).await {
                Ok(()) => {
                    tracing::info!("Disconnected {name} ({})", conn.interface_name);
                    disconnected.push(name);
                }
                Err(e) => {
                    let msg = format!("Failed to disconnect {name}: {e}");
                    tracing::error!("{msg}");
                    errors.push(msg);
                }
            }
        }

        if json {
            output::emit(
                &serde_json::json!({ "disconnected": disconnected, "errors": errors }),
                json,
            );
        } else {
            for name in &disconnected {
                println!("Disconnected from {name}");
            }
            for err in &errors {
                eprintln!("Error: {err}");
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(CliError::Other(errors.join("; ")))
        }
    } else {
        // No-arg disconnect: if exactly one connection is active, disconnect it.
        if name.is_none() && !tunnel && id.is_none() && instance.is_none() {
            let active = active_state(&state.pool).await?;

            match active.len() {
                0 => {
                    if json {
                        output::emit(
                            &serde_json::json!({ "disconnected": null, "message": "no active connections" }),
                            json,
                        );
                    } else {
                        println!("No active connections.");
                    }
                    return Ok(());
                }
                1 => {
                    let conn = &active[0];
                    let ifname = conn.interface_name.clone();
                    let name = conn.name.clone();
                    tracing::info!(
                        "Disconnecting sole active connection {name} on interface {ifname}..."
                    );
                    tear_down(conn, &state.pool).await?;
                    if json {
                        output::emit(
                            &serde_json::json!({ "disconnected": name, "interface": ifname }),
                            json,
                        );
                    } else {
                        println!("Disconnected from {name} ({ifname})");
                    }
                    return Ok(());
                }
                _ => {
                    let names: Vec<_> = active.iter().map(|c| c.name.as_str()).collect();
                    return Err(CliError::Usage(format!(
                        "Multiple active connections ({}). Specify which to disconnect, --all to disconnect all.",
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

        let target = resolve::resolve_disconnect_target(&spec, &state.pool).await?;

        let (target_id, target_name) = match &target {
            ResolvedTarget::Location(loc) => (loc.id, loc.name.clone()),
            ResolvedTarget::Tunnel(tun) => (tun.id, tun.name.clone()),
        };

        // Look up the actual interface name from active_state.
        let active = active_state(&state.pool).await?;

        let connection = active
            .iter()
            .find(|c| c.target_id == target_id)
            .ok_or_else(|| {
                CliError::NotFound(format!("'{target_name}' is not currently connected"))
            })?;

        let ifname = connection.interface_name.clone();

        tracing::info!("Disconnecting {target_name} on interface {ifname}...");

        let conn_info = defguard_core::connection::active_state::ActiveConnectionInfo {
            connection_type: match &target {
                ResolvedTarget::Location(_) => ConnectionType::Location,
                ResolvedTarget::Tunnel(_) => ConnectionType::Tunnel,
            },
            target_id,
            name: target_name.clone(),
            interface_name: ifname.clone(),
            stats: None,
        };

        tear_down(&conn_info, &state.pool).await?;

        if json {
            output::emit(
                &serde_json::json!({ "disconnected": target_name, "interface": ifname }),
                json,
            );
        } else {
            println!("Disconnected from {target_name} ({ifname})");
        }

        Ok(())
    }
}
