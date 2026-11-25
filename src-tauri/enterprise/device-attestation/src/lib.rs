use std::time::Duration;

use defguard_client_proto::defguard::{
    client::v1::{
        desktop_daemon_service_client::DesktopDaemonServiceClient, AttestDeviceIdentityRequest,
    },
    client_types::{AttestationFinishResponse, AttestationStartResponse},
    device_identity::v2::{
        attestation_challenge_solution, attestation_identity, AttestationChallenge,
        AttestationChallengeSolution, AttestationIdentity, AttestationStatus, IdentityErrorCode,
    },
};
use serde::Serialize;
use serde_json::json;
use sqlx::SqlitePool;
use tonic::{transport::Channel, Code, Status};

use defguard_client_core::{
    database::models::{
        device_identity::DeviceIdentity, instance::Instance, wireguard_keys::WireguardKeys, Id,
    },
    enrollment::{enrollment_post, CreatedDevice, EnrollmentError, EnrollmentSession},
};
use log::{debug, warn};

const DAEMON_OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttestationOutcome {
    Attested,
    Skipped,
    Failed,
}

fn registration_matches(
    registration: &CreatedDevice,
    instance: &Instance<Id>,
    keys: &WireguardKeys<Id>,
) -> bool {
    registration.core_device_id > 0
        && registration.instance_uuid == instance.uuid
        && registration.username == instance.username
        && keys.instance_id == instance.id
        && registration.wireguard_pubkey == keys.pubkey
}

#[derive(Debug)]
enum Failure {
    Registration,
    Database,
    DaemonResponse,
    Start,
    Finish,
    Unverified,
}

pub async fn attest_device(
    session: &EnrollmentSession,
    pool: &SqlitePool,
    instance_id: Id,
    daemon: DesktopDaemonServiceClient<Channel>,
) -> AttestationOutcome {
    if !cfg!(target_os = "linux") {
        return AttestationOutcome::Skipped;
    }
    match attest(session, pool, instance_id, daemon).await {
        Ok(outcome) => outcome,
        Err(stage) => {
            // proxy and daemon error messages can contain the submitted evidence
            warn!("Device attestation did not complete at stage {stage:?}");
            AttestationOutcome::Failed
        }
    }
}

async fn attest(
    session: &EnrollmentSession,
    pool: &SqlitePool,
    instance_id: Id,
    mut daemon: DesktopDaemonServiceClient<Channel>,
) -> Result<AttestationOutcome, Failure> {
    let registration = session
        .created_device()
        .map_err(|_| Failure::Registration)?
        .ok_or(Failure::Registration)?;
    let instance = Instance::find_by_id(pool, instance_id)
        .await
        .map_err(|_| Failure::Database)?
        .ok_or(Failure::Registration)?;
    let keys = WireguardKeys::find_by_instance_id(pool, instance_id)
        .await
        .map_err(|_| Failure::Database)?
        .ok_or(Failure::Registration)?;
    if !registration_matches(&registration, &instance, &keys) {
        return Err(Failure::Registration);
    }
    if let Some(existing) = DeviceIdentity::find_by_instance_id(pool, instance_id)
        .await
        .map_err(|_| Failure::Database)?
    {
        if existing.core_device_id != registration.core_device_id {
            return Err(Failure::Registration);
        }
        return Ok(AttestationOutcome::Skipped);
    }

    let created =
        match tokio::time::timeout(DAEMON_OPERATION_TIMEOUT, daemon.create_device_identity(()))
            .await
        {
            Ok(Ok(response)) => response.into_inner(),
            Ok(Err(status)) => {
                debug!("Device identity preparation skipped: {:?}", status.code());
                return Ok(AttestationOutcome::Skipped);
            }
            Err(_) => {
                debug!("Device identity preparation timed out");
                return Ok(AttestationOutcome::Skipped);
            }
        };
    let public = created
        .attestation_identity
        .ok_or(Failure::DaemonResponse)?;
    if !matches!(&public.backend, Some(attestation_identity::Backend::Tpm(tpm))
        if !tpm.ek_public.is_empty() && !tpm.ak_public.is_empty())
    {
        return Err(Failure::DaemonResponse);
    }
    let mut identity = DeviceIdentity::new(
        instance_id,
        registration.core_device_id,
        created.identity.ok_or(Failure::DaemonResponse)?,
    )
    .map_err(|_| Failure::DaemonResponse)?;
    // a lost finish response may hide success, so keep the keys before contacting Core
    identity.save(pool).await.map_err(|_| Failure::Database)?;
    let challenge = start(session, registration.core_device_id, public)
        .await
        .map_err(|_| Failure::Start)?;
    if challenge.session_id.is_empty() {
        return Err(Failure::Start);
    }
    let challenge_id = challenge.session_id.clone();
    let response = tokio::time::timeout(
        DAEMON_OPERATION_TIMEOUT,
        daemon.attest_device_identity(AttestDeviceIdentityRequest {
            identity: Some(identity.material.clone()),
            challenge: Some(challenge),
        }),
    )
    .await
    .unwrap_or_else(|_| Err(Status::deadline_exceeded("Device attestation timed out")));
    let solution = daemon_solution(&challenge_id, response.map(tonic::Response::into_inner));
    let evidence_available = matches!(
        &solution.result,
        Some(attestation_challenge_solution::Result::Tpm(_))
    );
    let response = finish(session, registration.core_device_id, solution)
        .await
        .map_err(|_| Failure::Finish)?;
    let identity_id = verified_identity_id(&response, evidence_available)?;
    identity
        .record_identity_id(pool, identity_id)
        .await
        .map_err(|_| Failure::Database)?;
    Ok(AttestationOutcome::Attested)
}

