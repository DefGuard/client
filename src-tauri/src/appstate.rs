use crate::{
    database::{ActiveConnection, DbPool},
    service::{
        proto::desktop_daemon_service_client::DesktopDaemonServiceClient, utils::setup_client,
    },
};
use std::sync::{Arc, Mutex};
use tonic::transport::Channel;

pub struct AppState {
    pub db: Arc<Mutex<Option<DbPool>>>,
    pub active_connections: Arc<Mutex<Vec<ActiveConnection>>>,
    pub client: DesktopDaemonServiceClient<Channel>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let client = setup_client().expect("Failed to setup gRPC client");
        AppState {
            db: Arc::new(Mutex::new(None)),
            active_connections: Arc::new(Mutex::new(Vec::new())),
            client,
        }
    }

    pub fn get_pool(&self) -> DbPool {
        self.db.lock().unwrap().as_ref().cloned().unwrap()
    }
    pub fn get_connections(&self) -> Vec<ActiveConnection> {
        self.active_connections.lock().unwrap().clone()
    }
    pub fn find_and_remove_connection(&self, location_id: i64) -> Option<ActiveConnection> {
        debug!(
            "Removing active connection for location with id: {}",
            location_id
        );
        let mut connections = self.active_connections.lock().unwrap();

        if let Some(index) = connections
            .iter()
            .position(|conn| conn.location_id == location_id)
        {
            // Found a connection with the specified location_id
            let removed_connection = connections.remove(index);
            info!(
                "Removed connection from active connections: {:#?}",
                removed_connection
            );
            Some(removed_connection)
        } else {
            None // Connection not found
        }
    }
    pub fn find_connection(&self, location_id: i64) -> Option<ActiveConnection> {
        let connections = self.active_connections.lock().unwrap();
        debug!("Checking for active connection with location id: {location_id} in active connections: {:#?}", connections);

        if let Some(connection) = connections
            .iter()
            .find(|conn| conn.location_id == location_id)
        {
            // 'connection' now contains the first element with the specified location_id
            debug!("Found connection: {:#?}", connection);
            Some(connection.to_owned())
        } else {
            error!("Element with location_id {} not found.", location_id);
            None
        }
    }
}
