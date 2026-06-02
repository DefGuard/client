use std::{fmt, path::PathBuf};

use chrono::{Duration, NaiveDateTime, Utc};
use database::models::Id;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::{
    fs::{set_permissions, Permissions},
    os::unix::fs::PermissionsExt,
};

pub mod app_config;
pub mod connection;
pub mod database;
pub mod error;
pub mod events;
pub mod proxy;
pub mod version;
pub mod wg_config;

// Re-export proto module for backward compatibility within core.
pub use defguard_client_proto::defguard as proto;

use crate::database::models::NoId;

#[macro_use]
extern crate log;

const BUNDLE_IDENTIFIER: &str = "net.defguard";

/// Returns the path to the user's data directory.
#[must_use]
pub fn app_data_dir() -> Option<PathBuf> {
    dirs_next::data_dir().map(|dir| dir.join(BUNDLE_IDENTIFIER))
}

/// Ensures path has appropriate permissions set (dg25-28):
/// - 700 for directories
/// - 600 for files
#[cfg(unix)]
pub fn set_perms(path: &std::path::Path) {
    let perms = if path.is_dir() { 0o700 } else { 0o600 };
    if let Err(err) = set_permissions(path, Permissions::from_mode(perms)) {
        log::warn!(
            "Failed to set permissions on path {}: {err}",
            path.display()
        );
    }
}

/// Location type used in commands to check if we use tunnel or location
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum ConnectionType {
    Tunnel,
    Location,
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectionType::Tunnel => write!(f, "tunnel"),
            ConnectionType::Location => write!(f, "location"),
        }
    }
}

/// Common fields for Tunnel and Location
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonWireguardFields {
    pub instance_id: Id,
    pub network_id: Id,
    pub name: String,
    pub address: String,
    pub pubkey: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub dns: Option<String>,
    pub route_all_traffic: bool,
}

/// Common fields for Connection and TunnelConnection due to shared command
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonConnection<I = NoId> {
    pub id: I,
    pub location_id: Id,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub connection_type: ConnectionType,
}

/// Common fields for LocationStats and TunnelStats due to shared command
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonLocationStats<I = NoId> {
    pub id: I,
    pub location_id: Id,
    pub upload: i64,
    pub download: i64,
    pub last_handshake: i64,
    pub collected_at: NaiveDateTime,
    pub listen_port: u32,
    pub persistent_keepalive_interval: Option<u16>,
    pub connection_type: ConnectionType,
}

/// Common fields for ConnectionInfo and TunnelConnectionInfo due to shared command
#[derive(Debug, Serialize)]
pub struct CommonConnectionInfo {
    pub id: Id,
    pub location_id: Id,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub upload: Option<i32>,
    pub download: Option<i32>,
}

pub const DEFAULT_ROUTE_IPV4: &str = "0.0.0.0/0";
pub const DEFAULT_ROUTE_IPV6: &str = "::/0";

pub enum DateTimeAggregation {
    Hour,
    Second,
}

impl DateTimeAggregation {
    #[must_use]
    pub fn fstring(&self) -> &'static str {
        match self {
            Self::Hour => "%Y-%m-%d %H:00:00",
            Self::Second => "%Y-%m-%d %H:%M:%S",
        }
    }
}

pub fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, error::Error> {
    let aggregation = match Utc::now().naive_utc() - from {
        duration if duration >= Duration::hours(8) => Ok(DateTimeAggregation::Hour),
        duration if duration < Duration::zero() => Err(error::Error::InternalError(format!(
            "Negative duration between dates: now ({}) and {from}",
            Utc::now().naive_utc(),
        ))),
        _ => Ok(DateTimeAggregation::Second),
    }?;
    Ok(aggregation)
}

use database::models::location::{
    infer_mfa_method, Location, LocationMfaMode, ServiceLocationMode,
};
use defguard_client_proto::defguard::client_types::DeviceConfig;

#[must_use]
pub fn into_location(dev_config: DeviceConfig, instance_id: Id) -> Location<NoId> {
    use LocationMfaMode as MfaMode;
    use ServiceLocationMode as SLocationMode;

    let location_mfa_mode = match dev_config.location_mfa_mode {
        Some(_location_mfa_mode) => dev_config.location_mfa_mode().into(),
        None =>
        {
            #[allow(deprecated)]
            if dev_config.mfa_enabled {
                MfaMode::Internal
            } else {
                MfaMode::Disabled
            }
        }
    };

    let service_location_mode = match dev_config.service_location_mode {
        Some(_service_location_mode) => dev_config.service_location_mode().into(),
        None => SLocationMode::Disabled,
    };

    Location {
        id: NoId,
        instance_id,
        network_id: dev_config.network_id,
        name: dev_config.network_name,
        address: dev_config.assigned_ip,
        pubkey: dev_config.pubkey,
        endpoint: dev_config.endpoint,
        allowed_ips: dev_config.allowed_ips,
        dns: dev_config.dns,
        route_all_traffic: false,
        keepalive_interval: dev_config.keepalive_interval.into(),
        location_mfa_mode,
        service_location_mode,
        mfa_method: infer_mfa_method(location_mfa_mode, None),
        posture_check_required: dev_config.posture_check_required.unwrap_or_default(),
    }
}
