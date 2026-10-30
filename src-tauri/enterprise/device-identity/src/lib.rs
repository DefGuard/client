use thiserror::Error;

#[cfg(target_os = "linux")]
#[path = "backend/linux.rs"]
mod backend;
#[cfg(target_os = "macos")]
#[path = "backend/macos.rs"]
mod backend;
#[cfg(windows)]
#[path = "backend/windows.rs"]
mod backend;

/// Platform mechanism used to protect the device identity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// TPM 2.0 accessed through the platform transport
    Tpm,
    /// Apple App Attest using a Secure Enclave-bound key
    /// MacOS 27+ only
    AppleAppAttest,
}

/// Protocol used by Core to verify hardware-backed identity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationMethod {
    /// Credential activation followed by identity key certification
    TpmKeyAttestation,
    /// Key attestation issued through Apple services
    AppleAppAttest,
}

/// Client-reported capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    /// Backend used on this platform
    pub backend: BackendKind,
    /// Attestation protocols available from this backend
    pub attestation_methods: Vec<AttestationMethod>,
    /// Raw and interpreted platform-specific capabilities
    pub details: PlatformCapabilities,
}

/// Backend-specific capability details without forcing a shared hardware model
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformCapabilities {
    /// Capabilities queried from TPM 2.0
    Tpm(TpmCapabilities),
    /// Capabilities reported by Apple platform APIs
    Apple(AppleCapabilities),
}

/// Runtime availability of Apple attestation services
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleCapabilities {
    /// Runtime App Attest support
    pub app_attest_supported: bool,
}

/// TPM capability snapshot including raw identifiers and checked profiles
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TpmCapabilities {
    /// Reported TPM algorithms and their raw attribute flags
    pub algorithms: Vec<AlgorithmCapability>,
    /// Elliptic curves reported as supported by the TPM
    pub ecc_curves: Vec<EccCurveId>,
    /// Manufacturer, firmware, limits and other fixed TPM properties
    pub fixed_properties: Vec<TpmProperty>,
    /// TPM state properties captured at query time
    pub variable_properties: Vec<TpmProperty>,
    /// Supported commands and their raw attributes
    pub commands: Vec<CommandCapability>,
    /// Hash algorithms and PCR allocation bitmaps
    pub pcr_banks: Vec<PcrBankCapability>,
    /// Profile checks, including profiles not yet checked
    pub profiles: Vec<ProfileSupport>,
}

/// Algorithm and attributes reported by TPM2_GetCapability
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgorithmCapability {
    /// Raw TPM_ALG_ID preserves identifiers unknown to this library
    pub id: AlgorithmId,
    /// Raw TPMA_ALGORITHM bitmask
    pub flags: u32,
}

/// TPM algorithm identifier, including values unknown to this library
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AlgorithmId(
    /// Raw 16-bit identifier defined by the TPM specification
    pub u16,
);

/// TPM elliptic curve identifier, including unknown values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EccCurveId(
    /// Raw 16-bit identifier defined by the TPM specification
    pub u16,
);

/// Raw property reported by TPM2_GetCapability
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpmProperty {
    /// Raw TPM_PT property identifier
    pub id: u32,
    /// Property value as returned by the TPM
    pub value: u32,
}

/// Attributes of a command supported by the TPM
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCapability {
    /// Raw TPMA_CC includes the command index and its attributes
    pub attributes: u32,
}

/// Available platform configuration registers for one hash algorithm
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcrBankCapability {
    /// Hash algorithm identifying this PCR bank
    pub hash_algorithm: AlgorithmId,
    /// TPMS_PCR_SELECTION bitmap, each set bit identifies an available PCR
    pub selection: Vec<u8>,
}

/// Library-defined combination of key and signature parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileId {
    /// ECC NIST P-256 with ECDSA and SHA-256
    EcdsaP256Sha256,
    /// RSA-2048 with RSASSA-PKCS1-v1_5 and SHA-256
    RsaSsa2048Sha256,
}

/// Result of checking a profile, distinct from individual algorithm availability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportStatus {
    /// The profile parameter combination passed the support check
    Supported,
    /// The profile parameter combination was rejected as unsupported
    Unsupported,
    /// No support check has been performed for this profile
    NotChecked,
}

/// Support assessment for one library-defined profile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSupport {
    /// Parameter combination being assessed
    pub profile: ProfileId,
    /// Whether support was confirmed, rejected or not checked
    pub support: SupportStatus,
}

