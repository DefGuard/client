use std::{
    fs::{create_dir_all, File, OpenOptions},
    path::PathBuf,
};

use log::LevelFilter;
use serde::{Deserialize, Serialize};
use struct_patch::Patch;
use strum::{Display, EnumString};
use tauri::{AppHandle, Manager};

static APP_CONFIG_FILE_NAME: &str = "config.json";

fn get_config_file_path(app: &AppHandle) -> PathBuf {
    let mut config_file_path = app
        .path()
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
        .read(true)
        .truncate(for_write)
        .write(true)
        .open(config_file_path)
        .expect("Failed to create and open app config.")
}

#[derive(Debug, Clone, Deserialize, Display, EnumString, PartialEq, Serialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Deserialize, Display, EnumString, PartialEq, Serialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AppTrayTheme {
    Color,
    White,
    Black,
    Gray,
}

// config stored in config.json in app data
// config is loaded once at startup and saved when modified to the app data file
// information's needed at startup of the application.
#[derive(Clone, Debug, Deserialize, Patch, Serialize)]
#[patch(attribute(derive(Debug, Deserialize, Serialize)))]
pub struct AppConfig {
    pub theme: AppTheme,
    pub tray_theme: AppTrayTheme,
    pub check_for_updates: bool,
    pub log_level: LevelFilter,
    /// In seconds. How much time after last network activity the connection is automatically dropped.
    pub peer_alive_period: u32,
}

// Important: keep in sync with client store default in frontend
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: AppTheme::Light,
            check_for_updates: true,
            tray_theme: AppTrayTheme::Color,
            log_level: LevelFilter::Info,
            peer_alive_period: 300,
        }
    }
}

impl AppConfig {
    /// Try to load application configuration from application data directory.
    /// If reading the configuration file fails, default settings will be returned.
    #[must_use]
    pub fn new(app: &AppHandle) -> Self {
        let config_path = get_config_file_path(app);
        if !config_path.exists() {
            eprintln!(
                "Application configuration file doesn't exist; initializing it with the defaults."
            );
            let res = Self::default();
            res.save(app);
            return res;
        }
        let config_file = get_config_file(app, false);
        let mut app_config = Self::default();
        match serde_json::from_reader::<_, AppConfigPatch>(config_file) {
            Ok(patch) => {
                app_config.apply(patch);
            }
            // If deserialization fails, remove file and return the default.
            Err(err) => {
                eprintln!(
                    "Failed to deserialize application configuration file: {err}. Using defaults."
                );
                app_config.save(app);
            }
        }
        app_config
    }

    /// Saves currently loaded AppConfig into app data dir file.
    /// Warning: this will always overwrite file contents.
    pub fn save(&self, app: &AppHandle) {
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
