// Match src/pages/client/types.ts.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKey {
    ConnectionChanged,
    InstanceUpdate,
    LocationUpdate,
    AppVersionFetch,
    ConfigChanged,
    DeadConnectionDropped,
    DeadConnectionReconnected,
    ApplicationConfigChanged,
    AddInstance,
    MfaTrigger,
    VersionMismatch,
    UuidMismatch,
    WindowSwapped,
    SessionStateChanged,
    InstanceUpdated,
    MfaOpenIdComplete,
    MfaOpenIdError,
    MfaMobileComplete,
    MfaMobileError,
    TunnelsDisabled,
    TunnelsEnabled,
}

impl From<EventKey> for &'static str {
    fn from(key: EventKey) -> &'static str {
        match key {
            EventKey::ConnectionChanged => "connection-changed",
            EventKey::InstanceUpdate => "instance-update",
            EventKey::LocationUpdate => "location-update",
            EventKey::AppVersionFetch => "app-version-fetch",
            EventKey::ConfigChanged => "config-changed",
            EventKey::DeadConnectionDropped => "dead-connection-dropped",
            EventKey::DeadConnectionReconnected => "dead-connection-reconnected",
            EventKey::ApplicationConfigChanged => "application-config-changed",
            EventKey::AddInstance => "add-instance",
            EventKey::MfaTrigger => "mfa-trigger",
            EventKey::VersionMismatch => "version-mismatch",
            EventKey::UuidMismatch => "uuid-mismatch",
            EventKey::WindowSwapped => "window-swapped",
            EventKey::SessionStateChanged => "session-state-changed",
            EventKey::InstanceUpdated => "instance-updated",
            EventKey::MfaOpenIdComplete => "mfa-openid-complete",
            EventKey::MfaOpenIdError => "mfa-openid-error",
            EventKey::MfaMobileComplete => "mfa-mobile-complete",
            EventKey::MfaMobileError => "mfa-mobile-error",
            EventKey::TunnelsDisabled => "tunnel-disabled-by-policy",
            EventKey::TunnelsEnabled => "tunnel-enabled-by-policy",
        }
    }
}
