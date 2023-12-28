pub mod appstate;
pub mod commands;
pub mod database;
pub mod error;
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

#[macro_use]
extern crate log;
