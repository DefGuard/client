//! defguard interface management daemon
//!
//! This binary is meant to run as a daemon with root privileges
//! and communicate with the desktop client over HTTP.

use clap::Parser;
use defguard_client::{
    service::{config::Config, run_server},
    utils::get_service_log_dir,
};
use std::io::stdout;
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    Layer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // parse config
    let config = Config::parse();

    // prepare log file appender
    let log_dir = get_service_log_dir();
    let file_appender = tracing_appender::rolling::daily(log_dir, "defguard-service.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // prepare log level filter for stdout
    let stdout_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{},hyper=info", config.log_level).into());

    // prepare log level filter for json file
    let json_filter = EnvFilter::new(format!("{},hyper=info", tracing::Level::DEBUG));

    // prepare tracing layers
    let stdout_layer = fmt::layer()
        .pretty()
        .with_writer(stdout.with_max_level(tracing::Level::DEBUG))
        .with_filter(stdout_filter);
    let json_file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking.with_max_level(tracing::Level::DEBUG))
        .with_filter(json_filter);

    // initialize tracing subscriber
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(json_file_layer)
        .init();

    // run gRPC server
    run_server(config).await?;

    Ok(())
}
