use std::{fs::OpenOptions, path::Path};

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    database::{models::instance::Instance, DB_POOL},
    events::{AddInstancePayload, EventKey},
};

const CONFIG_FILE_NAME: &str = "provisioning_config";

#[derive(Debug, Deserialize)]
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
                return None;
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
/// and triggers the process of adding an instance if necessary
pub async fn handle_client_initialization(app_handle: &AppHandle) {
    // check if client has already been initialized
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
                        info!(
                            "Provisioning config found in {data_dir:?}. Triggering enrollment start."
                        );
                        debug!("Provisioning config: {config:?}");
                        let _ = app_handle.emit(
                            EventKey::AddInstance.into(),
                            AddInstancePayload {
                                token: &config.enrollment_token,
                                url: &config.enrollment_url,
                            },
                        );
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
}
