use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    path::BaseDirectory,
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};

use crate::{
    app_config::AppTrayTheme,
    appstate::AppState,
    commands::{all_instances, all_locations, connect, disconnect},
    database::models::location::Location,
    error::Error,
    ConnectionType,
};

static SUBSCRIBE_UPDATES_LINK: &str = "https://defguard.net/newsletter";
static JOIN_COMMUNITY_LINK: &str = "https://matrix.to/#/#defguard:teonite.com";
static FOLLOW_US_LINK: &str = "https://floss.social/@defguard";

static TRAY_ICON_ID: &str = "tray";

pub async fn generate_tray_menu(app: &AppHandle) -> Result<TrayIcon, Error> {
    let app_state = app.state::<AppState>();
    debug!("Generating tray menu.");
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let subscribe_updates = MenuItem::with_id(
        app,
        "subscribe_updates",
        "Subscribe for updates",
        true,
        None::<&str>,
    )?;
    let join_community = MenuItem::with_id(
        app,
        "join_community",
        "Join our Community",
        true,
        None::<&str>,
    )?;
    let follow_us = MenuItem::with_id(app, "follow_us", "Follow us", true, None::<&str>)?;
    let tray_menu = MenuBuilder::new(app)
        // TODO: instances
        .separator()
        .items(&[&show, &hide])
        .separator()
        .items(&[&subscribe_updates, &join_community, &follow_us])
        .separator()
        .item(&quit)
        .build()?;

    // INSTANCE SECTION
    debug!("Getting all instances information for the tray menu");
    let all_instances = all_instances(app_state).await;
    // if let Ok(instances) = all_instances {
    //     let instance_count = instances.len();
    // debug!("Got {instance_count} instances to display in the tray menu");
    //     for instance in instances {
    //         let mut instance_menu = SystemTrayMenu::new();
    //         let all_locations = all_locations(instance.id, app_state.clone()).await.unwrap();
    //         debug!(
    //             "Found {} locations for the {} instance to display in the tray menu",
    //             all_locations.len(),
    //             instance
    //         );

    //         // TODO: apply icons instead of Connect/Disconnect when defguard utilizes tauri v2
    //         for location in all_locations {
    //             let item_name = if location.active {
    //                 format!("Disconnect: {}", location.name)
    //             } else {
    //                 format!("Connect: {}", location.name)
    //             };
    //             instance_menu = instance_menu.add_item(MenuItem::with_id(
    //                 app,
    //                 location.id.to_string(),
    //                 item_name,
    //                 true,
    //                 None,
    //             ));
    //             debug!("Added new tray menu item (instance {instance}) for location: {location}");
    //         }
    //         tray_menu = tray_menu.add_submenu(SystemTraySubmenu::new(instance.name, instance_menu));
    //     }
    // } else if let Err(err) = all_instances {
    //     warn!("Cannot load instance menu: {err:?}");
    // }

    // Load rest of tray menu options
    // TODO: move from above

    let tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(true)
        .build(app)?;
    debug!("Successfully generated tray menu");
    Ok(tray)
}

pub async fn reload_tray_menu(app_handle: &AppHandle) {
    let _system_menu = generate_tray_menu(app_handle).await.unwrap();
    // if let Err(err) = app_handle.tray_handle().set_menu(system_menu) {
    //     warn!("Unable to update tray menu {err:?}");
    // }
}

fn show_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_webview_window("main") {
        // If this fails, Tauri has a problem.
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

/// Handle tray actions.
pub fn handle_tray_event(app: &AppHandle, event: TrayIconEvent) {
    let handle = app.clone();
    if let TrayIconEvent::Click { id, .. } = event {
        match id.0.as_str() {
            "quit" => {
                info!("Received QUIT request. Initiating shutdown...");
                let app_state: State<AppState> = app.state();
                app_state.quit(app);
            }
            "show" => show_main_window(app),
            "hide" => {
                if let Some(main_window) = app.get_webview_window("main") {
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
            idstr if idstr.chars().all(char::is_numeric) => {
                tauri::async_runtime::spawn(async move {
                    handle_location_tray_menu(id.0, &handle).await;
                });
            }
            _ => {}
        }
    }
}

pub fn configure_tray_icon(app: &AppHandle, theme: &AppTrayTheme) -> Result<(), Error> {
    let tray_icon = app.tray_by_id(TRAY_ICON_ID).unwrap(); // FIXME: remove unwrap
    let resource_str = format!("resources/icons/tray-32x32-{theme}.png");
    debug!("Trying to load the tray icon from {resource_str}");
    if let Ok(icon_path) = app.path().resolve(&resource_str, BaseDirectory::Resource) {
        let icon = Image::from_path(icon_path)?;
        tray_icon.set_icon(Some(icon))?;
        debug!("Tray icon set to {resource_str} successfully.");
        Ok(())
    } else {
        error!("Loading tray icon resource {resource_str} failed! Resource not resolved.",);
        Err(Error::ResourceNotFound(resource_str))
    }
}

#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
}

async fn handle_location_tray_menu(id: String, handle: &AppHandle) {
    match id.parse::<i64>() {
        Ok(location_id) => {
            match Location::find_by_id(&handle.state::<AppState>().db, location_id).await {
                Ok(Some(location)) => {
                    let active_locations_ids = handle
                        .state::<AppState>()
                        .get_connection_id_by_type(ConnectionType::Location)
                        .await;

                    if active_locations_ids.contains(&location_id) {
                        info!("Disconnect location with ID {id}");
                        let _ =
                            disconnect(location_id, ConnectionType::Location, handle.clone()).await;
                    } else {
                        info!("Connect location with ID {id}");
                        // check is mfa enabled and trigger modal on frontend
                        if location.mfa_enabled() {
                            info!(
                                "MFA enabled for location with ID {:?}, trigger MFA modal",
                                location.id
                            );
                            handle
                                .emit(
                                    "mfa-trigger",
                                    Payload {
                                        message: "Trigger MFA event".into(),
                                    },
                                )
                                .unwrap();
                        } else if let Err(err) =
                            connect(location_id, ConnectionType::Location, None, handle.clone())
                                .await
                        {
                            info!(
                                "Unable to connect location with ID {}, error: {err:?}",
                                location.id
                            );
                        }
                    }
                }
                Ok(None) => warn!("Location does not exist"),
                Err(err) => warn!("Unable to find location: {err:?}"),
            };
        }
        Err(err) => warn!("Can't handle event due to: {err:?}"),
    }
}
