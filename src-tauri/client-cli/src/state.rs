use std::path::Path;

use defguard_core::{
    app_config::AppConfig,
    database::{handle_db_migrations, DbPool, DB_POOL},
    error::Error as CoreError,
};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Resolved CLI runtime state
pub struct State {
    /// shared SQLite pool
    pub pool: DbPool,
    /// resolved data directory
    #[allow(dead_code)]
    pub data_dir: String,
    /// loaded application configuration (theme, log level, MTU, etc.)
    pub app_config: AppConfig,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("usage: {0}")]
    Usage(String),

    #[error("{0}")]
    NotFound(String),

    #[error("daemon unavailable: {0}")]
    #[allow(dead_code)]
    DaemonUnavailable(String),

    #[error("MFA failed: {0}")]
    MfaFailed(String),

    #[error("MFA input required but no TTY: {0}")]
    MfaInputRequired(String),

    #[error("not enrolled: {0}")]
    NotEnrolled(String),

    #[error("{0}")]
    Other(String),

    #[error("database error: {0}")]
    Database(String),
}

impl From<CoreError> for CliError {
    fn from(err: CoreError) -> Self {
        match &err {
            CoreError::NotFound => CliError::NotFound(err.to_string()),
            CoreError::Database(_) => CliError::Database(err.to_string()),
            _ => CliError::Other(err.to_string()),
        }
    }
}

impl From<sqlx::Error> for CliError {
    fn from(err: sqlx::Error) -> Self {
        CliError::Database(err.to_string())
    }
}

impl State {
    /// Initialize the CLI runtime state: resolve data directory, open the
    /// shared SQLite pool, and run migrations.
    pub async fn init(data_dir_override: Option<&str>) -> Result<State, CliError> {
        // If a data directory override is provided, set DATABASE_URL so that
        // DB_POOL's lazy initializer uses it.  Must happen before first access.
        if let Some(dir) = data_dir_override {
            let db_path = format!("{dir}/defguard.db");
            std::env::set_var("DATABASE_URL", format!("sqlite://{db_path}"));
            info!("Using custom data directory: {dir}");
        }

        let data_dir = data_dir_override
            .map(String::from)
            .or_else(|| defguard_core::app_data_dir().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|| {
                warn!("No app data directory found, using current directory");
                ".".to_string()
            });

        debug!("Using data directory: {data_dir}");

        // Load application configuration (theme, MTU, log level, etc.).
        let app_config = AppConfig::new(Path::new(&data_dir));

        // Access the pool to trigger lazy initialization.
        let pool = DB_POOL.clone();

        // Run migrations.
        handle_db_migrations().await;

        info!("CLI state initialized");

        Ok(State {
            pool,
            data_dir,
            app_config,
        })
    }
}
