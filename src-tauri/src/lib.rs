// FIXME: actually refactor errors instead
#![allow(clippy::result_large_err)]

use semver::Version;

pub mod active_connections;
pub mod app_config;
#[cfg(target_os = "macos")]
pub mod apple;
pub mod appstate;
pub mod commands;
pub mod enterprise;
pub mod events;
pub mod log_watcher;
pub mod periodic;
pub mod proto;
pub mod service;
pub mod tray;
pub mod utils;
pub mod wg_config;
pub mod window_manager;

// Re-export from core so existing imports keep working.
pub use defguard_client_core::{
    app_data_dir,
    database,
    error,
    get_aggregation,
    set_perms,
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

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA"));
pub const MIN_CORE_VERSION: Version = Version::new(1, 6, 0);
pub const MIN_PROXY_VERSION: Version = Version::new(1, 6, 0);
pub const CLIENT_VERSION_HEADER: &str = "defguard-client-version";
pub const CLIENT_PLATFORM_HEADER: &str = "defguard-client-platform";
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
// Must be without ".log" suffix!
pub const LOG_FILENAME: &str = "defguard-client";

#[macro_use]
extern crate log;

/// Converts a tauri emit result into our error type.
pub fn tauri_err_to_app_err(e: tauri::Error) -> defguard_client_core::error::Error {
    defguard_client_core::error::Error::Tauri(e.to_string())
}
