use std::sync::LazyLock;

use hyper_util::rt::TokioIo;
#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;
#[cfg(unix)]
use tokio::net::UnixStream;
use tonic::transport::channel::{Channel, Endpoint};
#[cfg(unix)]
use tonic::transport::Uri;
use tower::service_fn;
#[cfg(windows)]
use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

#[cfg(unix)]
use super::daemon::DAEMON_SOCKET_PATH;
#[cfg(windows)]
use super::named_pipe::PIPE_NAME;
use super::proto::desktop_daemon_service_client::DesktopDaemonServiceClient;

pub(crate) static DAEMON_CLIENT: LazyLock<DesktopDaemonServiceClient<Channel>> =
    LazyLock::new(|| {
        debug!("Setting up gRPC client");
        // URL is ignored since we provide our own connectors for unix socket and windows named pipes.
        let endpoint = Endpoint::from_static("http://localhost");
        let channel;
        #[cfg(unix)]
        {
            channel = endpoint.connect_with_connector_lazy(service_fn(|_: Uri| async {
                // Connect to a Unix domain socket.
                let stream = match UnixStream::connect(DAEMON_SOCKET_PATH).await {
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
                info!("Created unix gRPC client");
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
                info!("Created windows gRPC client");
                Ok::<_, std::io::Error>(TokioIo::new(client))
            }));
        }
        DesktopDaemonServiceClient::new(channel)
    });
