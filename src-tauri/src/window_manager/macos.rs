use tauri::{AppHandle, LogicalPosition, Manager, Monitor, PhysicalSize, Position, WebviewWindow};

use crate::appstate::AppState;
use crate::window_manager::{WindowManager, NEW_UI_WINDOW_ID, OLD_UI_WINDOW_ID, TASKBAR_HEIGHT};

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
    size: PhysicalSize<u32>,
) -> Option<LogicalPosition<f64>> {
    let app_state = app.state::<AppState>();
    let tray_position = app_state.tray_click_position.lock().unwrap().to_owned()?;

    let monitor = get_monitor_for_position(app, tray_position.x, tray_position.y)?;

    let scale_factor = monitor.scale_factor();
    let monitor_position = monitor.position().to_logical::<f64>(scale_factor);
    let monitor_size = monitor.size().to_logical::<f64>(scale_factor);
    let tray_position = tray_position.to_logical::<f64>(scale_factor);
    let window_size = size.to_logical::<f64>(scale_factor);

    let mut x = tray_position.x;
    let mut y = tray_position.y;

    x = x.clamp(
        monitor_position.x,
        monitor_position.x + monitor_size.width - window_size.width,
    );
    y = y.clamp(
        monitor_position.y,
        monitor_position.y + monitor_size.height - window_size.height - TASKBAR_HEIGHT,
    );

    Some(LogicalPosition::new(x, y))
}

fn position_window_near_tray(app: &AppHandle, window: &WebviewWindow) {
    let size = window.outer_size().unwrap_or_default();
    if let Some(position) = get_tray_window_position(app, size) {
        if let Err(err) = window.set_position(Position::Logical(position)) {
            warn!("Failed to position window near tray icon: {err}");
        }
    }
}

impl WindowManager {
    pub fn open_tray(
        app: &AppHandle,
        _icon_x: i32,
        _icon_y: i32,
        _icon_width: u32,
        _icon_height: u32,
    ) -> tauri::Result<WebviewWindow> {
        let window = if let Some(window) = app.get_webview_window(NEW_UI_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_tray_window(app)?
        };
        position_window_near_tray(app, &window);
        #[cfg(target_os = "macos")]
        let _ = app.show();
        let _ = window.show();
        let _ = window.set_focus();
        Ok(window)
    }

    pub fn open_full_view(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = if let Some(window) = app.get_webview_window(OLD_UI_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_full_window(app)?
        };
        #[cfg(target_os = "macos")]
        let _ = app.show();
        let _ = window.show();
        let _ = window.set_focus();
        Ok(window)
    }
}
