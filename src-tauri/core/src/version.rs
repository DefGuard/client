use std::{
    cmp::Ordering,
    env,
    fs::{create_dir_all, File, OpenOptions},
    path::{Path, PathBuf},
};

use reqwest::header::HeaderMap;
use semver::Prerelease;
pub use semver::Version;
use serde::{Deserialize, Serialize};

use crate::mfa_contract::MfaContract;
#[cfg(unix)]
use crate::set_perms;

pub const MIN_CORE_VERSION: Version = Version::new(1, 6, 0);
pub const MIN_PROXY_VERSION: Version = Version::new(1, 6, 0);
pub const MIN_MULTI_STEP_MFA_VERSION: Version = Version::new(2, 2, 0);
pub const CORE_VERSION_HEADER: &str = "defguard-core-version";
pub const PROXY_VERSION_HEADER: &str = "defguard-component-version";
pub const CORE_CONNECTED_HEADER: &str = "defguard-core-connected";
pub const CLIENT_VERSION_HEADER: &str = "defguard-client-version";
pub const CLIENT_PLATFORM_HEADER: &str = "defguard-client-platform";
pub const LOG_FILENAME: &str = "defguard-client";
pub const WELCOME_FORCE_ENV_VAR: &str = "DEFGUARD_CLIENT_WELCOME_FORCE";
pub const WELCOME_SKIP_ENV_VAR: &str = "DEFGUARD_CLIENT_WELCOME_SKIP";
pub const WELCOME_CONTENT_VERSION: Version = Version::new(2, 1, 0);
pub use defguard_client_common::VERSION as PKG_VERSION;

/// Returns whether `version` meets or exceeds `minimum`, ignoring pre-release and build metadata.
#[must_use]
pub fn is_version_at_least(version: &Version, minimum: &Version) -> bool {
    let (mut version, mut minimum) = (version.clone(), minimum.clone());
    version.pre = Prerelease::EMPTY;
    minimum.pre = Prerelease::EMPTY;
    version.cmp_precedence(&minimum) != Ordering::Less
}

/// Resolves the MFA contract advertised by a proxy response, failing closed to legacy.
#[must_use]
pub fn mfa_contract_from_headers(headers: &HeaderMap) -> MfaContract {
    let core_connected = headers
        .get(CORE_CONNECTED_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "true");
    if !core_connected {
        return MfaContract::Legacy;
    }

    let compatible = [CORE_VERSION_HEADER, PROXY_VERSION_HEADER]
        .into_iter()
        .all(|header| {
            headers
                .get(header)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<Version>().ok())
                .is_some_and(|version| is_version_at_least(&version, &MIN_MULTI_STEP_MFA_VERSION))
        });

    if compatible {
        MfaContract::MultiStep
    } else {
        MfaContract::Legacy
    }
}

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
    #[serde(default)]
    welcome_shown: Option<Version>,
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
    env::var(WELCOME_FORCE_ENV_VAR).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn welcome_skip_enabled() -> bool {
    env::var(WELCOME_SKIP_ENV_VAR).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Checks the last known app version (persisted in `config_dir`) against `current_version`,
/// updating the on-disk state as needed.
///
/// Meant to be called exactly once, synchronously, during app setup.
#[must_use]
pub fn check_app_version(config_dir: &Path, current_version: &Version) -> VersionCheckResult {
    if welcome_skip_enabled() {
        return VersionCheckResult::Unchanged;
    }

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
            welcome_shown: None,
        }
        .save(config_dir);
        return VersionCheckResult::Init;
    }

    let file = get_version_state_file(config_dir, false);
    match serde_json::from_reader::<_, VersionState>(file) {
        Ok(state) => match state.version.cmp(current_version) {
            Ordering::Equal | Ordering::Greater => VersionCheckResult::Unchanged,
            Ordering::Less => {
                let previous = state.version;
                VersionState {
                    version: current_version.clone(),
                    welcome_shown: state.welcome_shown,
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
                welcome_shown: None,
            }
            .save(config_dir);
            VersionCheckResult::Init
        }
    }
}

fn read_version_state(config_dir: &Path) -> Option<VersionState> {
    let path = get_version_state_file_path(config_dir);
    if !path.exists() {
        return None;
    }
    let file = get_version_state_file(config_dir, false);
    serde_json::from_reader::<_, VersionState>(file).ok()
}

#[must_use]
pub fn should_show_welcome(config_dir: &Path) -> bool {
    if welcome_skip_enabled() {
        return false;
    }

    if welcome_force_enabled() {
        return true;
    }

    read_version_state(config_dir)
        .and_then(|state| state.welcome_shown)
        .is_none_or(|shown| shown < WELCOME_CONTENT_VERSION)
}

