use picky_asn1_x509::{Certificate, SubjectPublicKeyInfo};
use tss_esapi::{
    abstraction::{ek, AsymmetricAlgorithmSelection},
    constants::response_code::Tss2ResponseCodeKind,
    handles::KeyHandle,
    interface_types::{ecc::EccCurve, key_bits::RsaKeyBits},
    structures::Public,
    traits::Marshall,
    Context, Error,
};

use super::tpm_error;
use crate::{DeviceIdentityError, TpmEndorsementKey};

const ECC_PROFILE: AsymmetricAlgorithmSelection =
    AsymmetricAlgorithmSelection::Ecc(EccCurve::NistP256);
const RSA_PROFILE: AsymmetricAlgorithmSelection =
    AsymmetricAlgorithmSelection::Rsa(RsaKeyBits::Rsa2048);

/// Selects ECC, then RSA by certificate presence, defaulting to ECC if both are absent.
/// Only missing NV indices are ignored, other read errors propagate.
fn select_profile(
    context: &mut Context,
) -> Result<(AsymmetricAlgorithmSelection, Option<Vec<u8>>), Error> {
    for profile in [ECC_PROFILE, RSA_PROFILE] {
        match ek::retrieve_ek_pubcert(context, profile) {
            Ok(certificate) => return Ok((profile, Some(certificate))),
            Err(Error::Tss2Error(code)) if code.kind() == Some(Tss2ResponseCodeKind::Handle) => {}
            Err(error) => return Err(error),
        }
    }
    Ok((ECC_PROFILE, None))
}

/// Checks public-key equality only, not the certificate signature, validity or trust.
/// Malformed DER returns a backend error rather than a key mismatch.
pub(super) fn certificate_matches(
    public: &Public,
    certificate: &[u8],
) -> Result<bool, DeviceIdentityError> {
    let certificate: Certificate =
        picky_asn1_der::from_bytes(certificate).map_err(|_| DeviceIdentityError::BackendFailure)?;
    let public: SubjectPublicKeyInfo = public.clone().try_into().map_err(tpm_error)?;
    Ok(certificate.tbs_certificate.subject_public_key_info == public)
}

/// Recreates and flushes a transient EK, assuming empty Endorsement authorization.
/// Returns its input template and public data, rejecting a mismatched NV certificate.
pub(super) fn read(context: &mut Context) -> Result<TpmEndorsementKey, DeviceIdentityError> {
    let (profile, certificate) = select_profile(context).map_err(tpm_error)?;
    let template = ek::create_ek_public_from_default_template_2(profile, None)
        .map_err(tpm_error)?
        .marshall()
        .map_err(tpm_error)?;
    let handle = ek::create_ek_object_2(context, profile, None).map_err(tpm_error)?;
    let public = context
        .execute_with_temporary_object(handle.into(), |ctx, object| {
            ctx.read_public(KeyHandle::from(object))
                .map(|(public, _, _)| public)
        })
        .map_err(tpm_error)?;
    let certificate_chain_der = match certificate {
        Some(certificate) => {
            if !certificate_matches(&public, &certificate)? {
                return Err(DeviceIdentityError::Unsupported);
            }
            vec![certificate]
        }
        None => Vec::new(),
    };
    Ok(TpmEndorsementKey {
        template,
        public: public.marshall().map_err(tpm_error)?,
        certificate_chain_der,
    })
}
