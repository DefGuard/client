use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const NEW_UI_WINDOW_ID: &str = "new-ui";
pub const OLD_UI_WINDOW_ID: &str = "old-ui";
pub const NEW_UI_WIDTH: f64 = 380.0;
pub const NEW_UI_HEIGHT: f64 = 640.0;
pub const OLD_UI_WIDTH: f64 = 1280.0;
pub const OLD_UI_HEIGHT: f64 = 920.0;
pub const WINDOW_GAP: f64 = 20.0;

#[cfg(windows)]
pub const TASKBAR_HEIGHT: f64 = 48.0;
#[cfg(not(windows))]
pub const TASKBAR_HEIGHT: f64 = 0.0;

#[must_use]
pub fn new_ui_url() -> WebviewUrl {
    if cfg!(defguard_client_dev) {
        WebviewUrl::External("http://localhost:5072".parse().unwrap())
    } else {
        WebviewUrl::App("new-ui/".into())
    }
}

#[must_use]
pub fn old_ui_url() -> WebviewUrl {
    if cfg!(defguard_client_dev) {
        WebviewUrl::External("http://localhost:5071".parse().unwrap())
    } else {
        WebviewUrl::App("old-ui/index.html".into())
    }
}

pub struct WindowManager;

impl WindowManager {
    pub fn build_tray_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        WebviewWindowBuilder::new(app, NEW_UI_WINDOW_ID, new_ui_url())
            .title("New UI")
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
            .title("Old UI")
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
    let _ = WindowManager::open_tray(app, 0, 0, 0, 0);
}

pub(crate) fn show_new_ui_window_near_tray(app: &AppHandle) {
    let _ = WindowManager::open_tray(app, 0, 0, 0, 0);
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
    let _ = WindowManager::open_full_view(&app);
    if let Some(w) = tauri::Manager::get_webview_window(&app, NEW_UI_WINDOW_ID) {
        w.close().unwrap();
    }
}

#[tauri::command]
pub fn swap_to_new_ui(app: AppHandle) {
    show_new_ui_window(&app);
    if let Some(w) = tauri::Manager::get_webview_window(&app, OLD_UI_WINDOW_ID) {
        w.close().unwrap();
    }
}
