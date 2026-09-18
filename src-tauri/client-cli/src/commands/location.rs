use std::collections::HashMap;

use defguard_core::database::models::{
    instance::{ClientTrafficPolicy, Instance},
    location::{Location, LocationMfaMethod, LocationMfaStep},
    Id,
};
use serde_json::{json, Value};

use crate::{
    mfa::{join_methods, parse_method},
    output::{CommandOutput, LocationEntry},
    resolve::{self, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

const MIN_NAME_COL_WIDTH: usize = 8;
const MIN_ADDRESS_COL_WIDTH: usize = 15;
const MIN_ENDPOINT_COL_WIDTH: usize = 8;
const MIN_INST_COL_WIDTH: usize = 8;
const MIN_MFA_COL_WIDTH: usize = 3;

/// Return the widest value, but never less than `min`.
///
/// Count characters because `format!` pads strings by character, not byte.
pub(crate) fn col_width<'a>(values: impl Iterator<Item = &'a str>, min: usize) -> usize {
    values
        .map(|value| value.chars().count())
        .max()
        .unwrap_or(0)
        .max(min)
}

pub(crate) async fn handle_list(state: &State) -> Result<LocationListResult, CliError> {
    let locations = Location::all(&state.pool, false).await?;

    let instance_details = Instance::all(&state.pool)
        .await?
        .into_iter()
        .map(|instance| {
            (
                instance.id,
                InstanceDetails {
                    name: instance.name,
                    client_traffic_policy: instance.client_traffic_policy,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    Ok(LocationListResult {
        locations,
        instance_details,
    })
}

pub async fn handle_set(
    state: &State,
    name: &str,
    instance: Option<&str>,
    mfa_method: Option<&str>,
    mfa_steps: &[String],
    route_all_traffic: Option<bool>,
    predefined_traffic: bool,
) -> Result<LocationSetResult, CliError> {
    let spec = TargetSpec {
        name: Some(name.to_string()),
        tunnel: false,
        id: None,
        instance: instance.map(String::from),
    };

    let target = resolve::resolve_connect_target(&spec, &state.pool).await?;
    let location = match &target {
        ResolvedTarget::Location(loc) => loc,
        ResolvedTarget::Tunnel(_) => {
            return Err(CliError::NotFound(format!("Location '{name}' not found")));
        }
    };
    let location_id = location.id;

    let mut changed = Vec::new();

    if !mfa_steps.is_empty() {
        if mfa_method.is_some() {
            return Err(CliError::InvalidInput(
                "--mfa-step conflicts with --mfa-method; use only one.".into(),
            ));
        }
        if location.mfa_steps.len() <= 1 {
            return Err(CliError::InvalidInput(
                "--mfa-step requires a multi-step location; use --mfa-method for a single-step location.".into(),
            ));
        }
        let plan = parse_step_plan(name, mfa_steps, &location.mfa_steps)?;
        Location::set_mfa_step_plan(&state.pool, location_id, plan).await?;
        changed.push(format!("MFA steps → {}", mfa_steps.join(", ")));
    }

    if let Some(method_str) = mfa_method {
        let method = parse_method(method_str)?;
        Location::set_mfa_method(&state.pool, location_id, method).await?;
        changed.push(format!("MFA method → {method_str}"));
    }

    if let Some(true) = route_all_traffic {
        Location::update_routing(&state.pool, location_id, true).await?;
        changed.push("route-all-traffic → on".to_string());
    } else if predefined_traffic {
        Location::update_routing(&state.pool, location_id, false).await?;
        changed.push("route-all-traffic → off".to_string());
    }

    Ok(LocationSetResult {
        name: name.to_string(),
        changes: changed,
    })
}

pub async fn handle_show(
    state: &State,
    name: &str,
    instance: Option<&str>,
) -> Result<LocationShowResult, CliError> {
    let spec = TargetSpec {
        name: Some(name.to_string()),
        tunnel: false,
        id: None,
        instance: instance.map(String::from),
    };

    let target = resolve::resolve_connect_target(&spec, &state.pool).await?;
    let ResolvedTarget::Location(location) = &target else {
        return Err(CliError::NotFound(format!("Location '{name}' not found")));
    };
    let client_traffic_policy = Instance::find_by_id(&state.pool, location.instance_id)
        .await?
        .map_or(ClientTrafficPolicy::None, |instance| {
            instance.client_traffic_policy
        });

    Ok(LocationShowResult {
        name: location.name.clone(),
        address: location.address.clone(),
        endpoint: location.endpoint.clone(),
        pubkey: location.pubkey.clone(),
        allowed_ips: location.allowed_ips.clone(),
        dns: location.dns.clone(),
        mfa_method: location_mfa_label(location),
        mfa_steps: location
            .mfa_steps
            .iter()
            .map(|step| step.methods.iter().map(|entry| entry.method).collect())
            .collect(),
        mfa_step_plan: location.mfa_step_plan.to_vec(),
        route_all_traffic: match client_traffic_policy {
            ClientTrafficPolicy::None => location.route_all_traffic,
            ClientTrafficPolicy::DisableAllTraffic => false,
            ClientTrafficPolicy::ForceAllTraffic => true,
        },
        keepalive_interval: location.keepalive_interval,
    })
}

/// Parse one method per verification step for `location set --mfa-step`.
///
/// Unsupported methods can be saved because the desktop client reads the same
/// plan; `connect` rejects methods the CLI cannot run.
fn parse_step_plan(
    name: &str,
    raw: &[String],
    steps: &[LocationMfaStep],
) -> Result<Vec<LocationMfaMethod>, CliError> {
    if raw.len() != steps.len() {
        return Err(CliError::InvalidInput(format!(
            "Location '{name}' has {} verification steps but {} --mfa-step values were given.",
            steps.len(),
            raw.len()
        )));
    }
    raw.iter()
        .zip(steps.iter())
        .enumerate()
        .map(|(index, (value, step))| {
            let method = parse_method(value)?;
            let offered: Vec<LocationMfaMethod> =
                step.methods.iter().map(|entry| entry.method).collect();
            if !offered.contains(&method) {
                return Err(CliError::InvalidInput(format!(
                    "'{value}' is not available for step {} of '{name}' (offered: {}).",
                    index + 1,
                    join_methods(&offered)
                )));
            }
            Ok(method)
        })
        .collect()
}

pub(crate) fn mfa_label(method: Option<LocationMfaMethod>) -> &'static str {
    match method {
        Some(method) => method.as_str(),
        None => "none",
    }
}

/// Format the MFA column for a location. Multi-step locations show their step
/// count instead of a single method.
pub(crate) fn location_mfa_label(location: &Location<Id>) -> String {
    if location.mfa_steps.len() > 1 {
        return format!("{} steps", location.mfa_steps.len());
    }
    mfa_label(location.mfa_method).to_string()
}

pub(crate) struct InstanceDetails {
    pub name: String,
    pub client_traffic_policy: ClientTrafficPolicy,
}

pub struct LocationListResult {
    pub locations: Vec<Location<Id>>,
    pub instance_details: HashMap<Id, InstanceDetails>,
}

impl CommandOutput for LocationListResult {
    fn human(&self) -> String {
        if self.locations.is_empty() {
            "No locations configured. Use the desktop app to enroll an instance first.".to_string()
        } else {
            format_location_list_table(&self.locations, &self.instance_details)
        }
    }

    fn json(&self) -> Value {
        let locations = self
            .locations
            .iter()
            .map(|l| {
                let details = self.instance_details.get(&l.instance_id);
                let route_all_traffic = match details
                    .map_or(&ClientTrafficPolicy::None, |details| {
                        &details.client_traffic_policy
                    }) {
                    ClientTrafficPolicy::None => l.route_all_traffic,
                    ClientTrafficPolicy::DisableAllTraffic => false,
                    ClientTrafficPolicy::ForceAllTraffic => true,
                };
                LocationEntry {
                    id: l.id,
                    name: l.name.clone(),
                    instance: details.map(|details| details.name.clone()),
                    address: l.address.clone(),
                    endpoint: l.endpoint.clone(),
                    mfa_enabled: None,
                    mfa_method: Some(location_mfa_label(l)),
                    route_all_traffic: Some(route_all_traffic),
                }
            })
            .collect::<Vec<_>>();
        json!({ "locations": locations })
    }
}

fn format_location_list_table(
    locations: &[Location<Id>],
    instance_details: &HashMap<Id, InstanceDetails>,
) -> String {
    let name_col_width = col_width(
        locations.iter().map(|l| l.name.as_str()),
        MIN_NAME_COL_WIDTH,
    );
    let address_col_width = col_width(
        locations.iter().map(|l| l.address.as_str()),
        MIN_ADDRESS_COL_WIDTH,
    );
    let endpoint_col_width = col_width(
        locations.iter().map(|l| l.endpoint.as_str()),
        MIN_ENDPOINT_COL_WIDTH,
    );
    let inst_col_width = col_width(
        locations.iter().filter_map(|l| {
            instance_details
                .get(&l.instance_id)
                .map(|details| details.name.as_str())
        }),
        MIN_INST_COL_WIDTH,
    );
    // Measure MFA labels too so the Routing column stays aligned.
    let mfa_labels: Vec<String> = locations.iter().map(location_mfa_label).collect();
    let mfa_col_width = col_width(mfa_labels.iter().map(String::as_str), MIN_MFA_COL_WIDTH);

    let mut lines = vec![format!(
        "  {:>4}  {:<name_col_width$}  {:<address_col_width$}  {:<endpoint_col_width$}  {:<inst_col_width$}  {:<mfa_col_width$}  {}",
        "ID", "LOCATION", "ADDRESS", "ENDPOINT", "INSTANCE", "MFA", "Routing"
    )];
    for (location, mfa_label) in locations.iter().zip(&mfa_labels) {
        let details = instance_details.get(&location.instance_id);

        let instance_name = details.map_or("?", |instance| instance.name.as_str());
        let instance_traffic_policy = details.map_or(&ClientTrafficPolicy::None, |instance| {
            &instance.client_traffic_policy
        });

        let route_all_traffic = match instance_traffic_policy {
            ClientTrafficPolicy::None => location.route_all_traffic,
            ClientTrafficPolicy::DisableAllTraffic => false,
            ClientTrafficPolicy::ForceAllTraffic => true,
        };

        let route_label = if route_all_traffic {
            "All-traffic"
        } else {
            "Predefined"
        };

        lines.push(format!(
            "  {:>4}  {:<name_col_width$}  {:<address_col_width$}  {:<endpoint_col_width$}  {:<inst_col_width$}  {:<mfa_col_width$}  {}",
            location.id,
            location.name,
            location.address,
            location.endpoint,
            instance_name,
            mfa_label,
            route_label
        ));
    }
    lines.join("\n")
}

pub struct LocationShowResult {
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub pubkey: String,
    pub allowed_ips: String,
    pub dns: Option<String>,
    pub mfa_method: String,
    /// Methods offered by each verification step, in order. Empty when Edge
    /// returned no step data.
    pub mfa_steps: Vec<Vec<LocationMfaMethod>>,
    /// Saved method for each step, set by `location set --mfa-step`.
    pub mfa_step_plan: Vec<LocationMfaMethod>,
    pub route_all_traffic: bool,
    pub keepalive_interval: i64,
}

impl CommandOutput for LocationShowResult {
    fn human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Name:              {}", self.name));
        lines.push(format!("Address:           {}", self.address));
        lines.push(format!("Endpoint:          {}", self.endpoint));
        lines.push(format!("Pubkey:            {}", self.pubkey));
        lines.push(format!("Allowed IPs:       {}", self.allowed_ips));
        if let Some(dns) = &self.dns {
            lines.push(format!("DNS:               {dns}"));
        }
        lines.push(format!("MFA method:        {}", self.mfa_method));
        if self.mfa_steps.len() > 1 {
            for (index, methods) in self.mfa_steps.iter().enumerate() {
                lines.push(format!(
                    "MFA step {}:        {}",
                    index + 1,
                    join_methods(methods)
                ));
            }
            if !self.mfa_step_plan.is_empty() {
                lines.push(format!(
                    "MFA saved plan:    {}",
                    join_methods(&self.mfa_step_plan)
                ));
            }
        }
        lines.push(format!("Route all traffic: {}", self.route_all_traffic));
        lines.push(format!("Keepalive:         {}s", self.keepalive_interval));
        lines.join("\n")
    }

    fn json(&self) -> Value {
        let mut json = json!({
            "name": self.name,
            "address": self.address,
            "endpoint": self.endpoint,
            "pubkey": self.pubkey,
            "allowed_ips": self.allowed_ips,
            "mfa_method": self.mfa_method,
            "mfa_steps": self.mfa_steps
                .iter()
                .map(|methods| methods.iter().map(|method| method.as_str()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            "mfa_step_plan": self.mfa_step_plan
                .iter()
                .map(|method| method.as_str())
                .collect::<Vec<_>>(),
            "route_all_traffic": self.route_all_traffic,
            "keepalive_interval": self.keepalive_interval,
        });
        if let Some(dns) = &self.dns {
            json["dns"] = json!(dns);
        }
        json
    }
}

pub struct LocationSetResult {
    pub name: String,
    pub changes: Vec<String>,
}

impl CommandOutput for LocationSetResult {
    fn human(&self) -> String {
        if self.changes.is_empty() {
            format!("No changes for location '{}'.", self.name)
        } else {
            format!(
                "Updated location '{}': {}",
                self.name,
                self.changes.join(", ")
            )
        }
    }

    fn json(&self) -> Value {
        json!({
            "location": self.name,
            "changes": self.changes,
        })
    }
}

#[cfg(test)]
mod tests;
