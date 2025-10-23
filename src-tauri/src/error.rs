use std::net::AddrParseError;

use defguard_wireguard_rs::{error::WireguardInterfaceError, net::IpAddrParseError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Application config directory error: {0}")]
    Config(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Migrate error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("Wireguard error: {0}")]
    WireguardError(#[from] WireguardInterfaceError),
    #[error("WireGuard key error: {0}")]
    KeyDecode(#[from] base64::DecodeError),
    #[error("IP address/mask error: {0}")]
    IpAddrMask(#[from] IpAddrParseError),
    #[error("IP address parse error: {0}")]
    AddrParse(#[from] AddrParseError),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Failed to parse timestamp")]
    Datetime,
    #[error("Object not found")]
    NotFound,
    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("Failed to parse str to enum")]
    StrumError(#[from] strum::ParseError),
    #[error("Required resource not found {0}")]
    ResourceNotFound(String),
    #[error("Config parse error {0}")]
    ConfigParseError(String),
    #[error("Command failed: {0}")]
    CommandError(String),
    #[error("Core is not enterprise")]
    CoreNotEnterprise,
    #[error("Instance has no config polling token")]
    NoToken,
    #[error("Failed to lock app state member.")]
    StateLockFail,
    #[error("Failed to acquire lock on mutex. {0}")]
    PoisonError(String),
    #[error("Failed to convert value. {0}")]
    ConversionError(String),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

// we must manually implement serde::Serialize
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
