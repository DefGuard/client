use serde::{Deserialize, Serialize};

pub mod connection;
pub mod instance;
pub mod location;
pub mod location_stats;
pub mod tunnel;
#[cfg(target_os = "macos")]
pub mod tunnel_configuration;
pub mod wireguard_keys;

// Typestate structs to make working with optional IDs easier
pub type Id = i64;
#[derive(Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NoId;

const PURGE_DURATION: chrono::Duration = chrono::Duration::days(30);

#[cfg(target_os = "macos")]
use self::{location::Location, tunnel::Tunnel};

#[must_use]
/// Utility function to get all tunnels and locations from the database.
#[cfg(target_os = "macos")]
pub async fn get_all_tunnels_locations() -> (Vec<Tunnel<Id>>, Vec<Location<Id>>) {
    let tunnels = Tunnel::all(&*super::DB_POOL).await.unwrap_or_default();
    let locations = Location::all(&*super::DB_POOL, false)
        .await
        .unwrap_or_default();
    (tunnels, locations)
}
