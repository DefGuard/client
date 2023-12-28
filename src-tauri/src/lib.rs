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

pub enum ConnectionType {
    Tunnel,
    Location,
}

#[macro_use]
extern crate log;
