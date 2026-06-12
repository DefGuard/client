pub mod config;
pub mod daemon;
pub mod error;
pub mod utils;

#[cfg(windows)]
pub mod named_pipe;
#[cfg(windows)]
pub mod windows;

pub use defguard_client_common::{check_version_flag, version_string, VERSION};
pub use error::Error;
