use std::env;

use tss_esapi::{
    attributes::ObjectAttributesBuilder,
    constants::response_code::Tss2ResponseCodeKind,
    interface_types::{
        algorithm::{HashingAlgorithm, PublicAlgorithm},
        ecc::EccCurve,
        key_bits::RsaKeyBits,
        resource_handles::Hierarchy,
    },
    structures::{
        CreatePrimaryKeyResult, EccScheme, HashScheme, KeyDerivationFunctionScheme, PublicBuilder,
        PublicEccParameters, PublicEccParametersBuilder, PublicParameters, PublicRsaParameters,
        RsaExponent, RsaScheme, SymmetricDefinitionObject,
    },
    Context, Error, TctiNameConf,
};

use crate::{
    AttestationIdentity, AttestationMethod, AttestationRequest, AttestationResponse, BackendKind,
    Capabilities, DeviceIdentityError, Identity, PlatformCapabilities, PossessionChallenge,
    PossessionProof, ProfileId, ProfileSupport, SupportStatus,
};

mod raw_capabilities;

#[cfg(test)]
mod tests;

pub(super) async fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    tokio::task::spawn_blocking(read_capabilities)
        .await
        .map_err(|_| DeviceIdentityError::BackendFailure)?
}

fn read_capabilities() -> Result<Capabilities, DeviceIdentityError> {
    let configuration = match env::var("TSS2_TCTI") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => "device:/dev/tpmrm0".into(),
        Err(env::VarError::NotUnicode(_)) => return Err(DeviceIdentityError::BackendFailure),
    };
    let configuration: TctiNameConf = configuration
        .parse()
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    // the wrapper's ESYS pointer is private, so raw inventory uses its own short-lived context
    let mut details = raw_capabilities::read(&configuration)?;
    let mut context = Context::new(configuration).map_err(|_| DeviceIdentityError::Unavailable)?;
    details.profiles = [ProfileId::EcdsaP256Sha256, ProfileId::RsaSsa2048Sha256]
        .into_iter()
        .map(|profile| check_profile(&mut context, profile))
        .collect::<Result<_, _>>()?;
    let attestation_method = details
        .profiles
        .iter()
        .any(|profile| {
            profile.profile == ProfileId::EcdsaP256Sha256
                && profile.support == SupportStatus::Supported
        })
        .then_some(AttestationMethod::TpmKeyAttestation);
    Ok(Capabilities {
        backend: BackendKind::Tpm,
        attestation_method,
        details: PlatformCapabilities::Tpm(details),
    })
}

/// Checks the profile's parameter combination with TPM2_TestParms.
/// Reports unsupported parameters separately from TPM communication failures.
fn check_profile(
    context: &mut Context,
    profile: ProfileId,
) -> Result<ProfileSupport, DeviceIdentityError> {
    let hash = HashScheme::new(HashingAlgorithm::Sha256);
    let parameters = match profile {
        ProfileId::EcdsaP256Sha256 => PublicParameters::Ecc(PublicEccParameters::new(
            SymmetricDefinitionObject::Null,
            EccScheme::EcDsa(hash),
            EccCurve::NistP256,
            KeyDerivationFunctionScheme::Null,
        )),
        ProfileId::RsaSsa2048Sha256 => PublicParameters::Rsa(PublicRsaParameters::new(
            SymmetricDefinitionObject::Null,
            RsaScheme::RsaSsa(hash),
            RsaKeyBits::Rsa2048,
            RsaExponent::ZERO_EXPONENT,
        )),
    };
    let support = match context.test_parms(parameters) {
        Ok(()) => SupportStatus::Supported,
        Err(Error::Tss2Error(code))
            if matches!(
                code.kind(),
                Some(
                    Tss2ResponseCodeKind::Asymmetric
                        | Tss2ResponseCodeKind::Attributes
                        | Tss2ResponseCodeKind::Hash
                        | Tss2ResponseCodeKind::Value
                        | Tss2ResponseCodeKind::KeySize
                        | Tss2ResponseCodeKind::Mode
                        | Tss2ResponseCodeKind::Type
                        | Tss2ResponseCodeKind::Kdf
                        | Tss2ResponseCodeKind::Scheme
                        | Tss2ResponseCodeKind::Symmetric
                        | Tss2ResponseCodeKind::Curve
                )
            ) =>
        {
            SupportStatus::Unsupported
        }
        Err(_) => return Err(DeviceIdentityError::BackendFailure),
    };
    Ok(ProfileSupport { profile, support })
}

/// Assumes empty Owner authorization and leaves the SRK's authorization empty.
/// The caller must flush the transient key handle after use.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "create_identity is not implemented yet")
)]
fn create_srk(context: &mut Context) -> Result<CreatePrimaryKeyResult, Error> {
    let attributes = ObjectAttributesBuilder::new()
        .with_fixed_tpm(true)
        .with_fixed_parent(true)
        .with_sensitive_data_origin(true)
        .with_user_with_auth(true)
        .with_no_da(true)
        .with_decrypt(true)
        .with_sign_encrypt(false)
        .with_restricted(true)
        .build()?;
    let public = PublicBuilder::new()
        .with_public_algorithm(PublicAlgorithm::Ecc)
        .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
        .with_object_attributes(attributes)
        .with_ecc_parameters(
            PublicEccParametersBuilder::new_restricted_decryption_key(
                SymmetricDefinitionObject::AES_128_CFB,
                EccCurve::NistP256,
            )
            .build()?,
        )
        .with_ecc_unique_identifier(Default::default())
        .build()?;

    context.execute_with_nullauth_session(|ctx| {
        ctx.create_primary(Hierarchy::Owner, public, None, None, None, None)
    })
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
