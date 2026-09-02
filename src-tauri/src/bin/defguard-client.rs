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

#[cfg(target_os = "macos")]
use defguard_client::connection::apple::spawn_runloop_and_wait_for;
#[cfg(target_os = "linux")]
use defguard_client::utils::set_webkitgtk_variables;
use defguard_client::{check_version_flag, gui::run_app};
use tauri::async_runtime::block_on;

fn main() {
    // Handle --version / -V before starting the client.
    check_version_flag("defguard-client");

    // On Windows and Linux a deep-link launches a new process with the URL as its only
    // argument. That must start the GUI, not the CLI.
    let is_deep_link = if let Some(value) = env::args().nth(1) {
        value.starts_with("defguard://")
    } else {
        false
    };

    if env::args().count() <= 1 || is_deep_link {
        #[cfg(target_os = "linux")]
        set_webkitgtk_variables();
        run_app();
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
            spawn_runloop_and_wait_for(&done);
            let _ = worker.join();
        }
        #[cfg(not(target_os = "macos"))]
        block_on(defguard_cli::cli_main());
    }
}
