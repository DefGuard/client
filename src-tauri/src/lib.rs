use crate::database::{Connection, TunnelConnection};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
pub mod appstate;
pub mod commands;
pub mod database;
pub mod error;
pub mod service;
pub mod tray;
pub mod utils;
pub mod wg_config;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub enum LocationType {
    Tunnel,
    Location,
}

#[macro_use]
extern crate log;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct CommonConnection {
    pub id: Option<i64>,
    pub location_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub location_type: LocationType,
}
// Implementing From for Connection into CommonConnection
impl From<Connection> for CommonConnection {
    fn from(connection: Connection) -> Self {
        CommonConnection {
            id: connection.id,
            location_id: connection.location_id,
            connected_from: connection.connected_from,
            start: connection.start,
            end: connection.end,
            location_type: LocationType::Location,
        }
    }
}

// Implementing From for TunnelConnection into CommonConnection
impl From<TunnelConnection> for CommonConnection {
    fn from(tunnel_connection: TunnelConnection) -> Self {
        CommonConnection {
            id: tunnel_connection.id,
            location_id: tunnel_connection.tunnel_id, // Assuming you want to map tunnel_id to location_id
            connected_from: tunnel_connection.connected_from,
            start: tunnel_connection.start,
            end: tunnel_connection.end,
            location_type: LocationType::Tunnel, // You need to set the location_type appropriately based on your logic,
        }
    }
}
