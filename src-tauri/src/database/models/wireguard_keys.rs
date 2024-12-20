use base64::{prelude::BASE64_STANDARD, Engine};
use sqlx::{query_as, query_scalar, SqliteExecutor};
use x25519_dalek::{PublicKey, StaticSecret};

use super::{Id, NoId};

// User key pair
#[derive(Debug)]
pub struct WireguardKeys<I = NoId> {
    pub id: I,
    pub instance_id: Id,
    pub pubkey: String,
    pub prvkey: String,
}

impl WireguardKeys<Id> {
    pub async fn find_by_instance_id<'e, E>(
        executor: E,
        instance_id: Id,
    ) -> Result<Option<Self>, sqlx::Error>
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
    pub fn new(instance_id: Id, pubkey: String, prvkey: String) -> Self {
        Self {
            id: NoId,
            instance_id,
            pubkey,
            prvkey,
        }
    }

    #[must_use]
    pub fn generate(instance_id: Id) -> Self {
        let secret = StaticSecret::random();
        let public_key = PublicKey::from(&secret);

        Self {
            id: NoId,
            instance_id,
            pubkey: BASE64_STANDARD.encode(public_key),
            prvkey: BASE64_STANDARD.encode(secret.as_bytes()),
        }
    }

    pub async fn save<'e, E>(self, executor: E) -> Result<WireguardKeys<Id>, sqlx::Error>
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
