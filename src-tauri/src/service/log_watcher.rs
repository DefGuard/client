//! Log watcher for observing and parsing `defguard-service` log files
//!
//! This is meant to handle passing relevant logs from `defguard-service` daemon to the client GUI.
//! The watcher monitors a given directory for any changes. Whenever a change is detected
//! it parses the log files and sends logs relevant to a specified interface to the fronted.

use crate::{appstate::AppState, error::Error, utils::get_service_log_dir, ConnectionType};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use notify_debouncer_mini::{
    new_debouncer,
    notify::{self, RecursiveMode},
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{
    fs::{read_dir, File},
    io::{BufRead, BufReader},
    path::PathBuf,
    str::FromStr,
    time::{Duration, SystemTime},
};
use tauri::{async_runtime::TokioJoinHandle, AppHandle, Manager};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::Level;

#[derive(Error, Debug)]
pub enum LogWatcherError {
    #[error(transparent)]
    TauriError(#[from] tauri::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    NotifyError(#[from] notify::Error),
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LogLineFields {
    message: String,
    interface_name: Option<String>,
}

pub struct ServiceLogWatcher {
    interface_name: String,
    log_level: Level,
    from: Option<DateTime<Utc>>,
    log_dir: PathBuf,
    current_log_file: Option<PathBuf>,
    current_position: u64,
    handle: AppHandle,
    cancellation_token: CancellationToken,
    event_topic: String,
}

impl ServiceLogWatcher {
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
            current_position: 0,
            handle,
            cancellation_token,
            event_topic,
        }
    }

    /// Run the log watcher
    ///
    /// Setup a directory watcher with a 2 second debounce and parse the log dir on each change.
    pub fn run(&mut self) -> Result<(), LogWatcherError> {
        // setup debouncer
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = new_debouncer(Duration::from_secs(2), tx)?;

        debouncer
            .watcher()
            .watch(&self.log_dir, RecursiveMode::Recursive)?;

        // parse log dir initially before watching for changes
        self.parse_log_dir()?;

        for result in rx {
            if self.cancellation_token.is_cancelled() {
                info!(
                    "Received cancellation request. Stopping log watcher for interface {}",
                    self.interface_name
                );
                break;
            }
            match result {
                Ok(_events) => {
                    self.parse_log_dir()?;
                }
                Err(error) => println!("Error {error:?}"),
            }
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
        // get latest log file
        let latest_log_file = self.get_latest_log_file()?;
        info!("found latest log file: {latest_log_file:?}");

        // check if latest file changed
        if latest_log_file.is_some() && latest_log_file != self.current_log_file {
            self.current_log_file = latest_log_file;
            // reset read position
            self.current_position = 0;
        }

        // read and parse file from last position
        if let Some(log_file) = &self.current_log_file {
            let file = File::open(log_file)?;
            let size = file.metadata()?.len();
            let mut reader = BufReader::new(file);
            reader.seek_relative(self.current_position as i64)?;
            let mut parsed_lines = Vec::new();
            for line in reader.lines() {
                let line = line?;
                if let Some(parsed_line) = self.parse_log_line(line)? {
                    parsed_lines.push(parsed_line);
                }
            }
            // emit event with all relevant log lines
            if !parsed_lines.is_empty() {
                self.handle.emit_all(&self.event_topic, parsed_lines)?;
            }

            // update read position to end of file
            self.current_position = size;
        }
        Ok(())
    }

    /// Parse a service log line
    ///
    /// Deserializes the log line into a known struct and checks if the line is relevant
    /// to the specified interface. Also performs filtering by log level and optional timestamp.
    fn parse_log_line(&self, line: String) -> Result<Option<LogLine>, LogWatcherError> {
        debug!("Parsing log line: {line}");
        let log_line = serde_json::from_str::<LogLine>(&line)?;
        debug!("Parsed log line into: {log_line:?}");

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
        if let Some(interface_name) = &log_line.fields.interface_name {
            if interface_name != &self.interface_name {
                debug!("Interface name {interface_name} is not the configured name {}. Skipping line...", self.interface_name);
                return Ok(None);
            }
        }

        Ok(Some(log_line))
    }

    /// Find the latest log file in directory
    ///
    /// Log files are rotated daily and have a knows naming format,
    /// with the last 10 characters specifying a date (e.g. `2023-12-15`).
    fn get_latest_log_file(&self) -> Result<Option<PathBuf>, LogWatcherError> {
        debug!("Getting latest log file");
        let entries = read_dir(&self.log_dir)?;

        let mut latest_log = None;
        let mut latest_time = SystemTime::UNIX_EPOCH;
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

fn extract_timestamp(filename: &str) -> Option<SystemTime> {
    debug!("Extracting timestamp from log file name: {filename}");
    // we know that the date is always in the last 10 characters
    let split_pos = filename.char_indices().nth_back(9)?.0;
    let timestamp = &filename[split_pos..];
    // parse and convert to `SystemTime`
    if let Ok(timestamp) = NaiveDate::parse_from_str(timestamp, "%Y-%m-%d") {
        let timestamp = timestamp.and_time(NaiveTime::default()).timestamp();
        return Some(SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp as u64));
    }
    None
}

/// Starts a log watcher in a separate thread
///
/// The watcher parses `defguard-service` log files and extracts logs relevant
/// to the WireGuard interface for a given location.
/// Logs are then transmitted to the frontend by using `tauri` `Events`.
/// Returned value is the name of an event topic to monitor.
pub async fn spawn_log_watcher_task(
    handle: AppHandle,
    location_id: i64,
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
pub fn stop_log_watcher_task(handle: AppHandle, interface_name: String) -> Result<(), Error> {
    info!("Stopping log watcher task for interface {interface_name}");
    let app_state = handle.state::<AppState>();

    // get `CancellationToken` to manually stop watcher thread
    let mut log_watchers = app_state
        .log_watchers
        .lock()
        .expect("Failed to lock log watchers mutex");

    match log_watchers.remove(&interface_name) {
        Some(token) => {
            debug!("Using cancellation token for log watcher on interface {interface_name}");
            token.cancel();
            Ok(())
        }
        None => {
            error!("Log watcher for interface {interface_name} not found.");
            Err(Error::NotFound)
        }
    }
}
