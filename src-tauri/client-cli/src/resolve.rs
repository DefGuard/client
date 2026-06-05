use defguard_core::database::{
    models::{location::Location, tunnel::Tunnel, Id},
    DbPool,
};
use sqlx::Row;

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
        if let Some(loc) = find_location_by_id(pool, id).await? {
            if spec.tunnel {
                return Err(CliError::Usage(
                    "Cannot use --tunnel with --id pointing to a location".into(),
                ));
            }
            return Ok(ResolvedTarget::Location(loc));
        }
        if let Some(tun) = find_tunnel_by_id(pool, id).await? {
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
        let rows = sqlx::query(
            "SELECT id, name, pubkey, prvkey, address, server_pubkey, preshared_key, \
             allowed_ips, endpoint, dns, persistent_keep_alive, route_all_traffic, \
             pre_up, post_up, pre_down, post_down \
             FROM tunnel WHERE name = $1",
        )
        .bind(name)
        .fetch_all(pool)
        .await?;

        return match rows.len() {
            0 => Err(CliError::NotFound(format!("Tunnel '{name}' not found"))),
            1 => Ok(ResolvedTarget::Tunnel(tunnel_from_row(&rows[0]))),
            _ => Err(CliError::NotFound(format!(
                "Multiple tunnels named '{name}'"
            ))),
        };
    }

    // Resolve instance filter to instance id.
    let inst_id = if let Some(inst_name) = instance_filter {
        let rows = sqlx::query("SELECT id FROM instance WHERE name = $1")
            .bind(inst_name)
            .fetch_optional(pool)
            .await?;
        match rows {
            Some(r) => Some(r.get::<Id, _>("id")),
            None => {
                return Err(CliError::NotFound(format!(
                    "Instance '{inst_name}' not found"
                )));
            }
        }
    } else {
        None
    };

    // Search locations by name.
    let loc_query = if let Some(iid) = inst_id {
        sqlx::query(
            "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
             network_id, route_all_traffic, keepalive_interval, \
             location_mfa_mode, service_location_mode, mfa_method, posture_check_required \
             FROM location WHERE name = $1 AND instance_id = $2",
        )
        .bind(name)
        .bind(iid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
             network_id, route_all_traffic, keepalive_interval, \
             location_mfa_mode, service_location_mode, mfa_method, posture_check_required \
             FROM location WHERE name = $1",
        )
        .bind(name)
        .fetch_all(pool)
        .await?
    };

    // Search tunnels by name.
    let tun_rows = sqlx::query(
        "SELECT id, name, pubkey, prvkey, address, server_pubkey, preshared_key, \
         allowed_ips, endpoint, dns, persistent_keep_alive, route_all_traffic, \
         pre_up, post_up, pre_down, post_down \
         FROM tunnel WHERE name = $1",
    )
    .bind(name)
    .fetch_all(pool)
    .await?;

    match (loc_query.len(), tun_rows.len()) {
        (0, 0) => Err(CliError::NotFound(format!("'{name}' not found"))),
        (1, 0) => Ok(ResolvedTarget::Location(location_from_row(&loc_query[0]))),
        (0, 1) => Ok(ResolvedTarget::Tunnel(tunnel_from_row(&tun_rows[0]))),
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
    let rows = sqlx::query(
        "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
         network_id, route_all_traffic, keepalive_interval, \
         location_mfa_mode, service_location_mode, mfa_method, posture_check_required \
         FROM location",
    )
    .fetch_all(pool)
    .await?;

    match rows.len() {
        0 => Err(CliError::NotEnrolled(
            "No locations configured. Enroll an instance first.".into(),
        )),
        1 => Ok(ResolvedTarget::Location(location_from_row(&rows[0]))),
        _ => Err(CliError::NotFound(
            "Multiple locations available. Specify a name.".into(),
        )),
    }
}

async fn find_location_by_id(pool: &DbPool, id: Id) -> Result<Option<Location<Id>>, CliError> {
    let row = sqlx::query(
        "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
         network_id, route_all_traffic, keepalive_interval, \
         location_mfa_mode, service_location_mode, mfa_method, posture_check_required \
         FROM location WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(location_from_row))
}

async fn find_tunnel_by_id(pool: &DbPool, id: Id) -> Result<Option<Tunnel<Id>>, CliError> {
    let row = sqlx::query(
        "SELECT id, name, pubkey, prvkey, address, server_pubkey, preshared_key, \
         allowed_ips, endpoint, dns, persistent_keep_alive, route_all_traffic, \
         pre_up, post_up, pre_down, post_down \
         FROM tunnel WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(tunnel_from_row))
}

fn location_from_row(row: &sqlx::sqlite::SqliteRow) -> Location<Id> {
    use defguard_core::database::models::location::{
        LocationMfaMethod, LocationMfaMode, ServiceLocationMode,
    };

    let mfa_mode_raw: i64 = row.get("location_mfa_mode");
    let location_mfa_mode = match mfa_mode_raw {
        2 => LocationMfaMode::Internal,
        3 => LocationMfaMode::External,
        _ => LocationMfaMode::Disabled, // 1 or anything else
    };

    let svc_mode_raw: i64 = row.get("service_location_mode");
    let service_location_mode = match svc_mode_raw {
        2 => ServiceLocationMode::PreLogon,
        3 => ServiceLocationMode::AlwaysOn,
        _ => ServiceLocationMode::Disabled,
    };

    let mfa_method_raw: Option<i64> = row.get("mfa_method");
    let mfa_method = mfa_method_raw.map(|v| match v {
        0 => LocationMfaMethod::Totp,
        1 => LocationMfaMethod::Email,
        2 => LocationMfaMethod::Oidc,
        3 => LocationMfaMethod::Biometric,
        4 => LocationMfaMethod::MobileApprove,
        _ => LocationMfaMethod::Totp,
    });

    Location {
        id: row.get("id"),
        instance_id: row.get("instance_id"),
        network_id: row.get("network_id"),
        name: row.get("name"),
        address: row.get("address"),
        pubkey: row.get("pubkey"),
        endpoint: row.get("endpoint"),
        allowed_ips: row.get("allowed_ips"),
        dns: row.get("dns"),
        route_all_traffic: row.get("route_all_traffic"),
        keepalive_interval: row.get("keepalive_interval"),
        location_mfa_mode,
        service_location_mode,
        mfa_method,
        posture_check_required: row.get("posture_check_required"),
    }
}

fn tunnel_from_row(row: &sqlx::sqlite::SqliteRow) -> Tunnel<Id> {
    Tunnel {
        id: row.get("id"),
        name: row.get("name"),
        pubkey: row.get("pubkey"),
        prvkey: row.get("prvkey"),
        address: row.get("address"),
        server_pubkey: row.get("server_pubkey"),
        preshared_key: row.get("preshared_key"),
        allowed_ips: row.get("allowed_ips"),
        endpoint: row.get("endpoint"),
        dns: row.get("dns"),
        persistent_keep_alive: row.get("persistent_keep_alive"),
        route_all_traffic: row.get("route_all_traffic"),
        pre_up: row.get("pre_up"),
        post_up: row.get("post_up"),
        pre_down: row.get("pre_down"),
        post_down: row.get("post_down"),
    }
}
