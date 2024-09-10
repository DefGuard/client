use serde::{Deserialize, Serialize};
use sqlx::{query, query_as};

use super::{Id, NoId};
use crate::{error::Error, proto};

#[derive(Serialize, Deserialize, Debug)]
pub struct Instance<I = NoId> {
    pub id: I,
    pub name: String,
    pub uuid: String,
    pub url: String,
    pub proxy_url: String,
    pub username: String,
    pub token: Option<String>,
    pub disable_all_traffic: bool,
}

impl From<proto::InstanceInfo> for Instance<NoId> {
    fn from(instance_info: proto::InstanceInfo) -> Self {
        Self {
            id: NoId,
            name: instance_info.name,
            uuid: instance_info.id,
            url: instance_info.url,
            proxy_url: instance_info.proxy_url,
            username: instance_info.username,
            token: None,
            disable_all_traffic: false,
        }
    }
}

impl Instance<Id> {
    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let url = self.url.to_string();
        let proxy_url = self.proxy_url.to_string();
        // Update the existing record when there is an ID
        query!(
            "UPDATE instance SET name = $1, uuid = $2, url = $3, proxy_url = $4, username = $5, disable_all_traffic = $6 WHERE id = $7;",
            self.name,
            self.uuid,
            url,
            proxy_url,
            self.username,
            self.disable_all_traffic,
            self.id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn all<'e, E>(executor: E) -> Result<Vec<Self>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let instances = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", disable_all_traffic FROM instance;"
        )
        .fetch_all(executor)
        .await?;
        Ok(instances)
    }

    pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Self>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let instance = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", disable_all_traffic FROM instance WHERE id = $1;",
            id
        )
        .fetch_optional(executor)
        .await?;
        Ok(instance)
    }

    pub async fn find_by_uuid<'e, E>(executor: E, uuid: &str) -> Result<Option<Self>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let instance = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", disable_all_traffic FROM instance WHERE uuid = $1;",
            uuid
        )
        .fetch_optional(executor)
        .await?;
        Ok(instance)
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: i64) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        // delete instance
        query!("DELETE FROM instance WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }

    pub async fn delete<'e, E>(&self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        Instance::delete_by_id(executor, self.id).await?;
        Ok(())
    }
}

impl Instance<NoId> {
    #[must_use]
    pub fn new(
        name: String,
        uuid: String,
        url: String,
        proxy_url: String,
        username: String,
    ) -> Instance<NoId> {
        Instance {
            id: NoId,
            name,
            uuid,
            url,
            proxy_url,
            username,
            token: None,
            disable_all_traffic: false,
        }
    }

    pub async fn save<'e, E>(self, executor: E) -> Result<Instance<Id>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let url = self.url.clone();
        let proxy_url = self.proxy_url.clone();
        let result = query!(
            "INSERT INTO instance (name, uuid, url, proxy_url, username, token, disable_all_traffic) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id;",
            self.name,
            self.uuid,
            url,
            proxy_url,
            self.username,
            self.token,
            self.disable_all_traffic
        )
        .fetch_one(executor)
        .await?;
        Ok(Instance::<Id> {
            id: result.id,
            name: self.name,
            uuid: self.uuid,
            url: self.url,
            proxy_url: self.proxy_url,
            username: self.username,
            token: self.token,
            disable_all_traffic: self.disable_all_traffic,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceInfo<I = NoId> {
    pub id: I,
    pub name: String,
    pub uuid: String,
    pub url: String,
    pub proxy_url: String,
    pub active: bool,
    pub pubkey: String,
    pub disable_all_traffic: bool,
}
