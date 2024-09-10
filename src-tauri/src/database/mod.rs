pub mod models;

use std::{env, fs};

pub use models::{
    connection::{ActiveConnection, Connection, ConnectionInfo},
    instance::{Instance, InstanceInfo},
    location::Location,
    location_stats::LocationStats,
    settings::{Settings, SettingsLogLevel, SettingsTheme, TrayIconTheme},
    tunnel::{Tunnel, TunnelConnection, TunnelConnectionInfo, TunnelStats},
    wireguard_keys::WireguardKeys,
};
use tauri::AppHandle;

use crate::error::Error;

const DB_NAME: &str = "defguard.db";

pub type DbPool = sqlx::SqlitePool;

/// Initializes the database
pub async fn init_db(app_handle: &AppHandle) -> Result<DbPool, Error> {
    let db_url = prepare_db_url(app_handle)?;
    debug!("Connecting to database: {}", db_url);
    let pool = DbPool::connect(&db_url).await?;
    debug!("Running migrations.");
    sqlx::migrate!().run(&pool).await?;
    Settings::init_defaults(&pool).await?;
    info!("Applied migrations.");
    Ok(pool)
}

/// Returns database url. Checks for custom url in `DATABASE_URL` env variable.
/// Handles creating appropriate directories if they don't exist.
fn prepare_db_url(app_handle: &AppHandle) -> Result<String, Error> {
    if let Ok(url) = env::var("DATABASE_URL") {
        debug!("Using custom database url: {url}");
        Ok(url)
    } else {
        debug!("Using production database");
        // Check if database directory and file exists, create if they don't.
        let app_dir = app_handle
            .path_resolver()
            .app_data_dir()
            .ok_or(Error::Config)?;
        debug!("Creating app data dir at: {}", app_dir.to_string_lossy());
        fs::create_dir_all(&app_dir)?;
        info!("Created app data dir at: {}", app_dir.to_string_lossy());
        let db_path = app_dir.join(DB_NAME);
        if db_path.exists() {
            info!(
                "Database exists skipping database creation. Database path: {}",
                db_path.to_string_lossy()
            );
        } else {
            debug!(
                "Database not found. Creating database file at: {}",
                db_path.to_string_lossy()
            );
            fs::File::create(&db_path)?;
            info!(
                "Database file successfully created at: {}",
                db_path.to_string_lossy()
            );
        }
        Ok(format!(
            "sqlite://{}",
            db_path.to_str().expect("Failed to format DB path")
        ))
    }
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
