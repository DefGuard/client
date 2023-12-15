use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};

use crate::{appstate::AppState, database::TrayIconTheme, error::Error};

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

fn show_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_window("main") {
        if main_window
            .is_minimized()
            .expect("Failed to check minimization state")
        {
            main_window
                .unminimize()
                .expect("Failed to unminimize main window.");
        } else if !main_window
            .is_visible()
            .expect("Failed to check main window visibility")
        {
            main_window.show().expect("Failed to show main window.");
        }
    }
}

// handle tray actions
pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            show_main_window(app);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                let app_state: State<AppState> = app.state();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let _ = app_state.close_all_connections().await;
                        std::process::exit(0);
                    });
                });
            }
            "show" => show_main_window(app),
            "hide" => {
                if let Some(main_window) = app.get_window("main") {
                    if main_window
                        .is_visible()
                        .expect("Failed to check main window visibility")
                    {
                        main_window.hide().expect("Failed to hide main window");
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
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
