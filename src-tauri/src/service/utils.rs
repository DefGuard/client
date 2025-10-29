use std::{io::stdout, sync::LazyLock};

#[cfg(windows)]
use crate::service::{
    named_pipe::PIPE_NAME,
    proto::desktop_daemon_service_client::DesktopDaemonServiceClient, DAEMON_BASE_URL,
};
use hyper_util::rt::TokioIo;
#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;
#[cfg(unix)]
use tokio::net::UnixStream;
use tonic::transport::channel::{Channel, Endpoint};
#[cfg(unix)]
use tonic::transport::Uri;
use tower::service_fn;
use tracing::{debug, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    Layer,
};
#[cfg(windows)]
use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

pub(crate) static DAEMON_CLIENT: LazyLock<DesktopDaemonServiceClient<Channel>> =
    LazyLock::new(|| {
        debug!("Setting up gRPC client");
        let endpoint = Endpoint::from_static(DAEMON_BASE_URL); // Should not panic.
        let channel;
        #[cfg(unix)]
        {
            channel = endpoint.connect_with_connector_lazy(service_fn(|_: Uri| async {
                // Connect to a Unix domain socket.
                let stream = match UnixStream::connect(crate::service::DAEMON_SOCKET_PATH).await {
                    Ok(stream) => stream,
                    Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                        error!(
                            "Permission denied for UNIX domain socket; please refer to \
                            https://docs.defguard.net/support-1/troubleshooting#\
                            unix-socket-permission-errors-when-desktop-client-attempts-to-connect-\
                            to-vpn-on-linux-machines"
                        );
                        return Err(err);
                    }
                    Err(err) => {
                        error!("Problem connecting to UNIX domain socket: {err}");
                        return Err(err);
                    }
                };
                Ok::<_, std::io::Error>(TokioIo::new(stream))
            }));
        };
        #[cfg(windows)]
        {
            channel = endpoint.connect_with_connector_lazy(service_fn(|_| async {
                let client = loop {
                    match ClientOptions::new().open(PIPE_NAME) {
                        Ok(client) => break client,
                        Err(err) if err.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => (),
                        Err(err) => {
                            error!("Problem connecting to named pipe: {err}");
                            return Err(err);
                        }
                    }
                };
                Ok::<_, std::io::Error>(TokioIo::new(client))
            }));
        }
        DesktopDaemonServiceClient::new(channel)
    });

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
