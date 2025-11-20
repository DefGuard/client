use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{prelude::Type, query, query_as, SqliteExecutor};

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
    pub client_traffic_policy: ClientTrafficPolicy,
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
        let client_traffic_policy = ClientTrafficPolicy::from(&instance_info);
        Self {
            id: NoId,
            name: instance_info.name,
            uuid: instance_info.id,
            url: instance_info.url,
            proxy_url: instance_info.proxy_url,
            username: instance_info.username,
            token: None,
            client_traffic_policy,
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
            client_traffic_policy = $6, enterprise_enabled = $7, token = $8, \
            openid_display_name = $9 \
            WHERE id = $10;",
            self.name,
            self.uuid,
            self.url,
            self.proxy_url,
            self.username,
            self.client_traffic_policy,
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
            client_traffic_policy, enterprise_enabled, openid_display_name \
            FROM instance ORDER BY name ASC;"
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
            client_traffic_policy, enterprise_enabled, openid_display_name \
            FROM instance WHERE id = $1;",
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
            client_traffic_policy, enterprise_enabled, openid_display_name \
            FROM instance \
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
        let other_policy = ClientTrafficPolicy::from(other);
        self.name == other.name
            && self.uuid == other.id
            && self.url == other.url
            && self.proxy_url == other.proxy_url
            && self.username == other.username
            && self.client_traffic_policy == other_policy
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
            client_traffic_policy , enterprise_enabled) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id;",
            self.name,
            self.uuid,
            url,
            proxy_url,
            self.username,
            self.token,
            self.client_traffic_policy,
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
            client_traffic_policy: self.client_traffic_policy,
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
    pub client_traffic_policy: ClientTrafficPolicy,
    pub enterprise_enabled: bool,
    pub openid_display_name: Option<String>,
}

impl fmt::Display for InstanceInfo<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(ID: {})", self.name, self.id)
    }
}

/// Describes allowed traffic options for clients connecting to an instance.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[repr(u32)]
#[serde(rename_all = "snake_case")]
pub enum ClientTrafficPolicy {
    /// No restrictions
    None = 0,
    /// Clients are not allowed to route all traffic through the VPN.
    DisableAllTraffic = 1,
    /// Clients are forced to route all traffic through the VPN.
    ForceAllTraffic = 2,
}

/// Retrieves `ClientTrafficPolicy` from `proto::InstanceInfo` while ensuring backwards compatibility
impl From<&proto::InstanceInfo> for ClientTrafficPolicy {
    fn from(instance: &proto::InstanceInfo) -> Self {
        match (
            instance.client_traffic_policy,
            #[allow(deprecated)]
            instance.disable_all_traffic,
        ) {
            (Some(policy), _) => ClientTrafficPolicy::from(policy),
            (None, true) => ClientTrafficPolicy::DisableAllTraffic,
            (None, false) => ClientTrafficPolicy::None,
        }
    }
}

impl From<i32> for ClientTrafficPolicy {
    fn from(value: i32) -> Self {
        match value {
            1 => ClientTrafficPolicy::DisableAllTraffic,
            2 => ClientTrafficPolicy::ForceAllTraffic,
            _ => ClientTrafficPolicy::None,
        }
    }
}

impl From<Option<i32>> for ClientTrafficPolicy {
    fn from(value: Option<i32>) -> Self {
        match value {
            None => Self::None,
            Some(v) => Self::from(v),
        }
    }
}

impl From<i64> for ClientTrafficPolicy {
    fn from(value: i64) -> Self {
        Self::from(value as i32)
    }
}
