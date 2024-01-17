use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
pub mod appstate;
pub mod commands;
pub mod database;
pub mod error;
pub mod latest_app_version;
pub mod service;
pub mod tray;
pub mod utils;
pub mod wg_config;

pub mod proto {
    tonic::include_proto!("enrollment");
}

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

/// Location type used in commands to check if we using tunel or location
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub enum ConnectionType {
    Tunnel,
    Location,
}

#[macro_use]
extern crate log;

/// Common fields for Tunnel and Location
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonWireguardFields {
    pub instance_id: i64,
    // Native id of network from defguard
    pub network_id: i64,
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
pub struct CommonConnection {
    pub id: Option<i64>,
    pub location_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub connection_type: ConnectionType,
}

// Common fields for LocationStats and TunnelStats due to shared command
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonLocationStats {
    pub id: Option<i64>,
    pub location_id: i64,
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
    pub id: i64,
    pub location_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub upload: Option<i32>,
    pub download: Option<i32>,
}
