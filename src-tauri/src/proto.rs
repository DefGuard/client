use crate::database::models::{
    location::{
        infer_mfa_method, Location, LocationMfaMode as MfaMode,
        ServiceLocationMode as SLocationMode,
    },
    Id, NoId,
};

pub(crate) use defguard_client_proto::defguard;

#[must_use]
pub(crate) fn into_location(
    dev_config: defguard::client_types::DeviceConfig,
    instance_id: Id,
) -> Location<NoId> {
    let location_mfa_mode = match dev_config.location_mfa_mode {
        Some(_location_mfa_mode) => dev_config.location_mfa_mode().into(),
        None => {
            // handle legacy core response
            // DEPRECATED(1.5): superseeded by location_mfa_mode
            #[allow(deprecated)]
            if dev_config.mfa_enabled {
                MfaMode::Internal
            } else {
                MfaMode::Disabled
            }
        }
    };

    let service_location_mode = match dev_config.service_location_mode {
        Some(_service_location_mode) => dev_config.service_location_mode().into(),
        None => SLocationMode::Disabled, // Default to disabled if not set
    };

    Location {
        id: NoId,
        instance_id,
        network_id: dev_config.network_id,
        name: dev_config.network_name,
        address: dev_config.assigned_ip, // Transforming assigned_ip to address
        pubkey: dev_config.pubkey,
        endpoint: dev_config.endpoint,
        allowed_ips: dev_config.allowed_ips,
        dns: dev_config.dns,
        route_all_traffic: false,
        keepalive_interval: dev_config.keepalive_interval.into(),
        location_mfa_mode,
        service_location_mode,
        mfa_method: infer_mfa_method(location_mfa_mode, None),
        posture_check_required: dev_config.posture_check_required.unwrap_or_default(),
    }
}
