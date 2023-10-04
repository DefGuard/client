use std::sync::Mutex;

use crate::database::{Connection, DbPool};

#[derive(Default)]
pub struct AppState {
    pub db: Mutex<Option<DbPool>>,
    pub active_connections: Mutex<Vec<Connection>>,
}
impl AppState {
    pub fn get_pool(&self) -> DbPool {
        self.db.lock().unwrap().as_ref().cloned().unwrap()
    }
    pub fn find_and_remove_connection(&self, location_id: i64) -> Option<Connection> {
        let mut connections = self.active_connections.lock().unwrap();

        if let Some(index) = connections
            .iter()
            .position(|conn| conn.location_id == location_id)
        {
            // Found a connection with the specified location_id
            let removed_connection = connections.remove(index);
            Some(removed_connection)
        } else {
            None // Connection not found
        }
    }
    pub fn find_connection(&self, location_id: i64) -> Option<Connection> {
        let connections = self.active_connections.lock().unwrap();

        if let Some(connection) = connections
            .iter()
            .find(|conn| conn.location_id == location_id)
        {
            // 'connection' now contains the first element with the specified location_id
            Some(connection.to_owned())
        } else {
            error!("Element with location_id {} not found.", location_id);
            None
        }
    }
}
