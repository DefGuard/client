use std::{fs::OpenOptions, path::Path};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::database::{models::instance::Instance, DB_POOL};

const CONFIG_FILE_NAME: &str = "provisioning.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvisioningConfig {
    pub enrollment_url: String,
    pub enrollment_token: String,
}

impl ProvisioningConfig {
    /// Load configuration from a file at `path`.
    fn load(path: &Path) -> Option<Self> {
        let file = match OpenOptions::new().read(true).open(path) {
            Ok(file) => file,
            Err(err) => {
                warn!("Failed to open provisioning configuration file at {path:?}. Error details: {err}");
                return None;
            }
        };
        match serde_json::from_reader::<_, Self>(file) {
            Ok(config) => Some(config),
            Err(err) => {
                warn!("Failed to parse provisioning configuration file at {path:?}. Error details: {err}");
                None
            }
        }
    }
}

pub fn try_get_provisioning_config(app_data_dir: &Path) -> Option<ProvisioningConfig> {
    debug!("Trying to find provisioning config in {app_data_dir:?}");

    let config_file_path = app_data_dir.join(CONFIG_FILE_NAME);
    ProvisioningConfig::load(&config_file_path)
}

/// Checks if the client has already been initialized
/// and tries to load provisioning config from file if necessary
pub async fn handle_client_initialization(app_handle: &AppHandle) -> Option<ProvisioningConfig> {
    // check if client has already been initialized
    // we assume that if any instances exist the client has been initialized
    match Instance::all(&*DB_POOL).await {
        Ok(instances) => {
            if instances.is_empty() {
                debug!(
                    "Client has not been initialized yet. Checking if provisioning config exists"
                );
                let data_dir = app_handle
                    .path()
                    .app_data_dir()
                    .unwrap_or_else(|_| "UNDEFINED DATA DIRECTORY".into());
                match try_get_provisioning_config(&data_dir) {
                    Some(config) => {
                        info!("Provisioning config found in {data_dir:?}.");
                        debug!("Provisioning config: {config:?}");
                        return Some(config);
                    }
                    None => {
                        debug!("Provisioning config not found in {data_dir:?}. Proceeding with normal startup.")
                    }
                }
            }
        }
        Err(err) => {
            error!("Failed to verify if the client has already been initialized: {err}")
        }
    }

    None
}
