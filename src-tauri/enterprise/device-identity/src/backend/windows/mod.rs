use crate::{
    AttestationIdentity, AttestationRequest, AttestationResponse, Capabilities,
    DeviceIdentityError, Identity, PossessionChallenge, PossessionProof, TpmAttestationIdentity,
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
    _identity: &Identity,
    _request: &AttestationRequest,
) -> Result<AttestationResponse, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(super) async fn prove_possession(
    _identity: &Identity,
    _challenge: &PossessionChallenge,
) -> Result<PossessionProof, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}
