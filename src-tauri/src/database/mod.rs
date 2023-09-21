pub mod models;

use std::fs;
use tauri::AppHandle;

const DB_NAME: &str = "defguard.db";

pub type DbPool = sqlx::SqlitePool;
use crate::error::Error;

// Check if a database file exists, and create one if it does not.
pub async fn init_db(app_handle: &AppHandle) -> Result<DbPool, Error> {
    let app_dir = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or(Error::Config)?;
    let db_path = app_dir.join(DB_NAME);
    if !db_path.exists() {
        fs::File::create(&db_path)?;
    }
    let pool = DbPool::connect(&format!("sqlite://{}", db_path.to_str().unwrap())).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

pub use models::{
    connection::Connection,
    instance::{Instance, InstanceInfo},
    location::{Location, LocationStats},
    wireguard_keys::WireguardKeys,
};
