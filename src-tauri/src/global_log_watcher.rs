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
    time::{Duration, SystemTime},
};

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tauri::{async_runtime::TokioJoinHandle, AppHandle, Manager};
use tauri_plugin_log::LogTarget;
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::Level;

use crate::{
    appstate::AppState, database::models::Id, error::Error, utils::get_service_log_dir,
    ConnectionType,
};

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
pub struct LogDirs {
    // Service
    service_log_dir: PathBuf,
    current_service_log_file: Option<PathBuf>,
    // Client
    client_log_dir: PathBuf,
}

const DELAY: Duration = Duration::from_secs(2);

impl LogDirs {
    #[must_use]
    pub fn new(handle: &AppHandle) -> Result<Self, LogWatcherError> {
        let service_log_dir = get_service_log_dir().to_path_buf();
        let client_log_dir =
            handle
                .path_resolver()
                .app_log_dir()
                .ok_or(LogWatcherError::LogPathError(
                    "Path to client logs directory is empty.".to_string(),
                ))?;
        println!(
            "Log dirs are: {:?} and {:?}",
            service_log_dir, client_log_dir
        );

        return Ok(Self {
            service_log_dir,
            current_service_log_file: None,
            client_log_dir,
        });
    }

    /// Find the latest log file in directory for the service
    ///
    /// Log files are rotated daily and have a known naming format,
    /// with the last 10 characters specifying a date (e.g. `2023-12-15`).
    fn get_latest_log_file(&self) -> Result<Option<PathBuf>, LogWatcherError> {
        trace!(
            "Getting latest log file from directory: {:?}",
            self.service_log_dir
        );
        let entries = read_dir(&self.service_log_dir)?;

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

    fn get_current_service_file(&self) -> Result<File, LogWatcherError> {
        debug!(
            "Opening service log file: {:?}",
            self.current_service_log_file
        );
        match &self.current_service_log_file {
            Some(path) => {
                let file = File::open(path)?;
                debug!(
                    "Successfully opened service log file at {:?}",
                    self.current_service_log_file
                );
                Ok(file)
            }
            None => Err(LogWatcherError::LogPathError(format!(
                "Couldn't find service log file at: {:?}",
                self.current_service_log_file
            ))),
        }
    }

    fn get_client_file(&self) -> Result<File, LogWatcherError> {
        debug!(
            "Opening the log file for the client, using directory: {:?}",
            self.client_log_dir
        );
        let dir_str = self
            .client_log_dir
            .to_str()
            .ok_or(LogWatcherError::LogPathError(format!(
                "Couldn't convert the client log directory path ({:?}) to a string slice",
                self.client_log_dir
            )))?;
        let path = format!("{}/defguard-client.log", dir_str);
        debug!("Constructed client log file path: {path}");
        let file = File::open(&path)?;
        debug!("Client log file at {:?} opened successfully", path);
        Ok(file)
    }
}

#[derive(Debug)]
pub struct GlobalLogWatcher {
    log_level: Level,
    from: Option<DateTime<Utc>>,
    log_dirs: LogDirs,
    handle: AppHandle,
    cancellation_token: CancellationToken,
    event_topic: String,
}

impl GlobalLogWatcher {
    #[must_use]
    pub fn new(
        handle: AppHandle,
        cancellation_token: CancellationToken,
        event_topic: String,
        log_level: Level,
        from: Option<DateTime<Utc>>,
    ) -> Result<Self, LogWatcherError> {
        Ok(Self {
            log_level,
            from,
            log_dirs: LogDirs::new(&handle)?,
            handle,
            cancellation_token,
            event_topic,
        })
    }

