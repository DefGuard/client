use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, SystemTrayMenuItem};

use crate::{database::TrayIconTheme, error::Error};

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

pub fn configure_tray_icon(app: &AppHandle, theme: &TrayIconTheme) -> Result<(), Error> {
    let resource_str = format!("resources/icons/tray-32x32-{}.png", theme.as_ref());
    debug!("Tray icon loading from {:?}", &resource_str);
    match app.path_resolver().resolve_resource(&resource_str) {
        Some(icon_path) => {
            let icon = tauri::Icon::File(icon_path);
            app.tray_handle().set_icon(icon)?;
            debug!("Tray icon changed");
            Ok(())
        }
        None => {
            error!(
                "Loading tray icon resource {} failed! Resource not resolved.",
                &resource_str
            );
            Err(Error::ResourceNotFound(resource_str))
        }
    }
}
