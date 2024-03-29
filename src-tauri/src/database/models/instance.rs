use crate::{database::DbPool, error::Error, proto};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, FromRow};

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Instance {
    pub id: Option<i64>,
    pub name: String,
    pub uuid: String,
    pub url: String,
    pub proxy_url: String,
    pub username: String,
}

impl From<proto::InstanceInfo> for Instance {
    fn from(instance_info: proto::InstanceInfo) -> Self {
        Self {
            id: None,
            name: instance_info.name,
            uuid: instance_info.id,
            url: instance_info.url,
            proxy_url: instance_info.proxy_url,
            username: instance_info.username,
        }
    }
}

impl Instance {
    #[must_use]
    pub fn new(
        name: String,
        uuid: String,
        url: String,
        proxy_url: String,
        username: String,
    ) -> Self {
        Instance {
            id: None,
            name,
            uuid,
            url,
            proxy_url,
            username,
        }
    }

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let url = self.url.to_string();
        let proxy_url = self.proxy_url.to_string();
        match self.id {
            None => {
                let result = query!(
                    "INSERT INTO instance (name, uuid, url, proxy_url, username) VALUES ($1, $2, $3, $4, $5) RETURNING id;",
                    self.name,
                    self.uuid,
                    url,
                    proxy_url,
                    self.username,
                )
                .fetch_one(executor)
                .await?;
                self.id = Some(result.id);
                Ok(())
            }
            Some(id) => {
                // Update the existing record when there is an ID
                query!(
                    "UPDATE instance SET name = $1, uuid = $2, url = $3, proxy_url = $4, username = $5 WHERE id = $6;",
                    self.name,
                    self.uuid,
                    url,
                    proxy_url,
                    self.username,
                    id
                )
                .execute(executor)
                .await?;
                Ok(())
            }
        }
    }

    pub async fn all(pool: &DbPool) -> Result<Vec<Self>, Error> {
        let instances = query_as!(
            Self,
            "SELECT id \"id?\", name, uuid, url, proxy_url, username FROM instance;"
        )
        .fetch_all(pool)
        .await?;
        Ok(instances)
    }

    pub async fn find_by_id(pool: &DbPool, id: i64) -> Result<Option<Self>, Error> {
        let instance = query_as!(
            Self,
            "SELECT id \"id?\", name, uuid, url, proxy_url, username FROM instance WHERE id = $1;",
            id
        )
        .fetch_optional(pool)
        .await?;
        Ok(instance)
    }

    pub async fn delete_by_id(pool: &DbPool, id: i64) -> Result<(), Error> {
        // delete instance
        query!("DELETE FROM instance WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<(), Error> {
        match self.id {
            Some(id) => {
                Instance::delete_by_id(pool, id).await?;
                Ok(())
            }
            None => Err(Error::NotFound),
        }
    }
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: Option<i64>,
    pub name: String,
    pub uuid: String,
    pub url: String,
    pub proxy_url: String,
    pub active: bool,
    pub pubkey: String,
}
