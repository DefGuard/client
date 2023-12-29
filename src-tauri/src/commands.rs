use crate::{
    appstate::AppState,
    database::{
        models::{instance::InstanceInfo, settings::SettingsPatch},
        ActiveConnection, Connection, ConnectionInfo, Instance, Location, LocationStats, Settings,
        Tunnel, WireguardKeys,
    },
    error::Error,
    service::{
        log_watcher::{spawn_log_watcher_task, stop_log_watcher_task},
        proto::RemoveInterfaceRequest,
    },
    tray::configure_tray_icon,
    utils::{get_interface_name, setup_interface, spawn_stats_thread},
    wg_config::parse_wireguard_config,
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use sqlx::query;
use std::str::FromStr;
use struct_patch::Patch;
use tauri::{AppHandle, Manager, State};
use tracing::Level;

#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
}

// Create new WireGuard interface
#[tauri::command(async)]
pub async fn connect(location_id: i64, handle: AppHandle) -> Result<(), Error> {
    let state = handle.state::<AppState>();
    if let Some(location) = Location::find_by_id(&state.get_pool(), location_id).await? {
        debug!(
            "Creating new interface connection for location: {}",
            location.name
        );
        #[cfg(target_os = "macos")]
        let interface_name = get_interface_name();
        #[cfg(not(target_os = "macos"))]
        let interface_name = get_interface_name(&location);
        setup_interface(
            &location,
            interface_name.clone(),
            &state.get_pool(),
            state.client.clone(),
        )
        .await?;
        let address = local_ip()?;
        let connection =
            ActiveConnection::new(location_id, address.to_string(), interface_name.clone());
        state
            .active_connections
            .lock()
            .map_err(|_| Error::MutexError)?
            .push(connection);
        debug!(
            "Active connections: {:#?}",
            state
                .active_connections
                .lock()
                .map_err(|_| Error::MutexError)?
        );
        debug!("Sending event connection-changed.");
        handle.emit_all(
            "connection-changed",
            Payload {
                message: "Created new connection".into(),
            },
        )?;

        // Spawn stats threads
        debug!("Spawning stats thread");
        spawn_stats_thread(handle.clone(), interface_name.clone()).await;

        // spawn log watcher
        spawn_log_watcher_task(handle, location_id, interface_name, Level::DEBUG, None).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn disconnect(location_id: i64, handle: AppHandle) -> Result<(), Error> {
    debug!("Disconnecting location {}", location_id);
    let state = handle.state::<AppState>();

    if let Some(connection) = state.find_and_remove_connection(location_id) {
        debug!("Found active connection");
        trace!("Connection: {:#?}", connection);
        debug!("Removing interface");
        let mut client = state.client.clone();
        let interface_name = connection.interface_name.clone();
        let request = RemoveInterfaceRequest {
            interface_name: interface_name.clone(),
        };
        if let Err(error) = client.remove_interface(request).await {
            error!("Failed to remove interface: {error}");
            return Err(Error::InternalError);
        }
        debug!("Removed interface");
        debug!("Saving connection");
        trace!("Connection: {:#?}", connection);
        let mut connection: Connection = connection.into();
        connection.save(&state.get_pool()).await?;
        debug!("Connection saved");
        trace!("Saved connection: {connection:#?}");
        handle.emit_all(
            "connection-changed",
            Payload {
                message: "Created new connection".into(),
            },
        )?;

        stop_log_watcher_task(handle, interface_name)?;

        info!("Location {} disconnected", connection.location_id);
        Ok(())
    } else {
        error!("Connection for location with id: {location_id} not found");
        Err(Error::NotFound)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub pubkey: String,
    pub user_id: i64,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceConfig {
    pub network_id: i64,
    pub network_name: String,
    pub config: String,
    pub endpoint: String,
    pub assigned_ip: String,
    pub pubkey: String,
    pub allowed_ips: String,
    pub dns: Option<String>,
}

#[must_use]
pub fn device_config_to_location(device_config: DeviceConfig, instance_id: i64) -> Location {
    Location {
        id: None,
        instance_id,
        network_id: device_config.network_id,
        name: device_config.network_name,
        address: device_config.assigned_ip, // Transforming assigned_ip to address
        pubkey: device_config.pubkey,
        endpoint: device_config.endpoint,
        allowed_ips: device_config.allowed_ips,
        dns: device_config.dns,
        route_all_traffic: false,
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
pub struct CreateDeviceResponse {
    instance: InstanceResponse,
    configs: Vec<DeviceConfig>,
    device: Device,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveDeviceConfigResponse {
    locations: Vec<Location>,
    instance: Instance,
}

#[tauri::command(async)]
pub async fn save_device_config(
    private_key: String,
    response: CreateDeviceResponse,
    app_state: State<'_, AppState>,
    handle: AppHandle,
) -> Result<SaveDeviceConfigResponse, Error> {
    debug!("Received device configuration: {response:#?}");

    let mut transaction = app_state.get_pool().begin().await?;
    let mut instance = Instance::new(
        response.instance.name,
        response.instance.id,
        response.instance.url,
    );

    instance.save(&mut *transaction).await?;

    let mut keys = WireguardKeys::new(
        instance.id.expect("Missing instance ID"),
        response.device.pubkey,
        private_key,
    );
    keys.save(&mut *transaction).await?;
    for location in response.configs {
        let mut new_location =
            device_config_to_location(location, instance.id.expect("Missing instance ID"));
        new_location.save(&mut *transaction).await?;
    }
    transaction.commit().await?;
    info!("Instance created.");
    trace!("Created following instance: {instance:#?}");
    let locations = Location::find_by_instance_id(
        &app_state.get_pool(),
        instance.id.expect("Missing instance ID"),
    )
    .await?;
    trace!("Created following locations: {locations:#?}");
    handle.emit_all("instance-update", ())?;
    let res: SaveDeviceConfigResponse = SaveDeviceConfigResponse {
        locations,
        instance,
    };
    Ok(res)
}

#[tauri::command(async)]
pub async fn all_instances(app_state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, Error> {
    debug!("Retrieving all instances.");

    let instances = Instance::all(&app_state.get_pool()).await?;
    debug!("Found ({}) instances", instances.len());
    trace!("Instances found: {instances:#?}");
    let mut instance_info: Vec<InstanceInfo> = vec![];
    let connection_ids: Vec<i64> = app_state
        .active_connections
        .lock()
        .map_err(|_| Error::MutexError)?
        .iter()
        .map(|connection| connection.location_id)
        .collect();
    for instance in &instances {
        let Some(instance_id) = instance.id else {
            continue;
        };
        let locations = Location::find_by_instance_id(&app_state.get_pool(), instance_id).await?;
        let location_ids: Vec<i64> = locations
            .iter()
            .filter_map(|location| location.id)
            .collect();
        let connected = connection_ids
            .iter()
            .any(|item1| location_ids.iter().any(|item2| item1 == item2));
        let keys = WireguardKeys::find_by_instance_id(&app_state.get_pool(), instance_id)
            .await?
            .ok_or(Error::NotFound)?;
        instance_info.push(InstanceInfo {
            id: instance.id,
            uuid: instance.uuid.clone(),
            name: instance.name.clone(),
            url: instance.url.clone(),
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
}

#[tauri::command(async)]
pub async fn all_locations(
    instance_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<LocationInfo>, Error> {
    debug!("Retrieving all locations.");
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance_id).await?;
    let active_locations_ids: Vec<i64> = app_state
        .active_connections
        .lock()
        .map_err(|_| Error::MutexError)?
        .iter()
        .map(|con| con.location_id)
        .collect();
    let mut location_info = vec![];
    for location in locations {
        let info = LocationInfo {
            id: location.id.expect("Missing location ID"),
            instance_id: location.instance_id,
            name: location.name,
            address: location.address,
            endpoint: location.endpoint,
            active: active_locations_ids.contains(&location.id.expect("Missing location ID")),
            route_all_traffic: location.route_all_traffic,
        };
        location_info.push(info);
    }
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
    app_state: State<'_, AppState>,
) -> Result<LocationInterfaceDetails, Error> {
    debug!("Fetching location details for location ID {location_id}");
    let pool = app_state.get_pool();
    if let Some(location) = Location::find_by_id(&pool, location_id).await? {
        debug!("Fetching WireGuard keys for location {}", location.name);
        let keys = WireguardKeys::find_by_instance_id(&pool, location.instance_id)
            .await?
            .ok_or(Error::NotFound)?;
        let peer_pubkey = keys.pubkey;

        // generate interface name
        #[cfg(target_os = "macos")]
        let interface_name = get_interface_name();
        #[cfg(not(target_os = "macos"))]
        let interface_name = get_interface_name(&location);

        let result = query!(
            r#"
            SELECT last_handshake, listen_port as "listen_port!: u32",
              persistent_keepalive_interval as "persistent_keepalive_interval?: u16"
            FROM location_stats
            WHERE location_id = $1 ORDER BY collected_at DESC LIMIT 1
            "#,
            location_id
        )
        .fetch_optional(&pool)
        .await?;

        let (listen_port, persistent_keepalive_interval, last_handshake) = match result {
            Some(record) => (
                Some(record.listen_port),
                record.persistent_keepalive_interval,
                Some(record.last_handshake),
            ),
            None => (None, None, None),
        };

        Ok(LocationInterfaceDetails {
            location_id,
            name: interface_name,
            pubkey: location.pubkey,
            address: location.address,
            dns: location.dns,
            listen_port,
            peer_pubkey,
            peer_endpoint: location.endpoint,
            allowed_ips: location.allowed_ips,
            persistent_keepalive_interval,
            last_handshake,
        })
    } else {
        error!("Location ID {location_id} not found");
        Err(Error::NotFound)
    }
}

#[tauri::command(async)]
pub async fn update_instance(
    instance_id: i64,
    response: CreateDeviceResponse,
    app_state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), Error> {
    debug!("Received update_instance command");
    trace!("Processing following response:\n {response:#?}");

    let instance = Instance::find_by_id(&app_state.get_pool(), instance_id).await?;
    if let Some(mut instance) = instance {
        let mut transaction = app_state.get_pool().begin().await?;
        instance.name = response.instance.name;
        instance.url = response.instance.url;
        instance.save(&mut *transaction).await?;

        for location in response.configs {
            let mut new_location = device_config_to_location(location, instance_id);
            let old_location =
                Location::find_by_native_id(&mut *transaction, new_location.network_id).await?;
            if let Some(mut old_location) = old_location {
                old_location.name = new_location.name;
                old_location.address = new_location.address;
                old_location.pubkey = new_location.pubkey;
                old_location.endpoint = new_location.endpoint;
                old_location.allowed_ips = new_location.allowed_ips;
                old_location.save(&mut *transaction).await?;
            } else {
                new_location.save(&mut *transaction).await?;
            }
        }
        transaction.commit().await?;
        info!("Instance {instance_id} updated");
        app_handle.emit_all("instance-update", ())?;
        Ok(())
    } else {
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
    pub fn fstring(&self) -> String {
        match self {
            Self::Hour => "%Y-%m-%d %H:00:00",
            Self::Second => "%Y-%m-%d %H:%M:%S",
        }
        .into()
    }
}

fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, Error> {
    // Use hourly aggregation for longer periods
    let aggregation = match Utc::now().naive_utc() - from {
        duration if duration >= Duration::hours(8) => Ok(DateTimeAggregation::Hour),
        duration if duration < Duration::zero() => Err(Error::InternalError),
        _ => Ok(DateTimeAggregation::Second),
    }?;
    Ok(aggregation)
}

#[tauri::command]
pub async fn location_stats(
    location_id: i64,
    from: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<LocationStats>, Error> {
    trace!("Location stats command received");
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = get_aggregation(from)?;
    LocationStats::all_by_location_id(&app_state.get_pool(), location_id, &from, &aggregation).await
}

#[tauri::command]
pub async fn all_connections(
    location_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<ConnectionInfo>, Error> {
    debug!("Retrieving connections for location {location_id}");
    let connections =
        ConnectionInfo::all_by_location_id(&app_state.get_pool(), location_id).await?;
    debug!("Connections received, returning.");
    trace!("Connections found:\n{:#?}", connections);
    Ok(connections)
}

#[tauri::command]
pub async fn active_connection(
    location_id: i64,
    handle: AppHandle,
) -> Result<Option<ActiveConnection>, Error> {
    let state = handle.state::<AppState>();
    debug!("Retrieving active connection for location with id: {location_id}");
    if let Some(location) = Location::find_by_id(&state.get_pool(), location_id).await? {
        debug!("Location found");
        let connection = state.find_connection(location.id.expect("Missing location ID"));
        if connection.is_some() {
            debug!("Active connection found");
        }
        trace!("Connection:\n{:#?}", connection);
        debug!("Connection returned");
        Ok(connection)
    } else {
        error!("Location with id: {location_id} not found.");
        Err(Error::NotFound)
    }
}

#[tauri::command]
pub async fn last_connection(
    location_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Option<Connection>, Error> {
    debug!("Retrieving last connection for location {location_id}");
    let connection = Connection::latest_by_location_id(&app_state.get_pool(), location_id).await?;
    if connection.is_some() {
        trace!("Connection found");
    }
    Ok(connection)
}

#[tauri::command]
pub async fn update_location_routing(
    location_id: i64,
    route_all_traffic: bool,
    handle: AppHandle,
) -> Result<Location, Error> {
    let app_state = handle.state::<AppState>();
    debug!("Updating location routing {location_id}");
    if let Some(mut location) = Location::find_by_id(&app_state.get_pool(), location_id).await? {
        location.route_all_traffic = route_all_traffic;
        location.save(&app_state.get_pool()).await?;
        handle.emit_all(
            "location-update",
            Payload {
                message: "Location routing updated".into(),
            },
        )?;
        Ok(location)
    } else {
        error!("Location with id: {location_id} not found.");
        Err(Error::NotFound)
    }
}

#[tauri::command]
pub async fn get_settings(handle: AppHandle) -> Result<Settings, Error> {
    let app_state = handle.state::<AppState>();
    let settings = Settings::get(&app_state.get_pool()).await?;
    Ok(settings)
}

#[tauri::command]
pub async fn update_settings(data: SettingsPatch, handle: AppHandle) -> Result<Settings, Error> {
    let app_state = handle.state::<AppState>();
    let pool = &app_state.get_pool();
    let mut settings = Settings::get(pool).await?;
    settings.apply(data);
    settings.save(pool).await?;
    configure_tray_icon(&handle, &settings.tray_icon_theme)?;
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
        for location in instance_locations.iter() {
            if let Some(location_id) = location.id {
                if let Some(connection) = app_state.find_and_remove_connection(location_id) {
                    debug!("Found active connection for location({location_id}), closing...",);
                    let request = RemoveInterfaceRequest {
                        interface_name: connection.interface_name.clone(),
                    };
                    client
                        .remove_interface(request)
                        .await
                        .map_err(|_| Error::InternalError)?;
                    debug!("Connection closed and interface removed");
                }
            }
        }
        instance.delete(pool).await?;
    } else {
        error!("Instance {instance_id} not found");
        return Err(Error::NotFound);
    }
    handle.emit_all("instance-update", ())?;
    info!("Instance {instance_id}, deleted");
    Ok(())
}
#[tauri::command(async)]
pub async fn parse_tunnel_config(config: String) -> Result<Tunnel, Error> {
    debug!("Parsing config file");
    parse_wireguard_config(&config).map_err(|error| {
        error!("{error}");
        Error::ConfigParseError(error.to_string())
    })
}
#[tauri::command(async)]
pub async fn save_tunnel(mut tunnel: Tunnel, app_state: State<'_, AppState>) -> Result<(), Error> {
    debug!("Received tunnel configuration: {tunnel:#?}");
    tunnel.save(&app_state.get_pool()).await?;
    info!("Saved tunnel {tunnel:#?}");
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
}

#[tauri::command(async)]
pub async fn all_tunnels(app_state: State<'_, AppState>) -> Result<Vec<TunnelInfo>, Error> {
    debug!("Retrieving all instances.");

    let tunnels = Tunnel::all(&app_state.get_pool()).await?;
    debug!("Found ({}) tunnels", tunnels.len());
    trace!("Instances found: {tunnels:#?}");
    let mut tunnel_info: Vec<TunnelInfo> = vec![];

    for tunnel in tunnels {
        tunnel_info.push(TunnelInfo {
            id: tunnel.id,
            name: tunnel.name,
            address: tunnel.address,
            endpoint: tunnel.endpoint,
            route_all_traffic: tunnel.route_all_traffic,
            active: false,
        })
    }
    Ok(tunnel_info)
}
