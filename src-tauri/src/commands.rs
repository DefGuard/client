use crate::{
    appstate::AppState,
    database::{
        models::{
            instance::InstanceInfo, location_stats::LocationStats, settings::SettingsPatch, Id,
            NoId,
        },
        ActiveConnection, Connection, ConnectionInfo, Instance, Location, Settings, Tunnel,
        TunnelConnection, TunnelConnectionInfo, TunnelStats, WireguardKeys,
    },
    error::Error,
    events::{CONNECTION_CHANGED, INSTANCE_UPDATE, LOCATION_UPDATE},
    periodic::config::poll_instance,
    proto::{DeviceConfig, DeviceConfigResponse},
    service::{log_watcher::stop_log_watcher_task, proto::RemoveInterfaceRequest},
    tray::{configure_tray_icon, reload_tray_menu},
    utils::{
        disconnect_interface, get_location_interface_details, get_tunnel_interface_details,
        handle_connection_for_location, handle_connection_for_tunnel,
    },
    wg_config::parse_wireguard_config,
    CommonConnection, CommonConnectionInfo, CommonLocationStats, ConnectionType,
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, str::FromStr};
use struct_patch::Patch;
use tauri::{AppHandle, Manager, State};

#[derive(Clone, serde::Serialize)]
pub struct Payload {
    pub message: String,
}

// Create new WireGuard interface
#[tauri::command(async)]
pub async fn connect(
    location_id: i64,
    connection_type: ConnectionType,
    preshared_key: Option<String>,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Connecting location {location_id} using connection type {connection_type:?}");
    let state = handle.state::<AppState>();
    if connection_type.eq(&ConnectionType::Location) {
        if let Some(location) = Location::find_by_id(&state.get_pool(), location_id).await? {
            handle_connection_for_location(&location, preshared_key, handle.clone()).await?;
            reload_tray_menu(&handle).await;
        } else {
            error!("Location {location_id} not found");
            return Err(Error::NotFound);
        }
    } else if let Some(tunnel) = Tunnel::find_by_id(&state.get_pool(), location_id).await? {
        handle_connection_for_tunnel(&tunnel, handle).await?;
    } else {
        error!("Tunnel {location_id} not found");
        return Err(Error::NotFound);
    }
    info!("Connected to location with id: {location_id}");
    Ok(())
}

#[tauri::command]
pub async fn disconnect(
    location_id: i64,
    connection_type: ConnectionType,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Disconnecting location {location_id}");
    let state = handle.state::<AppState>();
    if let Some(connection) = state
        .find_and_remove_connection(location_id, &connection_type)
        .await
    {
        let interface_name = connection.interface_name.clone();
        debug!("Found active connection");
        trace!("Connection: {:#?}", connection);
        disconnect_interface(&connection, &state).await?;
        debug!("Connection saved");
        handle.emit_all(
            CONNECTION_CHANGED,
            Payload {
                message: "Created new connection".into(),
            },
        )?;
        stop_log_watcher_task(&handle, &interface_name)?;
        if connection_type == ConnectionType::Location {
            maybe_update_instance_config(location_id, &handle).await?;
        }
        info!("Disconnected from location with id: {location_id}");
        reload_tray_menu(&handle).await;
        Ok(())
    } else {
        error!("Error while disconnecting from location with id: {location_id} not found");
        Err(Error::NotFound)
    }
}

/// Triggers poll on location's instance config. Config will be updated if there are no more active
/// connections for this instance.
async fn maybe_update_instance_config(location_id: i64, handle: &AppHandle) -> Result<(), Error> {
    let state: State<'_, AppState> = handle.state();
    let pool = state.get_pool();
    let Some(location) = Location::find_by_id(&pool, location_id).await? else {
            error!("Location {location_id} not found, skipping config update check");
            return Err(Error::NotFound);
        };
    let Some(instance) = Instance::find_by_id(&pool, location.instance_id).await? else {
            error!("Instance {} not found, skipping config update check", location.instance_id);
            return Err(Error::NotFound);
        };
    poll_instance(&state.get_pool(), &instance, handle.clone()).await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub pubkey: String,
    pub user_id: i64,
    pub created_at: i64,
}

