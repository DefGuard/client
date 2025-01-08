use std::collections::{HashMap, HashSet};

use tauri::{
    async_runtime::{spawn, JoinHandle},
    AppHandle,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;

use crate::{
    app_config::AppConfig,
    database::{
        init_db,
        models::{connection::ActiveConnection, instance::Instance, location::Location, Id},
        DbPool,
    },
    error::Error,
    service::{
        proto::desktop_daemon_service_client::DesktopDaemonServiceClient, utils::setup_client,
    },
    utils::{disconnect_interface, stats_handler},
    ConnectionType,
};

pub struct AppState {
    pub db: DbPool,
    pub active_connections: Mutex<Vec<ActiveConnection>>,
    pub client: DesktopDaemonServiceClient<Channel>,
    pub log_watchers: std::sync::Mutex<HashMap<String, CancellationToken>>,
    pub app_config: std::sync::Mutex<AppConfig>,
    stat_threads: std::sync::Mutex<HashMap<Id, JoinHandle<()>>>, // location ID is the key
}

impl AppState {
    #[must_use]
    pub fn new(app_handle: &AppHandle) -> Self {
        AppState {
            db: init_db().expect("Failed to initalize database"),
            active_connections: Mutex::new(Vec::new()),
            client: setup_client().expect("Failed to setup gRPC client"),
            log_watchers: std::sync::Mutex::new(HashMap::new()),
            app_config: std::sync::Mutex::new(AppConfig::new(app_handle)),
            stat_threads: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn add_connection<S: Into<String>>(
        &self,
        location_id: Id,
        interface_name: S,
        connection_type: ConnectionType,
    ) {
        let ifname = interface_name.into();
        let connection = ActiveConnection::new(location_id, ifname.clone(), connection_type);
        debug!("Adding active connection for location ID: {location_id}");
        let mut connections = self.active_connections.lock().await;
        connections.push(connection);
        trace!("Current active connections: {connections:?}");
        drop(connections);

        debug!("Spawning thread for network statistics for location ID {location_id}");
        let handle = spawn(stats_handler(
            self.db.clone(),
            ifname,
            connection_type,
            self.client.clone(),
        ));
        let Some(old_handle) = self
            .stat_threads
            .lock()
            .unwrap()
            .insert(location_id, handle)
        else {
            return;
        };
        warn!("Something went wrong: old network statistics thread still exists");
        old_handle.abort();
        if let Err(err) = old_handle.await {
            debug!("Old network statistics thread for location ID {location_id} returned {err}");
        }
    }

    /// Try to remove a connection from the list of active connections.
    /// Return removed connection, or `None` if not found.
    pub(crate) async fn remove_connection(
        &self,
        location_id: Id,
        connection_type: ConnectionType,
    ) -> Option<ActiveConnection> {
        debug!("Removing active connection for location ID: {location_id}");

        // Stop statistics thread
        {
            let handle = self.stat_threads.lock().unwrap().remove(&location_id);
            if let Some(handle) = handle {
                debug!("Stopping network statistics thread for location ID {location_id}");
                handle.abort();
                if let Err(err) = handle.await {
                    debug!(
                        "Network statistics thread for location ID {location_id} returned {err}"
                    );
                }
            }
        }

        let mut connections = self.active_connections.lock().await;
        if let Some(index) = connections.iter().position(|conn| {
            conn.location_id == location_id && conn.connection_type == connection_type
        }) {
            // Found a connection with the specified location_id
            let removed_connection = connections.remove(index);
            debug!("Active connection has been removed from the active connections list.");
            Some(removed_connection)
        } else {
            debug!("No active connection found with location ID: {location_id}");
            None
        }
    }

    pub(crate) async fn get_connection_id_by_type(
        &self,
        connection_type: ConnectionType,
    ) -> Vec<Id> {
        let active_connections = self.active_connections.lock().await;

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

    pub(crate) async fn close_all_connections(&self) -> Result<(), crate::error::Error> {
        debug!("Closing all active connections");
        let active_connections = self.active_connections.lock().await;
        let active_connections_count = active_connections.len();
        debug!("Found {active_connections_count} active connections");
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

    pub(crate) async fn find_connection(
        &self,
        id: Id,
        connection_type: ConnectionType,
    ) -> Option<ActiveConnection> {
        let connections = self.active_connections.lock().await;
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
            debug!("Couldn't find connection with ID {id}, type: {connection_type} in active connections.");
            None
        }
    }

    /// Returns active connections for a given instance.
    pub(crate) async fn active_connections(
        &self,
        instance: &Instance<Id>,
    ) -> Result<Vec<ActiveConnection>, Error> {
        let locations: HashSet<Id> = Location::find_by_instance_id(&self.db, instance.id)
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
