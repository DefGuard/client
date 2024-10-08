//! Log watcher for observing and parsing `defguard-service` log files
//!
//! This is meant to handle passing relevant logs from `defguard-service` daemon to the client GUI.
//! The watcher monitors a given directory for any changes. Whenever a change is detected
//! it parses the log files and sends logs relevant to a specified interface to the fronted.

use std::{
    fs::{read_dir, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tauri::{async_runtime::TokioJoinHandle, AppHandle, Manager};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::Level;

use crate::{
    appstate::AppState, database::models::Id, error::Error, utils::get_service_log_dir,
    ConnectionType,
};

const DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Error)]
pub enum LogWatcherError {
    #[error(transparent)]
    TauriError(#[from] tauri::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
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

#[derive(Debug)]
pub struct ServiceLogWatcher<'a> {
    interface_name: String,
    log_level: Level,
    from: Option<DateTime<Utc>>,
    log_dir: &'a Path,
    current_log_file: Option<PathBuf>,
    handle: AppHandle,
    cancellation_token: CancellationToken,
    event_topic: String,
}

impl<'a> ServiceLogWatcher<'a> {
    #[must_use]
    pub fn new(
        handle: AppHandle,
        cancellation_token: CancellationToken,
        event_topic: String,
        interface_name: String,
        log_level: Level,
        from: Option<DateTime<Utc>>,
    ) -> Self {
        // get log file directory
        let log_dir = get_service_log_dir();
        info!("Log dir: {log_dir:?}");
        Self {
            interface_name,
            log_level,
            from,
            log_dir,
            current_log_file: None,
            handle,
            cancellation_token,
            event_topic,
        }
    }

    /// Run the log watcher
    ///
    /// Setup a directory watcher with a 2 second debounce and parse the log dir on each change.
    pub fn run(&mut self) -> Result<(), LogWatcherError> {
        // get latest log file
        let latest_log_file = self.get_latest_log_file()?;
        debug!("found latest log file: {latest_log_file:?}");
        self.current_log_file = latest_log_file;

        // indefinitely watch for changes
        loop {
            self.parse_log_dir()?;
            if self.cancellation_token.is_cancelled() {
                break;
            };
        }

        Ok(())
    }

    /// Parse the log file directory
    ///
    /// Analyzing the directory consists of finding the latest log file,
    /// parsing log lines and emitting tauri events whenever relevant logs are found.
    /// Current log file and latest read position are stored between runs
    /// so only new log lines are sent to the frontend whenever a change in
    /// the directory is detected.
    fn parse_log_dir(&mut self) -> Result<(), LogWatcherError> {
        // read and parse file from last position
        if let Some(log_file) = &self.current_log_file {
            let file = File::open(log_file)?;
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            let mut parsed_lines = Vec::new();
            loop {
                let size = reader.read_line(&mut line)?;
                if size == 0 {
                    // emit event with all relevant log lines
                    if !parsed_lines.is_empty() {
                        self.handle.emit_all(&self.event_topic, &parsed_lines)?;
                    }
                    parsed_lines.clear();

                    sleep(DELAY);

                    let latest_log_file = self.get_latest_log_file()?;
                    if latest_log_file.is_some() && latest_log_file != self.current_log_file {
                        self.current_log_file = latest_log_file;
                        break;
                    }
                } else {
                    if let Some(parsed_line) = self.parse_log_line(&line)? {
                        parsed_lines.push(parsed_line);
                    }
                    line.clear();
                }
                if self.cancellation_token.is_cancelled() {
                    info!(
                        "Received cancellation request. Stopping log watcher for interface {}",
                        self.interface_name
                    );
                    break;
                }
            }
        }

        Ok(())
    }

    /// Parse a service log line
    ///
    /// Deserializes the log line into a known struct and checks if the line is relevant
    /// to the specified interface. Also performs filtering by log level and optional timestamp.
    fn parse_log_line(&self, line: &str) -> Result<Option<LogLine>, LogWatcherError> {
        trace!("Parsing log line: {line}");
        let log_line = serde_json::from_str::<LogLine>(line)?;
        trace!("Parsed log line into: {log_line:?}");

        // filter by log level
        if log_line.level > self.log_level {
            debug!(
                "Log level {} is above configured verbosity threshold {}. Skipping line...",
                log_line.level, self.log_level
            );
            return Ok(None);
        }

        // filter by optional timestamp
        if let Some(from) = self.from {
            if log_line.timestamp < from {
                debug!("Timestamp is before configured threshold {from}. Skipping line...");
                return Ok(None);
            }
        }

        // publish all log lines with a matching interface name or with no interface name specified
        if let Some(ref span) = log_line.span {
            if let Some(interface_name) = &span.interface_name {
                if interface_name != &self.interface_name {
                    trace!("Interface name {interface_name} is not the configured name {}. Skipping line...", self.interface_name);
                    return Ok(None);
                }
            }
        }

        Ok(Some(log_line))
    }

