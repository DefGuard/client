use crate::{
    database::{
        models::{instance::InstanceInfo, location::peer_to_location_stats},
        Connection, ConnectionInfo, Instance, Location, LocationStats, WireguardKeys,
    },
    utils::{remove_whitespace, setup_interface, IS_MACOS},
    AppState,
};
use chrono::Utc;
use defguard_wireguard_rs::{WGApi, WireguardInterfaceApi};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tokio;
use tokio::time::{sleep, Duration};

#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
}

// Create new wireguard interface
#[tauri::command(async)]
pub async fn connect(location_id: i64, handle: tauri::AppHandle) -> Result<(), String> {
    let state = handle.state::<AppState>();
    if let Some(location) = Location::find_by_id(&state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())?
    {
        debug!(
            "Creating new interface connection for location: {}",
            location.name
        );
        let api = setup_interface(&location, &state.get_pool())
            .await
            .map_err(|err| err.to_string())?;
        let address = local_ip().map_err(|err| err.to_string())?;
        let connection = Connection::new(location_id, address.to_string());
        state.active_connections.lock().unwrap().push(connection);
        debug!(
            "Active connections: {:#?}",
            state.active_connections.lock().unwrap()
        );
        debug!("Sending event connection-changed.");
        handle
            .emit_all(
                "connection-changed",
                Payload {
                    message: "Created new connection".into(),
                },
            )
            .unwrap();
        // Spawn stats threads
        tokio::spawn(async move {
            let state = handle.state::<AppState>();
            loop {
                match api.read_interface_data() {
                    Ok(host) => {
                        let peers = host.peers;
                        for (_, peer) in peers {
                            let mut location_stats =
                                peer_to_location_stats(&peer, &state.get_pool())
                                    .await
                                    .map_err(|err| err.to_string())
                                    .unwrap();
                            debug!("Saving location stats: {:#?}", location_stats);
                            let _ = location_stats.save(&state.get_pool()).await;
                            debug!("Saved location stats: {:#?}", location_stats);
                        }
                    }
                    Err(e) => {
                        error!(
                            "Error {} while reading data for interface: {}",
                            e, location.name
                        );
                        debug!(
                            "Stopped stats thread for location: {}. Error: {}",
                            location.name,
                            e.to_string()
                        );
                        break;
                    }
                }
                sleep(Duration::from_secs(60)).await;
            }
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn disconnect(location_id: i64, handle: tauri::AppHandle) -> Result<(), String> {
    debug!("Disconnecting location with id: {}", location_id);
    let state = handle.state::<AppState>();
    if let Some(location) = Location::find_by_id(&state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())?
    {
        let api =
            WGApi::new(remove_whitespace(&location.name), IS_MACOS).map_err(|e| e.to_string())?;
        debug!("Removing interface");
        api.remove_interface().map_err(|err| err.to_string())?;
        debug!("Removed interface");
        if let Some(mut connection) = state.find_and_remove_connection(location_id) {
            debug!("Saving connection: {:#?}", connection);
            connection.end = Some(Utc::now().naive_utc()); // Get the current time as NaiveDateTime in UTC
            connection
                .save(&state.get_pool())
                .await
                .map_err(|err| err.to_string())?;
            debug!("Saved connection: {:#?}", connection);
        }
        handle
            .emit_all(
                "connection-changed",
                Payload {
                    message: "Created new connection".into(),
                },
            )
            .unwrap();
    }
    Ok(())
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
}

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

#[tauri::command(async)]
pub async fn save_device_config(
    private_key: String,
    response: CreateDeviceResponse,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("Received device configuration: {:#?}", response);
    let mut transaction = app_state
        .get_pool()
        .begin()
        .await
        .map_err(|err| err.to_string())?;
    let mut instance = Instance::new(
        response.instance.name,
        response.instance.id,
        response.instance.url,
    );
    instance
        .save(&mut *transaction)
        .await
        .map_err(|e| e.to_string())?;
    let mut keys = WireguardKeys::new(instance.id.unwrap(), response.device.pubkey, private_key);
    keys.save(&mut *transaction)
        .await
        .map_err(|err| err.to_string())?;
    for location in response.configs {
        let mut new_location = device_config_to_location(location, instance.id.unwrap());
        new_location
            .save(&mut *transaction)
            .await
            .map_err(|err| err.to_string())?;
    }
    transaction.commit().await.map_err(|err| err.to_string())?;
    info!("Instance created.");
    debug!("Created following instance: {:#?}", instance);
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance.id.unwrap())
        .await
        .map_err(|err| err.to_string())?;
    debug!("Created following locations: {:#?}", locations);
    Ok(())
}

#[tauri::command(async)]
pub async fn all_instances(app_state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {
    debug!("Retrieving all instances.");
    let instances = Instance::all(&app_state.get_pool())
        .await
        .map_err(|err| err.to_string())?;
    let mut instance_info: Vec<InstanceInfo> = vec![];
    for instance in &instances {
        let locations = Location::find_by_instance_id(&app_state.get_pool(), instance.id.unwrap())
            .await
            .map_err(|err| err.to_string())?;
        let connection_ids: Vec<i64> = app_state
            .active_connections
            .lock()
            .unwrap()
            .iter()
            .map(|connection| connection.location_id)
            .collect();
        let location_ids: Vec<i64> = locations
            .iter()
            .map(|location| location.id.unwrap())
            .collect();
        let connected = connection_ids
            .iter()
            .any(|item1| location_ids.iter().any(|item2| item1 == item2));
        let keys = WireguardKeys::find_by_instance_id(&app_state.get_pool(), instance.id.unwrap())
            .await
            .map_err(|err| err.to_string())?
            .unwrap();
        instance_info.push(InstanceInfo {
            id: instance.id,
            uuid: instance.uuid.clone(),
            name: instance.name.clone(),
            url: instance.url.clone(),
            connected,
            pubkey: keys.pubkey,
        });
        info!("Returning following instances: {:#?}", instance_info);
    }
    Ok(instance_info)
}

#[derive(Serialize, Debug)]
pub struct LocationInfo {
    pub id: i64,
    // Native id of network from defguard
    pub instance_id: i64,
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub active: bool,
}

#[tauri::command(async)]
pub async fn all_locations(
    instance_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<LocationInfo>, String> {
    debug!("Retrieving all locations");
    let locations = Location::find_by_instance_id(&app_state.get_pool(), instance_id)
        .await
        .map_err(|err| err.to_string())?;
    let active_locations_ids: Vec<i64> = app_state
        .active_connections
        .lock()
        .unwrap()
        .iter()
        .map(|con| con.location_id)
        .collect();
    let mut location_info = vec![];
    for location in locations {
        let info = LocationInfo {
            id: location.id.unwrap(),
            instance_id: location.instance_id,
            name: location.name,
            address: location.address,
            endpoint: location.endpoint,
            active: active_locations_ids.contains(&location.id.unwrap()),
        };
        location_info.push(info);
    }
    debug!("Returning all locations: {:#?}", location_info);

    Ok(location_info)
}
#[tauri::command(async)]
pub async fn update_instance(
    instance_id: i64,
    response: CreateDeviceResponse,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("Received following response: {:#?}", response);
    let instance = Instance::find_by_id(&app_state.get_pool(), instance_id)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(mut instance) = instance {
        let mut transaction = app_state
            .get_pool()
            .begin()
            .await
            .map_err(|err| err.to_string())?;
        instance.name = response.instance.name;
        instance.url = response.instance.url;
        instance
            .save(&mut *transaction)
            .await
            .map_err(|err| err.to_string())?;

        for location in response.configs {
            let mut new_location = device_config_to_location(location, instance_id);
            let old_location =
                Location::find_by_native_id(&mut *transaction, new_location.network_id)
                    .await
                    .map_err(|err| err.to_string())?;
            if let Some(mut old_location) = old_location {
                old_location.name = new_location.name;
                old_location.address = new_location.address;
                old_location.pubkey = new_location.pubkey;
                old_location.endpoint = new_location.endpoint;
                old_location.allowed_ips = new_location.allowed_ips;
                old_location
                    .save(&mut *transaction)
                    .await
                    .map_err(|err| err.to_string())?;
            } else {
                new_location
                    .save(&mut *transaction)
                    .await
                    .map_err(|err| err.to_string())?;
            }
        }
        transaction.commit().await.map_err(|err| err.to_string())?;
        info!("Updated instance with id: {}.", instance_id);
        Ok(())
    } else {
        Err("Instance not found".into())
    }
}
#[tauri::command]
pub async fn location_stats(
    location_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<LocationStats>, String> {
    LocationStats::all_by_location_id(&app_state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn all_connections(
    location_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<ConnectionInfo>, String> {
    debug!("Retrieving all conections.");
    let connections = ConnectionInfo::all_by_location_id(&app_state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())?;
    debug!(
        "Returning all connections for location with id: {}, {:#?} ",
        location_id, connections
    );
    Ok(connections)
}
#[tauri::command]
pub async fn active_connection(
    location_id: i64,
    handle: tauri::AppHandle,
) -> Result<Option<Connection>, String> {
    let state = handle.state::<AppState>();
    if let Some(location) = Location::find_by_id(&state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())?
    {
        debug!(
            "Returning active connection: {:#?}",
            state.find_connection(location.id.unwrap())
        );
        Ok(state.find_connection(location.id.unwrap()))
    } else {
        error!("Location with id: {} not found.", location_id);
        Err("Location not found".into())
    }
}

#[tauri::command]
pub async fn last_connection(
    location_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Connection, String> {
    if let Some(connection) = Connection::latest_by_location_id(&app_state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())?
    {
        debug!("Returning last connection: {:#?}", connection);
        Ok(connection)
    } else {
        error!("No connections for location: {}", location_id);
        Err("No connections for this device".into())
    }
}
