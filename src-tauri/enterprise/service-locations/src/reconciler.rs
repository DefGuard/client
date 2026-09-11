//! Keeps runtime service-location tunnels aligned with persisted configuration.
//!
//! A one-shot connection attempt is insufficient: the daemon may start before networking or DNS
//! is ready, a suspend may leave an interface up after the gateway has discarded its peer, and
//! platform event watchers may miss notifications. Posture-gated locations also need periodic
//! authorization renewal. The reconciler therefore retries forever on a timer and can be woken
//! early by platform events. Each pass is idempotent, so redundant wakeups are harmless.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

use defguard_client_core::error::Error as CoreError;
use defguard_client_posture::{
    inspector::{device_posture_data, DiskEncryptionTarget},
    request_posture_authorization,
};
use defguard_client_proto::defguard::client::v1::ServiceLocation;
use futures_util::{stream, StreamExt};
use log::{debug, error, info, warn};

use crate::{ServiceLocationData, ServiceLocationManager};

/// How long a posture session may go without evidence of life before it is renewed.
/// Deliberately below core's default `peer_disconnect_threshold` of 300s, so a session is refreshed
/// before core would drop the peer rather than after.
pub const POSTURE_SESSION_STALE_AFTER: Duration = Duration::from_secs(180);
/// Prevents a large configuration from flooding the proxy while avoiding serial timeout delays.
const MAX_CONCURRENT_POSTURE_AUTHORIZATIONS: usize = 8;

/// Whether a posture session needs renewing.
///
/// The interface is the only honest source here. A location the daemon believes it connected can
/// be dead: while a machine sleeps, core's `peer_disconnect_threshold` elapses and the gateway
/// drops the peer, leaving an interface that looks perfectly healthy and passes nothing. A
/// handshake is the evidence that the far side still has us.
///
/// `authorized_at` covers the case where no handshake has happened yet, which is normal immediately
/// after connecting and suspicious a few minutes later. It is the one thing here that cannot be
/// recovered from the interface, which is why it has to be remembered.
#[must_use]
pub(crate) fn posture_session_is_stale(
    last_handshake: Option<SystemTime>,
    authorized_at: Option<SystemTime>,
    now: SystemTime,
) -> bool {
    // Linux represents a peer that has never handshaken as the Unix epoch.
    let last_handshake = last_handshake.filter(|handshake| *handshake != SystemTime::UNIX_EPOCH);
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
#[derive(Debug)]
pub(crate) struct PostureAuthorizationRequest {
    pub instance_id: String,
    pub location_pubkey: String,
    pub location_name: String,
    pub core_location_id: i64,
    pub proxy_url: String,
    pub device_pubkey: String,
    pub token: Option<String>,
}

impl PostureAuthorizationRequest {
    fn new(instance: &ServiceLocationData, location: &ServiceLocation) -> Self {
        Self {
            instance_id: instance.instance_id.clone(),
            location_pubkey: location.pubkey.clone(),
            location_name: location.name.clone(),
            core_location_id: location.core_location_id,
            proxy_url: instance.proxy_url.clone(),
            device_pubkey: instance.device_pubkey.clone(),
            token: instance.token.clone(),
        }
    }
}

impl ServiceLocationManager {
    /// Collects posture-gated locations that should currently be connected and need authorization.
    ///
    /// Platform modules supply connection eligibility and handshake lookup while this method owns
    /// the common persisted-data traversal and posture-session staleness policy.
    pub(crate) fn collect_posture_authorization_requests(
        &self,
        data: &[ServiceLocationData],
        should_be_connected: impl Fn(&ServiceLocation) -> bool,
        last_handshake: impl Fn(&ServiceLocation) -> Option<SystemTime>,
    ) -> Vec<PostureAuthorizationRequest> {
        let mut pending = Vec::new();

        for instance in data {
            for location in &instance.service_locations {
                if !location.posture_check_required || !should_be_connected(location) {
                    continue;
                }

                if let Some(connected) =
                    self.connected_service_location(&instance.instance_id, &location.pubkey)
                {
                    let handshake = last_handshake(location);
                    if !posture_session_is_stale(
                        handshake,
                        connected.authorized_at,
                        SystemTime::now(),
                    ) {
                        continue;
                    }

                    debug!(
                        "Posture session for service location '{}' looks stale (last handshake: \
                        {handshake:?}), it will be renewed",
                        location.name
                    );
                }

                pending.push(PostureAuthorizationRequest::new(instance, location));
            }
        }

        pending
    }
}

/// What one reconcile pass should do with a persisted service location.
///
/// An authorization records a definitive posture outcome from this pass. An absent outcome means
/// the request failed transiently, while an approval key may be absent when posture checks were
/// removed from the location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconcileAction<'a> {
    LeaveConnected,
    LeaveDisconnected,
    WaitForAuthorization,
    Disconnect,
    Renew(Option<&'a str>),
    Connect(Option<&'a str>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PostureAuthorization {
    Approved(Option<String>),
    Rejected,
}

#[must_use]
pub(crate) fn reconcile_action(
    is_connected: bool,
    posture_check_required: bool,
    authorization: Option<&PostureAuthorization>,
) -> ReconcileAction<'_> {
    if posture_check_required && matches!(authorization, Some(PostureAuthorization::Rejected)) {
        return if is_connected {
            ReconcileAction::Disconnect
        } else {
            ReconcileAction::LeaveDisconnected
        };
    }

    let approved_key = match authorization {
        Some(PostureAuthorization::Approved(preshared_key)) => Some(preshared_key.as_deref()),
        Some(PostureAuthorization::Rejected) | None => None,
    };

    if is_connected {
        return approved_key.map_or(ReconcileAction::LeaveConnected, ReconcileAction::Renew);
    }

    if posture_check_required && approved_key.is_none() {
        ReconcileAction::WaitForAuthorization
    } else {
        ReconcileAction::Connect(approved_key.flatten())
    }
}

/// Posture outcomes obtained this pass, keyed by (instance id, location public key).
///
/// An absent map entry means a transient failure and leaves an existing location alone. An approval
/// with no key means core removed posture checks; a rejection tears an existing location down.
pub(crate) type PostureAuthorizations = HashMap<(String, String), PostureAuthorization>;

/// Obtains a definitive posture outcome for each location that needs one.
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

    // Run checks concurrently so one slow proxy does not delay every posture-gated location.
    let requests = stream::iter(pending.into_iter().map(|request| {
        let posture_data = posture_data.clone();
        async move {
            let Some(token) = request.token.clone().filter(|token| !token.is_empty()) else {
                error!(
                    "Cannot run a posture check for service location '{}': no polling token was \
                    stored for its instance. Re-enrolling the device will store one.",
                    request.location_name
                );
                return None;
            };

            match request_posture_authorization(
                &request.proxy_url,
                request.device_pubkey.clone(),
                request.core_location_id,
                token,
                posture_data,
            )
            .await
            {
                Ok(preshared_key) => {
                    info!(
                        "Posture check approved for service location '{}'",
                        request.location_name
                    );
                    Some((
                        (request.instance_id, request.location_pubkey),
                        PostureAuthorization::Approved(preshared_key),
                    ))
                }
                Err(CoreError::PostureCheckFailed(reason)) => {
                    error!(
                        "Posture check rejected for service location '{}': {reason}. Any existing \
                        tunnel will be disconnected.",
                        request.location_name
                    );
                    Some((
                        (request.instance_id, request.location_pubkey),
                        PostureAuthorization::Rejected,
                    ))
                }
                Err(err) => {
                    error!(
                        "Posture check could not be completed for service location '{}': {err}. \
                        Existing tunnel state will be preserved and the check retried.",
                        request.location_name
                    );
                    None
                }
            }
        }
    }))
    .buffer_unordered(MAX_CONCURRENT_POSTURE_AUTHORIZATIONS);
    futures_util::pin_mut!(requests);

    while let Some(authorization) = requests.next().await {
        if let Some((location, outcome)) = authorization {
            authorizations.insert(location, outcome);
        }
    }

    authorizations
}

