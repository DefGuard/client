use std::sync::{Arc, Mutex};

use tonic::transport::Channel;

use crate::{
    database::{ActiveConnection, Connection, DbPool},
    error::Error,
    service::{
        proto::{
            desktop_daemon_service_client::DesktopDaemonServiceClient, RemoveInterfaceRequest,
        },
        utils::setup_client,
    },
};

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
    #[must_use]
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
        debug!("Removing active connection for location with id: {location_id}");
        let mut connections = self.active_connections.lock().unwrap();

        if let Some(index) = connections
            .iter()
            .position(|conn| conn.location_id == location_id)
        {
            // Found a connection with the specified location_id
            let removed_connection = connections.remove(index);
            info!("Removed connection from active connections: {removed_connection:#?}");
            Some(removed_connection)
        } else {
            None // Connection not found
        }
    }

    pub async fn close_all_connections(&self) -> Result<(), crate::error::Error> {
        for connection in self.get_connections() {
            debug!("Found active connection");
            trace!("Connection: {connection:#?}");
            debug!("Removing interface");
            let mut client = self.client.clone();
            let request = RemoveInterfaceRequest {
               interface_name: connection.interface_name.clone(),
               post_down: None,
               pre_down: None,
            };
            if let Err(error) = client.remove_interface(request).await {
                error!("Failed to remove interface: {error}");
                return Err(Error::InternalError);
            }
            debug!("Removed interface");
            debug!("Saving connection");
            trace!("Connection: {connection:#?}");
            let mut connection: Connection = connection.into();
            connection.save(&self.get_pool()).await?;
            debug!("Connection saved");
            trace!("Saved connection: {connection:#?}");
            info!("Location {} disconnected", connection.location_id);
        }
        Ok(())
    }
    pub fn find_connection(&self, location_id: i64) -> Option<ActiveConnection> {
        let connections = self.active_connections.lock().unwrap();
        debug!("Checking for active connection with location id: {location_id} in active connections: {connections:#?}");

        if let Some(connection) = connections
            .iter()
            .find(|conn| conn.location_id == location_id)
        {
            // 'connection' now contains the first element with the specified location_id
            debug!("Found connection: {connection:#?}");
            Some(connection.to_owned())
        } else {
            error!("Element with location_id {location_id} not found.");
            None
        }
    }
}
