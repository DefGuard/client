use defguard_core::connection::{bring_up, ConnectionTarget};

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
    code: Option<&str>,
    code_command: Option<&str>,
    mfa_method: Option<&str>,
    _all_traffic: bool,
    _no_all_traffic: bool,
) -> Result<(), CliError> {
    // Phase 5 TODO: handle MFA (code, code_command, mfa_method)
    if code.is_some() || code_command.is_some() {
        return Err(CliError::Usage(
            "MFA not yet implemented. Use Phase 5 build.".into(),
        ));
    }

    let spec = TargetSpec {
        name: name.map(String::from),
        tunnel,
        id,
        instance: instance.map(String::from),
    };

    let target = resolve::resolve_connect_target(&spec, &state.pool).await?;

    let (target_name, psk, mtu) = match &target {
        ResolvedTarget::Location(loc) => {
            // Check if MFA is required.
            if loc.mfa_enabled() {
                return Err(CliError::MfaInputRequired(format!(
                    "Location '{}' requires MFA. Provide --code or --code-command.",
                    loc.name
                )));
            }

            // Posture check (enterprise feature, optional).
            #[cfg(feature = "posture")]
            {
                if loc.posture_check_required {
                    let psk = defguard_client_posture::authorize_posture_session(&state.pool)
                        .await
                        .map_err(|e| CliError::Other(e.to_string()))?;
                    (loc.name.clone(), psk, None)
                } else {
                    (loc.name.clone(), None, None)
                }
            }
            #[cfg(not(feature = "posture"))]
            {
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
