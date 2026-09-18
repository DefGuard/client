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
mod tests {
    use defguard_core::database::models::location::{
        LocationMfaMode, LocationMfaStepMethod, ServiceLocationMode,
    };
    use sqlx::types::Json;

    use super::*;

    fn make_location(
        id: Id,
        instance_id: Id,
        name: &str,
        endpoint: &str,
        mfa: bool,
    ) -> Location<Id> {
        Location {
            mfa_steps: Json::default(),
            mfa_step_plan: Json::default(),
            client_mtu: None,
            id,
            instance_id,
            network_id: 1,
            name: name.to_string(),
            address: "10.0.0.0/24".to_string(),
            pubkey: "pk".to_string(),
            endpoint: endpoint.to_string(),
            allowed_ips: "0.0.0.0/0".to_string(),
            dns: None,
            route_all_traffic: false,
            keepalive_interval: 25,
            location_mfa_mode: if mfa {
                LocationMfaMode::Internal
            } else {
                LocationMfaMode::Disabled
            },
            service_location_mode: ServiceLocationMode::Disabled,
            mfa_method: None,
            posture_check_required: false,
        }
    }

    fn make_instance_details(
        name: &str,
        client_traffic_policy: ClientTrafficPolicy,
    ) -> InstanceDetails {
        InstanceDetails {
            name: name.to_string(),
            client_traffic_policy,
        }
    }

    #[test]
    fn test_list_human_empty() {
        let result = LocationListResult {
            locations: Vec::new(),
            instance_details: HashMap::new(),
        };
        assert_eq!(
            result.human(),
            "No locations configured. Use the desktop app to enroll an instance first."
        );
    }

    #[test]
    fn test_list_human_with_data() {
        let loc = make_location(1, 10, "office", "1.2.3.4:51820", false);
        let mut instance_details = HashMap::new();
        instance_details.insert(10, make_instance_details("acme", ClientTrafficPolicy::None));
        let result = LocationListResult {
            locations: vec![loc],
            instance_details,
        };
        let s = result.human();
        assert!(s.contains("ID"));
        assert!(s.contains("office"));
        assert!(s.contains("acme"));
        assert!(s.contains("1.2.3.4:51820"));
    }

    fn routing_column(
        route_all_traffic: bool,
        client_traffic_policy: ClientTrafficPolicy,
    ) -> String {
        let mut location = make_location(1, 10, "office", "1.2.3.4:51820", false);
        location.route_all_traffic = route_all_traffic;
        let mut instance_details = HashMap::new();
        instance_details.insert(10, make_instance_details("acme", client_traffic_policy));
        LocationListResult {
            locations: vec![location],
            instance_details,
        }
        .human()
    }

    fn char_col(line: &str, needle: &str) -> usize {
        let byte = line.find(needle).expect("the line holds the value");
        line[..byte].chars().count()
    }

    #[test]
    fn test_list_human_columns_align_with_a_wide_mfa_cell() {
        // Exercise character width for "Kraków" and the wider "2 steps" MFA cell.
        let mut multistep = make_location(1, 10, "Kraków", "1.2.3.4:51820", true);
        multistep.mfa_steps = sqlx::types::Json(steps(&[
            &[LocationMfaMethod::Totp],
            &[LocationMfaMethod::Oidc],
        ]));
        let plain = make_location(2, 10, "office", "5.6.7.8:51820", false);
        let mut instance_details = HashMap::new();
        instance_details.insert(10, make_instance_details("acme", ClientTrafficPolicy::None));

        let table = LocationListResult {
            locations: vec![multistep, plain],
            instance_details,
        }
        .human();

        let lines: Vec<&str> = table.lines().collect();
        let routing_col = char_col(lines[0], "Routing");
        let mfa_col = char_col(lines[0], "MFA");
        for line in &lines[1..] {
            assert_eq!(char_col(line, "Predefined"), routing_col, "line: {line}");
        }
        assert_eq!(char_col(lines[1], "2 steps"), mfa_col);
        assert_eq!(char_col(lines[2], "none"), mfa_col);
    }

    #[test]
    fn test_list_human_force_all_traffic_overrides_location() {
        let table = routing_column(false, ClientTrafficPolicy::ForceAllTraffic);
        assert!(table.contains("All-traffic"));
        assert!(!table.contains("Predefined"));
    }

