use crate::{database::Location, error::Error, AppState};
use tauri::State;

pub async fn connect(location_id: i64, app_state: State<'_, AppState>) -> Result<(), Error> {
    if let Some(location) = Location::find_by_id(&app_state.get_pool(), location_id).await? {}
    Ok(())
}

///// Consume token and return instance and location info
//pub async fn consume_token(token: String) -> Result<(), Error> {}
