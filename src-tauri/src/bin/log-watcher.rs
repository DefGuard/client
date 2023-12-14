use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use defguard_client::utils::get_service_log_dir;
use notify_debouncer_mini::{new_debouncer, notify::*};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info, Level};
use defguard_client::database::info;

fn main() {
  tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();
    info!("Starting log watcher");

    let mut log_watcher = ServiceLogWatcher::new("TestNet".into());

    log_watcher.run();
}

#[derive(Debug, Deserialize)]
struct LogLine {
  timestamp: DateTime<Utc>,
  level: String,
  target: String,
  fields: LogLineFields,
}

#[derive(Debug, Deserialize)]
struct LogLineFields {
  message: String,
  interface_name: Option<String>,
}

struct ServiceLogWatcher {
    interface_name: String,
    log_dir: PathBuf,
    current_log_file: Option<PathBuf>,
    current_position: u64,
}

impl ServiceLogWatcher {
    #[must_use]
    fn new(interface_name: String) -> Self {
        // get log file directory
        let log_dir = get_service_log_dir();
        info!("Log dir: {log_dir:?}");
        Self {
            interface_name,
            log_dir,
            current_log_file: None,
            current_position: 0,
        }
    }

    #[must_use]
    fn run(&mut self) {
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
      let log_line = serde_json::from_str::<LogLine>(&line).expect("LogRocket: error serializing to JSON");
      info!("Line: {log_line:?}");
    }

    fn get_latest_log_file(&self) -> Option<PathBuf> {
        debug!("Getting latest log file");
        let entries = fs::read_dir(&self.log_dir).unwrap();

        let mut latest_log = None;
        let mut latest_time = SystemTime::UNIX_EPOCH;
        for entry in entries {
            if let Ok(entry) = entry {
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
