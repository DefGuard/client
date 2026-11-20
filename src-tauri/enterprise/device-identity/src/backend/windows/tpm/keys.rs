use tpm2_protocol::{
    basic::{TpmHandle, TpmUint16},
    data::{
        Tpm2bData, Tpm2bDigest, Tpm2bPrivate, Tpm2bPublic, Tpm2bSensitiveCreate, TpmAlgId,
        TpmEccCurve, TpmRh, TpmaObject, TpmlPcrSelection, TpmsEccParms, TpmsEccPoint,
        TpmsSchemeHash, TpmsSensitiveCreate, TpmtEccScheme, TpmtKdfScheme, TpmtPublic, TpmtSymDef,
        TpmtSymDefObject, TpmuAsymScheme, TpmuPublicId, TpmuPublicParms, TpmuSymKeyBits,
        TpmuSymMode,
    },
    frame::{TpmCreateCommand, TpmCreateResponse, TpmLoadCommand, TpmLoadResponse},
    TpmUnmarshal,
};

use super::{
    map_identity_rc, marshal, password_auth, with_primary, with_transient_response, Transport,
};
use crate::{DeviceIdentityError, TpmKeyBlobs};

fn aes_128_cfb() -> TpmtSymDefObject {
    TpmtSymDef {
        algorithm: TpmAlgId::Aes,
        key_bits: TpmuSymKeyBits::Aes(TpmUint16::from(128)),
        mode: TpmuSymMode::Aes(TpmAlgId::Cfb),
    }
}

fn null_symmetric() -> TpmtSymDef {
    TpmtSymDef {
        algorithm: TpmAlgId::Null,
        key_bits: TpmuSymKeyBits::Null,
        mode: TpmuSymMode::Null,
    }
}

fn storage_template() -> TpmtPublic {
    TpmtPublic {
        object_type: TpmAlgId::Ecc,
        name_alg: TpmAlgId::Sha256,
        object_attributes: TpmaObject::FIXED_TPM
            | TpmaObject::FIXED_PARENT
            | TpmaObject::SENSITIVE_DATA_ORIGIN
            | TpmaObject::USER_WITH_AUTH
            | TpmaObject::NO_DA
            | TpmaObject::RESTRICTED
            | TpmaObject::DECRYPT,
        auth_policy: Tpm2bDigest::new(),
        parameters: TpmuPublicParms::Ecc(TpmsEccParms {
            symmetric: aes_128_cfb(),
            scheme: TpmtEccScheme::default(),
            curve_id: TpmEccCurve::NistP256,
            kdf: TpmtKdfScheme::default(),
        }),
        unique: TpmuPublicId::Ecc(TpmsEccPoint::default()),
    }
}

fn signing_template(restricted: bool) -> TpmtPublic {
    let mut attributes = TpmaObject::FIXED_TPM
        | TpmaObject::FIXED_PARENT
        | TpmaObject::SENSITIVE_DATA_ORIGIN
        | TpmaObject::USER_WITH_AUTH
        | TpmaObject::NO_DA
        | TpmaObject::SIGN_ENCRYPT;
    if restricted {
        attributes |= TpmaObject::RESTRICTED;
    }
    TpmtPublic {
        object_type: TpmAlgId::Ecc,
        name_alg: TpmAlgId::Sha256,
        object_attributes: attributes,
        auth_policy: Tpm2bDigest::new(),
        parameters: TpmuPublicParms::Ecc(TpmsEccParms {
            symmetric: null_symmetric(),
            scheme: TpmtEccScheme {
                scheme: TpmAlgId::Ecdsa,
                details: TpmuAsymScheme::Hash(TpmsSchemeHash {
                    hash_alg: TpmAlgId::Sha256,
                }),
            },
            curve_id: TpmEccCurve::NistP256,
            kdf: TpmtKdfScheme::default(),
        }),
        unique: TpmuPublicId::Ecc(TpmsEccPoint::default()),
    }
}

