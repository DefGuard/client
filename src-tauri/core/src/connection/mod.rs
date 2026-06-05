pub mod active_connections;
pub mod daemon_client;
#[cfg(not(target_os = "macos"))]
pub mod setup;

#[cfg(target_os = "macos")]
pub mod apple;

#[cfg(not(target_os = "macos"))]
pub use setup::{disconnect_interface, execute_command, setup_interface, setup_interface_tunnel};

#[cfg(target_os = "macos")]
pub use apple::{
    get_managers_for_tunnels_and_locations, sync_locations_and_tunnels, tunnel_stats,
    TunnelConfiguration,
};
