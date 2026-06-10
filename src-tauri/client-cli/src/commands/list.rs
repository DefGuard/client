use std::collections::HashMap;

use defguard_core::database::models::{instance::Instance, location::Location, tunnel::Tunnel, Id};

use crate::{
    output,
    state::{CliError, State},
};

pub async fn handle(state: &State, json: bool) -> Result<(), CliError> {
    let instances = Instance::all(&state.pool).await?;
    let locations = Location::all(&state.pool, false).await?;
    let tunnels = Tunnel::all(&state.pool).await?;

    // Build instance-id → name lookup.
    let instance_names: HashMap<Id, String> =
        instances.iter().map(|i| (i.id, i.name.clone())).collect();

    // --- JSON entries ---
    let inst_json: Vec<serde_json::Value> = instances
        .iter()
        .map(|i| serde_json::json!({ "name": i.name, "url": i.url }))
        .collect();

    let loc_json: Vec<serde_json::Value> = locations
        .iter()
        .map(|l| {
            serde_json::json!({
                "name": l.name,
                "instance": instance_names.get(&l.instance_id).map(|n| n.as_str()).unwrap_or("?"),
                "address": l.address,
                "endpoint": l.endpoint,
                "mfa_enabled": l.mfa_enabled(),
                "route_all_traffic": l.route_all_traffic,
            })
        })
        .collect();

    let tun_json: Vec<serde_json::Value> = tunnels
        .iter()
        .map(
            |t| serde_json::json!({ "name": t.name, "address": t.address, "endpoint": t.endpoint }),
        )
        .collect();

    // --- Human message ---
    let message = if instances.is_empty() {
        "No instances enrolled. Use the desktop app or 'enroll' to get started.".to_string()
    } else {
        format_list_table(&instances, &locations, &tunnels)
    };

    output::emit(
        &serde_json::json!({
            "instances": inst_json,
            "locations": loc_json,
            "tunnels": tun_json,
            "message": message,
        }),
        json,
    );

    Ok(())
}

fn format_list_table(
    instances: &[Instance<Id>],
    locations: &[Location<Id>],
    tunnels: &[Tunnel<Id>],
) -> String {
    let mut inst_locs: HashMap<Id, Vec<&Location<Id>>> = HashMap::new();
    for loc in locations {
        inst_locs.entry(loc.instance_id).or_default().push(loc);
    }

    let loc_name_w = locations
        .iter()
        .map(|l| l.name.len())
        .max()
        .unwrap_or(4)
        .max(8);
    let endpoint_w = locations
        .iter()
        .map(|l| l.endpoint.len())
        .max()
        .unwrap_or(8)
        .max(8);

    let mut lines = Vec::new();

    for inst in instances {
        lines.push(format!("\n{} ({})", inst.name, inst.url));
        if let Some(locs) = inst_locs.get(&inst.id) {
            lines.push(format!(
                "  {:<loc_name_w$}  {:<15}  {:<endpoint_w$}  {:>3}  {:<11}",
                "LOCATION", "ADDRESS", "ENDPOINT", "MFA", "Routing"
            ));
            for loc in locs.iter() {
                let mfa = if loc.mfa_enabled() { "yes" } else { "no" };
                let route_label = if loc.route_all_traffic {
                    "All-traffic"
                } else {
                    "Predefined"
                };
                lines.push(format!(
                    "  {:<loc_name_w$}  {:<15}  {:<endpoint_w$}  {mfa:>3}  {route_label:<11}",
                    loc.name, loc.address, loc.endpoint
                ));
            }
        } else {
            lines.push("  (no locations)".to_string());
        }
    }

    if !tunnels.is_empty() {
        let tun_name_w = tunnels
            .iter()
            .map(|t| t.name.len())
            .max()
            .unwrap_or(4)
            .max(loc_name_w);
        let tun_endpoint_w = tunnels
            .iter()
            .map(|t| t.endpoint.len())
            .max()
            .unwrap_or(8)
            .max(endpoint_w);

        lines.push("\nTunnels".to_string());
        lines.push(format!(
            "  {:<tun_name_w$}  {:<15}  {:<tun_endpoint_w$}",
            "NAME", "ADDRESS", "ENDPOINT"
        ));
        for tun in tunnels {
            lines.push(format!(
                "  {:<tun_name_w$}  {:<15}  {:<tun_endpoint_w$}",
                tun.name, tun.address, tun.endpoint
            ));
        }
    }

    lines.join("\n")
}