/// Reference to a locally managed identity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentityRef {
    /// Local storage identifier
    pub id: String,
}

/// Required key profile and attestation protocol for a new identity
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRequirements {
    /// Key and signature parameters required for the identity
    pub profile: ProfileId,
    /// Protocol the created identity must support
    pub attestation_method: AttestationMethod,
}

/// Core session challenge and application binding for an attestation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationContext {
    /// Identifier of the attestation session established with Core
    pub session_id: String,
    /// Fresh challenge supplied by Core
    pub nonce: Vec<u8>,
    /// Application data to bind, such as the WireGuard public key
    pub binding: Vec<u8>,
}

/// Public backend data supplied to Core before generating attestation challenges
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationIdentity {
    /// Endorsement and attestation keys for credential activation
    Tpm(TpmAttestationIdentity),
    /// Identifier of the key generated by App Attest
    AppleAppAttest {
        /// App Attest key identifier stored locally by the client
        key_id: String,
    },
}

/// Public TPM keys used by Core to prepare MakeCredential
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpmAttestationIdentity {
    /// Endorsement key encoded as marshalled TPMT_PUBLIC
    pub ek_public: Vec<u8>,
    /// Available endorsement key certificates, each encoded as DER
    pub ek_certificate_chain_der: Vec<Vec<u8>>,
    /// Attestation key encoded as marshalled TPMT_PUBLIC
    pub ak_public: Vec<u8>,
}

/// Credential activation challenge prepared by Core for a specific EK and AK
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialChallenge {
    /// Contents of the TPM2B_ID_OBJECT produced by MakeCredential
    pub credential_blob: Vec<u8>,
    /// Contents of TPM2B_ENCRYPTED_SECRET carrying the encrypted protection seed
    pub encrypted_secret: Vec<u8>,
}

/// Core challenge binding a possession proof to a session and application data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossessionChallenge {
    /// Identifier of the proof session established with Core
    pub session_id: String,
    /// Fresh single-use challenge supplied by Core
    pub nonce: Vec<u8>,
    /// Application data the proof must also cover
    pub binding: Vec<u8>,
}

/// Backend-specific challenge supplied by Core for identity attestation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationChallenge {
    /// Credential activation challenge bound to the TPM endorsement and attestation keys
    Tpm(CredentialChallenge),
    /// App Attest uses the nonce supplied in the attestation context
    AppleAppAttest,
}

/// Complete input for a stateless identity attestation operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationRequest {
    /// Core session, fresh nonce and application binding
    pub context: AttestationContext,
    /// Backend-specific input determining the attestation protocol
    pub challenge: AttestationChallenge,
}

/// Attestation result submitted to Core for verification
#[derive(Clone, PartialEq, Eq)]
pub enum AttestationResponse {
    /// TPM credential activation result and identity key certification
    Tpm {
        /// Recovered Core secret, sensitive until the challenge is verified
        activation_secret: Vec<u8>,
        /// Identity key certification signed by the attestation key
        evidence: TpmEvidence,
    },
    /// Apple-issued evidence for an App Attest key
    AppleAppAttest(AppleEvidence),
}

/// TPM key certification evidence accompanying the credential activation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpmEvidence {
    /// Endorsement key encoded as marshalled TPMT_PUBLIC
    pub ek_public: Vec<u8>,
    /// Available endorsement key certificates, each encoded as DER
    pub ek_certificate_chain_der: Vec<Vec<u8>>,
    /// Attestation key encoded as marshalled TPMT_PUBLIC
    pub ak_public: Vec<u8>,
    /// Certified identity key encoded as marshalled TPMT_PUBLIC
    pub identity_public: Vec<u8>,
    /// Marshalled TPMS_ATTEST produced by TPM2_Certify
    pub attest: Vec<u8>,
    /// Attestation key signature over the marshalled attestation bytes
    pub signature: Signature,
}

/// App Attest evidence to be validated against Apple trust anchors
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleEvidence {
    /// CBOR attestation object returned by App Attest
    pub attestation_object: Vec<u8>,
}

/// Proof of key possession
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PossessionProof {
    /// Challenge signature verified with the registered public key
    Signature(Signature),
    /// App Attest assertion requiring signature and counter validation
    AppleAssertion(AppleAssertion),
}

/// Signature bytes with an explicit algorithm and encoding contract
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// Scheme and hash determining how signature bytes are interpreted
    pub algorithm: SignatureAlgorithm,
    /// Signature encoded according to the selected algorithm
    pub bytes: Vec<u8>,
}

