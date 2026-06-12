use defguard_core::database::models::{tunnel::Tunnel, Id};
use serde_json::json;

use crate::{
    output::CommandOutput,
    state::{CliError, State},
};

const MIN_NAME_COL_WIDTH: usize = 4;
const MIN_ADDR_COL_WIDTH: usize = 7;
const MIN_ENDPOINT_COL_WIDTH: usize = 8;

pub async fn handle_list(state: &State) -> Result<TunnelListResult, CliError> {
    let tunnels = Tunnel::all(&state.pool).await?;
    Ok(TunnelListResult { tunnels })
}

pub async fn handle_show(state: &State, name: &str) -> Result<TunnelShowResult, CliError> {
    let tunnels = Tunnel::find_by_name(&state.pool, name).await?;
    let tunnel = match tunnels.len() {
        0 => {
            return Err(CliError::NotFound(format!("Tunnel '{name}' not found")));
        }
        1 => tunnels
            .into_iter()
            .next()
            .expect("exactly one tunnel expected after length check"),
        _ => {
            return Err(CliError::NotFound(format!(
                "Multiple tunnels named '{name}'"
            )));
        }
    };
    Ok(TunnelShowResult { tunnel })
}

pub struct TunnelListResult {
    pub tunnels: Vec<Tunnel<Id>>,
}

impl CommandOutput for TunnelListResult {
    fn human(&self) -> String {
        if self.tunnels.is_empty() {
            "No tunnels configured.".to_string()
        } else {
            format_tunnel_list_table(&self.tunnels)
        }
    }

    fn json(&self) -> serde_json::Value {
        let tunnels: Vec<serde_json::Value> = self
            .tunnels
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "address": t.address,
                    "endpoint": t.endpoint,
                    "route_all_traffic": t.route_all_traffic,
                })
            })
            .collect();
        json!({ "tunnels": tunnels })
    }
}

fn format_tunnel_list_table(tunnels: &[Tunnel<Id>]) -> String {
    let name_col_width = tunnels
        .iter()
        .map(|t| t.name.len())
        .max()
        .unwrap_or(MIN_NAME_COL_WIDTH)
        .max(MIN_NAME_COL_WIDTH);
    let addr_col_width = tunnels
        .iter()
        .map(|t| t.address.len())
        .max()
        .unwrap_or(MIN_ADDR_COL_WIDTH)
        .max(MIN_ADDR_COL_WIDTH);
    let endpoint_col_width = tunnels
        .iter()
        .map(|t| t.endpoint.len())
        .max()
        .unwrap_or(MIN_ENDPOINT_COL_WIDTH)
        .max(MIN_ENDPOINT_COL_WIDTH);

    let mut lines = vec![format!(
        "  {:<name_col_width$}  {:<addr_col_width$}  {:<endpoint_col_width$}  {:>11}",
        "NAME", "ADDRESS", "ENDPOINT", "Routing"
    )];
    for tunnel in tunnels {
        lines.push(format!(
            "  {:<name_col_width$}  {:<addr_col_width$}  {:<endpoint_col_width$}  {:>11}",
            tunnel.name,
            tunnel.address,
            tunnel.endpoint,
            if tunnel.route_all_traffic {
                "All-traffic"
            } else {
                "Predefined"
            }
        ));
    }
    lines.join("\n")
}

pub struct TunnelShowResult {
    pub tunnel: Tunnel<Id>,
}

