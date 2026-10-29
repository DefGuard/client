#[cfg(not(target_os = "macos"))]
use std::str::FromStr;
use std::{collections::HashMap, fmt};

use defguard_wireguard_rs::net::IpAddrMask;
#[cfg(not(target_os = "macos"))]
use defguard_wireguard_rs::{key::Key, peer::Peer, InterfaceConfiguration};
use serde::{Deserialize, Serialize};
use sqlx::{
    prelude::Type, query, query_as, query_scalar, types::Json, SqliteConnection, SqliteExecutor,
};

#[cfg(not(target_os = "macos"))]
use super::wireguard_keys::WireguardKeys;
use super::{Id, NoId};
use crate::{
    database::{
        models::instance::{ClientTrafficPolicy, Instance},
        DbPool,
    },
    error::Error,
    proto::client_types::{
        LocationMfaMode as ProtoLocationMfaMode, MfaMethod as ProtoMfaMethod,
        MfaStep as ProtoMfaStep, ServiceLocationMode as ProtoServiceLocationMode,
    },
};
#[cfg(not(target_os = "macos"))]
use crate::{DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, Type)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, Type)]
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

/// Discriminants match the proto `MfaMethod` enum, except for `Fido2`, which the
/// protocol does not carry yet and which is therefore handled entirely on the
/// client.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Type,
)]
#[repr(u32)]
#[serde(rename_all = "lowercase")]
pub enum LocationMfaMethod {
    Totp = 0,
    Email = 1,
    Oidc = 2,
    Biometric = 3,
    MobileApprove = 4,
    Fido2 = 5,
}

impl LocationMfaMethod {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Totp => "totp",
            Self::Email => "email",
            Self::Oidc => "oidc",
            Self::Biometric => "biometric",
            Self::MobileApprove => "mobile",
            Self::Fido2 => "fido2",
        }
    }
}

impl From<ProtoMfaMethod> for LocationMfaMethod {
    fn from(value: ProtoMfaMethod) -> Self {
        match value {
            ProtoMfaMethod::Totp => Self::Totp,
            ProtoMfaMethod::Email => Self::Email,
            ProtoMfaMethod::Oidc => Self::Oidc,
            ProtoMfaMethod::Biometric => Self::Biometric,
            ProtoMfaMethod::MobileApprove => Self::MobileApprove,
            ProtoMfaMethod::Fido2 => Self::Fido2,
        }
    }
}