/// Signature scheme and hash with a defined byte encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSASSA-PKCS1-v1_5 with SHA-256, encoded as raw RSA signature bytes
    RsaSsaSha256,
    /// ECDSA with SHA-256, encoded as ASN.1 DER rather than raw r || s
    EcdsaSha256,
}

/// App Attest assertion for a challenge under a previously attested key
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleAssertion {
    /// CBOR assertion containing signature and authenticator data, including the counter
    pub assertion_object: Vec<u8>,
}

/// Platform-independent failure of a device identity operation
#[derive(Error, Debug, PartialEq)]
pub enum DeviceIdentityError {
    /// Required hardware or platform service is not available
    #[error("hardware identity backend is unavailable")]
    Unavailable,
    /// The caller lacks permission to access the backend
    #[error("permission to access hardware identity was denied")]
    PermissionDenied,
    /// The requested operation or parameters are not supported
    #[error("hardware identity operation is not supported")]
    Unsupported,
    /// The referenced identity can no longer be used
    #[error("hardware identity is no longer available")]
    IdentityLost,
    /// The received challenge is malformed or incompatible with the identity
    #[error("hardware identity challenge is invalid")]
    InvalidChallenge,
    /// Backend failure not represented by a more specific variant
    #[error("hardware identity backend failed")]
    BackendFailure,
}

/// Returns client-reported backend capabilities.
pub fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    backend::capabilities()
}

/// Creates an identity meeting the requirements without replacing an existing key.
pub fn create_identity(
    requirements: &IdentityRequirements,
) -> Result<IdentityRef, DeviceIdentityError> {
    backend::create_identity(requirements)
}

/// Returns the identity's public attestation data for enrollment with Core.
pub fn attestation_identity(
    identity: &IdentityRef,
) -> Result<AttestationIdentity, DeviceIdentityError> {
    backend::attestation_identity(identity)
}

/// Processes the backend challenge and generates evidence bound to the supplied Core context.
/// TPM returns the activation secret and certification, while Apple returns App Attest evidence.
pub fn attest_identity(
    identity: &IdentityRef,
    request: &AttestationRequest,
) -> Result<AttestationResponse, DeviceIdentityError> {
    backend::attest_identity(identity, request)
}

/// Produces a proof bound to Core's challenge using the referenced identity.
/// Core must verify it against the previously attested key.
pub fn prove_possession(
    identity: &IdentityRef,
    challenge: &PossessionChallenge,
) -> Result<PossessionProof, DeviceIdentityError> {
    backend::prove_possession(identity, challenge)
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
        assert_eq!(capabilities(), Err(DeviceIdentityError::Unsupported));
    }

    fn identity() -> IdentityRef {
        IdentityRef {
            id: "test-identity".into(),
        }
    }

    fn context() -> AttestationContext {
        AttestationContext {
            session_id: "test-session".into(),
            nonce: vec![1; 32],
            binding: vec![2; 32],
        }
    }

    #[test]
    fn identity_operations_return_unsupported() {
        let requirements = IdentityRequirements {
            profile: ProfileId::EcdsaP256Sha256,
            attestation_method: AttestationMethod::TpmKeyAttestation,
        };

        assert_eq!(
            create_identity(&requirements),
            Err(DeviceIdentityError::Unsupported)
        );
        assert_eq!(
            public_identity(&identity()),
            Err(DeviceIdentityError::Unsupported)
        );
    }

    #[test]
    fn attestation_identity_returns_unsupported() {
        assert_eq!(
            attestation_identity(&identity()),
            Err(DeviceIdentityError::Unsupported)
        );
    }

    #[test]
    fn attestation_returns_unsupported_for_each_challenge() {
        for challenge in [
            AttestationChallenge::Tpm(CredentialChallenge {
                credential_blob: vec![3; 32],
                encrypted_secret: vec![4; 32],
            }),
            AttestationChallenge::AppleAppAttest,
        ] {
            let request = AttestationRequest {
                context: context(),
                challenge,
            };

            assert!(matches!(
                attest_identity(&identity(), &request),
                Err(DeviceIdentityError::Unsupported)
            ));
        }
    }

    #[test]
    fn possession_proof_returns_unsupported() {
        let challenge = PossessionChallenge {
            session_id: "test-session".into(),
            nonce: vec![1; 32],
            binding: vec![2; 32],
        };

        assert_eq!(
            prove_possession(&identity(), &challenge),
            Err(DeviceIdentityError::Unsupported)
        );
    }
}
