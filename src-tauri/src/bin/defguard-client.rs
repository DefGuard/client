//! defguard desktop client

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, str::FromStr, sync::LazyLock};

#[cfg(unix)]
use defguard_client::set_perms;
#[cfg(windows)]
use defguard_client::utils::sync_connections;
use defguard_client::{
    active_connections::close_all_connections,
    app_config::AppConfig,
    appstate::AppState,
    commands::*,
    database::{
        models::{location_stats::LocationStats, tunnel::TunnelStats},
        DB_POOL,
    },
    periodic::run_periodic_tasks,
    service,
    tray::{configure_tray_icon, setup_tray, show_main_window},
    utils::load_log_targets,
    VERSION,
};
use log::{Level, LevelFilter};
use tauri::{AppHandle, Builder, Manager, RunEvent, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};

#[macro_use]
extern crate log;

// For tauri logging plugin:
// if found in metadata target name it will ignore the log if it was below info level.
const LOGGING_TARGET_IGNORE_LIST: [&str; 5] = ["tauri", "sqlx", "hyper", "h2", "tower"];

static LOG_INCLUDES: LazyLock<Vec<String>> = LazyLock::new(load_log_targets);

async fn startup(app_handle: &AppHandle) {
    debug!("Running database migrations, if there are any.");
    sqlx::migrate!()
        .run(&*DB_POOL)
        .await
        .expect("Failed to apply database migrations.");
    debug!("Applied all database migrations that were pending. If any.");
    debug!("Database setup has been completed successfully.");

    debug!("Purging old stats from the database.");
    if let Err(err) = LocationStats::purge(&*DB_POOL).await {
        error!("Failed to purge location stats: {err}");
    } else {
        debug!("Old location stats have been purged successfully.");
    }
    if let Err(err) = TunnelStats::purge(&*DB_POOL).await {
        error!("Failed to purge tunnel stats: {err}");
    } else {
        debug!("Old tunnel stats have been purged successfully.");
    }

    // Sync already active connections on windows.
    // When windows is restarted, the app doesn't close the active connections
    // and they are still running after the restart. We sync them here to
    // reflect the real system's state.
    // TODO: Find a way to intercept the shutdown event and close all connections
    #[cfg(target_os = "windows")]
    {
        match sync_connections(app_handle).await {
            Ok(_) => {
                info!(
                    "Synchronized application's active connections with the connections \
                    already open on the system, if there were any."
                )
            }
            Err(err) => {
                warn!(
                    "Failed to synchronize application's active connections with the connections \
                    already open on the system. \
                    The connections' state in the application may not reflect system's state. \
                    Reconnect manually to reset them. Error: {err}"
                )
            }
        };
    }

    // Run periodic tasks.
    let periodic_tasks_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        run_periodic_tasks(&periodic_tasks_handle).await;
        // One of the tasks exited, so something went wrong, quit the app
        error!("One of the periodic tasks has stopped unexpectedly. Exiting the application.");
        periodic_tasks_handle.exit(0);
    });
    debug!("Periodic tasks have been started.");

    // Load tray menu after database initialization, so all instance and locations can be shown.
    debug!(
        "Re-generating tray menu to show all available instances and locations as we have \
        connected to the database."
    );
    if let Err(err) = setup_tray(app_handle).await {
        error!("Failed to setup system tray: {err}");
    }
    let state = app_handle.state::<AppState>();
    let theme = state.app_config.lock().unwrap().tray_theme;
    match configure_tray_icon(app_handle, theme).await {
        Ok(()) => info!("System tray configured."),
        Err(err) => error!("Failed to configure system tray: {err}"),
    }
    debug!("Tray menu has been re-generated successfully.");
}