    #[test]
    fn test_list_human_disable_all_traffic_overrides_location() {
        let table = routing_column(true, ClientTrafficPolicy::DisableAllTraffic);
        assert!(table.contains("Predefined"));
        assert!(!table.contains("All-traffic"));
    }

    #[test]
    fn test_list_human_no_policy_keeps_location_setting() {
        assert!(routing_column(true, ClientTrafficPolicy::None).contains("All-traffic"));
        assert!(routing_column(false, ClientTrafficPolicy::None).contains("Predefined"));
    }

    #[test]
    fn test_list_json_empty() {
        let result = LocationListResult {
            locations: Vec::new(),
            instance_details: HashMap::new(),
        };
        let json = result.json();
        assert_eq!(json["locations"].as_array().unwrap().len(), 0);
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_list_json_with_data() {
        let loc = make_location(1, 10, "office", "1.2.3.4:51820", false);
        let mut instance_details = HashMap::new();
        instance_details.insert(10, make_instance_details("acme", ClientTrafficPolicy::None));
        let result = LocationListResult {
            locations: vec![loc],
            instance_details,
        };
        let json = result.json();
        let locations = json["locations"].as_array().unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0]["id"], 1);
        assert_eq!(locations[0]["name"], "office");
        assert_eq!(locations[0]["instance"], "acme");
    }

    #[test]
    fn test_show_human() {
        let result = LocationShowResult {
            name: "office".to_string(),
            address: "10.0.0.0/24".to_string(),
            endpoint: "1.2.3.4:51820".to_string(),
            pubkey: "pk".to_string(),
            allowed_ips: "0.0.0.0/0".to_string(),
            dns: Some("8.8.8.8".to_string()),
            mfa_method: "totp".to_string(),
            mfa_steps: Vec::new(),
            mfa_step_plan: Vec::new(),
            route_all_traffic: false,
            keepalive_interval: 25,
        };
        let s = result.human();
        assert!(s.contains("Name:              office"));
        assert!(s.contains("Address:           10.0.0.0/24"));
        assert!(s.contains("DNS:               8.8.8.8"));
        assert!(s.contains("MFA method:        totp"));
    }

    #[test]
    fn test_show_human_without_dns() {
        let result = LocationShowResult {
            name: "office".to_string(),
            address: "10.0.0.0/24".to_string(),
            endpoint: "1.2.3.4:51820".to_string(),
            pubkey: "pk".to_string(),
            allowed_ips: "0.0.0.0/0".to_string(),
            dns: None,
            mfa_method: "none".to_string(),
            mfa_steps: Vec::new(),
            mfa_step_plan: Vec::new(),
            route_all_traffic: true,
            keepalive_interval: 30,
        };
        let s = result.human();
        assert!(!s.contains("DNS"));
        assert!(s.contains("Route all traffic: true"));
    }

    #[test]
    fn test_show_json() {
        let result = LocationShowResult {
            name: "office".to_string(),
            address: "10.0.0.0/24".to_string(),
            endpoint: "1.2.3.4:51820".to_string(),
            pubkey: "pk".to_string(),
            allowed_ips: "0.0.0.0/0".to_string(),
            dns: Some("8.8.8.8".to_string()),
            mfa_method: "totp".to_string(),
            mfa_steps: Vec::new(),
            mfa_step_plan: Vec::new(),
            route_all_traffic: false,
            keepalive_interval: 25,
        };
        let json = result.json();
        assert_eq!(json["name"], "office");
        assert_eq!(json["dns"], "8.8.8.8");
        assert_eq!(json["mfa_method"], "totp");
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_show_json_without_dns() {
        let result = LocationShowResult {
            name: "office".to_string(),
            address: "10.0.0.0/24".to_string(),
            endpoint: "1.2.3.4:51820".to_string(),
            pubkey: "pk".to_string(),
            allowed_ips: "0.0.0.0/0".to_string(),
            dns: None,
            mfa_method: "none".to_string(),
            mfa_steps: Vec::new(),
            mfa_step_plan: Vec::new(),
            route_all_traffic: true,
            keepalive_interval: 30,
        };
        let json = result.json();
        assert!(json["dns"].is_null());
    }

    #[test]
    fn test_exit_code_zero() {
        assert_eq!(
            LocationListResult {
                locations: Vec::new(),
                instance_details: HashMap::new(),
            }
            .exit_code(),
            0
        );
        assert_eq!(
            LocationShowResult {
                name: "x".to_string(),
                address: "a".to_string(),
                endpoint: "e".to_string(),
                pubkey: "p".to_string(),
                allowed_ips: "0.0.0.0/0".to_string(),
                dns: None,
                mfa_method: "n".to_string(),
                mfa_steps: Vec::new(),
                mfa_step_plan: Vec::new(),
                route_all_traffic: false,
                keepalive_interval: 25,
            }
            .exit_code(),
            0
        );
        assert_eq!(
            LocationSetResult {
                name: "x".to_string(),
                changes: Vec::new(),
            }
            .exit_code(),
            0
        );
    }

    #[test]
    fn test_set_human_no_changes() {
        let result = LocationSetResult {
            name: "office".to_string(),
            changes: Vec::new(),
        };
        assert_eq!(result.human(), "No changes for location 'office'.");
    }

    #[test]
    fn test_set_human_with_changes() {
        let result = LocationSetResult {
            name: "office".to_string(),
            changes: vec![
                "MFA method → totp".to_string(),
                "route-all-traffic → on".to_string(),
            ],
        };
        let s = result.human();
        assert!(s.contains("Updated location 'office'"));
        assert!(s.contains("MFA method → totp"));
        assert!(s.contains("route-all-traffic → on"));
    }

    #[test]
    fn test_set_json() {
        let result = LocationSetResult {
            name: "office".to_string(),
            changes: vec!["MFA method → totp".to_string()],
        };
        let json = result.json();
        assert_eq!(json["location"], "office");
        assert_eq!(json["changes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_set_json_empty_changes() {
        let result = LocationSetResult {
            name: "office".to_string(),
            changes: Vec::new(),
        };
        let json = result.json();
        assert_eq!(json["location"], "office");
        assert_eq!(json["changes"].as_array().unwrap().len(), 0);
        assert!(json["message"].is_null());
    }

    fn steps(offered: &[&[LocationMfaMethod]]) -> Vec<LocationMfaStep> {
        offered
            .iter()
            .map(|methods| LocationMfaStep {
                methods: methods
                    .iter()
                    .map(|method| LocationMfaStepMethod {
                        method: *method,
                        configured: true,
                    })
                    .collect(),
            })
            .collect()
    }

    #[test]
    fn test_parse_step_plan_ok() {
        let steps = steps(&[
            &[LocationMfaMethod::Email, LocationMfaMethod::Totp],
            &[LocationMfaMethod::Totp],
        ]);
        let plan =
            parse_step_plan("office", &["email".to_string(), "totp".to_string()], &steps).unwrap();
        assert_eq!(
            plan,
            vec![LocationMfaMethod::Email, LocationMfaMethod::Totp]
        );
    }

    #[test]
    fn test_parse_step_plan_wrong_count_rejected() {
        let steps = steps(&[&[LocationMfaMethod::Totp], &[LocationMfaMethod::Email]]);
        let err = parse_step_plan("office", &["totp".to_string()], &steps).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("2 verification steps"));
    }

    #[test]
    fn test_parse_step_plan_bad_name_rejected() {
        let steps = steps(&[&[LocationMfaMethod::Totp], &[LocationMfaMethod::Email]]);
        let err = parse_step_plan(
            "office",
            &["totp".to_string(), "smoke-signals".to_string()],
            &steps,
        )
        .unwrap_err();
        assert!(err.to_string().contains("smoke-signals"));
    }

    #[test]
    fn test_parse_step_plan_method_absent_from_step_rejected() {
        let steps = steps(&[&[LocationMfaMethod::Totp], &[LocationMfaMethod::Email]]);
        let err = parse_step_plan("office", &["totp".to_string(), "oidc".to_string()], &steps)
            .unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("step 2"));
    }

    #[test]
    fn test_parse_step_plan_keeps_fido2_for_the_desktop() {
        let steps = steps(&[&[LocationMfaMethod::Fido2]]);
        let plan = parse_step_plan("office", &["fido2".to_string()], &steps).unwrap();
        assert_eq!(plan, vec![LocationMfaMethod::Fido2]);
    }

    #[test]
    fn test_location_mfa_label_reports_step_count() {
        let mut location = make_location(1, 1, "office", "1.2.3.4:51820", true);
        location.mfa_steps = sqlx::types::Json(steps(&[
            &[LocationMfaMethod::Totp],
            &[LocationMfaMethod::Oidc],
        ]));
        assert_eq!(location_mfa_label(&location), "2 steps");
    }
}
