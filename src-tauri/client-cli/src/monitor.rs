use chrono::Utc;
use defguard_core::connection::active_state::{active_state, ActiveConnectionInfo};
use tracing::error;

use crate::state::State;

fn is_stale(connection: &ActiveConnectionInfo, peer_alive_period: u32) -> Option<bool> {
    let last_handshake = connection.stats.as_ref()?.last_handshake?;
    let now: u64 = Utc::now().timestamp().try_into().ok()?;

    Some(now.saturating_sub(last_handshake) > u64::from(peer_alive_period))
}

pub async fn monitor(state: &State) -> Result<(), defguard_core::error::Error> {
    let connections = active_state(&state.pool).await?;
    if connections.is_empty() {
        return Ok(());
    }
    error!(state.app_config.peer_alive_period);
    for connection in connections {
        if is_stale(&connection, state.app_config.peer_alive_period).is_some_and(|v| v) {
        // if is_stale(&connection, 10).is_some_and(|v| v) {
            let result;
            #[cfg(not(target_os = "macos"))]
            {
                use defguard_core::connection::tear_down;

                result = tear_down(&connection).await;
            }
            #[cfg(target_os = "macos")]
            {
                result = macos_tear_down(connection.clone()).await;
            }
            if let Err(err) = result {
                error!("Error removing stale connection {}: {err}", connection.name);
            }
        }
    }
    Ok(())
}
