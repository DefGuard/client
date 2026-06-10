pub use semver::Version;

pub const MIN_CORE_VERSION: Version = Version::new(1, 6, 0);
pub const MIN_PROXY_VERSION: Version = Version::new(1, 6, 0);
pub const CLIENT_VERSION_HEADER: &str = "defguard-client-version";
pub const CLIENT_PLATFORM_HEADER: &str = "defguard-client-platform";
pub const LOG_FILENAME: &str = "defguard-client";
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

#[cfg(test)]
mod tests {
    use super::select_reported_app_version;

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
}
