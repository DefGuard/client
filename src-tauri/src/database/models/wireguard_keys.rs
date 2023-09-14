use crate::{database::DbPool, error::Error};
use base64::Engine;
use sqlx::{query, query_as, Error as SqlxError};
use x25519_dalek::{PublicKey, StaticSecret};

// User key pair
pub struct WireguardKeys {
    pub id: Option<i64>,
    pub instance_id: i64,
    pub pubkey: String,
    pub prvkey: String,
}

impl WireguardKeys {
    pub fn new(instance_id: i64) -> Self {
        let secret = StaticSecret::random();

        // Derive the corresponding public key
        let public_key = PublicKey::from(&secret);
        // Convert the keys to WireGuard's base64 format
        let prvkey = base64::prelude::BASE64_STANDARD.encode(secret.to_bytes());
        let pubkey = base64::prelude::BASE64_STANDARD.encode(public_key.to_bytes());

        WireguardKeys {
            id: None,
            instance_id,
            pubkey,
            prvkey,
        }
    }
    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        let result = query!(
            "INSERT INTO wireguard_keys (instance_id, pubkey, prvkey) \
            VALUES ($1, $2, $3) \
            RETURNING id;
            ",
            self.instance_id,
            self.pubkey,
            self.prvkey,
        )
        .fetch_one(pool)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }
    pub async fn find_by_location_id(
        pool: &DbPool,
        location_id: i64,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", instance_id, pubkey, prvkey \
            FROM wireguard_keys WHERE instance_id = $1;",
            location_id
        )
        .fetch_optional(pool)
        .await
    }
}
