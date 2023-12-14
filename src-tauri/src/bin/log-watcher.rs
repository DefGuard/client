use chrono::{NaiveDate, NaiveTime};
use defguard_client::utils::get_service_log_dir;
use notify_debouncer_mini::{new_debouncer, notify::*};
use sqlx::types::uuid::timestamp;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    println!("Starting log watcher");

    // get log file directory
    let log_dir = get_service_log_dir();

    println!("Log dir: {log_dir:?}");

    // setup debouncer
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_secs(2), tx).unwrap();

    debouncer
        .watcher()
        .watch(&log_dir, RecursiveMode::Recursive)
        .unwrap();

    // watch for changes in service log directory
    for result in rx {
        match result {
            Ok(_events) => {
                // get latest log file
                let latest_log = get_latest_log_file(log_dir);
                println!("found latest log file: {latest_log:?}");

                // check if latest file changes
                todo!();

                // if changed read and parse whole content of new file
                todo!();

                // if not changed read only the changed part
                todo!();
            }
            Err(error) => println!("Error {error:?}"),
        }
    }
}

fn get_latest_log_file(path: PathBuf) -> Option<PathBuf> {
    let entries = fs::read_dir(path).unwrap();

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

fn extract_timestamp(filename: &str) -> Option<SystemTime> {
    let split_pos = filename.char_indices().nth_back(10).unwrap().0;
    let timestamp = &filename[split_pos..];
    let timestamp = NaiveDate::parse_from_str(timestamp, "%Y-%m-%d")
        .unwrap()
        .and_time(NaiveTime::default())
        .timestamp();
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp as u64))
}
