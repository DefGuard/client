//! defguard desktop client

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lazy_static::lazy_static;
use log::{Level, LevelFilter};
use tauri::{Manager, State, SystemTrayEvent};
use tauri_plugin_log::LogTarget;

use defguard_client::{
    __cmd__active_connection, __cmd__all_connections, __cmd__all_instances, __cmd__all_locations,
    __cmd__connect, __cmd__disconnect, __cmd__last_connection, __cmd__location_stats,
    __cmd__save_device_config, __cmd__update_instance, __cmd__update_location_routing,
    appstate::AppState,
    commands::{
        active_connection, all_connections, all_instances, all_locations, connect, disconnect,
        last_connection, location_stats, save_device_config, update_instance,
        update_location_routing,
    },
    database,
    tray::create_tray_menu,
    utils::load_log_targets,
};
use std::{env, str::FromStr};

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

    let tray_menu = create_tray_menu();
    let system_tray = tauri::SystemTray::new().with_menu(tray_menu);

    let log_level =
        LevelFilter::from_str(&std::env::var("DEFGUARD_CLIENT_LOG_LEVEL").unwrap_or("info".into()))
            .unwrap_or(LevelFilter::Info);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            all_locations,
            save_device_config,
            all_instances,
            connect,
            disconnect,
            update_instance,
            location_stats,
            all_connections,
            last_connection,
            active_connection,
            update_location_routing,
        ])
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    let app_state: State<AppState> = app.state();
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            let _ = app_state.close_all_connections().await;
                            std::process::exit(0);
                        });
                    });
                }
                "show" => {
                    if let Some(main_window) = app.get_window("main") {
                        if main_window
                            .is_minimized()
                            .expect("Failed to check minimization state")
                        {
                            main_window
                                .unminimize()
                                .expect("Failed to unminimize main window.");
                        } else if !main_window
                            .is_visible()
                            .expect("Failed to check main window visibility")
                        {
                            main_window.show().expect("Failed to show main window.");
                        }
                    }
                }
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
                _ => {}
            },
            _ => {}
        })
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
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
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle();
            tauri::async_runtime::spawn(async move {
                debug!("Initializing database connection");
                let app_state: State<AppState> = handle.state();
                let db = database::init_db(&handle)
                    .await
                    .expect("Database initialization failed");
                *app_state.db.lock().unwrap() = Some(db);
                info!("Database initialization completed");
                info!("Starting main app thread.");
                let result = database::info(&app_state.get_pool()).await;
                info!("Database info result: {:#?}", result);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
