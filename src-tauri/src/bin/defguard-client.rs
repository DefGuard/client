//! Defguard desktop client

use std::env;

use tauri::async_runtime;

fn main() {
    // Handle --version / -V before starting the GUI.
    defguard_client::check_version_flag("defguard-client");

    // Without any arguments, launch the user interface.
    if env::args().count() <= 1 {
        defguard_client::gui::run_app();
    } else {
        async_runtime::block_on(defguard_cli::cli_main());
    }
}
