use self::{
    connection::verify_active_connections, purge_stats::purge_stats, version::poll_version,
};
use tauri::AppHandle;
use tokio::select;

use crate::enterprise::periodic::config::poll_config;

pub mod connection;
pub mod purge_stats;
pub mod version;

/// Runs all the client periodic tasks, finishing when any of them returns.
pub async fn run_periodic_tasks(app_handle: &AppHandle) {
    debug!(
        "Starting periodic tasks (config, version polling, stats purging and active connection verification)..."
    );
    select! {
        () = poll_version(app_handle.clone()) => {
            error!("Version polling task has stopped unexpectedly");
        }
        () = poll_config(app_handle.clone()) => {
            error!("Config polling task has stopped unexpectedly");
        }
        () = verify_active_connections(app_handle.clone()) => {
            error!("Active connection verification task has stopped unexpectedly");
        }
        () = purge_stats(app_handle.clone()) => {
            error!("Stats purging task has stopped unexpectedly");
        }
    };
}
