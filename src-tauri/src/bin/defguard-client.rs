//! Defguard desktop client

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, process::ExitCode};
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

fn main() -> ExitCode {
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
        return ExitCode::SUCCESS;
    }

    #[cfg(target_os = "macos")]
    {
        // NetworkExtension completion handlers are delivered on the main queue, which is
        // only serviced while a run loop is running on the main thread.
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = Arc::clone(&done);
        let worker = spawn(move || {
            let code = block_on(defguard_cli::cli_main());
            done_clone.store(true, Ordering::Release);
            code
        });
        spawn_runloop_and_wait_for(&done);
        worker.join().unwrap_or(ExitCode::FAILURE)
    }
    #[cfg(not(target_os = "macos"))]
    block_on(defguard_cli::cli_main())
}
