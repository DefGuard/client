use defguard_core::database::models::{instance::Instance, Id};

use crate::{
    output::CommandOutput,
    state::{CliError, State},
};

pub async fn handle_list(state: &State) -> Result<InstanceListResult, CliError> {
    let instances = Instance::all(&state.pool).await?;
    Ok(InstanceListResult { instances })
}

pub async fn handle_show(state: &State, name: &str) -> Result<InstanceShowResult, CliError> {
    let inst = Instance::find_by_name(&state.pool, name)
        .await?
        .ok_or_else(|| CliError::NotFound(format!("Instance '{name}' not found")))?;
    Ok(InstanceShowResult { inst })
}

pub struct InstanceListResult {
    pub instances: Vec<Instance<Id>>,
}

impl CommandOutput for InstanceListResult {
    fn human(&self) -> String {
        if self.instances.is_empty() {
            "No instances configured.".to_string()
        } else {
            format_instance_list_table(&self.instances)
        }
    }

    fn json(&self) -> serde_json::Value {
        let instances: Vec<serde_json::Value> = self
            .instances
            .iter()
            .map(|inst| {
                serde_json::json!({
                    "name": inst.name,
                    "url": inst.url,
                    "username": inst.username,
                    "traffic_policy": format!("{:?}", inst.client_traffic_policy),
                })
            })
            .collect();
        serde_json::json!({ "instances": instances })
    }
}

fn format_instance_list_table(instances: &[Instance<Id>]) -> String {
    let name_w = instances
        .iter()
        .map(|i| i.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let url_w = instances
        .iter()
        .map(|i| i.url.len())
        .max()
        .unwrap_or(3)
        .max(3);
    let user_w = instances
        .iter()
        .map(|i| i.username.len())
        .max()
        .unwrap_or(8)
        .max(8);

    let mut lines = vec![format!(
        "  {:<name_w$}  {:<url_w$}  {:<user_w$}  {:<15}",
        "NAME", "URL", "USERNAME", "TRAFFIC POLICY"
    )];
    for inst in instances {
        lines.push(format!(
            "  {:<name_w$}  {:<url_w$}  {:<user_w$}  {:<15}",
            inst.name,
            inst.url,
            inst.username,
            format!("{:?}", inst.client_traffic_policy),
        ));
    }
    lines.join("\n")
}

pub struct InstanceShowResult {
    pub inst: Instance<Id>,
}

impl CommandOutput for InstanceShowResult {
    fn human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Name:           {}", self.inst.name));
        lines.push(format!("UUID:           {}", self.inst.uuid));
        lines.push(format!("URL:            {}", self.inst.url));
        lines.push(format!("Proxy URL:      {}", self.inst.proxy_url));
        lines.push(format!("Username:       {}", self.inst.username));
        lines.push(format!(
            "Traffic policy: {:?}",
            self.inst.client_traffic_policy
        ));
        if let Some(ref display_name) = self.inst.openid_display_name {
            lines.push(format!("OIDC display:   {display_name}"));
        }
        lines.join("\n")
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.inst.name,
            "uuid": self.inst.uuid,
            "url": self.inst.url,
            "proxy_url": self.inst.proxy_url,
            "username": self.inst.username,
            "traffic_policy": format!("{:?}", self.inst.client_traffic_policy),
            "openid_display_name": self.inst.openid_display_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_instance(name: &str) -> Instance<Id> {
        Instance {
            id: 1,
            name: name.to_string(),
            uuid: "uuid-1".to_string(),
            url: "https://vpn.example.com".to_string(),
            proxy_url: "https://proxy.example.com".to_string(),
            username: "admin".to_string(),
            token: None,
            client_traffic_policy:
                defguard_core::database::models::instance::ClientTrafficPolicy::None,
            enterprise_enabled: false,
            openid_display_name: None,
        }
    }

    #[test]
    fn test_list_human_empty() {
        let result = InstanceListResult { instances: vec![] };
        assert_eq!(result.human(), "No instances configured.");
    }

    #[test]
    fn test_list_human_with_data() {
        let result = InstanceListResult {
            instances: vec![make_instance("acme")],
        };
        let s = result.human();
        assert!(s.contains("acme"));
        assert!(s.contains("https://vpn.example.com"));
        assert!(s.contains("admin"));
    }

    #[test]
    fn test_show_human() {
        let result = InstanceShowResult {
            inst: make_instance("acme"),
        };
        let s = result.human();
        assert!(s.contains("Name:           acme"));
        assert!(s.contains("URL:            https://vpn.example.com"));
        assert!(s.contains("Username:       admin"));
    }

    #[test]
    fn test_exit_code_zero() {
        assert_eq!(InstanceListResult { instances: vec![] }.exit_code(), 0);
        assert_eq!(
            InstanceShowResult {
                inst: make_instance("x"),
            }
            .exit_code(),
            0
        );
    }
}
