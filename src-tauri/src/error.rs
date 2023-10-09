use std::net::AddrParseError;

use base64;
use defguard_wireguard_rs::{error::WireguardInterfaceError, net::IpAddrParseError};
use local_ip_address::Error as LocalIpError;
use sqlx;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Config directory error")]
    Config,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Migrate error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("Wireguard error")]
    WireguardError(#[from] WireguardInterfaceError),
    #[error("WireGuard key error")]
    KeyDecode(#[from] base64::DecodeError),
    #[error("IP address/mask error")]
    IpAddrMask(#[from] IpAddrParseError),
    #[error("IP address/mask error")]
    AddrParse(#[from] AddrParseError),
    #[error("Local Ip Error")]
    LocalIpError(#[from] LocalIpError),
    #[error("Internal error")]
    InternalError,
    #[error("Failed to parse timestamp")]
    Datetime,
    #[error("Object not found")]
    NotFound,
    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),
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
