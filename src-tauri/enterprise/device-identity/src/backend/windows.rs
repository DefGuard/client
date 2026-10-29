use crate::{
    AttestationContext, AttestationEvidence, AttestationIdentity, AttestationMethod, Capabilities,
    CredentialChallenge, DeviceIdentityError, IdentityRef, IdentityRequirements,
    PossessionChallenge, PossessionProof, PublicIdentity,
};

pub(crate) fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn create_identity(
    _requirements: &IdentityRequirements,
) -> Result<IdentityRef, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn public_identity(
    _identity: &IdentityRef,
) -> Result<PublicIdentity, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn attestation_identity(
    _identity: &IdentityRef,
) -> Result<AttestationIdentity, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn activate_credential(
    _identity: &IdentityRef,
    _challenge: &CredentialChallenge,
) -> Result<Vec<u8>, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn attest_identity(
    _identity: &IdentityRef,
    _method: AttestationMethod,
    _context: &AttestationContext,
) -> Result<AttestationEvidence, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn prove_possession(
    _identity: &IdentityRef,
    _challenge: &PossessionChallenge,
) -> Result<PossessionProof, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}
