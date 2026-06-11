use defguard_core::connection::{active_state::active_state, tear_down};

use crate::{
    output::{self, DisconnectOutput, DisconnectedResult},
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
            output::emit(
                &DisconnectOutput {
                    disconnected: Some(DisconnectedResult::List(vec![])),
                    interface: None,
                    errors: vec![],
                    message: Some("No active connections.".into()),
                },
                json,
            );
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

        output::emit(
            &DisconnectOutput {
                disconnected: Some(DisconnectedResult::List(disconnected)),
                interface: None,
                errors,
                message: None,
            },
            json,
        );

        // Always Ok: the emitted JSON already contains per-item success/error
        // details. Returning Err would cause a second JSON error doc on stdout
        // when --json is active, breaking single-document consumers.
        Ok(())
    } else {
        // No-arg disconnect: if exactly one connection is active, disconnect it.
        if name.is_none() && !tunnel && id.is_none() && instance.is_none() {
            let active = active_state(&state.pool).await?;

            match active.len() {
                0 => {
                    output::emit(
                        &DisconnectOutput {
                            disconnected: None,
                            interface: None,
                            errors: vec![],
                            message: Some("No active connections.".into()),
                        },
                        json,
                    );
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
                    output::emit(
                        &DisconnectOutput {
                            disconnected: Some(DisconnectedResult::Single(name.clone())),
                            interface: Some(ifname.clone()),
                            errors: vec![],
                            message: Some(format!("Disconnected from {name} ({ifname})")),
                        },
                        json,
                    );
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

        tear_down(connection, &state.pool).await?;

        output::emit(
            &DisconnectOutput {
                disconnected: Some(DisconnectedResult::Single(target_name.clone())),
                interface: Some(ifname.clone()),
                errors: vec![],
                message: Some(format!("Disconnected from {target_name} ({ifname})")),
            },
            json,
        );

        Ok(())
    }
}