/// Signal used to wake the reconciler before its next tick.
///
/// `notify_one` is callable from synchronous code, which matters because the Windows watchers are
/// plain OS threads wrapping blocking syscalls. A wake that arrives while a pass is already running
/// is remembered rather than dropped, so an event can never be missed by arriving at a bad moment.
pub type ReconcileSignal = Arc<tokio::sync::Notify>;

/// Runs a loop that brings the running tunnels in line with what is on disk, forever.
///
/// Each pass is idempotent - already-correct locations are left alone - so waking it spuriously
/// costs nothing, and callers are free to wake it whenever something *might* have changed rather
/// than working out whether it did.
///
/// `wake` is the only way to react faster than `tick`. On Windows it is signalled by the network,
/// logon and resume watchers. **On Linux nothing signals it**, so there the tick is the sole
/// trigger and recovery from any disruption takes up to one interval.
pub async fn run_reconciler(
    manager: Arc<RwLock<ServiceLocationManager>>,
    wake: ReconcileSignal,
    tick: Duration,
) {
    info!("Service location reconciler started, reconciling every {tick:?}");

    loop {
        // Reconcile regular locations and login-dependent teardown before posture HTTP calls. With
        // no authorizations, connected posture locations are left alone and disconnected ones wait.
        let initial_outcome = {
            let mut manager_guard = manager
                .write()
                .expect("Failed to write-lock service location manager");
            manager_guard.reconcile(&PostureAuthorizations::new())
        };

        let pending = {
            let manager_guard = manager
                .read()
                .expect("Failed to read-lock service location manager");
            manager_guard.locations_needing_authorization()
        };

        let outcome = if pending.is_empty() {
            initial_outcome
        } else {
            let authorizations = authorize_pending(pending).await;
            let mut manager_guard = manager
                .write()
                .expect("Failed to write-lock service location manager");
            manager_guard.reconcile(&authorizations)
        };

        match outcome {
            Ok(true) => debug!("Service locations reconciled"),
            Ok(false) => warn!(
                "Service location reconcile pass completed with failures, retrying in {tick:?}"
            ),
            Err(err) => {
                error!("Service location reconcile pass failed: {err}. Retrying in {tick:?}");
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

    fn ago(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000 - seconds)
    }

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000)
    }

    #[test]
    fn test_a_recent_handshake_is_healthy() {
        assert!(!posture_session_is_stale(
            Some(ago(10)),
            Some(ago(10_000)),
            now()
        ));
    }

    /// A handshake outranks `authorized_at`: the far side has stopped answering, and having
    /// authorized recently does not make the tunnel work.
    #[test]
    fn test_an_old_handshake_is_stale_even_if_just_authorized() {
        assert!(posture_session_is_stale(
            Some(ago(1_000)),
            Some(ago(1)),
            now()
        ));
    }

    /// Expected right after connecting - there has been no traffic to handshake for yet.
    #[test]
    fn test_no_handshake_yet_is_healthy_if_authorized_recently() {
        assert!(!posture_session_is_stale(None, Some(ago(10)), now()));
    }

    #[test]
    fn test_epoch_handshake_falls_back_to_recent_authorization() {
        assert!(!posture_session_is_stale(
            Some(SystemTime::UNIX_EPOCH),
            Some(ago(10)),
            now()
        ));
    }

    /// The suspend case: authorized long ago, never handshaken, so nothing says it works.
    #[test]
    fn test_no_handshake_long_after_authorizing_is_stale() {
        assert!(posture_session_is_stale(None, Some(ago(1_000)), now()));
    }

    /// No record at all. Renewing is redundant at worst; assuming health leaves a dead tunnel up.
    #[test]
    fn test_no_evidence_at_all_is_stale() {
        assert!(posture_session_is_stale(None, None, now()));
    }

    /// A clock that moved backwards must not read as "ancient", which would renew every pass.
    #[test]
    fn test_a_handshake_in_the_future_is_not_stale() {
        let future = now() + Duration::from_secs(60);
        assert!(!posture_session_is_stale(Some(future), None, now()));
    }

    #[test]
    fn test_the_threshold_boundary_is_not_yet_stale() {
        assert!(!posture_session_is_stale(
            Some(now() - POSTURE_SESSION_STALE_AFTER),
            None,
            now()
        ));
    }

    #[test]
    fn test_connected_location_with_fresh_key_is_renewed() {
        let authorization = PostureAuthorization::Approved(Some("fresh-key".to_string()));
        assert_eq!(
            reconcile_action(true, true, Some(&authorization)),
            ReconcileAction::Renew(Some("fresh-key"))
        );
    }

    #[test]
    fn test_approval_without_a_key_connects_without_a_key() {
        let authorization = PostureAuthorization::Approved(None);
        assert_eq!(
            reconcile_action(false, true, Some(&authorization)),
            ReconcileAction::Connect(None)
        );
    }

    #[test]
    fn test_approval_without_a_key_removes_the_old_key_from_a_connected_location() {
        let authorization = PostureAuthorization::Approved(None);
        assert_eq!(
            reconcile_action(true, true, Some(&authorization)),
            ReconcileAction::Renew(None)
        );
    }

    #[test]
    fn test_posture_rejection_disconnects_a_connected_location() {
        assert_eq!(
            reconcile_action(true, true, Some(&PostureAuthorization::Rejected)),
            ReconcileAction::Disconnect
        );
    }

    #[test]
    fn test_transient_failure_leaves_a_connected_location_unchanged() {
        assert_eq!(
            reconcile_action(true, true, None),
            ReconcileAction::LeaveConnected
        );
    }

    #[test]
    fn test_posture_rejection_keeps_a_disconnected_location_down() {
        assert_eq!(
            reconcile_action(false, true, Some(&PostureAuthorization::Rejected)),
            ReconcileAction::LeaveDisconnected
        );
    }

    #[test]
    fn test_authorization_failure_keeps_a_posture_location_disconnected() {
        assert_eq!(
            reconcile_action(false, true, None),
            ReconcileAction::WaitForAuthorization
        );
    }

    #[test]
    fn test_regular_location_connects_without_authorization() {
        assert_eq!(
            reconcile_action(false, false, None),
            ReconcileAction::Connect(None)
        );
    }
}
