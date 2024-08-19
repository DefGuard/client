use tauri::{
    AppHandle, CustomMenuItem, Icon, Manager, State, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

use crate::{appstate::AppState, database::TrayIconTheme, error::Error};

static SUBSCRIBE_UPDATES_LINK: &str = "https://defguard.net/newsletter";
static JOIN_COMMUNITY_LINK: &str = "https://matrix.to/#/#defguard:teonite.com";
static FOLLOW_US_LINK: &str = "https://floss.social/@defguard";

#[must_use]
pub fn create_tray_menu() -> SystemTrayMenu {
    let quit = CustomMenuItem::new("quit", "Quit");
    let show = CustomMenuItem::new("show", "Show");
    let hide = CustomMenuItem::new("hide", "Hide");
    let subscribe_updates = CustomMenuItem::new("subscribe_updates", "Subscribe for updates");
    let join_community = CustomMenuItem::new("join_community", "Join our Community");
    let follow_us = CustomMenuItem::new("follow_us", "Follow us");
    SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(subscribe_updates)
        .add_item(join_community)
        .add_item(follow_us)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit)
}

fn show_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_window("main") {
        // if this fails tauri has a problem
        let minimized = main_window.is_minimizable().unwrap_or_default();
        let visible = main_window.is_visible().unwrap_or_default();
        if minimized {
            let _ = main_window.unminimize();
            let _ = main_window.set_focus();
        }
        if !visible {
            let _ = main_window.show();
            let _ = main_window.set_focus();
        }
    }
}

// handle tray actions
pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            if let Some(main_window) = app.get_window("main") {
                let visible = main_window.is_visible().unwrap_or_default();
                if visible {
                    let _ = main_window.hide();
                } else {
                    show_main_window(app);
                }
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                info!("Received QUIT request. Initiating shutdown...");
                let app_state: State<AppState> = app.state();
                app_state.quit(app);
            }
            "show" => show_main_window(app),
            "hide" => {
                if let Some(main_window) = app.get_window("main") {
                    if main_window.is_visible().unwrap_or_default() {
                        let _ = main_window.hide();
                    }
                }
            }
            "subscribe_updates" => {
                let _ = webbrowser::open(SUBSCRIBE_UPDATES_LINK);
            }
            "join_community" => {
                let _ = webbrowser::open(JOIN_COMMUNITY_LINK);
            }
            "follow_us" => {
                let _ = webbrowser::open(FOLLOW_US_LINK);
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn configure_tray_icon(app: &AppHandle, theme: &TrayIconTheme) -> Result<(), Error> {
    let resource_str = format!("resources/icons/tray-32x32-{}.png", theme.as_ref());
    debug!("Tray icon loading from {resource_str}");
    if let Some(icon_path) = app.path_resolver().resolve_resource(&resource_str) {
        let icon = Icon::File(icon_path);
        app.tray_handle().set_icon(icon)?;
        info!("Tray icon changed");
        Ok(())
    } else {
        error!("Loading tray icon resource {resource_str} failed! Resource not resolved.",);
        Err(Error::ResourceNotFound(resource_str))
    }
}
