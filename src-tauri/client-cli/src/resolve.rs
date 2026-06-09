use defguard_core::database::{
    models::{instance::Instance, location::Location, tunnel::Tunnel, Id},
    DbPool,
};

use crate::state::CliError;

/// The user's target specification, parsed from CLI arguments.
pub struct TargetSpec {
    pub name: Option<String>,
    pub tunnel: bool,
    pub id: Option<Id>,
    pub instance: Option<String>,
}

/// A resolved connection target.
pub enum ResolvedTarget {
    Location(Location<Id>),
    Tunnel(Tunnel<Id>),
}

/// Resolve a target for the `connect` command.
pub async fn resolve_connect_target(
    spec: &TargetSpec,
    pool: &DbPool,
) -> Result<ResolvedTarget, CliError> {
    // --id fast path
    if let Some(id) = spec.id {
        if let Some(loc) = Location::find_by_id(pool, id).await? {
            if spec.tunnel {
                return Err(CliError::Usage(
                    "Cannot use --tunnel with --id pointing to a location".into(),
                ));
            }
            return Ok(ResolvedTarget::Location(loc));
        }
        if let Some(tun) = Tunnel::find_by_id(pool, id).await? {
            return Ok(ResolvedTarget::Tunnel(tun));
        }
        return Err(CliError::NotFound(format!(
            "No location or tunnel with id {id}"
        )));
    }

    // Named target
    if let Some(ref name) = spec.name {
        return resolve_named(name, spec.tunnel, spec.instance.as_deref(), pool).await;
    }

    // No-arg: pick sole location
    resolve_sole_location(pool).await
}

/// Resolve a target for the `disconnect` command.
///
/// No-arg / --all resolution is handled directly in the disconnect handler
/// using `active_state`; this function is only called when a target is named.
pub async fn resolve_disconnect_target(
    spec: &TargetSpec,
    pool: &DbPool,
) -> Result<ResolvedTarget, CliError> {
    // TODO: Phase 4.3 -- use active_state for no-arg / --all
    resolve_connect_target(spec, pool).await
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn resolve_named(
    name: &str,
    tunnel_only: bool,
    instance_filter: Option<&str>,
    pool: &DbPool,
) -> Result<ResolvedTarget, CliError> {
    if tunnel_only {
        let tunnels = Tunnel::all(pool).await?;
        let matches: Vec<_> = tunnels.into_iter().filter(|t| t.name == name).collect();
        return match matches.len() {
            0 => Err(CliError::NotFound(format!("Tunnel '{name}' not found"))),
            1 => Ok(ResolvedTarget::Tunnel(matches.into_iter().next().unwrap())),
            _ => Err(CliError::NotFound(format!(
                "Multiple tunnels named '{name}'"
            ))),
        };
    }

    // Resolve instance filter and fetch matching locations.
    let loc_matches: Vec<Location<Id>> = if let Some(inst_name) = instance_filter {
        let inst = Instance::find_by_name(pool, inst_name)
            .await?
            .ok_or_else(|| CliError::NotFound(format!("Instance '{inst_name}' not found")))?;
        Location::find_by_instance_id(pool, inst.id, false)
            .await?
            .into_iter()
            .filter(|l| l.name == name)
            .collect()
    } else {
        Location::all(pool, false)
            .await?
            .into_iter()
            .filter(|l| l.name == name)
            .collect()
    };

    // Fetch all tunnels and filter by name.
    let tun_matches: Vec<_> = Tunnel::all(pool)
        .await?
        .into_iter()
        .filter(|t| t.name == name)
        .collect();

    match (loc_matches.len(), tun_matches.len()) {
        (0, 0) => Err(CliError::NotFound(format!("'{name}' not found"))),
        (1, 0) => Ok(ResolvedTarget::Location(
            loc_matches.into_iter().next().unwrap(),
        )),
        (0, 1) => Ok(ResolvedTarget::Tunnel(
            tun_matches.into_iter().next().unwrap(),
        )),
        (1, 1) => Err(CliError::NotFound(format!(
            "'{name}' matches both a location and a tunnel. Use --tunnel."
        ))),
        (n, 0) if n > 1 => Err(CliError::NotFound(format!(
            "'{name}' exists in multiple instances. Use --instance to pick one."
        ))),
        _ => Err(CliError::NotFound(format!(
            "'{name}' matches multiple targets"
        ))),
    }
}

async fn resolve_sole_location(pool: &DbPool) -> Result<ResolvedTarget, CliError> {
    let locations = Location::all(pool, false).await?;
    match locations.len() {
        0 => Err(CliError::NotEnrolled(
            "No locations configured. Enroll an instance first.".into(),
        )),
        1 => Ok(ResolvedTarget::Location(
            locations.into_iter().next().unwrap(),
        )),
        _ => Err(CliError::NotFound(
            "Multiple locations available. Specify a name.".into(),
        )),
    }
}
