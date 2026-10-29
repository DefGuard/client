use thiserror::Error;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod backend;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod backend;
#[cfg(windows)]
#[path = "windows.rs"]
mod backend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Tpm,
    AppleAppAttest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationMethod {
    TpmKeyAttestation,
    AppleAppAttest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    pub backend: BackendKind,
    pub attestation_methods: Vec<AttestationMethod>,
    pub details: PlatformCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformCapabilities {
    Tpm(TpmCapabilities),
    Apple(AppleCapabilities),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleCapabilities {
    pub app_attest_supported: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TpmCapabilities {
    pub algorithms: Vec<AlgorithmCapability>,
    pub ecc_curves: Vec<EccCurveId>,
    pub fixed_properties: Vec<TpmProperty>,
    pub variable_properties: Vec<TpmProperty>,
    pub commands: Vec<CommandCapability>,
    pub pcr_banks: Vec<PcrBankCapability>,
    pub profiles: Vec<ProfileSupport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgorithmCapability {
    /// raw TPM_ALG_ID preserves identifiers unknown to this library
    pub id: AlgorithmId,
    /// raw TPMA_ALGORITHM bitmask
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AlgorithmId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EccCurveId(pub u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpmProperty {
    pub id: u32,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCapability {
    /// raw TPMA_CC includes the command index and its attributes
    pub attributes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcrBankCapability {
    pub hash_algorithm: AlgorithmId,
    /// TPMS_PCR_SELECTION bitmap, each set bit identifies an available PCR
    pub selection: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileId {
    EcdsaP256Sha256,
    RsaSsa2048Sha256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportStatus {
    Supported,
    Unsupported,
    NotChecked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSupport {
    pub profile: ProfileId,
    pub support: SupportStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentityRef {
    /// local storage identifier, not a transient TPM handle
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicIdentity {
    pub public_key_der: Vec<u8>,
    pub backend: BackendKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationContext {
    pub session_id: String,
    pub nonce: Vec<u8>,
    pub binding: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationEvidence {
    Tpm(TpmEvidence),
    AppleAppAttest(AppleEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpmEvidence {
    /// marshalled TPMT_PUBLIC, not SPKI DER
    pub ek_public: Vec<u8>,
    pub ek_certificate_chain_der: Vec<Vec<u8>>,
    /// marshalled TPMT_PUBLIC, not SPKI DER
    pub ak_public: Vec<u8>,
    /// marshalled TPMT_PUBLIC, not SPKI DER
    pub identity_public: Vec<u8>,
    /// marshalled TPMS_ATTEST
    pub attest: Vec<u8>,
    pub signature: Signature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleEvidence {
    pub attestation_object: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PossessionProof {
    Signature(Signature),
    AppleAssertion(AppleAssertion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub algorithm: SignatureAlgorithm,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    RsaSsaSha256,
    /// signatures use ASN.1 DER encoding, not the raw r || s format
    EcdsaSha256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleAssertion {
    pub assertion_object: Vec<u8>,
}

#[derive(Error, Debug, PartialEq)]
pub enum DeviceIdentityError {
    #[error("hardware identity backend is unavailable")]
    Unavailable,
    #[error("permission to access hardware identity was denied")]
    PermissionDenied,
    #[error("hardware identity operation is not supported")]
    Unsupported,
    #[error("hardware identity is no longer available")]
    IdentityLost,
    #[error("hardware identity challenge is invalid")]
    InvalidChallenge,
    #[error("hardware identity backend failed")]
    BackendFailure,
}

pub fn probe() -> Result<Capabilities, DeviceIdentityError> {
    backend::probe()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_preserve_unknown_algorithm_ids_and_flags() {
        let capabilities = TpmCapabilities {
            algorithms: vec![AlgorithmCapability {
                id: AlgorithmId(0xffff),
                flags: 0x8000_0000,
            }],
            ..Default::default()
        };

        assert_eq!(capabilities.algorithms[0].id, AlgorithmId(0xffff));
        assert_eq!(capabilities.algorithms[0].flags, 0x8000_0000);
        assert!(capabilities.profiles.is_empty());
    }

    #[test]
    fn unimplemented_backend_returns_unsupported() {
        assert_eq!(probe(), Err(DeviceIdentityError::Unsupported));
    }
}
