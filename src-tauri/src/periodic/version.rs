use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::{
    app_config::AppConfig, appstate::AppState, commands::get_latest_app_version,
    events::APP_VERSION_FETCH,
};

const INTERVAL_IN_SECONDS: Duration = Duration::from_secs(12 * 60 * 60); // 12 hours

pub async fn poll_version(app_handle: AppHandle) {
    debug!("Starting the latest application version polling loop...");
    let state = app_handle.state::<AppState>();

    loop {
        debug!("Waiting to fetch latest application version for {INTERVAL_IN_SECONDS:?}...");
        sleep(INTERVAL_IN_SECONDS).await;

        let config_option: Option<AppConfig> = match state.app_config.lock() {
            Ok(guard) => Some(guard.clone()),
            Err(e) => {
                warn!(
                    "Check for updates: Could not lock app config mutex guard. Reason: {} Waiting for next loop.", e.to_string()
                );
                None
            }
        };
        if let Some(app_config) = config_option {
            if app_config.check_for_updates {
                let response = get_latest_app_version(app_handle.clone()).await;
                if let Ok(result) = response {
                    debug!("Fetched latest application version info: {result:?}");
                    let _ = app_handle.emit_all(APP_VERSION_FETCH, &result);
                } else {
                    let err = response.err().unwrap();
                    error!("Error while fetching latest application version: {err}");
                }
            } else {
                debug!("Checking for updates is turned off. Skipping latest application version fetch.");
            }
        };
    }
}
