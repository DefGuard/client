use core::fmt;
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
    app_config::{AppConfig, AppConfigPatch},
    appstate::AppState,
    database::models::{
        connection::{ActiveConnection, Connection, ConnectionInfo},
        instance::{Instance, InstanceInfo},
        location::Location,
        location_stats::LocationStats,
        tunnel::{Tunnel, TunnelConnection, TunnelConnectionInfo, TunnelStats},
        wireguard_keys::WireguardKeys,
        Id, NoId,
    },
    enterprise::periodic::config::poll_instance,
    error::Error,
    events::{APPLICATION_CONFIG_CHANGED, CONNECTION_CHANGED, INSTANCE_UPDATE, LOCATION_UPDATE},
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
        handle_connection_for_tunnel, verify_connection, ConnectionToVerify,
    },
    wg_config::parse_wireguard_config,
    CommonConnection, CommonConnectionInfo, CommonLocationStats, ConnectionType,
};

#[derive(Clone, Serialize)]
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
    let pool = state.get_pool();
    if connection_type == ConnectionType::Location {
        if let Some(location) = Location::find_by_id(&pool, location_id).await? {
            debug!(
                "Identified location with ID {location_id} as \"{}\", handling connection...",
                location.name
            );
            handle_connection_for_location(&location, preshared_key, handle.clone()).await?;
            reload_tray_menu(&handle).await;
            info!("Connected to location {location}");
            // verify if connection is alive
            tauri::async_runtime::spawn(verify_connection(
                handle.clone(),
                ConnectionToVerify::Location(location),
            ));
            debug!("Connection verification task spawned.");
        } else {
            error!("Location with ID {location_id} not found in the database, aborting connection attempt");
            return Err(Error::NotFound);
        }
    } else if let Some(tunnel) = Tunnel::find_by_id(&pool, location_id).await? {
        debug!(
            "Identified tunnel with ID {location_id} as \"{}\", handling connection...",
            tunnel.name
        );
        handle_connection_for_tunnel(&tunnel, handle.clone()).await?;
        info!("Successfully connected to tunnel {tunnel}");
        // verify if connection is alive
        tauri::async_runtime::spawn(verify_connection(
            handle.clone(),
            ConnectionToVerify::Tunnel(tunnel),
        ));
        debug!("Connection verification task spawned.");
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
        error!("Error while spawning the global log watcher task: {err}");
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
    let state = handle.state::<AppState>();
    let name = get_tunnel_or_location_name(location_id, connection_type, &state).await;
    debug!("Received a command to disconnect from the {connection_type} {name}({location_id})");

    debug!("Removing active connection for {connection_type} {name}({location_id}) from the application's state, if it exists...");
    if let Some(connection) = state.remove_connection(location_id, &connection_type).await {
        debug!("Found and removed active connection from the application's state for {connection_type} {name}({location_id})");
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
            let name = get_tunnel_or_location_name(location_id, connection_type, &state).await;
            if let Err(err) = maybe_update_instance_config(location_id, &handle).await {
                match err {
                    Error::CoreNotEnterprise => {
                        debug!(
                            "Tried to fetch instance config from core after disconnecting from {name}(ID: {location_id}), but the core is not enterprise, so we can't fetch the config."
                        );
                    }
                    Error::NoToken => {
                        debug!(
                            "Tried to fetch instance config from core after disconnecting from {name}(ID: {location_id}), but this location's instance has no polling token, so we can't fetch the config."
                        );
                    }
                    _ => {
                        warn!("Error while trying to fetch instance config after disconnecting from {name}(ID: {location_id}): {err}");
                    }
                }
            };
        }
        info!("Disconnected from {connection_type} {name}(ID: {location_id})");
        Ok(())
    } else {
        warn!("Couldn't disconnect from {connection_type} {name}(ID: {location_id}), as no active connection was found.");
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
        debug!("The newly saved device config has a polling token, automatic configuration polling will be possible if the core has an enterprise license.");
    } else {
        warn!(
            "Missing polling token for instance {}, core and/or proxy services may need an update, configuration polling won't work",
            instance.name,
        );
    }
    instance.token = response.token;

    debug!("Saving instance {}", instance.name);
    let instance = instance.save(&mut *transaction).await?;
    debug!("Saved instance {}", instance.name);

    let device = response
        .device
        .expect("Missing device info in device config response");
    let keys = WireguardKeys::new(instance.id, device.pubkey, private_key);
    debug!(
        "Saving wireguard key {} for instance {}({})",
        keys.pubkey, instance.name, instance.id
    );
    let keys = keys.save(&mut *transaction).await?;
    debug!(
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
        debug!(
            "Saved location {} for instance {}({})",
            new_location.name, instance.name, instance.id
        );
    }
    transaction.commit().await?;
    info!("New instance {} created.", instance);
    trace!("Created following instance: {instance:#?}");
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance.id).await?;
    trace!("Created following locations: {locations:#?}");
    handle.emit_all(INSTANCE_UPDATE, ())?;
    let res: SaveDeviceConfigResponse = SaveDeviceConfigResponse {
        locations,
        instance,
    };
    reload_tray_menu(&handle).await;
    Ok(res)
}

