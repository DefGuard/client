use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, SystemTrayMenuItem};

use crate::database::TrayIconTheme;

#[must_use]
pub fn create_tray_menu() -> SystemTrayMenu {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let show = CustomMenuItem::new("show".to_string(), "Show");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit)
}

pub fn configure_tray_icon(app: &AppHandle, theme: &TrayIconTheme) {
    let icon_path = match theme {
        TrayIconTheme::Black => app
            .path_resolver()
            .resolve_resource("icons/tray-32x32-black.png"),
        TrayIconTheme::Color => app
            .path_resolver()
            .resolve_resource("icons/tray-32x32-color.png"),
        TrayIconTheme::Gray => app
            .path_resolver()
            .resolve_resource("icons/tray-32x32-gray.png"),
        TrayIconTheme::White => app
            .path_resolver()
            .resolve_resource("icons/tray-32x32-white.png"),
    }
    .unwrap();
    let icon = tauri::Icon::File(icon_path);
    app.tray_handle().set_icon(icon).unwrap();
}