impl CommandOutput for TunnelShowResult {
    fn human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Name:                {}", self.tunnel.name));
        lines.push(format!("Address:             {}", self.tunnel.address));
        lines.push(format!("Endpoint:            {}", self.tunnel.endpoint));
        lines.push(format!("Pubkey:              {}", self.tunnel.pubkey));
        lines.push(format!(
            "Server pubkey:       {}",
            self.tunnel.server_pubkey
        ));
        if let Some(ref allowed) = self.tunnel.allowed_ips {
            lines.push(format!("Allowed IPs:         {allowed}"));
        }
        if let Some(ref dns) = self.tunnel.dns {
            lines.push(format!("DNS:                 {dns}"));
        }
        lines.push(format!(
            "Route all traffic:   {}",
            self.tunnel.route_all_traffic
        ));
        lines.push(format!(
            "Persistent keepalive: {}",
            self.tunnel.persistent_keep_alive
        ));
        if let Some(ref pre_up) = self.tunnel.pre_up {
            lines.push(format!("Pre-up:              {pre_up}"));
        }
        if let Some(ref post_up) = self.tunnel.post_up {
            lines.push(format!("Post-up:             {post_up}"));
        }
        if let Some(ref pre_down) = self.tunnel.pre_down {
            lines.push(format!("Pre-down:            {pre_down}"));
        }
        if let Some(ref post_down) = self.tunnel.post_down {
            lines.push(format!("Post-down:           {post_down}"));
        }
        lines.join("\n")
    }

    fn json(&self) -> serde_json::Value {
        json!({
            "name": self.tunnel.name,
            "address": self.tunnel.address,
            "endpoint": self.tunnel.endpoint,
            "pubkey": self.tunnel.pubkey,
            "server_pubkey": self.tunnel.server_pubkey,
            "allowed_ips": self.tunnel.allowed_ips,
            "dns": self.tunnel.dns,
            "route_all_traffic": self.tunnel.route_all_traffic,
            "persistent_keep_alive": self.tunnel.persistent_keep_alive,
            "pre_up": self.tunnel.pre_up,
            "post_up": self.tunnel.post_up,
            "pre_down": self.tunnel.pre_down,
            "post_down": self.tunnel.post_down,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tunnel(name: &str) -> Tunnel<Id> {
        Tunnel {
            id: 1,
            name: name.to_string(),
            pubkey: "pk".to_string(),
            prvkey: "sk".to_string(),
            address: "10.0.0.0/24".to_string(),
            server_pubkey: "spk".to_string(),
            preshared_key: None,
            allowed_ips: Some("0.0.0.0/0".to_string()),
            endpoint: "1.2.3.4:51820".to_string(),
            dns: Some("8.8.8.8".to_string()),
            persistent_keep_alive: 25,
            route_all_traffic: false,
            pre_up: None,
            post_up: None,
            pre_down: None,
            post_down: None,
        }
    }

    #[test]
    fn test_list_human_empty() {
        let result = TunnelListResult {
            tunnels: Vec::new(),
        };
        assert_eq!(result.human(), "No tunnels configured.");
    }

    #[test]
    fn test_list_human_with_data() {
        let result = TunnelListResult {
            tunnels: vec![make_tunnel("gateway")],
        };
        let s = result.human();
        assert!(s.contains("gateway"));
        assert!(s.contains("10.0.0.0/24"));
        assert!(s.contains("1.2.3.4:51820"));
    }

    #[test]
    fn test_show_human() {
        let result = TunnelShowResult {
            tunnel: make_tunnel("gateway"),
        };
        let s = result.human();
        assert!(s.contains("Name:                gateway"));
        assert!(s.contains("Address:             10.0.0.0/24"));
        assert!(s.contains("Endpoint:            1.2.3.4:51820"));
        assert!(s.contains("DNS:                 8.8.8.8"));
    }

    #[test]
    fn test_show_human_no_dns() {
        let mut tun = make_tunnel("gateway");
        tun.dns = None;
        tun.allowed_ips = None;
        let result = TunnelShowResult { tunnel: tun };
        let s = result.human();
        assert!(!s.contains("DNS"));
        assert!(!s.contains("Allowed IPs"));
    }

    #[test]
    fn test_exit_code_zero() {
        assert_eq!(
            TunnelListResult {
                tunnels: Vec::new()
            }
            .exit_code(),
            0
        );
        assert_eq!(
            TunnelShowResult {
                tunnel: make_tunnel("x"),
            }
            .exit_code(),
            0
        );
    }
}
