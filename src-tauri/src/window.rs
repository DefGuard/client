use tauri::{
    webview::WebviewWindowBuilder, AppHandle, LogicalPosition, Manager, Monitor, Position,
    WebviewUrl, WebviewWindow,
};

use crate::appstate::AppState;

pub const NEW_UI_WINDOW_ID: &str = "new-ui";
pub const OLD_UI_WINDOW_ID: &str = "old-ui";
pub const NEW_UI_WIDTH: f64 = 360.0;
pub const NEW_UI_HEIGHT: f64 = 675.0;
pub const OLD_UI_WIDTH: f64 = 920.0;
pub const OLD_UI_HEIGHT: f64 = 720.0;

fn new_ui_url() -> WebviewUrl {
    if cfg!(defguard_client_dev) {
        WebviewUrl::External("http://localhost:5072".parse().unwrap())
    } else {
        WebviewUrl::App("new-ui/".into())
    }
}

fn old_ui_url() -> WebviewUrl {
    if cfg!(defguard_client_dev) {
        WebviewUrl::External("http://localhost:5071".parse().unwrap())
    } else {
        WebviewUrl::App("old-ui/index.html".into())
    }
}

/// Try to get monitor at the given position, with a fall back to primary monitor, and then to the
/// first one on the list of available monitors.
fn get_monitor_for_position(app: &AppHandle, x: f64, y: f64) -> Option<Monitor> {
    if let Ok(Some(monitor)) = app.monitor_from_point(x, y) {
        return Some(monitor);
    }

    if let Ok(Some(monitor)) = app.primary_monitor() {
        return Some(monitor);
    }

    // On macOS, it seems this is the only working method (as of Tauri 2.11), but fortunately it
    // returns the current monitor as the first one.
    if let Ok(mut monitors) = app.available_monitors() {
        monitors.pop()
    } else {
        None
    }
}

fn get_tray_window_position(
    app: &AppHandle,
    width: f64,
    height: f64,
) -> Option<LogicalPosition<f64>> {
    let app_state = app.state::<AppState>();
    let tray_position = app_state.tray_click_position.lock().unwrap().to_owned()?;

    let monitor = get_monitor_for_position(app, tray_position.x, tray_position.y)?;

    let scale_factor = monitor.scale_factor();
    let monitor_position = monitor.position().to_logical::<f64>(scale_factor);
    let monitor_size = monitor.size().to_logical::<f64>(scale_factor);
    let tray_position = tray_position.to_logical::<f64>(scale_factor);

    let mut x = tray_position.x - (width / 2.0);
    let center_y = monitor_position.y + (monitor_size.height / 2.0);
    let mut y = if tray_position.y < center_y {
        tray_position.y
    } else {
        tray_position.y - height
    };

    x = x.clamp(
        monitor_position.x,
        monitor_position.x + monitor_size.width - width,
    );
    y = y.clamp(
        monitor_position.y,
        monitor_position.y + monitor_size.height - height,
    );

    Some(LogicalPosition::new(x, y))
}

fn position_window_near_tray(app: &AppHandle, window: &WebviewWindow, width: f64, height: f64) {
    if let Some(position) = get_tray_window_position(app, width, height) {
        if let Err(err) = window.set_position(Position::Logical(position)) {
            warn!("Failed to position window near tray icon: {err}");
        }
    }
}

fn show_new_ui_window_internal(app: &AppHandle, near_tray: bool) {
    let window = if let Some(window) = app.get_webview_window(NEW_UI_WINDOW_ID) {
        let _ = window.unminimize();
        window
    } else {
        WebviewWindowBuilder::new(app, NEW_UI_WINDOW_ID, new_ui_url())
            .title("New UI")
            .inner_size(NEW_UI_WIDTH, NEW_UI_HEIGHT)
            .build()
            .unwrap()
    };
    if near_tray {
        position_window_near_tray(app, &window, NEW_UI_WIDTH, NEW_UI_HEIGHT);
    }
    #[cfg(target_os = "macos")]
    let _ = app.show();
    let _ = window.show();
    let _ = window.set_focus();
}

pub(crate) fn show_new_ui_window(app: &AppHandle) {
    show_new_ui_window_internal(app, false);
}

pub(crate) fn show_new_ui_window_near_tray(app: &AppHandle) {
    show_new_ui_window_internal(app, true);
}

#[tauri::command]
pub fn open_new_ui_window(app: AppHandle) {
    show_new_ui_window(&app);
}

#[tauri::command]
pub fn open_old_ui_window(app: AppHandle) {
    let _window = WebviewWindowBuilder::new(&app, OLD_UI_WINDOW_ID, old_ui_url())
        .title("Old UI")
        .inner_size(OLD_UI_WIDTH, OLD_UI_HEIGHT)
        .build()
        .unwrap();
}

#[tauri::command]
pub fn swap_to_old_ui(app: AppHandle) {
    WebviewWindowBuilder::new(&app, OLD_UI_WINDOW_ID, old_ui_url())
        .title("Old UI")
        .inner_size(OLD_UI_WIDTH, OLD_UI_HEIGHT)
        .build()
        .unwrap();
    if let Some(w) = app.get_webview_window(NEW_UI_WINDOW_ID) {
        w.close().unwrap();
    }
}

#[tauri::command]
pub fn swap_to_new_ui(app: AppHandle) {
    show_new_ui_window(&app);
    if let Some(w) = app.get_webview_window(OLD_UI_WINDOW_ID) {
        w.close().unwrap();
    }
}
