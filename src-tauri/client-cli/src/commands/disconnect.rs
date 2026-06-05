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
        let active = active_state(&state.pool)
            .await
            .map_err(|e| CliError::Other(format!("Failed to query active connections: {e}")))?;

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
        let active = active_state(&state.pool)
            .await
            .map_err(|e| CliError::Other(format!("Failed to query active connections: {e}")))?;

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

        tear_down(&conn_info, &state.pool)
            .await
            .map_err(|e| CliError::Other(format!("Failed to disconnect: {e}")))?;

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
