use crate::service::{
    proto::desktop_daemon_service_client::DesktopDaemonServiceClient, DaemonError, DAEMON_BASE_URL,
};
use tonic::transport::channel::{Channel, Endpoint};
use tracing::debug;

pub fn setup_client() -> Result<DesktopDaemonServiceClient<Channel>, DaemonError> {
    debug!("Setting up gRPC client");
    let endpoint = Endpoint::from_shared(DAEMON_BASE_URL)?;
    let channel = endpoint.connect_lazy();
    let client = DesktopDaemonServiceClient::new(channel);
    Ok(client)
}
