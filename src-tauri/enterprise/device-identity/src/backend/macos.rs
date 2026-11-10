use crate::{
    AttestationIdentity, AttestationRequest, AttestationResponse, Capabilities,
    DeviceIdentityError, Identity, PossessionChallenge, PossessionProof,
};

pub(super) async fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(super) async fn create_identity() -> Result<Identity, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(super) async fn attestation_identity(
    _identity: &Identity,
) -> Result<AttestationIdentity, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
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
