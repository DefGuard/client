use chrono::{NaiveDateTime, Utc};
use sqlx::{query, query_as, Error as SqlxError, FromRow};
use std::time::SystemTime;

use crate::{commands::DateTimeAggregation, database::DbPool, error::Error};
use defguard_wireguard_rs::host::Peer;
use serde::{Deserialize, Serialize};

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct Location {
    pub id: Option<i64>,
    pub instance_id: i64,
    // Native id of network from defguard
    pub network_id: i64,
    pub name: String,
    pub address: String,
    pub pubkey: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub dns: Option<String>,
    pub route_all_traffic: bool,
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct LocationStats {
    id: Option<i64>,
    location_id: i64,
    upload: i64,
    download: i64,
    last_handshake: i64,
    collected_at: NaiveDateTime,
}

pub async fn peer_to_location_stats(peer: &Peer, pool: &DbPool) -> Result<LocationStats, Error> {
    let location = Location::find_by_public_key(pool, &peer.public_key.to_string()).await?;
    Ok(LocationStats {
        id: None,
        location_id: location.id.unwrap(),
        upload: peer.tx_bytes as i64,
        download: peer.rx_bytes as i64,
        last_handshake: peer.last_handshake.map_or(0, |ts| {
            ts.duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs() as i64)
        }),
        collected_at: Utc::now().naive_utc(),
    })
}

impl Location {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        instance_id: i64,
        network_id: i64,
        name: String,
        address: String,
        pubkey: String,
        endpoint: String,
        allowed_ips: String,
        dns: Option<String>,
    ) -> Self {
        Location {
            id: None,
            instance_id,
            network_id,
            name,
            address,
            pubkey,
            endpoint,
            allowed_ips,
            dns,
            route_all_traffic: false,
        }
    }

    pub async fn all(pool: &DbPool) -> Result<Vec<Self>, Error> {
        let locations = query_as!(
            Self,
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, route_all_traffic \
        FROM location;"
        )
        .fetch_all(pool)
        .await?;
        Ok(locations)
    }

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        match self.id {
            None => {
                // Insert a new record when there is no ID
                let result = query!(
                "INSERT INTO location (instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, route_all_traffic) \
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
                RETURNING id;",
                self.instance_id,
                self.name,
                self.address,
                self.pubkey,
                self.endpoint,
                self.allowed_ips,
                self.dns,
                self.network_id,
                self.route_all_traffic,
            )
            .fetch_one(executor)
            .await?;
                self.id = Some(result.id);
            }
            Some(id) => {
                // Update the existing record when there is an ID
                query!(
                "UPDATE location SET instance_id = $1, name = $2, address = $3, pubkey = $4, endpoint = $5, allowed_ips = $6, dns = $7, \
                network_id = $8, route_all_traffic = $9 WHERE id = $10;",
                self.instance_id,
                self.name,
                self.address,
                self.pubkey,
                self.endpoint,
                self.allowed_ips,
                self.dns,
                self.network_id,
                self.route_all_traffic,
                id,
            )
            .execute(executor)
            .await?;
            }
        }

        Ok(())
    }

    pub async fn find_by_id(pool: &DbPool, location_id: i64) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, route_all_traffic \
            FROM location WHERE id = $1;",
            location_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_instance_id(
        pool: &DbPool,
        instance_id: i64,
    ) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, route_all_traffic \
            FROM location WHERE instance_id = $1;",
            instance_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_public_key(pool: &DbPool, pubkey: &str) -> Result<Self, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, route_all_traffic \
            FROM location WHERE pubkey = $1;",
            pubkey
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_native_id<'e, E>(
        executor: E,
        instance_id: i64,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        query_as!(
            Self,
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, route_all_traffic \
            FROM location WHERE network_id = $1;",
            instance_id
        )
        .fetch_optional(executor)

        .await
    }
}

impl LocationStats {
    #[must_use]
    pub fn new(
        location_id: i64,
        upload: i64,
        download: i64,
        last_handshake: i64,
        collected_at: NaiveDateTime,
    ) -> Self {
        LocationStats {
            id: None,
            location_id,
            upload,
            download,
            last_handshake,
            collected_at,
        }
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        let result = query!(
            "INSERT INTO location_stats (location_id, upload, download, last_handshake, collected_at) \
            VALUES ($1, $2, $3, $4, $5) \
            RETURNING id;",
            self.location_id,
            self.upload,
            self.download,
            self.last_handshake,
            self.collected_at,
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
            r#"
            WITH cte AS (
                SELECT 
                    id, location_id, 
                    COALESCE(upload - LAG(upload) OVER (PARTITION BY location_id ORDER BY collected_at), 0) as upload, 
                    COALESCE(download - LAG(download) OVER (PARTITION BY location_id ORDER BY collected_at), 0) as download, 
                    last_handshake, strftime($1, collected_at) as collected_at
                FROM location_stats
                ORDER BY collected_at
	            LIMIT -1 OFFSET 1
            )
            SELECT 
                id, location_id, 
            	SUM(MAX(upload, 0)) as "upload!: i64", 
            	SUM(MAX(download, 0)) as "download!: i64", 
            	last_handshake, 
            	collected_at as "collected_at!: NaiveDateTime"
            FROM cte
            WHERE location_id = $2
            AND collected_at >= $3
            GROUP BY collected_at
            ORDER BY collected_at;
            "#,
            aggregation,
            location_id,
            from
        )
        .fetch_all(pool)
        .await?;
        Ok(stats)
    }
}
