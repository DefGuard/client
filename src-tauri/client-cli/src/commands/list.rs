use std::collections::HashMap;

use defguard_core::database::models::{instance::Instance, location::Location, tunnel::Tunnel, Id};

use crate::{
    output::{self, InstanceEntry, ListOutput, LocationEntry, TunnelEntry},
    state::{CliError, State},
};

pub async fn handle(state: &State, json: bool) -> Result<(), CliError> {
    let instances = Instance::all(&state.pool).await?;
    let locations = Location::all(&state.pool, false).await?;
    let tunnels = Tunnel::all(&state.pool).await?;

    // Build instance-id → name lookup.
    let instance_names: HashMap<Id, String> =
        instances.iter().map(|i| (i.id, i.name.clone())).collect();

    // --- Typed entries ---
    let inst_entries: Vec<InstanceEntry> = instances
        .iter()
        .map(|i| InstanceEntry {
            name: i.name.clone(),
            url: i.url.clone(),
        })
        .collect();

    let loc_entries: Vec<LocationEntry> = locations
        .iter()
        .map(|l| LocationEntry {
            name: l.name.clone(),
            instance: instance_names.get(&l.instance_id).cloned(),
            address: l.address.clone(),
            endpoint: l.endpoint.clone(),
            mfa_enabled: Some(l.mfa_enabled()),
            mfa_method: None,
            route_all_traffic: Some(l.route_all_traffic),
        })
        .collect();

    let tun_entries: Vec<TunnelEntry> = tunnels
        .iter()
        .map(|t| TunnelEntry {
            name: t.name.clone(),
            address: t.address.clone(),
            endpoint: t.endpoint.clone(),
        })
        .collect();

    // --- Human message ---
    let message = if instances.is_empty() {
        "No instances enrolled. Use the desktop app or 'enroll' to get started.".to_string()
    } else {
        format_list_table(&instances, &locations, &tunnels)
    };

    output::emit(
        &ListOutput {
            instances: inst_entries,
            locations: loc_entries,
            tunnels: tun_entries,
            message,
        },
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
