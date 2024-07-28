pub mod commands;
pub mod models;
pub mod protect;

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tauri::AppHandle;

use crate::error::Error;

const DB_NAME: &str = "defguard.db";

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub type DbPool = sqlx::SqlitePool;

// Check if a database file exists, and create one if it does not.
pub async fn init_db(app_handle: &AppHandle, db_password: Option<String>) -> Result<DbPool, Error> {
    let mut db_file_path = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or(Error::Config)?;
    db_file_path.push(DB_NAME);
    let connect_options = match db_password {
        Some(pass) => SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(db_file_path.clone())
            .pragma("key", pass.clone())
            .pragma("journal_mode", "Delete"),
        None => SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(db_file_path.clone()),
    };
    let pool = SqlitePoolOptions::new()
        .connect_with(connect_options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Settings::init_defaults(&pool).await?;
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
    settings::{Settings, SettingsLogLevel, SettingsTheme, TrayIconTheme},
    tunnel::{Tunnel, TunnelConnection, TunnelConnectionInfo, TunnelStats},
    wireguard_keys::WireguardKeys,
};
