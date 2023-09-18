use crate::{
    database::{Instance, Location, WireguardKeys},
    error::Error,
    utils::setup_interface,
    AppState,
};
use tauri::State;
use wireguard_rs::netlink::delete_interface;

// Create new wireguard interface
pub async fn connect(location_id: i64, app_state: State<'_, AppState>) -> Result<(), Error> {
    if let Some(location) = Location::find_by_id(&app_state.get_pool(), location_id).await? {
        setup_interface(location, &app_state.get_pool()).await?;
    }
    Ok(())
}

// Create new wireguard interface
pub async fn disconnect(location_id: i64, app_state: State<'_, AppState>) -> Result<(), Error> {
    if let Some(location) = Location::find_by_id(&app_state.get_pool(), location_id).await? {
        delete_interface(&location.name)?;
    }
    Ok(())
}
#[derive(Debug)]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub pubkey: String,
    pub user_id: i64,
    pub created_at: i64,
}

pub struct CreateDeviceResponse {
    instance: Instance,
    device_config: Vec<Location>,
    device: Device,
}

/// Get location id and
pub async fn save_device_config(
    private_key: String,
    mut response: CreateDeviceResponse,
    app_state: State<'_, AppState>,
) -> Result<(), Error> {
    let mut transaction = app_state.get_pool().begin().await?;
    response.instance.save(&mut *transaction).await?;
    let mut keys = WireguardKeys::new(
        response.instance.id.unwrap(),
        private_key,
        config.device.pubkey,
    );
    keys.save(&mut *transaction).await?;
    for location in response.device_config {
        location.save(&mut *transaction).await?;
    }
    Ok(())
}
