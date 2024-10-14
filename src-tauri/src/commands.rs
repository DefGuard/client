use std::{
    collections::{HashMap, HashSet},
    env,
    str::FromStr,
};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};
use struct_patch::Patch;
use tauri::{AppHandle, Manager, State};

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
    enterprise::periodic::config::poll_instance,
    error::Error,
    events::{CONNECTION_CHANGED, INSTANCE_UPDATE, LOCATION_UPDATE},
    log_watcher::{
        global_log_watcher::{spawn_global_log_watcher_task, stop_global_log_watcher_task},
        service_log_watcher::stop_log_watcher_task,
    },
    proto::{DeviceConfig, DeviceConfigResponse},
    service::proto::RemoveInterfaceRequest,
    tray::{configure_tray_icon, reload_tray_menu},
    utils::{
        disconnect_interface, execute_command, get_location_interface_details,
        get_tunnel_interface_details, get_tunnel_or_location_name, handle_connection_for_location,
        handle_connection_for_tunnel,
    },
    wg_config::parse_wireguard_config,
    CommonConnection, CommonConnectionInfo, CommonLocationStats, ConnectionType,
};

#[derive(Clone, serde::Serialize)]
pub struct Payload {
    pub message: String,
}

// Create new WireGuard interface
#[tauri::command(async)]
pub async fn connect(
    location_id: Id,
    connection_type: ConnectionType,
    preshared_key: Option<String>,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Received a command to connect to a {connection_type} with ID {location_id}");
    let state = handle.state::<AppState>();
    if connection_type == ConnectionType::Location {
        if let Some(location) = Location::find_by_id(&state.get_pool(), location_id).await? {
            debug!(
                "Identified location with ID {location_id} as \"{}\", handling connection...",
                location.name
            );
            handle_connection_for_location(&location, preshared_key, handle.clone()).await?;
            reload_tray_menu(&handle).await;
            info!("Connected to location {} successfully", location.name);
        } else {
            error!("Location with ID {location_id} not found in the database, aborting connection attempt");
            return Err(Error::NotFound);
        }
    } else if let Some(tunnel) = Tunnel::find_by_id(&state.get_pool(), location_id).await? {
        debug!(
            "Identified tunnel with ID {location_id} as \"{}\", handling connection...",
            tunnel.name
        );
        handle_connection_for_tunnel(&tunnel, handle).await?;
        info!("Connected to tunnel {} successfully", tunnel.name);
    } else {
        error!("Tunnel {location_id} not found");
        return Err(Error::NotFound);
    }
    Ok(())
}

#[tauri::command(async)]
pub async fn start_global_logwatcher(handle: AppHandle) -> Result<(), Error> {
    let result = spawn_global_log_watcher_task(&handle, tracing::Level::DEBUG).await;
    if let Err(err) = result {
        error!("Error while spawning the global log watcher task: {}", err)
    }
    Ok(())
}

#[tauri::command(async)]
pub async fn stop_global_logwatcher(handle: AppHandle) -> Result<(), Error> {
    stop_global_log_watcher_task(&handle)
}

#[tauri::command]
pub async fn disconnect(
    location_id: Id,
    connection_type: ConnectionType,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Received a command to disconnect from a {connection_type} with ID {location_id}");
    let state = handle.state::<AppState>();

    let name = match get_tunnel_or_location_name(location_id, connection_type, &state).await {
        Some(name) => {
            debug!("Identified {connection_type} with ID {location_id} as \"{name}\", handling disconnection...");
            name
        }
        None => {
            debug!("Could not identify {connection_type} with ID {location_id}, this {connection_type} will be referred to as UNKNOWN, trying to disconnect anyway...");
            "UNKNOWN".to_string()
        }
    };

    if let Some(connection) = state.remove_connection(location_id, &connection_type).await {
        debug!("Found active connection for {connection_type} {name}({location_id})");
        trace!("Connection: {connection:?}");
        disconnect_interface(&connection, &state).await?;
        debug!("Emitting the event informing the frontend about the disconnection from {connection_type} {name}({location_id})");
        handle.emit_all(
            CONNECTION_CHANGED,
            Payload {
                message: "Created new connection".into(),
            },
        )?;
        debug!("Event emitted successfully");
        stop_log_watcher_task(&handle, &connection.interface_name)?;
        reload_tray_menu(&handle).await;
        if connection_type == ConnectionType::Location {
            if let Err(err) = maybe_update_instance_config(location_id, &handle).await {
                warn!("Failed to update instance for location {location_id}: {err}");
            };
        }

        info!("Finished disconnecting from {connection_type} {name}({location_id})");
        Ok(())
    } else {
        error!("Error while disconnecting from {connection_type} {name}({location_id}): connection not found");
        Err(Error::NotFound)
    }
}

