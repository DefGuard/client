use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, SqliteExecutor};

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
    pub enterprise_enabled: bool,
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
            disable_all_traffic: instance_info.disable_all_traffic,
            enterprise_enabled: instance_info.enterprise_enabled,
        }
    }
}

impl Instance<Id> {
    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        let url = self.url.to_string();
        let proxy_url = self.proxy_url.to_string();
        // Update the existing record when there is an ID
        query!(
            "UPDATE instance SET name = $1, uuid = $2, url = $3, proxy_url = $4, username = $5, disable_all_traffic = $6, enterprise_enabled = $7, token = $8 WHERE id = $9;",
            self.name,
            self.uuid,
            url,
            proxy_url,
            self.username,
            self.disable_all_traffic,
            self.enterprise_enabled,
            self.token,
            self.id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn all<'e, E>(executor: E) -> Result<Vec<Self>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let instances = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", disable_all_traffic, enterprise_enabled FROM instance;"
        )
        .fetch_all(executor)
        .await?;
        Ok(instances)
    }

    pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Self>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let instance = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", disable_all_traffic, enterprise_enabled FROM instance WHERE id = $1;",
            id
        )
        .fetch_optional(executor)
        .await?;
        Ok(instance)
    }

    pub async fn find_by_uuid<'e, E>(executor: E, uuid: &str) -> Result<Option<Self>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let instance = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", disable_all_traffic, enterprise_enabled FROM instance WHERE uuid = $1;",
            uuid
        )
        .fetch_optional(executor)
        .await?;
        Ok(instance)
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: i64) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        // delete instance
        query!("DELETE FROM instance WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }

    pub async fn delete<'e, E>(&self, executor: E) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        Instance::delete_by_id(executor, self.id).await?;
        Ok(())
    }

    pub async fn disable_enterprise_features<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        debug!(
            "Disabling enterprise features for instance {}({})",
            self.name, self.id
        );
        self.enterprise_enabled = false;
        self.disable_all_traffic = false;
        self.save(executor).await?;
        debug!(
            "Disabled enterprise features for instance {}({})",
            self.name, self.id
        );
        Ok(())
    }
}

// This compares proto::InstanceInfo, not to be confused with regular InstanceInfo defined below
impl PartialEq<proto::InstanceInfo> for Instance<Id> {
    fn eq(&self, other: &proto::InstanceInfo) -> bool {
        self.name == other.name
            && self.uuid == other.id
            && self.url == other.url
            && self.proxy_url == other.proxy_url
            && self.username == other.username
            && self.disable_all_traffic == other.disable_all_traffic
            && self.enterprise_enabled == other.enterprise_enabled
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
            enterprise_enabled: false,
        }
    }

    pub async fn save<'e, E>(self, executor: E) -> Result<Instance<Id>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let url = self.url.clone();
        let proxy_url = self.proxy_url.clone();
        let result = query!(
            "INSERT INTO instance (name, uuid, url, proxy_url, username, token, disable_all_traffic, enterprise_enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id;",
            self.name,
            self.uuid,
            url,
            proxy_url,
            self.username,
            self.token,
            self.disable_all_traffic,
            self.enterprise_enabled
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
            enterprise_enabled: self.enterprise_enabled,
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
    pub enterprise_enabled: bool,
}
