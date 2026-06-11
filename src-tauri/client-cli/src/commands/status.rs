use defguard_core::connection::active_state::{active_state, ActiveConnectionInfo};

use crate::{
    output::{self, ActiveEntry, StatusOutput},
    state::{CliError, State},
};

pub async fn handle(state: &State, json: bool) -> Result<(), CliError> {
    let connections = active_state(&state.pool).await?;

    // Build typed entries.
    let entries: Vec<ActiveEntry> = connections
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
        .collect();

    let message = if connections.is_empty() {
        "No active connections.".to_string()
    } else {
        format_status_table(&connections)
    };

    output::emit(
        &StatusOutput {
            active: entries,
            message,
        },
        json,
    );

    Ok(())
}

/// Build a human-readable status table string.
fn format_status_table(connections: &[ActiveConnectionInfo]) -> String {
    let name_w = connections
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let iface_w = connections
        .iter()
        .map(|c| c.interface_name.len())
        .max()
        .unwrap_or(9)
        .max(9);

    let mut lines = vec![format!("\nActive Connections")];
    lines.push(format!(
        "  {:<name_w$}  TYPE       {:<iface_w$}  TX          RX          {:<9}",
        "NAME", "INTERFACE", "HANDSHAKE"
    ));

    for conn in connections {
        let tx = conn
            .stats
            .as_ref()
            .map(|s| format_bytes(s.tx_bytes))
            .unwrap_or_else(|| "-".to_string());
        let rx = conn
            .stats
            .as_ref()
            .map(|s| format_bytes(s.rx_bytes))
            .unwrap_or_else(|| "-".to_string());
        let handshake = conn
            .stats
            .as_ref()
            .and_then(|s| s.last_handshake)
            .filter(|&s| s != 0)
            .map(format_handshake)
            .unwrap_or_else(|| "never".to_string());

        lines.push(format!(
            "  {:<name_w$}  {:<10}  {:<iface_w$}  {:<10}  {:<10}  {handshake:<9}",
            conn.name,
            conn.connection_type.to_string(),
            conn.interface_name,
            tx,
            rx
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
    let elapsed = match now.duration_since(then) {
        Ok(d) => d,
        Err(_) => return "now".to_string(),
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