#[tauri::command(async)]
pub async fn all_instances(app_state: State<'_, AppState>) -> Result<Vec<InstanceInfo<Id>>, Error> {
    debug!("Getting information about all instances.");
    let instances = Instance::all(&app_state.get_pool()).await?;
    trace!(
        "Found {} instances to return information about.",
        instances.len()
    );
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
    debug!(
        "Returning information about {} instances",
        instance_info.len()
    );
    trace!("Returning following instances information: {instance_info:#?}");
    Ok(instance_info)
}

#[derive(Debug, Serialize)]
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

impl fmt::Display for LocationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[tauri::command(async)]
pub async fn all_locations(
    instance_id: Id,
    app_state: State<'_, AppState>,
) -> Result<Vec<LocationInfo>, Error> {
    let Some(instance) = Instance::find_by_id(&app_state.get_pool(), instance_id).await? else {
        error!("Tried to get all locations for the instance with ID {instance_id}, but the instance was not found.");
        return Err(Error::NotFound);
    };
    trace!(
        "Getting information about all locations for instance {}.",
        instance.name
    );
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance_id).await?;
    trace!(
        "Found {} locations for instance {instance} to return information about.",
        locations.len()
    );
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
    trace!(
        "Returning information about {} locations for instance {instance}",
        location_info.len()
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
    debug!("Received command to update instance with id {instance_id}");
    trace!("Processing following response:\n {response:#?}");
    let pool = app_state.get_pool();

    if let Some(mut instance) = Instance::find_by_id(&pool, instance_id).await? {
        debug!("The instance with id {instance_id} to update was found: {instance}");
        let mut transaction = pool.begin().await?;
        do_update_instance(&mut transaction, &mut instance, response).await?;
        transaction.commit().await?;

        app_handle.emit_all(INSTANCE_UPDATE, ())?;
        reload_tray_menu(&app_handle).await;
        Ok(())
    } else {
        error!("Instance to update with id {instance_id} was not found, aborting update");
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
            // ignore route_all_traffic flag as core does not have it
            .map(|mut location| {
                location.route_all_traffic = false;
                location
            })
            .collect();
    let db_locations: HashSet<Location<NoId>> = HashSet::from_iter(db_locations);
    let core_locations: Vec<Location<NoId>> = device_config
        .configs
        .iter()
        .map(|config| device_config_to_location(config.clone(), instance.id))
        .map(Location::<NoId>::from)
        // just to make sure we are really on the same page
        .map(|mut location| {
            location.route_all_traffic = false;
            location
        })
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
    debug!("Updating instance {}", instance);
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
            "Disabling all traffic for all locations of instance {}.",
            instance
        );
        Location::disable_all_traffic_for_all(transaction.as_mut(), instance.id).await?;
        debug!(
            "Disabled all traffic for all locations of instance {}.",
            instance
        );
    }
    instance.disable_all_traffic = instance_info.disable_all_traffic;
    instance.enterprise_enabled = instance_info.enterprise_enabled;
    // Token may be empty if it was not issued
    // This happens during polling, as core doesn't issue a new token for polling request
    if response.token.is_some() {
        instance.token = response.token;
        debug!("Set polling token for instance {}", instance.name);
    } else {
        debug!(
            "No polling token received for instance {}, not updating",
            instance.name
        );
    }
    instance.save(transaction.as_mut()).await?;
    debug!(
        "A new base configuration has been applied to instance {}, even if nothing changed",
        instance
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
                    "Location {} configuration updated for instance {}",
                    current_location, instance,
                );
            } else {
                // create new location
                debug!(
                    "Creating new location {new_location} for instance {}",
                    instance
                );
                let new_location = new_location.save(transaction.as_mut()).await?;
                info!(
                    "New location {} created for instance {}",
                    new_location, instance
                );
            }
        }

        // remove locations which were present in current locations
        // but no longer found in core response
        debug!("Removing locations for instance {}.", instance);
        for removed_location in current_locations {
            removed_location.delete(transaction.as_mut()).await?;
            info!(
                "Removed location {} for instance {} during instance update",
                removed_location, instance
            );
        }
        debug!("Finished updating locations for instance {}", instance);
    } else {
        info!(
            "Locations for instance {} didn't change. Not updating them.",
            instance
        );
    }
    Ok(())
}

