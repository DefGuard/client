use std::env;

use tss_esapi::{
    attributes::ObjectAttributesBuilder,
    constants::response_code::Tss2ResponseCodeKind,
    handles::KeyHandle,
    interface_types::{
        algorithm::{HashingAlgorithm, PublicAlgorithm},
        ecc::EccCurve,
        key_bits::RsaKeyBits,
        resource_handles::Hierarchy,
    },
    structures::{
        CreatePrimaryKeyResult, EccScheme, HashScheme, KeyDerivationFunctionScheme, Public,
        PublicBuilder, PublicEccParameters, PublicEccParametersBuilder, PublicParameters,
        PublicRsaParameters, RsaExponent, RsaScheme, SymmetricDefinitionObject,
    },
    traits::Marshall,
    Context, Error, TctiNameConf,
};

use crate::{
    AttestationIdentity, AttestationMethod, AttestationRequest, AttestationResponse, BackendKind,
    Capabilities, DeviceIdentityError, Identity, PlatformCapabilities, PossessionChallenge,
    PossessionProof, ProfileId, ProfileSupport, SupportStatus, TpmAttestationIdentity, TpmIdentity,
    TpmKeyBlobs,
};

mod ek;
mod raw_capabilities;

#[cfg(test)]
mod tests;

pub(super) async fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    tokio::task::spawn_blocking(read_capabilities)
        .await
        .map_err(|_| DeviceIdentityError::BackendFailure)?
}

/// Reads TSS2_TCTI, defaulting to device:/dev/tpmrm0 only when the variable is absent.
/// Invalid configuration is returned as an error rather than replaced.
fn tcti_configuration() -> Result<TctiNameConf, DeviceIdentityError> {
    let configuration = match env::var("TSS2_TCTI") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => "device:/dev/tpmrm0".into(),
        Err(env::VarError::NotUnicode(_)) => return Err(DeviceIdentityError::BackendFailure),
    };
    configuration
        .parse()
        .map_err(|_| DeviceIdentityError::BackendFailure)
}

fn read_capabilities() -> Result<Capabilities, DeviceIdentityError> {
    let configuration = tcti_configuration()?;
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

/// Builds the ECC P-256 storage template used to recreate the same parent later.
/// The caller retains this input template, not the generated SRK public area.
fn srk_template() -> Result<Public, Error> {
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
    PublicBuilder::new()
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
        .build()
}

/// Assumes empty Owner authorization and leaves the SRK's authorization empty.
/// The caller must flush the transient key handle after use.
fn create_srk(context: &mut Context, template: Public) -> Result<CreatePrimaryKeyResult, Error> {
    context.execute_with_nullauth_session(|ctx| {
        ctx.create_primary(Hierarchy::Owner, template, None, None, None, None)
    })
}

/// Creates an ECDSA P-256/SHA-256 key with empty authorization, without loading it.
/// Restricted keys serve as AKs, unrestricted keys sign possession challenges.
fn create_signing_key(
    context: &mut Context,
    parent: KeyHandle,
    restricted: bool,
) -> Result<TpmKeyBlobs, Error> {
    let attributes = ObjectAttributesBuilder::new()
        .with_fixed_tpm(true)
        .with_fixed_parent(true)
        .with_sensitive_data_origin(true)
        .with_user_with_auth(true)
        .with_no_da(true)
        .with_sign_encrypt(true)
        .with_decrypt(false)
        .with_restricted(restricted)
        .build()?;
    let public = PublicBuilder::new()
        .with_public_algorithm(PublicAlgorithm::Ecc)
        .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
        .with_object_attributes(attributes)
        .with_ecc_parameters(
            PublicEccParametersBuilder::new_unrestricted_signing_key(
                EccScheme::EcDsa(HashScheme::new(HashingAlgorithm::Sha256)),
                EccCurve::NistP256,
            )
            .with_restricted(restricted)
            .build()?,
        )
        .with_ecc_unique_identifier(Default::default())
        .build()?;
    let created = context
        .execute_with_nullauth_session(|ctx| ctx.create(parent, public, None, None, None, None))?;
    Ok(TpmKeyBlobs {
        public: created.out_public.marshall()?,
        private: created.out_private.value().to_vec(),
    })
}

/// Collects EK data and creates AK and identity blobs without persistent handles.
/// Releases the transient SRK after key creation, including error paths.
fn create_tpm_identity(context: &mut Context) -> Result<TpmIdentity, DeviceIdentityError> {
    let ek = ek::read(context)?;
    let template = srk_template().map_err(tpm_error)?;
    let srk_template = template.marshall().map_err(tpm_error)?;
    let srk = create_srk(context, template).map_err(tpm_error)?;
    context
        .execute_with_temporary_object(srk.key_handle.into(), |ctx, parent| {
            let parent = KeyHandle::from(parent);
            let ak = create_signing_key(ctx, parent, true)?;
            let identity_key = create_signing_key(ctx, parent, false)?;
            Ok(TpmIdentity {
                ek,
                srk_template,
                ak,
                identity_key,
            })
        })
        .map_err(tpm_error)
}

/// Maps recognized authorization and algorithm errors to facade errors.
/// Other errors become BackendFailure.
fn tpm_error(error: Error) -> DeviceIdentityError {
    match error {
        Error::Tss2Error(code) => match code.kind() {
            Some(Tss2ResponseCodeKind::AuthFail | Tss2ResponseCodeKind::BadAuth) => {
                DeviceIdentityError::PermissionDenied
            }
            Some(
                Tss2ResponseCodeKind::Asymmetric
                | Tss2ResponseCodeKind::Curve
                | Tss2ResponseCodeKind::KeySize
                | Tss2ResponseCodeKind::Scheme
                | Tss2ResponseCodeKind::Hash
                | Tss2ResponseCodeKind::Symmetric
                | Tss2ResponseCodeKind::Kdf
                | Tss2ResponseCodeKind::Mode,
            ) => DeviceIdentityError::Unsupported,
            _ => DeviceIdentityError::BackendFailure,
        },
        _ => DeviceIdentityError::BackendFailure,
    }
}

/// Runs TPM identity creation on a blocking worker, leaving persistence to the caller.
pub(super) async fn create_identity() -> Result<Identity, DeviceIdentityError> {
    tokio::task::spawn_blocking(|| {
        let configuration = tcti_configuration()?;
        let mut context =
            Context::new(configuration).map_err(|_| DeviceIdentityError::Unavailable)?;
        create_tpm_identity(&mut context).map(Identity::Tpm)
    })
    .await
    .map_err(|_| DeviceIdentityError::BackendFailure)?
}

/// Copies the supplied EK and AK public data without accessing the TPM.
/// Rejects identities from other backends.
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
