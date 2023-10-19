use crate::{
    service::{
        proto::desktop_daemon_service_client::DesktopDaemonServiceClient, DaemonError,
        DAEMON_BASE_URL,
    },
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

/// Adds routing for allowed_ips
pub fn configure_routing(allowed_ips: Vec<String>, interface_name: &str) -> Result<(), DaemonError> {
    info!(
        "Configuring routing for allowed ips: {:?}",
        allowed_ips
    );
    for allowed_ip in allowed_ips {
        // TODO: Handle windows when wireguard_rs adds support
        // Add a route for the allowed IP using the `ip -4 route add` command
        if let Err(err) = add_route(&allowed_ip, interface_name) {
            error!("Error adding route for {}: {}", allowed_ip, err);
        } else {
            debug!("Added route for {}", allowed_ip);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn add_route(allowed_ip: &str, interface_name: &str) -> Result<(), std::io::Error> {
    std::process::Command::new("ip")
        .args(["-4", "route", "add", allowed_ip, "dev", interface_name])
        .output()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn add_route(allowed_ip: &str, interface_name: &str) -> Result<(), std::io::Error> {
    std::process::Command::new("route")
        .args([
            "-n",
            "add",
            "-net",
            allowed_ip,
            "-interface",
            interface_name,
        ])
        .output()?;
    Ok(())
}

/// Adds DNS config for interface
pub fn configure_dns() -> Result<(), DaemonError> {
    todo!()
}
