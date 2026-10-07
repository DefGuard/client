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
use defguard_wireguard_rs::net::IpAddrMask;
use serde::Serialize;
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
    networks_conflict, ConnectionType,
};

#[cfg(target_os = "macos")]
const TUNNEL_START_DELAY: Duration = Duration::from_secs(1);

/// Identifies the type of connection target.
pub enum ConnectionTarget {
    Location(Location<Id>),
    Tunnel(Tunnel<Id>),
}

#[derive(Clone, Debug, Serialize)]
pub struct ConflictingConnection {
    pub id: Id,
    pub connection_type: ConnectionType,
    pub name: String,
}

impl ConnectionTarget {
    fn as_conflict(&self) -> ConflictingConnection {
        let (id, connection_type, name) = match self {
            Self::Location(location) => (location.id, ConnectionType::Location, &location.name),
            Self::Tunnel(tunnel) => (tunnel.id, ConnectionType::Tunnel, &tunnel.name),
        };
        ConflictingConnection {
            id,
            connection_type,
            name: name.clone(),
        }
    }

    async fn routed_networks(
        &self,
        pool: &DbPool,
        route_all_traffic: Option<bool>,
    ) -> Result<Vec<IpAddrMask>, Error> {
        match self {
            Self::Location(location) => location.routed_networks(pool, route_all_traffic).await,
            Self::Tunnel(tunnel) => Ok(tunnel.routed_networks(route_all_traffic)),
        }
    }

    pub async fn ensure_no_route_conflict(
        &self,
        pool: &DbPool,
        route_all_traffic: Option<bool>,
    ) -> Result<(), Error> {
        let ConflictingConnection {
            id,
            connection_type,
            name,
        } = self.as_conflict();
        let networks = self.routed_networks(pool, route_all_traffic).await?;

        let mut conflicts = Vec::new();
        for (active_id, active_type) in active_connection_ids().await {
            if (active_id, active_type) == (id, connection_type) {
                continue;
            }
            let active = match active_type {
                ConnectionType::Location => Location::find_by_id(pool, active_id)
                    .await?
                    .map(Self::Location),
                ConnectionType::Tunnel => {
                    Tunnel::find_by_id(pool, active_id).await?.map(Self::Tunnel)
                }
            };
            let Some(active) = active else { continue };
            let active_networks = active.routed_networks(pool, None).await?;
            if networks.iter().any(|network| {
                active_networks
                    .iter()
                    .any(|active| networks_conflict(network, active))
            }) {
                conflicts.push(active.as_conflict());
            }
        }

        if conflicts.is_empty() {
            return Ok(());
        }
        let error = Error::RouteConflict { name, conflicts };
        error!("Refusing to connect {connection_type} {id}: {error}");
        Err(error)
    }
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
        .ensure_no_route_conflict(pool, route_all_traffic)
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
