use std::sync::LazyLock;

use defguard_client_proto::defguard::client::v1::desktop_daemon_service_client::DesktopDaemonServiceClient;
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
const DAEMON_SOCKET_PATH: &str = "/var/run/defguard.socket";

/// Returns the daemon socket path.  In test/debug builds the
/// `DEFGUARD_DAEMON_SOCKET` environment variable can override the default
/// (useful for integration tests).  In release builds the override is
/// disabled to prevent an undocumented channel-redirection surface.
#[cfg(unix)]
#[must_use]
pub fn daemon_socket_path() -> String {
    #[cfg(any(test, debug_assertions))]
    if let Ok(path) = std::env::var("DEFGUARD_DAEMON_SOCKET") {
        return path;
    }
    DAEMON_SOCKET_PATH.to_string()
}
#[cfg(windows)]
const PIPE_NAME: &str = r"\\.\pipe\defguard_daemon";

pub static DAEMON_CLIENT: LazyLock<DesktopDaemonServiceClient<Channel>> = LazyLock::new(|| {
    debug!("Setting up gRPC client");
    let endpoint = Endpoint::from_static("http://localhost");
    let channel;
    #[cfg(unix)]
    {
        channel = endpoint.connect_with_connector_lazy(service_fn(|_: Uri| async {
            let stream = match UnixStream::connect(daemon_socket_path()).await {
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
