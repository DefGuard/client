use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};

use crate::{appstate::AppState, database::TrayIconTheme, error::Error};

static SUBSCRIBE_UPDATES_LINK: &str = "https://defguard.net/newsletter";
static JOIN_COMMUNITY_LINK: &str = "https://matrix.to/#/#defguard:teonite.com";
static FOLLOW_US_LINK: &str = "https://floss.social/@defguard";

#[must_use]
pub fn create_tray_menu() -> SystemTrayMenu {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let show = CustomMenuItem::new("show".to_string(), "Show");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let subscribe_updates =
        CustomMenuItem::new("subscribe_updates".to_string(), "Subscribe for updates");
    let join_community = CustomMenuItem::new("join_community".to_string(), "Join our Community");
    let follow_us = CustomMenuItem::new("follow_us".to_string(), "Follow us");
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
        let minimized = main_window.is_minimizable().unwrap();
        let visible = main_window.is_visible().unwrap();
        if minimized {
            main_window.unminimize().unwrap();
            main_window.set_focus().unwrap();
        }
        if !visible {
            main_window.show().unwrap();
            main_window.set_focus().unwrap();
        }
    }
}

// handle tray actions
pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            if let Some(main_window) = app.get_window("main") {
                let visibility = main_window.is_visible().unwrap();
                if visibility {
                    main_window.hide().unwrap();
                } else {
                    show_main_window(app);
                }
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                info!("Received QUIT request. Initiating shutdown...");
                let app_state: State<AppState> = app.state();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let _ = app_state.close_all_connections().await;
                        app.exit(0);
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
    debug!("Tray icon loading from {:?}", &resource_str);
    match app.path_resolver().resolve_resource(&resource_str) {
        Some(icon_path) => {
            let icon = tauri::Icon::File(icon_path);
            app.tray_handle().set_icon(icon)?;
            info!("Tray icon changed");
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
