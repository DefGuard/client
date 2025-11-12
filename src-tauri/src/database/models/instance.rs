use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, SqliteExecutor};

use super::{Id, NoId};
use crate::proto;

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
    pub openid_display_name: Option<String>,
}

impl fmt::Display for Instance<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(ID: {})", self.name, self.id)
    }
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
            openid_display_name: instance_info.openid_display_name,
        }
    }
}

impl Instance<Id> {
    pub(crate) async fn save<'e, E>(&mut self, executor: E) -> Result<(), sqlx::Error>
    where
        E: SqliteExecutor<'e>,
    {
        query!(
            "UPDATE instance SET name = $1, uuid = $2, url = $3, proxy_url = $4, username = $5, \
            disable_all_traffic = $6, enterprise_enabled = $7, token = $8, openid_display_name = $9 WHERE id = $10;",
            self.name,
            self.uuid,
            self.url,
            self.proxy_url,
            self.username,
            self.disable_all_traffic,
            self.enterprise_enabled,
            self.token,
            self.openid_display_name,
            self.id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn all<'e, E>(executor: E) -> Result<Vec<Self>, sqlx::Error>
    where
        E: SqliteExecutor<'e>,
    {
        let instances = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", \
            disable_all_traffic, enterprise_enabled, openid_display_name FROM instance ORDER BY name ASC;"
        )
        .fetch_all(executor)
        .await?;
        Ok(instances)
    }

    pub(crate) async fn find_by_id<'e, E>(executor: E, id: Id) -> Result<Option<Self>, sqlx::Error>
    where
        E: SqliteExecutor<'e>,
    {
        let instance = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token \"token?\", \
            disable_all_traffic, enterprise_enabled, openid_display_name FROM instance WHERE id = $1;",
            id
        )
        .fetch_optional(executor)
        .await?;
        Ok(instance)
    }

    pub(crate) async fn delete_by_id<'e, E>(executor: E, id: Id) -> Result<(), sqlx::Error>
    where
        E: SqliteExecutor<'e>,
    {
        // delete instance
        query!("DELETE FROM instance WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: SqliteExecutor<'e>,
    {
        Instance::delete_by_id(executor, self.id).await?;
        Ok(())
    }

    pub(crate) async fn all_with_token<'e, E>(executor: E) -> Result<Vec<Self>, sqlx::Error>
    where
        E: SqliteExecutor<'e>,
    {
        let instances = query_as!(
            Self,
            "SELECT id \"id: _\", name, uuid, url, proxy_url, username, token, \
            disable_all_traffic, enterprise_enabled, openid_display_name FROM instance
            WHERE token IS NOT NULL ORDER BY name ASC;"
        )
        .fetch_all(executor)
        .await?;
        Ok(instances)
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
            && self.openid_display_name == other.openid_display_name
    }
}

impl Instance<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> Result<Instance<Id>, sqlx::Error>
    where
        E: SqliteExecutor<'e>,
    {
        let url = self.url.clone();
        let proxy_url = self.proxy_url.clone();
        let result = query!(
            "INSERT INTO instance (name, uuid, url, proxy_url, username, token, \
            disable_all_traffic, enterprise_enabled) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id;",
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
            openid_display_name: self.openid_display_name,
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
    pub openid_display_name: Option<String>,
}

impl fmt::Display for InstanceInfo<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(ID: {})", self.name, self.id)
    }
}
