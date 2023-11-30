pub mod models;

use std::fs;

use tauri::AppHandle;

use crate::error::Error;

const DB_NAME: &str = "defguard.db";

pub type DbPool = sqlx::SqlitePool;

// Check if a database file exists, and create one if it does not.
pub async fn init_db(app_handle: &AppHandle) -> Result<DbPool, Error> {
    let app_dir = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or(Error::Config)?;
    // Create app data directory if it doesnt exist
    debug!("Creating app data dir at: {}", app_dir.to_string_lossy());
    fs::create_dir_all(&app_dir)?;
    info!("Created app data dir at: {}", app_dir.to_string_lossy());
    let db_path = app_dir.join(DB_NAME);
    if db_path.exists() {
        info!(
            "Database exists skipping creating database. Database path: {}",
            db_path.to_string_lossy()
        );
    } else {
        debug!(
            "Database not found creating database file at: {}",
            db_path.to_string_lossy()
        );
        fs::File::create(&db_path)?;
        info!(
            "Database file succesfully created at: {}",
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

pub async fn info(pool: &DbPool) -> Result<(), Error> {
    let instances = Instance::all(pool).await?;
    let locations = Location::all(pool).await?;
    debug!(
        "Found {} locations in {} instances",
        locations.len(),
        instances.len()
    );
    trace!("Instances Found:\n {instances:#?}");
    trace!("Locations Found:\n {locations:#?}");
    Ok(())
}

pub use models::{
    connection::{ActiveConnection, Connection, ConnectionInfo},
    instance::{Instance, InstanceInfo},
    location::{Location, LocationStats},
    wireguard_keys::WireguardKeys,
};
