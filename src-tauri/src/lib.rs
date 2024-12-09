use std::{fmt, path::PathBuf};

use chrono::NaiveDateTime;
use database::models::NoId;
use serde::{Deserialize, Serialize};

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
    tonic::include_proto!("defguard.proxy");
}

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA"));
// This must match tauri.bundle.identifier from tauri.conf.json.
static BUNDLE_IDENTIFIER: &str = "net.defguard";
// Returns the path to the user’s data directory.
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

use self::database::models::Id;

/// Common fields for Tunnel and Location
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonWireguardFields {
    pub instance_id: Id,
    // Native id of network from defguard
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
