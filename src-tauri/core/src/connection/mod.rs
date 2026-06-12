pub mod active_connections;
pub mod active_state;
pub mod daemon_client;
pub mod setup;

#[cfg(target_os = "macos")]
pub mod apple;

#[cfg(not(target_os = "macos"))]
use crate::database::models::connection::ActiveConnection;
#[cfg(target_os = "macos")]
pub use apple::sync_locations_and_tunnels;
#[cfg(not(target_os = "macos"))]
use chrono::Utc;
pub use setup::{disconnect_interface, execute_command};
#[cfg(not(target_os = "macos"))]
pub use setup::{setup_interface, setup_interface_tunnel};

use active_state::ActiveConnectionInfo;

use crate::database::{
    models::{location::Location, tunnel::Tunnel, Id},
    DbPool,
};
use crate::error::Error;

/// Identifies the type of connection target.
pub enum ConnectionTarget<'a> {
    Location(&'a Location<Id>),
    Tunnel(&'a Tunnel<Id>),
}

/// Bring a WireGuard interface up for the given target.
///
/// On macOS this returns [`Error::BackendUnavailable`] — the CLI does not
/// yet support connection management on macOS.  Use the desktop client.
pub async fn bring_up(
    target: ConnectionTarget<'_>,
    psk: Option<String>,
    mtu: Option<u32>,
    pool: &DbPool,
    route_all_traffic: Option<bool>,
) -> Result<String, Error> {
    #[cfg(not(target_os = "macos"))]
    {
        match target {
            ConnectionTarget::Location(loc) => {
                setup::setup_interface(loc, &loc.name, psk, mtu, pool, route_all_traffic).await
            }
            ConnectionTarget::Tunnel(tun) => {
                setup::setup_interface_tunnel(tun, &tun.name, mtu, route_all_traffic).await
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = (target, psk, mtu, pool, route_all_traffic);
        Err(Error::BackendUnavailable(
            "VPN connection management is not yet supported on macOS from the CLI. \
             Use the desktop client."
                .into(),
        ))
    }
}

/// Tear down a WireGuard interface identified by `ActiveConnectionInfo`.
///
/// On macOS this returns [`Error::BackendUnavailable`] — see [`bring_up`].
pub async fn tear_down(conn: &ActiveConnectionInfo) -> Result<(), Error> {
    #[cfg(not(target_os = "macos"))]
    {
        let connection = ActiveConnection {
            location_id: conn.target_id,
            connection_type: conn.connection_type,
            start: Utc::now().naive_utc(),
            interface_name: conn.interface_name.clone(),
        };

        disconnect_interface(&connection).await
    }

    #[cfg(target_os = "macos")]
    {
        let _ = conn;
        Err(Error::BackendUnavailable(
            "VPN connection management is not yet supported on macOS from the CLI. \
             Use the desktop client."
                .into(),
        ))
    }
}
