use crate::database::models::{
    location::{
        infer_mfa_method, Location, LocationMfaMode as MfaMode,
        ServiceLocationMode as SLocationMode,
    },
    Id, NoId,
};

pub(crate) mod defguard {
    pub(crate) use crate::service::proto::defguard::client_types;

    pub(crate) mod proxy {
        pub(crate) mod v1 {
            tonic::include_proto!("defguard.proxy.v1");
        }
    }
}

impl defguard::client_types::DeviceConfig {
    #[must_use]
    pub(crate) fn into_location(self, instance_id: Id) -> Location<NoId> {
        let location_mfa_mode = match self.location_mfa_mode {
            Some(_location_mfa_mode) => self.location_mfa_mode().into(),
            None => {
                // handle legacy core response
                // DEPRECATED(1.5): superseeded by location_mfa_mode
                #[allow(deprecated)]
                if self.mfa_enabled {
                    MfaMode::Internal
                } else {
                    MfaMode::Disabled
                }
            }
        };

        let service_location_mode = match self.service_location_mode {
            Some(_service_location_mode) => self.service_location_mode().into(),
            None => SLocationMode::Disabled, // Default to disabled if not set
        };

        Location {
            id: NoId,
            instance_id,
            network_id: self.network_id,
            name: self.network_name,
            address: self.assigned_ip, // Transforming assigned_ip to address
            pubkey: self.pubkey,
            endpoint: self.endpoint,
            allowed_ips: self.allowed_ips,
            dns: self.dns,
            route_all_traffic: false,
            keepalive_interval: self.keepalive_interval.into(),
            location_mfa_mode,
            service_location_mode,
            mfa_method: infer_mfa_method(location_mfa_mode, None),
            posture_check_required: self.posture_check_required.unwrap_or_default(),
        }
    }
}
