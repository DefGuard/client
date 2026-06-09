use std::collections::HashMap;

use defguard_core::database::models::{
    instance::Instance,
    location::{Location, LocationMfaMethod},
    Id,
};

use crate::{
    output,
    resolve::{self, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

pub async fn handle_list(state: &State, json: bool) -> Result<(), CliError> {
    let locations = Location::all(&state.pool, false).await?;

    // Build instance-id → name cache.
    let instance_names: HashMap<Id, String> = {
        let mut map = HashMap::new();
        for loc in &locations {
            if let std::collections::hash_map::Entry::Vacant(e) = map.entry(loc.instance_id) {
                if let Some(inst) = Instance::find_by_id(&state.pool, loc.instance_id).await? {
                    e.insert(inst.name);
                }
            }
        }
        map
    };

    if json {
        let entries: Vec<serde_json::Value> = locations
            .iter()
            .map(|l| {
                serde_json::json!({
                    "name": l.name,
                    "address": l.address,
                    "endpoint": l.endpoint,
                    "instance": instance_names.get(&l.instance_id).map(|n| n.as_str()).unwrap_or("?"),
                    "mfa_method": mfa_label(l.mfa_method),
                    "route_all_traffic": l.route_all_traffic,
                })
            })
            .collect();
        output::emit(&serde_json::json!({ "locations": entries }), json);
    } else {
        if locations.is_empty() {
            println!("No locations configured.");
            return Ok(());
        }

        // Compute column widths.
        let name_w = locations
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
        let inst_w = locations
            .iter()
            .filter_map(|l| instance_names.get(&l.instance_id).map(|n| n.len()))
            .max()
            .unwrap_or(8)
            .max(8);

        println!(
            "  {:<name_w$}  {:<15}  {:<endpoint_w$}  {:<inst_w$}  {:>3}  {:<11}",
            "LOCATION", "ADDRESS", "ENDPOINT", "INSTANCE", "MFA", "Routing"
        );

        for loc in &locations {
            let inst = instance_names
                .get(&loc.instance_id)
                .map(|n| n.as_str())
                .unwrap_or("?");
            println!(
                "  {:<name_w$}  {:<15}  {:<endpoint_w$}  {:<inst_w$}  {:>3}  {:>11}",
                loc.name,
                loc.address,
                loc.endpoint,
                inst,
                mfa_label(loc.mfa_method),
                if loc.route_all_traffic {
                    "All-traffic"
                } else {
                    "Predefined"
                }
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
        _ => {
            return Err(CliError::NotFound(format!("Location '{name}' not found")));
        }
    };

    let mut changed = Vec::new();

    // Update MFA method via the core setter (clamps against location_mfa_mode).
    if let Some(method_str) = mfa_method {
        let method = parse_mfa_method(method_str)?;
        Location::set_mfa_method(&state.pool, location_id, method).await?;
        changed.push(format!("MFA method → {method_str}"));
    }

    // Update routing via the core setter (rejects when policy forbids).
    if let Some(true) = route_all_traffic {
        Location::update_routing(&state.pool, location_id, true).await?;
        changed.push("route-all-traffic → on".to_string());
    } else if no_route_all_traffic {
        Location::update_routing(&state.pool, location_id, false).await?;
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
        _ => {
            return Err(CliError::NotFound(format!("Location '{name}' not found")));
        }
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
                "mfa_method": mfa_label(location.mfa_method),
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
        println!("MFA method:        {}", mfa_label(location.mfa_method));
        println!("Route all traffic: {}", location.route_all_traffic);
        println!("Keepalive:         {}s", location.keepalive_interval);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a user-facing MFA method string into the model enum.
fn parse_mfa_method(raw: &str) -> Result<LocationMfaMethod, CliError> {
    match raw.to_lowercase().as_str() {
        "totp" => Ok(LocationMfaMethod::Totp),
        "email" => Ok(LocationMfaMethod::Email),
        "oidc" => Ok(LocationMfaMethod::Oidc),
        "biometric" => Ok(LocationMfaMethod::Biometric),
        "mobile" | "mobile_approve" => Ok(LocationMfaMethod::MobileApprove),
        _ => Err(CliError::Usage(format!(
            "Invalid MFA method '{raw}'. Valid: totp, email, oidc, biometric, mobile."
        ))),
    }
}

/// Human label for an optional MFA method.
fn mfa_label(method: Option<LocationMfaMethod>) -> &'static str {
    match method {
        Some(LocationMfaMethod::Totp) => "totp",
        Some(LocationMfaMethod::Email) => "email",
        Some(LocationMfaMethod::Oidc) => "oidc",
        Some(LocationMfaMethod::Biometric) => "biometric",
        Some(LocationMfaMethod::MobileApprove) => "mobile",
        None => "none",
    }
}
