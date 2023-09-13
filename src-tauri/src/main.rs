// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::State;
pub mod database;
pub mod error;

fn main() {
    tauri::Builder::default()
      .setup(|app| {
            let handle = app.handle();
            tauri::async_runtime::spawn(async move  {
              let db = database::init_db(&handle).await.expect("Database initialize should succeed");
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
