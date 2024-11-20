pub mod models;

use std::{env, fs};

pub use models::{
    connection::{ActiveConnection, Connection, ConnectionInfo},
    instance::{Instance, InstanceInfo},
    location::Location,
    location_stats::LocationStats,
    tunnel::{Tunnel, TunnelConnection, TunnelConnectionInfo, TunnelStats},
    wireguard_keys::WireguardKeys,
};
use tauri::AppHandle;

use crate::error::Error;

const DB_NAME: &str = "defguard.db";

pub(crate) type DbPool = sqlx::SqlitePool;

/// Initializes the database
pub async fn init_db(app_handle: &AppHandle) -> Result<DbPool, Error> {
    let db_url = prepare_db_url(app_handle)?;
    debug!("Connecting to database: {db_url}");
    let pool = DbPool::connect(&db_url).await?;
    debug!("Running database migrations, if there are any.");
    sqlx::migrate!().run(&pool).await?;
    debug!("Applied all database migrations that were pending. If any.");
    Ok(pool)
}

/// Returns database url. Checks for custom url in `DATABASE_URL` env variable.
/// Handles creating appropriate directories if they don't exist.
fn prepare_db_url(app_handle: &AppHandle) -> Result<String, Error> {
    if let Ok(url) = env::var("DATABASE_URL") {
        info!("The default database location has been just overridden by the DATABASE_URL environment variable. The application will use the database located at: {url}");
        Ok(url)
    } else {
        debug!("A production database will be used as no custom DATABASE_URL was provided.");
        // Check if database directory and file exists, create if they don't.
        let app_dir = app_handle
            .path_resolver()
            .app_data_dir()
            .ok_or(Error::Config(
                "Application data directory is not defined. Cannot proceed. Is the application running on a supported platform?".to_string()
            ))?;
        if app_dir.exists() {
            debug!(
                "Application data directory already exists at: {}, skipping its creation.",
                app_dir.to_string_lossy()
            );
        } else {
            debug!(
                "Creating application data directory at: {}",
                app_dir.to_string_lossy()
            );
            fs::create_dir_all(&app_dir)?;
            debug!(
                "Created application data directory at: {}",
                app_dir.to_string_lossy()
            );
        }
        let db_path = app_dir.join(DB_NAME);
        if db_path.exists() {
            debug!(
                "Database file already exists at: {}. Skipping its creation.",
                db_path.to_string_lossy()
            );
        } else {
            debug!(
                "Database file not found at {}. Creating a new one.",
                db_path.to_string_lossy()
            );
            fs::File::create(&db_path)?;
            info!(
                "A new, empty database file has been created at: {} as no previous database file was found. This file will be used to store application data.",
                db_path.to_string_lossy()
            );
        }
        debug!(
            "Application's database file is located at: {}",
            db_path.to_string_lossy()
        );
        Ok(format!(
            "sqlite://{}",
            db_path.to_str().expect("Failed to format DB path")
        ))
    }
}
