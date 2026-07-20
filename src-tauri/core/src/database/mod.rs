use std::{
    env,
    fs::{create_dir_all, File},
    path::PathBuf,
    str::FromStr,
    sync::LazyLock,
    time::Duration,
};

use sqlx::sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqlitePool};

#[cfg(unix)]
use crate::set_perms;
use crate::{app_data_dir, error::Error};

const DB_NAME: &str = "defguard.db";

pub mod models;

pub type DbPool = SqlitePool;

pub static DB_POOL: LazyLock<SqlitePool> = LazyLock::new(|| {
    let db_url = prepare_db_url().expect("Wrong database URL.");
    let opts = SqliteConnectOptions::from_str(&db_url)
        .expect("Failed to set database connenction options.")
        .create_if_missing(true)
        .auto_vacuum(SqliteAutoVacuum::Incremental)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    debug!("Connecting to database: {db_url} with options: {opts:?}");
    SqlitePool::connect_lazy_with(opts)
});

/// Extracts a filesystem path from a SQLite connection URL, returning `None` for
/// non-file databases (e.g. `:memory:`) or empty paths.
///
/// Accepts the `sqlite://` and `sqlite:` scheme prefixes as well as a bare path,
/// and strips any trailing query string (e.g. `?mode=rwc`).
fn sqlite_url_to_path(url: &str) -> Option<PathBuf> {
    let path = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))
        .unwrap_or(url);
    let path = path.split('?').next().unwrap_or(path);
    if path.is_empty() || path == ":memory:" {
        return None;
    }
    Some(PathBuf::from(path))
}

/// Returns the filesystem path of the client's SQLite database file.
///
/// Mirrors the resolution used by [`prepare_db_url`]: honors the `DATABASE_URL`
/// environment variable when set, otherwise falls back to the default location
/// inside the application data directory. Returns `None` when the path cannot be
/// determined — e.g. an in-memory/non-file `DATABASE_URL`, or an undefined
/// application data directory.
///
/// This is a side-effect-free resolver (unlike [`prepare_db_url`], it does not
/// create directories or files) intended for consumers such as posture checks
/// that need to know which partition backs the database.
#[must_use]
pub fn db_file_path() -> Option<PathBuf> {
    if let Ok(url) = env::var("DATABASE_URL") {
        sqlite_url_to_path(&url)
    } else {
        Some(app_data_dir()?.join(DB_NAME))
    }
}

/// Returns database URL. Checks for custom URL in `DATABASE_URL` environment variable.
/// Handles creating appropriate directories if they don't exist.
fn prepare_db_url() -> Result<String, Error> {
    if let Ok(url) = env::var("DATABASE_URL") {
        info!(
            "The default database location has been just overridden by the DATABASE_URL \
            environment variable. The application will use the database located at: {url}"
        );
        Ok(url)
    } else {
        debug!("A production database will be used as no custom DATABASE_URL was provided.");
        // Check if database directory and file exists, create if they don't.
        let app_dir = app_data_dir().ok_or(Error::Config(
            "Application data directory is not defined. Cannot proceed. Is the application \
            running on a supported platform?"
                .to_string(),
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
            create_dir_all(&app_dir)?;
            debug!(
                "Created application data directory at: {}",
                app_dir.to_string_lossy()
            );
        }
        #[cfg(unix)]
        set_perms(&app_dir);
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
            File::create(&db_path)?;
            info!(
                "A new, empty database file has been created at: {} as no previous database file \
                was found. This file will be used to store application data.",
                db_path.to_string_lossy()
            );
        }
        #[cfg(unix)]
        set_perms(&db_path);
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

pub async fn handle_db_migrations() {
    debug!("Running database migrations, if there are any.");
    sqlx::migrate!("../migrations")
        .run(&*DB_POOL)
        .await
        .expect("Failed to apply database migrations.");
    debug!("Applied all database migrations that were pending. If any.");
    debug!("Database setup has been completed successfully.");
}
