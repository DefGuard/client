use std::{fmt, fs, path::Path};

use log::{debug, warn};
use serde::{Deserialize, Serialize};

const CONFIG_FILE_NAME: &str = "provisioning.json";

#[derive(Clone, Deserialize, Serialize)]
pub struct ProvisioningConfig {
    pub enrollment_url: String,
    pub enrollment_token: String,
}

impl fmt::Debug for ProvisioningConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            enrollment_url,
            enrollment_token: _,
        } = self;

        f.debug_struct("ProvisioningConfig")
            .field("enrollment_url", enrollment_url)
            .field("enrollment_token", &"***")
            .finish()
    }
}

impl ProvisioningConfig {
    /// Load configuration from a file at `path`.
    fn load(path: &Path) -> Option<Self> {
        let file_content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                warn!(
                    "Failed to open provisioning configuration file at {}. Error details: {err}",
                    path.display()
                );
                return None;
            }
        };

        let file_content = file_content.trim_start_matches('\u{FEFF}');

        match serde_json::from_str::<Self>(file_content) {
            Ok(config) => Some(config),
            Err(err) => {
                warn!(
                    "Failed to parse provisioning configuration file at {}. Error details: {err}",
                    path.display()
                );
                None
            }
        }
    }
}

/// Try to find and load the provisioning configuration from the given app data directory.
#[must_use]
pub fn try_get_provisioning_config(app_data_dir: &Path) -> Option<ProvisioningConfig> {
    debug!(
        "Trying to find provisioning config in {}",
        app_data_dir.display()
    );

    let config_file_path = app_data_dir.join(CONFIG_FILE_NAME);
    ProvisioningConfig::load(&config_file_path)
}
