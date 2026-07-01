#[cfg(not(target_os = "windows"))]
use tauri::Manager;
use tauri::{AppHandle, Emitter, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::{
    database::{models::location::Location, DB_POOL},
    events::EventKey,
};

/// Returns `true` if there are any non-service locations in the database.
pub async fn has_non_service_locations() -> bool {
    Location::exist(&*DB_POOL, false).await.unwrap_or_default()
}

pub const COMPACT_WINDOW_ID: &str = "compact-view";
pub const FULL_VIEW_WINDOW_ID: &str = "full-view";
pub const COMPACT_WINDOW_WIDTH: f64 = 380.0;
pub const COMPACT_WINDOW_HEIGHT: f64 = 640.0;
pub const FULL_VIEW_WINDOW_WIDTH: f64 = 800.0;
pub const FULL_VIEW_WINDOW_HEIGHT: f64 = 700.0;
#[cfg(not(target_os = "linux"))]
const WINDOW_GAP: f64 = 20.0;
const WINDOW_TITLE: &str = "Defguard";

#[must_use]
pub fn compact_view_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev)) {
        WebviewUrl::External("http://localhost:5072/compact/".parse().unwrap())
    } else {
        WebviewUrl::App("compact/".into())
    }
}

#[must_use]
pub fn full_view_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev)) {
        WebviewUrl::External("http://localhost:5072/full/".parse().unwrap())
    } else {
        WebviewUrl::App("full/".into())
    }
}

pub struct WindowManager;

impl WindowManager {
    pub fn build_tray_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = WebviewWindowBuilder::new(app, COMPACT_WINDOW_ID, compact_view_ui_url())
            .title(WINDOW_TITLE)
            .inner_size(COMPACT_WINDOW_WIDTH, COMPACT_WINDOW_HEIGHT)
            .resizable(false)
            .decorations(false)
            .visible(false)
            .always_on_top(true)
            .skip_taskbar(true);
        #[cfg(target_os = "macos")]
        let window = window.hidden_title(true);

        let window = window.build()?;

        #[cfg(target_os = "macos")]
        if let Err(err) = macos::enable_rounded_corners(&window) {
            warn!("Failed to enable rounded corners on tray window: {err}");
        }

        Ok(window)
    }

    pub fn build_full_view_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        WebviewWindowBuilder::new(app, FULL_VIEW_WINDOW_ID, full_view_ui_url())
            .title(WINDOW_TITLE)
            .inner_size(FULL_VIEW_WINDOW_WIDTH, FULL_VIEW_WINDOW_HEIGHT)
            .min_inner_size(FULL_VIEW_WINDOW_WIDTH, FULL_VIEW_WINDOW_HEIGHT)
            .decorations(cfg!(not(any(windows, target_os = "macos"))))
            .visible(false)
            .build()
    }
}

#[cfg(not(windows))]
impl WindowManager {
    pub fn open_tray(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = if let Some(window) = app.get_webview_window(COMPACT_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_tray_window(app)?
        };
        #[cfg(target_os = "macos")]
        macos::position_window_near_tray(app, &window);
        #[cfg(target_os = "macos")]
        let _ = app.set_dock_visibility(false);
        #[cfg(target_os = "macos")]
        let _ = app.show();
        let _ = window.show();
        let _ = window.set_focus();
        Ok(window)
    }

    pub fn open_full_view(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = if let Some(window) = app.get_webview_window(FULL_VIEW_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_full_view_window(app)?
        };
        #[cfg(target_os = "macos")]
        let _ = app.set_dock_visibility(true);
        #[cfg(target_os = "macos")]
        let _ = app.show();
        let _ = window.show();
        let _ = window.set_focus();
        Ok(window)
    }
}

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// Export tauri commands so they can be registered in main.rs
pub(crate) fn show_tray_window(app: &AppHandle) {
    let _ = WindowManager::open_tray(app);
}

#[tauri::command]
pub fn open_tray_window(app: AppHandle) {
    show_tray_window(&app);
}

#[tauri::command]
pub fn open_full_view_window(app: AppHandle) {
    let _ = WindowManager::open_full_view(&app);
}

#[tauri::command]
pub fn swap_to_full_view(app: AppHandle) {
    info!("swap_to_full_view called");
    if let Some(window) = tauri::Manager::get_webview_window(&app, COMPACT_WINDOW_ID) {
        if let Err(err) = window.hide() {
            error!("swap_to_full_view task: Failed to hide new-ui window: {err:?}");
        }
    }
    if let Err(err) = WindowManager::open_full_view(&app) {
        error!("swap_to_full_view task: Failed to open full view: {err:?}");
    } else if let Err(err) = app.emit(EventKey::WindowSwapped.into(), ()) {
        error!("swap_to_full_view task: Failed to emit window swapped event: {err:?}");
    }
}

#[tauri::command]
pub fn close_tray_window(app: AppHandle) {
    info!("close_tray_window called");

    if let Some(window) = tauri::Manager::get_webview_window(&app, COMPACT_WINDOW_ID) {
        info!("close_tray_window task: Hiding new-ui window");
        if let Err(err) = window.hide() {
            error!("close_tray_window task: Failed to hide new-ui window: {err:?}");
        }
    } else {
        warn!("close_tray_window task: new-ui window not found");
    }
}

#[tauri::command]
pub fn swap_to_tray(app: AppHandle) {
    info!("swap_to_tray called");
    show_tray_window(&app);
    if let Some(window) = tauri::Manager::get_webview_window(&app, FULL_VIEW_WINDOW_ID) {
        if let Err(err) = window.hide() {
            error!("swap_to_tray task: Failed to hide full-view window: {err:?}");
        }
    }
    if let Err(err) = app.emit(EventKey::WindowSwapped.into(), ()) {
        error!("swap_to_tray task: Failed to emit window swapped event: {err:?}");
    }
}