async fn start(
    session: &EnrollmentSession,
    device_id: Id,
    public: AttestationIdentity,
) -> Result<AttestationChallenge, EnrollmentError> {
    let response = enrollment_post(
        session,
        "api/v1/enrollment/attestation/start",
        json!({"device_id": device_id, "attestation_identity": public}),
    )
    .await?;
    let response: AttestationStartResponse =
        response.json().await.map_err(|_| EnrollmentError::Other {
            message: "Invalid attestation start response".into(),
        })?;
    response.challenge.ok_or_else(|| EnrollmentError::Other {
        message: "Missing attestation challenge".into(),
    })
}

async fn finish(
    session: &EnrollmentSession,
    device_id: Id,
    solution: AttestationChallengeSolution,
) -> Result<AttestationFinishResponse, EnrollmentError> {
    enrollment_post(
        session,
        "api/v1/enrollment/attestation/finish",
        json!({"device_id": device_id, "solution": solution}),
    )
    .await?
    .json()
    .await
    .map_err(|_| EnrollmentError::Other {
        message: "Invalid attestation finish response".into(),
    })
}

fn daemon_solution(
    session_id: &str,
    response: Result<AttestationChallengeSolution, Status>,
) -> AttestationChallengeSolution {
    let code = match response {
        Ok(solution)
            if solution.session_id == session_id
                && matches!(
                    &solution.result,
                    Some(
                        attestation_challenge_solution::Result::Tpm(_)
                            | attestation_challenge_solution::Result::Error(_)
                    )
                ) =>
        {
            return solution;
        }
        Ok(_) => IdentityErrorCode::BackendFailure,
        Err(status) => match status.code() {
            Code::Unavailable => IdentityErrorCode::TemporarilyUnavailable,
            Code::DeadlineExceeded => IdentityErrorCode::Timeout,
            Code::ResourceExhausted => IdentityErrorCode::RateLimited,
            Code::PermissionDenied => IdentityErrorCode::PermissionDenied,
            Code::Unimplemented => IdentityErrorCode::Unsupported,
            Code::FailedPrecondition => IdentityErrorCode::IdentityLost,
            Code::InvalidArgument => IdentityErrorCode::InvalidChallenge,
            _ => IdentityErrorCode::BackendFailure,
        },
    };
    AttestationChallengeSolution {
        session_id: session_id.into(),
        result: Some(attestation_challenge_solution::Result::Error(code.into())),
    }
}

fn verified_identity_id(
    response: &AttestationFinishResponse,
    evidence_available: bool,
) -> Result<&str, Failure> {
    let state = response
        .identity_state
        .as_ref()
        .ok_or(Failure::Unverified)?;
    if !evidence_available
        || state.attestation_status != i32::from(AttestationStatus::Attested)
        || state.error.is_some()
        || state.identity_id.is_empty()
    {
        return Err(Failure::Unverified);
    }
    Ok(&state.identity_id)
}
