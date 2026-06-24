use defguard_core::connection::active_state::{active_state, ActiveConnectionInfo};
use serde_json::{json, Value};

use crate::{
    output::{ActiveEntry, CommandOutput},
    state::{CliError, State},
};

const MIN_NAME_COL_WIDTH: usize = 4;
const MIN_IFACE_COL_WIDTH: usize = 9;

pub(crate) async fn handle(state: &State) -> Result<StatusResult, CliError> {
    let connections = active_state(&state.pool).await?;
    Ok(StatusResult { connections })
}

pub struct StatusResult {
    pub connections: Vec<ActiveConnectionInfo>,
}

impl CommandOutput for StatusResult {
    fn human(&self) -> String {
        if self.connections.is_empty() {
            "No active connections.".to_string()
        } else {
            format_status_table(&self.connections)
        }
    }

    fn json(&self) -> Value {
        let active = self
            .connections
            .iter()
            .map(|c| ActiveEntry {
                connection_type: c.connection_type.to_string(),
                name: c.name.clone(),
                interface: c.interface_name.clone(),
                listen_port: c.stats.as_ref().map(|s| s.listen_port),
                tx_bytes: c.stats.as_ref().map(|s| s.tx_bytes),
                rx_bytes: c.stats.as_ref().map(|s| s.rx_bytes),
                last_handshake_secs: c.stats.as_ref().and_then(|s| s.last_handshake),
            })
            .collect::<Vec<_>>();
        json!({ "active": active })
    }
}

/// Build a human-readable status table string.
fn format_status_table(connections: &[ActiveConnectionInfo]) -> String {
    let name_col_width = connections
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(MIN_NAME_COL_WIDTH)
        .max(MIN_NAME_COL_WIDTH);
    let iface_col_width = connections
        .iter()
        .map(|c| c.interface_name.len())
        .max()
        .unwrap_or(MIN_IFACE_COL_WIDTH)
        .max(MIN_IFACE_COL_WIDTH);

    let mut lines = vec![format!("\nActive Connections")];
    lines.push(format!(
        "  {:<name_col_width$}  TYPE       {:<iface_col_width$}  TX          RX          {:<9}",
        "NAME", "INTERFACE", "HANDSHAKE"
    ));

    for connection in connections {
        let tx = connection
            .stats
            .as_ref()
            .map_or_else(|| "-".to_string(), |s| format_bytes(s.tx_bytes));
        let rx = connection
            .stats
            .as_ref()
            .map_or_else(|| "-".to_string(), |s| format_bytes(s.rx_bytes));
        let handshake = connection
            .stats
            .as_ref()
            .and_then(|s| s.last_handshake)
            .filter(|&s| s != 0)
            .map_or_else(|| "never".to_string(), format_handshake);

        lines.push(format!(
            "  {:<name_col_width$}  {:<10}  {:<iface_col_width$}  {:<10}  {:<10}  {handshake:<9}",
            connection.name, connection.connection_type, connection.interface_name, tx, rx
        ));
    }

    lines.join("\n")
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit_idx])
    }
}

fn format_handshake(secs: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let then = UNIX_EPOCH + Duration::from_secs(secs);
    let now = SystemTime::now();
    let Ok(elapsed) = now.duration_since(then) else {
        return "now".to_string();
    };

    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

#[cfg(test)]
mod tests {
    use defguard_core::{connection::active_state::InterfaceStats, ConnectionType};

    use super::*;

    fn make_conn(
        name: &str,
        iface: &str,
        stats: Option<InterfaceStats>,
        conn_type: ConnectionType,
    ) -> ActiveConnectionInfo {
        ActiveConnectionInfo {
            connection_type: conn_type,
            target_id: 1,
            name: name.to_string(),
            interface_name: iface.to_string(),
            stats,
        }
    }

    #[test]
    fn test_human_empty() {
        let result = StatusResult {
            connections: Vec::new(),
        };
        let s = result.human();
        if cfg!(target_os = "macos") {
            assert!(s.contains("not yet supported on macOS"));
        } else {
            assert!(s.contains("No active connections"));
        }
    }

    #[test]
    fn test_human_with_connections() {
        let result = StatusResult {
            connections: vec![make_conn(
                "office",
                "wg0",
                Some(InterfaceStats {
                    listen_port: 51820,
                    tx_bytes: 1024,
                    rx_bytes: 2048,
                    last_handshake: Some(1700000000),
                }),
                ConnectionType::Location,
            )],
        };
        let s = result.human();
        if !cfg!(target_os = "macos") {
            assert!(s.contains("office"));
            assert!(s.contains("wg0"));
            assert!(s.contains("1.0 KiB"));
            assert!(s.contains("2.0 KiB"));
        }
    }

    #[test]
    fn test_json_empty() {
        let result = StatusResult {
            connections: Vec::new(),
        };
        let json = result.json();
        assert_eq!(json["active"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_json_with_connections() {
        let result = StatusResult {
            connections: vec![
                make_conn(
                    "office",
                    "wg0",
                    Some(InterfaceStats {
                        listen_port: 51820,
                        tx_bytes: 1024,
                        rx_bytes: 2048,
                        last_handshake: Some(1700000000),
                    }),
                    ConnectionType::Location,
                ),
                make_conn("data-center", "wg1", None, ConnectionType::Tunnel),
            ],
        };
        let json = result.json();
        let active = json["active"].as_array().unwrap();
        assert_eq!(active.len(), 2);

        assert_eq!(active[0]["name"], "office");
        assert_eq!(active[0]["connection_type"], "location");
        assert_eq!(active[0]["interface"], "wg0");
        assert_eq!(active[0]["listen_port"], 51820);
        assert_eq!(active[0]["tx_bytes"], 1024);
        assert_eq!(active[0]["rx_bytes"], 2048);
        assert_eq!(active[0]["last_handshake_secs"], 1700000000);

        assert_eq!(active[1]["name"], "data-center");
        assert_eq!(active[1]["connection_type"], "tunnel");
        assert_eq!(active[1]["interface"], "wg1");
        assert!(active[1]["listen_port"].is_null());
    }

    #[test]
    fn test_json_no_message_field() {
        let result = StatusResult {
            connections: Vec::new(),
        };
        let json = result.json();
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_exit_code_zero() {
        let result = StatusResult {
            connections: Vec::new(),
        };
        assert_eq!(result.exit_code(), 0);
    }
}
