use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use sqlx::{query_as, query_scalar, SqliteExecutor};

use super::{Id, NoId};
use crate::{error::Error, CommonConnection, CommonConnectionInfo, ConnectionType};

#[derive(Debug, Serialize, Clone)]
pub struct Connection<I = NoId> {
    pub id: I,
    pub location_id: Id,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl Connection<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> Result<Connection<Id>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO connection (location_id, connected_from, start, end) \
            VALUES ($1, $2, $3, $4) RETURNING id \"id!\"",
            self.location_id,
            self.connected_from,
            self.start,
            self.end,
        )
        .fetch_one(executor)
        .await?;

        Ok(Connection::<Id> {
            id,
            location_id: self.location_id,
            connected_from: self.connected_from,
            start: self.start,
            end: self.end,
        })
    }

    pub async fn all_by_location_id<'e, E>(
        executor: E,
        location_id: Id,
    ) -> Result<Vec<Connection<Id>>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let connections = query_as!(
            Connection,
            "SELECT id, location_id, connected_from, start, end \
            FROM connection WHERE location_id = $1",
            location_id
        )
        .fetch_all(executor)
        .await?;
        Ok(connections)
    }

    pub async fn latest_by_location_id<'e, E>(
        executor: E,
        location_id: Id,
    ) -> Result<Option<Connection<Id>>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let connection = query_as!(
            Connection,
            "SELECT id, location_id, connected_from, start, end \
            FROM connection WHERE location_id = $1 \
            ORDER BY end DESC LIMIT 1",
            location_id
        )
        .fetch_optional(executor)
        .await?;
        Ok(connection)
    }
}

/// Historical connection
#[derive(Debug, Serialize)]
pub struct ConnectionInfo {
    pub id: Id,
    pub location_id: Id,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub upload: Option<i32>,
    pub download: Option<i32>,
}

impl From<ConnectionInfo> for CommonConnectionInfo {
    fn from(val: ConnectionInfo) -> Self {
        CommonConnectionInfo {
            id: val.id,
            location_id: val.location_id,
            connected_from: val.connected_from,
            start: val.start,
            end: val.end,
            upload: val.upload,
            download: val.download,
        }
    }
}

impl ConnectionInfo {
    pub async fn all_by_location_id<'e, E>(executor: E, location_id: Id) -> Result<Vec<Self>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        // Because we store interface information for given timestamp select last upload and download
        // before connection ended
        // FIXME: Optimize query
        let connections = query_as!(
            ConnectionInfo,
            "SELECT c.id, c.location_id, \
            c.connected_from \"connected_from!\", c.start \"start!\", \
            c.end \"end!\", \
            COALESCE(( \
                SELECT ls.upload \
                FROM location_stats ls \
                WHERE ls.location_id = c.location_id \
                AND ls.collected_at >= c.start \
                AND ls.collected_at <= c.end \
                ORDER BY ls.collected_at DESC LIMIT 1 \
            ), 0) \"upload: _\", \
            COALESCE(( \
                SELECT ls.download \
                FROM location_stats ls \
                WHERE ls.location_id = c.location_id \
                AND ls.collected_at >= c.start \
                AND ls.collected_at <= c.end \
                ORDER BY ls.collected_at DESC LIMIT 1 \
            ), 0) \"download: _\" \
            FROM connection c WHERE location_id = $1 \
            ORDER BY start DESC",
            location_id
        )
        .fetch_all(executor)
        .await?;

        Ok(connections)
    }
}

/// Connections stored in memory after creating interface
#[derive(Clone, Debug, Serialize)]
pub struct ActiveConnection {
    pub location_id: Id,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub interface_name: String,
    pub connection_type: ConnectionType,
}

impl ActiveConnection {
    #[must_use]
    pub fn new(
        location_id: Id,
        connected_from: String,
        interface_name: String,
        connection_type: ConnectionType,
    ) -> Self {
        let start = Utc::now().naive_utc();
        Self {
            location_id,
            connected_from,
            start,
            interface_name,
            connection_type,
        }
    }
}

impl From<&ActiveConnection> for Connection<NoId> {
    fn from(active_connection: &ActiveConnection) -> Self {
        Connection {
            id: NoId,
            location_id: active_connection.location_id,
            connected_from: active_connection.connected_from.clone(),
            start: active_connection.start,
            end: Utc::now().naive_utc(),
        }
    }
}

impl From<Connection<Id>> for CommonConnection<Id> {
    fn from(connection: Connection<Id>) -> Self {
        CommonConnection {
            id: connection.id,
            location_id: connection.location_id,
            connected_from: connection.connected_from,
            start: connection.start,
            end: connection.end,
            connection_type: ConnectionType::Location,
        }
    }
}
