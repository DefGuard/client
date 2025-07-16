use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{prelude::Type, query, query_as, query_scalar, Error as SqlxError, SqliteExecutor};

use super::{Id, NoId};
use crate::{error::Error, proto::LocationMfa as ProtoLocationMfa};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[repr(u32)]
pub enum LocationMfaType {
    Disabled = 1,
    Internal = 2,
    External = 3,
}

impl From<ProtoLocationMfa> for LocationMfaType {
    fn from(value: ProtoLocationMfa) -> Self {
        match value {
            ProtoLocationMfa::Unspecified | ProtoLocationMfa::Disabled => LocationMfaType::Disabled,
            ProtoLocationMfa::Internal => LocationMfaType::Internal,
            ProtoLocationMfa::External => LocationMfaType::External,
        }
    }
}

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
    pub keepalive_interval: i64,
    pub location_mfa: LocationMfaType,
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
    #[cfg(windows)]
    pub(crate) async fn all<'e, E>(executor: E) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
          Self,
          "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id,\
          route_all_traffic, keepalive_interval, location_mfa \"location_mfa: LocationMfaType\" \
          FROM location;"
      )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn save<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        // Update the existing record when there is an ID
        query!(
            "UPDATE location SET instance_id = $1, name = $2, address = $3, pubkey = $4, \
            endpoint = $5, allowed_ips = $6, dns = $7, network_id = $8, route_all_traffic = $9, \
            keepalive_interval = $10, location_mfa = $11 WHERE id = $12",
            self.instance_id,
            self.name,
            self.address,
            self.pubkey,
            self.endpoint,
            self.allowed_ips,
            self.dns,
            self.network_id,
            self.route_all_traffic,
            self.keepalive_interval,
            self.location_mfa,
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub(crate) async fn find_by_id<'e, E>(
        executor: E,
        location_id: Id,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic,  keepalive_interval, location_mfa \"location_mfa: LocationMfaType\" \
            FROM location WHERE id = $1",
            location_id
        )
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn find_by_instance_id<'e, E>(
        executor: E,
        instance_id: Id,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, keepalive_interval, location_mfa \"location_mfa: LocationMfaType\" \
            FROM location WHERE instance_id = $1",
            instance_id
        )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn find_by_public_key<'e, E>(
        executor: E,
        pubkey: &str,
    ) -> Result<Self, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, keepalive_interval, location_mfa \"location_mfa: LocationMfaType\" \
            FROM location WHERE pubkey = $1;",
            pubkey
        )
        .fetch_one(executor)
        .await
    }

    pub(crate) async fn delete<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query!("DELETE FROM location WHERE id = $1;", self.id)
            .execute(executor)
            .await?;
        Ok(())
    }

    /// Disables all traffic for locations related to the given instance
    pub(crate) async fn disable_all_traffic_for_all<'e, E>(
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

    pub(crate) fn mfa_enabled(&self) -> bool {
        match self.location_mfa {
            LocationMfaType::Disabled => false,
            LocationMfaType::Internal | LocationMfaType::External => true,
        }
    }
}

impl Location<NoId> {
    pub(crate) async fn save<'e, E>(self, executor: E) -> Result<Location<Id>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        // Insert a new record when there is no ID
        let id = query_scalar!(
            "INSERT INTO location (instance_id, name, address, pubkey, endpoint, allowed_ips, \
            dns, network_id, route_all_traffic, keepalive_interval, location_mfa) \
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
            self.keepalive_interval,
            self.location_mfa
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
            keepalive_interval: self.keepalive_interval,
            location_mfa: self.location_mfa,
        })
    }
}

impl From<Location<Id>> for Location {
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
            keepalive_interval: location.keepalive_interval,
            location_mfa: location.location_mfa,
        }
    }
}
