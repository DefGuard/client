use std::net::AddrParseError;

use base64;
use local_ip_address::Error as LocalIpError;
use sqlx;
use thiserror::Error;
use wireguard_rs::{error::WireguardInterfaceError, IpAddrParseError};

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
}