/// If `datetime` is Some, parses the date string, otherwise returns `DateTime` one hour ago.
pub(crate) fn parse_timestamp(from: Option<String>) -> Result<DateTime<Utc>, Error> {
    Ok(match from {
        Some(from) => DateTime::<Utc>::from_str(&from).map_err(|_| Error::Datetime)?,
        None => Utc::now() - Duration::hours(1),
    })
}

pub(crate) enum DateTimeAggregation {
    Hour,
    Second,
}

impl DateTimeAggregation {
    /// Returns database format string for a given aggregation variant.
    #[must_use]
    pub(crate) fn fstring(&self) -> &'static str {
        match self {
            Self::Hour => "%Y-%m-%d %H:00:00",
            Self::Second => "%Y-%m-%d %H:%M:%S",
        }
    }
}

pub(crate) fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, Error> {
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
            None,
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
    let name = get_tunnel_or_location_name(location_id, connection_type, &state).await;
    debug!(
        "Checking if there is an active connection for location {}(ID: {})",
        name, location_id
    );
    let connection = state.find_connection(location_id, connection_type).await;
    if connection.is_some() {
        debug!("Found active connection for location {name}(ID: {location_id})");
    }
    trace!("Connection retrieved:\n{connection:#?}");
    debug!(
        "Active connection information for location {name}(ID: {location_id}) has been found, returning connection information",
    );
    Ok(connection)
}

