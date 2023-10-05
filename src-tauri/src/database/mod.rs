pub mod models;

use crate::utils::LogExt;
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
        .ok_or(Error::Config)
        .log()?;
    // Create app data directory if it doesnt exist
    debug!("Creating app data dir at: {}", app_dir.to_string_lossy());
    fs::create_dir_all(&app_dir).log()?;
    info!("Created app data dir at: {}", app_dir.to_string_lossy());
    let db_path = app_dir.join(DB_NAME);
    if !db_path.exists() {
        debug!(
            "Database not found creating database file at: {}",
            db_path.to_string_lossy()
        );
        fs::File::create(&db_path).log()?;
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
    let pool = DbPool::connect(&format!("sqlite://{}", db_path.to_str().unwrap()))
        .await
        .log()?;
    debug!("Running migrations.");
    sqlx::migrate!().run(&pool).await.log()?;
    info!("Applied migrations.");
    Ok(pool)
}

pub async fn info(pool: &DbPool) -> Result<(), Error> {
    debug!("Following locations and instances are saved.");
    let instances = Instance::all(pool).await.log()?;
    debug!(
        "All instances found in database during start: {:#?}",
        instances
    );
    let locations = Location::all(pool).await.log()?;
    debug!(
        "All locations found in database during start: {:#?}",
        locations
    );
    Ok(())
}

pub use models::{
    connection::{Connection, ConnectionInfo},
    instance::{Instance, InstanceInfo},
    location::{Location, LocationStats},
    wireguard_keys::WireguardKeys,
};
