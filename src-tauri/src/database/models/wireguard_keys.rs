use crate::{database::DbPool, error::Error};
use sqlx::{query, query_as, Error as SqlxError};

// User key pair
#[derive(Debug)]
pub struct WireguardKeys {
    pub id: Option<i64>,
    pub instance_id: i64,
    pub pubkey: String,
    pub prvkey: String,
}

impl WireguardKeys {
    pub fn new(instance_id: i64, pubkey: String, prvkey: String) -> Self {
        WireguardKeys {
            id: None,
            instance_id,
            pubkey,
            prvkey,
        }
    }
    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let result = query!(
            "INSERT INTO wireguard_keys (instance_id, pubkey, prvkey) \
            VALUES ($1, $2, $3) \
            RETURNING id;
            ",
            self.instance_id,
            self.pubkey,
            self.prvkey,
        )
        .fetch_one(executor)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }
    pub async fn find_by_instance_id(
        pool: &DbPool,
        instance_id: i64,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", instance_id, pubkey, prvkey \
            FROM wireguard_keys WHERE instance_id = $1;",
            instance_id
        )
        .fetch_optional(pool)
        .await
    }
}
