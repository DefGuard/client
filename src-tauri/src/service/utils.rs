use std::io::stdout;

use tonic::transport::channel::{Channel, Endpoint};
use tracing::{debug, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    Layer,
};

use super::config::Config;
use crate::service::{
    proto::desktop_daemon_service_client::DesktopDaemonServiceClient, DaemonError, DAEMON_BASE_URL,
};

pub fn setup_client() -> Result<DesktopDaemonServiceClient<Channel>, DaemonError> {
    debug!("Setting up gRPC client");
    let endpoint = Endpoint::from_shared(DAEMON_BASE_URL)?;
    let channel = endpoint.connect_lazy();
    let client = DesktopDaemonServiceClient::new(channel);
    Ok(client)
}

pub fn logging_setup(config: &Config) -> WorkerGuard {
    // prepare log file appender
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, "defguard-service.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // prepare log level filter for stdout
    let stdout_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{},hyper=info", config.log_level).into());

    // prepare log level filter for json file
    let json_filter = EnvFilter::new(format!("{},hyper=info", Level::DEBUG));

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

    guard
}
