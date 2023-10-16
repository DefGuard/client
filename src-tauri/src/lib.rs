pub mod appstate;
pub mod commands;
pub mod database;
pub mod error;
pub mod service;
pub mod tray;
pub mod utils;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[macro_use]
extern crate log;
