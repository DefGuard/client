use objc2_app_kit::{NSWindow, NSWindowButton, NSWindowStyleMask};
use objc2_foundation::NSPoint;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Monitor, Position, Runtime, WebviewWindow,
};

use crate::{appstate::AppState, window_manager::WINDOW_GAP};

/// Enforce rounded window corners. Not yet available in Tauri.
pub(crate) fn enable_rounded_corners<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), String> {
    window
        .with_webview(move |webview| {
            let ns_window = unsafe { &*webview.ns_window().cast::<NSWindow>() };
            // Add necessary styles for rounded corners.
            let style_mask = ns_window.styleMask()
                | NSWindowStyleMask::Borderless
                | NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable;
            ns_window.setStyleMask(style_mask);
            ns_window.setTitlebarAppearsTransparent(true);

            // Position traffic light buttons: 20px from left, 12px from top to comply with figma design
            let buttons = [
                (
                    ns_window.standardWindowButton(NSWindowButton::CloseButton),
                    20.0,
                ),
                (
                    ns_window.standardWindowButton(NSWindowButton::MiniaturizeButton),
                    40.0,
                ),
                (
                    ns_window.standardWindowButton(NSWindowButton::ZoomButton),
                    60.0,
                ),
            ];
            for (button, x) in buttons {
                if let Some(btn) = button {
                    let superview_height = unsafe {
                        btn.superview()
                            .map(|sv| sv.frame().size.height)
                            .unwrap_or(28.0)
                    };
                    let y = superview_height - 12.0 - btn.frame().size.height;
                    btn.setFrameOrigin(NSPoint::new(x, y));
                }
            }
        })
        .map_err(|err| err.to_string())
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
    window_size: LogicalSize<f64>,
) -> Option<LogicalPosition<f64>> {
    let app_state = app.state::<AppState>();
    let mut x;
    let mut y;

    if let Some(tray_position) = *app_state.tray_click_position.lock().unwrap() {
        let monitor = get_monitor_for_position(app, tray_position.x, tray_position.y)?;

        let scale_factor = monitor.scale_factor();
        let monitor_position = monitor.position().to_logical::<f64>(scale_factor);
        let monitor_size = monitor.size().to_logical::<f64>(scale_factor);
        let tray_position = tray_position.to_logical::<f64>(scale_factor);

        x = tray_position.x;
        y = tray_position.y;

        x = x.clamp(
            monitor_position.x,
            monitor_position.x + monitor_size.width - window_size.width,
        );
        y = y.clamp(
            monitor_position.y,
            monitor_position.y + monitor_size.height - window_size.height,
        );
    } else {
        let monitor = app.primary_monitor().ok().flatten()?;
        let scale_factor = monitor.scale_factor();
        let monitor_position = monitor.position().to_logical::<f64>(scale_factor);
        let monitor_size = monitor.size().to_logical::<f64>(scale_factor);

        x = monitor_position.x + monitor_size.width - window_size.width - WINDOW_GAP;
        y = monitor_position.y + WINDOW_GAP;
    }

    Some(LogicalPosition::new(x, y))
}

pub(super) fn position_window_near_tray(app: &AppHandle, window: &WebviewWindow) {
    let size = window.outer_size().unwrap_or_default();
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    if let Some(position) = get_tray_window_position(app, size.to_logical::<f64>(scale_factor)) {
        if let Err(err) = window.set_position(Position::Logical(position)) {
            warn!("Failed to position window near tray icon: {err}");
        }
    }
}
