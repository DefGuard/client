use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;

use crate::{
    app_config::AppConfig,
    database::{
        models::{connection::ActiveConnection, instance::Instance, location::Location, Id},
        DbPool,
    },
    service::{
        proto::desktop_daemon_service_client::DesktopDaemonServiceClient, utils::setup_client,
    },
    utils::disconnect_interface,
    ConnectionType,
};

pub struct AppState {
    pub db: std::sync::Mutex<Option<DbPool>>,
    pub active_connections: Arc<Mutex<Vec<ActiveConnection>>>,
    pub client: DesktopDaemonServiceClient<Channel>,
    pub log_watchers: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>>,
    pub app_config: Arc<std::sync::Mutex<AppConfig>>,
}

impl AppState {
    #[must_use]
    pub fn new(app_handle: &AppHandle) -> Self {
        let client = setup_client().expect("Failed to setup gRPC client");
        AppState {
            db: std::sync::Mutex::new(None),
            active_connections: Arc::new(Mutex::new(Vec::new())),
            client,
            log_watchers: Arc::new(std::sync::Mutex::new(HashMap::new())),
            app_config: Arc::new(std::sync::Mutex::new(AppConfig::new(app_handle))),
        }
    }

    pub(crate) fn get_pool(&self) -> DbPool {
        self.db
            .lock()
            .expect("Failed to lock dbpool mutex")
            .clone()
            .expect("Missing database connection pool")
    }

    /// Try to remove a connection from the list of active connections.
    /// Return removed connection, or `None` if not found.
    pub async fn remove_connection(
        &self,
        location_id: Id,
        connection_type: &ConnectionType,
    ) -> Option<ActiveConnection> {
        trace!("Removing active connection for location with id: {location_id}");
        let mut connections = self.active_connections.lock().await;

        if let Some(index) = connections.iter().position(|conn| {
            conn.location_id == location_id && conn.connection_type.eq(connection_type)
        }) {
            // Found a connection with the specified location_id
            let removed_connection = connections.remove(index);
            trace!("Active connection has been removed from the active connections list.");
            Some(removed_connection)
        } else {
            debug!("No active connection found with location_id: {location_id}");
            None
        }
    }

    pub async fn get_connection_id_by_type(&self, connection_type: &ConnectionType) -> Vec<Id> {
        let active_connections = self.active_connections.lock().await;

        let connection_ids = active_connections
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
        debug!("Closing all active connections...");
        let active_connections = self.active_connections.lock().await;
        let active_connections_count = active_connections.len();
        debug!("Found {} active connections", active_connections_count);
        for connection in active_connections.iter() {
            debug!(
                "Found active connection with location {}",
                connection.location_id
            );
            trace!("Connection: {connection:#?}");
            debug!("Removing interface {}", connection.interface_name);
            disconnect_interface(connection, self).await?;
        }
        if active_connections_count > 0 {
            info!("All active connections ({active_connections_count}) have been closed.");
        } else {
            debug!("There were no active connections to close, nothing to do.");
        }
        Ok(())
    }

    pub async fn find_connection(
        &self,
        id: Id,
        connection_type: ConnectionType,
    ) -> Option<ActiveConnection> {
        let connections = self.active_connections.lock().await;
        trace!(
        "Checking for active connection with id: {id}, connection_type: {connection_type:?} in active connections."
    );

        if let Some(connection) = connections
            .iter()
            .find(|conn| conn.location_id == id && conn.connection_type == connection_type)
        {
            // 'connection' now contains the first element with the specified id and connection_type
            trace!("Found connection: {connection:?}");
            Some(connection.to_owned())
        } else {
            debug!("Couldn't find connection with id: {id}, connection_type: {connection_type:?} in active connections.");
            None
        }
    }

    /// Returns active connections for a given instance.
    pub(crate) async fn active_connections(
        &self,
        instance: &Instance<Id>,
    ) -> Result<Vec<ActiveConnection>, crate::error::Error> {
        let locations: HashSet<Id> = Location::find_by_instance_id(&self.get_pool(), instance.id)
            .await?
            .iter()
            .map(|location| location.id)
            .collect();
        Ok(self
            .active_connections
            .lock()
            .await
            .iter()
            .filter(|connection| locations.contains(&connection.location_id))
            .cloned()
            .collect())
    }

    /// Close all connections, then terminate the application.
    pub fn quit(&self, app_handle: &AppHandle) {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let _ = self.close_all_connections().await;
                app_handle.exit(0);
            });
        });
    }
}
