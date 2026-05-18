pub mod defguard {
    pub mod enterprise {
        pub mod posture {
            pub mod v2 {
                tonic::include_proto!("defguard.enterprise.posture.v2");
            }
        }
    }
    pub mod client {
        pub mod v1 {
            tonic::include_proto!("defguard.client.v1");
        }
    }
}
