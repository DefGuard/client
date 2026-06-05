use sqlx::Row;

use crate::{
    output,
    state::{CliError, State},
};

pub async fn handle(state: &State, json: bool) -> Result<(), CliError> {
    // Query instances.
    let instance_rows = sqlx::query("SELECT id, name, url FROM instance ORDER BY name ASC")
        .fetch_all(&state.pool)
        .await?;

    // Query locations.
    let loc_rows = sqlx::query(
        "SELECT l.name, l.address, l.endpoint, l.location_mfa_mode, l.route_all_traffic, \
         i.name AS instance_name \
         FROM location l JOIN instance i ON l.instance_id = i.id \
         ORDER BY l.name ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    // Query tunnels.
    let tun_rows = sqlx::query("SELECT name, address, endpoint FROM tunnel ORDER BY name ASC")
        .fetch_all(&state.pool)
        .await?;

    if json {
        let instances: Vec<serde_json::Value> = instance_rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.get::<String, _>("name"),
                    "url": r.get::<String, _>("url"),
                })
            })
            .collect();

        let locations: Vec<serde_json::Value> = loc_rows
            .iter()
            .map(|r| {
                let mfa_mode: i64 = r.get("location_mfa_mode");
                serde_json::json!({
                    "name": r.get::<String, _>("name"),
                    "instance": r.get::<String, _>("instance_name"),
                    "address": r.get::<String, _>("address"),
                    "endpoint": r.get::<String, _>("endpoint"),
                    "mfa_enabled": mfa_mode != 1,
                    "route_all_traffic": r.get::<bool, _>("route_all_traffic"),
                })
            })
            .collect();

        let tunnels: Vec<serde_json::Value> = tun_rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.get::<String, _>("name"),
                    "address": r.get::<String, _>("address"),
                    "endpoint": r.get::<String, _>("endpoint"),
                })
            })
            .collect();

        output::emit(
            &serde_json::json!({ "instances": instances, "locations": locations, "tunnels": tunnels }),
            json,
        );
    } else {
        // Human-readable table.
        if instance_rows.is_empty() {
            println!("No instances enrolled. Use the desktop app or 'enroll' to get started.");
            return Ok(());
        }

        // Gather instance→location mapping.
        let mut inst_locs: std::collections::HashMap<String, Vec<&sqlx::sqlite::SqliteRow>> =
            std::collections::HashMap::new();
        for row in &loc_rows {
            let inst_name: String = row.get("instance_name");
            inst_locs.entry(inst_name).or_default().push(row);
        }

        // Compute column widths (header vs data).
        let loc_name_w = loc_rows
            .iter()
            .map(|r| r.get::<String, _>("name").len())
            .max()
            .unwrap_or(4)
            .max(8); // "LOCATION"
        let endpoint_w = loc_rows
            .iter()
            .map(|r| r.get::<String, _>("endpoint").len())
            .max()
            .unwrap_or(8)
            .max(8); // "ENDPOINT"

        for inst_row in &instance_rows {
            let inst_name: String = inst_row.get("name");
            let inst_url: String = inst_row.get("url");

            let locations = inst_locs.get(&inst_name);

            if let Some(locs) = locations {
                println!("\n{inst_name} ({inst_url})");
                println!(
                    "  {:<loc_name_w$}  {:<15}  {:<endpoint_w$}  {:>3}  {:<11}",
                    "LOCATION", "ADDRESS", "ENDPOINT", "MFA", "Routing"
                );

                for loc in locs.iter() {
                    let name: String = loc.get("name");
                    let address: String = loc.get("address");
                    let endpoint: String = loc.get("endpoint");
                    let mfa_mode: i64 = loc.get("location_mfa_mode");
                    let mfa = if mfa_mode == 1 { "no" } else { "yes" };
                    let route: bool = loc.get("route_all_traffic");
                    let route_label = if route { "All-traffic" } else { "Predefined" };

                    println!(
                        "  {:<loc_name_w$}  {:<15}  {:<endpoint_w$}  {mfa:>3}  {route_label:<11}",
                        name, address, endpoint
                    );
                }
            } else {
                println!("\n{inst_name} ({inst_url})");
                println!("  (no locations)");
            }
        }

        // Tunnels.
        if !tun_rows.is_empty() {
            let tun_name_w = tun_rows
                .iter()
                .map(|r| r.get::<String, _>("name").len())
                .max()
                .unwrap_or(4)
                .max(loc_name_w); // align with location column
            let tun_endpoint_w = tun_rows
                .iter()
                .map(|r| r.get::<String, _>("endpoint").len())
                .max()
                .unwrap_or(8)
                .max(endpoint_w); // align with location column

            println!("\nTunnels");
            println!(
                "  {:<tun_name_w$}  {:<15}  {:<tun_endpoint_w$}",
                "NAME", "ADDRESS", "ENDPOINT"
            );
            for tun in &tun_rows {
                let name: String = tun.get("name");
                let address: String = tun.get("address");
                let endpoint: String = tun.get("endpoint");
                println!(
                    "  {:<tun_name_w$}  {:<15}  {:<tun_endpoint_w$}",
                    name, address, endpoint
                );
            }
        }
    }

    Ok(())
}