#[tauri::command(async)]
pub async fn last_connection(
    location_id: Id,
    connection_type: ConnectionType,
    app_state: State<'_, AppState>,
) -> Result<Option<CommonConnection<Id>>, Error> {
    let name = get_tunnel_or_location_name(location_id, connection_type, &app_state).await;

    debug!(
        "Retrieving last connection information for {connection_type} {name}(ID: {location_id})"
    );
    if connection_type == ConnectionType::Location {
        if let Some(connection) =
            Connection::latest_by_location_id(&app_state.get_pool(), location_id).await?
        {
            debug!(
                "Last connection to {connection_type} {name} has been made at {}",
                connection.end
            );
            Ok(Some(connection.into()))
        } else {
            debug!("No previous connections to {connection_type} {name} have been found.");
            Ok(None)
        }
    } else if let Some(connection) =
        TunnelConnection::latest_by_tunnel_id(&app_state.get_pool(), location_id).await?
    {
        debug!(
            "Last connection to {connection_type} {name} has been made at {}",
            connection.end
        );
        Ok(Some(connection.into()))
    } else {
        debug!("No previous connections to {connection_type} {name} have been found.");
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
    debug!("Updating location routing {location_id} with {connection_type}");
    let name = get_tunnel_or_location_name(location_id, connection_type, &app_state).await;

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
                debug!("Location routing updated for location {name}(ID: {location_id})");
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
pub async fn delete_instance(instance_id: Id, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting instance with ID {instance_id}");
    let app_state = handle.state::<AppState>();
    let mut client = app_state.client.clone();
    let pool = &app_state.get_pool();
    let Some(instance) = Instance::find_by_id(pool, instance_id).await? else {
        error!("Couldn't delete instance: instance with ID {instance_id} could not be found.");
        return Err(Error::NotFound);
    };
    debug!("The instance that is being deleted has been identified as {instance}");

    let instance_locations = Location::find_by_instance_id(pool, instance_id).await?;
    if !instance_locations.is_empty() {
        debug!(
            "Found locations associated with the instance {instance}, closing their connections..."
        );
    }
    for location in instance_locations {
        if let Some(connection) = app_state
            .remove_connection(location.id, &ConnectionType::Location)
            .await
        {
            debug!("Found active connection for location {location}, closing...");
            let request = RemoveInterfaceRequest {
                interface_name: connection.interface_name.clone(),
                endpoint: location.endpoint.clone(),
            };
            client.remove_interface(request).await.map_err(|status| {
                    error!("Error occurred while removing interface {} for location {location}, status: {status}", connection.interface_name);
                    Error::InternalError(format!(
                        "There was an error while removing interface for location {location}, error message: {}. Check logs for more details.", status.message()
                    ))
                })?;
            info!("The connection to location {location} has been closed, as it was associated with the instance {instance} that is being deleted.");
        }
    }
    instance.delete(pool).await?;
    reload_tray_menu(&handle).await;

    handle.emit_all(INSTANCE_UPDATE, ())?;
    info!("Successfully deleted instance {instance}.");
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
    debug!("Received tunnel configuration to update: {tunnel:?}");
    tunnel.save(&app_state.get_pool()).await?;
    info!("The tunnel {tunnel} configuration has been updated.");
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
    debug!("Received tunnel configuration to save: {tunnel:?}");
    let tunnel = tunnel.save(&app_state.get_pool()).await?;
    info!("The tunnel {tunnel} configuration has been saved.");
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
    trace!("Getting information about all tunnels");

    let tunnels = Tunnel::all(&app_state.get_pool()).await?;
    trace!("Found ({}) tunnels to get information about", tunnels.len());
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

    trace!(
        "Returning information about all ({}) tunnels",
        tunnel_info.len()
    );
    Ok(tunnel_info)
}

#[tauri::command(async)]
pub async fn tunnel_details(
    tunnel_id: Id,
    app_state: State<'_, AppState>,
) -> Result<Tunnel<Id>, Error> {
    debug!("Retrieving details about tunnel with ID {tunnel_id}.");

    if let Some(tunnel) = Tunnel::find_by_id(&app_state.get_pool(), tunnel_id).await? {
        debug!("The tunnel {tunnel} has been found, returning its details.");
        Ok(tunnel)
    } else {
        error!("Tunnel with ID {tunnel_id} not found, cannot retrieve its details.");
        Err(Error::NotFound)
    }
}

#[tauri::command(async)]
pub async fn delete_tunnel(tunnel_id: Id, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting tunnel with ID {tunnel_id}");
    let app_state = handle.state::<AppState>();
    let mut client = app_state.client.clone();
    let pool = &app_state.get_pool();
    let Some(tunnel) = Tunnel::find_by_id(pool, tunnel_id).await? else {
        error!("The tunnel to delete with ID {tunnel_id} could not be found, cannot delete.");
        return Err(Error::NotFound);
    };
    debug!("The tunnel to delete with ID {tunnel_id} has been identified as {tunnel}, proceeding with deletion.");

    if let Some(connection) = app_state
        .remove_connection(tunnel_id, &ConnectionType::Tunnel)
        .await
    {
        debug!("Found active connection for tunnel {tunnel} which is being deleted, closing the connection.");
        if let Some(pre_down) = &tunnel.pre_down {
            debug!("Executing defined PreDown command before removing the interface {} for the tunnel {tunnel}: {pre_down}", connection.interface_name);
            let _ = execute_command(pre_down);
            info!("Executed defined PreDown command before removing the interface {} for the tunnel {tunnel}: {pre_down}", connection.interface_name);
        }
        let request = RemoveInterfaceRequest {
            interface_name: connection.interface_name.clone(),
            endpoint: tunnel.endpoint.clone(),
        };
        client
            .remove_interface(request)
            .await
            .map_err(|status| {
                error!("An error occurred while removing interface {} for tunnel {tunnel}, status: {status}",
                connection.interface_name);
                Error::InternalError(
                    format!(
                        "An error occurred while removing interface {} for tunnel {tunnel}, error message: {}. Check logs for more details.", connection.interface_name, status.message()
                    )
                )
            })?;
        info!("Network interface {} has been removed and the connection to tunnel {tunnel} has been closed.", connection.interface_name);
        if let Some(post_down) = &tunnel.post_down {
            debug!("Executing defined PostDown command after removing the interface {} for the tunnel {tunnel}: {post_down}", connection.interface_name);
            let _ = execute_command(post_down);
            info!("Executed defined PostDown command after removing the interface {} for the tunnel {tunnel}: {post_down}", connection.interface_name);
        }
    }
    tunnel.delete(pool).await?;

    info!("Successfully deleted tunnel {tunnel}");
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

    debug!("Fetching latest application version, client metadata: current version: {current_version} and operating system: {operating_system}");

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
            "The latest release version of the application available for download is {}, it was released on {}.",
            response.version, response.release_date
        );
        Ok(response)
    } else {
        let err = res.err().unwrap();
        error!("Failed to fetch latest application version {err}");
        Err(Error::CommandError(err.to_string()))
    }
}

