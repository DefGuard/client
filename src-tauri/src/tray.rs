use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder},
    path::BaseDirectory,
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};

use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};

#[cfg(not(target_os = "linux"))]
use crate::window_manager::WindowManager;
use crate::{
    active_connections::{get_connection_id_by_type, ACTIVE_CONNECTIONS},
    appstate::AppState,
    commands::{all_instances, all_locations, connect, disconnect},
    database::{models::location::Location, DB_POOL},
    error::Error,
    events::EventKey,
    window_manager::{show_tray_window, COMPACT_WINDOW_ID, FULL_VIEW_WINDOW_ID},
    ConnectionType,
};

const SUBSCRIBE_UPDATES_LINK: &str = "https://defguard.net/newsletter";
const JOIN_COMMUNITY_LINK: &str = "https://github.com/DefGuard/defguard/discussions/new/choose";
const FOLLOW_US_LINK: &str = "https://floss.social/@defguard";

const TRAY_ICON_ID: &str = "tray";

const TRAY_EVENT_QUIT: &str = "quit";
const TRAY_EVENT_SHOW: &str = "show";
const TRAY_EVENT_HIDE: &str = "hide";
const TRAY_EVENT_UPDATES: &str = "updates";
const TRAY_EVENT_COMMUNITY: &str = "community";
const TRAY_EVENT_FOLLOW: &str = "follow";

fn store_tray_click_position(app: &AppHandle, event: &TrayIconEvent) {
    let position = match event {
        TrayIconEvent::Click {
            button_state: MouseButtonState::Down,
            rect,
            ..
        } => Some(rect.position.to_physical(1.0)),
        _ => None,
    };

    if let Some(position) = position {
        *app.state::<AppState>().tray_click_position.lock().unwrap() = Some(position);
    }
}

/// Generate contents of system tray menu.
async fn generate_tray_menu(app: &AppHandle) -> Result<Menu<impl Runtime>, Error> {
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
        TRAY_EVENT_COMMUNITY,
        "Community support",
        true,
        None::<&str>,
    )?;
    let follow_us = MenuItem::with_id(app, TRAY_EVENT_FOLLOW, "Follow us", true, None::<&str>)?;

    let mut menu = MenuBuilder::new(app);
    debug!("Getting all instances information for the tray menu");
    match all_instances().await {
        Ok(instances) => {
            let instance_count = instances.len();
            debug!("Got {instance_count} instances to display in the tray menu");

            // One instance omits sub-menu.
            if instance_count == 1 {
                let instance = &instances[0];
                let all_locations = all_locations(instance.id).await?;
                debug!(
                    "Found {} locations for the {instance} instance to display in the tray menu",
                    all_locations.len(),
                );
                // TODO: Use icons instead of Connect/Disconnect when Defguard utilizes tauri v2.
                for location in all_locations {
                    let menu_item = MenuItem::with_id(
                        app,
                        location.id.to_string(),
                        location.menu_label(),
                        true,
                        None::<&str>,
                    )?;
                    menu = menu.item(&menu_item);
                }
            } else {
                for instance in instances {
                    let mut instance_menu = SubmenuBuilder::new(app, &instance.name);
                    let all_locations = all_locations(instance.id).await?;
                    debug!(
                        "Found {} locations for the {instance} instance to display in the tray menu",
                        all_locations.len(),
                    );

                    // TODO: Use icons instead of Connect/Disconnect when Defguard utilizes tauri v2.
                    for location in all_locations {
                        let menu_item = MenuItem::with_id(
                            app,
                            location.id.to_string(),
                            location.menu_label(),
                            true,
                            None::<&str>,
                        )?;
                        instance_menu = instance_menu.item(&menu_item);
                    }
                    let submenu = instance_menu.build()?;
                    menu = menu.item(&submenu);
                }
            }
        }
        Err(err) => {
            warn!("Cannot load instance menu: {err:?}");
        }
    }

    Ok(menu
        .separator()
        .items(&[&show, &hide])
        .separator()
        .items(&[&subscribe_updates, &join_community, &follow_us])
        .separator()
        .item(&quit)
        .build()?)
}

