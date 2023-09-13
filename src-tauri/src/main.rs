// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, State};
pub mod appstate;
pub mod database;
pub mod error;
use appstate::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle();
            tauri::async_runtime::spawn(async move {
                let app_state: State<AppState> = handle.state();
                let db = database::init_db(&handle)
                    .await
                    .expect("Database initialize should succeed");
                *app_state.db.lock().unwrap() = Some(db);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