/// Triggers poll on location's instance config. Config will be updated if there are no more active
/// connections for this instance.
async fn maybe_update_instance_config(location_id: Id, handle: &AppHandle) -> Result<(), Error> {
    let state: State<'_, AppState> = handle.state();
    let mut transaction = state.get_pool().begin().await?;
    let Some(location) = Location::find_by_id(&mut *transaction, location_id).await? else {
        error!("Location {location_id} not found, skipping config update check");
        return Err(Error::NotFound);
    };
    let Some(mut instance) = Instance::find_by_id(&mut *transaction, location.instance_id).await?
    else {
        error!(
            "Instance {} not found, skipping config update check",
            location.instance_id
        );
        return Err(Error::NotFound);
    };
    poll_instance(&mut transaction, &mut instance, handle).await?;
    transaction.commit().await?;
    handle.emit_all(INSTANCE_UPDATE, ())?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: Id,
    pub name: String,
    pub pubkey: String,
    pub user_id: Id,
    pub created_at: i64,
}

#[must_use]
pub fn device_config_to_location(device_config: DeviceConfig, instance_id: Id) -> Location<NoId> {
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
    debug!("Saving device configuration: {response:#?}.");

    let mut transaction = app_state.get_pool().begin().await?;
    let instance_info = response
        .instance
        .expect("Missing instance info in device config response");
    let mut instance: Instance = instance_info.into();
    if response.token.is_some() {
        info!("Received polling token for instance {}", instance.name);
    } else {
        warn!(
            "Missing polling token for instance {}, core and/or proxy services may need an update, configuration polling won't work",
            instance.name,
        );
    }
    instance.token = response.token;

    debug!("Saving instance {}", instance.name);
    let instance = instance.save(&mut *transaction).await?;
    info!("Saved instance {}", instance.name);

    let device = response
        .device
        .expect("Missing device info in device config response");
    let keys = WireguardKeys::new(instance.id, device.pubkey, private_key);
    debug!(
        "Saving wireguard key {} for instance {}({})",
        keys.pubkey, instance.name, instance.id
    );
    let keys = keys.save(&mut *transaction).await?;
    info!(
        "Saved wireguard key {} for instance {}({})",
        keys.pubkey, instance.name, instance.id
    );
    for location in response.configs {
        let new_location = device_config_to_location(location, instance.id);
        debug!(
            "Saving location {} for instance {}({})",
            new_location.name, instance.name, instance.id
        );
        let new_location = new_location.save(&mut *transaction).await?;
        info!(
            "Saved location {} for instance {}({})",
            new_location.name, instance.name, instance.id
        );
    }
    transaction.commit().await?;
    info!("Instance {}({:?}) created.", instance.name, instance.id);
    trace!("Created following instance: {instance:#?}");
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance.id).await?;
    trace!("Created following locations: {locations:#?}");
    handle.emit_all(INSTANCE_UPDATE, ())?;
    info!(
        "Device configuration saved for instance {}({})",
        instance.name, instance.id,
    );
    let res: SaveDeviceConfigResponse = SaveDeviceConfigResponse {
        locations,
        instance,
    };
    reload_tray_menu(&handle).await;
    Ok(res)
}