#[tauri::command(async)]
pub async fn command_get_app_config(app_state: State<'_, AppState>) -> Result<AppConfig, Error> {
    debug!("Running command get app config.");
    let res = app_state.app_config.lock().unwrap().clone();
    trace!("Returning config: {res:?}");
    Ok(res)
}

#[tauri::command(async)]
pub async fn command_set_app_config(
    config_patch: AppConfigPatch,
    emit_event: bool,
    app_handle: AppHandle,
) -> Result<AppConfig, Error> {
    let app_state: State<AppState> = app_handle.state();
    debug!("Command set app config received.");
    trace!("Command payload: {config_patch:?}");
    let tray_changed = config_patch.tray_theme.clone();
    let res = {
        let mut app_config = app_state.app_config.lock().unwrap();
        app_config.apply(config_patch);
        app_config.save(&app_handle);
        app_config.clone()
    };
    info!("Config changed successfully");
    if tray_changed.is_some() {
        debug!("Tray theme included in config change, tray will be updated.");
        match configure_tray_icon(&app_handle, &res.tray_theme) {
            Ok(()) => debug!("Tray updated upon config change"),
            Err(err) => error!("Tray change failed. Reason: {err}"),
        }
    }
    if emit_event {
        match app_handle.emit_all(APPLICATION_CONFIG_CHANGED, ()) {
            Ok(()) => debug!("Config changed event emitted successfully"),
            Err(err) => {
                error!("Emission of config changed event failed. Reason: {err}");
            }
        }
    }
    Ok(res)
}
