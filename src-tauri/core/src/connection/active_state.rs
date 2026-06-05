//! Reconstruct currently-active WireGuard connections by querying the platform backend.
//!
//! The daemon (Linux/Windows) and Network Extension managers (macOS) are the shared
//! source of truth for interface state.  `active_state` calls the daemon's `ListInterfaces`
//! RPC for an unfiltered snapshot of all managed interfaces, then matches each peer's
//! public key to a `Location` or `Tunnel` in the database.

use crate::{
    database::{
        models::{location::Location, tunnel::Tunnel, Id},
        DbPool,
    },
    error::Error,
    ConnectionType,
};

/// Describes a currently-active WireGuard connection.
#[derive(Clone, Debug)]
pub struct ActiveConnectionInfo {
    /// Whether this is a server-defined `Location` or an imported `Tunnel`.
    pub connection_type: ConnectionType,
    /// The database id of the target (`Location.id` or `Tunnel.id`).
    pub target_id: Id,
    /// Human-readable name of the location or tunnel.
    pub name: String,
    /// Platform interface name, e.g. `"wg0"` on Linux.
    pub interface_name: String,
    /// Live statistics from the most recent backend probe, if any.
    pub stats: Option<InterfaceStats>,
}

/// Snapshot of per-interface statistics retrieved from the backend.
#[derive(Clone, Debug)]
pub struct InterfaceStats {
    pub listen_port: u32,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub last_handshake: Option<u64>,
}

/// Query the platform backend for all currently-up WireGuard interfaces and match each
/// peer back to a known `Location` or `Tunnel`.
///
/// On Linux/Windows this calls the daemon's `ListInterfaces` RPC, which returns an
/// **unfiltered** snapshot of all managed interfaces (unlike `ReadInterfaceData`, which
/// drops peers that haven't completed a handshake or whose stats haven't changed).
///
/// On macOS the Network Extension path is stubbed (pending the NE spike).
pub async fn active_state(pool: &DbPool) -> Result<Vec<ActiveConnectionInfo>, Error> {
    #[cfg(not(target_os = "macos"))]
    {
        active_state_daemon(pool).await
    }

    #[cfg(target_os = "macos")]
    {
        // Stub: NE-based enumeration pending the macOS spike.
        let _ = pool;
        Ok(Vec::new())
    }
}

#[cfg(not(target_os = "macos"))]
async fn active_state_daemon(pool: &DbPool) -> Result<Vec<ActiveConnectionInfo>, Error> {
    use tonic::Code;

    use crate::connection::daemon_client::DAEMON_CLIENT;

    let request = tonic::Request::new(());
    let response = DAEMON_CLIENT
        .clone()
        .list_interfaces(request)
        .await
        .map_err(|err| {
            if err.code() == Code::Unavailable {
                log::error!("Daemon unavailable: {err}");
                Error::InternalError(
                    "Background service is unavailable. Make sure the service is running.".into(),
                )
            } else {
                log::error!("Failed to call ListInterfaces: {err}");
                Error::InternalError(format!("ListInterfaces failed: {err}"))
            }
        })?;
    let inner = response.into_inner();

    log::info!(
        "ListInterfaces returned {} managed interface(s)",
        inner.interfaces.len()
    );

    let mut results = Vec::new();

    for managed in &inner.interfaces {
        let Some(iface_data) = &managed.data else {
            continue;
        };

        for peer in &iface_data.peers {
            let public_key = &peer.public_key;

            // Try matching the peer to a Location first.
            match Location::find_by_public_key(pool, public_key).await {
                Ok(location) => {
                    log::info!(
                        "Matched peer {public_key} to location {} (id={})",
                        location.name,
                        location.id
                    );
                    results.push(ActiveConnectionInfo {
                        connection_type: ConnectionType::Location,
                        target_id: location.id,
                        name: location.name.clone(),
                        interface_name: managed.interface_name.clone(),
                        stats: peer_stats(iface_data, peer),
                    });
                    continue;
                }
                Err(sqlx::Error::RowNotFound) => {
                    // Not a Location, try Tunnel below.
                }
                Err(err) => {
                    log::warn!("DB error looking up public key {public_key}: {err}");
                    continue;
                }
            }

            // Then try matching to a Tunnel.
            match Tunnel::find_by_server_public_key(pool, public_key).await {
                Ok(tunnel) => {
                    log::info!(
                        "Matched peer {public_key} to tunnel {} (id={})",
                        tunnel.name,
                        tunnel.id
                    );
                    results.push(ActiveConnectionInfo {
                        connection_type: ConnectionType::Tunnel,
                        target_id: tunnel.id,
                        name: tunnel.name.clone(),
                        interface_name: managed.interface_name.clone(),
                        stats: peer_stats(iface_data, peer),
                    });
                    continue;
                }
                Err(sqlx::Error::RowNotFound) => {
                    // Not a Tunnel either.
                }
                Err(err) => {
                    log::warn!("DB error looking up server public key {public_key}: {err}");
                    continue;
                }
            }

            log::debug!("Peer {public_key} does not match any Location or Tunnel, skipping");
        }
    }

    log::info!("active_state: found {} active connection(s)", results.len());
    Ok(results)
}

/// Extract per-peer stats from an `InterfaceData` response.
#[cfg(not(target_os = "macos"))]
fn peer_stats(
    iface: &defguard_client_proto::defguard::client::v1::InterfaceData,
    peer: &defguard_client_proto::defguard::client::v1::Peer,
) -> Option<InterfaceStats> {
    Some(InterfaceStats {
        listen_port: iface.listen_port,
        tx_bytes: peer.tx_bytes,
        rx_bytes: peer.rx_bytes,
        last_handshake: peer.last_handshake,
    })
}
