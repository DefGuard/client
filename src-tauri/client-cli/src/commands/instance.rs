use defguard_core::database::models::{instance::Instance, Id};
use serde_json::{json, Value};

use crate::{
    output::CommandOutput,
    state::{CliError, State},
};

const MIN_NAME_COL_WIDTH: usize = 4;
const MIN_URL_COL_WIDTH: usize = 3;
const MIN_USER_COL_WIDTH: usize = 8;

pub async fn handle_list(state: &State) -> Result<InstanceListResult, CliError> {
    let instances = Instance::all(&state.pool).await?;
    Ok(InstanceListResult { instances })
}

pub async fn handle_show(state: &State, name: &str) -> Result<InstanceShowResult, CliError> {
    let instance = Instance::find_by_name(&state.pool, name)
        .await?
        .ok_or_else(|| CliError::NotFound(format!("Instance '{name}' not found")))?;
    Ok(InstanceShowResult { instance })
}

pub struct InstanceListResult {
    pub instances: Vec<Instance<Id>>,
}

impl CommandOutput for InstanceListResult {
    fn human(&self) -> String {
        if self.instances.is_empty() {
            "No instances configured. Use the desktop app to enroll first.".to_string()
        } else {
            format_instance_list_table(&self.instances)
        }
    }

    fn json(&self) -> Value {
        let instances = self
            .instances
            .iter()
            .map(|inst| {
                json!({
                    "name": inst.name,
                    "url": inst.url,
                    "username": inst.username,
                    "traffic_policy": format!("{:?}", inst.client_traffic_policy),
                })
            })
            .collect::<Vec<_>>();
        json!({ "instances": instances })
    }
}

fn format_instance_list_table(instances: &[Instance<Id>]) -> String {
    let name_col_width = instances
        .iter()
        .map(|i| i.name.len())
        .max()
        .unwrap_or(MIN_NAME_COL_WIDTH)
        .max(MIN_NAME_COL_WIDTH);
    let url_col_width = instances
        .iter()
        .map(|i| i.url.len())
        .max()
        .unwrap_or(MIN_URL_COL_WIDTH)
        .max(MIN_URL_COL_WIDTH);
    let user_col_width = instances
        .iter()
        .map(|i| i.username.len())
        .max()
        .unwrap_or(MIN_USER_COL_WIDTH)
        .max(MIN_USER_COL_WIDTH);

    let mut lines = vec![format!(
        "  {:<name_col_width$}  {:<url_col_width$}  {:<user_col_width$}  {:<15}",
        "NAME", "URL", "USERNAME", "TRAFFIC POLICY"
    )];
    for instance in instances {
        lines.push(format!(
            "  {:<name_col_width$}  {:<url_col_width$}  {:<user_col_width$}  {:<15}",
            instance.name,
            instance.url,
            instance.username,
            format!("{:?}", instance.client_traffic_policy),
        ));
    }
    lines.join("\n")
}

pub struct InstanceShowResult {
    pub instance: Instance<Id>,
}

impl CommandOutput for InstanceShowResult {
    fn human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Name:           {}", self.instance.name));
        lines.push(format!("UUID:           {}", self.instance.uuid));
        lines.push(format!("URL:            {}", self.instance.url));
        lines.push(format!("Proxy URL:      {}", self.instance.proxy_url));
        lines.push(format!("Username:       {}", self.instance.username));
        lines.push(format!(
            "Traffic policy: {:?}",
            self.instance.client_traffic_policy
        ));
        if let Some(ref display_name) = self.instance.openid_display_name {
            lines.push(format!("OIDC display:   {display_name}"));
        }
        lines.join("\n")
    }

    fn json(&self) -> Value {
        json!({
            "name": self.instance.name,
            "uuid": self.instance.uuid,
            "url": self.instance.url,
            "proxy_url": self.instance.proxy_url,
            "username": self.instance.username,
            "traffic_policy": format!("{:?}", self.instance.client_traffic_policy),
            "openid_display_name": self.instance.openid_display_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use defguard_core::database::models::instance::ClientTrafficPolicy;

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
            client_traffic_policy: ClientTrafficPolicy::None,
            enterprise_enabled: false,
            openid_display_name: None,
        }
    }

    #[test]
    fn test_list_human_empty() {
        let result = InstanceListResult {
            instances: Vec::new(),
        };
        assert_eq!(
            result.human(),
            "No instances configured. Use the desktop app to enroll first."
        );
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
            instance: make_instance("acme"),
        };
        let s = result.human();
        assert!(s.contains("Name:           acme"));
        assert!(s.contains("URL:            https://vpn.example.com"));
        assert!(s.contains("Username:       admin"));
    }

    #[test]
    fn test_exit_code_zero() {
        assert_eq!(
            InstanceListResult {
                instances: Vec::new()
            }
            .exit_code(),
            0
        );
        assert_eq!(
            InstanceShowResult {
                instance: make_instance("x"),
            }
            .exit_code(),
            0
        );
    }
}
