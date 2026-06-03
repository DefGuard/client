pub use semver::Version;

pub const MIN_CORE_VERSION: Version = Version::new(1, 6, 0);
pub const MIN_PROXY_VERSION: Version = Version::new(1, 6, 0);
pub const CLIENT_VERSION_HEADER: &str = "defguard-client-version";
pub const CLIENT_PLATFORM_HEADER: &str = "defguard-client-platform";
pub const LOG_FILENAME: &str = "defguard-client";
pub use defguard_client_common::VERSION as PKG_VERSION;
