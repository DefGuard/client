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
        debug!(
            "Database not found creating database file at: {}",
            db_path.to_string_lossy()
        );
        fs::File::create(&db_path)?;
        info!(
            "Database file succesfully created at: {}",
            db_path.to_string_lossy()
        );
    } else {
        info!(
            "Database exists skipping creating database. Database path: {}",
            db_path.to_string_lossy()
        );
    }
    debug!("Connecting to database: {}", db_path.to_string_lossy());
    let pool = DbPool::connect(&format!("sqlite://{}", db_path.to_str().unwrap())).await?;
    debug!("Running migrations.");
    sqlx::migrate!().run(&pool).await?;
    info!("Applied migrations.");
    Ok(pool)
}

pub use models::{
    connection::{Connection, ConnectionInfo},
    instance::{Instance, InstanceInfo},
    location::{Location, LocationStats},
    wireguard_keys::WireguardKeys,
};
