/// Non-macOS connection setup helpers.
use std::str::FromStr;

use std::process::Command;

use defguard_client_common::{find_free_tcp_port, get_interface_name};
use defguard_wireguard_rs::{key::Key, net::IpAddrMask, peer::Peer, InterfaceConfiguration};
use tonic::Code;

use crate::{
    connection::daemon_client::DAEMON_CLIENT,
    database::{
        models::{
            connection::{ActiveConnection, Connection},
            location::Location,
            tunnel::{Tunnel, TunnelConnection},
            Id,
        },
        DbPool, DB_POOL,
    },
    error::Error,
    ConnectionType, DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6,
};
use defguard_client_proto::defguard::client::v1::{CreateInterfaceRequest, RemoveInterfaceRequest};

pub async fn setup_interface(
    location: &Location<Id>,
    name: &str,
    preshared_key: Option<String>,
    mtu: Option<u32>,
    pool: &DbPool,
) -> Result<String, Error> {
    debug!("Setting up interface for location: {location}");
    let interface_name = get_interface_name(name);

    debug!("Looking for a free port for interface {interface_name}.");
    let Some(port) = find_free_tcp_port() else {
        let msg = format!(
            "Couldn't find free port during interface {interface_name} setup for location {location}"
        );
        error!("{msg}");
        return Err(Error::InternalError(msg));
    };
    debug!("Found free port: {port} for interface {interface_name}.");

    let interface_config = location
        .interface_configuration(pool, interface_name.clone(), preshared_key, mtu)
        .await?;
    debug!("Creating interface for location {location} with configuration {interface_config:?}");
    let request = CreateInterfaceRequest {
        config: Some(interface_config.clone().into()),
        dns: location.dns.clone(),
    };
    if let Err(error) = DAEMON_CLIENT.clone().create_interface(request).await {
        if error.code() == Code::Unavailable {
            error!(
                "Failed to set up connection for location {location}; background service is \
                unavailable. Make sure the service is running. Error: {error}"
            );
            Err(Error::InternalError(
                "Background service is unavailable. Make sure the service is running.".into(),
            ))
        } else {
            error!(
                "Failed to send a request to the background service to create an interface for \
                location {location}. Error: {error}"
            );
            Err(Error::InternalError(format!(
                "Failed to send a request to the background service to create an interface for \
                location {location}. Error: {error}. Check logs for details."
            )))
        }
    } else {
        info!(
            "The interface for location {location} has been created successfully, interface \
            name: {}.",
            interface_config.name
        );
        Ok(interface_name)
    }
}

