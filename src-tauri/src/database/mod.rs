pub mod commands;
pub mod models;
pub mod protect;

use std::path::PathBuf;

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tauri::AppHandle;

use crate::error::Error;

const DB_UNPROTECTED_NAME: &str = "defguard.db";

const DB_PROTECTED_NAME: &str = "defguard.enc.db";

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub type DbPool = sqlx::SqlitePool;

// Connect to database, default or provided with arg db_file
pub async fn init_db_connection(
    app_handle: &AppHandle,
    db_password: Option<String>,
    db_file: Option<PathBuf>,
) -> Result<DbPool, Error> {
    let db_file_path = match db_file {
        Some(file) => file,
        None => {
            let mut file_path = app_handle
                .path_resolver()
                .app_data_dir()
                .ok_or(Error::Config)?;
            match db_password {
                Some(_) => {
                    file_path.push(DB_PROTECTED_NAME);
                }
                None => {
                    file_path.push(DB_UNPROTECTED_NAME);
                }
            };
            file_path
        }
    };
    info!("Path to db: {:?}", db_file_path.clone());
    let connect_options = match db_password {
        Some(pass) => SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(db_file_path.clone())
            .pragma("key", pass.clone()),
        None => SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(db_file_path.clone()),
    };
    info!("Conn options: {:?}", connect_options.clone());
    let pool = SqlitePoolOptions::new()
        .connect_with(connect_options)
        .await?;
    info!("Pool created");
    MIGRATOR.run(&pool).await?;
    info!("Migrations applied");
    Settings::init_defaults(&pool).await?;
    info(&pool).await?;
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
