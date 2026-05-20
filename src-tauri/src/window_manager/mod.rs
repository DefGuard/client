use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

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
pub const WINDOW_GAP: f64 = 20.0;

#[must_use]
pub fn new_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev, debug_assertions)) {
        WebviewUrl::External("http://localhost:5072".parse().unwrap())
    } else {
        WebviewUrl::App("new-ui/".into())
    }
}

#[must_use]
pub fn old_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev, debug_assertions)) {
        WebviewUrl::External("http://localhost:5071".parse().unwrap())
    } else {
        WebviewUrl::App("old-ui/index.html".into())
    }
}

pub struct WindowManager;

impl WindowManager {
    pub fn build_tray_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        WebviewWindowBuilder::new(app, NEW_UI_WINDOW_ID, new_ui_url())
            .title("Defguard")
            .inner_size(NEW_UI_WIDTH, NEW_UI_HEIGHT)
            .resizable(false)
            .decorations(false)
            .visible(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .build()
    }

    pub fn build_full_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        WebviewWindowBuilder::new(app, OLD_UI_WINDOW_ID, old_ui_url())
            .title("Defguard")
            .inner_size(OLD_UI_WIDTH, OLD_UI_HEIGHT)
            .decorations(true)
            .build()
    }
}

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(target_os = "windows"))]
pub mod macos;

// Export tauri commands so they can be registered in main.rs
pub(crate) fn show_new_ui_window(app: &AppHandle) {
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
    tauri::async_runtime::spawn(async move {
        // Sleep briefly to let the IPC handler return
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        tracing::info!("swap_to_old_ui task: Opening full view");
        match WindowManager::open_full_view(&app) {
            Ok(_) => {
                tracing::info!(
                    "swap_to_old_ui task: open_full_view succeeded, sleeping before destroy"
                );
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                if let Some(w) = tauri::Manager::get_webview_window(&app, NEW_UI_WINDOW_ID) {
                    tracing::info!("swap_to_old_ui task: Destroying new-ui window");
                    if let Err(e) = w.destroy() {
                        tracing::error!(
                            "swap_to_old_ui task: Failed to destroy new-ui window: {:?}",
                            e
                        );
                    }
                } else {
                    tracing::warn!("swap_to_old_ui task: new-ui window not found to destroy");
                }
            }
            Err(e) => {
                tracing::error!("swap_to_old_ui task: open_full_view failed: {:?}", e);
            }
        }
    });
}

#[tauri::command]
pub fn close_tray_window(app: AppHandle) {
    tracing::info!("close_tray_window called");
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        if let Some(w) = tauri::Manager::get_webview_window(&app, NEW_UI_WINDOW_ID) {
            tracing::info!("close_tray_window task: Destroying new-ui window");
            if let Err(e) = w.destroy() {
                tracing::error!(
                    "close_tray_window task: Failed to destroy new-ui window: {:?}",
                    e
                );
            }
        } else {
            tracing::warn!("close_tray_window task: new-ui window not found to destroy");
        }
    });
}

#[tauri::command]
pub fn swap_to_new_ui(app: AppHandle) {
    tracing::info!("swap_to_new_ui called");
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        tracing::info!("swap_to_new_ui task: Showing new UI window");
        show_new_ui_window(&app);
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        if let Some(w) = tauri::Manager::get_webview_window(&app, OLD_UI_WINDOW_ID) {
            tracing::info!("swap_to_new_ui task: Destroying old-ui window");
            if let Err(e) = w.destroy() {
                tracing::error!(
                    "swap_to_new_ui task: Failed to destroy old-ui window: {:?}",
                    e
                );
            }
        } else {
            tracing::warn!("swap_to_new_ui task: old-ui window not found to destroy");
        }
    });
}
