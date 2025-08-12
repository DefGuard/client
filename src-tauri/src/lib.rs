// FIXME: actually refactor errors instead
#![allow(clippy::result_large_err)]
use std::{fmt, path::PathBuf};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use self::database::models::{Id, NoId};

pub mod app_config;
pub mod appstate;
pub mod commands;
pub mod database;
pub mod enterprise;
pub mod error;
pub mod events;
pub mod log_watcher;
pub mod periodic;
pub mod service;
pub mod tray;
pub mod utils;
pub mod wg_config;

pub mod proto {
    use crate::database::models::{
        location::{Location, LocationMfaMode as MfaMode},
        Id, NoId,
    };

    tonic::include_proto!("defguard.proxy");

    impl DeviceConfig {
        #[must_use]
        pub(crate) fn into_location(self, instance_id: Id) -> Location<NoId> {
            let location_mfa_mode = match self.location_mfa_mode {
                Some(_location_mfa_mode) => self.location_mfa_mode().into(),
                None => {
                    // handle legacy core response
                    // DEPRECATED(1.5): superseeded by location_mfa_mode
                    #[allow(deprecated)]
                    if self.mfa_enabled {
                        MfaMode::Internal
                    } else {
                        MfaMode::Disabled
                    }
                }
            };

            Location {
                id: NoId,
                instance_id,
                network_id: self.network_id,
                name: self.network_name,
                address: self.assigned_ip, // Transforming assigned_ip to address
                pubkey: self.pubkey,
                endpoint: self.endpoint,
                allowed_ips: self.allowed_ips,
                dns: self.dns,
                route_all_traffic: false,
                keepalive_interval: self.keepalive_interval.into(),
                location_mfa_mode,
            }
        }
    }
}

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA"));
// This must match tauri.bundle.identifier from tauri.conf.json.
const BUNDLE_IDENTIFIER: &str = "net.defguard";
// Returns the path to the user’s data directory.
#[must_use]
pub fn app_data_dir() -> Option<PathBuf> {
    dirs_next::data_dir().map(|dir| dir.join(BUNDLE_IDENTIFIER))
}

/// Location type used in commands to check if we using tunnel or location
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

#[macro_use]
extern crate log;

/// Common fields for Tunnel and Location
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonWireguardFields {
    pub instance_id: Id,
    // Native network ID from Defguard Core.
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

// Common fields for LocationStats and TunnelStats due to shared command
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
// Common fields for ConnectionInfo and TunnelConnectionInfo due to shared command
#[derive(Debug, Serialize)]
pub struct CommonConnectionInfo {
    pub id: Id,
    pub location_id: Id,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub upload: Option<i32>,
    pub download: Option<i32>,
}
