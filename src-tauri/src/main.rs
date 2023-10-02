// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod appstate;
pub mod commands;
pub mod database;
pub mod error;
pub mod utils;

use appstate::AppState;
use tauri::{Manager, State};
use tauri_plugin_log::LogTarget;

use tauri::SystemTrayEvent;
mod tray;
use crate::commands::{
    all_instances, all_locations, connect, disconnect, last_connection, location_stats,
    save_device_config, update_instance,
};
use crate::tray::create_tray_menu;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[macro_use]
extern crate log;

// Specify log targets
const LOG_TARGETS: [LogTarget; 2] = [LogTarget::Stdout, LogTarget::LogDir];

// TODO: Refactor later
#[allow(clippy::single_match)]
fn main() {
    let tray_menu = create_tray_menu();
    let system_tray = tauri::SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            all_locations,
            save_device_config,
            all_instances,
            connect,
            disconnect,
            update_instance,
            location_stats,
            last_connection
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
                    std::process::exit(0x0);
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
                info!("Starting main app thread.")
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
