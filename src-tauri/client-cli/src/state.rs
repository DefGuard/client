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
            CoreError::BackendUnavailable(_) => CliError::DaemonUnavailable(err.to_string()),
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
    pub async fn init() -> Result<State, CliError> {
        let data_dir = defguard_core::app_data_dir()
            .map(|p| p.to_string_lossy().to_string())
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

        Ok(State { pool, app_config })
    }
}
