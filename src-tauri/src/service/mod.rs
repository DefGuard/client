#[cfg(not(target_os = "macos"))]
pub mod client;
pub mod config;
pub mod proto;

#[cfg(not(target_os = "macos"))]
pub mod daemon;
#[cfg(windows)]
pub mod named_pipe;
pub mod utils;
#[cfg(windows)]
pub mod windows;
