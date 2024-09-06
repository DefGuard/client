use chrono::{NaiveDateTime, Utc};
use sqlx::{query, query_as, FromRow};
use std::time::SystemTime;

use crate::{
    commands::DateTimeAggregation,
    database::{DbPool, Location},
    error::Error,
    CommonLocationStats, ConnectionType,
};
use defguard_wireguard_rs::host::Peer;
use serde::{Deserialize, Serialize};

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct LocationStats {
    id: Option<i64>,
    location_id: i64,
    upload: i64,
    download: i64,
    last_handshake: i64,
    collected_at: NaiveDateTime,
    listen_port: u32,
    persistent_keepalive_interval: Option<u16>,
}

impl From<LocationStats> for CommonLocationStats {
    fn from(location_stats: LocationStats) -> Self {
        CommonLocationStats {
            id: location_stats.id,
            location_id: location_stats.location_id,
            upload: location_stats.upload,
            download: location_stats.download,
            last_handshake: location_stats.last_handshake,
            collected_at: location_stats.collected_at,
            listen_port: location_stats.listen_port,
            persistent_keepalive_interval: location_stats.persistent_keepalive_interval,
            connection_type: ConnectionType::Location,
        }
    }
}

pub async fn peer_to_location_stats(
    peer: &Peer,
    listen_port: u32,
    pool: &DbPool,
) -> Result<LocationStats, Error> {
    let location = Location::find_by_public_key(pool, &peer.public_key.to_string()).await?;
    Ok(LocationStats {
        id: None,
        location_id: location.id,
        upload: peer.tx_bytes as i64,
        download: peer.rx_bytes as i64,
        last_handshake: peer.last_handshake.map_or(0, |ts| {
            ts.duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs() as i64)
        }),
        collected_at: Utc::now().naive_utc(),
        listen_port,
        persistent_keepalive_interval: peer.persistent_keepalive_interval,
    })
}

impl LocationStats {
    #[must_use]
    pub fn new(
        location_id: i64,
        upload: i64,
        download: i64,
        last_handshake: i64,
        collected_at: NaiveDateTime,
        listen_port: u32,
        persistent_keepalive_interval: Option<u16>,
    ) -> Self {
        LocationStats {
            id: None,
            location_id,
            upload,
            download,
            last_handshake,
            collected_at,
            listen_port,
            persistent_keepalive_interval,
        }
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        let result = query!(
            "INSERT INTO location_stats (location_id, upload, download, last_handshake, collected_at, listen_port, persistent_keepalive_interval) \
            VALUES ($1, $2, $3, $4, $5, $6, $7) \
            RETURNING id;",
            self.location_id,
            self.upload,
            self.download,
            self.last_handshake,
            self.collected_at,
            self.listen_port,
            self.persistent_keepalive_interval,
        )
        .fetch_one(pool)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }

    pub async fn all_by_location_id(
        pool: &DbPool,
        location_id: i64,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<Self>, Error> {
        let aggregation = aggregation.fstring();
        let stats = query_as!(
            LocationStats,
            "WITH cte AS ( \
                SELECT \
                    id, location_id, \
                    COALESCE(upload - LAG(upload) OVER (PARTITION BY location_id ORDER BY collected_at), 0) upload, \
                    COALESCE(download - LAG(download) OVER (PARTITION BY location_id ORDER BY collected_at), 0) download, \
                    last_handshake, strftime($1, collected_at) collected_at, listen_port, persistent_keepalive_interval \
                FROM location_stats \
                ORDER BY collected_at \
	            LIMIT -1 OFFSET 1 \
            ) \
            SELECT \
                id, location_id, \
            	SUM(MAX(upload, 0)) \"upload!: i64\", \
            	SUM(MAX(download, 0)) \"download!: i64\", \
            	last_handshake, \
            	collected_at \"collected_at!: NaiveDateTime\", \
            	listen_port \"listen_port!: u32\", \
            	persistent_keepalive_interval \"persistent_keepalive_interval?: u16\" \
            FROM cte \
            WHERE location_id = $2 AND collected_at >= $3 \
            GROUP BY collected_at ORDER BY collected_at",
            aggregation,
            location_id,
            from
        )
        .fetch_all(pool)
        .await?;
        Ok(stats)
    }
}
