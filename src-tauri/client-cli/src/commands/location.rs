use sqlx::Row as _;

use crate::{
    output,
    resolve::{self, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

pub async fn handle_list(state: &State, json: bool) -> Result<(), CliError> {
    let rows = sqlx::query(
        "SELECT l.name, l.address, l.endpoint, l.location_mfa_mode, i.name AS instance_name, \
         l.mfa_method, l.route_all_traffic \
         FROM location l JOIN instance i ON l.instance_id = i.id ORDER BY l.name ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    if json {
        let locations: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.get::<String, _>("name"),
                    "address": r.get::<String, _>("address"),
                    "endpoint": r.get::<String, _>("endpoint"),
                    "instance": r.get::<String, _>("instance_name"),
                    "mfa_method": mfa_label(r.get::<Option<i32>, _>("mfa_method")),
                    "route_all_traffic": r.get::<bool, _>("route_all_traffic"),
                })
            })
            .collect();
        output::emit(&serde_json::json!({ "locations": locations }), json);
    } else {
        if rows.is_empty() {
            println!("No locations configured.");
            return Ok(());
        }

        // Compute column widths.
        let name_w = rows
            .iter()
            .map(|r| r.get::<String, _>("name").len())
            .max()
            .unwrap_or(4)
            .max(8); // "LOCATION"
        let endpoint_w = rows
            .iter()
            .map(|r| r.get::<String, _>("endpoint").len())
            .max()
            .unwrap_or(8)
            .max(8); // "ENDPOINT"
        let inst_w = rows
            .iter()
            .map(|r| r.get::<String, _>("instance_name").len())
            .max()
            .unwrap_or(8)
            .max(8); // "INSTANCE"

        println!(
            "  {:<name_w$}  {:<15}  {:<endpoint_w$}  {:<inst_w$}  {:>3}  {:<11}",
            "LOCATION", "ADDRESS", "ENDPOINT", "INSTANCE", "MFA", "Routing"
        );

        for row in &rows {
            let name: String = row.get("name");
            let address: String = row.get("address");
            let endpoint: String = row.get("endpoint");
            let instance: String = row.get("instance_name");
            let mfa: Option<i32> = row.get("mfa_method");
            let route: bool = row.get("route_all_traffic");
            println!(
                "  {:<name_w$}  {:<15}  {:<endpoint_w$}  {:<inst_w$}  {:>3}  {:>11}",
                name,
                address,
                endpoint,
                instance,
                mfa_label(mfa),
                if route { "All-traffic" } else { "Predefined" }
            );
        }
    }

    Ok(())
}

pub async fn handle_set(
    state: &State,
    json: bool,
    name: &str,
    instance: Option<&str>,
    mfa_method: Option<&str>,
    route_all_traffic: Option<bool>,
    no_route_all_traffic: bool,
) -> Result<(), CliError> {
    let spec = TargetSpec {
        name: Some(name.to_string()),
        tunnel: false,
        id: None,
        instance: instance.map(String::from),
    };

    let target = resolve::resolve_connect_target(&spec, &state.pool).await?;
    let location_id = match &target {
        ResolvedTarget::Location(loc) => loc.id,
        _ => return Err(CliError::NotFound(format!("Location '{name}' not found"))),
    };

    let mut changed = Vec::new();

    // Update MFA method.
    if let Some(method) = mfa_method {
        let mfa_val = match method.to_lowercase().as_str() {
            "totp" => "Totp",
            "email" => "Email",
            "oidc" => "Oidc",
            "mobile" | "mobile_approve" => "MobileApprove",
            _ => {
                return Err(CliError::Usage(format!(
                    "Invalid MFA method '{method}'. Valid: totp, email, oidc, mobile."
                )));
            }
        };
        sqlx::query("UPDATE location SET mfa_method = $1 WHERE id = $2")
            .bind(mfa_val)
            .bind(location_id)
            .execute(&state.pool)
            .await?;
        changed.push(format!("MFA method → {method}"));
    }

    // Update route-all-traffic.
    if route_all_traffic == Some(true) {
        sqlx::query("UPDATE location SET route_all_traffic = 1 WHERE id = $1")
            .bind(location_id)
            .execute(&state.pool)
            .await?;
        changed.push("route-all-traffic → on".to_string());
    } else if no_route_all_traffic {
        sqlx::query("UPDATE location SET route_all_traffic = 0 WHERE id = $1")
            .bind(location_id)
            .execute(&state.pool)
            .await?;
        changed.push("route-all-traffic → off".to_string());
    }

    if changed.is_empty() {
        if json {
            output::emit(
                &serde_json::json!({ "location": name, "message": "no changes" }),
                json,
            );
        } else {
            println!("No changes for location '{name}'.");
        }
        return Ok(());
    }

    if json {
        output::emit(
            &serde_json::json!({ "location": name, "changes": changed }),
            json,
        );
    } else {
        println!("Updated location '{name}': {}", changed.join(", "));
    }

    Ok(())
}

pub async fn handle_show(
    state: &State,
    json: bool,
    name: &str,
    instance: Option<&str>,
) -> Result<(), CliError> {
    let spec = TargetSpec {
        name: Some(name.to_string()),
        tunnel: false,
        id: None,
        instance: instance.map(String::from),
    };

    let target = resolve::resolve_connect_target(&spec, &state.pool).await?;
    let location = match &target {
        ResolvedTarget::Location(loc) => loc,
        _ => return Err(CliError::NotFound(format!("Location '{name}' not found"))),
    };

    if json {
        output::emit(
            &serde_json::json!({
                "name": location.name,
                "address": location.address,
                "endpoint": location.endpoint,
                "pubkey": location.pubkey,
                "allowed_ips": location.allowed_ips,
                "dns": location.dns,
                "mfa_method": location.mfa_method.as_ref().map(|m| format!("{m:?}")),
                "route_all_traffic": location.route_all_traffic,
                "keepalive_interval": location.keepalive_interval,
            }),
            json,
        );
    } else {
        println!("Name:              {}", location.name);
        println!("Address:           {}", location.address);
        println!("Endpoint:          {}", location.endpoint);
        println!("Pubkey:            {}", location.pubkey);
        println!("Allowed IPs:       {}", location.allowed_ips);
        if let Some(dns) = &location.dns {
            println!("DNS:               {dns}");
        }
        println!(
            "MFA method:        {}",
            location
                .mfa_method
                .as_ref()
                .map(|m| format!("{m:?}"))
                .unwrap_or_else(|| "none".to_string())
        );
        println!("Route all traffic: {}", location.route_all_traffic);
        println!("Keepalive:         {}s", location.keepalive_interval);
    }

    Ok(())
}

fn mfa_label(v: Option<i32>) -> &'static str {
    match v {
        Some(0) => "totp",
        Some(1) => "email",
        Some(2) => "oidc",
        Some(3) => "biometric",
        Some(4) => "mobile",
        _ => "none",
    }
}
