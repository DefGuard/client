use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder},
    path::BaseDirectory,
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager,
};

use crate::{
    active_connections::get_connection_id_by_type,
    app_config::AppTrayTheme,
    commands::{all_instances, all_locations, connect, disconnect},
    database::{models::location::Location, DB_POOL},
    error::Error,
    ConnectionType,
};

const SUBSCRIBE_UPDATES_LINK: &str = "https://defguard.net/newsletter";
const JOIN_COMMUNITY_LINK: &str = "https://matrix.to/#/#defguard:teonite.com";
const FOLLOW_US_LINK: &str = "https://floss.social/@defguard";

const MAIN_WINDOW_ID: &str = "main";

const TRAY_ICON_ID: &str = "tray";

const TRAY_EVENT_QUIT: &str = "quit";
const TRAY_EVENT_SHOW: &str = "show";
const TRAY_EVENT_HIDE: &str = "hide";
const TRAY_EVENT_UPDATES: &str = "updates";
const TRAY_EVENT_COMMINITY: &str = "community";
const TRAY_EVENT_FOLLOW: &str = "follow";

pub async fn generate_tray_menu(app: &AppHandle) -> Result<TrayIcon, Error> {
    debug!("Generating tray menu.");
    let quit = MenuItem::with_id(app, TRAY_EVENT_QUIT, "Quit", true, None::<&str>)?;
    let show = MenuItem::with_id(app, TRAY_EVENT_SHOW, "Show", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, TRAY_EVENT_HIDE, "Hide", true, None::<&str>)?;
    let subscribe_updates = MenuItem::with_id(
        app,
        TRAY_EVENT_UPDATES,
        "Subscribe for updates",
        true,
        None::<&str>,
    )?;
    let join_community = MenuItem::with_id(
        app,
        TRAY_EVENT_COMMINITY,
        "Join our community",
        true,
        None::<&str>,
    )?;
    let follow_us = MenuItem::with_id(app, TRAY_EVENT_FOLLOW, "Follow us", true, None::<&str>)?;

    // INSTANCE SECTION
    let mut instance_menu = SubmenuBuilder::new(app, "Instances");
    debug!("Getting all instances information for the tray menu");
    match all_instances().await {
        Ok(instances) => {
            let instance_count = instances.len();
            debug!("Got {instance_count} instances to display in the tray menu");
            for instance in instances {
                let all_locations = all_locations(instance.id).await.unwrap();
                debug!(
                    "Found {} locations for the {} instance to display in the tray menu",
                    all_locations.len(),
                    instance
                );

                // TODO: Use icons instead of Connect/Disconnect when Defguard utilizes tauri v2.
                for location in all_locations {
                    let item_name = format!(
                        "{}: {}",
                        if location.active {
                            "Disconnect"
                        } else {
                            "Connect"
                        },
                        location.name
                    );
                    let menu_item = MenuItem::with_id(
                        app,
                        location.id.to_string(),
                        item_name,
                        true,
                        None::<&str>,
                    )?;
                    instance_menu = instance_menu.item(&menu_item);
                }
            }
        }
        Err(err) => {
            warn!("Cannot load instance menu: {err:?}");
        }
    }

    let tray_menu = MenuBuilder::new(app)
        .items(&[&instance_menu.build()?])
        .separator()
        .items(&[&show, &hide])
        .separator()
        .items(&[&subscribe_updates, &join_community, &follow_us])
        .separator()
        .item(&quit)
        .build()?;

    // On macOS, always show menu under system tray icon.
    #[cfg(target_os = "macos")]
    let tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_tray_menu_event)
        .build(app)?;
    // On other systems (especially Windows), system tray menu is on right-click,
    // and double-click shows the main window.
    #[cfg(not(target_os = "macos"))]
    let tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|icon, event| {
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                show_main_window(icon.app_handle())
            }
        })
        .on_menu_event(handle_tray_menu_event)
        .build(app)?;

    debug!("Tray menu successfully generated");
    Ok(tray)
}

pub async fn reload_tray_menu(app_handle: &AppHandle) {
    match generate_tray_menu(app_handle).await {
        Ok(_) => debug!("System tray menu re-generarted."),
        Err(err) => error!("Failed to re-generate system tray menu: {err}"),
    }
}

fn hide_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_webview_window(MAIN_WINDOW_ID) {
        if let Err(err) = main_window.hide() {
            warn!("Failed to hide main window: {err}");
        }
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_webview_window(MAIN_WINDOW_ID) {
        if main_window.is_minimized().unwrap_or_default() {
            if let Err(err) = main_window.unminimize() {
                warn!("Failed to unminimize main window: {err}");
            }
        }
        if let Err(err) = main_window.show() {
            warn!("Failed to show main window: {err}");
        }
        let _ = main_window.set_focus();
    }
}

/// Handle tray actions.
pub fn handle_tray_menu_event(app: &AppHandle, event: MenuEvent) {
    let handle = app.clone();
    match event.id.as_ref() {
        TRAY_EVENT_QUIT => {
            info!("Received QUIT request. Initiating shutdown...");
            handle.exit(0);
        }
        TRAY_EVENT_SHOW => show_main_window(app),
        TRAY_EVENT_HIDE => hide_main_window(app),
        TRAY_EVENT_UPDATES => {
            let _ = webbrowser::open(SUBSCRIBE_UPDATES_LINK);
        }
        TRAY_EVENT_COMMINITY => {
            let _ = webbrowser::open(JOIN_COMMUNITY_LINK);
        }
        TRAY_EVENT_FOLLOW => {
            let _ = webbrowser::open(FOLLOW_US_LINK);
        }
        id if id.chars().all(char::is_numeric) => {
            tauri::async_runtime::spawn(async move {
                handle_location_tray_menu(event.id.0, &handle).await;
            });
        }
        _ => {}
    }
}

pub fn configure_tray_icon(app: &AppHandle, theme: &AppTrayTheme) -> Result<(), Error> {
    let Some(tray_icon) = app.tray_by_id(TRAY_ICON_ID) else {
        error!("System tray menu not initialized.");
        return Ok(());
    };
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
struct MfaPayload<'a> {
    message: &'a str,
}

async fn handle_location_tray_menu(id: String, handle: &AppHandle) {
    match id.parse::<i64>() {
        Ok(location_id) => {
            match Location::find_by_id(&*DB_POOL, location_id).await {
                Ok(Some(location)) => {
                    let active_locations_ids =
                        get_connection_id_by_type(ConnectionType::Location).await;

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
                            let _ = handle.emit(
                                "mfa-trigger",
                                MfaPayload {
                                    message: "Trigger MFA event",
                                },
                            );
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
