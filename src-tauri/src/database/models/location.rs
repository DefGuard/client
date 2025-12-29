use std::fmt;
#[cfg(not(target_os = "macos"))]
use std::str::FromStr;

#[cfg(not(target_os = "macos"))]
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::Type, query, query_as, query_scalar, Error as SqlxError, SqliteExecutor};

#[cfg(not(target_os = "macos"))]
use super::wireguard_keys::WireguardKeys;
use super::{Id, NoId};
#[cfg(not(target_os = "macos"))]
use crate::utils::{DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6};
use crate::{
    error::Error,
    proto::{
        LocationMfaMode as ProtoLocationMfaMode, ServiceLocationMode as ProtoServiceLocationMode,
    },
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[repr(u32)]
#[serde(rename_all = "lowercase")]
pub enum LocationMfaMode {
    Disabled = 1,
    Internal = 2,
    External = 3,
}

impl From<ProtoLocationMfaMode> for LocationMfaMode {
    fn from(value: ProtoLocationMfaMode) -> Self {
        match value {
            ProtoLocationMfaMode::Unspecified | ProtoLocationMfaMode::Disabled => {
                LocationMfaMode::Disabled
            }
            ProtoLocationMfaMode::Internal => LocationMfaMode::Internal,
            ProtoLocationMfaMode::External => LocationMfaMode::External,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[repr(u32)]
#[serde(rename_all = "lowercase")]
pub enum ServiceLocationMode {
    Disabled = 1,
    PreLogon = 2,
    AlwaysOn = 3,
}

impl From<ProtoServiceLocationMode> for ServiceLocationMode {
    fn from(value: ProtoServiceLocationMode) -> Self {
        match value {
            ProtoServiceLocationMode::Unspecified | ProtoServiceLocationMode::Disabled => {
                ServiceLocationMode::Disabled
            }
            ProtoServiceLocationMode::Prelogon => ServiceLocationMode::PreLogon,
            ProtoServiceLocationMode::Alwayson => ServiceLocationMode::AlwaysOn,
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
    pub pubkey: String, // Remote
    pub endpoint: String,
    pub allowed_ips: String,
    pub dns: Option<String>,
    pub route_all_traffic: bool,
    pub keepalive_interval: i64,
    pub location_mfa_mode: LocationMfaMode,
    pub service_location_mode: ServiceLocationMode,
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
    /// Ignores service locations
    #[cfg(any(windows, target_os = "macos"))]
    pub(crate) async fn all<'e, E>(
        executor: E,
        include_service_locations: bool,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        let max_service_location_mode =
            Self::get_service_location_mode_filter(include_service_locations);
        query_as!(
          Self,
            "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, network_id,\
            route_all_traffic, keepalive_interval, \
            location_mfa_mode \"location_mfa_mode: LocationMfaMode\", service_location_mode \"service_location_mode: ServiceLocationMode\" \
            FROM location WHERE service_location_mode <= $1 \
            ORDER BY name ASC;",
            max_service_location_mode
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
            keepalive_interval = $10, location_mfa_mode = $11, service_location_mode = $12 WHERE id = $13",
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
            self.location_mfa_mode,
            self.service_location_mode,
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
            network_id, route_all_traffic,  keepalive_interval, \
            location_mfa_mode \"location_mfa_mode: LocationMfaMode\", service_location_mode \"service_location_mode: ServiceLocationMode\" \
            FROM location WHERE id = $1",
            location_id
        )
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn find_by_instance_id<'e, E>(
        executor: E,
        instance_id: Id,
        include_service_locations: bool,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        let max_service_location_mode =
            Self::get_service_location_mode_filter(include_service_locations);
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, keepalive_interval, location_mfa_mode \"location_mfa_mode: LocationMfaMode\", service_location_mode \"service_location_mode: ServiceLocationMode\" \
            FROM location WHERE instance_id = $1 AND service_location_mode <= $2 \
            ORDER BY name ASC",
            instance_id,
            max_service_location_mode
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
            network_id, route_all_traffic, keepalive_interval, location_mfa_mode \"location_mfa_mode: LocationMfaMode\", service_location_mode \"service_location_mode: ServiceLocationMode\" \
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
        match self.location_mfa_mode {
            LocationMfaMode::Disabled => false,
            LocationMfaMode::Internal | LocationMfaMode::External => true,
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) async fn interface_configuration<'e, E>(
        &self,
        executor: E,
        interface_name: String,
        preshared_key: Option<String>,
        mtu: Option<u32>,
    ) -> Result<InterfaceConfiguration, Error>
    where
        E: SqliteExecutor<'e>,
    {
        debug!("Looking for WireGuard keys for location {self} instance");
        let Some(keys) = WireguardKeys::find_by_instance_id(executor, self.instance_id).await?
        else {
            error!("No keys found for instance: {}", self.instance_id);
            return Err(Error::InternalError(
                "No keys found for instance".to_string(),
            ));
        };
        debug!("WireGuard keys found for location {self} instance");

        // prepare peer config
        debug!("Decoding location {self} public key: {}.", self.pubkey);
        let peer_key = Key::from_str(&self.pubkey)?;
        debug!("Location {self} public key decoded: {peer_key}");
        let mut peer = Peer::new(peer_key);

        debug!("Parsing location {self} endpoint: {}", self.endpoint);
        peer.set_endpoint(&self.endpoint)?;
        peer.persistent_keepalive_interval = Some(25);
        debug!("Parsed location {self} endpoint: {}", self.endpoint);

        if let Some(psk) = preshared_key {
            debug!("Decoding location {self} preshared key.");
            let peer_psk = Key::from_str(&psk)?;
            info!("Location {self} preshared key decoded.");
            peer.preshared_key = Some(peer_psk);
        }

        debug!("Parsing location {self} allowed IPs: {}", self.allowed_ips);
        let allowed_ips = if self.route_all_traffic {
            debug!("Using all traffic routing for location {self}");
            vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
        } else {
            debug!(
                "Using predefined location {self} traffic: {}",
                self.allowed_ips
            );
            self.allowed_ips.split(',').map(str::to_string).collect()
        };
        for allowed_ip in &allowed_ips {
            match IpAddrMask::from_str(allowed_ip) {
                Ok(addr) => {
                    peer.allowed_ips.push(addr);
                }
                Err(err) => {
                    // Handle the error from IpAddrMask::from_str, if needed
                    error!(
                        "Error parsing IP address {allowed_ip} while setting up interface for \
                        location {self}, error details: {err}"
                    );
                }
            }
        }
        debug!(
            "Parsed allowed IPs for location {self}: {:?}",
            peer.allowed_ips
        );

        let addresses = self
            .address
            .split(',')
            .map(str::trim)
            .map(IpAddrMask::from_str)
            .collect::<Result<_, _>>()
            .map_err(|err| {
                let msg = format!("Failed to parse IP addresses '{}': {err}", self.address);
                error!("{msg}");
                Error::InternalError(msg)
            })?;
        let interface_config = InterfaceConfiguration {
            name: interface_name,
            prvkey: keys.prvkey,
            addresses,
            port: 0,
            peers: vec![peer],
            mtu,
        };

        Ok(interface_config)
    }

    /// Returns a filter value that can be used in SQL queries like `service_location_mode <= ?` when querying locations
    /// to exclude (<= 1) or include service locations (all service locations modes).
    fn get_service_location_mode_filter(include_service_locations: bool) -> i32 {
        if include_service_locations {
            i32::MAX
        } else {
            ServiceLocationMode::Disabled as i32
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
            dns, network_id, route_all_traffic, keepalive_interval, location_mfa_mode, service_location_mode) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
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
            self.location_mfa_mode,
            self.service_location_mode,
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
            location_mfa_mode: self.location_mfa_mode,
            service_location_mode: self.service_location_mode,
        })
    }
}

impl<I> Location<I> {
    pub fn is_service_location(&self) -> bool {
        self.service_location_mode != ServiceLocationMode::Disabled
            && self.location_mfa_mode == LocationMfaMode::Disabled
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
            location_mfa_mode: location.location_mfa_mode,
            service_location_mode: location.service_location_mode,
        }
    }
}
