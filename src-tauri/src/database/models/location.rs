use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, query_scalar, Error as SqlxError, SqliteExecutor};

use super::{Id, NoId};
use crate::error::Error;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Location<I = NoId> {
    pub id: I,
    pub instance_id: Id,
    // Native ID of network from Defguard
    pub network_id: Id,
    pub name: String,
    pub address: String,
    pub pubkey: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub dns: Option<String>,
    pub route_all_traffic: bool,
    pub mfa_enabled: bool,
    pub keepalive_interval: i64,
}

impl fmt::Display for Location<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(ID: {})", self.name, self.id)
    }
}

impl fmt::Display for Location<NoId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Location<Id> {
    pub async fn all<'e, E>(executor: E) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id,\
            route_all_traffic, mfa_enabled, keepalive_interval \
            FROM location;"
        )
        .fetch_all(executor)
        .await
    }

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        // Update the existing record when there is an ID
        query!(
            "UPDATE location SET instance_id = $1, name = $2, address = $3, pubkey = $4, endpoint = $5, allowed_ips = $6, dns = $7, \
            network_id = $8, route_all_traffic = $9, mfa_enabled = $10, keepalive_interval = $11 WHERE id = $12",
            self.instance_id,
            self.name,
            self.address,
            self.pubkey,
            self.endpoint,
            self.allowed_ips,
            self.dns,
            self.network_id,
            self.route_all_traffic,
            self.mfa_enabled,
            self.keepalive_interval,
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn find_by_id<'e, E>(executor: E, location_id: Id) -> Result<Option<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, \
            route_all_traffic, mfa_enabled, keepalive_interval FROM location WHERE id = $1",
            location_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_by_instance_id<'e, E>(
        executor: E,
        instance_id: Id,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, \
            route_all_traffic, mfa_enabled, keepalive_interval FROM location WHERE instance_id = $1",
            instance_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn find_by_name<'e, E>(executor: E, name: &str) -> Result<Self, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, \
            route_all_traffic, mfa_enabled, keepalive_interval FROM location WHERE name = $1",
            name
        )
        .fetch_one(executor)
        .await
    }

    pub async fn find_by_public_key<'e, E>(executor: E, pubkey: &str) -> Result<Self, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id, \
            route_all_traffic, mfa_enabled, keepalive_interval FROM location WHERE pubkey = $1;",
            pubkey
        )
        .fetch_one(executor)
        .await
    }

    pub async fn delete<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query!("DELETE FROM location WHERE id = $1;", self.id)
            .execute(executor)
            .await?;
        Ok(())
    }

    /// Disables all traffic for locations related to the given instance
    pub async fn disable_all_traffic_for_all<'e, E>(
        executor: E,
        instance_id: Id,
    ) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        query!(
            "UPDATE location SET route_all_traffic = 0 WHERE instance_id = $1;",
            instance_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl Location<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> Result<Location<Id>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        // Insert a new record when there is no ID
        let id = query_scalar!(
            "INSERT INTO location (instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, mfa_enabled, keepalive_interval) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
            RETURNING id \"id!\"",
            self.instance_id,
            self.name,
            self.address,
            self.pubkey,
            self.endpoint,
            self.allowed_ips,
            self.dns,
            self.network_id,
            self.route_all_traffic,
            self.mfa_enabled,
            self.keepalive_interval
        )
        .fetch_one(executor)
        .await?;

        Ok(Location::<Id> {
            id,
            instance_id: self.instance_id,
            name: self.name,
            address: self.address,
            pubkey: self.pubkey,
            endpoint: self.endpoint,
            allowed_ips: self.allowed_ips,
            dns: self.dns,
            network_id: self.network_id,
            route_all_traffic: self.route_all_traffic,
            mfa_enabled: self.mfa_enabled,
            keepalive_interval: self.keepalive_interval,
        })
    }
}

impl From<Location<Id>> for Location<NoId> {
    fn from(location: Location<Id>) -> Self {
        Self {
            id: NoId,
            instance_id: location.instance_id,
            network_id: location.network_id,
            name: location.name,
            address: location.address,
            pubkey: location.pubkey,
            endpoint: location.endpoint,
            allowed_ips: location.allowed_ips,
            dns: location.dns,
            route_all_traffic: location.route_all_traffic,
            mfa_enabled: location.mfa_enabled,
            keepalive_interval: location.keepalive_interval,
        }
    }
}
