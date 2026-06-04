use defguard_core::connection::active_state::{active_state, ActiveConnectionInfo};
use serde::Serialize;

use crate::{
    output,
    state::{CliError, State},
};

/// Renderable active-connection entry.
#[derive(Serialize)]
struct ActiveEntry {
    /// "location" or "tunnel"
    connection_type: String,
    name: String,
    interface: String,
    listen_port: Option<u32>,
    tx_bytes: Option<u64>,
    rx_bytes: Option<u64>,
    last_handshake_secs: Option<u64>,
}

impl From<&ActiveConnectionInfo> for ActiveEntry {
    fn from(info: &ActiveConnectionInfo) -> Self {
        ActiveEntry {
            connection_type: info.connection_type.to_string(),
            name: info.name.clone(),
            interface: info.interface_name.clone(),
            listen_port: info.stats.as_ref().map(|s| s.listen_port),
            tx_bytes: info.stats.as_ref().map(|s| s.tx_bytes),
            rx_bytes: info.stats.as_ref().map(|s| s.rx_bytes),
            last_handshake_secs: info.stats.as_ref().and_then(|s| s.last_handshake),
        }
    }
}

pub async fn handle(state: &State, json: bool) -> Result<(), CliError> {
    let connections = active_state(&state.pool)
        .await
        .map_err(|err| CliError::Other(format!("Failed to query active state: {err}")))?;

    if connections.is_empty() {
        if json {
            output::emit(&serde_json::json!({ "active": [] }), json);
        } else {
            println!("No active connections.");
        }
    } else {
        let entries: Vec<ActiveEntry> = connections.iter().map(ActiveEntry::from).collect();
        output::emit(&serde_json::json!({ "active": entries }), json);
    }

    Ok(())
}
