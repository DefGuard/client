use chrono::NaiveDateTime;
use sqlx::{query, query_as, Error as SqlxError, FromRow};

use crate::{database::DbPool, error::Error};
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
}

#[derive(FromRow)]
pub struct LocationStats {
    id: Option<i64>,
    location_id: i64,
    upload: i64,
    download: i64,
    last_handshake: i64,
    collected_at: NaiveDateTime,
}

impl Location {
    pub fn new(
        instance_id: i64,
        network_id: i64,
        name: String,
        address: String,
        pubkey: String,
        endpoint: String,
        allowed_ips: String,
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
        }
    }

    pub async fn all(pool: &DbPool) -> Result<Vec<Self>, Error> {
        let locations = query_as!(
            Self,
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, network_id \
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
                "INSERT INTO location (instance_id, name, address, pubkey, endpoint, allowed_ips, network_id) \
                VALUES ($1, $2, $3, $4, $5, $6, $7) \
                RETURNING id;",
                self.instance_id,
                self.name,
                self.address,
                self.pubkey,
                self.endpoint,
                self.allowed_ips,
                self.network_id,
            )
            .fetch_one(executor)
            .await?;
                self.id = Some(result.id);
            }
            Some(id) => {
                // Update the existing record when there is an ID
                query!(
                "UPDATE location SET instance_id = $1, name = $2, address = $3, pubkey = $4, endpoint = $5, allowed_ips = $6, network_id = $7 \
                WHERE id = $8;",
                self.instance_id,
                self.name,
                self.address,
                self.pubkey,
                self.endpoint,
                self.allowed_ips,
                self.network_id,
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
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, network_id \
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
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, network_id \
            FROM location WHERE instance_id = $1;",
            instance_id
        )
        .fetch_all(pool)
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
            "SELECT id \"id?\", instance_id, name, address, pubkey, endpoint, allowed_ips, network_id \
            FROM location WHERE network_id = $1;",
            instance_id
        )
        .fetch_optional(executor)
        .await
    }
}

impl LocationStats {
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
}
