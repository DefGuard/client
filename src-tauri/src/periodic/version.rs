use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::{appstate::AppState, commands::get_latest_app_version, database::Settings};

const INTERVAL_IN_SECONDS: Duration = Duration::from_secs(12 * 60 * 60); // 12 hours

pub async fn check_version(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let pool = &state.get_pool();

    loop {
        debug!("Waiting to fetch latest application version");
        sleep(INTERVAL_IN_SECONDS).await;

        let settings = Settings::get(pool).await;

        if let Ok(settings) = settings {
            if settings.check_for_updates {
                let response = get_latest_app_version(app_handle.clone()).await;

                if let Ok(result) = response {
                    debug!("Fetched latest application version info: {result:?}");

                    let _ = app_handle.emit_all("app-version-fetch", &result);
                } else {
                    let err = response.err().unwrap();
                    error!("Error while fetching latest application version: {err}");
                }
            } else {
                debug!("Checking for updates is turned off. Skipping latest application version fetch.");
            }
        } else {
            let err = settings.err().unwrap();
            error!("Error while fetching settings: {err}");
        }
    }
}
