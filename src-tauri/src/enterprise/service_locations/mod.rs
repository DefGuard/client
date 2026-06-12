pub use defguard_client_service_locations::{
    to_service_location, ServiceLocationData, ServiceLocationError, ServiceLocationManager,
    SingleServiceLocationData,
};

#[cfg(windows)]
pub use defguard_client_service_locations::windows;
