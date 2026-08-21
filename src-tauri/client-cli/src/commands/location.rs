use std::collections::HashMap;

use defguard_core::database::models::{
    instance::{ClientTrafficPolicy, Instance},
    location::{Location, LocationMfaMethod},
    Id,
};
use serde_json::{json, Value};

use crate::{
    output::{CommandOutput, LocationEntry},
    resolve::{self, ResolvedTarget, TargetSpec},
    state::{CliError, State},
};

const MIN_NAME_COL_WIDTH: usize = 8;
const MIN_ENDPOINT_COL_WIDTH: usize = 8;
const MIN_INST_COL_WIDTH: usize = 8;

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
    let location_id = match &target {
        ResolvedTarget::Location(loc) => loc.id,
        ResolvedTarget::Tunnel(_) => {
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
        mfa_method: mfa_label(location.mfa_method).to_string(),
        route_all_traffic: match client_traffic_policy {
            ClientTrafficPolicy::None => location.route_all_traffic,
            ClientTrafficPolicy::DisableAllTraffic => false,
            ClientTrafficPolicy::ForceAllTraffic => true,
        },
        keepalive_interval: location.keepalive_interval,
    })
}

fn parse_mfa_method(raw: &str) -> Result<LocationMfaMethod, CliError> {
    match raw.to_lowercase().as_str() {
        "totp" => Ok(LocationMfaMethod::Totp),
        "email" => Ok(LocationMfaMethod::Email),
        "oidc" => Ok(LocationMfaMethod::Oidc),
        "biometric" => Ok(LocationMfaMethod::Biometric),
        "mobile" | "mobile_approve" => Ok(LocationMfaMethod::MobileApprove),
        "fido2" => Ok(LocationMfaMethod::Fido2),
        _ => Err(CliError::Usage(format!(
            "Invalid MFA method '{raw}'. Valid: totp, email, oidc, biometric, mobile, fido2."
        ))),
    }
}

pub(crate) fn mfa_label(method: Option<LocationMfaMethod>) -> &'static str {
    match method {
        Some(method) => method.as_str(),
        None => "none",
    }
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
                    mfa_method: Some(mfa_label(l.mfa_method).to_string()),
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
    let name_col_width = locations
        .iter()
        .map(|l| l.name.len())
        .max()
        .unwrap_or(MIN_NAME_COL_WIDTH)
        .max(MIN_NAME_COL_WIDTH);
    let endpoint_col_width = locations
        .iter()
        .map(|l| l.endpoint.len())
        .max()
        .unwrap_or(MIN_ENDPOINT_COL_WIDTH)
        .max(MIN_ENDPOINT_COL_WIDTH);
    let inst_col_width = locations
        .iter()
        .filter_map(|l| {
            instance_details
                .get(&l.instance_id)
                .map(|details| details.name.len())
        })
        .max()
        .unwrap_or(MIN_INST_COL_WIDTH)
        .max(MIN_INST_COL_WIDTH);

    let mut lines = vec![format!(
        "  {:>4}  {:<name_col_width$}  {:<15}  {:<endpoint_col_width$}  {:<inst_col_width$}  {:>3}  {:<11}",
        "ID", "LOCATION", "ADDRESS", "ENDPOINT", "INSTANCE", "MFA", "Routing"
    )];
    for location in locations {
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
            "  {:>4}  {:<name_col_width$}  {:<15}  {:<endpoint_col_width$}  {:<inst_col_width$}  {:>3}  {:>11}",
            location.id,
            location.name,
            location.address,
            location.endpoint,
            instance_name,
            mfa_label(location.mfa_method),
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
    use defguard_core::database::models::location::{LocationMfaMode, ServiceLocationMode};

    use super::*;

    fn make_location(
        id: Id,
        instance_id: Id,
        name: &str,
        endpoint: &str,
        mfa: bool,
    ) -> Location<Id> {
        Location {
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
}
