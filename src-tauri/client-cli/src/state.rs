use defguard_core::database::{handle_db_migrations, DbPool, DB_POOL};
use thiserror::Error;

/// Resolved CLI runtime state.
pub struct State {
    /// The shared SQLite pool (initialized with WAL + busy_timeout).
    pub pool: DbPool,
    /// The resolved data directory.
    pub data_dir: String,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("usage: {0}")]
    Usage(String),

    #[error("{0}")]
    NotFound(String),

    #[error("daemon unavailable: {0}")]
    Unavailable(String),

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

impl From<defguard_core::error::Error> for CliError {
    fn from(err: defguard_core::error::Error) -> Self {
        match &err {
            defguard_core::error::Error::NotFound => CliError::NotFound(err.to_string()),
            defguard_core::error::Error::Database(_) => CliError::Database(err.to_string()),
            _ => CliError::Other(err.to_string()),
        }
    }
}

impl From<sqlx::Error> for CliError {
    fn from(err: sqlx::Error) -> Self {
        CliError::Database(err.to_string())
    }
}

/// Initialize the CLI runtime state: resolve data directory, open the
/// shared SQLite pool, and run migrations.
pub async fn init(data_dir_override: Option<&str>) -> Result<State, CliError> {
    // If a data directory override is provided, set DATABASE_URL so that
    // DB_POOL's lazy initializer uses it.  Must happen before first access.
    if let Some(dir) = data_dir_override {
        let db_path = format!("{dir}/defguard.db");
        std::env::set_var("DATABASE_URL", format!("sqlite://{db_path}"));
        tracing::info!("Using custom data directory: {dir}");
    }

    let data_dir = data_dir_override
        .map(String::from)
        .or_else(|| defguard_core::app_data_dir().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| {
            tracing::warn!("No app data directory found, using current directory");
            ".".to_string()
        });

    tracing::debug!("Using data directory: {data_dir}");

    // Access the pool to trigger lazy initialization.
    let pool = DB_POOL.clone();

    // Run migrations.
    handle_db_migrations().await;

    tracing::info!("CLI state initialized");

    Ok(State { pool, data_dir })
}
