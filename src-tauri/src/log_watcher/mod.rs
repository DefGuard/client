#[cfg(target_os = "macos")]
use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use thiserror::Error;
use tracing::Level;

pub mod global_log_watcher;
pub mod service_log_watcher;

#[derive(Error, Debug)]
pub enum LogWatcherError {
    #[error(transparent)]
    TauriError(#[from] tauri::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    TokioError(#[from] regex::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Error while accessing the log file: {0}")]
    LogPathError(String),
    #[error("Failed to parse log line: {0}")]
    LogParseError(String),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum LogSource {
    /// Service logs (Linux) or VPN Extension logs (macOS)
    /// Serializes to "Vpn" for frontend compatibility
    #[cfg(not(target_os = "macos"))]
    #[serde(rename = "VPN")]
    Service,
    Client,
    #[cfg(target_os = "macos")]
    #[serde(rename = "VPN")]
    VpnExtension,
}

/// Represents a single line in log file
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct LogLine {
    timestamp: DateTime<Utc>,
    #[serde_as(as = "DisplayFromStr")]
    level: Level,
    target: String,
    fields: LogLineFields,
    span: Option<Span>,
    source: Option<LogSource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Span {
    interface_name: Option<String>,
    name: Option<String>,
    peer: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LogLineFields {
    message: String,
}

fn extract_timestamp(filename: &str) -> Option<NaiveDate> {
    trace!("Extracting timestamp from log file name: {filename}");
    // we know that the date is always in the last 10 characters
    let split_pos = filename.char_indices().nth_back(9)?.0;
    let timestamp = &filename[split_pos..];
    // parse and convert to `NaiveDate`
    NaiveDate::parse_from_str(timestamp, "%Y-%m-%d").ok()
}

const HOME_SPLIT_PATTERN: &str = "/Library/Containers/";
const LOGS_LOCATION: &str = "Library/Group Containers/group.net.defguard/Logs/vpn-extension.log";

/// Get the VPN extension log file path on macOS
#[cfg(target_os = "macos")]
fn get_vpn_extension_log_path() -> Result<PathBuf, LogWatcherError> {
    let home = std::env::var("HOME").map_err(|_| {
        LogWatcherError::LogPathError("HOME environment variable not set".to_string())
    })?;

    // If sandboxed, HOME looks like: /Users/username/Library/Containers/net.defguard/Data
    // We need to extract /Users/username from it
    let real_home = if home.contains(HOME_SPLIT_PATTERN) {
        home.split(HOME_SPLIT_PATTERN)
            .next()
            .unwrap_or(&home)
            .to_string()
    } else {
        home
    };

    Ok(PathBuf::from(real_home).join(LOGS_LOCATION))
}
