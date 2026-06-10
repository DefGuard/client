use serde::{Deserialize, Serialize};

pub mod connection;
pub mod instance;
pub mod location;
pub mod location_stats;
pub mod tunnel;
#[cfg(target_os = "macos")]
pub mod tunnel_configuration;
pub mod wireguard_keys;

// Typestate structs to make working with optional IDs easier
pub type Id = i64;
#[derive(Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NoId;

const PURGE_DURATION: chrono::Duration = chrono::Duration::days(30);
