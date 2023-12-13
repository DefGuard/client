use serde::Serialize;
use struct_patch::Patch;
use strum::{AsRefStr, EnumString};
use tauri::{AppHandle, Event, Manager, State};

use crate::{
    appstate::AppState,
    database::{models::settings::SettingsPatch, Settings},
    tray::configure_tray_icon,
};

#[derive(Debug, Serialize, EnumString, AsRefStr)]
pub enum AppEvent {
    ConfigChange,
}

pub fn handle_config_change_event(event: Event, app: &AppHandle) {
    let payload = event.payload();
    if let Some(payload_str) = payload {
        if let Ok(settings_patch) = serde_json::from_str::<SettingsPatch>(payload_str) {
            tauri::async_runtime::spawn(async move {
                let app_state: State<AppState> = app.state();
                let pool = app_state.get_pool();
                if let Ok(mut settigs) = Settings::get(&pool).await {
                    settigs.apply(settings_patch);
                    settigs.save(&pool).await;
                    // reconfigure app
                    configure_tray_icon(&app, &settigs.tray_icon_theme);
                }
            });
        }
    }
}
