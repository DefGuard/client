use std::io::stdout;

#[cfg(windows)]
use crate::service::DAEMON_BASE_URL;
#[cfg(unix)]
use crate::service::DAEMON_SOCKET_PATH;
#[cfg(unix)]
use hyper_util::rt::TokioIo;
#[cfg(unix)]
use tokio::net::UnixStream;
use tonic::transport::channel::{Channel, Endpoint};
#[cfg(unix)]
use tonic::transport::Uri;
#[cfg(unix)]
use tower::service_fn;
use tracing::{debug, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    Layer,
};

use crate::service::{
    proto::desktop_daemon_service_client::DesktopDaemonServiceClient, DaemonError,
};

#[cfg(unix)]
pub fn setup_client() -> Result<DesktopDaemonServiceClient<Channel>, DaemonError> {
    debug!("Setting up gRPC client");
    let endpoint = Endpoint::try_from("http://[::]:50051")?;
    let channel = endpoint.connect_with_connector_lazy(service_fn(|_: Uri| async {
        // Connect to a Uds socket
        Ok::<_, std::io::Error>(TokioIo::new(UnixStream::connect(DAEMON_SOCKET_PATH).await?))
    }));
    let client = DesktopDaemonServiceClient::new(channel);
    Ok(client)
}

#[cfg(windows)]
pub fn setup_client() -> Result<DesktopDaemonServiceClient<Channel>, DaemonError> {
    debug!("Setting up gRPC client");
    let endpoint = Endpoint::from_shared(DAEMON_BASE_URL)?;
    let channel = endpoint.connect_lazy();
    let client = DesktopDaemonServiceClient::new(channel);
    Ok(client)
}

pub fn logging_setup(log_dir: &str, log_level: &str) -> WorkerGuard {
    // prepare log file appender
    let file_appender = tracing_appender::rolling::daily(log_dir, "defguard-service.log");
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

    guard
}
