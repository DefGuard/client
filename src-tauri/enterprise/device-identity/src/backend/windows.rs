use crate::{
    AttestationChallenge, AttestationContext, AttestationMethod, AttestationProgress,
    AttestationSession, Capabilities, DeviceIdentityError, IdentityRef, IdentityRequirements,
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

pub(crate) fn begin_attestation(
    _identity: &IdentityRef,
    _method: AttestationMethod,
    _context: AttestationContext,
) -> Result<AttestationProgress, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn continue_attestation(
    _session: AttestationSession,
    _challenge: AttestationChallenge,
) -> Result<AttestationProgress, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(crate) fn prove_possession(
    _identity: &IdentityRef,
    _challenge: &PossessionChallenge,
) -> Result<PossessionProof, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}