impl From<LocationMfaMethod> for ProtoMfaMethod {
    fn from(value: LocationMfaMethod) -> Self {
        match value {
            LocationMfaMethod::Totp => Self::Totp,
            LocationMfaMethod::Email => Self::Email,
            LocationMfaMethod::Oidc => Self::Oidc,
            LocationMfaMethod::Biometric => Self::Biometric,
            LocationMfaMethod::MobileApprove => Self::MobileApprove,
            LocationMfaMethod::Fido2 => Self::Fido2,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub struct LocationMfaStepMethod {
    pub method: LocationMfaMethod,
    pub configured: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub struct LocationMfaStep {
    pub methods: Vec<LocationMfaStepMethod>,
}

impl From<ProtoMfaStep> for LocationMfaStep {
    fn from(value: ProtoMfaStep) -> Self {
        Self {
            methods: value
                .methods
                .iter()
                .map(|entry| LocationMfaStepMethod {
                    method: entry.method().into(),
                    configured: entry.configured,
                })
                .collect(),
        }
    }
}

#[must_use]
pub fn legacy_mfa_steps(mode: LocationMfaMode) -> Vec<LocationMfaStep> {
    let methods = match mode {
        LocationMfaMode::Disabled => return Vec::new(),
        LocationMfaMode::Internal => vec![
            LocationMfaMethod::Totp,
            LocationMfaMethod::Email,
            LocationMfaMethod::MobileApprove,
        ],
        LocationMfaMode::External => vec![LocationMfaMethod::Oidc],
    };
    vec![LocationMfaStep {
        methods: methods
            .into_iter()
            .map(|method| LocationMfaStepMethod {
                method,
                configured: true,
            })
            .collect(),
    }]
}

#[must_use]
pub fn infer_mfa_method(
    mode: LocationMfaMode,
    method: Option<LocationMfaMethod>,
) -> Option<LocationMfaMethod> {
    match mode {
        LocationMfaMode::Disabled => method,
        LocationMfaMode::Internal => match method {
            Some(LocationMfaMethod::Oidc) | None => Some(LocationMfaMethod::Totp),
            Some(m) => Some(m),
        },
        LocationMfaMode::External => Some(LocationMfaMethod::Oidc),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
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
    pub mfa_method: Option<LocationMfaMethod>,
    #[serde(default)]
    pub posture_check_required: bool,
    #[serde(default)]
    pub mfa_steps: Json<Vec<LocationMfaStep>>,
    #[serde(default)]
    pub mfa_step_plan: Json<Vec<LocationMfaMethod>>,
    /// MTU suggested by Defguard Core for this location. Used only when MTU not set in the
    /// application settings.
    #[serde(default)]
    pub client_mtu: Option<u32>,
    #[serde(default)]
    pub auto_connect: bool,
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
    pub async fn all<'e, E>(executor: E, include_service_locations: bool) -> sqlx::Result<Vec<Self>>
    where
        E: SqliteExecutor<'e>,
    {
        let max_service_location_mode =
            Self::get_service_location_mode_filter(include_service_locations);
        query_as!(
            Self,
            "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, keepalive_interval, \
            location_mfa_mode \"location_mfa_mode: LocationMfaMode\", \
            service_location_mode \"service_location_mode: ServiceLocationMode\", \
            mfa_method \"mfa_method: _\", posture_check_required, mfa_steps \"mfa_steps: _\", \
            mfa_step_plan \"mfa_step_plan: _\", client_mtu \"client_mtu: _\", auto_connect \
            FROM location WHERE service_location_mode <= $1 \
            ORDER BY name ASC",
            max_service_location_mode
        )
        .fetch_all(executor)
        .await
    }

    pub async fn exist<'e, E>(executor: E, include_service_locations: bool) -> sqlx::Result<bool>
    where
        E: SqliteExecutor<'e>,
    {
        let max_service_location_mode =
            Self::get_service_location_mode_filter(include_service_locations);
        let result = query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM location WHERE service_location_mode <= $1)",
            max_service_location_mode
        )
        .fetch_one(executor)
        .await?;

        Ok(result != 0)
    }

    /// Find locations by name (excluding service locations).
    /// Returns all matches - callers must handle cross-instance ambiguity.
    pub async fn find_by_name<'e, E>(executor: E, name: &str) -> sqlx::Result<Vec<Self>>
    where
        E: SqliteExecutor<'e>,
    {
        let max = Self::get_service_location_mode_filter(false);
        query_as!(
            Self,
            "SELECT id, instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, keepalive_interval, \
            location_mfa_mode \"location_mfa_mode: LocationMfaMode\", \
            service_location_mode \"service_location_mode: ServiceLocationMode\", \
            mfa_method \"mfa_method: _\", posture_check_required, mfa_steps \"mfa_steps: _\", \
            mfa_step_plan \"mfa_step_plan: _\", client_mtu \"client_mtu: _\", auto_connect \
            FROM location WHERE name = $1 AND service_location_mode <= $2 ORDER BY name ASC",
            name,
            max,
        )
        .fetch_all(executor)
        .await
    }

    pub async fn save<'e, E>(&mut self, executor: E) -> sqlx::Result<()>
    where
        E: SqliteExecutor<'e>,
    {
        // Update the existing record when there is an ID
        query!(
            "UPDATE location SET instance_id = $1, name = $2, address = $3, pubkey = $4, \
            endpoint = $5, allowed_ips = $6, dns = $7, network_id = $8, route_all_traffic = $9, \
            keepalive_interval = $10, location_mfa_mode = $11, service_location_mode = $12, \
            mfa_method = $13, posture_check_required = $14, mfa_steps = $15, \
            mfa_step_plan = $16, client_mtu = $17, auto_connect = $18 \
            WHERE id = $19",
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
            self.mfa_method,
            self.posture_check_required,
            self.mfa_steps,
            self.mfa_step_plan,
            self.client_mtu,
            self.auto_connect,
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn find_by_id<'e, E>(executor: E, location_id: Id) -> sqlx::Result<Option<Self>>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic,  keepalive_interval, \
            location_mfa_mode \"location_mfa_mode: LocationMfaMode\", \
            service_location_mode \"service_location_mode: ServiceLocationMode\",
            mfa_method \"mfa_method: _\", posture_check_required, mfa_steps \"mfa_steps: _\", \
            mfa_step_plan \"mfa_step_plan: _\", client_mtu \"client_mtu: _\", auto_connect \
            FROM location WHERE id = $1",
            location_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_by_instance_id<'e, E>(
        executor: E,
        instance_id: Id,
        include_service_locations: bool,
    ) -> sqlx::Result<Vec<Self>>
    where
        E: SqliteExecutor<'e>,
    {
        let max_service_location_mode =
            Self::get_service_location_mode_filter(include_service_locations);
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, keepalive_interval, \
            location_mfa_mode \"location_mfa_mode: LocationMfaMode\", \
            service_location_mode \"service_location_mode: ServiceLocationMode\",
            mfa_method \"mfa_method: _\", posture_check_required, mfa_steps \"mfa_steps: _\", \
            mfa_step_plan \"mfa_step_plan: _\", client_mtu \"client_mtu: _\", auto_connect \
            FROM location WHERE instance_id = $1 AND service_location_mode <= $2 \
            ORDER BY name ASC",
            instance_id,
            max_service_location_mode
        )
        .fetch_all(executor)
        .await
    }

    pub async fn find_by_public_key<'e, E>(executor: E, pubkey: &str) -> sqlx::Result<Self>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", instance_id, name, address, pubkey, endpoint, allowed_ips, dns, \
            network_id, route_all_traffic, keepalive_interval, \
            location_mfa_mode \"location_mfa_mode: LocationMfaMode\", \
            service_location_mode \"service_location_mode: ServiceLocationMode\",
            mfa_method \"mfa_method: _\", posture_check_required, mfa_steps \"mfa_steps: _\", \
            mfa_step_plan \"mfa_step_plan: _\", client_mtu \"client_mtu: _\", auto_connect \
            FROM location WHERE pubkey = $1",
            pubkey
        )
        .fetch_one(executor)
        .await
    }

    pub async fn delete<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: SqliteExecutor<'e>,
    {
        query!("DELETE FROM location WHERE id = $1", self.id)
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
            "UPDATE location SET route_all_traffic = 0 WHERE instance_id = $1",
            instance_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    #[must_use]
    pub fn mfa_enabled(&self) -> bool {
        match self.location_mfa_mode {
            LocationMfaMode::Disabled => false,
            LocationMfaMode::Internal | LocationMfaMode::External => true,
        }
    }

    pub async fn effective_route_all_traffic(
        &self,
        pool: &DbPool,
        route_all_traffic: Option<bool>,
    ) -> Result<bool, Error> {
        let Some(instance) = Instance::find_by_id(pool, self.instance_id).await? else {
            error!("Instance {} not found", self.instance_id);
            return Err(Error::InternalError(format!(
                "Instance {} not found",
                self.instance_id
            )));
        };
        Ok(match instance.client_traffic_policy {
            ClientTrafficPolicy::ForceAllTraffic => true,
            ClientTrafficPolicy::DisableAllTraffic => false,
            ClientTrafficPolicy::None => route_all_traffic.unwrap_or(self.route_all_traffic),
        })
    }

    pub async fn routed_networks(
        &self,
        pool: &DbPool,
        route_all_traffic: Option<bool>,
    ) -> Result<Vec<IpAddrMask>, Error> {
        Ok(crate::routed_networks(
            &self.address,
            &self.allowed_ips,
            self.effective_route_all_traffic(pool, route_all_traffic)
                .await?,
        ))
    }

    #[cfg(not(target_os = "macos"))]
    pub async fn interface_configuration(
        &self,
        pool: &DbPool,
        interface_name: String,
        preshared_key: Option<String>,
        mtu: Option<u32>,
        route_all_traffic: Option<bool>,
    ) -> Result<InterfaceConfiguration, Error> {
        debug!("Looking for WireGuard keys for location {self} instance");
        let Some(keys) = WireguardKeys::find_by_instance_id(pool, self.instance_id).await? else {
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
        let route_all_traffic = self
            .effective_route_all_traffic(pool, route_all_traffic)
            .await?;
        let allowed_ips = if route_all_traffic {
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
            mtu: self.effective_mtu(mtu),
            fwmark: None, // TODO: add
        };

        Ok(interface_config)
    }

    /// Persist a per-location MFA method override, clamped via [`infer_mfa_method`]
    /// against the location's MFA mode (`location_mfa_mode`).
    pub async fn set_mfa_method(
        pool: &DbPool,
        location_id: Id,
        method: LocationMfaMethod,
    ) -> Result<(), Error> {
        let mut location = Self::find_by_id(pool, location_id)
            .await?
            .ok_or(Error::NotFound)?;
        let inferred = infer_mfa_method(location.location_mfa_mode, Some(method));
        location.mfa_method = inferred;
        location.save(pool).await?;
        Ok(())
    }

    pub async fn set_mfa_step_plan(
        pool: &DbPool,
        location_id: Id,
        plan: Vec<LocationMfaMethod>,
    ) -> Result<(), Error> {
        let mut location = Self::find_by_id(pool, location_id)
            .await?
            .ok_or(Error::NotFound)?;
        location.mfa_step_plan = Json(plan);
        location.save(pool).await?;
        Ok(())
    }

    pub async fn set_auto_connect(
        pool: &DbPool,
        location_id: Id,
        auto_connect: bool,
    ) -> Result<(), Error> {
        // immediate lock so two concurrent requests can't both pass the all traffic check
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut location = Self::find_by_id(&mut *transaction, location_id)
            .await?
            .ok_or(Error::NotFound)?;
        if auto_connect {
            let context = AutoConnectContext::load(&mut transaction).await?;
            if let Some(block) = context.block(&location) {
                return Err(Error::InvalidInput(block.reason().into()));
            }
        }
        location.auto_connect = auto_connect;
        location.save(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Persist the route-all-traffic flag for a location, rejecting the update if
    /// the owning instance's [`ClientTrafficPolicy`] forbids the requested value.
    /// Returns whether auto-connect was turned off to avoid an all traffic collision.
    pub async fn update_routing(
        pool: &DbPool,
        location_id: Id,
        route_all_traffic: bool,
    ) -> Result<bool, Error> {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut location = Self::find_by_id(&mut *transaction, location_id)
            .await?
            .ok_or(Error::NotFound)?;

        let instance = Instance::find_by_id(&mut *transaction, location.instance_id)
            .await?
            .ok_or(Error::NotFound)?;

        if instance.client_traffic_policy == ClientTrafficPolicy::DisableAllTraffic
            && route_all_traffic
        {
            return Err(Error::InvalidInput(
                "Instance has route_all_traffic disabled.".into(),
            ));
        }
        if instance.client_traffic_policy == ClientTrafficPolicy::ForceAllTraffic
            && !route_all_traffic
        {
            return Err(Error::InvalidInput(
                "Instance has route_all_traffic enforced.".into(),
            ));
        }

        location.route_all_traffic = route_all_traffic;
        let auto_connect_disabled = location.auto_connect
            && AutoConnectContext::load(&mut transaction)
                .await?
                .block(&location)
                .is_some();
        if auto_connect_disabled {
            info!(
                "Location {location} now routes all traffic while another all traffic location \
                is set to auto-connect, disabling its auto-connect"
            );
            location.auto_connect = false;
        }
        location.save(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(auto_connect_disabled)
    }

    /// Returns a filter value that can be used in SQL queries like `service_location_mode <= ?`
    /// when querying locations to exclude (<= 1) or include service locations (all service
    /// locations modes).
    fn get_service_location_mode_filter(include_service_locations: bool) -> i32 {
        if include_service_locations {
            i32::MAX
        } else {
            ServiceLocationMode::Disabled as i32
        }
    }
}

impl Location<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> sqlx::Result<Location<Id>>
    where
        E: SqliteExecutor<'e>,
    {
        // Insert a new record when there is no ID
        let id = query_scalar!(
            "INSERT INTO location (instance_id, name, address, pubkey, endpoint, allowed_ips, \
            dns, network_id, route_all_traffic, keepalive_interval, location_mfa_mode, \
            service_location_mode, mfa_method, posture_check_required, mfa_steps, \
            mfa_step_plan, client_mtu, auto_connect) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
            $18) \
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
            self.mfa_method,
            self.posture_check_required,
            self.mfa_steps,
            self.mfa_step_plan,
            self.client_mtu,
            self.auto_connect,
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
            mfa_method: self.mfa_method,
            posture_check_required: self.posture_check_required,
            mfa_steps: self.mfa_steps,
            mfa_step_plan: self.mfa_step_plan,
            client_mtu: self.client_mtu,
            auto_connect: self.auto_connect,
        })
    }
}

impl<I> Location<I> {
    /// Resolve the MTU to set on this location's interface.
    #[must_use]
    pub fn effective_mtu(&self, mtu: Option<u32>) -> Option<u32> {
        mtu.or(self.client_mtu)
    }

    pub fn is_service_location(&self) -> bool {
        self.service_location_mode != ServiceLocationMode::Disabled
            && self.location_mfa_mode == LocationMfaMode::Disabled
    }

    /// Service locations ignore `route_all_traffic`, the daemon routes their `allowed_ips` as sent.
    pub fn routes_all_traffic(&self, policy: &ClientTrafficPolicy) -> bool {
        if self.is_service_location() {
            return self
                .allowed_ips
                .split(',')
                .filter_map(|network| network.trim().parse::<IpAddrMask>().ok())
                .any(|network| network.cidr == 0);
        }
        match policy {
            ClientTrafficPolicy::ForceAllTraffic => true,
            ClientTrafficPolicy::DisableAllTraffic => false,
            ClientTrafficPolicy::None => self.route_all_traffic,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoConnectBlock {
    Mfa,
    ServiceLocation,
    AllTrafficTaken,
}

impl AutoConnectBlock {
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::Mfa => "Locations with MFA can't auto-connect.",
            Self::ServiceLocation => "Service locations can't auto-connect.",
            Self::AllTrafficTaken => {
                "Another all traffic location is already set to auto-connect or managed by the \
                service."
            }
        }
    }
}

/// Snapshot of all instances' locations, since all traffic routes collide machine-wide.
pub struct AutoConnectContext {
    policies: HashMap<Id, ClientTrafficPolicy>,
    locations: Vec<Location<Id>>,
}

impl AutoConnectContext {
    pub async fn load(connection: &mut SqliteConnection) -> Result<Self, Error> {
        let policies = Instance::all(&mut *connection)
            .await?
            .into_iter()
            .map(|instance| (instance.id, instance.client_traffic_policy))
            .collect();
        let locations = Location::all(&mut *connection, true).await?;
        Ok(Self {
            policies,
            locations,
        })
    }

    #[must_use]
    pub fn locations(&self) -> &[Location<Id>] {
        &self.locations
    }

    #[must_use]
    pub fn routes_all_traffic(&self, location: &Location<Id>) -> bool {
        self.policies
            .get(&location.instance_id)
            .is_some_and(|policy| location.routes_all_traffic(policy))
    }

    /// Checks `location` as given, so callers can pass unsaved changes.
    #[must_use]
    pub fn block(&self, location: &Location<Id>) -> Option<AutoConnectBlock> {
        if location.mfa_enabled() {
            return Some(AutoConnectBlock::Mfa);
        }
        if location.service_location_mode != ServiceLocationMode::Disabled {
            return Some(AutoConnectBlock::ServiceLocation);
        }
        let taken = self.routes_all_traffic(location)
            && self.locations.iter().any(|other| {
                other.id != location.id
                    && (other.auto_connect || other.is_service_location())
                    && self.routes_all_traffic(other)
            });
        taken.then_some(AutoConnectBlock::AllTrafficTaken)
    }

    /// Locations set to auto-connect that can't, which must never stay saved.
    pub fn invalid(&self) -> impl Iterator<Item = (&Location<Id>, AutoConnectBlock)> {
        self.locations
            .iter()
            .filter(|location| location.auto_connect)
            .filter_map(|location| self.block(location).map(|block| (location, block)))
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
            mfa_method: location.mfa_method,
            posture_check_required: location.posture_check_required,
            mfa_steps: location.mfa_steps,
            mfa_step_plan: location.mfa_step_plan,
            client_mtu: location.client_mtu,
            auto_connect: location.auto_connect,
        }
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use super::*;
    use crate::{
        database::models::instance::{ClientTrafficPolicy, Instance, OpenIdProviderKind},
        mfa_contract::MfaContract,
    };

    fn new_instance() -> Instance<NoId> {
        Instance {
            id: NoId,
            name: "instance".into(),
            uuid: "uuid-1".into(),
            url: "https://core.example".into(),
            proxy_url: "https://proxy.example".into(),
            username: "alice".into(),
            token: None,
            client_traffic_policy: ClientTrafficPolicy::None,
            enterprise_enabled: false,
            disable_tunnels: false,
            openid_display_name: None,
            openid_provider_kind: OpenIdProviderKind::Custom,
            mfa_configured_methods: None,
            mfa_contract: MfaContract::Legacy,
            mfa_capabilities: None,
            smtp_configured: None,
            openid_available: None,
        }
    }

    fn new_location(instance_id: Id) -> Location<NoId> {
        Location {
            id: NoId,
            instance_id,
            network_id: 1,
            name: "loc".into(),
            address: "10.0.0.2/24".into(),
            pubkey: "pk".into(),
            endpoint: "1.2.3.4:51820".into(),
            allowed_ips: "0.0.0.0/0".into(),
            dns: None,
            route_all_traffic: false,
            keepalive_interval: 25,
            location_mfa_mode: LocationMfaMode::Disabled,
            service_location_mode: ServiceLocationMode::Disabled,
            mfa_method: None,
            posture_check_required: false,
            mfa_steps: Default::default(),
            mfa_step_plan: Default::default(),
            client_mtu: None,
            auto_connect: false,
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_location_crud_round_trip(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let location = new_location(instance.id).save(&pool).await.unwrap();

        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert_eq!(found.name, "loc");
        assert_eq!(found.instance_id, instance.id);

        location.delete(&pool).await.unwrap();
        assert!(Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_mfa_method_serde_matches_frontend_contract() {
        // The frontend compares these strings, and `as_str` says "mobile" for the same variant.
        assert_eq!(
            serde_json::to_string(&LocationMfaMethod::MobileApprove).unwrap(),
            "\"mobileapprove\""
        );
        assert_eq!(
            serde_json::to_string(&LocationMfaMethod::Fido2).unwrap(),
            "\"fido2\""
        );
    }

    #[test]
    fn test_effective_mtu_precedence() {
        let mut location = new_location(1);

        // Nothing configured anywhere: let the system pick.
        assert_eq!(location.effective_mtu(None), None);

        // Core-provided MTU is used when settings leave it unset.
        location.client_mtu = Some(1380);
        assert_eq!(location.effective_mtu(None), Some(1380));

        // An MTU pinned in the settings overrides the core-provided one.
        assert_eq!(location.effective_mtu(Some(1420)), Some(1420));

        // ...and is used even when the core sent nothing.
        location.client_mtu = None;
        assert_eq!(location.effective_mtu(Some(1420)), Some(1420));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_client_mtu_round_trip(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.client_mtu = Some(1380);
        let mut location = location.save(&pool).await.unwrap();

        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert_eq!(found.client_mtu, Some(1380));

        // The core dropping the value clears it on the next save.
        location.client_mtu = None;
        location.save(&pool).await.unwrap();
        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert_eq!(found.client_mtu, None);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_auto_connect_round_trip(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut location = new_location(instance.id).save(&pool).await.unwrap();
        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert!(!found.auto_connect);

        location.auto_connect = true;
        location.save(&pool).await.unwrap();
        let mut found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert!(found.auto_connect);

        found.name = "renamed".into();
        found.save(&pool).await.unwrap();
        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert!(found.auto_connect);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let location = new_location(instance.id).save(&pool).await.unwrap();

        Location::set_auto_connect(&pool, location.id, true)
            .await
            .unwrap();
        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert!(found.auto_connect);

        Location::set_auto_connect(&pool, location.id, false)
            .await
            .unwrap();
        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert!(!found.auto_connect);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_rejects_mfa_location(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.location_mfa_mode = LocationMfaMode::Internal;
        let location = location.save(&pool).await.unwrap();

        let result = Location::set_auto_connect(&pool, location.id, true).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert!(!found.auto_connect);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_allows_one_all_traffic_location(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut first = new_location(instance.id);
        first.route_all_traffic = true;
        let first = first.save(&pool).await.unwrap();
        let mut second = new_location(instance.id);
        second.network_id = 2;
        second.route_all_traffic = true;
        let second = second.save(&pool).await.unwrap();
        let mut predefined = new_location(instance.id);
        predefined.network_id = 3;
        let predefined = predefined.save(&pool).await.unwrap();

        Location::set_auto_connect(&pool, first.id, true)
            .await
            .unwrap();
        let result = Location::set_auto_connect(&pool, second.id, true).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
        Location::set_auto_connect(&pool, predefined.id, true)
            .await
            .unwrap();

        Location::set_auto_connect(&pool, first.id, false)
            .await
            .unwrap();
        Location::set_auto_connect(&pool, second.id, true)
            .await
            .unwrap();
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_rejects_all_traffic_with_service_location(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut service = new_location(instance.id);
        service.service_location_mode = ServiceLocationMode::AlwaysOn;
        let service = service.save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.network_id = 2;
        location.route_all_traffic = true;
        let location = location.save(&pool).await.unwrap();

        let result = Location::set_auto_connect(&pool, location.id, true).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
        let result = Location::set_auto_connect(&pool, service.id, true).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_allows_all_traffic_beside_predefined_service_location(
        pool: SqlitePool,
    ) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut service = new_location(instance.id);
        service.allowed_ips = "10.0.0.0/24".into();
        service.service_location_mode = ServiceLocationMode::AlwaysOn;
        service.save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.network_id = 2;
        location.route_all_traffic = true;
        let location = location.save(&pool).await.unwrap();

        Location::set_auto_connect(&pool, location.id, true)
            .await
            .unwrap();
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_force_all_traffic_allows_one_location(pool: SqlitePool) {
        let mut instance = new_instance();
        instance.client_traffic_policy = ClientTrafficPolicy::ForceAllTraffic;
        let instance = instance.save(&pool).await.unwrap();
        let first = new_location(instance.id).save(&pool).await.unwrap();
        let mut second = new_location(instance.id);
        second.network_id = 2;
        let second = second.save(&pool).await.unwrap();

        Location::set_auto_connect(&pool, first.id, true)
            .await
            .unwrap();
        let result = Location::set_auto_connect(&pool, second.id, true).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_update_routing_disables_colliding_auto_connect(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut first = new_location(instance.id);
        first.route_all_traffic = true;
        first.auto_connect = true;
        let first = first.save(&pool).await.unwrap();
        let mut second = new_location(instance.id);
        second.network_id = 2;
        second.auto_connect = true;
        let second = second.save(&pool).await.unwrap();

        let auto_connect_disabled = Location::update_routing(&pool, second.id, true)
            .await
            .unwrap();
        assert!(auto_connect_disabled);

        let found = |id| Location::find_by_id(&pool, id);
        assert!(found(first.id).await.unwrap().unwrap().auto_connect);
        let second = found(second.id).await.unwrap().unwrap();
        assert!(second.route_all_traffic);
        assert!(!second.auto_connect);
        assert_auto_connect_valid(&pool).await;
    }

    async fn assert_auto_connect_valid(pool: &SqlitePool) {
        let mut connection = pool.acquire().await.unwrap();
        let context = AutoConnectContext::load(&mut connection).await.unwrap();
        let invalid = context
            .invalid()
            .map(|(location, block)| (location.name.clone(), block))
            .collect::<Vec<_>>();
        assert!(invalid.is_empty(), "invalid auto-connect: {invalid:?}");
    }

    async fn save_other_instance(pool: &SqlitePool) -> Instance<Id> {
        let mut instance = new_instance();
        instance.name = "other".into();
        instance.uuid = "uuid-2".into();
        instance.save(pool).await.unwrap()
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_allows_one_all_traffic_location_across_instances(
        pool: SqlitePool,
    ) {
        let instance = new_instance().save(&pool).await.unwrap();
        let other_instance = save_other_instance(&pool).await;
        let mut first = new_location(instance.id);
        first.route_all_traffic = true;
        let first = first.save(&pool).await.unwrap();
        let mut second = new_location(other_instance.id);
        second.route_all_traffic = true;
        let second = second.save(&pool).await.unwrap();

        Location::set_auto_connect(&pool, first.id, true)
            .await
            .unwrap();
        let result = Location::set_auto_connect(&pool, second.id, true).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
        assert_auto_connect_valid(&pool).await;
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_rejects_all_traffic_with_other_instance_service_location(
        pool: SqlitePool,
    ) {
        let instance = new_instance().save(&pool).await.unwrap();
        let other_instance = save_other_instance(&pool).await;
        let mut service = new_location(other_instance.id);
        service.allowed_ips = "0.0.0.0/0".into();
        service.service_location_mode = ServiceLocationMode::AlwaysOn;
        service.save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.route_all_traffic = true;
        let location = location.save(&pool).await.unwrap();

        let result = Location::set_auto_connect(&pool, location.id, true).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_update_routing_disables_auto_connect_beside_all_traffic_service_location(
        pool: SqlitePool,
    ) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut service = new_location(instance.id);
        service.allowed_ips = "::/0".into();
        service.service_location_mode = ServiceLocationMode::AlwaysOn;
        service.save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.network_id = 2;
        location.auto_connect = true;
        let location = location.save(&pool).await.unwrap();

        let auto_connect_disabled = Location::update_routing(&pool, location.id, true)
            .await
            .unwrap();

        assert!(auto_connect_disabled);
        assert_auto_connect_valid(&pool).await;
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_update_routing_keeps_auto_connect_without_collision(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.auto_connect = true;
        let location = location.save(&pool).await.unwrap();

        let auto_connect_disabled = Location::update_routing(&pool, location.id, true)
            .await
            .unwrap();

        assert!(!auto_connect_disabled);
        let found = Location::find_by_id(&pool, location.id)
            .await
            .unwrap()
            .expect("location should exist");
        assert!(found.auto_connect);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_set_auto_connect_off_ignores_blocks(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.location_mfa_mode = LocationMfaMode::Internal;
        location.auto_connect = true;
        let location = location.save(&pool).await.unwrap();

        Location::set_auto_connect(&pool, location.id, false)
            .await
            .unwrap();

        assert_auto_connect_valid(&pool).await;
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_concurrent_all_traffic_requests_keep_one(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut ids = Vec::new();
        for network_id in 1..=2 {
            let mut location = new_location(instance.id);
            location.network_id = network_id;
            location.route_all_traffic = true;
            ids.push(location.save(&pool).await.unwrap().id);
        }

        let (first, second) = tokio::join!(
            Location::set_auto_connect(&pool, ids[0], true),
            Location::set_auto_connect(&pool, ids[1], true),
        );

        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        assert_auto_connect_valid(&pool).await;
    }

    #[test]
    fn test_infer_mfa_method() {
        use LocationMfaMethod::{Biometric, Email, Oidc, Totp};
        use LocationMfaMode::{Disabled, External, Internal};

        // Disabled mode passes the configured method through unchanged.
        assert_eq!(infer_mfa_method(Disabled, None), None);
        assert_eq!(infer_mfa_method(Disabled, Some(Totp)), Some(Totp));

        // Internal mode forces Totp when no method or OIDC is configured.
        assert_eq!(infer_mfa_method(Internal, None), Some(Totp));
        assert_eq!(infer_mfa_method(Internal, Some(Oidc)), Some(Totp));
        // Internal mode keeps any other explicit method.
        assert_eq!(infer_mfa_method(Internal, Some(Email)), Some(Email));
        assert_eq!(infer_mfa_method(Internal, Some(Biometric)), Some(Biometric));

        // External mode always resolves to OIDC, ignoring the configured method.
        assert_eq!(infer_mfa_method(External, None), Some(Oidc));
        assert_eq!(infer_mfa_method(External, Some(Totp)), Some(Oidc));
    }

    #[test]
    fn test_location_mfa_mode_from_proto() {
        assert_eq!(
            LocationMfaMode::from(ProtoLocationMfaMode::Unspecified),
            LocationMfaMode::Disabled
        );
        assert_eq!(
            LocationMfaMode::from(ProtoLocationMfaMode::Disabled),
            LocationMfaMode::Disabled
        );
        assert_eq!(
            LocationMfaMode::from(ProtoLocationMfaMode::Internal),
            LocationMfaMode::Internal
        );
        assert_eq!(
            LocationMfaMode::from(ProtoLocationMfaMode::External),
            LocationMfaMode::External
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_effective_route_all_traffic(pool: SqlitePool) {
        use ClientTrafficPolicy::{DisableAllTraffic, ForceAllTraffic, None as NoPolicy};

        // (policy, stored flag, per-call override, expected)
        let cases = [
            (NoPolicy, false, None, false),
            (NoPolicy, false, Some(true), true),
            (NoPolicy, true, None, true),
            (NoPolicy, true, Some(false), false),
            (ForceAllTraffic, false, None, true),
            (ForceAllTraffic, false, Some(false), true),
            (DisableAllTraffic, true, None, false),
            (DisableAllTraffic, true, Some(true), false),
        ];

        let mut instance = new_instance().save(&pool).await.unwrap();
        let mut location = new_location(instance.id).save(&pool).await.unwrap();

        for (policy, route_all_traffic, override_value, expected) in cases {
            instance.client_traffic_policy = policy.clone();
            instance.save(&pool).await.unwrap();
            location.route_all_traffic = route_all_traffic;

            let effective = location
                .effective_route_all_traffic(&pool, override_value)
                .await
                .unwrap();
            assert_eq!(
                effective, expected,
                "policy {policy:?}, flag {route_all_traffic}, override {override_value:?}"
            );
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_routed_networks(pool: SqlitePool) {
        let instance = new_instance().save(&pool).await.unwrap();
        let mut location = new_location(instance.id);
        location.address = "10.6.0.2/24, fd00::2/64".into();
        location.allowed_ips = "10.0.0.0/8, bad-entry, 192.168.1.0/24".into();
        let location = location.save(&pool).await.unwrap();

        let networks = |list: &[&str]| -> Vec<IpAddrMask> {
            list.iter().map(|entry| entry.parse().unwrap()).collect()
        };
        assert_eq!(
            location.routed_networks(&pool, None).await.unwrap(),
            networks(&["10.6.0.2/24", "fd00::2/64", "10.0.0.0/8", "192.168.1.0/24"])
        );
        assert_eq!(
            location.routed_networks(&pool, Some(true)).await.unwrap(),
            networks(&["10.6.0.2/24", "fd00::2/64", "0.0.0.0/0", "::/0"])
        );
    }

    #[test]
    fn test_service_location_mode_from_proto() {
        assert_eq!(
            ServiceLocationMode::from(ProtoServiceLocationMode::Unspecified),
            ServiceLocationMode::Disabled
        );
        assert_eq!(
            ServiceLocationMode::from(ProtoServiceLocationMode::Disabled),
            ServiceLocationMode::Disabled
        );
        assert_eq!(
            ServiceLocationMode::from(ProtoServiceLocationMode::Prelogon),
            ServiceLocationMode::PreLogon
        );
        assert_eq!(
            ServiceLocationMode::from(ProtoServiceLocationMode::Alwayson),
            ServiceLocationMode::AlwaysOn
        );
    }
}
