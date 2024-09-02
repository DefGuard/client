use chrono::{NaiveDateTime, Utc};
use sqlx::{query, query_as, Error as SqlxError, FromRow};
use std::{
    fmt::{Display, Formatter},
    time::SystemTime,
};

use crate::{
    commands::DateTimeAggregation, database::DbPool, error::Error, CommonLocationStats,
    ConnectionType,
};
use defguard_wireguard_rs::host::Peer;
use serde::{Deserialize, Serialize};

use super::{Id, NoId};

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct Location<I = NoId> {
    pub id: I,
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
    pub mfa_enabled: bool,
    pub keepalive_interval: i64,
}

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

impl Display for Location<Id> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ID {}] {}", self.id, self.name)
    }
}

impl Display for Location<NoId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Location<Id> {
    pub async fn all(pool: &DbPool) -> Result<Vec<Self>, Error> {
        let locations = query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id,\
             route_all_traffic, mfa_enabled, keepalive_interval \
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
        // Update the existing record when there is an ID
        query!(
            "UPDATE location SET instance_id = $1, name = $2, address = $3, pubkey = $4, endpoint = $5, allowed_ips = $6, dns = $7, \
            network_id = $8, route_all_traffic = $9, mfa_enabled = $10, keepalive_interval = $11 WHERE id = $12;",
            self.instance_id,
            self.name,
            self.address,
            self.pubkey,
            self.endpoint,
            self.allowed_ips,
            self.dns,
            self.network_id,
            self.route_all_traffic,
            self.mfa_enabled,
            self.keepalive_interval,
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(pool: &DbPool, location_id: i64) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, \
            route_all_traffic, mfa_enabled, keepalive_interval \
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
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, \
            route_all_traffic, mfa_enabled, keepalive_interval \
            FROM location WHERE instance_id = $1;",
            instance_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_public_key(pool: &DbPool, pubkey: &str) -> Result<Self, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, \
            route_all_traffic, mfa_enabled, keepalive_interval \
            FROM location WHERE pubkey = $1;",
            pubkey
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        info!("Removing location {self:?}");
        query!("DELETE FROM location WHERE id = $1;", self.id)
            .execute(executor)
            .await?;
        Ok(())
    }
}

impl Location<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> Result<Location<Id>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        // Insert a new record when there is no ID
        let result = query!(
                "INSERT INTO location (instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, route_all_traffic, mfa_enabled, keepalive_interval) \
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
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
                self.mfa_enabled,
                self.keepalive_interval
            )
            .fetch_one(executor)
            .await?;

        Ok(Location::<Id> {
            id: result.id,
            instance_id: self.instance_id,
            name: self.name,
            address: self.address,
            pubkey: self.pubkey,
            endpoint: self.endpoint,
            allowed_ips: self.allowed_ips,
            dns: self.dns,
            network_id: self.network_id,
            route_all_traffic: self.route_all_traffic,
            mfa_enabled: self.mfa_enabled,
            keepalive_interval: self.keepalive_interval,
        })
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
