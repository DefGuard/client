use chrono::{TimeDelta, Utc};
use defguard_core::{
    connection::{
        active_connections::ACTIVE_CONNECTIONS,
        active_state::{active_state, ActiveConnectionInfo},
    },
    ConnectionType,
};
use tracing::error;

use crate::state::State;

fn is_stale(connection: &ActiveConnectionInfo, peer_alive_period: u32) -> Option<bool> {
    let Some(stats) = connection.stats else {
        return None;
    };
    let Some(last_handshake) = stats.last_handshake else {
        return None;
    }

    let last_handshake = connection.stats?.last_handshake?;

    Some(connection.stats?.last_handshake? > peer_alive_period as u64)
}

pub async fn monitor(state: &State) -> Result<(), defguard_core::error::Error> {
    let connections = active_state(&state.pool).await?;
    if connections.is_empty() {
        return Ok(());
    }
    for connection in connections {
        if is_stale(&connection, state.app_config.peer_alive_period).is_some_and(|v| v) {
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
