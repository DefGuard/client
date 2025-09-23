use std::{collections::HashSet, sync::LazyLock};

use tokio::sync::Mutex;

use crate::{
    database::{
        models::{connection::ActiveConnection, instance::Instance, location::Location, Id},
        DB_POOL,
    },
    error::Error,
    utils::disconnect_interface,
    ConnectionType,
};

pub(crate) static ACTIVE_CONNECTIONS: LazyLock<Mutex<Vec<ActiveConnection>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub(crate) async fn get_connection_id_by_type(connection_type: ConnectionType) -> Vec<Id> {
    let active_connections = ACTIVE_CONNECTIONS.lock().await;

    let connection_ids = active_connections
        .iter()
        .filter_map(|con| {
            if con.connection_type == connection_type {
                Some(con.location_id)
            } else {
                None
            }
        })
        .collect();

    connection_ids
}

pub async fn close_all_connections() -> Result<(), Error> {
    debug!("Closing all active connections");
    let active_connections = ACTIVE_CONNECTIONS.lock().await;
    let active_connections_count = active_connections.len();
    debug!("Found {active_connections_count} active connections");
    for connection in active_connections.iter() {
        debug!(
            "Found active connection with location {}",
            connection.location_id
        );
        trace!("Connection: {connection:#?}");
        debug!("Removing interface {}", connection.interface_name);
        disconnect_interface(connection).await?;
    }
    if active_connections_count > 0 {
        info!("All active connections ({active_connections_count}) have been closed.");
    } else {
        debug!("There were no active connections to close, nothing to do.");
    }
    Ok(())
}

pub(crate) async fn find_connection(
    id: Id,
    connection_type: ConnectionType,
) -> Option<ActiveConnection> {
    let connections = ACTIVE_CONNECTIONS.lock().await;
    trace!(
        "Checking for active connection with ID {id}, type {connection_type} in active connections."
    );

    if let Some(connection) = connections
        .iter()
        .find(|conn| conn.location_id == id && conn.connection_type == connection_type)
    {
        // 'connection' now contains the first element with the specified id and connection_type
        trace!("Found connection: {connection:?}");
        Some(connection.to_owned())
    } else {
        debug!(
            "Couldn't find connection with ID {id}, type: {connection_type} in active connections."
        );
        None
    }
}

/// Returns active connections for a given instance.
pub(crate) async fn active_connections(
    instance: &Instance<Id>,
) -> Result<Vec<ActiveConnection>, Error> {
    let locations: HashSet<Id> = Location::find_by_instance_id(&*DB_POOL, instance.id)
        .await?
        .iter()
        .map(|location| location.id)
        .collect();
    Ok(ACTIVE_CONNECTIONS
        .lock()
        .await
        .iter()
        .filter(|connection| locations.contains(&connection.location_id))
        .cloned()
        .collect())
}
