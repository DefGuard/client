pub mod active_connections;
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

/// Tear down a WireGuard interface for the given target.
///
/// On Linux/Windows this sends a `RemoveInterface` request to the local daemon.
/// On macOS it stops the Network Extension VPN tunnel.
pub async fn tear_down(target: ConnectionTarget<'_>) -> Result<(), Error> {
    #[cfg(not(target_os = "macos"))]
    {
        use defguard_client_common::get_interface_name;
        use defguard_client_proto::defguard::client::v1::RemoveInterfaceRequest;
        use tonic::Code;

        use crate::connection::daemon_client::DAEMON_CLIENT;

        let (ifname, endpoint) = match target {
            ConnectionTarget::Location(loc) => {
                (get_interface_name(&loc.name), loc.endpoint.clone())
            }
            ConnectionTarget::Tunnel(tunnel) => {
                (get_interface_name(&tunnel.name), tunnel.endpoint.clone())
            }
        };

        let request = RemoveInterfaceRequest {
            interface_name: ifname.clone(),
            endpoint,
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

    #[cfg(target_os = "macos")]
    {
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
}

// Legacy re-exports - used by the desktop root crate (src/utils.rs).
#[cfg(not(target_os = "macos"))]
pub use setup::{disconnect_interface, execute_command, setup_interface, setup_interface_tunnel};

#[cfg(target_os = "macos")]
pub use apple::{
    get_managers_for_tunnels_and_locations, location_tunnel_configuration,
    sync_locations_and_tunnels, tunnel_stats, tunnel_tunnel_configuration, TunnelConfiguration,
};
