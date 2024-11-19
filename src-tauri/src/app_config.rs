use std::{
    fs::{create_dir_all, File, OpenOptions},
    path::PathBuf,
};

use log::LevelFilter;
use serde::{Deserialize, Serialize};
use struct_patch::Patch;
use strum::{AsRefStr, Display, EnumString};
use tauri::AppHandle;

static APP_CONFIG_FILE_NAME: &str = "config.json";

fn get_config_file_path(app: &AppHandle) -> PathBuf {
    let mut config_file_path = app
        .path_resolver()
        .app_data_dir()
        .expect("Failed to access app data");
    if !config_file_path.exists() {
        create_dir_all(&config_file_path).expect("Failed to create missing app data dir");
    }
    config_file_path.push(APP_CONFIG_FILE_NAME);
    config_file_path
}

fn get_config_file(app: &AppHandle, for_write: bool) -> File {
    let config_file_path = get_config_file_path(app);
    OpenOptions::new()
        .create(true)
        .truncate(for_write)
        .write(true)
        .open(config_file_path)
        .expect("Failed to create and open app config.")
}

#[derive(Debug, Clone, EnumString, Display, Serialize, Deserialize, Copy, PartialEq)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, EnumString, Display, Serialize, Deserialize, Copy, PartialEq)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AppLogLevel {
    Error,
    Info,
    Debug,
    Trace,
    Warn,
}

#[derive(Debug, Clone, EnumString, Display, Serialize, Deserialize, Copy, PartialEq, AsRefStr)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AppTrayTheme {
    Color,
    White,
    Black,
    Gray,
}

impl Into<LevelFilter> for AppLogLevel {
    fn into(self) -> LevelFilter {
        match self {
            AppLogLevel::Debug => LevelFilter::Debug,
            AppLogLevel::Error => LevelFilter::Error,
            AppLogLevel::Info => LevelFilter::Info,
            AppLogLevel::Trace => LevelFilter::Trace,
            AppLogLevel::Warn => LevelFilter::Warn,
        }
    }
}

// config stored in config.json in app data
// config is loaded once at startup and saved when modified to the app data file
// information's needed at startup of the application.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Patch)]
#[patch(attribute(derive(Debug, Deserialize, Serialize)))]
pub struct AppConfig {
    pub theme: AppTheme,
    pub tray_theme: AppTrayTheme,
    pub check_for_updates: bool,
    pub log_level: AppLogLevel,
    /// In seconds. How much time should client wait after connecting for the first handshake.
    pub connection_verification_time: u32,
    /// In seconds. How much time can be between handshakes before connection is automatically dropped.
    pub peer_alive_period: u32,
}

// Important: keep in sync with client store default in frontend
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: AppTheme::Light,
            check_for_updates: true,
            tray_theme: AppTrayTheme::Color,
            log_level: AppLogLevel::Info,
            connection_verification_time: 10,
            peer_alive_period: 300,
        }
    }
}

impl AppConfig {
    /// Will try to load from app data dir file and if fails will return default config
    pub fn new(app: &AppHandle) -> Self {
        let config_path = get_config_file_path(app);
        if !config_path.exists() {
            debug!(
                "Application configuration file doesn't exist; initializing it with the defaults."
            );
            let res = Self::default();
            res.save(app);
            return res;
        }
        let config_file = get_config_file(app, false);
        match serde_json::from_reader::<_, AppConfigPatch>(config_file) {
            Ok(patch) => {
                debug!("Config deserialized successfully");
                let mut res = AppConfig::default();
                res.apply(patch);
                res
            }
            // if deserialization failed, remove file and return default
            Err(_) => {
                let res = Self::default();
                res.save(app);
                res
            }
        }
    }

    /// Saves currently loaded AppConfig into app data dir file.
    /// Warning: this will always overwrite file contents.
    pub fn save(self, app: &AppHandle) {
        let file = get_config_file(app, true);
        match serde_json::to_writer(file, &self) {
            Ok(()) => debug!("Application configuration file has been saved."),
            Err(err) => {
                error!(
                    "Application configuration file couldn't be saved. Failed to serialize: {err}",
                );
            }
        }
    }
}
