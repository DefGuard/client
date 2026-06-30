use chrono::Utc;
use defguard_core::connection::{
    active_state::{active_state, ActiveConnectionInfo},
    tear_down,
};
use tracing::error;

use crate::state::State;

/// Determine whether a connection is stale based on its latest WireGuard handshake.
///
/// Returns `None` when live backend stats are unavailable or the connection has no
/// recorded handshake, because in that case the CLI cannot safely decide whether the
/// connection is stale.
fn is_stale(connection: &ActiveConnectionInfo, peer_alive_period: u32) -> bool {
    if let Some(stats) = connection.stats.as_ref() {
        if let Some(last_handshake) = stats.last_handshake {
            let now = Utc::now().timestamp() as u64;
            return now.saturating_sub(last_handshake) > u64::from(peer_alive_period);
        }
    }
    false
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

    let peer_alive_period = state.app_config.peer_alive_period;

    #[cfg(target_os = "macos")]
    {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };

        use defguard_core::connection::apple::spawn_runloop_and_wait_for;

        let semaphore = Arc::new(AtomicBool::new(false));
        let semaphore_clone = Arc::clone(&semaphore);
        let handle = tokio::spawn(async move {
            for connection in connections {
                if is_stale(&connection, peer_alive_period) {
                    let result = tear_down(&connection).await;
                    if let Err(err) = result {
                        error!("Error removing stale connection {}: {err}", connection.name);
                    }
                }
            }
            semaphore_clone.store(true, Ordering::Release);
        });
        spawn_runloop_and_wait_for(&semaphore);
        let _ = handle.await.unwrap();
    }

    #[cfg(not(target_os = "macos"))]
    {
        for connection in connections {
            if is_stale(&connection, peer_alive_period) {
                let result = tear_down(&connection).await;
                if let Err(err) = result {
                    error!("Error removing stale connection {}: {err}", connection.name);
                }
            }
        }
    }
}
