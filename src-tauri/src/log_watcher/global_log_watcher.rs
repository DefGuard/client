//! Global log watcher that monitors both the service and client logs.
//!
// FIXME: Some of the code here overlaps with the `log_watcher` module and could be refactored to avoid duplication.

use std::{
    fs::{read_dir, File},
    io::{BufRead, BufReader},
    path::PathBuf,
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use tauri::{async_runtime::TokioJoinHandle, AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;
use tracing::Level;

use crate::{
    appstate::AppState,
    error::Error,
    log_watcher::{extract_timestamp, LogLine, LogLineFields, LogSource, LogWatcherError},
    utils::get_service_log_dir,
};

/// Helper struct to handle log directory logic
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
    pub fn new(handle: &AppHandle) -> Result<Self, LogWatcherError> {
        debug!("Getting log directories for service and client to watch.");
        let service_log_dir = get_service_log_dir().to_path_buf();
        let client_log_dir = handle.path().app_log_dir().map_err(|_| {
            LogWatcherError::LogPathError("Path to client logs directory is empty.".to_string())
        })?;
        debug!(
            "Log directories of service and client have been identified by the global log watcher: \
            {service_log_dir:?} and {client_log_dir:?}"
        );

        Ok(Self {
            service_log_dir,
            current_service_log_file: None,
            client_log_dir,
        })
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
        trace!(
            "Opening service log file: {:?}",
            self.current_service_log_file
        );
        match &self.current_service_log_file {
            Some(path) => {
                let file = File::open(path)?;
                trace!(
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
        trace!(
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
        let path = format!("{dir_str}/defguard-client.log");
        trace!("Constructed client log file path: {path}");
        let file = File::open(&path)?;
        trace!("Client log file at {path:?} opened successfully");
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

    /// Start log watching, calls the [`parse_log_dirs`] function.
    pub fn run(&mut self) -> Result<(), LogWatcherError> {
        self.parse_log_dirs()
    }

    /// Parse the log files
    ///
    /// This function will open the log files and read them line by line, parsing each line
    /// into a [`LogLine`] struct and emitting it to the frontend. It can be stopped by cancelling
    /// the token by calling [`stop_global_log_watcher_task()`]
    fn parse_log_dirs(&mut self) -> Result<(), LogWatcherError> {
        trace!("Processing log directories for service and client.");
        self.log_dirs.current_service_log_file = self.log_dirs.get_latest_log_file()?;
        trace!(
            "Latest service log file found: {:?}",
            self.log_dirs.current_service_log_file
        );

        let mut service_reader = if let Ok(file) = self.log_dirs.get_current_service_file() {
            Some(BufReader::new(file))
        } else {
            None
        };
        let mut client_reader = if let Ok(file) = self.log_dirs.get_client_file() {
            Some(BufReader::new(file))
        } else {
            None
        };

        trace!("Checking if log files are available");
        if service_reader.is_none() && client_reader.is_none() {
            warn!(
                "Couldn't read files at {:?} and {:?}, there will be no logs reported in the client.",
                self.log_dirs.current_service_log_file, self.log_dirs.client_log_dir
            );
            // Wait for logs to appear.
            sleep(DELAY);
            return Ok(());
        }
        trace!("Log files are available, starting to read lines.");

        let mut service_line = String::new();
        let mut client_line = String::new();
        let mut parsed_lines = Vec::new();

        // Track the amount of bytes read from the log lines
        let mut service_line_read;
        let mut client_line_read;

        debug!("Global log watcher is starting the loop for reading client and service log files");
        loop {
            if self.cancellation_token.is_cancelled() {
                debug!("Received cancellation request. Stopping global log watcher");
                break;
            }
            // Service
            // If the reader is present, read the log file to the end.
            // Parse every line. If we hit EOF, check if there's a new log file.
            // If there is, switch to it and leave the loop.
            if let Some(reader) = &mut service_reader {
                trace!("Reading service log lines");
                loop {
                    service_line_read = reader.read_line(&mut service_line)?;
                    if service_line_read == 0 {
                        trace!("Read 0 bytes from service log file, probably reached EOF.");
                        let latest_log_file = self.log_dirs.get_latest_log_file()?;
                        if latest_log_file.is_some()
                            && latest_log_file != self.log_dirs.current_service_log_file
                        {
                            debug!(
                                "Found a new service log file: {latest_log_file:?}, switching to it."
                            );
                            self.log_dirs.current_service_log_file = latest_log_file;
                            break;
                        }
                    } else {
                        if let Some(parsed_line) = self.parse_service_log_line(&service_line) {
                            parsed_lines.push(parsed_line);
                        }
                        service_line.clear();
                    }

                    if service_line_read == 0 {
                        break;
                    }
                }
            } else {
                debug!("Service log reader is not present, not reading service log lines.");
            }

            // Client
            // If the reader is present, read the log file to the end.
            // Parse every line.
            // Warning: don't use anything other than a trace log level in this loop for logs that would appear on every iteration (or very often)
            // This could result in the reader constantly producing and consuming logs without any progress.
            if let Some(reader) = &mut client_reader {
                loop {
                    client_line_read = reader.read_line(&mut client_line)?;
                    if client_line_read > 0 {
                        match self.parse_client_log_line(&client_line) {
                            Ok(Some(parsed_line)) => {
                                parsed_lines.push(parsed_line);
                            }
                            Ok(None) => {
                                // Line was filtered out, do nothing
                            }
                            Err(_) => {
                                // Don't log it, as it will cause an endless loop
                            }
                        }
                        client_line.clear();
                    } else {
                        break;
                    }
                }
            } else {
                debug!("Client log file reader is not present, not reading client logs.");
            }

            trace!("Read 0 bytes from both log files, we've reached EOF in both cases.");
            if !parsed_lines.is_empty() {
                parsed_lines.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                trace!("Emitting parsed lines for the frontend");
                self.handle.emit(&self.event_topic, &parsed_lines)?;
                trace!("Emitted {} lines to the frontend", parsed_lines.len());
                parsed_lines.clear();
            }
            trace!("Sleeping for {DELAY:?} seconds before reading again");
            sleep(DELAY);
        }

        Ok(())
    }

    /// Parse a service log line
    ///
    /// Deserializes the log line into a known struct.
    /// Also performs filtering by log level and optional timestamp.
    fn parse_service_log_line(&self, line: &str) -> Option<LogLine> {
        let Ok(mut log_line) = serde_json::from_str::<LogLine>(line) else {
            warn!("Failed to parse service log line: {line}");
            return None;
        };

        // filter by log level
        if log_line.level > self.log_level {
            return None;
        }

        // filter by optional timestamp
        if let Some(from) = self.from {
            if log_line.timestamp < from {
                return None;
            }
        }

        log_line.source = Some(LogSource::Service);

        Some(log_line)
    }

    /// Parse a client log line into a known struct using regex.
    /// If the line doesn't match the regex, it's filtered out.
    fn parse_client_log_line(&self, line: &str) -> Result<Option<LogLine>, LogWatcherError> {
        // Example log:
        // [2024-10-09][09:08:41][DEBUG][defguard_client::commands] Retrieving all locations.
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

        let timestamp = format!("{timestamp_date} {timestamp_time}");
        let timestamp = Utc.from_utc_datetime(
            &NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S%.f").map_err(|err| {
                LogWatcherError::LogParseError(format!(
                    "Failed to parse timestamp {timestamp} with error: {err}"
                ))
            })?,
        );

        let level = tracing::Level::from_str(
            captures
                .get(3)
                .ok_or(LogWatcherError::LogParseError(line.to_string()))?
                .as_str(),
        )
        .map_err(|err| {
            LogWatcherError::LogParseError(format!("Failed to parse log level with error: {err}"))
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
            source: Some(LogSource::Client),
        };

        if log_line.level > self.log_level {
            return Ok(None);
        }

        if let Some(from) = self.from {
            if log_line.timestamp < from {
                return Ok(None);
            }
        }

        Ok(Some(log_line))
    }
}

/// Starts a global log watcher in a separate thread
pub async fn spawn_global_log_watcher_task(
    handle: &AppHandle,
    log_level: Level,
) -> Result<String, Error> {
    debug!("Spawning global log watcher.");
    let app_state = handle.state::<AppState>();

    // Show logs only from the last hour
    let from = Some(Utc::now() - Duration::from_secs(60 * 60));

    let event_topic = "log-update-global".to_string();

    // explicitly clone before topic is moved into the closure
    let topic_clone = event_topic.clone();
    let handle_clone = handle.clone();

    // prepare cancellation token
    let token = CancellationToken::new();
    let token_clone = token.clone();

    // spawn the task
    let _join_handle: TokioJoinHandle<Result<(), LogWatcherError>> = tokio::spawn(async move {
        GlobalLogWatcher::new(handle_clone, token_clone, topic_clone, log_level, from)?.run()?;
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

    debug!("Global log watcher spawned");
    Ok(event_topic)
}

pub fn stop_global_log_watcher_task(handle: &AppHandle) -> Result<(), Error> {
    debug!("Cancelling global log watcher task");
    let app_state = handle.state::<AppState>();

    // get `CancellationToken` to manually stop watcher thread
    let mut log_watchers = app_state
        .log_watchers
        .lock()
        .expect("Failed to lock log watchers mutex");

    if let Some(token) = log_watchers.remove("GLOBAL") {
        debug!("Using cancellation token for global log watcher");
        token.cancel();
        debug!("Global log watcher cancelled");
        Ok(())
    } else {
        // Silently ignore if global log watcher is not found, as there is nothing to cancel
        debug!("Global log watcher not found, nothing to cancel");
        Ok(())
    }
}
