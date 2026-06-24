use std::collections::HashMap;

use defguard_core::database::models::{instance::Instance, location::Location, tunnel::Tunnel, Id};
use serde_json::json;

use crate::{
    commands::location::mfa_label,
    output::{CommandOutput, InstanceEntry, LocationEntry, TunnelEntry},
    state::{CliError, State},
};

const MIN_LOCATION_NAME_COL_WIDTH: usize = 8;
const MIN_ENDPOINT_COL_WIDTH: usize = 8;
const MIN_TUNNEL_NAME_COL_WIDTH: usize = 4;

pub(crate) async fn handle(state: &State) -> Result<ListResult, CliError> {
    let instances = Instance::all(&state.pool).await?;
    let locations = Location::all(&state.pool, false).await?;
    let tunnels = Tunnel::all(&state.pool).await?;
    Ok(ListResult {
        instances,
        locations,
        tunnels,
    })
}

pub struct ListResult {
    pub instances: Vec<Instance<Id>>,
    pub locations: Vec<Location<Id>>,
    pub tunnels: Vec<Tunnel<Id>>,
}

impl CommandOutput for ListResult {
    fn human(&self) -> String {
        if self.instances.is_empty() {
            "No instances enrolled. Use the desktop app to get started.".to_string()
        } else {
            format_list_table(&self.instances, &self.locations, &self.tunnels)
        }
    }

    fn json(&self) -> serde_json::Value {
        let instance_names = self
            .instances
            .iter()
            .map(|i| (i.id, i.name.clone()))
            .collect::<HashMap<_, _>>();

        let instances = self
            .instances
            .iter()
            .map(|i| InstanceEntry {
                id: i.id,
                name: i.name.clone(),
                url: i.url.clone(),
            })
            .collect::<Vec<_>>();

        let locations: Vec<LocationEntry> = self
            .locations
            .iter()
            .map(|l| LocationEntry {
                id: l.id,
                name: l.name.clone(),
                instance: instance_names.get(&l.instance_id).cloned(),
                address: l.address.clone(),
                endpoint: l.endpoint.clone(),
                mfa_enabled: Some(l.mfa_enabled()),
                mfa_method: Some(mfa_label(l.mfa_method).to_string()),
                route_all_traffic: Some(l.route_all_traffic),
            })
            .collect();

        let tunnels: Vec<TunnelEntry> = self
            .tunnels
            .iter()
            .map(|t| TunnelEntry {
                id: t.id,
                name: t.name.clone(),
                address: t.address.clone(),
                endpoint: t.endpoint.clone(),
            })
            .collect();

        json!({
            "instances": instances,
            "locations": locations,
            "tunnels": tunnels,
        })
    }
}

