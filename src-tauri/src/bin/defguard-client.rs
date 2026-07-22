//! Defguard desktop client

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
#[cfg(target_os = "macos")]
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::spawn,
};

use tauri::async_runtime::block_on;

fn main() {
    // Handle --version / -V before starting the GUI.
    defguard_client::check_version_flag("defguard-client");

    #[cfg(target_os = "linux")]
    defguard_client::utils::set_webkitgtk_variables();

    // Without any arguments, launch the user interface.
    if env::args().count() <= 1 {
        defguard_client::gui::run_app();
    } else {
        #[cfg(target_os = "macos")]
        {
            // NetworkExtension completion handlers are delivered on the main queue, which is
            // only serviced while a run loop is running on the main thread.
            let done = Arc::new(AtomicBool::new(false));
            let done_clone = Arc::clone(&done);
            let worker = spawn(move || {
                let _code = block_on(defguard_cli::cli_main());
                done_clone.store(true, Ordering::Release);
            });
            defguard_client::connection::apple::spawn_runloop_and_wait_for(&done);
            let _ = worker.join();
        }
        #[cfg(not(target_os = "macos"))]
        block_on(defguard_cli::cli_main());
    }
}
