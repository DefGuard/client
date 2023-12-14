//! Log watcher for observing and parsing `defguard-service` log files
//!
//! This is meant to allow passing relevant logs from `defguard-service` daemon to the client GUI.

use crate::utils::get_service_log_dir;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::{
    fs::{read_dir, File},
    io::{BufRead, BufReader},
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tauri::AppHandle;
use thiserror::Error;
use tracing::Level;

#[derive(Error, Debug)]
pub enum LogWatcherError {}

/// Represents a single line in log file
#[serde_as]
#[derive(Debug, Deserialize)]
struct LogLine {
    timestamp: DateTime<Utc>,
    #[serde_as(as = "DisplayFromStr")]
    level: Level,
    target: String,
    fields: LogLineFields,
}

#[derive(Debug, Deserialize)]
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
    event_topic: String,
}

impl ServiceLogWatcher {
    #[must_use]
    pub fn new(
        handle: AppHandle,
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
            event_topic,
        }
    }

    pub fn run(&mut self) -> Result<(), LogWatcherError> {
        // setup debouncer
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = new_debouncer(Duration::from_secs(2), tx).unwrap();

        debouncer
            .watcher()
            .watch(&self.log_dir, RecursiveMode::Recursive)
            .unwrap();

        // parse log dir initially before watching for changes
        self.parse_log_dir();

        for result in rx {
            match result {
                Ok(_events) => {
                    self.parse_log_dir();
                }
                Err(error) => println!("Error {error:?}"),
            }
        }
        Ok(())
    }

    fn parse_log_dir(&mut self) {
        // get latest log file
        let latest_log_file = self.get_latest_log_file();
        info!("found latest log file: {latest_log_file:?}");

        // check if latest file changed
        if latest_log_file.is_some() && latest_log_file != self.current_log_file {
            self.current_log_file = latest_log_file;
            // reset read position
            self.current_position = 0;
        }

        // read and parse file from last position
        if let Some(log_file) = &self.current_log_file {
            let file = File::open(log_file).unwrap();
            let size = file.metadata().unwrap().len();
            let mut reader = BufReader::new(file);
            reader.seek_relative(self.current_position as i64).unwrap();
            for line in reader.lines() {
                let line = line.unwrap();
                self.parse_log_line(line);
            }
            // update read position to end of file
            self.current_position = size;
        }

        // if not changed read only the changed part
        todo!();
    }

    fn parse_log_line(&self, line: String) {
        let log_line =
            serde_json::from_str::<LogLine>(&line).expect("LogRocket: error serializing to JSON");
        info!("Line: {log_line:?}");
    }

    fn get_latest_log_file(&self) -> Option<PathBuf> {
        debug!("Getting latest log file");
        let entries = read_dir(&self.log_dir).unwrap();

        let mut latest_log = None;
        let mut latest_time = SystemTime::UNIX_EPOCH;
        for entry in entries.flatten() {
            // skip directories
            if entry.metadata().unwrap().is_file() {
                let filename = entry.file_name().to_string_lossy().into_owned();
                if let Some(timestamp) = extract_timestamp(&filename) {
                    if timestamp > latest_time {
                        latest_time = timestamp;
                        latest_log = Some(entry.path());
                    }
                }
            }
        }
        latest_log
    }
}

fn extract_timestamp(filename: &str) -> Option<SystemTime> {
    debug!("Extracting timestamp from log file name: {filename}");
    // we know that the date is always in the last 10 characters
    let split_pos = filename.char_indices().nth_back(9).unwrap().0;
    let timestamp = &filename[split_pos..];
    // parse and convert to `SystemTime`
    let timestamp = NaiveDate::parse_from_str(timestamp, "%Y-%m-%d")
        .unwrap()
        .and_time(NaiveTime::default())
        .timestamp();
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp as u64))
}