    /// Run the log watcher
    ///
    /// Setup a directory watcher with a 2 second debounce and parse the log dir on each change.
    pub fn run(&mut self) -> Result<(), LogWatcherError> {
        loop {
            self.parse_log_dirs()?;
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
    fn parse_log_dirs(&mut self) -> Result<(), LogWatcherError> {
        debug!("Parsing log directories");
        self.log_dirs.current_service_log_file = self.get_latest_log_file()?;
        debug!(
            "Latest service log file found: {:?}",
            self.log_dirs.current_service_log_file
        );

        debug!("Opening log files");
        let mut service_reader = if let Ok(file) = self.log_dirs.get_current_service_file() {
            debug!("Service log file opened successfully");
            Some(BufReader::new(file))
        } else {
            None
        };
        let mut client_reader = if let Ok(file) = self.log_dirs.get_client_file() {
            debug!("Client log file opened successfully");
            Some(BufReader::new(file))
        } else {
            None
        };

        debug!("Checking if log files are available");
        if service_reader.is_none() && client_reader.is_none() {
            warn!(
                "Couldn't read files at {:?} and {:?}, there will be no logs reported in the client.",
                self.log_dirs.current_service_log_file, self.log_dirs.client_log_dir
            );
            // Wait for logs to appear.
            sleep(DELAY);
            return Ok(());
        }
        debug!("Log files are available, starting to read lines.");

        let mut service_line = String::new();
        let mut client_line = String::new();
        let mut parsed_lines = Vec::new();
        let mut service_line_read: usize = 0;
        let mut client_line_read: usize = 0;

        debug!("Starting the log reading loop");
        loop {
            // Service
            if let Some(reader) = &mut service_reader {
                trace!("Reading service log lines");
                loop {
                    service_line_read = reader.read_line(&mut service_line)?;
                    if service_line_read == 0 {
                        trace!("Read 0 bytes from service log file, probably reached EOF.");
                        let latest_log_file = self.get_latest_log_file()?;
                        if latest_log_file.is_some()
                            && latest_log_file != self.log_dirs.current_service_log_file
                        {
                            debug!(
                                "Found a new service log file: {:?}, switching to it.",
                                latest_log_file
                            );
                            self.log_dirs.current_service_log_file = latest_log_file;
                            break;
                        }
                    } else {
                        trace!("Read service log line: {service_line:?}");
                        if let Some(parsed_line) = self.parse_service_log_line(&service_line)? {
                            trace!("Parsed service log line: {parsed_line:?}");
                            parsed_lines.push(parsed_line);
                        }
                        service_line.clear();
                    }

                    if service_line_read == 0 {
                        break;
                    }
                }
            }

            // Client
            if let Some(reader) = &mut client_reader {
                // read to the eof
                let mut client_raw_lines = Vec::new();
                let mut raw_line = String::new();

                loop {
                    client_line_read = reader.read_line(&mut raw_line)?;
                    if client_line_read > 0 {
                        client_raw_lines.push(raw_line.clone());
                        raw_line.clear();
                    } else {
                        break;
                    }
                }

                client_raw_lines
                    .iter()
                    .for_each(|line| match self.parse_client_log_line(&line) {
                        Ok(Some(parsed_line)) => {
                            trace!("Parsed client log line: {parsed_line:?}");
                            parsed_lines.push(parsed_line);
                        }
                        Ok(None) => {
                            trace!("The following log line was filtered out: {line:?}");
                        }
                        // silently ignore errors
                        Err(_) => (),
                    });
            }

            trace!("Read 0 bytes from both log files, we've reached EOF in both cases.");
            if !parsed_lines.is_empty() {
                parsed_lines.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                trace!("Emitting parsed lines for the frontend");
                self.handle.emit_all(&self.event_topic, &parsed_lines)?;
                trace!("Emitted {} lines to the frontend", parsed_lines.len());
            }
            parsed_lines.clear();
            trace!("Sleeping for {DELAY:?} seconds before reading again");
            sleep(DELAY);

            if self.cancellation_token.is_cancelled() {
                info!("Received cancellation request. Stopping global log watcher");
                break;
            }
        }

        Ok(())
    }

    /// Parse a service log line
    ///
    /// Deserializes the log line into a known struct.
    /// Also performs filtering by log level and optional timestamp.
    fn parse_service_log_line(&self, line: &str) -> Result<Option<LogLine>, LogWatcherError> {
        trace!("Parsing service log line: {line}");
        let log_line = if let Ok(line) = serde_json::from_str::<LogLine>(line) {
            line
        } else {
            warn!("Failed to parse service log line: {line}");
            return Ok(None);
        };
        trace!("Parsed service log line into: {log_line:?}");

        // filter by log level
        if log_line.level > self.log_level {
            trace!(
                "Log level {} is above configured verbosity threshold {}. Skipping line...",
                log_line.level,
                self.log_level
            );
            return Ok(None);
        }

        // filter by optional timestamp
        if let Some(from) = self.from {
            if log_line.timestamp < from {
                trace!(
                    "Timestamp {} is below configured threshold {from}. Skipping line...",
                    log_line.timestamp
                );
                return Ok(None);
            }
        }

        trace!("Successfully parsed service log line.");

        Ok(Some(log_line))
    }

    fn parse_client_log_line(&self, line: &str) -> Result<Option<LogLine>, LogWatcherError> {
        trace!("Parsing client log line: {line}");
        trace!("Preparing regex to parse a line...");
        let regex = Regex::new(r"\[(.*?)\]\[(.*?)\]\[(.*?)\]\[(.*?)\] (.*)")?;
        let captures = regex
            .captures(line)
            .ok_or(LogWatcherError::LogParseError(line.to_string()))?;
        let timestamp_date = captures
            .get(1)
            .ok_or(LogWatcherError::LogParseError(line.to_string()))?
            .as_str();
        let timestamp_time = captures
            .get(2)
            .ok_or(LogWatcherError::LogParseError(line.to_string()))?
            .as_str();

        let timestamp = format!("{} {}", timestamp_date, timestamp_time);
        let timestamp = Utc.from_utc_datetime(
            &NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S").map_err(|e| {
                LogWatcherError::LogParseError(format!(
                    "Failed to parse timestamp {} with error: {}",
                    timestamp, e
                ))
            })?,
        );

        let level = tracing::Level::from_str(
            captures
                .get(3)
                .ok_or(LogWatcherError::LogParseError(line.to_string()))?
                .as_str(),
        )
        .map_err(|e| {
            LogWatcherError::LogParseError(format!("Failed to parse log level with error: {}", e))
        })?;

        let target = captures
            .get(4)
            .ok_or(LogWatcherError::LogParseError(line.to_string()))?
            .as_str()
            .to_string();

        let message = captures
            .get(5)
            .ok_or(LogWatcherError::LogParseError(line.to_string()))?
            .as_str();

        let fields = LogLineFields {
            message: message.to_string(),
        };

        let log_line = LogLine {
            timestamp,
            level,
            target,
            fields,
            span: None,
        };

        if log_line.level > self.log_level {
            trace!(
                "Log level {} is above configured verbosity threshold {}. Skipping line...",
                log_line.level,
                self.log_level
            );
            return Ok(None);
        }

        if let Some(from) = self.from {
            if log_line.timestamp < from {
                trace!("Timestamp is before configured threshold {from}. Skipping line...");
                return Ok(None);
            }
        }

        trace!(
            "Successfully parsed client log line from file {:?}",
            self.log_dirs.client_log_dir
        );
        Ok(Some(log_line))
    }

