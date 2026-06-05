pub mod active_connections;
pub mod active_state;
pub mod daemon_client;
pub mod setup;

#[cfg(target_os = "macos")]
pub mod apple;

use crate::{
    database::{
        models::{location::Location, tunnel::Tunnel, Id},
        DbPool,
    },
    error::Error,
};

/// Identifies the type of connection target - a server-side Location or an imported Tunnel.
#[derive(Clone)]
pub enum ConnectionTarget<'a> {
    Location(&'a Location<Id>),
    Tunnel(&'a Tunnel<Id>),
}

/// Bring a WireGuard interface up for the given target.
///
/// On Linux/Windows this sends a `CreateInterface` request to the local daemon
/// (`defguard-service`). On macOS it saves a `TunnelConfiguration` to system preferences
/// and starts the Network Extension tunnel.
///
/// Returns the interface name (e.g. `"wg0"` on Linux, or an empty string on macOS where
/// the system manages the tunnel name).
pub async fn bring_up(
    target: ConnectionTarget<'_>,
    psk: Option<String>,
    mtu: Option<u32>,
    pool: &DbPool,
) -> Result<String, Error> {
    #[cfg(not(target_os = "macos"))]
    {
        match target {
            ConnectionTarget::Location(loc) => {
                setup::setup_interface(loc, &loc.name, psk, mtu, pool).await
            }
            ConnectionTarget::Tunnel(tunnel) => {
                setup::setup_interface_tunnel(tunnel, &tunnel.name, mtu).await
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::time::Duration;

        match target {
            ConnectionTarget::Location(loc) => {
                let tunnel_config = loc.tunnel_configurarion(psk, mtu).await?;
                tunnel_config.save();
                // Work-around MFA propagation delay. FIXME: remove once Core API is corrected.
                tokio::time::sleep(Duration::from_secs(1)).await;
                tunnel_config.start_tunnel();
                Ok(String::new())
            }
            ConnectionTarget::Tunnel(tunnel) => {
                let tunnel_config = tunnel.tunnel_configurarion(mtu)?;
                tunnel_config.save();
                tunnel_config.start_tunnel();
                Ok(String::new())
            }
        }
    }
}

/// Tear down a WireGuard interface by name and endpoint.
///
/// On Linux/Windows this sends a `RemoveInterface` request to the local daemon.
/// Use this when you know the actual interface name (e.g. from `active_state`).
#[cfg(not(target_os = "macos"))]
pub async fn tear_down(ifname: &str, endpoint: &str) -> Result<(), Error> {
    use defguard_client_proto::defguard::client::v1::RemoveInterfaceRequest;
    use tonic::Code;

    use crate::connection::daemon_client::DAEMON_CLIENT;

    let request = RemoveInterfaceRequest {
        interface_name: ifname.to_string(),
        endpoint: endpoint.to_string(),
    };

    if let Err(error) = DAEMON_CLIENT.clone().remove_interface(request).await {
        if error.code() == Code::Unavailable {
            Err(Error::InternalError(
                "Background service is unavailable. Make sure the service is running.".into(),
            ))
        } else {
            Err(Error::InternalError(format!(
                "Failed to remove interface {ifname}: {error}"
            )))
        }
    } else {
        log::info!("Interface {ifname} removed successfully.");
        Ok(())
    }
}

/// Tear down a WireGuard interface for the given target.
///
/// On macOS this stops the Network Extension VPN tunnel.
/// On Linux/Windows, prefer `tear_down(ifname, endpoint)` to avoid deriving
/// the wrong interface name.
#[cfg(target_os = "macos")]
pub async fn tear_down(target: ConnectionTarget<'_>) -> Result<(), Error> {
    match target {
        ConnectionTarget::Location(loc) => {
            if !loc.stop_vpn_tunnel() {
                Err(Error::InternalError(format!(
                    "Failed to stop VPN tunnel for location {}",
                    loc.name
                )))
            } else {
                Ok(())
            }
        }
        ConnectionTarget::Tunnel(tunnel) => {
            if !tunnel.stop_vpn_tunnel() {
                Err(Error::InternalError(format!(
                    "Failed to stop VPN tunnel for tunnel {}",
                    tunnel.name
                )))
            } else {
                Ok(())
            }
        }
    }
}

// Legacy re-exports - now pub(crate) since desktop migrated to bring_up / tear_down.
#[cfg(not(target_os = "macos"))]
pub(crate) use setup::disconnect_interface;

#[cfg(target_os = "macos")]
pub use apple::{
    get_managers_for_tunnels_and_locations, location_tunnel_configuration,
    sync_locations_and_tunnels, tunnel_stats, tunnel_tunnel_configuration, TunnelConfiguration,
};
