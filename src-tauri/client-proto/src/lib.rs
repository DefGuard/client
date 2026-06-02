pub mod conversions;

pub mod defguard {
    pub mod client_types {
        tonic::include_proto!("defguard.client_types");
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
