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

    let instance_names: HashMap<Id, String> = {
        let mut map = HashMap::new();
        for loc in &locations {
            if !map.contains_key(&loc.instance_id) {
                if let Some(inst) = Instance::find_by_id(&state.pool, loc.instance_id).await? {
                    map.insert(loc.instance_id, inst.name);
                }
            }
        }
        map
    };

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

    let message = if locations.is_empty() {
        "No locations configured.".to_string()
    } else {
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

        let mut lines = vec![format!(
            "  {:<name_w$}  {:<15}  {:<endpoint_w$}  {:<inst_w$}  {:>3}  {:<11}",
            "LOCATION", "ADDRESS", "ENDPOINT", "INSTANCE", "MFA", "Routing"
        )];
        for loc in &locations {
            let inst = instance_names
                .get(&loc.instance_id)
                .map(|n| n.as_str())
                .unwrap_or("?");
            lines.push(format!(
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
            ));
        }
        lines.join("\n")
    };

    output::emit(
        &serde_json::json!({ "locations": entries, "message": message }),
        json,
    );

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

    if let Some(method_str) = mfa_method {
        let method = parse_mfa_method(method_str)?;
        Location::set_mfa_method(&state.pool, location_id, method).await?;
        changed.push(format!("MFA method → {method_str}"));
    }

    if let Some(true) = route_all_traffic {
        Location::update_routing(&state.pool, location_id, true).await?;
        changed.push("route-all-traffic → on".to_string());
    } else if no_route_all_traffic {
        Location::update_routing(&state.pool, location_id, false).await?;
        changed.push("route-all-traffic → off".to_string());
    }

    let message = if changed.is_empty() {
        format!("No changes for location '{name}'.")
    } else {
        format!("Updated location '{name}': {}", changed.join(", "))
    };

    output::emit(
        &serde_json::json!({ "location": name, "changes": changed, "message": message }),
        json,
    );

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

    let mfa = mfa_label(location.mfa_method);

    let message = {
        let mut lines = Vec::new();
        lines.push(format!("Name:              {}", location.name));
        lines.push(format!("Address:           {}", location.address));
        lines.push(format!("Endpoint:          {}", location.endpoint));
        lines.push(format!("Pubkey:            {}", location.pubkey));
        lines.push(format!("Allowed IPs:       {}", location.allowed_ips));
        if let Some(dns) = &location.dns {
            lines.push(format!("DNS:               {dns}"));
        }
        lines.push(format!("MFA method:        {mfa}"));
        lines.push(format!("Route all traffic: {}", location.route_all_traffic));
        lines.push(format!(
            "Keepalive:         {}s",
            location.keepalive_interval
        ));
        lines.join("\n")
    };

    output::emit(
        &serde_json::json!({
            "name": location.name,
            "address": location.address,
            "endpoint": location.endpoint,
            "pubkey": location.pubkey,
            "allowed_ips": location.allowed_ips,
            "dns": location.dns,
            "mfa_method": mfa,
            "route_all_traffic": location.route_all_traffic,
            "keepalive_interval": location.keepalive_interval,
            "message": message,
        }),
        json,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
