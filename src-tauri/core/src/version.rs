use std::{
    fs::{create_dir_all, File, OpenOptions},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub use semver::Version;

#[cfg(unix)]
use crate::set_perms;

pub const MIN_CORE_VERSION: Version = Version::new(1, 6, 0);
pub const MIN_PROXY_VERSION: Version = Version::new(1, 6, 0);
pub const CLIENT_VERSION_HEADER: &str = "defguard-client-version";
pub const CLIENT_PLATFORM_HEADER: &str = "defguard-client-platform";
pub const LOG_FILENAME: &str = "defguard-client";
pub const WELCOME_FORCE_ENV_VAR: &str = "DEFGUARD_CLIENT_WELCOME_FORCE";
pub use defguard_client_common::VERSION as PKG_VERSION;

/// Selects the version string the client should report: the build-version override when present
/// and non-blank, otherwise the package version.
#[must_use]
pub fn select_reported_app_version(
    package_version: &str,
    build_version_override: Option<&str>,
) -> String {
    build_version_override
        .filter(|version| !version.trim().is_empty())
        .map_or_else(|| package_version.to_owned(), str::to_owned)
}

static VERSION_STATE_FILE_NAME: &str = "version.json";

fn get_version_state_file_path(config_dir: &Path) -> PathBuf {
    let mut path = config_dir.to_path_buf();
    if !path.exists() {
        create_dir_all(&path).expect("Failed to create missing app data dir");
    }
    #[cfg(unix)]
    set_perms(&path);
    path.push(VERSION_STATE_FILE_NAME);
    #[cfg(unix)]
    set_perms(&path);
    path
}

fn get_version_state_file(config_dir: &Path, for_write: bool) -> File {
    let path = get_version_state_file_path(config_dir);
    OpenOptions::new()
        .create(true)
        .read(true)
        .truncate(for_write)
        .write(true)
        .open(path)
        .expect("Failed to create and open version state file.")
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VersionState {
    version: Version,
}

impl VersionState {
    fn save(&self, config_dir: &Path) {
        let file = get_version_state_file(config_dir, true);
        match serde_json::to_writer(file, &self) {
            Ok(()) => debug!("Version state file has been saved."),
            Err(err) => error!("Version state file couldn't be saved. Failed to serialize: {err}"),
        }
    }
}

/// Result of comparing the last known app version (persisted on disk) against the currently
/// running version.
#[derive(Clone, Debug, PartialEq)]
pub enum VersionCheckResult {
    /// No version state file existed on disk yet (fresh install, or first run of this check).
    Init,
    /// Stored version matches the current version. Also returned for a downgrade (current
    /// version lower than the stored one) — the file is left untouched in that case so the
    /// highest version ever seen isn't lost.
    Unchanged,
    /// Stored version is lower than the current version.
    Upgraded { previous: Version, current: Version },
}

fn welcome_force_enabled() -> bool {
    std::env::var(WELCOME_FORCE_ENV_VAR)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Checks the last known app version (persisted in `config_dir`) against `current_version`,
/// updating the on-disk state as needed.
///
/// Meant to be called exactly once, synchronously, during app setup.
#[must_use]
pub fn check_app_version(config_dir: &Path, current_version: &Version) -> VersionCheckResult {
    if welcome_force_enabled() {
        return VersionCheckResult::Upgraded {
            previous: current_version.clone(),
            current: current_version.clone(),
        };
    }

    let path = get_version_state_file_path(config_dir);
    if !path.exists() {
        VersionState {
            version: current_version.clone(),
        }
        .save(config_dir);
        return VersionCheckResult::Init;
    }

    let file = get_version_state_file(config_dir, false);
    match serde_json::from_reader::<_, VersionState>(file) {
        Ok(state) => match state.version.cmp(current_version) {
            std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                VersionCheckResult::Unchanged
            }
            std::cmp::Ordering::Less => {
                let previous = state.version;
                VersionState {
                    version: current_version.clone(),
                }
                .save(config_dir);
                VersionCheckResult::Upgraded {
                    previous,
                    current: current_version.clone(),
                }
            }
        },
        Err(err) => {
            error!("Failed to deserialize version state file: {err}. Treating as first run.");
            VersionState {
                version: current_version.clone(),
            }
            .save(config_dir);
            VersionCheckResult::Init
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        check_app_version, select_reported_app_version, Version, VersionCheckResult,
        VERSION_STATE_FILE_NAME, WELCOME_FORCE_ENV_VAR,
    };

    #[test]
    fn test_reported_app_version_uses_override_when_present() {
        assert_eq!(
            select_reported_app_version("1.6.8", Some("1.6.8-beta1")),
            "1.6.8-beta1"
        );
    }

    #[test]
    fn test_reported_app_version_falls_back_to_package_version_without_override() {
        assert_eq!(select_reported_app_version("1.6.8", None), "1.6.8");
    }

    #[test]
    fn test_reported_app_version_ignores_empty_override() {
        assert_eq!(select_reported_app_version("1.6.8", Some("   ")), "1.6.8");
    }

    #[test]
    fn test_check_app_version_init_when_missing() {
        let dir = tempdir().unwrap();
        let current = Version::new(1, 2, 0);

        let result = check_app_version(dir.path(), &current);

        assert_eq!(result, VersionCheckResult::Init);
        assert!(dir.path().join(VERSION_STATE_FILE_NAME).exists());
    }

    #[test]
    fn test_check_app_version_unchanged_when_same() {
        let dir = tempdir().unwrap();
        let current = Version::new(1, 2, 0);
        let _ = check_app_version(dir.path(), &current);

        let result = check_app_version(dir.path(), &current);

        assert_eq!(result, VersionCheckResult::Unchanged);
    }

    #[test]
    fn test_check_app_version_upgraded_when_current_is_newer() {
        let dir = tempdir().unwrap();
        let previous = Version::new(1, 2, 0);
        let _ = check_app_version(dir.path(), &previous);

        let current = Version::new(1, 3, 0);
        let result = check_app_version(dir.path(), &current);

        assert_eq!(
            result,
            VersionCheckResult::Upgraded {
                previous: previous.clone(),
                current: current.clone(),
            }
        );

        // File should now reflect the new version.
        let result = check_app_version(dir.path(), &current);
        assert_eq!(result, VersionCheckResult::Unchanged);
    }

    #[test]
    fn test_check_app_version_unchanged_on_downgrade() {
        let dir = tempdir().unwrap();
        let previous = Version::new(1, 3, 0);
        let _ = check_app_version(dir.path(), &previous);

        let older = Version::new(1, 2, 0);
        let result = check_app_version(dir.path(), &older);

        assert_eq!(result, VersionCheckResult::Unchanged);

        // File should still hold the higher version, not the downgrade.
        let contents = fs::read_to_string(dir.path().join(VERSION_STATE_FILE_NAME)).unwrap();
        assert!(contents.contains("1.3.0"));
    }

    #[test]
    fn test_check_app_version_corrupt_file_falls_back_to_init() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(VERSION_STATE_FILE_NAME),
            b"{ not valid json",
        )
        .unwrap();

        let current = Version::new(1, 2, 0);
        let result = check_app_version(dir.path(), &current);

        assert_eq!(result, VersionCheckResult::Init);
    }

    #[test]
    fn test_check_app_version_force_upgraded_via_env_var() {
        let dir = tempdir().unwrap();
        let current = Version::new(1, 2, 0);
        let _ = check_app_version(dir.path(), &current);

        for value in ["1", "true", "TRUE"] {
            std::env::set_var(WELCOME_FORCE_ENV_VAR, value);
            let result = check_app_version(dir.path(), &current);
            std::env::remove_var(WELCOME_FORCE_ENV_VAR);

            assert_eq!(
                result,
                VersionCheckResult::Upgraded {
                    previous: current.clone(),
                    current: current.clone(),
                }
            );
        }

        // Flag unset: normal behavior resumes.
        let result = check_app_version(dir.path(), &current);
        assert_eq!(result, VersionCheckResult::Unchanged);
    }
}
