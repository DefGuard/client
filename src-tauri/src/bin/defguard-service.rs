//! defguard interface management daemon
//!
//! This binary is meant to run as a daemon with root privileges
//! and communicate with the desktop client over HTTP.

use clap::Parser;
use defguard_client::service::{config::Config, run_server};
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    Layer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // parse config
    let config = Config::parse();

    // prepare log file appender
    let file_appender = tracing_appender::rolling::daily("/var/log", "defguard-service.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // prepare log level filter for stdout
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{},hyper=info", config.log_level).into());

    // prepare tracing layers
    let stdout_layer = fmt::layer()
        .pretty()
        .with_writer(std::io::stdout.with_max_level(tracing::Level::DEBUG))
        .with_filter(filter);
    let json_file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking.with_max_level(tracing::Level::TRACE));

    // initialize tracing subscriber
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(json_file_layer)
        .init();

    // run gRPC server
    run_server(config).await?;

    Ok(())
}
