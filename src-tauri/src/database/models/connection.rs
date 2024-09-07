use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use sqlx::{query, query_as, FromRow};

use crate::{
    database::DbPool, error::Error, CommonConnection, CommonConnectionInfo, ConnectionType,
};

use super::{Id, NoId};

#[derive(FromRow, Debug, Serialize, Clone)]
pub struct Connection<I = NoId> {
    pub id: I,
    pub location_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl Connection<NoId> {
    pub async fn save(self, pool: &DbPool) -> Result<Connection<Id>, Error> {
        let result = query!(
            "INSERT INTO connection (location_id, connected_from, start, end) \
            VALUES ($1, $2, $3, $4) RETURNING id;",
            self.location_id,
            self.connected_from,
            self.start,
            self.end,
        )
        .fetch_one(pool)
        .await?;
        Ok(Connection::<Id> {
            id: result.id,
            location_id: self.location_id,
            connected_from: self.connected_from,
            start: self.start,
            end: self.end,
        })
    }

    pub async fn all_by_location_id(
        pool: &DbPool,
        location_id: i64,
    ) -> Result<Vec<Connection<Id>>, Error> {
        let connections = query_as!(
            Connection,
            "SELECT id, location_id, connected_from, start, end \
            FROM connection WHERE location_id = $1",
            location_id
        )
        .fetch_all(pool)
        .await?;
        Ok(connections)
    }

    pub async fn latest_by_location_id(
        pool: &DbPool,
        location_id: i64,
    ) -> Result<Option<Connection<Id>>, Error> {
        let connection = query_as!(
            Connection,
            "SELECT id, location_id, connected_from, start, end \
            FROM connection WHERE location_id = $1 \
            ORDER BY end DESC LIMIT 1",
            location_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(connection)
    }
}

/// Historical connection
#[derive(FromRow, Debug, Serialize)]
pub struct ConnectionInfo {
    pub id: i64,
    pub location_id: i64,
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
    pub async fn all_by_location_id(pool: &DbPool, location_id: i64) -> Result<Vec<Self>, Error> {
        // Because we store interface information for given timestamp select last upload and download
        // before connection ended
        // FIXME: Optimize query
        let connections = query_as!(
            ConnectionInfo,
            "SELECT c.id \"id!\", c.location_id \"location_id!\", \
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
        .fetch_all(pool)
        .await?;

        Ok(connections)
    }
}

/// Connections stored in memory after creating interface
#[derive(Debug, Serialize, Clone)]
pub struct ActiveConnection {
    pub location_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub interface_name: String,
    pub connection_type: ConnectionType,
}
impl ActiveConnection {
    #[must_use]
    pub fn new(
        location_id: i64,
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
