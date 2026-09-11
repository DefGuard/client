#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("internal daemon error: {0}")]
    Internal(String),
    #[error("wireguard interface error: {0}")]
    WireGuard(#[from] defguard_wireguard_rs::error::WireguardInterfaceError),
    #[error("service location error: {0}")]
    ServiceLocation(#[from] defguard_client_service_locations::ServiceLocationError),
    #[error("conversion error: {0}")]
    Conversion(String),
    #[error("not found: {0}")]
    NotFound(String),
}
