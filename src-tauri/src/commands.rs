use crate::{
    database::{
        models::{instance::InstanceInfo, location::peer_to_location_stats},
        Connection, Instance, Location, WireguardKeys,
    },
    utils::setup_interface,
    AppState,
};
use chrono::Utc;
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use tauri::{State, Manager};
use tokio;
use wireguard_rs::{netlink::delete_interface, wgapi::WGApi};

// Create new wireguard interface
#[tauri::command(async)]
pub async fn connect(location_id: i64, handle: tauri::AppHandle) -> Result<(), String> {
    let state = handle.state::<AppState>();
    if let Some(location) = Location::find_by_id(&state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())?
    {
        setup_interface(&location, &state.get_pool())
            .await
            .map_err(|err| err.to_string())?;
        let address = local_ip().map_err(|err| err.to_string())?;
        let connection = Connection::new(location_id, address.to_string());
        state
            .active_connections
            .lock()
            .unwrap()
            .push(connection);
        // Spawn stats threads
        let api = WGApi::new(location.name, false);
        tokio::spawn(async move {
            let state = handle.state::<AppState>();
            loop {
                match api.read_host() {
                    Ok(host) => {
                        let peers = host.peers;
                        for (_, peer) in peers {
                            let mut location_stats =
                                peer_to_location_stats(&peer, &state.get_pool())
                                    .await
                                    .map_err(|err| err.to_string())
                                    .unwrap();
                            let _ = location_stats.save(&state.get_pool()).await;
                        }
                    }
                    Err(e) => println!("error: {}", e),
                }
            }
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn disconnect(location_id: i64, app_state: State<'_, AppState>) -> Result<(), String> {
    if let Some(location) = Location::find_by_id(&app_state.get_pool(), location_id)
        .await
        .map_err(|err| err.to_string())?
    {
        delete_interface(&location.name).map_err(|err| err.to_string())?;
        if let Some(mut connection) = app_state.find_and_remove_connection(location_id) {
            connection.end = Some(Utc::now().naive_utc()); // Get the current time as NaiveDateTime in UTC
            connection
                .save(&app_state.get_pool())
                .await
                .map_err(|err| err.to_string())?;
        }
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

#[derive(Serialize, Deserialize)]
pub struct DeviceConfig {
    pub network_id: i64,
    pub network_name: String,
    pub config: String,
    pub endpoint: String,
    pub assigned_ip: String,
    pub pubkey: String,
    pub allowed_ips: String,
}

pub fn device_config_to_location(device_config: DeviceConfig) -> Location {
    Location {
        id: None,
        instance_id: device_config.network_id,
        network_id: device_config.network_id,
        name: device_config.network_name,
        address: device_config.assigned_ip, // Transforming assigned_ip to address
        pubkey: device_config.pubkey,
        endpoint: device_config.endpoint,
        allowed_ips: device_config.allowed_ips,
    }
}
#[derive(Serialize, Deserialize)]
pub struct InstanceResponse {
    // uuid
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
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
    let mut keys = WireguardKeys::new(instance.id.unwrap(), private_key, response.device.pubkey);
    keys.save(&mut *transaction)
        .await
        .map_err(|err| err.to_string())?;
    for location in response.configs {
        let mut new_location = device_config_to_location(location);
        new_location
            .save(&mut *transaction)
            .await
            .map_err(|err| err.to_string())?;
    }
    transaction.commit().await.map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub async fn all_instances(app_state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {
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
        })
    }
    Ok(instance_info)
}

#[derive(Serialize)]
pub struct LocationInfo {
    pub id: i64,
    pub instance_id: i64,
    // Native id of network from defguard
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

    Ok(location_info)
}