#[tauri::command(async)]
pub async fn all_instances(app_state: State<'_, AppState>) -> Result<Vec<InstanceInfo<Id>>, Error> {
    debug!("Retrieving all instances.");

    let instances = Instance::all(&app_state.get_pool()).await?;
    debug!("Found ({}) instances", instances.len());
    trace!("Instances found: {instances:#?}");
    let mut instance_info = Vec::new();
    let connection_ids = app_state
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
            id: instance.id,
            uuid: instance.uuid,
            name: instance.name,
            url: instance.url,
            proxy_url: instance.proxy_url,
            active: connected,
            pubkey: keys.pubkey,
            disable_all_traffic: instance.disable_all_traffic,
            enterprise_enabled: instance.enterprise_enabled,
        });
    }
    debug!("Instances retrieved({})", instance_info.len());
    trace!("Returning following instances: {instance_info:#?}");
    Ok(instance_info)
}

#[derive(Serialize, Debug)]
pub struct LocationInfo {
    pub id: Id,
    pub instance_id: Id,
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub active: bool,
    pub route_all_traffic: bool,
    pub connection_type: ConnectionType,
    pub pubkey: String,
    pub mfa_enabled: bool,
    pub network_id: Id,
}

#[tauri::command(async)]
pub async fn all_locations(
    instance_id: Id,
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
    debug!("Locations retrieved({})", location_info.len());
    debug!(
        "Returning {} locations for instance {instance_id}",
        location_info.len(),
    );
    trace!("Locations returned:\n{location_info:#?}");

    Ok(location_info)
}

#[derive(Serialize, Debug)]
pub struct LocationInterfaceDetails {
    pub location_id: Id,
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
    location_id: Id,
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
    instance_id: Id,
    response: DeviceConfigResponse,
    app_state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), Error> {
    debug!("Received update_instance command");
    trace!("Processing following response:\n {response:#?}");
    let pool = app_state.get_pool();

    if let Some(mut instance) = Instance::find_by_id(&pool, instance_id).await? {
        let mut transaction = pool.begin().await?;
        do_update_instance(&mut transaction, &mut instance, response).await?;
        transaction.commit().await?;

        app_handle.emit_all(INSTANCE_UPDATE, ())?;
        reload_tray_menu(&app_handle).await;
        Ok(())
    } else {
        error!("Instance with id {instance_id} not found");
        Err(Error::NotFound)
    }
}

/// Returns true if configuration in instance_info differs from current configuration
pub async fn locations_changed(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &Instance<Id>,
    device_config: &DeviceConfigResponse,
) -> Result<bool, Error> {
    let db_locations: Vec<Location<NoId>> =
        Location::find_by_instance_id(transaction.as_mut(), instance.id)
            .await?
            .into_iter()
            .map(Location::<NoId>::from)
            .collect();
    let db_locations: HashSet<Location<NoId>> = HashSet::from_iter(db_locations);
    let core_locations: Vec<Location<NoId>> = device_config
        .configs
        .iter()
        .map(|config| device_config_to_location(config.clone(), instance.id))
        .map(Location::<NoId>::from)
        .collect();
    let core_locations: HashSet<Location<NoId>> = HashSet::from_iter(core_locations);

    Ok(db_locations != core_locations)
}

