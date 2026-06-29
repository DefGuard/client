use chrono::Utc;
use defguard_core::connection::active_state::{active_state, ActiveConnectionInfo};
use tracing::error;

use crate::state::State;

/// Determine whether a connection is stale based on its latest WireGuard handshake.
///
/// Returns `None` when live backend stats are unavailable or the connection has no
/// recorded handshake, because in that case the CLI cannot safely decide whether the
/// connection is stale.
fn is_stale(connection: &ActiveConnectionInfo, peer_alive_period: u32) -> Option<bool> {
    let last_handshake = connection.stats.as_ref()?.last_handshake?;
    let now: u64 = Utc::now().timestamp().try_into().ok()?;

    Some(now.saturating_sub(last_handshake) > u64::from(peer_alive_period))
}

/// Disconnect active connections whose latest handshake is older than the configured
/// peer alive period.
///
/// Connections without usable live stats are left untouched. Failures are logged and do
/// not stop cleanup of the remaining connections.
pub async fn tear_down_stale_connections(state: &State) {
    let connections = match active_state(&state.pool).await {
        Ok(connections) => connections,
        Err(err) => {
            error!("Failed to retrieve active connections: {err}");
            return;
        }
    };
    if connections.is_empty() {
        return;
    }

    for connection in connections {
        if is_stale(&connection, state.app_config.peer_alive_period).is_some_and(|v| v) {
            use defguard_core::connection::tear_down;

            let result = tear_down(&connection).await;
            if let Err(err) = result {
                error!("Error removing stale connection {}: {err}", connection.name);
            }
        }
    }
}