fn format_list_table(
    instances: &[Instance<Id>],
    locations: &[Location<Id>],
    tunnels: &[Tunnel<Id>],
) -> String {
    let mut instance_locations: HashMap<Id, Vec<&Location<Id>>> = HashMap::new();
    for location in locations {
        instance_locations
            .entry(location.instance_id)
            .or_default()
            .push(location);
    }

    let location_name_col_width = locations
        .iter()
        .map(|l| l.name.len())
        .max()
        .unwrap_or(MIN_LOCATION_NAME_COL_WIDTH)
        .max(MIN_LOCATION_NAME_COL_WIDTH);
    let endpoint_col_width = locations
        .iter()
        .map(|l| l.endpoint.len())
        .max()
        .unwrap_or(MIN_ENDPOINT_COL_WIDTH)
        .max(MIN_ENDPOINT_COL_WIDTH);

    let mut lines = Vec::new();

    for instance in instances {
        lines.push(format!("\n{} ({})", instance.name, instance.url));
        if let Some(locations) = instance_locations.get(&instance.id) {
            lines.push(format!(
                "  {:>4}  {:<location_name_col_width$}  {:<15}  {:<endpoint_col_width$}  {:>3}  {:<11}",
                "ID", "LOCATION", "ADDRESS", "ENDPOINT", "MFA", "Routing"
            ));
            for location in locations {
                let mfa = if location.mfa_enabled() { "yes" } else { "no" };
                let route_label = if location.route_all_traffic {
                    "All-traffic"
                } else {
                    "Predefined"
                };
                lines.push(format!(
                    "  {:>4}  {:<location_name_col_width$}  {:<15}  {:<endpoint_col_width$}  {mfa:>3}  {route_label:<11}",
                    location.id, location.name, location.address, location.endpoint
                ));
            }
        } else {
            lines.push("  (no locations)".to_string());
        }
    }

    if !tunnels.is_empty() {
        let tunnel_name_col_width = tunnels
            .iter()
            .map(|t| t.name.len())
            .max()
            .unwrap_or(MIN_TUNNEL_NAME_COL_WIDTH)
            .max(location_name_col_width);
        let tunnel_endpoint_col_width = tunnels
            .iter()
            .map(|t| t.endpoint.len())
            .max()
            .unwrap_or(MIN_ENDPOINT_COL_WIDTH)
            .max(endpoint_col_width);

        lines.push("\nTunnels".to_string());
        lines.push(format!(
            "  {:>4}  {:<tunnel_name_col_width$}  {:<15}  {:<tunnel_endpoint_col_width$}",
            "ID", "NAME", "ADDRESS", "ENDPOINT"
        ));
        for tunnel in tunnels {
            lines.push(format!(
                "  {:>4}  {:<tunnel_name_col_width$}  {:<15}  {:<tunnel_endpoint_col_width$}",
                tunnel.id, tunnel.name, tunnel.address, tunnel.endpoint
            ));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use defguard_core::database::models::{
        instance::{ClientTrafficPolicy, Instance},
        location::{Location, LocationMfaMode, ServiceLocationMode},
        tunnel::Tunnel,
        Id,
    };

    use super::*;

    fn make_instance(id: Id, name: &str, url: &str) -> Instance<Id> {
        Instance {
            id,
            name: name.to_string(),
            uuid: format!("uuid-{id}"),
            url: url.to_string(),
            proxy_url: String::new(),
            username: "user".to_string(),
            token: None,
            client_traffic_policy: ClientTrafficPolicy::None,
            enterprise_enabled: false,
            openid_display_name: None,
        }
    }

    fn make_location(id: Id, instance_id: Id, name: &str, endpoint: &str) -> Location<Id> {
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
            location_mfa_mode: LocationMfaMode::Disabled,
            service_location_mode: ServiceLocationMode::Disabled,
            mfa_method: None,
            posture_check_required: false,
        }
    }

    fn make_tunnel(id: Id, name: &str, endpoint: &str) -> Tunnel<Id> {
        Tunnel {
            id,
            name: name.to_string(),
            pubkey: "pk".to_string(),
            prvkey: "prvk".to_string(),
            address: "10.1.0.0/24".to_string(),
            server_pubkey: "spk".to_string(),
            preshared_key: None,
            allowed_ips: Some("0.0.0.0/0".to_string()),
            endpoint: endpoint.to_string(),
            dns: None,
            persistent_keep_alive: 25,
            route_all_traffic: false,
            pre_up: None,
            post_up: None,
            pre_down: None,
            post_down: None,
        }
    }

    #[test]
    fn test_human_empty() {
        let result = ListResult {
            instances: Vec::new(),
            locations: Vec::new(),
            tunnels: Vec::new(),
        };
        let s = result.human();
        assert!(s.contains("No instances enrolled"));
    }

    #[test]
    fn test_human_with_data() {
        let inst = make_instance(1, "acme", "https://acme.example");
        let loc = make_location(10, 1, "office", "1.2.3.4:51820");
        let tun = make_tunnel(20, "datacenter", "5.6.7.8:51820");

        let result = ListResult {
            instances: vec![inst],
            locations: vec![loc],
            tunnels: vec![tun],
        };
        let s = result.human();
        assert!(s.contains("ID"));
        assert!(s.contains("acme"));
        assert!(s.contains("office"));
        assert!(s.contains("datacenter"));
        assert!(s.contains("Tunnels"));
    }

    #[test]
    fn test_json_empty() {
        let result = ListResult {
            instances: Vec::new(),
            locations: Vec::new(),
            tunnels: Vec::new(),
        };
        let json = result.json();
        assert_eq!(json["instances"].as_array().unwrap().len(), 0);
        assert_eq!(json["locations"].as_array().unwrap().len(), 0);
        assert_eq!(json["tunnels"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_json_with_data() {
        let inst = make_instance(1, "acme", "https://acme.example");
        let loc = make_location(10, 1, "office", "1.2.3.4:51820");
        let loc2 = make_location(11, 1, "home", "9.9.9.9:51820");
        let tun = make_tunnel(20, "datacenter", "5.6.7.8:51820");

        let result = ListResult {
            instances: vec![inst],
            locations: vec![loc, loc2],
            tunnels: vec![tun],
        };
        let json = result.json();

        let instances = json["instances"].as_array().unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0]["id"], 1);
        assert_eq!(instances[0]["name"], "acme");
        assert_eq!(instances[0]["url"], "https://acme.example");

        let locations = json["locations"].as_array().unwrap();
        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0]["id"], 10);
        assert_eq!(locations[0]["name"], "office");
        assert_eq!(locations[0]["instance"], "acme");
        assert_eq!(locations[1]["name"], "home");

        let tunnels = json["tunnels"].as_array().unwrap();
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0]["id"], 20);
        assert_eq!(tunnels[0]["name"], "datacenter");
        assert_eq!(tunnels[0]["endpoint"], "5.6.7.8:51820");
    }

    #[test]
    fn test_json_no_message_field() {
        let result = ListResult {
            instances: Vec::new(),
            locations: Vec::new(),
            tunnels: Vec::new(),
        };
        let json = result.json();
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_exit_code_zero() {
        let result = ListResult {
            instances: Vec::new(),
            locations: Vec::new(),
            tunnels: Vec::new(),
        };
        assert_eq!(result.exit_code(), 0);
    }
}
