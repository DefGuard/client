pub mod connection;
pub mod instance;
pub mod location;
pub mod settings;
pub mod tunnel;
pub mod wireguard_keys;

// Typestate structs to make working with optional ids easier
pub type Id = i64;
#[derive(Debug, Clone)]
pub struct NoId;
