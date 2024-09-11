use crate::error::Error;
use sqlx::{query_as, query_scalar, Error as SqlxError, SqliteExecutor};

use super::{Id, NoId};

// User key pair
#[derive(Debug)]
pub struct WireguardKeys<I = NoId> {
    pub id: I,
    pub instance_id: i64,
    pub pubkey: String,
    pub prvkey: String,
}

impl WireguardKeys<Id> {
    pub async fn find_by_instance_id<'e, E>(
        executor: E,
        instance_id: i64,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, pubkey, prvkey \
            FROM wireguard_keys WHERE instance_id = $1",
            instance_id
        )
        .fetch_optional(executor)
        .await
    }
}

impl WireguardKeys<NoId> {
    #[must_use]
    pub fn new(instance_id: i64, pubkey: String, prvkey: String) -> Self {
        WireguardKeys {
            id: NoId,
            instance_id,
            pubkey,
            prvkey,
        }
    }

    pub async fn save<'e, E>(self, executor: E) -> Result<WireguardKeys<Id>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO wireguard_keys (instance_id, pubkey, prvkey) \
            VALUES ($1, $2, $3) RETURNING id \"id!\"",
            self.instance_id,
            self.pubkey,
            self.prvkey,
        )
        .fetch_one(executor)
        .await?;
        Ok(WireguardKeys::<Id> {
            id,
            instance_id: self.instance_id,
            pubkey: self.pubkey,
            prvkey: self.prvkey,
        })
    }
}
