//! defguard desktop client

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, str::FromStr};

use defguard_client::{
    __cmd__active_connection, __cmd__all_connections, __cmd__all_instances, __cmd__all_locations,
    __cmd__all_tunnels, __cmd__connect, __cmd__delete_instance, __cmd__delete_tunnel,
    __cmd__disconnect, __cmd__get_latest_app_version, __cmd__get_settings, __cmd__last_connection,
    __cmd__location_interface_details, __cmd__location_stats, __cmd__open_link,
    __cmd__parse_tunnel_config, __cmd__save_device_config, __cmd__save_tunnel,
    __cmd__start_global_logwatcher, __cmd__stop_global_logwatcher, __cmd__tunnel_details,
    __cmd__update_instance, __cmd__update_location_routing, __cmd__update_settings,
    __cmd__update_tunnel,
    appstate::AppState,
    commands::{
        active_connection, all_connections, all_instances, all_locations, all_tunnels, connect,
        delete_instance, delete_tunnel, disconnect, get_latest_app_version, get_settings,
        last_connection, location_interface_details, location_stats, open_link,
        parse_tunnel_config, save_device_config, save_tunnel, start_global_logwatcher,
        stop_global_logwatcher, tunnel_details, update_instance, update_location_routing,
        update_settings, update_tunnel,
    },
    database::{self, models::settings::Settings},
    enterprise::periodic::config::poll_config,
    events::SINGLE_INSTANCE,
    periodic::version::poll_version,
    tray::{configure_tray_icon, handle_tray_event, reload_tray_menu},
    utils::load_log_targets,
    VERSION,
};
use lazy_static::lazy_static;
use log::{Level, LevelFilter};
#[cfg(target_os = "macos")]
use tauri::{api::process, Env};
use tauri::{Builder, Manager, RunEvent, State, SystemTray, WindowEvent};
use tauri_plugin_log::LogTarget;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[macro_use]
extern crate log;

// for tauri log plugin
const LOG_TARGETS: [LogTarget; 2] = [LogTarget::Stdout, LogTarget::LogDir];

lazy_static! {
    static ref LOG_INCLUDES: Vec<String> = load_log_targets();
}

// TODO: Refactor later
#[allow(clippy::single_match)]
#[tokio::main]
async fn main() {
    // add bundled `wireguard-go` binary to PATH
    #[cfg(target_os = "macos")]
    {
        debug!("Adding bundled wireguard-go binary to PATH");
        let current_bin_path =
            process::current_binary(&Env::default()).expect("Failed to get current binary path");
        let current_bin_dir = current_bin_path
            .parent()
            .expect("Failed to get current binary directory");
        let current_path = env::var("PATH").expect("Failed to get current PATH variable");
        env::set_var(
            "PATH",
            format! {"{current_path}:{}", current_bin_dir.to_str().unwrap()},
        );
        debug!("Added binary dir {current_bin_dir:?} to PATH");
    }

    let log_level =
        LevelFilter::from_str(&env::var("DEFGUARD_CLIENT_LOG_LEVEL").unwrap_or("info".into()))
            .unwrap_or(LevelFilter::Info);

    let app = Builder::default()
        .invoke_handler(tauri::generate_handler![
            all_locations,
            save_device_config,
            all_instances,
            connect,
            disconnect,
            update_instance,
            location_stats,
            location_interface_details,
            all_connections,
            last_connection,
            active_connection,
            update_location_routing,
            get_settings,
            update_settings,
            delete_instance,
            parse_tunnel_config,
            save_tunnel,
            all_tunnels,
            open_link,
            tunnel_details,
            update_tunnel,
            delete_tunnel,
            get_latest_app_version,
            start_global_logwatcher,
            stop_global_logwatcher
        ])
        .on_window_event(|event| match event.event() {
            WindowEvent::CloseRequested { api, .. } => {
                #[cfg(not(target_os = "macos"))]
                let _ = event.window().hide();

                #[cfg(target_os = "macos")]
                let _ = tauri::AppHandle::hide(&event.window().app_handle());

                api.prevent_close();
            }
            _ => {}
        })
        .system_tray(SystemTray::new())
        .on_system_tray_event(handle_tray_event)
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let _ = app.emit_all(SINGLE_INSTANCE, Payload { args: argv, cwd });
        }))
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets(LOG_TARGETS)
                .level(log_level)
                .filter(|metadata| {
                    if metadata.level() == Level::Error {
                        return true;
                    }
                    if !LOG_INCLUDES.is_empty() {
                        for target in LOG_INCLUDES.iter() {
                            if metadata.target().contains(target) {
                                return true;
                            }
                        }
                        return false;
                    }
                    true
                })
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(AppState::default())
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    info!("Starting version v{}...", VERSION);
    // initialize database
    let app_handle = app.handle();
    debug!("Initializing database connection...");
    let app_state: State<AppState> = app_handle.state();
    let db = database::init_db(&app_handle)
        .await
        .expect("Database initialization failed");
    *app_state.db.lock().unwrap() = Some(db);
    info!("Database initialization completed");
    debug!("Getting database info to check if the connection is working...");
    let result = database::info(&app_state.get_pool()).await;
    if let Err(e) = result {
        error!(
            "There was an error while getting the database info: {:?}. The database connection might not be working.",
            e
        );
    }
    info!(
        "Database info has been fetched successfully. The connection with the database is working."
    );

    // configure tray
    debug!("Configuring tray icon...");
    if let Ok(settings) = Settings::get(&app_state.get_pool()).await {
        let _ = configure_tray_icon(&app_handle, &settings.tray_icon_theme);
    }
    debug!("Tray icon has been configured successfully");

    // run periodic tasks
    debug!("Starting periodic tasks...");
    tauri::async_runtime::spawn(poll_version(app_handle.clone()));
    tauri::async_runtime::spawn(poll_config(app_handle.clone()));
    debug!("Periodic tasks have been started");

    // load tray menu after database initialization to show all instance and locations
    debug!("Reloading tray menu to show all instances and locations...");
    reload_tray_menu(&app_handle).await;
    debug!("Tray menu has been reloaded successfully");

    // Handle Ctrl-C
    debug!("Setting up Ctrl-C handler...");
    tauri::async_runtime::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Signal handler failure");
        debug!("Ctrl-C handler: quitting the app");
        let app_state: State<AppState> = app_handle.state();
        app_state.quit(&app_handle);
    });
    debug!("Ctrl-C handler has been set up successfully");

    // run app
    info!("Running the application...");
    app.run(|app_handle, event| match event {
        // prevent shutdown on window close
        RunEvent::ExitRequested { api, .. } => {
            debug!("Received exit request");
            api.prevent_exit();
        }
        // handle shutdown
        RunEvent::Exit => {
            info!("Exiting event loop...");
            let app_state: State<AppState> = app_handle.state();
            app_state.quit(app_handle);
        }
        _ => {
            trace!("Received event: {event:?}");
        }
    });
}