fn main() {
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
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(not(target_os = "macos"))]
                let _ = window.hide();

                #[cfg(target_os = "macos")]
                let _ = tauri::AppHandle::hide(window.app_handle());

                api.prevent_close();
            }
        })
        // Initialize plugins here, except for `tauri_plugin_log` which is handled in `setup()`.
        // Single instance plugin should always be the first to register.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Running instance might be hidden, so show it.
            show_main_window(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            // Register for linux and dev windows builds
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            let app_handle = app.app_handle();

            // Prepare `AppConfig`.
            let config = AppConfig::new(app_handle);

            // Setup logging.

            // If deriving from env value fails, use config default (env overrides config file).
            let config_log_level = config.log_level;
            let log_level = match &env::var("DEFGUARD_CLIENT_LOG_LEVEL") {
                Ok(env_value) => LevelFilter::from_str(env_value).unwrap_or(config_log_level),
                Err(_) => config_log_level,
            };
            app_handle.plugin(
                tauri_plugin_log::Builder::new()
                    .format(move |out, message, record| {
                        out.finish(format_args!(
                            "{}[{}][{}] {}",
                            tauri_plugin_log::TimezoneStrategy::UseUtc
                                .get_now()
                                // Sets the time format. Service's logs have a subsecond part, so we
                                // also need to include it here, otherwise the logs couldn't be sorted
                                // correctly when displayed together in the UI.
                                .format(&time::macros::format_description!(
                                "[[[year]-[month]-[day]][[[hour]:[minute]:[second].[subsecond]]"
                            ))
                                .unwrap(),
                            record.level(),
                            record.target(),
                            message
                        ));
                    })
                    .targets([
                        Target::new(TargetKind::Stdout),
                        Target::new(TargetKind::LogDir { file_name: None }),
                    ])
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
                        // Log all errors, warnings and infos.
                        let level = metadata.level();
                        if level == LevelFilter::Error
                            || level == LevelFilter::Warn
                            || level == LevelFilter::Info
                        {
                            return true;
                        }
                        // Otherwise do not log these targets.
                        for target in &LOGGING_TARGET_IGNORE_LIST {
                            if metadata.target().contains(target) {
                                return false;
                            }
                        }
                        true
                    })
                    .build(),
            )?;

            let state = AppState::new(config);
            app.manage(state);

            info!("App setup completed, log level: {log_level}");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri application");

    info!("Starting Defguard client version {VERSION}");

    // Run application.
    debug!("Starting the main application event loop.");
    app.run(|app_handle, event| match event {
        // Startup tasks
        RunEvent::Ready => {
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| "UNDEFINED DATA DIRECTORY".into());
            let log_dir = app_handle
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| "UNDEFINED LOG DIRECTORY".into());

            // Ensure directories have appropriate permissions (dg25-28).
            #[cfg(unix)]
            set_perms(&data_dir);
            #[cfg(unix)]
            set_perms(&log_dir);
            info!(
                "Application data (database file) will be stored in: {data_dir:?} and application \
                logs in: {log_dir:?}. Logs of the background Defguard service responsible for \
                managing VPN connections at the network level will be stored in: {}.",
                service::config::DEFAULT_LOG_DIR
            );
            tauri::async_runtime::block_on(startup(app_handle));

            // Handle Ctrl-C.
            debug!("Setting up Ctrl-C handler.");
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Signal handler failure");
                debug!("Ctrl-C handler: quitting the app");
                app_handle_clone.exit(0);
            });
            debug!("Ctrl-C handler has been set up successfully");
        }
        RunEvent::ExitRequested { code, api, .. } => {
            debug!("Received exit request");
            // `code` is `None` when the exit is requested by user interaction.
            if code.is_none() {
                // Prevent shutdown on window close.
                api.prevent_exit();
            }
        }
        // Handle shutdown.
        RunEvent::Exit => {
            debug!("Exiting the application's main event loop.");
            tauri::async_runtime::block_on(async {
                let _ = close_all_connections().await;
                // This will clean the database file, pruning write-ahead log.
                DB_POOL.close().await;
            });
        }
        _ => {
            trace!("Received event: {event:?}");
        }
    });
}
