use std::{collections::HashMap, fmt};

use defguard_client_core::{
    database::models::{
        location::{Location, ServiceLocationMode},
        Id,
    },
    error::Error as CoreError,
};
use defguard_client_proto::defguard::client::v1::{
    ServiceLocation, ServiceLocationMode as ProtoServiceLocationMode,
};
use defguard_wireguard_rs::{error::WireguardInterfaceError, WGApi};
use log::warn;
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
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
    #[cfg(windows)]
    #[error(transparent)]
    WindowsServiceError(#[from] windows_service::Error),
}

#[allow(dead_code)]
#[derive(Default)]
pub struct ServiceLocationManager {
    // Interface name: WireGuard API instance
    wgapis: HashMap<String, WGApi>,
    // Instance ID: Service locations connected under that instance
    connected_service_locations: HashMap<String, Vec<ServiceLocation>>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct ServiceLocationData {
    pub service_locations: Vec<ServiceLocation>,
    pub instance_id: String,
    pub private_key: String,
}

#[allow(dead_code)]
pub struct SingleServiceLocationData {
    pub service_location: ServiceLocation,
    pub instance_id: String,
    pub private_key: String,
}

impl fmt::Debug for ServiceLocationData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceLocationData")
            .field("service_locations", &self.service_locations)
            .field("instance_id", &self.instance_id)
            .field("private_key", &"***")
            .finish()
    }
}

impl fmt::Debug for SingleServiceLocationData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SingleServiceLocationData")
            .field("service_locations", &self.service_location)
            .field("instance_id", &self.instance_id)
            .field("private_key", &"***")
            .finish()
    }
}

pub fn to_service_location(location: &Location<Id>) -> Result<ServiceLocation, CoreError> {
    if !location.is_service_location() {
        warn!("Location {location} is not a service location, so it can't be converted to one.");
        return Err(CoreError::ConversionError(format!(
            "Failed to convert location {location} to a service location as it's either not marked \
            as one or has MFA enabled."
        )));
    }

    let mode = match location.service_location_mode {
        ServiceLocationMode::Disabled => {
            warn!(
            "Location {location} has an invalid service location mode, so it can't be converted to \
            one."
        );
            return Err(CoreError::ConversionError(format!(
                "Location {location} has an invalid service location mode ({:?}), so it can't be \
                converted to one.",
                location.service_location_mode
            )));
        }
        ServiceLocationMode::PreLogon => ProtoServiceLocationMode::PreLogon as i32,
        ServiceLocationMode::AlwaysOn => ProtoServiceLocationMode::AlwaysOn as i32,
    };

    Ok(ServiceLocation {
        name: location.name.clone(),
        address: location.address.clone(),
        pubkey: location.pubkey.clone(),
        endpoint: location.endpoint.clone(),
        allowed_ips: location.allowed_ips.clone(),
        dns: location.dns.clone().unwrap_or_default(),
        keepalive_interval: location.keepalive_interval.try_into().unwrap_or(0),
        mode,
    })
}
