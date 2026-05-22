use std::time::Duration;

use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tokio::time::sleep;

#[cfg(not(target_os = "linux"))]
use crate::database::{models::location::Location, DB_POOL};

/// Returns `true` if there are any non-service locations in the database.
#[cfg(not(target_os = "linux"))]
pub async fn has_non_service_locations() -> bool {
    match Location::all(&*DB_POOL, false).await {
        Ok(locations) => !locations.is_empty(),
        Err(_) => false,
    }
}

pub const NEW_UI_WINDOW_ID: &str = "new-ui";
pub const OLD_UI_WINDOW_ID: &str = "old-ui";
pub const NEW_UI_WIDTH: f64 = 380.0;
pub const NEW_UI_HEIGHT: f64 = 640.0;
pub const OLD_UI_WIDTH: f64 = 1280.0;
pub const OLD_UI_HEIGHT: f64 = 920.0;
const WINDOW_GAP: f64 = 20.0;
const WINDOW_TITLE: &str = "Defguard";
// Sleep briefly to let the IPC handler return.
const UI_SWAP_DELAY: Duration = Duration::from_millis(50);

#[must_use]
pub fn new_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev)) {
        WebviewUrl::External("http://localhost:5072".parse().unwrap())
    } else {
        WebviewUrl::App("new-ui/".into())
    }
}

#[must_use]
pub fn old_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev)) {
        WebviewUrl::External("http://localhost:5071".parse().unwrap())
    } else {
        WebviewUrl::App("old-ui/index.html".into())
    }
}

pub struct WindowManager;

impl WindowManager {
    pub fn build_tray_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = WebviewWindowBuilder::new(app, NEW_UI_WINDOW_ID, new_ui_url())
            .title(WINDOW_TITLE)
            .inner_size(NEW_UI_WIDTH, NEW_UI_HEIGHT)
            .resizable(false)
            .visible(false)
            .always_on_top(true)
            .skip_taskbar(true);
        #[cfg(target_os = "macos")]
        let window = window.hidden_title(true);

        let window = window.build()?;

        #[cfg(target_os = "macos")]
        if let Err(err) = macos::enable_rounded_corners(&window) {
            tracing::warn!("Failed to enable rounded corners on tray window: {err}");
        }

        Ok(window)
    }

    pub fn build_full_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        WebviewWindowBuilder::new(app, OLD_UI_WINDOW_ID, old_ui_url())
            .title(WINDOW_TITLE)
            .inner_size(OLD_UI_WIDTH, OLD_UI_HEIGHT)
            .decorations(true)
            .visible(false)
            .build()
    }
}

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// Export tauri commands so they can be registered in main.rs
pub(crate) fn show_new_ui_window(app: &AppHandle) {
    #[cfg(not(target_os = "linux"))]
    let _ = WindowManager::open_tray(app);
}

pub(crate) fn show_new_ui_window_near_tray(app: &AppHandle) {
    show_new_ui_window(app);
}

#[tauri::command]
pub fn open_new_ui_window(app: AppHandle) {
    show_new_ui_window(&app);
}

#[tauri::command]
pub fn open_old_ui_window(app: AppHandle) {
    let _ = WindowManager::open_full_view(&app);
}

#[tauri::command]
pub fn swap_to_old_ui(app: AppHandle) {
    tracing::info!("swap_to_old_ui called");
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(true);
    tauri::async_runtime::spawn(async move {
        sleep(UI_SWAP_DELAY).await;
        if let Some(window) = tauri::Manager::get_webview_window(&app, NEW_UI_WINDOW_ID) {
            if let Err(err) = window.hide() {
                tracing::error!("swap_to_old_ui task: Failed to hide new-ui window: {err:?}");
            }
        }
        if let Err(err) = WindowManager::open_full_view(&app) {
            tracing::error!("swap_to_old_ui task: Failed to open full view: {err:?}");
        }
    });
}

#[tauri::command]
pub fn close_tray_window(app: AppHandle) {
    tracing::info!("close_tray_window called");
    tauri::async_runtime::spawn(async move {
        sleep(UI_SWAP_DELAY).await;
        if let Some(window) = tauri::Manager::get_webview_window(&app, NEW_UI_WINDOW_ID) {
            tracing::info!("close_tray_window task: Hiding new-ui window");
            if let Err(err) = window.hide() {
                tracing::error!("close_tray_window task: Failed to hide new-ui window: {err:?}");
            }
        } else {
            tracing::warn!("close_tray_window task: new-ui window not found");
        }
    });
}

#[tauri::command]
pub fn swap_to_new_ui(app: AppHandle) {
    tracing::info!("swap_to_new_ui called");
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(false);
    tauri::async_runtime::spawn(async move {
        sleep(UI_SWAP_DELAY).await;
        show_new_ui_window(&app);
        if let Some(window) = tauri::Manager::get_webview_window(&app, OLD_UI_WINDOW_ID) {
            if let Err(err) = window.hide() {
                tracing::error!("swap_to_new_ui task: Failed to hide old-ui window: {err:?}");
            }
        }
    });
}
