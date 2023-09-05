// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri::SystemTrayEvent;
mod tray;
use crate::tray::create_tray_menu;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

fn main() {
    let tray_menu = create_tray_menu();
    let system_tray = tauri::SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
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
                        } else {
                            if !main_window
                                .is_visible()
                                .expect("Failed to check main window visibility")
                            {
                                main_window.show().expect("Failed to show main window.");
                            }
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
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            _ => {}
        });
}