#[must_use]
pub fn device_config_to_location(device_config: DeviceConfig, instance_id: i64) -> Location<NoId> {
    Location {
        id: NoId,
        instance_id,
        network_id: device_config.network_id,
        name: device_config.network_name,
        address: device_config.assigned_ip, // Transforming assigned_ip to address
        pubkey: device_config.pubkey,
        endpoint: device_config.endpoint,
        allowed_ips: device_config.allowed_ips,
        dns: device_config.dns,
        route_all_traffic: false,
        mfa_enabled: device_config.mfa_enabled,
        keepalive_interval: device_config.keepalive_interval.into(),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceResponse {
    // uuid
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveDeviceConfigResponse {
    locations: Vec<Location<Id>>,
    instance: Instance<Id>,
}

#[tauri::command(async)]
pub async fn save_device_config(
    private_key: String,
    response: DeviceConfigResponse,
    app_state: State<'_, AppState>,
    handle: AppHandle,
) -> Result<SaveDeviceConfigResponse, Error> {
    debug!("Received device configuration: {response:#?}.");

    let mut transaction = app_state.get_pool().begin().await?;
    let instance_info = response
        .instance
        .expect("Missing instance info in device config response");
    let mut instance: Instance = instance_info.into();
    instance.token = response.token;

    let instance = instance.save(&mut *transaction).await?;

    let device = response
        .device
        .expect("Missing device info in device config response");
    let mut keys = WireguardKeys::new(instance.id, device.pubkey, private_key);
    keys.save(&mut *transaction).await?;
    for location in response.configs {
        let new_location = device_config_to_location(location, instance.id);
        new_location.save(&mut *transaction).await?;
    }
    transaction.commit().await?;
    info!("Instance created.");
    trace!("Created following instance: {instance:#?}");
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance.id).await?;
    trace!("Created following locations: {locations:#?}");
    handle.emit_all(INSTANCE_UPDATE, ())?;
    let res: SaveDeviceConfigResponse = SaveDeviceConfigResponse {
        locations,
        instance,
    };
    info!("Device configuration saved.");
    Ok(res)
}

#[tauri::command(async)]
pub async fn all_instances(app_state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, Error> {
    debug!("Retrieving all instances.");

    let instances = Instance::all(&app_state.get_pool()).await?;
    debug!("Found ({}) instances", instances.len());
    trace!("Instances found: {instances:#?}");
    let mut instance_info: Vec<InstanceInfo> = Vec::new();
    let connection_ids: Vec<i64> = app_state
        .get_connection_id_by_type(&ConnectionType::Location)
        .await;
    for instance in instances {
        let locations = Location::find_by_instance_id(&app_state.get_pool(), instance.id).await?;
        let location_ids: Vec<i64> = locations.iter().map(|location| location.id).collect();
        let connected = connection_ids
            .iter()
            .any(|item1| location_ids.iter().any(|item2| item1 == item2));
        let keys = WireguardKeys::find_by_instance_id(&app_state.get_pool(), instance.id)
            .await?
            .ok_or(Error::NotFound)?;
        instance_info.push(InstanceInfo {
            id: Some(instance.id),
            uuid: instance.uuid,
            name: instance.name,
            url: instance.url,
            proxy_url: instance.proxy_url,
            active: connected,
            pubkey: keys.pubkey,
        });
    }
    info!("Instances retrieved({})", instance_info.len());
    trace!("Returning following instances: {instance_info:#?}");
    Ok(instance_info)
}

#[derive(Serialize, Debug)]
pub struct LocationInfo {
    pub id: i64,
    pub instance_id: i64,
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub active: bool,
    pub route_all_traffic: bool,
    pub connection_type: ConnectionType,
    pub pubkey: String,
    pub mfa_enabled: bool,
    pub network_id: i64,
}

#[tauri::command(async)]
pub async fn all_locations(
    instance_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<LocationInfo>, Error> {
    debug!("Retrieving all locations.");
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance_id).await?;
    let active_locations_ids: Vec<i64> = app_state
        .get_connection_id_by_type(&ConnectionType::Location)
        .await;
    let mut location_info = Vec::new();
    for location in locations {
        let info = LocationInfo {
            id: location.id,
            instance_id: location.instance_id,
            name: location.name,
            address: location.address,
            endpoint: location.endpoint,
            active: active_locations_ids.contains(&location.id),
            route_all_traffic: location.route_all_traffic,
            connection_type: ConnectionType::Location,
            pubkey: location.pubkey,
            mfa_enabled: location.mfa_enabled,
            network_id: location.network_id,
        };
        location_info.push(info);
    }
    info!("Locations retrieved({})", location_info.len());
    debug!(
        "Returning {} locations for instance {instance_id}",
        location_info.len(),
    );
    trace!("Locations returned:\n{location_info:#?}");

    Ok(location_info)
}

#[derive(Serialize, Debug)]
pub struct LocationInterfaceDetails {
    pub location_id: i64,
    // client interface config
    pub name: String,    // interface name generated from location name
    pub pubkey: String,  // own pubkey of client interface
    pub address: String, // IP within WireGuard network assigned to the client
    pub dns: Option<String>,
    pub listen_port: Option<u32>,
    // peer config
    pub peer_pubkey: String,
    pub peer_endpoint: String,
    pub allowed_ips: String,
    pub persistent_keepalive_interval: Option<u16>,
    pub last_handshake: Option<i64>,
}

#[tauri::command(async)]
pub async fn location_interface_details(
    location_id: i64,
    connection_type: ConnectionType,
    app_state: State<'_, AppState>,
) -> Result<LocationInterfaceDetails, Error> {
    let pool = app_state.get_pool();
    match connection_type {
        ConnectionType::Location => get_location_interface_details(location_id, &pool).await,
        ConnectionType::Tunnel => get_tunnel_interface_details(location_id, &pool).await,
    }
}

#[tauri::command(async)]
pub async fn update_instance(
    instance_id: i64,
    response: DeviceConfigResponse,
    app_state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), Error> {
    debug!("Received update_instance command");
    trace!("Processing following response:\n {response:#?}");
    let pool = app_state.get_pool();

    if let Some(mut instance) = Instance::find_by_id(&pool, instance_id).await? {
        // fetch existing locations for given instance
        let mut current_locations = Location::find_by_instance_id(&pool, instance_id).await?;

        let mut transaction = pool.begin().await?;

        // update instance
        debug!("Updating instance {instance_id}.");
        let instance_info = response
            .instance
            .expect("Missing instance info in device config response");
        instance.name = instance_info.name;
        instance.url = instance_info.url;
        instance.proxy_url = instance_info.proxy_url;
        instance.username = instance_info.username;
        instance.save(&mut *transaction).await?;

        // process locations received in response
        debug!("Updating locations for instance {instance_id}.");
        for location in response.configs {
            // parse device config
            let new_location = device_config_to_location(location, instance_id);

            // check if location is already present in current locations
            if let Some(position) = current_locations
                .iter()
                .position(|loc| loc.network_id == new_location.network_id)
            {
                // remove from list of existing locations
                let mut current_location = current_locations.remove(position);
                debug!(
                    "Updating existing location {} for instance {instance_id}.",
                    current_location.name
                );
                // update existing location
                current_location.name = new_location.name;
                current_location.address = new_location.address;
                current_location.pubkey = new_location.pubkey;
                current_location.endpoint = new_location.endpoint;
                current_location.allowed_ips = new_location.allowed_ips;
                current_location.mfa_enabled = new_location.mfa_enabled;
                current_location.keepalive_interval = new_location.keepalive_interval;
                current_location.dns = new_location.dns;
                current_location.save(&mut *transaction).await?;
                info!(
                    "Location {} updated for instance {instance_id}.",
                    current_location.name
                );
            } else {
                // create new location
                debug!("Creating new location for instance {instance_id}.");
                new_location.save(&mut *transaction).await?;
                info!("New location created for instance {instance_id}.");
            }
        }
        info!("Locations updated for instance {instance_id}.");

        // remove locations which were present in current locations
        // but no longer found in core response
        debug!("Removing locations for instance {instance_id}.");
        for removed_location in current_locations {
            removed_location.delete(&mut *transaction).await?;
        }
        info!("Locations removed for instance {instance_id}.");

        transaction.commit().await?;

        info!("Instance {instance_id} updated");
        app_handle.emit_all(INSTANCE_UPDATE, ())?;
        Ok(())
    } else {
        error!("Instance with id {instance_id} not found");
        Err(Error::NotFound)
    }
}

/// If `datetime` is Some, parses the date string, otherwise returns `DateTime` one hour ago.
pub(crate) fn parse_timestamp(from: Option<String>) -> Result<DateTime<Utc>, Error> {
    Ok(match from {
        Some(from) => DateTime::<Utc>::from_str(&from).map_err(|_| Error::Datetime)?,
        None => Utc::now() - Duration::hours(1),
    })
}

pub enum DateTimeAggregation {
    Hour,
    Second,
}

impl DateTimeAggregation {
    /// Returns database format string for given aggregation variant
    #[must_use]
    pub fn fstring(&self) -> &'static str {
        match self {
            Self::Hour => "%Y-%m-%d %H:00:00",
            Self::Second => "%Y-%m-%d %H:%M:%S",
        }
    }
}

fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, Error> {
    // Use hourly aggregation for longer periods
    let aggregation = match Utc::now().naive_utc() - from {
        duration if duration >= Duration::hours(8) => Ok(DateTimeAggregation::Hour),
        duration if duration < Duration::zero() => Err(Error::InternalError(format!(
            "Negative duration between dates: now ({}) and {from}",
            Utc::now().naive_utc(),
        ))),
        _ => Ok(DateTimeAggregation::Second),
    }?;
    Ok(aggregation)
}

#[tauri::command(async)]
pub async fn location_stats(
    location_id: i64,
    connection_type: ConnectionType,
    from: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<CommonLocationStats>, Error> {
    trace!("Location stats command received");
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats: Vec<CommonLocationStats> = match connection_type {
        ConnectionType::Location => LocationStats::all_by_location_id(
            &app_state.get_pool(),
            location_id,
            &from,
            &aggregation,
        )
        .await?
        .into_iter()
        .map(Into::into)
        .collect(),
        ConnectionType::Tunnel => {
            TunnelStats::all_by_tunnel_id(&app_state.get_pool(), location_id, &from, &aggregation)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        }
    };

    Ok(stats)
}

#[tauri::command(async)]
pub async fn all_connections(
    location_id: i64,
    connection_type: ConnectionType,
    app_state: State<'_, AppState>,
) -> Result<Vec<CommonConnectionInfo>, Error> {
    debug!("Retrieving connections for location {location_id}");
    let connections: Vec<CommonConnectionInfo> = match connection_type {
        ConnectionType::Location => {
            ConnectionInfo::all_by_location_id(&app_state.get_pool(), location_id)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        }
        ConnectionType::Tunnel => {
            TunnelConnectionInfo::all_by_tunnel_id(&app_state.get_pool(), location_id)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        }
    };
    info!("Connections retrieved({})", connections.len());
    trace!("Connections found:\n{connections:#?}");
    Ok(connections)
}

#[tauri::command(async)]
pub async fn all_tunnel_connections(
    location_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<TunnelConnectionInfo>, Error> {
    debug!("Retrieving connections for location {location_id}");
    let connections =
        TunnelConnectionInfo::all_by_tunnel_id(&app_state.get_pool(), location_id).await?;
    info!("Tunnel connections retrieved({})", connections.len());
    trace!("Connections found:\n{connections:#?}");
    Ok(connections)
}

#[tauri::command(async)]
pub async fn active_connection(
    location_id: i64,
    connection_type: ConnectionType,
    handle: AppHandle,
) -> Result<Option<ActiveConnection>, Error> {
    let state = handle.state::<AppState>();
    debug!("Retrieving active connection for location with id: {location_id}");
    debug!("Location found");
    let connection = state.find_connection(location_id, connection_type).await;
    if connection.is_some() {
        debug!("Active connection found");
    }
    trace!("Connection retrieved:\n{connection:#?}");
    info!("Connection retrieved");
    Ok(connection)
}

#[tauri::command(async)]
pub async fn last_connection(
    location_id: i64,
    connection_type: ConnectionType,
    app_state: State<'_, AppState>,
) -> Result<Option<CommonConnection>, Error> {
    debug!("Retrieving last connection for location {location_id} with type {connection_type:?}");
    if connection_type == ConnectionType::Location {
        if let Some(connection) =
            Connection::latest_by_location_id(&app_state.get_pool(), location_id).await?
        {
            info!("Found last connection at {}", connection.end);
            Ok(Some(connection.into()))
        } else {
            Ok(None)
        }
    } else if let Some(connection) =
        TunnelConnection::latest_by_tunnel_id(&app_state.get_pool(), location_id).await?
    {
        info!("Found last connection at {}", connection.end);
        Ok(Some(connection.into()))
    } else {
        info!("No last connection found");
        Ok(None)
    }
}

#[tauri::command(async)]
pub async fn update_location_routing(
    location_id: i64,
    route_all_traffic: bool,
    connection_type: ConnectionType,
    handle: AppHandle,
) -> Result<(), Error> {
    let app_state = handle.state::<AppState>();
    debug!("Updating location routing {location_id} with {connection_type:?}");

    match connection_type {
        ConnectionType::Location => {
            if let Some(mut location) =
                Location::find_by_id(&app_state.get_pool(), location_id).await?
            {
                location.route_all_traffic = route_all_traffic;
                location.save(&app_state.get_pool()).await?;
                info!("Location routing updated for location {location_id}");
                handle.emit_all(
                    LOCATION_UPDATE,
                    Payload {
                        message: "Location routing updated".into(),
                    },
                )?;
                Ok(())
            } else {
                error!(
                    "Couldn't update location routing: location with id {location_id} not found."
                );
                Err(Error::NotFound)
            }
        }
        ConnectionType::Tunnel => {
            if let Some(mut tunnel) = Tunnel::find_by_id(&app_state.get_pool(), location_id).await?
            {
                tunnel.route_all_traffic = route_all_traffic;
                tunnel.save(&app_state.get_pool()).await?;
                info!("Tunnel routing updated for tunnel {location_id}");
                handle.emit_all(
                    LOCATION_UPDATE,
                    Payload {
                        message: "Tunnel routing updated".into(),
                    },
                )?;
                Ok(())
            } else {
                error!("Couldn't update tunnel routing: tunnel with id {location_id} not found.");
                Err(Error::NotFound)
            }
        }
    }
}

#[tauri::command(async)]
pub async fn get_settings(handle: AppHandle) -> Result<Settings, Error> {
    debug!("Retrieving settings");
    let app_state = handle.state::<AppState>();
    let settings = Settings::get(&app_state.get_pool()).await?;
    info!("Settings retrieved");
    Ok(settings)
}

#[tauri::command(async)]
pub async fn update_settings(data: SettingsPatch, handle: AppHandle) -> Result<Settings, Error> {
    let app_state = handle.state::<AppState>();
    let pool = &app_state.get_pool();
    trace!("Pool received");
    let mut settings = Settings::get(pool).await?;
    trace!("Settings read from table");
    settings.apply(data);
    debug!("Saving settings");
    settings.save(pool).await?;
    debug!("Settings saved, reconfiguring tray icon.");
    match configure_tray_icon(&handle, &settings.tray_icon_theme) {
        Ok(()) => {}
        Err(e) => {
            error!(
                "Tray configuration update failed during settings update. err : {}",
                e.to_string()
            );
        }
    }
    debug!("Tray icon updated");
    info!("Settings updated");
    Ok(settings)
}

#[tauri::command(async)]
pub async fn delete_instance(instance_id: i64, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting instance {instance_id}");
    let app_state = handle.state::<AppState>();
    let mut client = app_state.client.clone();
    let pool = &app_state.get_pool();
    if let Some(instance) = Instance::find_by_id(pool, instance_id).await? {
        let instance_locations = Location::find_by_instance_id(pool, instance_id).await?;
        for location in instance_locations {
            if let Some(connection) = app_state
                .find_and_remove_connection(location.id, &ConnectionType::Location)
                .await
            {
                debug!(
                    "Found active connection for location {} ({}), closing...",
                    location.name, location.id
                );
                let request = RemoveInterfaceRequest {
                    interface_name: connection.interface_name.clone(),
                    pre_down: None,
                    post_down: None,
                };
                client.remove_interface(request).await.map_err(|status| {
                        let msg =
                            format!("Error occured while removing interface {} for location {} ({}), status: {status}",
                            connection.interface_name, location.name, location.id
                        );
                        error!("{msg}");
                        Error::InternalError(msg)
                    })?;
                info!("Connection closed and interface removed");
            }
        }
        instance.delete(pool).await?;
    } else {
        error!("Instance {instance_id} not found");
        return Err(Error::NotFound);
    }
    handle.emit_all(INSTANCE_UPDATE, ())?;
    info!("Instance {instance_id}, deleted");
    Ok(())
}

#[tauri::command]
pub fn parse_tunnel_config(config: &str) -> Result<Tunnel, Error> {
    debug!("Parsing config file");
    let tunnel_config = parse_wireguard_config(config).map_err(|error| {
        error!("{error}");
        Error::ConfigParseError(error.to_string())
    })?;
    info!("Config file parsed");
    Ok(tunnel_config)
}

#[tauri::command(async)]
pub async fn save_tunnel(mut tunnel: Tunnel, handle: AppHandle) -> Result<(), Error> {
    let app_state = handle.state::<AppState>();
    debug!("Received tunnel configuration: {tunnel:#?}");
    tunnel.save(&app_state.get_pool()).await?;
    info!("Saved tunnel {tunnel:#?}");
    handle.emit_all(
        LOCATION_UPDATE,
        Payload {
            message: "Tunnel saved".into(),
        },
    )?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub id: Option<i64>,
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub active: bool,
    pub route_all_traffic: bool,
    pub connection_type: ConnectionType,
}

#[tauri::command(async)]
pub async fn all_tunnels(app_state: State<'_, AppState>) -> Result<Vec<TunnelInfo>, Error> {
    debug!("Retrieving all instances.");

    let tunnels = Tunnel::all(&app_state.get_pool()).await?;
    debug!("Found ({}) tunnels", tunnels.len());
    trace!("Tunnels found: {tunnels:#?}");
    let mut tunnel_info = Vec::new();
    let active_tunnel_ids = app_state
        .get_connection_id_by_type(&ConnectionType::Tunnel)
        .await;

    for tunnel in tunnels {
        tunnel_info.push(TunnelInfo {
            id: tunnel.id,
            name: tunnel.name,
            address: tunnel.address,
            endpoint: tunnel.endpoint,
            route_all_traffic: tunnel.route_all_traffic,
            active: active_tunnel_ids.contains(&tunnel.id.expect("Missing Tunnel ID")),
            connection_type: ConnectionType::Tunnel,
        });
    }

    info!("Tunnels retrieved({})", tunnel_info.len());
    Ok(tunnel_info)
}

#[tauri::command(async)]
pub async fn tunnel_details(
    tunnel_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Tunnel, Error> {
    debug!("Retrieving Tunnel with ID {tunnel_id}.");

    if let Some(tunnel) = Tunnel::find_by_id(&app_state.get_pool(), tunnel_id).await? {
        info!("Found tunnel {tunnel_id}");
        Ok(tunnel)
    } else {
        error!("Tunnel with ID: {tunnel_id}, not found");
        Err(Error::NotFound)
    }
}

#[tauri::command(async)]
pub async fn delete_tunnel(tunnel_id: i64, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting tunnel {tunnel_id}");
    let app_state = handle.state::<AppState>();
    let mut client = app_state.client.clone();
    let pool = &app_state.get_pool();
    if let Some(tunnel) = Tunnel::find_by_id(pool, tunnel_id).await? {
        if let Some(connection) = app_state
            .find_and_remove_connection(tunnel_id, &ConnectionType::Tunnel)
            .await
        {
            debug!("Found active connection for tunnel({tunnel_id}), closing...",);
            let request = RemoveInterfaceRequest {
                interface_name: connection.interface_name.clone(),
                pre_down: tunnel.pre_down.clone(),
                post_down: tunnel.post_up.clone(),
            };
            client
                .remove_interface(request)
                .await
                .map_err(|status| {
                    let msg =
                        format!("Error occured while removing interface {} for tunnel {tunnel_id}, status: {status}",
                        connection.interface_name
                    );
                    error!("{msg}");
                    Error::InternalError(msg)
                })?;
            info!("Connection closed and interface removed");
        }
        tunnel.delete(pool).await?;
    } else {
        error!("Tunnel {tunnel_id} not found");
        return Err(Error::NotFound);
    }
    info!("Tunnel {tunnel_id}, deleted");
    Ok(())
}

#[tauri::command]
pub fn open_link(link: &str) -> Result<(), Error> {
    match webbrowser::open(link) {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::CommandError(e.to_string())),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppVersionInfo {
    pub version: String,
    pub release_date: String,
    pub release_notes_url: String,
    pub update_url: String,
}

static PRODUCT_NAME: &str = "defguard-client";

#[tauri::command(async)]
pub async fn get_latest_app_version(handle: AppHandle) -> Result<AppVersionInfo, Error> {
    let app_version = handle.package_info().version.to_string();
    let current_version = app_version.as_str();
    let operating_system = env::consts::OS;

    let mut request_data = HashMap::new();
    request_data.insert("product", PRODUCT_NAME);
    request_data.insert("client_version", current_version);
    request_data.insert("operating_system", operating_system);

    info!("Fetching latest application version with args: current version {current_version} and operating system {operating_system}");

    let client = reqwest::Client::new();
    let res = client
        .post("https://pkgs.defguard.net/api/update/check")
        .json(&request_data)
        .send()
        .await;

    if let Ok(response) = res {
        let response_json = response.json::<AppVersionInfo>().await;

        let response = response_json.map_err(|err| {
            error!("Failed to deserialize latest application version response {err}");
            Error::CommandError(err.to_string())
        })?;

        info!("Latest application version fetched: {}", response.version);
        Ok(response)
    } else {
        let err = res.err().unwrap();
        error!("Failed to fetch latest application version {err}");
        Err(Error::CommandError(err.to_string()))
    }
}
