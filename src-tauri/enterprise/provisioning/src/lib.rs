use std::{fmt, fs, io, path::Path};

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

/// Remove the one-time provisioning configuration after the client is initialized.
pub fn clear_provisioning_config(app_data_dir: &Path) -> io::Result<()> {
    match fs::remove_file(app_data_dir.join(CONFIG_FILE_NAME)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_load_valid_config() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE_NAME),
            r#"{"enrollment_url":"https://enroll","enrollment_token":"secret"}"#,
        )
        .unwrap();

        let config = try_get_provisioning_config(dir.path()).expect("config should load");
        assert_eq!(config.enrollment_url, "https://enroll");
        assert_eq!(config.enrollment_token, "secret");
    }

    #[test]
    fn test_missing_file_returns_none() {
        let dir = tempdir().unwrap();
        assert!(try_get_provisioning_config(dir.path()).is_none());
    }

    #[test]
    fn test_clear_provisioning_config_removes_file_and_is_idempotent() {
        let dir = tempdir().unwrap();
        let config_file = dir.path().join(CONFIG_FILE_NAME);
        fs::write(&config_file, "{}").unwrap();

        clear_provisioning_config(dir.path()).expect("config file should be removed");
        assert!(!config_file.exists());
        clear_provisioning_config(dir.path()).expect("missing config file should be ignored");
    }

    #[test]
    fn test_malformed_json_returns_none() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), b"{ not valid json").unwrap();
        assert!(try_get_provisioning_config(dir.path()).is_none());
    }

    #[test]
    fn test_bom_prefixed_config_parses() {
        let dir = tempdir().unwrap();
        // A UTF-8 BOM prefix must be tolerated.
        let content =
            "\u{FEFF}{\"enrollment_url\":\"https://enroll\",\"enrollment_token\":\"secret\"}";
        fs::write(dir.path().join(CONFIG_FILE_NAME), content).unwrap();

        let config =
            try_get_provisioning_config(dir.path()).expect("BOM-prefixed config should load");
        assert_eq!(config.enrollment_url, "https://enroll");
        assert_eq!(config.enrollment_token, "secret");
    }

    #[test]
    fn test_debug_redacts_token() {
        let config = ProvisioningConfig {
            enrollment_url: "https://enroll".into(),
            enrollment_token: "super-secret".into(),
        };

        let rendered = format!("{config:?}");
        assert!(rendered.contains("***"));
        assert!(!rendered.contains("super-secret"));
        // The non-sensitive URL is still shown.
        assert!(rendered.contains("https://enroll"));
    }
}