pub(super) struct CreatedKeys {
    pub(super) srk_template: Vec<u8>,
    pub(super) ak: TpmKeyBlobs,
    pub(super) identity_key: TpmKeyBlobs,
}

pub(super) fn create(transport: &mut impl Transport) -> Result<CreatedKeys, DeviceIdentityError> {
    let template = storage_template();
    let srk_template = marshal(&template)?;
    let (ak, identity_key) = with_primary(
        transport,
        TpmRh::Owner,
        template,
        super::map_tpm_rc,
        |transport, parent| {
            let ak = create_signing_key(transport, parent, true)?;
            let identity_key = create_signing_key(transport, parent, false)?;
            Ok((ak, identity_key))
        },
    )?;
    Ok(CreatedKeys {
        srk_template,
        ak,
        identity_key,
    })
}

fn create_signing_key(
    transport: &mut impl Transport,
    parent: TpmHandle,
    restricted: bool,
) -> Result<TpmKeyBlobs, DeviceIdentityError> {
    let command = TpmCreateCommand {
        handles: [parent],
        in_sensitive: Tpm2bSensitiveCreate {
            inner: TpmsSensitiveCreate::default(),
        },
        in_public: Tpm2bPublic {
            inner: signing_template(restricted),
        },
        outside_info: Tpm2bData::new(),
        creation_pcr: TpmlPcrSelection::default(),
    };
    let response: TpmCreateResponse = super::send_authenticated(transport, &command)?;
    Ok(TpmKeyBlobs {
        public: marshal(&response.out_public.inner)?,
        private: response.out_private.as_ref().to_vec(),
    })
}

#[cfg(test)]
pub(super) fn reload(
    transport: &mut impl Transport,
    identity: &crate::TpmIdentity,
) -> Result<(), DeviceIdentityError> {
    let template = decode_public(&identity.srk_template)?;
    with_primary(
        transport,
        TpmRh::Owner,
        template,
        super::map_tpm_rc,
        |transport, parent| {
            with_loaded_key(transport, parent, &identity.ak, |transport, _| {
                with_loaded_key(transport, parent, &identity.identity_key, |_, _| Ok(()))
            })
        },
    )
}

pub(super) fn with_loaded_key<T: Transport, R>(
    transport: &mut T,
    parent: TpmHandle,
    blobs: &TpmKeyBlobs,
    operation: impl FnOnce(&mut T, TpmHandle) -> Result<R, DeviceIdentityError>,
) -> Result<R, DeviceIdentityError> {
    let public = decode_public(&blobs.public)?;
    let private = Tpm2bPrivate::try_from(blobs.private.as_slice())
        .map_err(|_| DeviceIdentityError::IdentityLost)?;
    let command = TpmLoadCommand {
        handles: [parent],
        in_private: private,
        in_public: Tpm2bPublic { inner: public },
    };
    with_transient_response::<T, _, TpmLoadResponse, _>(
        transport,
        &command,
        &[password_auth()],
        map_identity_rc,
        |transport, handle, _| operation(transport, handle),
    )
}

pub(super) fn decode_public(bytes: &[u8]) -> Result<TpmtPublic, DeviceIdentityError> {
    let (public, remaining) =
        TpmtPublic::unmarshal(bytes).map_err(|_| DeviceIdentityError::IdentityLost)?;
    if !remaining.is_empty() {
        return Err(DeviceIdentityError::IdentityLost);
    }
    Ok(public)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_attributes(template: &TpmtPublic) -> u32 {
        let bytes = marshal(template).expect("marshal public template");
        u32::from_be_bytes(bytes[4..8].try_into().expect("four attribute bytes"))
    }

    #[test]
    fn test_storage_template_has_expected_attributes() {
        assert_eq!(encoded_attributes(&storage_template()), 0x0003_0472);
    }

    #[test]
    fn test_signing_templates_have_expected_attributes() {
        assert_eq!(encoded_attributes(&signing_template(true)), 0x0005_0472);
        assert_eq!(encoded_attributes(&signing_template(false)), 0x0004_0472);
    }
}
