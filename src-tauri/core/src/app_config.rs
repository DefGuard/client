use std::{
    fs::{create_dir_all, File, OpenOptions},
    path::Path,
};

use log::LevelFilter;
use serde::{Deserialize, Serialize};
use struct_patch::Patch;

#[cfg(unix)]
use crate::set_perms;

static APP_CONFIG_FILE_NAME: &str = "config.json";

fn get_config_file_path(config_dir: &Path) -> std::path::PathBuf {
    let mut config_file_path = config_dir.to_path_buf();
    if !config_file_path.exists() {
        create_dir_all(&config_file_path).expect("Failed to create missing app data dir");
    }
    #[cfg(unix)]
    set_perms(&config_file_path);
    config_file_path.push(APP_CONFIG_FILE_NAME);
    #[cfg(unix)]
    set_perms(&config_file_path);
    config_file_path
}

fn get_config_file(config_dir: &Path, for_write: bool) -> File {
    let config_file_path = get_config_file_path(config_dir);
    OpenOptions::new()
        .create(true)
        .read(true)
        .truncate(for_write)
        .write(true)
        .open(config_file_path)
        .expect("Failed to create and open app config.")
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    Light,
    Dark,
}

// config stored in config.json in app data
// config is loaded once at startup and saved when modified to the app data file
// information's needed at startup of the application.
#[derive(Clone, Debug, Deserialize, Patch, Serialize)]
#[patch(attribute(derive(Debug, Deserialize, Serialize)))]
pub struct AppConfig {
    pub theme: AppTheme,
    pub check_for_updates: bool,
    pub log_level: LevelFilter,
    /// In seconds. How much time after last network activity the connection is automatically dropped.
    pub peer_alive_period: u32,
    /// Maximal transmission unit. 0 means default value.
    mtu: u32,
}

// Important: keep in sync with client store default in frontend
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: AppTheme::Light,
            check_for_updates: true,
            log_level: LevelFilter::Info,
            peer_alive_period: 300,
            mtu: 0,
        }
    }
}

impl AppConfig {
    /// Try to load application configuration from the given config directory.
    /// If reading the configuration file fails, default settings will be returned.
    #[must_use]
    pub fn new(config_dir: &Path) -> Self {
        let config_path = get_config_file_path(config_dir);
        if !config_path.exists() {
            eprintln!(
                "Application configuration file doesn't exist; initializing it with the defaults."
            );
            let res = Self::default();
            res.save(config_dir);
            return res;
        }
        let config_file = get_config_file(config_dir, false);
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
                app_config.save(config_dir);
            }
        }
        app_config
    }

    /// Saves currently loaded AppConfig into the given config directory file.
    /// Warning: this will always overwrite file contents.
    pub fn save(&self, config_dir: &Path) {
        let file = get_config_file(config_dir, true);
        match serde_json::to_writer(file, &self) {
            Ok(()) => debug!("Application configuration file has been saved."),
            Err(err) => {
                error!(
                    "Application configuration file couldn't be saved. Failed to serialize: {err}",
                );
            }
        }
    }

    /// Wraps MTU in an Option. We don't store Option directly in AppConfig to avoid struct-patch
    /// ambiguity when applying updates coming from the frontend. An incoming MTU value of 0 is
    /// interpreted as a request to fall back to the default.
    #[must_use]
    pub fn mtu(&self) -> Option<u32> {
        match self.mtu {
            0 => None,
            v => Some(v),
        }
    }
}
