pub mod active_connections;
pub mod active_state;
pub mod daemon_client;
pub mod setup;

#[cfg(target_os = "macos")]
pub mod apple;

#[cfg(target_os = "macos")]
use std::time::Duration;

use active_state::ActiveConnectionInfo;
#[cfg(target_os = "macos")]
pub use apple::sync_locations_and_tunnels;
use chrono::Utc;
pub use setup::{disconnect_interface, execute_command};
#[cfg(not(target_os = "macos"))]
pub use setup::{setup_interface, setup_interface_tunnel};
#[cfg(target_os = "macos")]
use tokio::time::sleep;

use crate::{
    connection::active_connections::active_connection_ids,
    database::{
        models::{connection::ActiveConnection, location::Location, tunnel::Tunnel, Id},
        DbPool,
    },
    error::Error,
    ConnectionType,
};

#[cfg(target_os = "macos")]
const TUNNEL_START_DELAY: Duration = Duration::from_secs(1);

/// Identifies the type of connection target.
pub enum ConnectionTarget {
    Location(Location<Id>),
    Tunnel(Tunnel<Id>),
}

impl ConnectionTarget {
    pub async fn ensure_single_all_traffic_connection(
        &self,
        pool: &DbPool,
        route_all_traffic: Option<bool>,
    ) -> Result<(), Error> {
        let (id, connection_type, name, holds_default_route) = match self {
            Self::Location(location) => (
                location.id,
                ConnectionType::Location,
                &location.name,
                location
                    .holds_default_route(pool, route_all_traffic)
                    .await?,
            ),
            Self::Tunnel(tunnel) => (
                tunnel.id,
                ConnectionType::Tunnel,
                &tunnel.name,
                tunnel.holds_default_route(route_all_traffic),
            ),
        };

        if !holds_default_route {
            return Ok(());
        }

        if let Some((active_type, active_name)) =
            find_default_route_owner(pool, (id, connection_type)).await?
        {
            error!(
                "Refusing to connect {connection_type} \"{name}\" (ID {id}): it routes all \
                traffic, but {active_type} \"{active_name}\" already holds the default route."
            );
            return Err(Error::AllTrafficConflict(format!(
                "Can't connect to {connection_type} \"{name}\": {active_type} \"{active_name}\" \
                is already routing all traffic. Only one connection can route all traffic at a \
                time, so disconnect it first or turn off \"route all traffic\" for one of them."
            )));
        }

        Ok(())
    }
}

async fn find_default_route_owner(
    pool: &DbPool,
    exclude: (Id, ConnectionType),
) -> Result<Option<(ConnectionType, String)>, Error> {
    for (id, connection_type) in active_connection_ids().await {
        if (id, connection_type) == exclude {
            continue;
        }
        let owner = match connection_type {
            ConnectionType::Location => match Location::find_by_id(pool, id).await? {
                Some(location) => location
                    .holds_default_route(pool, None)
                    .await?
                    .then_some(location.name),
                None => None,
            },
            ConnectionType::Tunnel => match Tunnel::find_by_id(pool, id).await? {
                Some(tunnel) => tunnel.holds_default_route(None).then_some(tunnel.name),
                None => None,
            },
        };
        if let Some(name) = owner {
            return Ok(Some((connection_type, name)));
        }
    }

    Ok(None)
}

/// Bring a WireGuard interface up for the given target.
pub async fn bring_up(
    target: ConnectionTarget,
    psk: Option<String>,
    mtu: Option<u32>,
    pool: &DbPool,
    route_all_traffic: Option<bool>,
) -> Result<String, Error> {
    target
        .ensure_single_all_traffic_connection(pool, route_all_traffic)
        .await?;

    #[cfg(not(target_os = "macos"))]
    {
        match target {
            ConnectionTarget::Location(loc) => {
                let name = loc.name.clone();
                setup::setup_interface(loc, &name, psk, mtu, pool, route_all_traffic).await
            }
            ConnectionTarget::Tunnel(tun) => {
                let name = tun.name.clone();
                setup::setup_interface_tunnel(tun, &name, mtu, route_all_traffic).await
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let tunnel_config = match target {
            ConnectionTarget::Location(loc) => loc.tunnel_configuration(psk, mtu).await,
            ConnectionTarget::Tunnel(tun) => tun.tunnel_configuration(mtu),
        }?;

        tunnel_config.save();
        sleep(TUNNEL_START_DELAY).await;
        tunnel_config.start_tunnel();

        // On macOS the interface name is managed by the system.
        Ok(String::new())
    }
}

/// Tear down a WireGuard interface identified by `ActiveConnectionInfo`.
//
// FIXME: This constructs an `ActiveConnection` with `start: Utc::now()`,
// which records a zero-duration connection when saved. This impacts the
// connection history overview (all entries appear instant). Connection
// tracking should be refactored to carry the real start time from the
// active-state record through to the history persistence path.
pub async fn tear_down(conn: &ActiveConnectionInfo) -> Result<(), Error> {
    let connection = ActiveConnection {
        location_id: conn.target_id,
        connection_type: conn.connection_type,
        start: Utc::now().naive_utc(),
        interface_name: conn.interface_name.clone(),
    };

    disconnect_interface(&connection).await
}
