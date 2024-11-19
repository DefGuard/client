use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use log::LevelFilter;
use serde::{Deserialize, Serialize};
use struct_patch::Patch;
use strum::{AsRefStr, Display, EnumString};
use tauri::AppHandle;
use tracing::field::debug;

const APP_CONFIG_FILE_NAME: &str = "config.json";

fn get_config_file_path(app: &AppHandle) -> PathBuf {
    let mut config_file_path = app
        .path_resolver()
        .app_data_dir()
        .expect("Failed to access app data");
    if !config_file_path.exists() {
        std::fs::create_dir_all(&config_file_path).expect("Failed to create missing app data dir");
    }
    config_file_path.push(APP_CONFIG_FILE_NAME);
    config_file_path
}

fn get_config_file(app: &AppHandle, for_write: bool) -> File {
    let config_file_path = get_config_file_path(app);
    match config_file_path.exists() {
        true => std::fs::OpenOptions::new()
            .read(true)
            .write(for_write)
            .truncate(for_write)
            .open(config_file_path)
            .expect("Failed to open app config"),
        false => std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(config_file_path)
            .expect("Failed to create and open app config."),
    }
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
/// information's needed at startup of the application.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Patch)]
#[patch(attribute(derive(Debug, Serialize, Deserialize)))]
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
            debug!("App config doesn't exist in app data, initializing app with default config.");
            let res = Self::default();
            res.save(app);
            return res;
        }
        let mut config_file = get_config_file(app, false);
        let mut file_contents = String::new();
        if config_file.read_to_string(&mut file_contents).is_ok() {
            match serde_json::from_str::<AppConfigPatch>(&file_contents) {
                Ok(patch) => {
                    debug!("Config deserialized successfully");
                    let mut res = AppConfig::default();
                    res.apply(patch);
                    return res;
                }
                // if deserialization failed, remove file and return default
                Err(_) => {
                    let res = Self::default();
                    res.save(app);
                    return res;
                }
            }
        }
        let res = Self::default();
        res.save(app);
        res
    }

    /// Saves currently loaded AppConfig into app data dir file. Warning! this will always override file contents.
    pub fn save(self, app: &AppHandle) {
        let mut file = get_config_file(app, true);
        match serde_json::to_vec(&self) {
            Ok(serialized) => {
                file.write_all(&serialized)
                    .expect("Failed to write app config file.");
                debug("App config saved.");
            }
            Err(e) => {
                error!(
                    "Application config couldn't be saved. Serialization of application config failed. Reason: {}",
                    e.to_string()
                );
            }
        }
    }
}
