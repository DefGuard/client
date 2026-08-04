use std::{collections::HashMap, fmt};
#[cfg(any(windows, target_os = "linux"))]
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use defguard_client_core::{
    database::models::{
        location::{Location, ServiceLocationMode},
        Id,
    },
    error::Error as CoreError,
};
use defguard_client_proto::defguard::client::v1::{
    SaveServiceLocationsRequest, ServiceLocation, ServiceLocationMode as ProtoServiceLocationMode,
};
use defguard_wireguard_rs::{error::WireguardInterfaceError, WGApi};
#[cfg(any(windows, target_os = "linux"))]
use log::info;
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

/// Current schema version of the on-disk service location JSON file.
///
/// Files written by older clients predate versioning and deserialize with `schema_version == 0`
/// (see the `#[serde(default)]` on [`ServiceLocationData::schema_version`]).
pub const SERVICE_LOCATION_SCHEMA_VERSION: u32 = 1;

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct ServiceLocationData {
    pub service_locations: Vec<ServiceLocation>,
    pub instance_id: String,
    pub private_key: String,
    #[serde(default)]
    pub proxy_url: String,
    /// The *device's* WireGuard public key (`WireguardKeys.pubkey`), used to identify this device
    /// to the proxy. This is **not** [`ServiceLocation::pubkey`], which is the remote peer key.
    #[serde(default)]
    pub device_pubkey: String,
    /// Device polling token, used to authenticate posture requests made by the service.
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub schema_version: u32,
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
            .field("proxy_url", &self.proxy_url)
            .field("device_pubkey", &self.device_pubkey)
            .field("token", &self.token.as_ref().map(|_| "***"))
            .field("schema_version", &self.schema_version)
            .finish()
    }
}

impl ServiceLocationData {
    /// Builds the on-disk representation from a daemon save request.
    ///
    /// `service_locations` is passed separately rather than taken from the request because each
    /// platform first filters the requested set down to the modes it supports.
    ///
    /// Every other field is copied from the request here and nowhere else, so a field added to
    /// `SaveServiceLocationsRequest` has exactly one place to be wired in — it cannot be silently
    /// dropped on the way to disk by one platform but not the other.
    #[must_use]
    pub fn from_save_request(
        request: &SaveServiceLocationsRequest,
        service_locations: Vec<ServiceLocation>,
    ) -> Self {
        Self {
            service_locations,
            instance_id: request.instance_id.clone(),
            private_key: request.private_key.clone(),
            proxy_url: request.proxy_url.clone(),
            device_pubkey: request.device_pubkey.clone(),
            token: request.token.clone(),
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        }
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
        network_id: location.network_id,
        posture_check_required: location.posture_check_required,
    })
}

