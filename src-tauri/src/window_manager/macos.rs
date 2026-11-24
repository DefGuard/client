use objc2_app_kit::{NSWindow, NSWindowButton, NSWindowStyleMask, NSWindowTitleVisibility};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Position, Runtime, WebviewWindow};

use crate::{
    appstate::AppState,
    window_manager::{monitor_for_point, WINDOW_GAP},
};

pub(crate) fn enable_rounded_corners<R: Runtime>(
    window: &WebviewWindow<R>,
    enable_system_controls: bool,
) -> Result<(), String> {
    window
        .with_webview(move |webview| {
            let ns_window = unsafe { &*webview.ns_window().cast::<NSWindow>() };
            // Add necessary styles for rounded corners.
            let style_mask = ns_window.styleMask()
                | NSWindowStyleMask::Borderless
                | NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::FullSizeContentView;
            ns_window.setStyleMask(style_mask);
            ns_window.setTitlebarAppearsTransparent(true);
            ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);

            let buttons = [
                ns_window.standardWindowButton(NSWindowButton::CloseButton),
                ns_window.standardWindowButton(NSWindowButton::MiniaturizeButton),
                ns_window.standardWindowButton(NSWindowButton::ZoomButton),
            ];
            for btn in buttons.into_iter().flatten() {
                btn.setHidden(!enable_system_controls);
            }
        })
        .map_err(|err| err.to_string())
}

// f64::clamp panics and aborts the app when min > max (window larger than the display),
// here min wins so the window sticks to the top or left edge
fn clamp_pinned(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn get_tray_window_position(
    app: &AppHandle,
    window_size: LogicalSize<f64>,
) -> Option<LogicalPosition<f64>> {
    let app_state = app.state::<AppState>();
    let mut x;
    let mut y;

    if let Some(tray_rect) = *app_state.tray_icon_rect.lock().unwrap() {
        let tray_position = tray_rect.position;
        let monitor = monitor_for_point(app, Some(tray_position))?;

        let scale_factor = monitor.scale_factor();
        let monitor_position = monitor.position().to_logical::<f64>(scale_factor);
        let monitor_size = monitor.size().to_logical::<f64>(scale_factor);
        let tray_position = tray_position.to_logical::<f64>(scale_factor);

        x = tray_position.x;
        y = tray_position.y;

        x = clamp_pinned(
            x,
            monitor_position.x,
            monitor_position.x + monitor_size.width - window_size.width,
        );
        y = clamp_pinned(
            y,
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
