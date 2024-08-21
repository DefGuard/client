use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use secrecy::Secret;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;

use crate::{
    database::{ActiveConnection, DbPool},
    service::{
        proto::desktop_daemon_service_client::DesktopDaemonServiceClient, utils::setup_client,
    },
    utils::disconnect_interface,
    ConnectionType,
};

pub struct AppState {
    pub db: Arc<Mutex<Option<DbPool>>>,
    pub active_connections: Arc<Mutex<Vec<ActiveConnection>>>,
    pub client: DesktopDaemonServiceClient<Channel>,
    pub log_watchers: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub db_password: Arc<Mutex<Option<Secret<String>>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        let client = setup_client().expect("Failed to setup gRPC client");
        AppState {
            db: Arc::new(Mutex::new(None)),
            active_connections: Arc::new(Mutex::new(Vec::new())),
            client,
            log_watchers: Arc::new(Mutex::new(HashMap::new())),
            db_password: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_pool(&self) -> DbPool {
        self.db
            .lock()
            .expect("Failed to lock dbpool mutex")
            .as_ref()
            .cloned()
            .unwrap()
    }

    pub fn get_connections(&self) -> Vec<ActiveConnection> {
        self.active_connections
            .lock()
            .expect("Failed to lock active connections mutex")
            .clone()
    }
    pub fn find_and_remove_connection(
        &self,
        location_id: i64,
        connection_type: &ConnectionType,
    ) -> Option<ActiveConnection> {
        debug!("Removing active connection for location with id: {location_id}");
        let mut connections = self.active_connections.lock().unwrap();

        if let Some(index) = connections.iter().position(|conn| {
            conn.location_id == location_id && conn.connection_type.eq(connection_type)
        }) {
            // Found a connection with the specified location_id
            let removed_connection = connections.remove(index);
            info!("Removed connection from active connections: {removed_connection:#?}");
            Some(removed_connection)
        } else {
            debug!("No active connection found with location_id: {location_id}");
            None
        }
    }

    pub fn get_connection_id_by_type(&self, connection_type: &ConnectionType) -> Vec<i64> {
        let active_connections = self.active_connections.lock().unwrap();

        let connection_ids: Vec<i64> = active_connections
            .iter()
            .filter_map(|con| {
                if con.connection_type.eq(connection_type) {
                    Some(con.location_id)
                } else {
                    None
                }
            })
            .collect();

        connection_ids
    }

    pub async fn close_all_connections(&self) -> Result<(), crate::error::Error> {
        info!("Closing all active connections...");
        let active_connections = self.get_connections();
        info!("Found {} active connections", active_connections.len());
        for connection in active_connections {
            debug!(
                "Found active connection with location {}",
                connection.location_id
            );
            trace!("Connection: {connection:#?}");
            debug!("Removing interface {}", connection.interface_name);
            disconnect_interface(connection, self).await?;
        }
        info!("All active connections closed");
        Ok(())
    }

    pub fn find_connection(
        &self,
        id: i64,
        connection_type: ConnectionType,
    ) -> Option<ActiveConnection> {
        let connections = self.active_connections.lock().unwrap();
        debug!(
        "Checking for active connection with id: {id}, connection_type: {connection_type:?} in active connections: {connections:#?}"
    );

        if let Some(connection) = connections
            .iter()
            .find(|conn| conn.location_id == id && conn.connection_type == connection_type)
        {
            // 'connection' now contains the first element with the specified id and connection_type
            debug!("Found connection: {connection:#?}");
            Some(connection.to_owned())
        } else {
            error!("Couldn't find connection with id: {id}, connection_type: {connection_type:?} in active connections.");
            None
        }
    }
}