/// Setup system tray.
/// This function should only be called once.
pub async fn setup_tray(app: &AppHandle) -> Result<(), Error> {
    let tray_menu = generate_tray_menu(app).await?;

    TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|icon, event| {
            store_tray_click_position(icon.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = icon.app_handle();

                #[cfg(target_os = "linux")]
                show_main_window(app);

                #[cfg(not(target_os = "linux"))]
                {
                    let main_visible = app
                        .get_webview_window(FULL_VIEW_WINDOW_ID)
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);

                    if main_visible {
                        if let Some(w) = app.get_webview_window(FULL_VIEW_WINDOW_ID) {
                            let _ = w.hide();
                        }
                    }

                    let tray_visible = app
                        .get_webview_window(COMPACT_WINDOW_ID)
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);

                    if tray_visible {
                        if let Some(w) = app.get_webview_window(COMPACT_WINDOW_ID) {
                            let _ = w.hide();
                        }
                    } else {
                        let has_locations = tauri::async_runtime::block_on(
                            crate::window_manager::has_non_service_locations(),
                        );
                        if has_locations {
                            if let Some(old_ui) = app.get_webview_window(FULL_VIEW_WINDOW_ID) {
                                let _ = old_ui.hide();
                            }
                            show_tray_window(app);
                        } else {
                            let _ = WindowManager::open_full_view(app);
                        }
                    }
                }
            }
        })
        .on_menu_event(handle_tray_menu_event)
        .build(app)?;

    debug!("Tray menu successfully generated");
    Ok(())
}

/// Reload menu contents in system tray.
pub(crate) async fn reload_tray_menu(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ICON_ID) else {
        error!("System tray menu not initialized.");
        return;
    };

    let menu = generate_tray_menu(app).await.ok();
    match tray.set_menu(menu) {
        Ok(()) => debug!("System tray menu re-generarted."),
        Err(err) => error!("Failed to re-generate system tray menu: {err}"),
    }
}

fn hide_visible_windows(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Err(err) = app.hide() {
        warn!("Failed to hide application: {err}");
    }
    for (id, window) in app.webview_windows() {
        if window.is_visible().unwrap_or(false) {
            if let Err(err) = window.hide() {
                warn!("Failed to hide window {id}: {err}");
            }
        }
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app
        .get_webview_window(COMPACT_WINDOW_ID)
        .or_else(|| app.get_webview_window(FULL_VIEW_WINDOW_ID))
    {
        if let Err(err) = window.unminimize() {
            warn!("Failed to unminimize main window: {err}");
        }
        #[cfg(target_os = "macos")]
        if let Err(err) = app.show() {
            warn!("Failed to show application: {err}");
        }
        if let Err(err) = window.show() {
            warn!("Failed to show main window: {err}");
        }
        let _ = window.set_focus();
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
        TRAY_EVENT_SHOW => show_tray_window(app),
        TRAY_EVENT_HIDE => hide_visible_windows(app),
        TRAY_EVENT_UPDATES => {
            let _ = webbrowser::open(SUBSCRIBE_UPDATES_LINK);
        }
        TRAY_EVENT_COMMUNITY => {
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

/// Show correct system tray icon, depending on the theme and connection status.
pub async fn configure_tray_icon(app_handle: &AppHandle) -> Result<(), Error> {
    let Some(tray_icon) = app_handle.tray_by_id(TRAY_ICON_ID) else {
        error!("System tray menu not initialized.");
        return Ok(());
    };

    let mut resource_str = String::from("resources/icons/tray-32x32");
    let active_connections = ACTIVE_CONNECTIONS.lock().await;
    if !active_connections.is_empty() {
        resource_str.push_str("-active");
    }
    resource_str.push_str(".png");
    debug!("Trying to load the tray icon from {resource_str}");
    if let Ok(icon_path) = app_handle
        .path()
        .resolve(&resource_str, BaseDirectory::Resource)
    {
        let icon = Image::from_path(icon_path)?;
        tray_icon.set_icon(Some(icon))?;
        debug!("Tray icon set to {resource_str} successfully.");
        Ok(())
    } else {
        error!("Loading tray icon resource {resource_str} failed! Resource not resolved.");
        Err(Error::ResourceNotFound(resource_str))
    }
}

async fn handle_location_tray_menu(id: String, app: &AppHandle) {
    match id.parse::<i64>() {
        Ok(location_id) => {
            match Location::find_by_id(&*DB_POOL, location_id).await {
                Ok(Some(location)) => {
                    let active_locations_ids =
                        get_connection_id_by_type(ConnectionType::Location).await;

                    if active_locations_ids.contains(&location_id) {
                        info!("Disconnect location with ID {id}");
                        let _ =
                            disconnect(location_id, ConnectionType::Location, app.clone()).await;
                    } else {
                        info!("Connect location with ID {id}");
                        // Check if MFA is enabled. If so, trigger modal on frontend.
                        if location.mfa_enabled() {
                            info!("MFA enabled for location with ID {id}, trigger MFA modal");
                            show_main_window(app);
                            let _ = app.emit(EventKey::MfaTrigger.into(), &location);
                        } else if let Err(err) =
                            connect(location_id, ConnectionType::Location, None, app.clone()).await
                        {
                            info!("Unable to connect location with ID {id}, error: {err:?}");
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
