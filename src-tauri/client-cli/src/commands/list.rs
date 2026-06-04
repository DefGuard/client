use serde::Serialize;
use sqlx::Row;

use crate::{
    output,
    state::{CliError, State},
};

/// Renderable summary of an instance with its locations.
#[derive(Serialize, Debug)]
struct InstanceSummary {
    name: String,
    url: String,
    location_count: usize,
}

/// Renderable summary of a location.
#[derive(Serialize, Debug)]
struct LocationSummary {
    name: String,
    instance: String,
    address: String,
    endpoint: String,
    mfa_enabled: bool,
}

/// Renderable summary of a tunnel.
#[derive(Serialize, Debug)]
struct TunnelSummary {
    name: String,
    address: String,
    endpoint: String,
}

pub async fn handle(state: &State, json: bool) -> Result<(), CliError> {
    // Query instances.
    let instance_rows = sqlx::query("SELECT id, name, url FROM instance ORDER BY name ASC")
        .fetch_all(&state.pool)
        .await?;

    let instances: Vec<InstanceSummary> = instance_rows
        .iter()
        .map(|r| InstanceSummary {
            name: r.get("name"),
            url: r.get("url"),
            location_count: 0, // filled below
        })
        .collect();

    // Query locations.
    let loc_rows = sqlx::query(
        "SELECT l.name, l.address, l.endpoint, l.location_mfa_mode, i.name AS instance_name, \
         l.instance_id \
         FROM location l JOIN instance i ON l.instance_id = i.id \
         ORDER BY l.name ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    let locations: Vec<LocationSummary> = loc_rows
        .iter()
        .map(|r| {
            let mfa_mode: i64 = r.get("location_mfa_mode");
            LocationSummary {
                name: r.get("name"),
                instance: r.get("instance_name"),
                address: r.get("address"),
                endpoint: r.get("endpoint"),
                mfa_enabled: mfa_mode != 1, // 1 = Disabled
            }
        })
        .collect();

    // Count locations per instance.
    let mut instances = instances;
    for inst in &mut instances {
        inst.location_count = locations
            .iter()
            .filter(|l| {
                loc_rows.iter().any(|r| {
                    let inst_name: String = r.get("instance_name");
                    r.get::<String, _>("name") == l.name && inst_name == inst.name
                })
            })
            .count();
    }

    // Simpler: count by instance name directly.
    let mut instance_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for loc in &locations {
        *instance_counts.entry(loc.instance.clone()).or_default() += 1;
    }
    for inst in &mut instances {
        inst.location_count = instance_counts.get(&inst.name).copied().unwrap_or(0);
    }

    // Query tunnels.
    let tun_rows = sqlx::query("SELECT name, address, endpoint FROM tunnel ORDER BY name ASC")
        .fetch_all(&state.pool)
        .await?;

    let tunnels: Vec<TunnelSummary> = tun_rows
        .iter()
        .map(|r| TunnelSummary {
            name: r.get("name"),
            address: r.get("address"),
            endpoint: r.get("endpoint"),
        })
        .collect();

    // Output.
    if !instances.is_empty() {
        output::emit(&serde_json::json!({ "instances": instances }), json);
    } else {
        if json {
            output::emit(&serde_json::json!({ "instances": [] }), json);
        }
        tracing::warn!("No instances enrolled yet.");
    }

    if !locations.is_empty() {
        output::emit(&serde_json::json!({ "locations": locations }), json);
    }

    if !tunnels.is_empty() {
        output::emit(&serde_json::json!({ "tunnels": tunnels }), json);
    }

    Ok(())
}
