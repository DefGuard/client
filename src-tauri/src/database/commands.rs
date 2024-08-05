use tauri::{AppHandle, Manager};

use crate::{
    app_config::AppConfig, appstate::AppState, database::DB_UNPROTECTED_NAME,
    utils::after_db_app_setup,
};

use super::{init_db_connection, protect::protect_db};

#[tauri::command]
pub async fn command_protect_db(app_handle: AppHandle, password: String) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut config = AppConfig::new(&app_handle);
    if config.db_protected {
        return Err("Already protected.".into());
    }
    info!("Db protection started");
    info!("Closing connections");
    state
        .close_all_connections()
        .await
        .map_err(|e| e.to_string())?;
    info!("Connections closed, proceeding");
    {
        let pool = state.get_pool();
        protect_db(&app_handle, &pool, &password)
            .await
            .map_err(|e| e.to_string())?;
        let mut old_db_path = app_handle.path_resolver().app_data_dir().unwrap();
        old_db_path.push(DB_UNPROTECTED_NAME);
        std::fs::remove_file(&old_db_path).ok();
    }
    let pool_option = {
        let mut guard = state.db.lock().expect("Failed to lock db mutex");
        guard.take()
    };
    match pool_option {
        Some(pool) => pool.close().await,
        None => {}
    }
    config.db_protected = true;
    config.save(&app_handle).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn command_unlock_db(app_handle: AppHandle, password: String) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let config = AppConfig::new(&app_handle);
    if !config.db_protected {
        return Err("Db is not protected".into());
    }
    let pool = init_db_connection(&app_handle, Some(password), None)
        .await
        .map_err(|_| "Wrong password!".to_string())?;
    *state.db.lock().expect("Failed to lock dbpool mutex") = Some(pool);
    after_db_app_setup(app_handle.clone())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Will return true if connection exists and false if not.
#[tauri::command]
pub async fn command_db_conn_status(app_handle: AppHandle) -> Result<bool, String> {
    let state = app_handle.state::<AppState>();
    let pool = state.db.lock().map_err(|e| e.to_string())?;
    match *pool {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}
