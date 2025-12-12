//! defguard desktop client

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
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
        handle_db_migrations,
        models::{location_stats::LocationStats, tunnel::TunnelStats},
        DB_POOL,
    },
    enterprise::provisioning::handle_client_initialization,
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
    #[cfg(windows)]
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
    #[cfg(target_os = "macos")]
    {
        use defguard_client::{
            apple::get_managers_for_tunnels_and_locations, utils::get_all_tunnels_locations,
        };

        let semaphore = Arc::new(AtomicBool::new(false));
        let semaphore_clone = Arc::clone(&semaphore);

        let handle = tauri::async_runtime::spawn(async move {
            if let Err(err) = defguard_client::apple::sync_locations_and_tunnels().await {
                error!("Failed to sync locations and tunnels: {err}");
            }
            semaphore_clone.store(true, Ordering::Release);
        });
        defguard_client::apple::spawn_runloop_and_wait_for(semaphore);
        let _ = handle.await;

        let (tunnels, locations) = get_all_tunnels_locations().await;
        let handle = app_handle.clone();
        // Observer thread is blocking, so its better not to mess with the tauri runtime,
        // hence std::thread::spawn.
        std::thread::spawn(move || {
            defguard_client::apple::observer_thread(get_managers_for_tunnels_and_locations(
                &tunnels, &locations,
            ));
            error!("VPN observer thread has exited unexpectedly, quitting the app.");
            handle.exit(0);
        });

        let handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            defguard_client::apple::connection_state_update_thread(&handle).await;
            error!("Connection state update thread has exited unexpectedly, quitting the app.");
            handle.exit(0);
        });
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
            command_set_app_config,
            get_provisioning_config,
            get_platform_header
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
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Create Help menu on macOS.
            // https://github.com/tauri-apps/tauri/issues/9371
            #[cfg(target_os = "macos")]
            {
                use tauri_plugin_opener::OpenerExt;

                const DOC_ITEM_ID: &str = "doc";
                const REPORT_ITEM_ID: &str = "issue";
                const DOC_URL: &str = "https://docs.defguard.net/using-defguard-for-end-users/desktop-client";
                const REPORT_URL: &str = "https://github.com/DefGuard/client/issues/new?labels=bug&template=bug_report.md";
                if let Some(menu) = app.menu() {
                    if let Some(help_submenu) = menu.get(tauri::menu::HELP_SUBMENU_ID) {
                        let report_item = tauri::menu::MenuItem::with_id(
                            app,
                            REPORT_ITEM_ID,
                            "Report an issue",
                            true,
                            None::<&str>,
                        )?;
                        let _ = help_submenu.as_submenu_unchecked().append(&report_item);
                        let doc_item = tauri::menu::MenuItem::with_id(
                            app,
                            DOC_ITEM_ID,
                            "Defguard Desktop Client Help",
                            true,
                            None::<&str>,
                        )?;
                        let _ = help_submenu.as_submenu_unchecked().append(&doc_item);
                    }
                }
                app.on_menu_event(move |app, event| {
                    let id = event.id();
                    if id == DOC_ITEM_ID {
                        let _ = app.opener().open_url(DOC_URL, None::<&str>);
                    } else if id == REPORT_ITEM_ID {
                        let _ = app.opener().open_url(REPORT_URL, None::<&str>);
                    }
                });
            }

            // Register for Linux and debug Windows builds.
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

            // run DB migrations
            tauri::async_runtime::block_on(handle_db_migrations());

            // Check if client needs to be initialized
            // and try to load provisioning config if necessary
            let provisioning_config =
                tauri::async_runtime::block_on(handle_client_initialization(app_handle));

            let state = AppState::new(config, provisioning_config);
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
            {
                set_perms(&data_dir);
                set_perms(&log_dir);
            }

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
            #[cfg(target_os = "macos")]
            {
                let semaphore = Arc::new(AtomicBool::new(false));
                let semaphore_clone = Arc::clone(&semaphore);

                let handle = tauri::async_runtime::spawn(async move {
                    let _ = close_all_connections().await;
                    // This will clean the database file, pruning write-ahead log.
                    DB_POOL.close().await;
                    semaphore_clone.store(true, Ordering::Release);
                });
                // Obj-C API needs a runtime, but at this point Tauri has closed its runtime, so
                // create a temporary one.
                defguard_client::apple::spawn_runloop_and_wait_for(semaphore);
                tauri::async_runtime::block_on(async move {
                    let _ = handle.await;
                });
            }
            #[cfg(not(target_os = "macos"))]
            {
                tauri::async_runtime::block_on(async move {
                    let _ = close_all_connections().await;
                    // This will clean the database file, pruning write-ahead log.
                    DB_POOL.close().await;
                });
            }
        }
        _ => {
            trace!("Received event: {event:?}");
        }
    });
}
