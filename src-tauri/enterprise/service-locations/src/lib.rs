use std::{collections::HashMap, fmt, fs, path::Path};
#[cfg(any(windows, target_os = "linux"))]
use std::{
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

use defguard_client_core::{
    database::models::{
        location::{Location, ServiceLocationMode},
        Id,
    },
    error::Error as CoreError,
};
#[cfg(any(windows, target_os = "linux"))]
use defguard_client_posture::{
    inspector::{device_posture_data, DiskEncryptionTarget},
    request_posture_authorization,
};
use defguard_client_proto::defguard::client::v1::{
    SaveServiceLocationsRequest, ServiceLocation, ServiceLocationMode as ProtoServiceLocationMode,
};
use defguard_wireguard_rs::{error::WireguardInterfaceError, WGApi};
use log::warn;
#[cfg(any(windows, target_os = "linux"))]
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;

#[derive(Debug, thiserror::Error)]
pub enum ServiceLocationError {
    #[error("Error occurred while initializing service location API: {0}")]
    InitError(String),
    #[error("Failed to load service location storage: {0}")]
    LoadError(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    DecodeError(#[from] base64::DecodeError),
    #[error(transparent)]
    WireGuardError(#[from] WireguardInterfaceError),
    #[error(transparent)]
    AddrParseError(#[from] defguard_wireguard_rs::net::IpAddrParseError),
    #[error("WireGuard interface error: {0}")]
    InterfaceError(String),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    ProtoEnumError(#[from] prost::UnknownEnumValue),
    #[cfg(windows)]
    #[error(transparent)]
    WindowsServiceError(#[from] windows_service::Error),
}

#[allow(dead_code)]
#[derive(Default)]
pub struct ServiceLocationManager {
    // Interface name: WireGuard API instance
    wgapis: HashMap<String, WGApi>,
    // Instance ID: Service locations connected under that instance
    connected_service_locations: HashMap<String, Vec<ServiceLocation>>,
    #[cfg(any(windows, target_os = "linux"))]
    configuration_generation: u64,
    // (Instance ID, location public key): when its posture session was last approved.
    //
    // Kept beside `connected_service_locations` rather than folded into it: the alternative meant
    // changing that map's element type at every one of its call sites, most of them in Windows code
    // that cannot be compiled here. Entries for locations that are no longer connected are pruned
    // during the health check, so the two cannot drift apart for long.
    #[cfg(any(windows, target_os = "linux"))]
    posture_sessions: HashMap<(String, String), SystemTime>,
}

/// Current schema version of the on-disk service location JSON file.
///
/// Files written by older clients predate versioning and deserialize with `schema_version == 0`
/// (see the `#[serde(default)]` on [`ServiceLocationData::schema_version`]).
pub const SERVICE_LOCATION_SCHEMA_VERSION: u32 = 1;

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct ServiceLocationData {
    pub service_locations: Vec<ServiceLocation>,
    pub instance_id: String,
    pub private_key: String,
    #[serde(default)]
    pub proxy_url: String,
    /// The *device's* WireGuard public key (`WireguardKeys.pubkey`), used to identify this device
    /// to the proxy. This is **not** [`ServiceLocation::pubkey`], which is the remote peer key.
    #[serde(default)]
    pub device_pubkey: String,
    /// Device polling token, used to authenticate posture requests made by the service.
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub schema_version: u32,
}

#[allow(dead_code)]
pub struct SingleServiceLocationData {
    pub service_location: ServiceLocation,
    pub instance_id: String,
    pub private_key: String,
}

impl fmt::Debug for ServiceLocationData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceLocationData")
            .field("service_locations", &self.service_locations)
            .field("instance_id", &self.instance_id)
            .field("private_key", &"***")
            .field("proxy_url", &self.proxy_url)
            .field("device_pubkey", &self.device_pubkey)
            .field("token", &self.token.as_ref().map(|_| "***"))
            .field("schema_version", &self.schema_version)
            .finish()
    }
}

impl ServiceLocationData {
    /// Builds the on-disk representation from a daemon save request.
    ///
    /// `service_locations` is passed separately rather than taken from the request because each
    /// platform first filters the requested set down to the modes it supports.
    ///
    /// Every other field is copied from the request here and nowhere else, so a field added to
    /// `SaveServiceLocationsRequest` has exactly one place to be wired in — it cannot be silently
    /// dropped on the way to disk by one platform but not the other.
    #[must_use]
    pub fn from_save_request(
        request: &SaveServiceLocationsRequest,
        service_locations: Vec<ServiceLocation>,
    ) -> Self {
        Self {
            service_locations,
            instance_id: request.instance_id.clone(),
            private_key: request.private_key.clone(),
            proxy_url: request.proxy_url.clone(),
            device_pubkey: request.device_pubkey.clone(),
            token: request.token.clone(),
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        }
    }
}

impl fmt::Debug for SingleServiceLocationData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SingleServiceLocationData")
            .field("service_locations", &self.service_location)
            .field("instance_id", &self.instance_id)
            .field("private_key", &"***")
            .finish()
    }
}

/// Whether the file at `path` already holds exactly `contents`.
///
/// This is what makes a save idempotent. Service locations are pushed on **every** poll cycle so a
/// failed push retries without any bookkeeping, which means the overwhelmingly common case is that
/// nothing changed. Saving is not a cheap no-op by default: it ends by disconnecting and
/// reconnecting every tunnel, so a save that proceeded regardless would drop working tunnels at the
/// poll interval forever. Comparing here lets the caller return before any of that.
///
/// A read failure counts as "differs", so a missing or unreadable file is simply rewritten.
#[must_use]
pub fn is_unchanged_on_disk(path: &Path, contents: &str) -> bool {
    fs::read_to_string(path).is_ok_and(|existing| existing == contents)
}

pub fn to_service_location(location: &Location<Id>) -> Result<ServiceLocation, CoreError> {
    if !location.is_service_location() {
        warn!("Location {location} is not a service location, so it can't be converted to one.");
        return Err(CoreError::ConversionError(format!(
            "Failed to convert location {location} to a service location as it's either not marked \
            as one or has MFA enabled."
        )));
    }

    let mode = match location.service_location_mode {
        ServiceLocationMode::Disabled => {
            warn!(
            "Location {location} has an invalid service location mode, so it can't be converted to \
            one."
        );
            return Err(CoreError::ConversionError(format!(
                "Location {location} has an invalid service location mode ({:?}), so it can't be \
                converted to one.",
                location.service_location_mode
            )));
        }
        ServiceLocationMode::PreLogon => ProtoServiceLocationMode::PreLogon as i32,
        ServiceLocationMode::AlwaysOn => ProtoServiceLocationMode::AlwaysOn as i32,
    };

    Ok(ServiceLocation {
        name: location.name.clone(),
        address: location.address.clone(),
        pubkey: location.pubkey.clone(),
        endpoint: location.endpoint.clone(),
        allowed_ips: location.allowed_ips.clone(),
        dns: location.dns.clone().unwrap_or_default(),
        keepalive_interval: location.keepalive_interval.try_into().unwrap_or(0),
        mode,
        network_id: location.network_id,
        posture_check_required: location.posture_check_required,
    })
}

/// How long a posture session may go without evidence of life before it is renewed.
///
/// Deliberately below core's default `peer_disconnect_threshold` of 300s, so a session is refreshed
/// before core would drop the peer rather than after. A deployment that lowers that threshold below
/// this one would need this plumbed through (D21).
#[cfg(any(windows, target_os = "linux"))]
pub const POSTURE_SESSION_STALE_AFTER: Duration = Duration::from_secs(180);

/// Whether a posture session needs renewing.
///
/// The interface is the only honest source here. A location the daemon believes it connected can be
/// dead: while a machine sleeps, core's `peer_disconnect_threshold` elapses and the gateway drops the
/// peer, leaving an interface that looks perfectly healthy and passes nothing. A handshake is the
/// evidence that the far side still has us.
///
/// `authorized_at` covers the case where no handshake has happened yet, which is normal immediately
/// after connecting and suspicious a few minutes later. It is the one thing here that cannot be
/// recovered from the interface, which is why it has to be remembered.
#[cfg(any(windows, target_os = "linux"))]
#[must_use]
pub fn posture_session_is_stale(
    last_handshake: Option<SystemTime>,
    authorized_at: Option<SystemTime>,
    now: SystemTime,
) -> bool {
    let beyond_threshold = |moment: SystemTime| {
        now.duration_since(moment)
            .is_ok_and(|elapsed| elapsed > POSTURE_SESSION_STALE_AFTER)
    };

    match (last_handshake, authorized_at) {
        // A handshake is the strongest evidence available, so it wins whenever there is one.
        (Some(handshake), _) => beyond_threshold(handshake),
        // Never handshaken: expected just after connecting, suspicious much later.
        (None, Some(authorized)) => beyond_threshold(authorized),
        // Neither, so the daemon has no record of authorizing this at all. Renewing is the safe
        // reading: at worst it is redundant, whereas assuming health leaves a dead tunnel up.
        (None, None) => true,
    }
}

/// A location that cannot be connected until a posture check approves it.
///
/// Carries everything the request needs, so authorization can happen with no lock held. Note
/// `network_id` is core's id for the location, which is what the posture endpoint expects, and
/// `device_pubkey` is this device's key rather than the remote peer's.
#[cfg(any(windows, target_os = "linux"))]
#[derive(Debug)]
pub struct PostureAuthorizationRequest {
    pub instance_id: String,
    pub location_pubkey: String,
    pub location_name: String,
    pub network_id: i64,
    pub proxy_url: String,
    pub device_pubkey: String,
    pub token: Option<String>,
    pub configuration_generation: u64,
}

/// Posture approvals obtained this pass, keyed by (instance id, location public key).
///
/// An absent map entry means authorization failed and leaves the location alone. A present entry
/// with no key means core approved connecting without a PSK because posture checks were removed.
#[cfg(any(windows, target_os = "linux"))]
pub struct PostureAuthorization {
    configuration_generation: u64,
    preshared_key: Option<String>,
}

#[cfg(any(windows, target_os = "linux"))]
pub type PostureAuthorizations = HashMap<(String, String), PostureAuthorization>;

/// What one reconcile pass should do with a persisted service location.
///
/// An authorization is present only when posture authorization succeeded during this pass. Its key
/// may be absent when posture checks were removed from the location.
#[cfg(any(windows, target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconcileAction<'a> {
    LeaveConnected,
    WaitForAuthorization,
    Renew(Option<&'a str>),
    Connect(Option<&'a str>),
}

#[cfg(any(windows, target_os = "linux"))]
#[must_use]
pub(crate) fn reconcile_action(
    is_connected: bool,
    posture_check_required: bool,
    authorization: Option<&PostureAuthorization>,
    configuration_generation: u64,
) -> ReconcileAction<'_> {
    let authorization = authorization
        .filter(|authorization| authorization.configuration_generation == configuration_generation)
        .map(|authorization| authorization.preshared_key.as_deref());

    if is_connected {
        return authorization.map_or(ReconcileAction::LeaveConnected, ReconcileAction::Renew);
    }

    if posture_check_required && authorization.is_none() {
        ReconcileAction::WaitForAuthorization
    } else {
        ReconcileAction::Connect(authorization.flatten())
    }
}

/// Obtains a preshared key for each location that needs one.
///
/// Posture data is collected once per pass rather than once per location: on Windows that is a WMI
/// query, and every location on a machine reports the same posture anyway.
///
/// Failures are logged and skipped, never propagated. A rejected device and an unreachable proxy are
/// treated alike - the location simply is not connected, and the next pass tries again.
#[cfg(any(windows, target_os = "linux"))]
async fn authorize_pending(pending: Vec<PostureAuthorizationRequest>) -> PostureAuthorizations {
    let mut authorizations = PostureAuthorizations::new();
    if pending.is_empty() {
        return authorizations;
    }

    debug!(
        "{} service location(s) need a posture check before they can be connected",
        pending.len()
    );
    let posture_data = device_posture_data(DiskEncryptionTarget::RootFilesystem);

    for request in pending {
        let Some(token) = request.token.clone().filter(|token| !token.is_empty()) else {
            error!(
                "Cannot run a posture check for service location '{}': no polling token was stored \
                for its instance. Re-enrolling the device will store one.",
                request.location_name
            );
            continue;
        };

        match request_posture_authorization(
            &request.proxy_url,
            request.device_pubkey.clone(),
            request.network_id,
            token,
            posture_data.clone(),
        )
        .await
        {
            Ok(preshared_key) => {
                info!(
                    "Posture check approved for service location '{}'",
                    request.location_name
                );
                authorizations.insert(
                    (request.instance_id, request.location_pubkey),
                    PostureAuthorization {
                        configuration_generation: request.configuration_generation,
                        preshared_key,
                    },
                );
            }
            Err(err) => error!(
                "Posture check failed for service location '{}': {err}. It will stay disconnected \
                and be retried.",
                request.location_name
            ),
        }
    }

    authorizations
}

#[cfg(any(windows, target_os = "linux"))]
impl ServiceLocationManager {
    pub(crate) fn note_configuration_changed(&mut self) {
        self.configuration_generation = self.configuration_generation.wrapping_add(1);
    }

    /// Forgets posture sessions for locations that are no longer connected.
    ///
    /// Keeps this map from being a second, drifting source of truth: a location that is removed or
    /// disconnected would otherwise leave its timestamp behind forever, and a location reconnected
    /// later would inherit it and look healthier than it is.
    pub(crate) fn prune_posture_sessions(&mut self) {
        self.posture_sessions.retain(|(instance_id, pubkey), _| {
            self.connected_service_locations
                .get(instance_id)
                .is_some_and(|locations| {
                    locations.iter().any(|location| location.pubkey == *pubkey)
                })
        });
    }
}

/// Signal used to wake the reconciler before its next tick.
///
/// `notify_one` is callable from synchronous code, which matters because the Windows watchers are
/// plain OS threads wrapping blocking syscalls. A wake that arrives while a pass is already running
/// is remembered rather than dropped, so an event can never be missed by arriving at a bad moment.
#[cfg(any(windows, target_os = "linux"))]
pub type ReconcileSignal = std::sync::Arc<tokio::sync::Notify>;

/// Brings the running tunnels in line with what is on disk, forever.
///
/// Replaces a retry loop that gave up permanently after a fixed number of attempts, which left a
/// machine that booted before its network was ready disconnected until the service restarted. This
/// never gives up: every tick it looks at what should be running and fixes the difference.
///
/// Each pass is idempotent - already-correct locations are left alone - so waking it spuriously
/// costs nothing, and callers are free to wake it whenever something *might* have changed rather
/// than working out whether it did.
///
/// `wake` is the only way to react faster than `tick`. On Windows it is signalled by the network,
/// logon and resume watchers. **On Linux nothing signals it**, so there the tick is the sole trigger
/// and recovery from any disruption takes up to one interval.
#[cfg(any(windows, target_os = "linux"))]
pub async fn run_reconciler(
    manager: Arc<RwLock<ServiceLocationManager>>,
    wake: ReconcileSignal,
    tick: Duration,
) {
    info!("Service location reconciler started, reconciling every {tick:?}");

    loop {
        // Authorize first, mutate second. Working out what needs a posture check takes only a read
        // guard, the checks themselves are HTTP round trips of up to 5s each and are made with no
        // guard at all, and only the final step takes the write guard.
        //
        // The ordering is enforced rather than merely intended: these are `std` guards, so they are
        // `!Send` and holding one across the await below would not compile.
        let pending = {
            let manager = manager.read().unwrap();
            manager.locations_needing_authorization()
        };

        let authorizations = authorize_pending(pending).await;

        let outcome = {
            let mut manager = manager.write().unwrap();
            manager.reconcile(&authorizations)
        };

        match outcome {
            Ok(true) => debug!("Service locations reconciled, everything is as it should be"),
            Ok(false) => warn!(
                "Service location reconcile pass completed with failures, retrying in {tick:?}"
            ),
            Err(err) => {
                warn!("Service location reconcile pass failed: {err}. Retrying in {tick:?}");
            }
        }

        tokio::select! {
            () = tokio::time::sleep(tick) => {}
            () = wake.notified() => debug!("Service location reconciler woken early by an event"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(windows, target_os = "linux"))]
    mod staleness {
        use super::*;

        fn ago(seconds: u64) -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000 - seconds)
        }

        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000)
        }

        #[test]
        fn a_recent_handshake_is_healthy() {
            assert!(!posture_session_is_stale(
                Some(ago(10)),
                Some(ago(10_000)),
                now()
            ));
        }

        /// A handshake outranks `authorized_at`: the far side has stopped answering, and having
        /// authorized recently does not make the tunnel work.
        #[test]
        fn an_old_handshake_is_stale_even_if_just_authorized() {
            assert!(posture_session_is_stale(
                Some(ago(1_000)),
                Some(ago(1)),
                now()
            ));
        }

        /// Expected right after connecting - there has been no traffic to handshake for yet.
        #[test]
        fn no_handshake_yet_is_healthy_if_authorized_recently() {
            assert!(!posture_session_is_stale(None, Some(ago(10)), now()));
        }

        /// The suspend case: authorized long ago, never handshaken, so nothing says it works.
        #[test]
        fn no_handshake_long_after_authorizing_is_stale() {
            assert!(posture_session_is_stale(None, Some(ago(1_000)), now()));
        }

        /// No record at all. Renewing is redundant at worst; assuming health leaves a dead tunnel up.
        #[test]
        fn no_evidence_at_all_is_stale() {
            assert!(posture_session_is_stale(None, None, now()));
        }

        /// A clock that moved backwards must not read as "ancient", which would renew every pass.
        #[test]
        fn a_handshake_in_the_future_is_not_stale() {
            let future = now() + Duration::from_secs(60);
            assert!(!posture_session_is_stale(Some(future), None, now()));
        }

        #[test]
        fn the_threshold_boundary_is_not_yet_stale() {
            assert!(!posture_session_is_stale(
                Some(now() - POSTURE_SESSION_STALE_AFTER),
                None,
                now()
            ));
        }
    }

    #[cfg(any(windows, target_os = "linux"))]
    mod reconciliation {
        use super::*;

        fn authorization(
            configuration_generation: u64,
            preshared_key: Option<&str>,
        ) -> PostureAuthorization {
            PostureAuthorization {
                configuration_generation,
                preshared_key: preshared_key.map(str::to_string),
            }
        }

        #[test]
        fn connected_location_with_fresh_key_is_renewed() {
            let authorization = authorization(1, Some("fresh-key"));
            assert_eq!(
                reconcile_action(true, true, Some(&authorization), 1),
                ReconcileAction::Renew(Some("fresh-key"))
            );
        }

        #[test]
        fn approval_without_a_key_connects_without_a_key() {
            let authorization = authorization(1, None);
            assert_eq!(
                reconcile_action(false, true, Some(&authorization), 1),
                ReconcileAction::Connect(None)
            );
        }

        #[test]
        fn approval_without_a_key_removes_the_old_key_from_a_connected_location() {
            let authorization = authorization(1, None);
            assert_eq!(
                reconcile_action(true, true, Some(&authorization), 1),
                ReconcileAction::Renew(None)
            );
        }

        #[test]
        fn authorization_failure_keeps_a_posture_location_disconnected() {
            assert_eq!(
                reconcile_action(false, true, None, 1),
                ReconcileAction::WaitForAuthorization
            );
        }

        #[test]
        fn authorization_from_an_older_configuration_is_discarded() {
            let authorization = authorization(1, Some("stale-key"));
            assert_eq!(
                reconcile_action(false, true, Some(&authorization), 2),
                ReconcileAction::WaitForAuthorization
            );
        }
    }

    /// A save is a no-op only if this comparison is exact: getting it wrong does not merely cost a
    /// disk write, it drops and rebuilds every tunnel on the box.
    #[test]
    fn unchanged_contents_are_detected() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("locations.json");

        assert!(
            !is_unchanged_on_disk(&path, "first"),
            "a missing file must count as changed"
        );

        fs::write(&path, "first").expect("failed to write");
        assert!(
            is_unchanged_on_disk(&path, "first"),
            "identical contents must be detected as unchanged"
        );
        assert!(
            !is_unchanged_on_disk(&path, "second"),
            "different contents must be detected as changed"
        );
        assert!(
            !is_unchanged_on_disk(&path, "first "),
            "a trailing-whitespace difference must still count as changed"
        );
    }

    /// JSON exactly as written by a client that predates posture checks: no `proxy_url`,
    /// `device_pubkey`, `token` or `schema_version` at the top level, and no `network_id` or
    /// `posture_check_required` inside the locations. This is the upgrade path.
    const LEGACY_JSON: &str = r#"{
      "service_locations": [
        {
          "name": "Office",
          "address": "10.0.0.2/24",
          "pubkey": "remote-peer-pubkey",
          "endpoint": "vpn.example.com:51820",
          "allowed_ips": "10.0.0.0/24",
          "keepalive_interval": 25,
          "dns": "10.0.0.1",
          "mode": 2
        }
      ],
      "instance_id": "d3a5b1f0-0000-0000-0000-000000000001",
      "private_key": "device-private-key"
    }"#;

    #[test]
    fn legacy_json_without_new_fields_deserializes_with_defaults() {
        let data: ServiceLocationData = serde_json::from_str(LEGACY_JSON)
            .expect("legacy service location file must still load");

        assert_eq!(data.instance_id, "d3a5b1f0-0000-0000-0000-000000000001");
        assert_eq!(data.private_key, "device-private-key");
        assert_eq!(data.proxy_url, "");
        assert_eq!(data.device_pubkey, "");
        assert_eq!(data.token, None);
        // 0 marks a file written before schema versioning existed.
        assert_eq!(data.schema_version, 0);

        assert_eq!(data.service_locations.len(), 1);
        let location = &data.service_locations[0];
        assert_eq!(location.name, "Office");
        assert_eq!(location.pubkey, "remote-peer-pubkey");
        assert_eq!(location.network_id, 0);
        assert!(!location.posture_check_required);
    }

    #[test]
    fn truncated_json_still_fails_to_deserialize() {
        // A container-level `#[serde(default)]` on `ServiceLocation` would let malformed entries
        // silently vanish; make sure missing required keys are still an error.
        let json = r#"{
          "service_locations": [{ "network_id": 7 }],
          "instance_id": "id",
          "private_key": "key"
        }"#;

        assert!(serde_json::from_str::<ServiceLocationData>(json).is_err());
    }

    #[test]
    fn round_trip_preserves_new_fields() {
        let data = ServiceLocationData {
            service_locations: vec![ServiceLocation {
                name: "Office".into(),
                address: "10.0.0.2/24".into(),
                pubkey: "remote-peer-pubkey".into(),
                endpoint: "vpn.example.com:51820".into(),
                allowed_ips: "10.0.0.0/24".into(),
                keepalive_interval: 25,
                dns: "10.0.0.1".into(),
                mode: ProtoServiceLocationMode::AlwaysOn as i32,
                network_id: 42,
                posture_check_required: true,
            }],
            instance_id: "instance-uuid".into(),
            private_key: "device-private-key".into(),
            proxy_url: "https://proxy.example.com".into(),
            device_pubkey: "device-public-key".into(),
            token: Some("polling-token".into()),
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        };

        let json = serde_json::to_string(&data).expect("serialization must succeed");
        let restored: ServiceLocationData =
            serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(restored.proxy_url, "https://proxy.example.com");
        assert_eq!(restored.device_pubkey, "device-public-key");
        assert_eq!(restored.token.as_deref(), Some("polling-token"));
        assert_eq!(restored.schema_version, SERVICE_LOCATION_SCHEMA_VERSION);
        assert_eq!(restored.service_locations[0].network_id, 42);
        assert!(restored.service_locations[0].posture_check_required);
        // The remote peer key must not be confused with the device key.
        assert_eq!(restored.service_locations[0].pubkey, "remote-peer-pubkey");
    }

    #[test]
    fn debug_masks_private_key_and_token() {
        let data = ServiceLocationData {
            service_locations: Vec::new(),
            instance_id: "instance-uuid".into(),
            private_key: "super-secret-private-key".into(),
            proxy_url: "https://proxy.example.com".into(),
            device_pubkey: "device-public-key".into(),
            token: Some("super-secret-token".into()),
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        };

        let debug = format!("{data:?}");
        assert!(!debug.contains("super-secret-private-key"), "{debug}");
        assert!(!debug.contains("super-secret-token"), "{debug}");
        // Non-secret fields are still visible for diagnostics.
        assert!(debug.contains("https://proxy.example.com"), "{debug}");
        assert!(debug.contains("device-public-key"), "{debug}");
    }

    #[test]
    fn debug_of_absent_token_is_not_masked_as_present() {
        let data = ServiceLocationData {
            service_locations: Vec::new(),
            instance_id: "instance-uuid".into(),
            private_key: "private".into(),
            proxy_url: String::new(),
            device_pubkey: String::new(),
            token: None,
            schema_version: SERVICE_LOCATION_SCHEMA_VERSION,
        };

        assert!(format!("{data:?}").contains("token: None"));
    }

    #[test]
    fn single_service_location_debug_masks_private_key() {
        let data = SingleServiceLocationData {
            service_location: ServiceLocation {
                name: "Office".into(),
                address: "10.0.0.2/24".into(),
                pubkey: "remote-peer-pubkey".into(),
                endpoint: "vpn.example.com:51820".into(),
                allowed_ips: "10.0.0.0/24".into(),
                keepalive_interval: 25,
                dns: "10.0.0.1".into(),
                mode: ProtoServiceLocationMode::AlwaysOn as i32,
                network_id: 42,
                posture_check_required: true,
            },
            instance_id: "instance-uuid".into(),
            private_key: "super-secret-private-key".into(),
        };

        assert!(!format!("{data:?}").contains("super-secret-private-key"));
    }
}
