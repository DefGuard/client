// FIXME: actually refactor errors instead
#![allow(clippy::result_large_err)]

#[cfg(target_os = "macos")]
pub mod apple;
pub mod appstate;
pub mod commands;
pub mod enterprise;
pub mod events;
pub mod log_watcher;
pub mod periodic;
pub mod service;
pub mod tray;
pub mod utils;
pub mod window_manager;

pub use defguard_client_core::{
    app_config,
    app_data_dir,
    connection,
    database,
    error,
    get_aggregation,
    into_location,
    proxy,
    version::{
        Version, CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, LOG_FILENAME, MIN_CORE_VERSION,
        MIN_PROXY_VERSION,
    },
    wg_config,
    // Shared types
    CommonConnection,
    CommonConnectionInfo,
    CommonLocationStats,
    CommonWireguardFields,
    ConnectionType,
    // DateTime aggregation
    DateTimeAggregation,
    // Constants
    DEFAULT_ROUTE_IPV4,
    DEFAULT_ROUTE_IPV6,
};

#[cfg(unix)]
pub use defguard_client_core::set_perms;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA"));
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub use defguard_client_common::{check_version_flag, version_string};

#[macro_use]
extern crate log;

/// Converts a tauri emit result into our error type.
#[must_use]
pub fn tauri_err_to_app_err(e: tauri::Error) -> defguard_client_core::error::Error {
    defguard_client_core::error::Error::Tauri(e.to_string())
}
