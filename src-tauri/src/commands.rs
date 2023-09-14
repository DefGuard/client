use crate::{database::Location, error::Error, utils::setup_interface, AppState};
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

///// Consume token and return instance and location info
//pub async fn consume_token(token: String) -> Result<(), Error> {}
