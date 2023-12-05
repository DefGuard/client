use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use sqlx::{query, query_as, FromRow};

use crate::{database::DbPool, error::Error};

#[derive(FromRow, Debug, Serialize, Clone)]
pub struct Connection {
    pub id: Option<i64>,
    pub location_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl Connection {
    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        let result = query!(
            "INSERT INTO connection (location_id, connected_from, start, end) \
            VALUES ($1, $2, $3, $4) \
            RETURNING id;",
            self.location_id,
            self.connected_from,
            self.start,
            self.end,
        )
        .fetch_one(pool)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }

    pub async fn all_by_location_id(pool: &DbPool, location_id: i64) -> Result<Vec<Self>, Error> {
        let connections = query_as!(
            Connection,
            r#"
            SELECT id, location_id, connected_from, start, end 
            FROM connection
            WHERE location_id = $1
            "#,
            location_id
        )
        .fetch_all(pool)
        .await?;
        Ok(connections)
    }

    pub async fn latest_by_location_id(
        pool: &DbPool,
        location_id: i64,
    ) -> Result<Option<Self>, Error> {
        let connection = query_as!(
            Connection,
            r#"
            SELECT id, location_id, connected_from, start, end
            FROM connection
            WHERE location_id = $1
            ORDER BY end DESC
            LIMIT 1
            "#,
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

impl ConnectionInfo {
    pub async fn all_by_location_id(pool: &DbPool, location_id: i64) -> Result<Vec<Self>, Error> {
        // Because we store interface information for given timestamp select last upload and download
        // before connection ended
        // FIXME: Optimize query
        let connections = query_as!(
            ConnectionInfo,
            r#"
              SELECT
                  c.id as "id!",
                  c.location_id as "location_id!",
                  c.connected_from as "connected_from!",
                  c.start as "start!",
                  c.end as "end!",
                  COALESCE((
                      SELECT ls.upload
                      FROM location_stats AS ls
                      WHERE ls.location_id = c.location_id
                      AND ls.collected_at >= c.start
                      AND ls.collected_at <= c.end
                      ORDER BY ls.collected_at DESC
                      LIMIT 1
                  ), 0) as "upload: _",
                  COALESCE((
                      SELECT ls.download
                      FROM location_stats AS ls
                      WHERE ls.location_id = c.location_id
                      AND ls.collected_at >= c.start
                      AND ls.collected_at <= c.end
                      ORDER BY ls.collected_at DESC
                      LIMIT 1
                  ), 0) as "download: _"
              FROM connection AS c WHERE location_id = $1
              ORDER BY start DESC;
            "#,
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
}
impl ActiveConnection {
    #[must_use]
    pub fn new(location_id: i64, connected_from: String, interface_name: String) -> Self {
        let start = Utc::now().naive_utc();
        Self {
            location_id,
            connected_from,
            start,
            interface_name,
        }
    }
}

impl From<ActiveConnection> for Connection {
    fn from(active_connection: ActiveConnection) -> Self {
        Connection {
            id: None,
            location_id: active_connection.location_id,
            connected_from: active_connection.connected_from,
            start: active_connection.start,
            end: Utc::now().naive_utc(),
        }
    }
}
