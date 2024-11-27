use serde::{Deserialize, Serialize};

pub mod connection;
pub mod instance;
pub mod location;
pub mod location_stats;
pub mod tunnel;
pub mod wireguard_keys;

// Typestate structs to make working with optional IDs easier
pub type Id = i64;
#[derive(Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NoId;
