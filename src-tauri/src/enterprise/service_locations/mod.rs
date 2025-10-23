use std::collections::HashMap;

use defguard_wireguard_rs::{error::WireguardInterfaceError, WGApi};
use serde::{Deserialize, Serialize};

use crate::{
    database::models::{
        location::{Location, ServiceLocationMode},
        Id,
    },
    service::proto::ServiceLocation,
};

#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, thiserror::Error)]
pub enum ServiceLocationError {
    #[error("Error occurred while initializing service location API: {0}")]
    InitError(String),
    #[error("Failed to load service location storage: {0}")]
    LoadError(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    DecodeError(#[from] base64::DecodeError),
    #[error(transparent)]
    WireGuardError(#[from] WireguardInterfaceError),
    #[error(transparent)]
    AddrParseError(#[from] defguard_wireguard_rs::net::IpAddrParseError),
    #[error("WireGuard interface error: {0}")]
    InterfaceError(String),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    ProtoEnumError(#[from] prost::UnknownEnumValue),
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    WindowsServiceError(#[from] windows_service::Error),
}

#[derive(Default)]
pub(crate) struct ServiceLocationManager {
    // Interface name: WireGuard API instance
    wgapis: HashMap<String, WGApi>,
    // Instance ID: Service locations connected under that instance
    connected_service_locations: HashMap<String, Vec<ServiceLocation>>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ServiceLocationData {
    pub service_locations: Vec<ServiceLocation>,
    pub instance_id: String,
    pub private_key: String,
}

pub(crate) struct SingleServiceLocationData {
    pub service_location: ServiceLocation,
    pub instance_id: String,
    pub private_key: String,
}

impl std::fmt::Debug for ServiceLocationData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceLocationData")
            .field("service_locations", &self.service_locations)
            .field("instance_id", &self.instance_id)
            .field("private_key", &"***")
            .finish()
    }
}

impl std::fmt::Debug for SingleServiceLocationData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingleServiceLocationData")
            .field("service_locations", &self.service_location)
            .field("instance_id", &self.instance_id)
            .field("private_key", &"***")
            .finish()
    }
}

impl Location<Id> {
    pub fn to_service_location(&self) -> Result<ServiceLocation, crate::error::Error> {
        if !self.is_service_location() {
            warn!(
                "Location {} is not a service location, so it can't be converted to one.",
                self
            );
            return Err(crate::error::Error::ConversionError(format!(
                "Failed to convert location {} to a service location as it's either not marked as one or has MFA enabled.",
                self
            )));
        }

        let mode = match self.service_location_mode {
            ServiceLocationMode::Disabled => {
                warn!(
                "Location {} has an invalid service location mode, so it can't be converted to one.",
                self
            );
                return Err(
                    crate::error::Error::ConversionError(format!("Location {} has an invalid service location mode ({:?}), so it can't be converted to one.", self, self.service_location_mode))
                );
            }
            ServiceLocationMode::PreLogon => 0,
            ServiceLocationMode::AlwaysOn => 1,
        };

        Ok(ServiceLocation {
            name: self.name.clone(),
            address: self.address.clone(),
            pubkey: self.pubkey.clone(),
            endpoint: self.endpoint.clone(),
            allowed_ips: self.allowed_ips.clone(),
            dns: self.dns.clone().unwrap_or_default(),
            keepalive_interval: self.keepalive_interval.try_into().unwrap_or(0),
            mode: mode,
        })
    }
}
