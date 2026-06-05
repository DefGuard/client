use defguard_core::connection::{active_state::active_state, tear_down};

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
        // TODO: Phase 4.3 - disconnect all active connections via active_state
        return Err(CliError::Usage("--all not yet implemented".into()));
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
    let active = active_state(&state.pool)
        .await
        .map_err(|e| CliError::Other(format!("Failed to query active connections: {e}")))?;

    let connection = active
        .iter()
        .find(|c| c.target_id == target_id)
        .ok_or_else(|| CliError::NotFound(format!("'{target_name}' is not currently connected")))?;

    let ifname = connection.interface_name.clone();
    let endpoint = match &target {
        ResolvedTarget::Location(loc) => loc.endpoint.clone(),
        ResolvedTarget::Tunnel(tun) => tun.endpoint.clone(),
    };

    tracing::info!("Disconnecting {target_name} on interface {ifname}...");
    tear_down(&ifname, &endpoint)
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
