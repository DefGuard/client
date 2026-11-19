use crate::{
    AttestationChallenge, AttestationIdentity, AttestationRequest, AttestationResponse,
    Capabilities, DeviceIdentityError, Identity, PossessionChallenge, PossessionProof,
    TpmAttestationIdentity,
};

mod tpm;

pub(super) async fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    tokio::task::spawn_blocking(tpm::read_capabilities)
        .await
        .map_err(|_| DeviceIdentityError::BackendFailure)?
}

pub(super) async fn create_identity() -> Result<Identity, DeviceIdentityError> {
    tokio::task::spawn_blocking(tpm::create_identity)
        .await
        .map_err(|_| DeviceIdentityError::BackendFailure)?
        .map(Identity::Tpm)
}

pub(super) async fn attestation_identity(
    identity: &Identity,
) -> Result<AttestationIdentity, DeviceIdentityError> {
    let Identity::Tpm(identity) = identity else {
        return Err(DeviceIdentityError::Unsupported);
    };
    Ok(AttestationIdentity::Tpm(TpmAttestationIdentity {
        ek_public: identity.ek.public.clone(),
        ek_certificate_chain_der: identity.ek.certificate_chain_der.clone(),
        ak_public: identity.ak.public.clone(),
    }))
}

pub(super) async fn attest_identity(
    identity: &Identity,
    request: &AttestationRequest,
) -> Result<AttestationResponse, DeviceIdentityError> {
    let Identity::Tpm(identity) = identity else {
        return Err(DeviceIdentityError::Unsupported);
    };
    let AttestationChallenge::Tpm(challenge) = &request.challenge else {
        return Err(DeviceIdentityError::InvalidChallenge);
    };
    let identity = identity.clone();
    let data = request.data.clone();
    let challenge = challenge.clone();
    tokio::task::spawn_blocking(move || tpm::attest_identity(identity, data, challenge))
        .await
        .map_err(|_| DeviceIdentityError::BackendFailure)?
}

pub(super) async fn prove_possession(
    identity: &Identity,
    challenge: &PossessionChallenge,
) -> Result<PossessionProof, DeviceIdentityError> {
    let Identity::Tpm(identity) = identity else {
        return Err(DeviceIdentityError::Unsupported);
    };
    let srk_template = identity.srk_template.clone();
    let identity_key = identity.identity_key.clone();
    let data = challenge.data.clone();
    tokio::task::spawn_blocking(move || tpm::prove_possession(srk_template, identity_key, data))
        .await
        .map_err(|_| DeviceIdentityError::BackendFailure)?
}
