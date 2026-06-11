pub mod active_connections;
pub mod active_state;
pub mod daemon_client;
pub mod setup;

#[cfg(target_os = "macos")]
pub mod apple;

#[cfg(target_os = "macos")]
pub use apple::sync_locations_and_tunnels;
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
pub async fn bring_up(
    target: ConnectionTarget<'_>,
    psk: Option<String>,
    mtu: Option<u32>,
    pool: &DbPool,
) -> Result<String, Error> {
    match target {
        ConnectionTarget::Location(loc) => {
            setup::setup_interface(loc, &loc.name, psk, mtu, pool).await
        }
        ConnectionTarget::Tunnel(tun) => setup::setup_interface_tunnel(tun, &tun.name, mtu).await,
    }
}

/// Tear down a WireGuard interface identified by `ActiveConnectionInfo`.
pub async fn tear_down(conn: &ActiveConnectionInfo, pool: &DbPool) -> Result<(), Error> {
    use crate::database::models::connection::ActiveConnection;

    let connection = ActiveConnection {
        location_id: conn.target_id,
        connection_type: conn.connection_type,
        start: chrono::Utc::now().naive_utc(),
        interface_name: conn.interface_name.clone(),
    };

    let _ = pool;
    disconnect_interface(&connection).await
}
