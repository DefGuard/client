use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

const APP_CONFIG_NAME: &str = "config.json";

fn get_config_file_path(app: &AppHandle) -> PathBuf {
    let mut config_file_path = app
        .path_resolver()
        .app_data_dir()
        .expect("Failed to access app data");
    config_file_path.push(APP_CONFIG_NAME);
    config_file_path
}

fn get_config_file(app: &AppHandle) -> File {
    let config_file_path = get_config_file_path(app);
    match config_file_path.exists() {
        true => std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .open(config_file_path)
            .expect("Failed to open app config"),
        false => std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(config_file_path)
            .expect("Failed to create and open app config."),
    }
}

// config stored in config.json in app data
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppConfig {
    pub db_protected: bool,
}

impl AppConfig {
    pub fn new(app: &AppHandle) -> Self {
        let mut contents = String::new();
        let mut config_file = get_config_file(app);
        if config_file.read_to_string(&mut contents).is_ok() {
            match serde_json::from_str::<AppConfig>(&contents) {
                Ok(res) => return res,
                // if deserialization failed, remove file and return default
                Err(_) => {
                    fs::remove_file(get_config_file_path(app)).ok();
                }
            }
        }
        let mut config_file_path = app
            .path_resolver()
            .app_data_dir()
            .expect("Failed to access app data");
        config_file_path.push(APP_CONFIG_NAME);
        if config_file_path.exists() {
            let mut contents = String::new();
            if let Ok(mut config_file) = File::open(config_file_path.clone()) {
                if config_file.read_to_string(&mut contents).is_ok() {
                    match serde_json::from_str::<AppConfig>(&contents) {
                        Ok(res) => return res,
                        // if deserialization failed, remove file and return default
                        Err(_) => {
                            fs::remove_file(config_file_path).ok();
                        }
                    }
                }
            }
        }

        AppConfig::default()
    }

    pub fn save(self, app: &AppHandle) -> Result<(), serde_json::Error> {
        let mut file = get_config_file(app);
        let serialized = serde_json::to_vec(&self)?;
        file.write_all(&serialized)
            .expect("Failed to write app config file.");
        Ok(())
    }
}

#[tauri::command]
pub async fn command_get_app_config(handle: AppHandle) -> Result<AppConfig, String> {
    let app_config = AppConfig::new(&handle);
    Ok(app_config)
}
