pub mod config;
pub mod daemon;
pub mod error;
pub mod utils;
pub mod version;

#[cfg(windows)]
pub mod named_pipe;
#[cfg(windows)]
pub mod windows;

pub use error::Error;
pub use version::VERSION;
