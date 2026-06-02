pub mod daemon_client;
pub mod setup;

#[cfg(target_os = "macos")]
pub mod apple;

#[cfg(not(target_os = "macos"))]
pub use setup::{disconnect_interface, execute_command, setup_interface, setup_interface_tunnel};

#[cfg(target_os = "macos")]
pub use apple::{
    get_managers_for_tunnels_and_locations, location_tunnel_configuration,
    sync_locations_and_tunnels, tunnel_stats, tunnel_tunnel_configuration, TunnelConfiguration,
};
