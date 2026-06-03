//! Reconstruct currently-active WireGuard connections by querying the platform backend.
//!
//! The daemon (Linux/Windows) and Network Extension managers (macOS) are the shared
//! source of truth for interface state.  `active_state` enumerates candidate interfaces,
//! queries live stats from the backend, and matches each peer's public key to a
//! `Location` or `Tunnel` in the database.

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
/// On Linux this enumerates interfaces whose name starts with `"wg"` via
/// `nix::if_nameindex`, probes the daemon for each, and performs pubkey matching.
/// On Windows and macOS the `#[cfg]` branches are stubbed; they follow the same
/// pattern using platform-specific enumeration.
pub async fn active_state(pool: &DbPool) -> Result<Vec<ActiveConnectionInfo>, Error> {
    #[cfg(target_os = "linux")]
    {
        active_state_linux(pool).await
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Stub: return empty for non-Linux platforms until per-platform
        // enumeration is implemented (Phase 2 follow-up).
        let _ = pool;
        Ok(Vec::new())
    }
}

#[cfg(target_os = "linux")]
use sqlx;
async fn active_state_linux(pool: &DbPool) -> Result<Vec<ActiveConnectionInfo>, Error> {
    use defguard_client_proto::defguard::client::v1::ReadInterfaceDataRequest;

    use crate::connection::daemon_client::DAEMON_CLIENT;

    let ifaces = nix::ifaddrs::getifaddrs().map_err(|err| {
        log::error!("Failed to enumerate network interfaces: {err}");
        Error::InternalError(format!("Failed to enumerate network interfaces: {err}"))
    })?;

    let mut results = Vec::new();

    for iface in ifaces {
        let name = iface.interface_name;
        if !name.starts_with("wg") {
            continue;
        }

        log::debug!("Probing daemon for interface {name}");

        let request = ReadInterfaceDataRequest {
            interface_name: name.to_string(),
        };

        let mut stream = match DAEMON_CLIENT.clone().read_interface_data(request).await {
            Ok(response) => response.into_inner(),
            Err(err) => {
                log::warn!("Failed to connect to stats stream for interface {name}: {err}");
                continue;
            }
        };

        // Take only the first stream item — a one-shot stats snapshot.
        let interface_data = match stream.message().await {
            Ok(Some(data)) => data,
            Ok(None) => {
                log::debug!("Empty stats stream for interface {name}, skipping");
                continue;
            }
            Err(err) => {
                log::warn!("Error reading stats for interface {name}: {err}");
                continue;
            }
        };

        log::debug!(
            "Received interface data for {name}: listen_port={}, peer_count={}",
            interface_data.listen_port,
            interface_data.peers.len()
        );

        for peer in &interface_data.peers {
            let public_key = &peer.public_key;

            // Try matching the peer to a Location first.
            match Location::find_by_public_key(pool, public_key).await {
                Ok(location) => {
                    log::debug!(
                        "Matched interface {name} peer {public_key} to location {}",
                        location.name
                    );
                    results.push(ActiveConnectionInfo {
                        connection_type: ConnectionType::Location,
                        target_id: location.id,
                        name: location.name.clone(),
                        interface_name: name.to_string(),
                        stats: peer_stats(&interface_data, peer),
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
                    log::debug!(
                        "Matched interface {name} peer {public_key} to tunnel {}",
                        tunnel.name
                    );
                    results.push(ActiveConnectionInfo {
                        connection_type: ConnectionType::Tunnel,
                        target_id: tunnel.id,
                        name: tunnel.name.clone(),
                        interface_name: name.to_string(),
                        stats: peer_stats(&interface_data, peer),
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

            log::debug!(
                "Peer {public_key} on interface {name} does not match any \
                 Location or Tunnel, skipping"
            );
        }
    }

    log::debug!("active_state: found {} active connection(s)", results.len());
    Ok(results)
}

/// Extract per-peer stats from an `InterfaceData` stream item.
#[cfg(target_os = "linux")]
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