pub async fn setup_interface_tunnel(
    tunnel: &Tunnel<Id>,
    name: &str,
    mtu: Option<u32>,
) -> Result<String, Error> {
    debug!("Setting up interface for tunnel {tunnel}");
    let interface_name = get_interface_name(name);

    debug!(
        "Decoding tunnel {tunnel} public key: {}.",
        tunnel.server_pubkey
    );
    let peer_key = Key::from_str(&tunnel.server_pubkey)?;
    debug!("Tunnel {tunnel} public key decoded.");
    let mut peer = Peer::new(peer_key);

    debug!("Parsing tunnel {tunnel} endpoint: {}", tunnel.endpoint);
    peer.set_endpoint(&tunnel.endpoint)?;
    peer.persistent_keepalive_interval = Some(
        tunnel
            .persistent_keep_alive
            .try_into()
            .expect("Failed to parse persistent keep alive"),
    );
    debug!("Parsed tunnel {tunnel} endpoint: {}", tunnel.endpoint);

    if let Some(psk) = &tunnel.preshared_key {
        debug!("Decoding tunnel {tunnel} preshared key.");
        let peer_psk = Key::from_str(psk)?;
        debug!("Preshared key for tunnel {tunnel} decoded.");
        peer.preshared_key = Some(peer_psk);
    }

    debug!(
        "Parsing tunnel {tunnel} allowed ips: {:?}",
        tunnel.allowed_ips
    );
    let allowed_ips = if tunnel.route_all_traffic {
        debug!("Using all traffic routing for tunnel {tunnel}");
        vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
    } else {
        let msg = match &tunnel.allowed_ips {
            Some(ips) => format!("Using predefined location traffic for tunnel {tunnel}: {ips}"),
            None => "No allowed IP addresses found in tunnel {tunnel} configuration".to_string(),
        };
        debug!("{msg}");
        tunnel
            .allowed_ips
            .as_ref()
            .map(|ips| ips.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    };
    for allowed_ip in &allowed_ips {
        match IpAddrMask::from_str(allowed_ip.trim()) {
            Ok(addr) => {
                peer.allowed_ips.push(addr);
            }
            Err(err) => {
                error!("Error parsing IP address {allowed_ip}: {err}");
            }
        }
    }
    debug!("Parsed tunnel {tunnel} allowed IPs: {:?}", peer.allowed_ips);

    debug!("Looking for a free port for interface {interface_name}.");
    let Some(port) = find_free_tcp_port() else {
        let msg = format!(
            "Couldn't find free port for interface {interface_name} while setting up tunnel \
            {tunnel}"
        );
        error!("{msg}");
        return Err(Error::InternalError(msg));
    };
    debug!("Found free port: {port} for interface {interface_name}.");

    let addresses = tunnel
        .address
        .split(',')
        .map(str::trim)
        .map(IpAddrMask::from_str)
        .collect::<Result<_, _>>()
        .map_err(|err| {
            let msg = format!("Failed to parse IP addresses '{}': {err}", tunnel.address);
            error!("{msg}");
            Error::InternalError(msg)
        })?;
    let interface_config = InterfaceConfiguration {
        name: interface_name.clone(),
        prvkey: tunnel.prvkey.clone(),
        addresses,
        port,
        peers: vec![peer.clone()],
        mtu,
        fwmark: None,
    };

    debug!("Creating interface {interface_config:?}");
    let request = CreateInterfaceRequest {
        config: Some(interface_config.clone().into()),
        dns: tunnel.dns.clone(),
    };
    if let Some(pre_up) = &tunnel.pre_up {
        debug!(
            "Executing defined PreUp command before setting up the interface {} for the tunnel \
            {tunnel}: {pre_up}",
            interface_config.name
        );
        let _ = execute_command(pre_up);
        info!(
            "Executed defined PreUp command before setting up the interface {} for the tunnel \
            {tunnel}: {pre_up}",
            interface_config.name
        );
    }
    if let Err(error) = DAEMON_CLIENT.clone().create_interface(request).await {
        error!(
            "Failed to create a network interface ({}) for tunnel {tunnel}: {error}",
            interface_config.name
        );
        return Err(Error::InternalError(format!(
            "Failed to create a network interface ({}) for tunnel {tunnel}, error message: {}. \
            Check logs for more details.",
            interface_config.name,
            error.message()
        )));
    }

    info!(
        "Network interface {} for tunnel {tunnel} created successfully.",
        interface_config.name
    );
    if let Some(post_up) = &tunnel.post_up {
        debug!(
            "Executing defined PostUp command after setting up the interface {} for the tunnel \
            {tunnel}: {post_up}",
            interface_config.name
        );
        let _ = execute_command(post_up);
        info!(
            "Executed defined PostUp command after setting up the interface {} for the tunnel \
            {tunnel}: {post_up}",
            interface_config.name
        );
    }
    debug!(
        "Created interface {} with config: {interface_config:?}",
        interface_config.name
    );

    Ok(interface_name)
}

pub async fn disconnect_interface(active_connection: &ActiveConnection) -> Result<(), Error> {
    debug!(
        "Disconnecting interface {}.",
        active_connection.interface_name
    );
    let location_id = active_connection.location_id;
    let interface_name = active_connection.interface_name.clone();

    match active_connection.connection_type {
        ConnectionType::Location => {
            let Some(location) = Location::find_by_id(&*DB_POOL, location_id).await? else {
                error!(
                    "Error while disconnecting interface {interface_name}, location with ID \
                    {location_id} not found"
                );
                return Err(Error::NotFound);
            };

            #[cfg(target_os = "macos")]
            {
                let result = location.stop_vpn_tunnel();
                error!(
                    "stop_tunnel() for location {} returned {result:?}",
                    location.name
                );
                if !result {
                    return Err(Error::InternalError("Error from tunnel".into()));
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                let request = RemoveInterfaceRequest {
                    interface_name,
                    endpoint: location.endpoint.clone(),
                };
                debug!(
                    "Sending request to the background service to remove interface {} for \
                    location {}...",
                    active_connection.interface_name, location.name
                );
                if let Err(error) = DAEMON_CLIENT.clone().remove_interface(request).await {
                    let msg = if error.code() == Code::Unavailable {
                        format!(
                            "Couldn't remove interface {}. Background service is unavailable. \
                            Please make sure the service is running. Error: {error}.",
                            active_connection.interface_name
                        )
                    } else {
                        format!(
                            "Failed to send a request to the background service to remove \
                            interface {}. Error: {error}.",
                            active_connection.interface_name
                        )
                    };
                    error!("{msg}");
                }
            }

            let connection: Connection = active_connection.into();
            let connection = connection.save(&*DB_POOL).await?;
            debug!(
                "Saved location {} new connection status in the database",
                location.name
            );
            trace!("Saved connection: {connection:?}");
            info!(
                "Network interface {} for location {location} has been removed",
                active_connection.interface_name
            );
            debug!("Finished disconnecting from location {}", location.name);
        }
        ConnectionType::Tunnel => {
            let Some(tunnel) = Tunnel::find_by_id(&*DB_POOL, location_id).await? else {
                error!(
                    "Error while disconnecting interface {interface_name}, tunnel with ID \
                    {location_id} not found"
                );
                return Err(Error::NotFound);
            };
            if let Some(pre_down) = &tunnel.pre_down {
                debug!(
                    "Executing defined PreDown command before setting up the interface {} for \
                    the tunnel {tunnel}: {pre_down}",
                    active_connection.interface_name
                );
                let _ = execute_command(pre_down);
                info!(
                    "Executed defined PreDown command before setting up the interface {} for \
                    the tunnel {tunnel}: {pre_down}",
                    active_connection.interface_name
                );
            }

            #[cfg(target_os = "macos")]
            {
                let result = tunnel.stop_vpn_tunnel();
                error!(
                    "stop_tunnel() for tunnel {} returned {result:?}",
                    tunnel.name
                );
                if !result {
                    return Err(Error::InternalError("Error from tunnel".into()));
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                let request = RemoveInterfaceRequest {
                    interface_name,
                    endpoint: tunnel.endpoint.clone(),
                };
                if let Err(error) = DAEMON_CLIENT.clone().remove_interface(request).await {
                    error!(
                        "Error while removing interface {}, error details: {error:?}",
                        active_connection.interface_name
                    );
                    return Err(Error::InternalError(format!(
                        "Failed to remove interface, error message: {}",
                        error.message()
                    )));
                }
            }
            if let Some(post_down) = &tunnel.post_down {
                debug!(
                    "Executing defined PostDown command after removing the interface {} for \
                    the tunnel {tunnel}: {post_down}",
                    active_connection.interface_name
                );
                let _ = execute_command(post_down);
                info!(
                    "Executed defined PostDown command after removing the interface {} for \
                    the tunnel {tunnel}: {post_down}",
                    active_connection.interface_name
                );
            }
            let connection: TunnelConnection = active_connection.into();
            let connection = connection.save(&*DB_POOL).await?;
            debug!(
                "Saved new tunnel {} connection status in the database",
                tunnel.name
            );
            trace!("Saved connection: {connection:#?}");
            info!(
                "Network interface {} for tunnel {tunnel} has been removed",
                active_connection.interface_name
            );
            debug!("Finished disconnecting from tunnel {}", tunnel.name);
        }
    }

    Ok(())
}

pub fn execute_command(command: &str) -> Result<(), Error> {
    debug!("Executing command: {command}");
    let mut command_parts = command.split_whitespace();

    if let Some(command) = command_parts.next() {
        let output = Command::new(command).args(command_parts).output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("Command {command} executed successfully. Stdout: {stdout}");
            if !stderr.is_empty() {
                error!("Command produced the following output on stderr: {stderr}");
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Error while executing command: {command}. Stderr: {stderr}");
        }
    }
    Ok(())
}