    /// Find the latest log file in directory
    ///
    /// Log files are rotated daily and have a knows naming format,
    /// with the last 10 characters specifying a date (e.g. `2023-12-15`).
    fn get_latest_log_file(&self) -> Result<Option<PathBuf>, LogWatcherError> {
        debug!("Getting latest log file from directory: {:?}", self.log_dir);
        let entries = read_dir(self.log_dir)?;

        let mut latest_log = None;
        let mut latest_time = NaiveDate::MIN;
        for entry in entries.flatten() {
            // skip directories
            if entry.metadata()?.is_file() {
                let filename = entry.file_name().to_string_lossy().into_owned();
                if let Some(timestamp) = extract_timestamp(&filename) {
                    if timestamp > latest_time {
                        latest_time = timestamp;
                        latest_log = Some(entry.path());
                    }
                }
            }
        }

        Ok(latest_log)
    }
}

fn extract_timestamp(filename: &str) -> Option<NaiveDate> {
    trace!("Extracting timestamp from log file name: {filename}");
    // we know that the date is always in the last 10 characters
    let split_pos = filename.char_indices().nth_back(9)?.0;
    let timestamp = &filename[split_pos..];
    // parse and convert to `NaiveDate`
    NaiveDate::parse_from_str(timestamp, "%Y-%m-%d").ok()
}

/// Starts a log watcher in a separate thread
///
/// The watcher parses `defguard-service` log files and extracts logs relevant
/// to the WireGuard interface for a given location.
/// Logs are then transmitted to the frontend by using `tauri` `Events`.
/// Returned value is the name of an event topic to monitor.
pub async fn spawn_log_watcher_task(
    handle: AppHandle,
    location_id: Id,
    interface_name: String,
    connection_type: ConnectionType,
    log_level: Level,
    from: Option<String>,
) -> Result<String, Error> {
    info!("Spawning log watcher task for location ID {location_id}, interface {interface_name}");
    let app_state = handle.state::<AppState>();

    // parse `from` timestamp
    let from = from.and_then(|from| DateTime::<Utc>::from_str(&from).ok());

    let connection_type = if connection_type.eq(&ConnectionType::Tunnel) {
        "Tunnel"
    } else {
        "Location"
    };
    let event_topic = format!("log-update-{connection_type}-{location_id}");
    debug!("Using event topic: {event_topic}");

    // explicitly clone before topic is moved into the closure
    let topic_clone = event_topic.clone();
    let interface_name_clone = interface_name.clone();
    let handle_clone = handle.clone();

    // prepare cancellation token
    let token = CancellationToken::new();
    let token_clone = token.clone();

    // spawn task
    let _join_handle: TokioJoinHandle<Result<(), LogWatcherError>> = tokio::spawn(async move {
        let mut log_watcher = ServiceLogWatcher::new(
            handle_clone,
            token_clone,
            topic_clone,
            interface_name_clone,
            log_level,
            from,
        );
        log_watcher.run()?;
        Ok(())
    });

    // store `CancellationToken` to manually stop watcher thread
    let mut log_watchers = app_state
        .log_watchers
        .lock()
        .expect("Failed to lock log watchers mutex");
    if let Some(old_token) = log_watchers.insert(interface_name.clone(), token) {
        // cancel previous log watcher for this interface
        debug!("Existing log watcher for interface {interface_name} found. Cancelling...");
        old_token.cancel();
    }

    Ok(event_topic)
}

/// Stops the log watcher thread
pub fn stop_log_watcher_task(handle: &AppHandle, interface_name: &str) -> Result<(), Error> {
    info!("Stopping log watcher task for interface {interface_name}");
    let app_state = handle.state::<AppState>();

    // get `CancellationToken` to manually stop watcher thread
    let mut log_watchers = app_state
        .log_watchers
        .lock()
        .expect("Failed to lock log watchers mutex");

    if let Some(token) = log_watchers.remove(interface_name) {
        debug!("Using cancellation token for log watcher on interface {interface_name}");
        token.cancel();
        Ok(())
    } else {
        error!("Log watcher for interface {interface_name} not found.");
        Err(Error::NotFound)
    }
}
