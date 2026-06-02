pub mod config;
pub mod daemon;
pub mod error;
pub mod utils;

#[cfg(windows)]
pub mod named_pipe;
#[cfg(windows)]
pub mod windows;

pub use common::VERSION;
pub use error::Error;
