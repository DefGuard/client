use crate::{
    AttestationIdentity, AttestationRequest, AttestationResponse, Capabilities,
    DeviceIdentityError, IdentityRef, PossessionChallenge, PossessionProof,
};

pub(crate) fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) async fn create_identity() -> Result<IdentityRef, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn attestation_identity(
    _identity: &IdentityRef,
) -> Result<AttestationIdentity, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) async fn attest_identity(
    _identity: &IdentityRef,
    _request: &AttestationRequest,
) -> Result<AttestationResponse, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) async fn prove_possession(
    _identity: &IdentityRef,
    _challenge: &PossessionChallenge,
) -> Result<PossessionProof, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}