pub async fn do_update_instance(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
    response: DeviceConfigResponse,
) -> Result<(), Error> {
    // update instance
    debug!("Updating instance {}({}).", instance.name, instance.id);
    let locations_changed = locations_changed(transaction, instance, &response).await?;
    let instance_info = response
        .instance
        .expect("Missing instance info in device config response");
    instance.name = instance_info.name;
    instance.url = instance_info.url;
    instance.proxy_url = instance_info.proxy_url;
    instance.username = instance_info.username;
    // Make sure to update the locations too if we are disabling all traffic
    if instance.disable_all_traffic != instance_info.disable_all_traffic
        && instance_info.disable_all_traffic
    {
        debug!(
            "Disabling all traffic for all locations of instance {}({}).",
            instance.name, instance.id
        );
        Location::disable_all_traffic_for_all(transaction.as_mut(), instance.id).await?;
        debug!(
            "Disabled all traffic for all locations of instance {}({}).",
            instance.name, instance.id
        );
    }
    instance.disable_all_traffic = instance_info.disable_all_traffic;
    instance.enterprise_enabled = instance_info.enterprise_enabled;
    // Token may be empty if it was not issued
    // This happens during polling, as core doesn't issue a new token for polling request
    if response.token.is_some() {
        instance.token = response.token;
        info!("Set polling token for instance {}", instance.name);
    } else {
        debug!(
            "No polling token received for instance {}, not updating",
            instance.name
        );
    }
    instance.save(transaction.as_mut()).await?;
    debug!(
        "Instance {}({}) main config applied from core's response.",
        instance.name, instance.id
    );

    // check if locations have changed
    if locations_changed {
        // process locations received in response
        debug!(
            "Updating locations for instance {}({}).",
            instance.name, instance.id
        );
        // fetch existing locations for given instance
        let mut current_locations =
            Location::find_by_instance_id(transaction.as_mut(), instance.id).await?;
        for location in response.configs {
            // parse device config
            let new_location = device_config_to_location(location, instance.id);

            // check if location is already present in current locations
            if let Some(position) = current_locations
                .iter()
                .position(|loc| loc.network_id == new_location.network_id)
            {
                // remove from list of existing locations
                let mut current_location = current_locations.remove(position);
                debug!(
                    "Updating existing location {}({}) for instance {}({}).",
                    current_location.name, current_location.id, instance.name, instance.id,
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
                current_location.save(transaction.as_mut()).await?;
                info!(
                    "Location {}({}) updated for instance {}({}).",
                    current_location.name, current_location.id, instance.name, instance.id,
                );
            } else {
                // create new location
                debug!(
                    "Creating new location for instance {}({}).",
                    instance.name, instance.id
                );
                let new_location = new_location.save(transaction.as_mut()).await?;
                info!(
                    "New location {}({}) created for instance {}({})",
                    new_location.name, new_location.id, instance.name, instance.id
                );
            }
        }
        info!(
            "Locations updated for instance {}({}).",
            instance.name, instance.id
        );

        // remove locations which were present in current locations
        // but no longer found in core response
        debug!(
            "Removing locations for instance {}({}).",
            instance.name, instance.id
        );
        for removed_location in current_locations {
            removed_location.delete(transaction.as_mut()).await?;
        }
        info!(
            "Locations removed for instance {}({}).",
            instance.name, instance.id
        );
    } else {
        info!(
            "Locations for instance {}({}) didn't change. Not updating them.",
            instance.name, instance.id
        );
    }
    info!(
        "Instance {}({}) update is done.",
        instance.name, instance.id
    );
    Ok(())
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
    location_id: Id,
    connection_type: ConnectionType,
    from: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<CommonLocationStats<Id>>, Error> {
    trace!("Location stats command received");
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats = match connection_type {
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
    location_id: Id,
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
    debug!("Connections retrieved({})", connections.len());
    trace!("Connections found:\n{connections:#?}");
    Ok(connections)
}

#[tauri::command(async)]
pub async fn all_tunnel_connections(
    location_id: Id,
    app_state: State<'_, AppState>,
) -> Result<Vec<TunnelConnectionInfo>, Error> {
    debug!("Retrieving connections for location {location_id}");
    let connections =
        TunnelConnectionInfo::all_by_tunnel_id(&app_state.get_pool(), location_id).await?;
    debug!("Tunnel connections retrieved({})", connections.len());
    trace!("Connections found:\n{connections:#?}");
    Ok(connections)
}

#[tauri::command(async)]
pub async fn active_connection(
    location_id: Id,
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
    debug!("Active connection information for location has been retrieved");
    Ok(connection)
}

#[tauri::command(async)]
pub async fn last_connection(
    location_id: Id,
    connection_type: ConnectionType,
    app_state: State<'_, AppState>,
) -> Result<Option<CommonConnection<Id>>, Error> {
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
    location_id: Id,
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
                // Check if the instance has route_all_traffic disabled
                let instance = Instance::find_by_id(&app_state.get_pool(), location.instance_id)
                    .await?
                    .ok_or(Error::NotFound)?;
                if instance.disable_all_traffic && route_all_traffic {
                    error!(
                        "Couldn't update location routing: instance with id {} has route_all_traffic disabled.", instance.id
                    );
                    return Err(Error::InternalError(
                        "Instance has route_all_traffic disabled".into(),
                    ));
                }

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
    debug!("Settings retrieved");
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
pub async fn delete_instance(instance_id: Id, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting instance {instance_id}");
    let app_state = handle.state::<AppState>();
    let mut client = app_state.client.clone();
    let pool = &app_state.get_pool();
    let Some(instance) = Instance::find_by_id(pool, instance_id).await? else {
        error!("Instance {instance_id} not found");
        return Err(Error::NotFound);
    };

    let instance_locations = Location::find_by_instance_id(pool, instance_id).await?;
    for location in instance_locations {
        if let Some(connection) = app_state
            .remove_connection(location.id, &ConnectionType::Location)
            .await
        {
            debug!(
                "Found active connection for location {} ({}), closing...",
                location.name, location.id
            );
            let request = RemoveInterfaceRequest {
                interface_name: connection.interface_name.clone(),
                endpoint: location.endpoint,
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
    reload_tray_menu(&handle).await;

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
pub async fn update_tunnel(mut tunnel: Tunnel<Id>, handle: AppHandle) -> Result<(), Error> {
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

#[tauri::command(async)]
pub async fn save_tunnel(tunnel: Tunnel<NoId>, handle: AppHandle) -> Result<(), Error> {
    let app_state = handle.state::<AppState>();
    debug!("Received tunnel configuration: {tunnel:#?}");
    let tunnel = tunnel.save(&app_state.get_pool()).await?;
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
pub struct TunnelInfo<I = NoId> {
    pub id: I,
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub active: bool,
    pub route_all_traffic: bool,
    pub connection_type: ConnectionType,
}

#[tauri::command(async)]
pub async fn all_tunnels(app_state: State<'_, AppState>) -> Result<Vec<TunnelInfo<Id>>, Error> {
    debug!("Retrieving all tunnels.");

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
            active: active_tunnel_ids.contains(&tunnel.id),
            connection_type: ConnectionType::Tunnel,
        });
    }

    debug!("All tunnels retrieved.");
    Ok(tunnel_info)
}

#[tauri::command(async)]
pub async fn tunnel_details(
    tunnel_id: Id,
    app_state: State<'_, AppState>,
) -> Result<Tunnel<Id>, Error> {
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
pub async fn delete_tunnel(tunnel_id: Id, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting tunnel {tunnel_id}");
    let app_state = handle.state::<AppState>();
    let mut client = app_state.client.clone();
    let pool = &app_state.get_pool();
    let Some(tunnel) = Tunnel::find_by_id(pool, tunnel_id).await? else {
        error!("Tunnel {tunnel_id} not found");
        return Err(Error::NotFound);
    };

    if let Some(connection) = app_state
        .remove_connection(tunnel_id, &ConnectionType::Tunnel)
        .await
    {
        debug!("Found active connection for tunnel({tunnel_id}), closing...",);
        if let Some(pre_down) = &tunnel.pre_down {
            debug!("Executing specified PreDown command: {pre_down}");
            let _ = execute_command(pre_down);
            info!("Executed specified PreDown command: {pre_down}");
        }
        let request = RemoveInterfaceRequest {
            interface_name: connection.interface_name.clone(),
            endpoint: tunnel.endpoint.clone(),
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
        if let Some(post_down) = &tunnel.post_down {
            debug!("Executing specified PostDown command: {post_down}");
            let _ = execute_command(post_down);
            info!("Executed specified PostDown command: {post_down}");
        }
        info!("Connection closed and interface removed");
    }
    tunnel.delete(pool).await?;

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

    debug!("Fetching latest application version with args: current version {current_version} and operating system {operating_system}");

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

        info!(
            "The latest application release version avaialble for download is: {}",
            response.version
        );
        Ok(response)
    } else {
        let err = res.err().unwrap();
        error!("Failed to fetch latest application version {err}");
        Err(Error::CommandError(err.to_string()))
    }
}