pub fn mark_welcome_shown(config_dir: &Path, current_version: &Version) {
    let mut state = read_version_state(config_dir).unwrap_or(VersionState {
        version: current_version.clone(),
        welcome_shown: None,
    });
    state.welcome_shown = Some(WELCOME_CONTENT_VERSION);
    state.save(config_dir);
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use reqwest::header::{HeaderMap, HeaderValue};
    use tempfile::tempdir;

    use super::{
        check_app_version, is_version_at_least, mark_welcome_shown, mfa_contract_from_headers,
        select_reported_app_version, should_show_welcome, Version, VersionCheckResult,
        CORE_CONNECTED_HEADER, CORE_VERSION_HEADER, MIN_MULTI_STEP_MFA_VERSION,
        PROXY_VERSION_HEADER, VERSION_STATE_FILE_NAME, WELCOME_FORCE_ENV_VAR, WELCOME_SKIP_ENV_VAR,
    };
    use crate::mfa_contract::MfaContract;

    #[test]
    fn test_should_show_welcome_when_state_file_missing() {
        let dir = tempdir().unwrap();

        assert!(should_show_welcome(dir.path()));
    }

    #[test]
    fn test_should_show_welcome_when_never_marked() {
        let dir = tempdir().unwrap();
        let _ = check_app_version(dir.path(), &Version::new(2, 1, 0));

        assert!(should_show_welcome(dir.path()));
    }

    #[test]
    fn test_should_not_show_welcome_after_marking() {
        let dir = tempdir().unwrap();
        let current = Version::new(2, 1, 0);
        let _ = check_app_version(dir.path(), &current);

        mark_welcome_shown(dir.path(), &current);

        assert!(!should_show_welcome(dir.path()));
    }

    #[test]
    fn test_should_show_welcome_when_marked_below_content_version() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(VERSION_STATE_FILE_NAME),
            br#"{"version":"2.1.0","welcome_shown":"2.0.0"}"#,
        )
        .unwrap();

        assert!(should_show_welcome(dir.path()));
    }

    #[test]
    fn test_check_app_version_preserves_welcome_shown_on_upgrade() {
        let dir = tempdir().unwrap();
        let previous = Version::new(2, 1, 0);
        let _ = check_app_version(dir.path(), &previous);
        mark_welcome_shown(dir.path(), &previous);

        let current = Version::new(2, 1, 1);
        let result = check_app_version(dir.path(), &current);

        assert_eq!(result, VersionCheckResult::Upgraded { previous, current });
        assert!(!should_show_welcome(dir.path()));
    }

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

    fn mfa_headers(
        core_version: Option<&str>,
        proxy_version: Option<&str>,
        core_connected: Option<&str>,
    ) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in [
            (CORE_VERSION_HEADER, core_version),
            (PROXY_VERSION_HEADER, proxy_version),
            (CORE_CONNECTED_HEADER, core_connected),
        ] {
            if let Some(value) = value {
                headers.insert(name, HeaderValue::from_str(value).unwrap());
            }
        }
        headers
    }

    #[test]
    fn test_mfa_contract_from_headers_accepts_connected_supported_versions() {
        let headers = mfa_headers(Some("2.2.0-alpha1"), Some("2.2.0"), Some("true"));

        assert_eq!(mfa_contract_from_headers(&headers), MfaContract::MultiStep);
    }

    #[test]
    fn test_mfa_contract_from_headers_requires_connected_core() {
        for core_connected in [None, Some("false"), Some("TRUE"), Some("invalid")] {
            let headers = mfa_headers(Some("2.2.0"), Some("2.2.0"), core_connected);

            assert_eq!(
                mfa_contract_from_headers(&headers),
                MfaContract::Legacy,
                "core-connected={core_connected:?}"
            );
        }
    }

    #[test]
    fn test_mfa_contract_from_headers_requires_supported_versions() {
        let cases = [
            ("missing core version", None, Some("2.2.0")),
            ("missing proxy version", Some("2.2.0"), None),
            ("invalid core version", Some("invalid"), Some("2.2.0")),
            ("invalid proxy version", Some("2.2.0"), Some("invalid")),
            ("old core version", Some("2.1.9"), Some("2.2.0")),
            ("old proxy version", Some("2.2.0"), Some("2.1.9")),
        ];

        for (case, core_version, proxy_version) in cases {
            let headers = mfa_headers(core_version, proxy_version, Some("true"));

            assert_eq!(
                mfa_contract_from_headers(&headers),
                MfaContract::Legacy,
                "{case}"
            );
        }
    }

    #[test]
    fn test_version_at_least_ignores_prerelease() {
        let version = Version::parse("2.2.0-alpha1").unwrap();

        assert!(is_version_at_least(&version, &MIN_MULTI_STEP_MFA_VERSION));
    }

    #[test]
    fn test_version_at_least_rejects_older_version() {
        let version = Version::parse("2.1.9").unwrap();

        assert!(!is_version_at_least(&version, &MIN_MULTI_STEP_MFA_VERSION));
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
            env::set_var(WELCOME_FORCE_ENV_VAR, value);
            let result = check_app_version(dir.path(), &current);
            env::remove_var(WELCOME_FORCE_ENV_VAR);

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

    #[test]
    fn test_check_app_version_skip_via_env_var() {
        let dir = tempdir().unwrap();
        let previous = Version::new(1, 2, 0);
        let _ = check_app_version(dir.path(), &previous);

        let current = Version::new(1, 3, 0);
        for value in ["1", "true", "TRUE"] {
            env::set_var(WELCOME_SKIP_ENV_VAR, value);
            let result = check_app_version(dir.path(), &current);
            env::remove_var(WELCOME_SKIP_ENV_VAR);

            assert_eq!(result, VersionCheckResult::Unchanged);
        }

        // Flag unset: normal behavior resumes, upgrade is detected.
        let result = check_app_version(dir.path(), &current);
        assert_eq!(
            result,
            VersionCheckResult::Upgraded {
                previous: previous.clone(),
                current: current.clone(),
            }
        );
    }
}
