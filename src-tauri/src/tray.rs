use tauri::{
    AppHandle, CustomMenuItem, Icon, Manager, State, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, SystemTraySubmenu,
};

use crate::{
    appstate::AppState,
    commands::{all_instances, all_locations, connect, disconnect},
    database::{Location, TrayIconTheme},
    error::Error,
    ConnectionType,
};

use regex::Regex;

static SUBSCRIBE_UPDATES_LINK: &str = "https://defguard.net/newsletter";
static JOIN_COMMUNITY_LINK: &str = "https://matrix.to/#/#defguard:teonite.com";
static FOLLOW_US_LINK: &str = "https://floss.social/@defguard";

pub async fn generate_tray_menu(app_state: State<'_, AppState>) -> Result<SystemTrayMenu, Error> {
    let quit = CustomMenuItem::new("quit", "Quit");
    let show = CustomMenuItem::new("show", "Show");
    let hide = CustomMenuItem::new("hide", "Hide");
    let subscribe_updates = CustomMenuItem::new("subscribe_updates", "Subscribe for updates");
    let join_community = CustomMenuItem::new("join_community", "Join our Community");
    let follow_us = CustomMenuItem::new("follow_us", "Follow us");
    let mut tray_menu = SystemTrayMenu::new();

    // INSTANCE SECTION
    info!("Load all instances for tray menu");
    let all_instances = all_instances(app_state.clone()).await;
    debug!("All instances: {:?}", all_instances);
    if let Ok(instances) = all_instances {
        for instance in instances {
            let mut instance_menu = SystemTrayMenu::new();
            let all_locations = all_locations(
                instance.id.expect("Missing instannce id"),
                app_state.clone(),
            )
            .await
            .unwrap();
            debug!(
                "All locations {:?} in instance {:?}",
                all_locations, instance
            );

            // TODO: apply icons instead of Connect/Disconnect when defguard utilizes tauri v2
            for location in all_locations {
                let item_name = if location.active {
                    format!("Disconnect: {}", location.name)
                } else {
                    format!("Connect: {}", location.name)
                };
                instance_menu =
                    instance_menu.add_item(CustomMenuItem::new(location.id.to_string(), item_name));
                debug!("Added new menu item for {:?}", location);
            }
            tray_menu = tray_menu.add_submenu(SystemTraySubmenu::new(instance.name, instance_menu));
        }
    } else if let Err(err) = all_instances {
        warn!("Cannot load instance menu: {:?}", err);
    }

    // Load rest of tray menu options
    tray_menu = tray_menu
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(show)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(subscribe_updates)
        .add_item(join_community)
        .add_item(follow_us)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    info!("Successfully sets tray menu");
    Ok(tray_menu)
}

pub async fn reload_tray_menu(app_handle: &AppHandle) {
    let system_menu = generate_tray_menu(app_handle.state::<AppState>())
        .await
        .unwrap();
    if let Err(err) = app_handle.tray_handle().set_menu(system_menu) {
        warn!("Unable to update tray menu {err:?}");
    }
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
    let handle = app.clone();
    let regx = Regex::new(r"^\d+$").unwrap();
    match event {
        SystemTrayEvent::LeftClick { .. } | SystemTrayEvent::RightClick { .. } => {
            tauri::async_runtime::spawn(async move {
                reload_tray_menu(&handle).await;
            });
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
            _ if regx.is_match(&id) => {
                tauri::async_runtime::spawn(async move {
                    let location_id = id.parse::<i64>().unwrap();
                    let location =
                        Location::find_by_id(&handle.state::<AppState>().get_pool(), location_id)
                            .await
                            .unwrap();

                    let active_locations_ids: Vec<i64> = handle
                        .state::<AppState>()
                        .get_connection_id_by_type(&ConnectionType::Location)
                        .await;

                    if active_locations_ids.contains(&location_id) {
                        info!("Disconnect location with id {}", id);
                        let _ =
                            disconnect(location_id, ConnectionType::Location, handle.clone()).await;
                    } else {
                        info!("Connect location with id {}", id);
                        let _ = connect(
                            location_id,
                            ConnectionType::Location,
                            Some(location.unwrap().pubkey),
                            handle.clone(),
                        )
                        .await;
                    }

                    reload_tray_menu(&handle).await;
                });
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
