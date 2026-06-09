use defguard_core::connection::{active_state::active_state, bring_up, ConnectionTarget};

use crate::{
    mfa,
    mfa_code::CodeSource,
    output,
    resolve::{self, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

#[allow(clippy::too_many_arguments)]
pub async fn handle(
    state: &State,
    json: bool,
    name: Option<&str>,
    tunnel: bool,
    id: Option<i64>,
    instance: Option<&str>,
    code: Option<&str>,
    code_command: Option<&str>,
    mfa_method: Option<&str>,
    all_traffic: bool,
    no_all_traffic: bool,
) -> Result<(), CliError> {
    // Per-call routing overrides are not yet supported.
    if all_traffic || no_all_traffic {
        return Err(CliError::Usage(
            "--all-traffic / --no-all-traffic per-call override is not yet supported.".into(),
        ));
    }

    let spec = TargetSpec {
        name: name.map(String::from),
        tunnel,
        id,
        instance: instance.map(String::from),
    };

    let target = resolve::resolve_connect_target(&spec, &state.pool).await?;

    // Idempotency: if the target is already connected, report and exit 0.
    let (target_id, target_name) = match &target {
        ResolvedTarget::Location(loc) => (loc.id, loc.name.as_str()),
        ResolvedTarget::Tunnel(tun) => (tun.id, tun.name.as_str()),
    };
    let active = active_state(&state.pool)
        .await
        .map_err(|e| CliError::Other(format!("Failed to query active connections: {e}")))?;
    if active.iter().any(|c| c.target_id == target_id) {
        if json {
            output::emit(
                &serde_json::json!({ "connected": target_name, "already": true }),
                json,
            );
        } else {
            println!("Already connected to {target_name}");
        }
        return Ok(());
    }

    let (target_name, psk, mtu) = match &target {
        ResolvedTarget::Location(loc) => {
            // Determine the MFA code source from CLI flags.
            let code_source = code
                .map(|c| CodeSource::Literal(c.to_string()))
                .or_else(|| code_command.map(|cmd| CodeSource::Command(cmd.to_string())));

            if loc.mfa_enabled() {
                use defguard_core::database::{models::instance::Instance, DB_POOL};

                let source = if let Some(s) = code_source {
                    s
                } else if atty::is(atty::Stream::Stdin) {
                    CodeSource::Interactive
                } else {
                    return Err(CliError::MfaInputRequired(format!(
                        "Location '{}' requires MFA but no --code, --code-command, or TTY is available.",
                        loc.name
                    )));
                };

                let inst = Instance::find_by_id(&*DB_POOL, loc.instance_id)
                    .await
                    .map_err(|e| CliError::Other(format!("Failed to load instance: {e}")))?
                    .ok_or_else(|| {
                        CliError::Other(format!("Instance {} not found", loc.instance_id))
                    })?;

                let psk = mfa::authorize(loc, &source, &inst, mfa_method).await?;
                (loc.name.clone(), Some(psk), None)
            } else if loc.posture_check_required {
                // Posture only (no MFA).
                let psk = defguard_client_posture::authorize_posture_session(loc)
                    .await
                    .map_err(|e| CliError::Other(e.to_string()))?;
                (loc.name.clone(), Some(psk), None)
            } else {
                (loc.name.clone(), None, None)
            }
        }
        ResolvedTarget::Tunnel(tun) => (tun.name.clone(), tun.preshared_key.clone(), None),
    };

    tracing::info!("Connecting to {target_name}...");
    let conn_target = match &target {
        ResolvedTarget::Location(loc) => ConnectionTarget::Location(loc),
        ResolvedTarget::Tunnel(tun) => ConnectionTarget::Tunnel(tun),
    };
    bring_up(conn_target, psk, mtu, &state.pool)
        .await
        .map_err(|e| CliError::Other(format!("Failed to connect: {e}")))?;

    if json {
        output::emit(&serde_json::json!({ "connected": target_name }), json);
    } else {
        println!("Connected to {target_name}");
    }

    Ok(())
}
