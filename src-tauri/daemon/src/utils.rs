use std::{
    fs,
    io::{self, stdout},
    path::Path,
};

use tracing::Level;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{InitError, RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    Layer,
};

const SERVICE_LOG_PREFIX: &str = "defguard-service";
const OLD_SERVICE_LOG_PREFIX: &str = "defguard-service.log.";

#[derive(Debug, thiserror::Error)]
pub enum LoggingSetupError {
    #[error("failed to migrate service log files: {0}")]
    Migration(#[source] io::Error),
    #[error("failed to initialize service log appender: {0}")]
    Appender(#[from] InitError),
}

pub fn logging_setup(
    log_dir: &str,
    log_level: &str,
    log_max_files: usize,
) -> Result<WorkerGuard, LoggingSetupError> {
    migrate_service_log_files(Path::new(log_dir)).map_err(LoggingSetupError::Migration)?;

    // prepare log file appender
    let mut appender_builder = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(SERVICE_LOG_PREFIX)
        .filename_suffix("log");
    if log_max_files > 0 {
        appender_builder = appender_builder.max_log_files(log_max_files);
    }
    let file_appender = appender_builder.build(log_dir)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // prepare log level filter for stdout
    let stdout_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{log_level},hyper=info,h2=info").into());

    // prepare log level filter for JSON file
    let json_filter = EnvFilter::new("DEBUG,hyper=info,h2=info");

    // prepare tracing layers
    let stdout_layer = fmt::layer()
        .pretty()
        .with_writer(stdout.with_max_level(Level::DEBUG))
        .with_filter(stdout_filter);
    let json_file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking.with_max_level(Level::DEBUG))
        .with_filter(json_filter);

    // initialize tracing subscriber
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(json_file_layer)
        .init();

    Ok(guard)
}

fn migrate_service_log_files(log_dir: &Path) -> io::Result<()> {
    let entries = match fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let filename = entry.file_name();
        let Some(filename) = filename.to_str() else {
            continue;
        };
        let Some(date) = filename.strip_prefix(OLD_SERVICE_LOG_PREFIX) else {
            continue;
        };
        if !is_log_date(date) {
            continue;
        }

        let new_path = log_dir.join(format!("{SERVICE_LOG_PREFIX}.{date}.log"));
        if new_path.exists() {
            eprintln!(
                "Skipping service log migration because destination already exists: {}",
                new_path.display()
            );
            continue;
        }

        fs::rename(entry.path(), new_path)?;
    }

    Ok(())
}

fn is_log_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}