/// Repeatedly attempts to auto-connect all persisted service locations until every location
/// connects or `retry_count` attempts are exhausted, sleeping `retry_delay` between attempts.
///
/// Safe to call at startup: `connect_to_service_locations` skips already-connected locations, so
/// retrying is idempotent. Intended to be spawned as a background task by the daemon, which owns
/// the retry policy and passes it in.
#[cfg(any(windows, target_os = "linux"))]
pub async fn connect_service_locations(
    manager: Arc<RwLock<ServiceLocationManager>>,
    retry_count: u32,
    retry_delay: Duration,
) {
    for attempt in 1..=retry_count {
        info!("Attempting to auto-connect service locations (attempt {attempt}/{retry_count})");
        match manager.write().unwrap().connect_to_service_locations() {
            Ok(true) => {
                info!(
                    "All service locations connected successfully (attempt {attempt}/{retry_count})"
                );
                break;
            }
            Ok(false) => warn!(
                "Service location auto-connect attempt {attempt}/{retry_count} completed with some \
                failures"
            ),
            Err(err) => {
                warn!("Service location auto-connect attempt {attempt}/{retry_count} failed: {err}")
            }
        }

        if attempt < retry_count {
            tokio::time::sleep(retry_delay).await;
        }
    }

    info!("Service location auto-connect task finished");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JSON exactly as written by a client that predates posture checks: no `proxy_url`,
    /// `device_pubkey`, `token` or `schema_version` at the top level, and no `network_id` or
    /// `posture_check_required` inside the locations. This is the upgrade path.
    const LEGACY_JSON: &str = r#"{
      "service_locations": [
        {
          "name": "Office",
          "address": "10.0.0.2/24",
          "pubkey": "remote-peer-pubkey",
          "endpoint": "vpn.example.com:51820",
          "allowed_ips": "10.0.0.0/24",
          "keepalive_interval": 25,
          "dns": "10.0.0.1",
          "mode": 2
        }
      ],
      "instance_id": "d3a5b1f0-0000-0000-0000-000000000001",
      "private_key": "device-private-key"
    }"#;

    #[test]
    fn legacy_json_without_new_fields_deserializes_with_defaults() {
        let data: ServiceLocationData = serde_json::from_str(LEGACY_JSON)
            .expect("legacy service location file must still load");

        assert_eq!(data.instance_id, "d3a5b1f0-0000-0000-0000-000000000001");
        assert_eq!(data.private_key, "device-private-key");
        assert_eq!(data.proxy_url, "");
        assert_eq!(data.device_pubkey, "");
        assert_eq!(data.token, None);
        // 0 marks a file written before schema versioning existed.
        assert_eq!(data.schema_version, 0);

        assert_eq!(data.service_locations.len(), 1);
        let location = &data.service_locations[0];
        assert_eq!(location.name, "Office");
        assert_eq!(location.pubkey, "remote-peer-pubkey");
        assert_eq!(location.network_id, 0);
        assert!(!location.posture_check_required);
    }

    #[test]
    fn truncated_json_still_fails_to_deserialize() {
        // A container-level `#[serde(default)]` on `ServiceLocation` would let malformed entries
        // silently vanish; make sure missing required keys are still an error.
        let json = r#"{
          "service_locations": [{ "network_id": 7 }],
          "instance_id": "id",
          "private_key": "key"
        }"#;

        assert!(serde_json::from_str::<ServiceLocationData>(json).is_err());
    }

    #[test]
    fn round_trip_preserves_new_fields() {
        let data = ServiceLocationData {
            service_locations: vec![ServiceLocation {
                name: "Office".into(),
                address: "10.0.0.2/24".into(),
                pubkey: "remote-peer-pubkey".into(),
                endpoint: "vpn.example.com:51820".into(),
                allowed_ips: "10.0.0.0/24".into(),
                keepalive_interval: 25,
                dns: "10.0.0.1".into(),
                mode: ProtoServiceLocationMode::AlwaysOn as i32,
                network_id: 42,
                posture_check_required: true,
            }],
            instance_id: "instance-uuid".into(),
            private_key: "device-private-key".into(),
            proxy_url: "https://proxy.example.com".into(),
            device_pubkey: "device-public-key".into(),
            token: Some("polling-token".into()),
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        };

        let json = serde_json::to_string(&data).expect("serialization must succeed");
        let restored: ServiceLocationData =
            serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(restored.proxy_url, "https://proxy.example.com");
        assert_eq!(restored.device_pubkey, "device-public-key");
        assert_eq!(restored.token.as_deref(), Some("polling-token"));
        assert_eq!(restored.schema_version, SERVICE_LOCATION_SCHEMA_VERSION);
        assert_eq!(restored.service_locations[0].network_id, 42);
        assert!(restored.service_locations[0].posture_check_required);
        // The remote peer key must not be confused with the device key.
        assert_eq!(restored.service_locations[0].pubkey, "remote-peer-pubkey");
    }

    #[test]
    fn debug_masks_private_key_and_token() {
        let data = ServiceLocationData {
            service_locations: Vec::new(),
            instance_id: "instance-uuid".into(),
            private_key: "super-secret-private-key".into(),
            proxy_url: "https://proxy.example.com".into(),
            device_pubkey: "device-public-key".into(),
            token: Some("super-secret-token".into()),
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        };

        let debug = format!("{data:?}");
        assert!(!debug.contains("super-secret-private-key"), "{debug}");
        assert!(!debug.contains("super-secret-token"), "{debug}");
        // Non-secret fields are still visible for diagnostics.
        assert!(debug.contains("https://proxy.example.com"), "{debug}");
        assert!(debug.contains("device-public-key"), "{debug}");
    }

    #[test]
    fn debug_of_absent_token_is_not_masked_as_present() {
        let data = ServiceLocationData {
            service_locations: Vec::new(),
            instance_id: "instance-uuid".into(),
            private_key: "private".into(),
            proxy_url: String::new(),
            device_pubkey: String::new(),
            token: None,
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        };

        assert!(format!("{data:?}").contains("token: None"));
    }

    #[test]
    fn single_service_location_debug_masks_private_key() {
        let data = SingleServiceLocationData {
            service_location: ServiceLocation {
                name: "Office".into(),
                address: "10.0.0.2/24".into(),
                pubkey: "remote-peer-pubkey".into(),
                endpoint: "vpn.example.com:51820".into(),
                allowed_ips: "10.0.0.0/24".into(),
                keepalive_interval: 25,
                dns: "10.0.0.1".into(),
                mode: ProtoServiceLocationMode::AlwaysOn as i32,
                network_id: 42,
                posture_check_required: true,
            },
            instance_id: "instance-uuid".into(),
            private_key: "super-secret-private-key".into(),
        };

        assert!(!format!("{data:?}").contains("super-secret-private-key"));
    }
}
