//! defguard desktop client

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, str::FromStr, sync::LazyLock};

#[cfg(target_os = "windows")]
use defguard_client::utils::sync_connections;
use defguard_client::{
    appstate::AppState,
    commands::*,
    events::SINGLE_INSTANCE,
    periodic::run_periodic_tasks,
    service,
    tray::{configure_tray_icon, handle_tray_event, reload_tray_menu},
    utils::load_log_targets,
    VERSION,
};
use log::{Level, LevelFilter};
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
// if found in metadata target name it will ignore the log if it was below info level
const LOGGING_TARGET_IGNORE_LIST: [&str; 5] = ["tauri", "sqlx", "hyper", "h2", "tower"];

static LOG_INCLUDES: LazyLock<Vec<String>> = LazyLock::new(load_log_targets);

#[tokio::main]
async fn main() {
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
            stop_global_logwatcher,
            command_get_app_config,
            command_set_app_config
        ])
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                #[cfg(not(target_os = "macos"))]
                let _ = event.window().hide();

                #[cfg(target_os = "macos")]
                let _ = tauri::AppHandle::hide(&event.window().app_handle());

                api.prevent_close();
            }
        })
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let _ = app.emit_all(SINGLE_INSTANCE, Payload { args: argv, cwd });
        }))
        .setup(|app| {
            {
                let state = AppState::new(&app.app_handle());
                app.manage(state);
            }
            let app_state: State<AppState> = app.state();

            // use config default if deriving from env value fails so that env can override config file
            let config_log_level = app_state.app_config.lock().unwrap().log_level;

            let log_level = match &env::var("DEFGUARD_CLIENT_LOG_LEVEL") {
                Ok(env_value) => LevelFilter::from_str(env_value).unwrap_or(config_log_level),
                Err(_) => config_log_level,
            };

            // Sets the time format. Service's logs have a subsecond part, so we also need to include it here,
            // otherwise the logs couldn't be sorted correctly when displayed together in the UI.
            let format = time::macros::format_description!(
                "[[[year]-[month]-[day]][[[hour]:[minute]:[second].[subsecond]]"
            );

            app.handle()
                .plugin(
                    tauri_plugin_log::Builder::default()
                        .format(move |out, message, record| {
                            out.finish(format_args!(
                                "{}[{}][{}] {}",
                                tauri_plugin_log::TimezoneStrategy::UseUtc
                                    .get_now()
                                    .format(&format)
                                    .unwrap(),
                                record.level(),
                                record.target(),
                                message
                            ));
                        })
                        .targets(LOG_TARGETS)
                        .level(log_level)
                        .filter(|metadata| {
                            if metadata.level() == Level::Error {
                                return true;
                            }
                            if !LOG_INCLUDES.is_empty() {
                                for target in &*LOG_INCLUDES {
                                    if metadata.target().contains(target) {
                                        return true;
                                    }
                                }
                                return false;
                            }
                            true
                        })
                        .filter(|metadata| {
                            // Log all errors, warnings and infos
                            if metadata.level() == LevelFilter::Error
                                || metadata.level() == LevelFilter::Warn
                                || metadata.level() == LevelFilter::Info
                            {
                                return true;
                            }
                            // Otherwise do not log the following targets
                            for target in &LOGGING_TARGET_IGNORE_LIST {
                                if metadata.target().contains(target) {
                                    return false;
                                }
                            }
                            true
                        })
                        .build(),
                )
                .unwrap();

            info!("App setup completed, log level: {log_level}");

            Ok(())
        })
        .system_tray(SystemTray::new())
        .on_system_tray_event(handle_tray_event)
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    info!("Starting Defguard client version {VERSION}");
    // initialize database
    let app_handle = app.handle();

    info!(
        "The application data (database file) will be stored in: {:?} \
        and the application logs in: {:?}. Logs of the background defguard service responsible for \
        managing the VPN connections at the network level will be stored in: {}.",
        // display the path to the app data directory, convert option<pathbuf> to option<&str>
        app_handle
            .path_resolver()
            .app_data_dir()
            .unwrap_or_else(|| "UNDEFINED DATA DIRECTORY".into()),
        app_handle
            .path_resolver()
            .app_log_dir()
            .unwrap_or_else(|| "UNDEFINED LOG DIRECTORY".into()),
        service::config::DEFAULT_LOG_DIR
    );

    debug!("Running database migrations, if there are any.");
    let app_state: State<AppState> = app_handle.state();
    sqlx::migrate!()
        .run(&app_state.db)
        .await
        .expect("Failed to apply database migrations");
    debug!("Applied all database migrations that were pending. If any.");
    debug!("Database setup has been completed successfully.");

    // Sync already active connections on windows.
    // When windows is restarted, the app doesn't close the active connections
    // and they are still running after the restart. We sync them here to
    // reflect the real system's state.
    // TODO: Find a way to intercept the shutdown event and close all connections
    #[cfg(target_os = "windows")]
    {
        match sync_connections(&app_handle).await {
            Ok(_) => {
                info!("Synchronized application's active connections with the connections already open on the system, if there were any.")
            }
            Err(err) => {
                warn!(
                    "Failed to synchronize application's active connections with the connections already open on the system. \
                    The connections' state in the application may not reflect system's state. Reconnect manually to reset them. Error: {err}"
                )
            }
        };
    }

    // configure tray
    debug!("Configuring tray icon...");
    if let Ok(app_config) = &app_state.app_config.lock() {
        let _ = configure_tray_icon(&app_handle, &app_config.tray_theme);
        debug!("Tray icon has been configured successfully");
    } else {
        error!("Could not lock app config guard for tray configuration during app init.");
    }

    // run periodic tasks
    debug!(
        "Starting periodic tasks (config, version polling and active connection verification)..."
    );
    let periodic_tasks_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        run_periodic_tasks(&periodic_tasks_handle).await;
        // One of the tasks exited, so something went wrong, quit the app
        error!("One of the periodic tasks has stopped unexpectedly. Exiting the application.");
        let app_state: State<AppState> = periodic_tasks_handle.state();
        app_state.quit(&periodic_tasks_handle);
    });
    debug!("Periodic tasks have been started");

    // load tray menu after database initialization to show all instance and locations
    debug!("Re-generating tray menu to show all available instances and locations as we have connected to the database...");
    reload_tray_menu(&app_handle).await;
    debug!("Tray menu has been re-generated successfully");

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
    debug!("Starting the main application event loop...");
    app.run(|app_handle, event| match event {
        // prevent shutdown on window close
        RunEvent::ExitRequested { api, .. } => {
            debug!("Received exit request");
            api.prevent_exit();
        }
        // handle shutdown
        RunEvent::Exit => {
            debug!("Exiting the application's main event loop...");
            let app_state: State<AppState> = app_handle.state();
            app_state.quit(app_handle);
        }
        _ => {
            trace!("Received event: {event:?}");
        }
    });
}
