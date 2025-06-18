use connection::verify_active_connections;
use tauri::AppHandle;
use tokio::select;
use version::poll_version;

use crate::enterprise::periodic::config::poll_config;

pub mod connection;
pub mod version;

/// Runs all the client periodic tasks, finishing when any of them returns.
pub async fn run_periodic_tasks(app_handle: &AppHandle) {
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
    };
}
