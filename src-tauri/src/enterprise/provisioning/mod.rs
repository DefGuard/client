use defguard_client_core::database::{models::instance::Instance, DB_POOL};
pub use defguard_client_provisioning::{try_get_provisioning_config, ProvisioningConfig};
use tauri::{AppHandle, Manager};

/// Checks if the client has already been initialized
/// and tries to load provisioning config from file if necessary
pub async fn handle_client_initialization(app_handle: &AppHandle) -> Option<ProvisioningConfig> {
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
                            "Provisioning config found in {}: {config:?}",
                            data_dir.display()
                        );
                        return Some(config);
                    }
                    None => {
                        debug!(
                            "Provisioning config not found in {}. Proceeding with normal startup.",
                            data_dir.display()
                        );
                    }
                }
            }
        }
        Err(err) => {
            error!("Failed to verify if the client has already been initialized: {err}");
        }
    }

    None
}
