use std::io::stdout;

use tracing::Level;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{InitError, RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    Layer,
};

static SERVICE_LOG_PREFIX: &str = "defguard-service";

pub fn logging_setup(
    log_dir: &str,
    log_level: &str,
    log_max_files: usize,
) -> Result<WorkerGuard, InitError> {
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
