pub mod conversions;
pub mod posture_ext;

pub mod defguard {
    pub mod device_identity {
        pub mod v2 {
            tonic::include_proto!("defguard.device_identity.v2");
        }
    }

    #[allow(clippy::enum_variant_names)]
    pub mod client_types {
        tonic::include_proto!("defguard.client_types");
    }

    pub mod device_identity {
        pub mod v2 {
            tonic::include_proto!("defguard.device_identity.v2");
        }
    }

    pub mod client {
        pub mod v1 {
            tonic::include_proto!("defguard.client.v1");
        }
    }

    pub mod proxy {
        pub mod v1 {
            tonic::include_proto!("defguard.proxy.v1");
        }
    }

    pub mod enterprise {
        pub mod posture {
            pub mod v2 {
                tonic::include_proto!("defguard.enterprise.posture.v2");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::defguard::client_types::DeviceConfig;

    fn dns_from(dns: Option<serde_json::Value>) -> Option<String> {
        let mut config = json!({
            "network_id": 1,
            "network_name": "net",
            "config": "",
            "endpoint": "",
            "assigned_ip": "",
            "pubkey": "",
            "allowed_ips": "",
            "mfa_enabled": false,
            "keepalive_interval": 25,
        });
        if let Some(dns) = dns {
            config["dns"] = dns;
        }
        serde_json::from_value::<DeviceConfig>(config).unwrap().dns
    }

    #[test]
    fn test_device_config_dns() {
        assert_eq!(dns_from(Some(json!(""))), None);
        assert_eq!(dns_from(Some(json!(null))), None);
        assert_eq!(dns_from(None), None);
        assert_eq!(
            dns_from(Some(json!("10.0.0.1, example.com"))),
            Some("10.0.0.1, example.com".into())
        );
    }
}
