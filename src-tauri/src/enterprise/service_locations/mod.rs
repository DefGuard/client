pub use defguard_service_locations::{
    to_service_location, ServiceLocationData, ServiceLocationError, ServiceLocationManager,
    SingleServiceLocationData,
};

#[cfg(windows)]
pub use defguard_service_locations::windows;