    /// Find the latest log file in directory
    ///
    /// Log files are rotated daily and have a knows naming format,
    /// with the last 10 characters specifying a date (e.g. `2023-12-15`).
    fn get_latest_log_file(&self) -> Result<Option<PathBuf>, LogWatcherError> {
        let entries = read_dir(&self.log_dirs.service_log_dir)?;

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

/// Starts a global log watcher in a separate thread
pub async fn spawn_global_log_watcher_task(
    handle: &AppHandle,
    log_level: Level,
    from: Option<String>,
) -> Result<String, Error> {
    debug!("Spawning global log watcher.");
    let app_state = handle.state::<AppState>();

    // parse `from` timestamp
    // let from = from.and_then(|from| DateTime::<Utc>::from_str(&from).ok());
    // set from as from 1 hour ago
    let from = Some(Utc::now() - Duration::from_secs(60 * 60));

    let event_topic = format!("log-update-global");
    debug!("Using event topic: {event_topic}");

    // explicitly clone before topic is moved into the closure
    let topic_clone = event_topic.clone();
    let handle_clone = handle.clone();

    // prepare cancellation token
    let token = CancellationToken::new();
    let token_clone = token.clone();

    // spawn task
    let _join_handle: TokioJoinHandle<Result<(), LogWatcherError>> = tokio::spawn(async move {
        let mut log_watcher =
            GlobalLogWatcher::new(handle_clone, token_clone, topic_clone, log_level, from)?;
        log_watcher.run()?;
        Ok(())
    });

    // store `CancellationToken` to manually stop watcher thread
    let mut log_watchers = app_state
        .log_watchers
        .lock()
        .expect("Failed to lock log watchers mutex");
    if let Some(old_token) = log_watchers.insert("GLOBAL".to_string(), token) {
        // cancel previous global log watcher
        debug!("Existing global log watcher found. Cancelling...");
        old_token.cancel();
    }

    info!("Global log watcher spawned");

    Ok(event_topic)
}

/// Stops the log watcher thread
pub fn stop_global_log_watcher_task(handle: &AppHandle) -> Result<(), Error> {
    info!("Stopping global log watcher task");
    let app_state = handle.state::<AppState>();

    // get `CancellationToken` to manually stop watcher thread
    let mut log_watchers = app_state
        .log_watchers
        .lock()
        .expect("Failed to lock log watchers mutex");

    if let Some(token) = log_watchers.remove("GLOBAL") {
        debug!("Using cancellation token for global log watcher");
        token.cancel();
        Ok(())
    } else {
        error!("Global log watcher not found.");
        Err(Error::NotFound)
    }
}
