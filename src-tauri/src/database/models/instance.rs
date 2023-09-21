use crate::{database::DbPool, error::Error};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, FromRow};

#[derive(FromRow, Serialize, Deserialize)]
pub struct Instance {
    pub id: Option<i64>,
    pub name: String,
    pub uuid: String,
    pub url: String,
}

impl Instance {
    pub fn new(name: String, uuid: String, url: String) -> Self {
        Instance {
            id: None,
            name,
            uuid,
            url,
        }
    }

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let result = query!(
            "INSERT INTO instance (name, uuid, url) VALUES ($1, $2, $3) RETURNING id;",
            self.name,
            self.uuid,
            self.url
        )
        .fetch_one(executor)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }
    pub async fn all(pool: &DbPool) -> Result<Vec<Self>, Error> {
        let instances = query_as!(Self, "SELECT id \"id?\", name, uuid, url FROM instance;")
            .fetch_all(pool)
            .await?;
        Ok(instances)
    }
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: Option<i64>,
    pub name: String,
    pub uuid: String,
    pub url: String,
    pub connected: bool,
    pub pubkey: String,
}
