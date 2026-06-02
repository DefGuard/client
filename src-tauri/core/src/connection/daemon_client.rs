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

use defguard_client_proto::defguard::client::v1::desktop_daemon_service_client::DesktopDaemonServiceClient;

#[cfg(unix)]
const DAEMON_SOCKET_PATH: &str = "/var/run/defguard.socket";
#[cfg(windows)]
const PIPE_NAME: &str = r"\\.\pipe\defguard_daemon";

pub static DAEMON_CLIENT: LazyLock<DesktopDaemonServiceClient<Channel>> = LazyLock::new(|| {
    log::debug!("Setting up gRPC client");
    let endpoint = Endpoint::from_static("http://localhost");
    let channel;
    #[cfg(unix)]
    {
        channel = endpoint.connect_with_connector_lazy(service_fn(|_: Uri| async {
            let stream = match UnixStream::connect(DAEMON_SOCKET_PATH).await {
                Ok(stream) => stream,
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                    log::error!(
                        "Permission denied for UNIX domain socket; please refer to \
                            https://docs.defguard.net/support-1/troubleshooting#\
                            unix-socket-permission-errors-when-desktop-client-attempts-to-connect-\
                            to-vpn-on-linux-machines"
                    );
                    return Err(err);
                }
                Err(err) => {
                    log::error!("Problem connecting to UNIX domain socket: {err}");
                    return Err(err);
                }
            };
            log::info!("Created unix gRPC client");
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
                        log::error!("Problem connecting to named pipe: {err}");
                        return Err(err);
                    }
                }
            };
            log::info!("Created windows gRPC client");
            Ok::<_, std::io::Error>(TokioIo::new(client))
        }));
    }
    DesktopDaemonServiceClient::new(channel)
});
