use std::{mem::size_of, ptr::null_mut, slice};

use picky_asn1::{bit_string::BitString, wrapper::IntegerAsn1};
use picky_asn1_x509::{
    oids, AlgorithmIdentifierParameters, Certificate, EcParameters, PublicKey, SubjectPublicKeyInfo,
};
use tpm2_protocol::{
    basic::{TpmHandle, TpmUint16, TpmUint32},
    data::{
        Tpm2bDigest, Tpm2bEccParameter, Tpm2bPublicKeyRsa, TpmAlgId, TpmEccCurve, TpmPt, TpmRh,
        TpmaObject, TpmsEccParms, TpmsEccPoint, TpmsRsaParms, TpmtEccScheme, TpmtKdfScheme,
        TpmtPublic, TpmtRsaScheme, TpmtSymDefObject, TpmuAsymScheme, TpmuPublicId, TpmuPublicParms,
        TpmuSymKeyBits, TpmuSymMode,
    },
    frame::{
        TpmNvReadCommand, TpmNvReadPublicCommand, TpmNvReadPublicResponse, TpmNvReadResponse,
        TpmReadPublicCommand, TpmReadPublicResponse,
    },
    TpmUnmarshal,
};
use tracing::debug;
use windows_sys::Win32::{
    Foundation::{GetLastError, SetLastError, CRYPT_E_NOT_FOUND, NTE_NOT_FOUND},
    Security::Cryptography::{
        CertCloseStore, CertEnumCertificatesInStore, CertFreeCertificateContext, NCryptFreeObject,
        NCryptGetProperty, NCryptOpenStorageProvider, CERT_CONTEXT, HCERTSTORE,
        MS_PLATFORM_CRYPTO_PROVIDER, NCRYPT_PCP_EKCERT_PROPERTY, NCRYPT_PROV_HANDLE,
    },
};

use crate::{DeviceIdentityError, TpmEndorsementKey};

const ECC_EK_CERTIFICATE_INDEX: u32 = 0x01c0_000a;
const RSA_EK_CERTIFICATE_INDEX: u32 = 0x01c0_0002;
const MAX_NV_READ_SIZE: usize = 1024;
const EK_AUTH_POLICY_SHA256: [u8; 32] = [
    0x83, 0x71, 0x97, 0x67, 0x44, 0x84, 0xb3, 0xf8, 0x1a, 0x90, 0xcc, 0x8d, 0x46, 0xa5, 0xd7, 0x24,
    0xfd, 0x52, 0xd7, 0x6e, 0x06, 0x52, 0x0b, 0x64, 0xf2, 0xa1, 0xda, 0x1b, 0x33, 0x14, 0x69, 0xaa,
];

#[derive(Clone, Copy)]
enum Profile {
    EccP256,
    Rsa2048,
}

impl Profile {
    fn certificate_index(self) -> u32 {
        match self {
            Self::EccP256 => ECC_EK_CERTIFICATE_INDEX,
            Self::Rsa2048 => RSA_EK_CERTIFICATE_INDEX,
        }
    }
}

pub(super) fn read(
    transport: &mut impl super::Transport,
    nv_buffer_max: Option<u32>,
) -> Result<TpmEndorsementKey, DeviceIdentityError> {
    let (profile, certificates) = select_profile(transport, nv_buffer_max)?;
    let template = ek_template(profile)?;
    let template_bytes = super::marshal(&template)?;
    let public_bytes = super::with_primary(
        transport,
        TpmRh::Endorsement,
        template,
        super::map_tpm_rc,
        |transport, handle| {
            let command = TpmReadPublicCommand { handles: [handle] };
            let response: TpmReadPublicResponse = super::send_frame(transport, &command)?;
            super::marshal(&response.out_public.inner)
        },
    )?;
    let certificate_chain_der = if certificates.is_empty() {
        Vec::new()
    } else {
        let mut matching_certificate = None;
        for certificate in certificates {
            if certificate_matches(&public_bytes, &certificate)? {
                matching_certificate = Some(certificate);
                break;
            }
        }
        vec![matching_certificate.ok_or(DeviceIdentityError::Unsupported)?]
    };
    Ok(TpmEndorsementKey {
        template: template_bytes,
        public: public_bytes,
        certificate_chain_der,
    })
}

fn select_profile(
    transport: &mut impl super::Transport,
    nv_buffer_max: Option<u32>,
) -> Result<(Profile, Vec<Vec<u8>>), DeviceIdentityError> {
    if let Some(certificate) = read_nv_certificate(
        transport,
        Profile::EccP256.certificate_index(),
        nv_buffer_max,
    )? {
        return Ok((Profile::EccP256, vec![certificate]));
    }
    let cached_certificates = read_cached_certificates().unwrap_or_else(|error| {
        debug!(?error, "Could not read cached EK certificates");
        Vec::new()
    });
    let ecc_certificates = find_profile_certificates(&cached_certificates, Profile::EccP256);
    if !ecc_certificates.is_empty() {
        return Ok((Profile::EccP256, ecc_certificates));
    }
    if let Some(certificate) = read_nv_certificate(
        transport,
        Profile::Rsa2048.certificate_index(),
        nv_buffer_max,
    )? {
        return Ok((Profile::Rsa2048, vec![certificate]));
    }
    let rsa_certificates = find_profile_certificates(&cached_certificates, Profile::Rsa2048);
    if !rsa_certificates.is_empty() {
        return Ok((Profile::Rsa2048, rsa_certificates));
    }
    Ok((Profile::EccP256, Vec::new()))
}

fn read_nv_certificate(
    transport: &mut impl super::Transport,
    index: u32,
    nv_buffer_max: Option<u32>,
) -> Result<Option<Vec<u8>>, DeviceIdentityError> {
    let index_handle = TpmHandle::from(index);
    let public_command = TpmNvReadPublicCommand {
        handles: [index_handle],
    };
    let Some(public): Option<TpmNvReadPublicResponse> =
        super::send_allow_missing_handle(transport, &public_command)?
    else {
        return Ok(None);
    };
    let nv_public = public.nv_public.inner;
    if nv_public.nv_index != index_handle {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let data_size = usize::from(u16::from(nv_public.data_size));
    if data_size == 0 {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let Some(nv_buffer_max) = nv_buffer_max.filter(|value| *value != 0) else {
        debug!(
            property = format_args!("{:#010x}", TpmPt::NvBufferMax.value()),
            value = ?nv_buffer_max.map(|value| format!("{value:#010x}")),
            "TPM_PT_NV_BUFFER_MAX is missing or zero"
        );
        return Err(DeviceIdentityError::BackendFailure);
    };
    let read_limit = usize::try_from(nv_buffer_max)
        .map_err(|_| DeviceIdentityError::BackendFailure)?
        .min(MAX_NV_READ_SIZE);
    let mut certificate = Vec::with_capacity(data_size);
    let mut offset = 0;
    while offset < data_size {
        let read_size = read_limit.min(data_size - offset);
        let command = TpmNvReadCommand {
            handles: [index_handle, index_handle],
            size: TpmUint16::from(
                u16::try_from(read_size).map_err(|_| DeviceIdentityError::BackendFailure)?,
            ),
            offset: TpmUint16::from(
                u16::try_from(offset).map_err(|_| DeviceIdentityError::BackendFailure)?,
            ),
        };
        let response: TpmNvReadResponse = super::send_authenticated(transport, &command)?;
        let bytes = response.data.as_ref();
        if bytes.len() != read_size {
            return Err(DeviceIdentityError::BackendFailure);
        }
        certificate.extend_from_slice(bytes);
        offset += read_size;
    }
    Ok(Some(certificate))
}

fn find_profile_certificates(certificates: &[Vec<u8>], profile: Profile) -> Vec<Vec<u8>> {
    let mut matching = Vec::new();
    for der in certificates {
        match certificate_has_profile(der, profile) {
            Ok(true) => matching.push(der.clone()),
            Ok(false) => {}
            Err(error) => debug!(?error, "Could not parse cached EK certificate"),
        }
    }
    matching
}

fn certificate_has_profile(der: &[u8], profile: Profile) -> Result<bool, DeviceIdentityError> {
    let certificate: Certificate =
        picky_asn1_der::from_bytes(der).map_err(|_| DeviceIdentityError::BackendFailure)?;
    let info = &certificate.tbs_certificate.subject_public_key_info;
    Ok(match profile {
        Profile::EccP256 => {
            info.algorithm.is_a(oids::ec_public_key())
                && matches!(
                    (info.algorithm.parameters(), &info.subject_public_key),
                    (
                        AlgorithmIdentifierParameters::Ec(EcParameters::NamedCurve(curve)),
                        PublicKey::Ec(_)
                    ) if curve.0 == oids::secp256r1()
                )
        }
        Profile::Rsa2048 => {
            info.algorithm.is_a(oids::rsa_encryption())
                && matches!(
                    &info.subject_public_key,
                    PublicKey::Rsa(key)
                        if key.0.modulus.as_unsigned_bytes_be().len() == 256
                )
        }
    })
}

fn read_cached_certificates() -> Result<Vec<Vec<u8>>, DeviceIdentityError> {
    unsafe {
        let mut provider = NCRYPT_PROV_HANDLE::default();
        let status = NCryptOpenStorageProvider(&mut provider, MS_PLATFORM_CRYPTO_PROVIDER, 0);
        if status == NTE_NOT_FOUND {
            return Ok(Vec::new());
        }
        if status != 0 {
            return Err(DeviceIdentityError::BackendFailure);
        }
        let provider = ProviderHandle(provider);
        let mut store: HCERTSTORE = null_mut();
        let mut returned = 0;
        let status = NCryptGetProperty(
            provider.0,
            NCRYPT_PCP_EKCERT_PROPERTY,
            (&mut store as *mut HCERTSTORE).cast(),
            u32::try_from(size_of::<HCERTSTORE>())
                .map_err(|_| DeviceIdentityError::BackendFailure)?,
            &mut returned,
            0,
        );
        if status == NTE_NOT_FOUND {
            return Ok(Vec::new());
        }
        if status != 0 || returned as usize != size_of::<HCERTSTORE>() || store.is_null() {
            return Err(DeviceIdentityError::BackendFailure);
        }
        let store = CertificateStore(store);
        let mut certificates = Vec::new();
        let mut previous = null_mut();
        loop {
            SetLastError(0);
            let current = CertEnumCertificatesInStore(store.0, previous);
            if current.is_null() {
                if GetLastError() == CRYPT_E_NOT_FOUND as u32 {
                    break;
                }
                return Err(DeviceIdentityError::BackendFailure);
            }
            previous = current;
            let context: &CERT_CONTEXT = &*current;
            let length = context.cbCertEncoded as usize;
            if length == 0 || context.pbCertEncoded.is_null() {
                CertFreeCertificateContext(current);
                return Err(DeviceIdentityError::BackendFailure);
            }
            certificates.push(slice::from_raw_parts(context.pbCertEncoded, length).to_vec());
        }
        Ok(certificates)
    }
}

struct ProviderHandle(NCRYPT_PROV_HANDLE);

impl Drop for ProviderHandle {
    fn drop(&mut self) {
        unsafe {
            NCryptFreeObject(self.0);
        }
    }
}

struct CertificateStore(HCERTSTORE);

impl Drop for CertificateStore {
    fn drop(&mut self) {
        unsafe {
            CertCloseStore(self.0, 0);
        }
    }
}

fn ek_template(profile: Profile) -> Result<TpmtPublic, DeviceIdentityError> {
    let auth_policy = Tpm2bDigest::try_from(EK_AUTH_POLICY_SHA256.as_slice())
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    let object_attributes = TpmaObject::FIXED_TPM
        | TpmaObject::FIXED_PARENT
        | TpmaObject::SENSITIVE_DATA_ORIGIN
        | TpmaObject::ADMIN_WITH_POLICY
        | TpmaObject::RESTRICTED
        | TpmaObject::DECRYPT;
    let symmetric = TpmtSymDefObject {
        algorithm: TpmAlgId::Aes,
        key_bits: TpmuSymKeyBits::Aes(TpmUint16::from(128)),
        mode: TpmuSymMode::Aes(TpmAlgId::Cfb),
    };
    Ok(match profile {
        Profile::EccP256 => {
            let zero_coordinate = Tpm2bEccParameter::try_from([0; 32].as_slice())
                .map_err(|_| DeviceIdentityError::BackendFailure)?;
            TpmtPublic {
                object_type: TpmAlgId::Ecc,
                name_alg: TpmAlgId::Sha256,
                object_attributes,
                auth_policy,
                parameters: TpmuPublicParms::Ecc(TpmsEccParms {
                    symmetric,
                    scheme: TpmtEccScheme::default(),
                    curve_id: TpmEccCurve::NistP256,
                    kdf: TpmtKdfScheme::default(),
                }),
                unique: TpmuPublicId::Ecc(TpmsEccPoint {
                    x: zero_coordinate,
                    y: zero_coordinate,
                }),
            }
        }
        Profile::Rsa2048 => {
            let zero_modulus = Tpm2bPublicKeyRsa::try_from([0; 256].as_slice())
                .map_err(|_| DeviceIdentityError::BackendFailure)?;
            TpmtPublic {
                object_type: TpmAlgId::Rsa,
                name_alg: TpmAlgId::Sha256,
                object_attributes,
                auth_policy,
                parameters: TpmuPublicParms::Rsa(TpmsRsaParms {
                    symmetric,
                    scheme: TpmtRsaScheme {
                        scheme: TpmAlgId::Null,
                        details: TpmuAsymScheme::Null,
                    },
                    key_bits: TpmUint16::from(2048),
                    exponent: TpmUint32::from(0),
                }),
                unique: TpmuPublicId::Rsa(zero_modulus),
            }
        }
    })
}

fn certificate_matches(
    public_bytes: &[u8],
    certificate_der: &[u8],
) -> Result<bool, DeviceIdentityError> {
    let certificate: Certificate = picky_asn1_der::from_bytes(certificate_der)
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    let (public, remaining) =
        TpmtPublic::unmarshal(public_bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
    if !remaining.is_empty() {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let public = subject_public_key_info(&public)?;
    Ok(certificate.tbs_certificate.subject_public_key_info == public)
}

fn subject_public_key_info(
    public: &TpmtPublic,
) -> Result<SubjectPublicKeyInfo, DeviceIdentityError> {
    match (&public.object_type, &public.parameters, &public.unique) {
        (TpmAlgId::Ecc, TpmuPublicParms::Ecc(parameters), TpmuPublicId::Ecc(point))
            if parameters.curve_id == TpmEccCurve::NistP256
                && point.x.as_ref().len() == 32
                && point.y.as_ref().len() == 32 =>
        {
            let mut point_bytes = Vec::with_capacity(65);
            point_bytes.push(0x04);
            point_bytes.extend_from_slice(point.x.as_ref());
            point_bytes.extend_from_slice(point.y.as_ref());
            Ok(SubjectPublicKeyInfo::new_ec_key(
                oids::secp256r1(),
                BitString::with_bytes(point_bytes),
            ))
        }
        (TpmAlgId::Rsa, TpmuPublicParms::Rsa(parameters), TpmuPublicId::Rsa(modulus))
            if u16::from(parameters.key_bits) == 2048 && modulus.as_ref().len() == 256 =>
        {
            let exponent = u32::from(parameters.exponent);
            let exponent = if exponent == 0 { 65_537 } else { exponent };
            Ok(SubjectPublicKeyInfo::new_rsa_key(
                IntegerAsn1::from_bytes_be_unsigned(modulus.as_ref().to_vec()),
                IntegerAsn1::from_bytes_be_unsigned(exponent.to_be_bytes().to_vec()),
            ))
        }
        _ => Err(DeviceIdentityError::BackendFailure),
    }
}

#[cfg(test)]
mod nv_read_tests {
    use super::*;
    use tpm2_protocol::data::TpmCc;
    const INDEX: u32 = ECC_EK_CERTIFICATE_INDEX;
    struct FakeTransport {
        certificate: Vec<u8>,
        commands: Vec<u32>,
        reads: Vec<(u16, u16)>,
    }
    impl super::super::Transport for FakeTransport {
        fn submit(&mut self, command: &[u8]) -> Result<Vec<u8>, DeviceIdentityError> {
            let command_code = u32::from_be_bytes(
                command
                    .get(6..10)
                    .ok_or(DeviceIdentityError::BackendFailure)?
                    .try_into()
                    .map_err(|_| DeviceIdentityError::BackendFailure)?,
            );
            self.commands.push(command_code);
            if command_code == TpmCc::NvReadPublic.value() {
                let index = u32::from_be_bytes(
                    command
                        .get(10..14)
                        .ok_or(DeviceIdentityError::BackendFailure)?
                        .try_into()
                        .map_err(|_| DeviceIdentityError::BackendFailure)?,
                );
                if index != INDEX {
                    return Err(DeviceIdentityError::BackendFailure);
                }
                return Ok(nv_read_public_response(
                    index,
                    self.certificate.len() as u16,
                ));
            }
            if command_code != TpmCc::NvRead.value() {
                panic!("unexpected TPM command: {command_code:#010x}");
            }
            let parameters = command
                .get(command.len().saturating_sub(4)..)
                .ok_or(DeviceIdentityError::BackendFailure)?;
            let size = u16::from_be_bytes(
                parameters[..2]
                    .try_into()
                    .map_err(|_| DeviceIdentityError::BackendFailure)?,
            );
            let offset = u16::from_be_bytes(
                parameters[2..]
                    .try_into()
                    .map_err(|_| DeviceIdentityError::BackendFailure)?,
            );
            self.reads.push((size, offset));
            let start = usize::from(offset);
            let end = start
                .checked_add(usize::from(size))
                .ok_or(DeviceIdentityError::BackendFailure)?;
            let bytes = self
                .certificate
                .get(start..end)
                .ok_or(DeviceIdentityError::BackendFailure)?;
            let mut parameters = Vec::with_capacity(2 + bytes.len());
            parameters.extend_from_slice(&size.to_be_bytes());
            parameters.extend_from_slice(bytes);
            Ok(session_response(&parameters))
        }
    }
    fn nv_read_public_response(index: u32, data_size: u16) -> Vec<u8> {
        let mut body = Vec::with_capacity(18);
        body.extend_from_slice(&14_u16.to_be_bytes());
        body.extend_from_slice(&index.to_be_bytes());
        body.extend_from_slice(&0x000b_u16.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        body.extend_from_slice(&data_size.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        response(&body)
    }
    fn response(body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(10 + body.len());
        bytes.extend_from_slice(&0x8001_u16.to_be_bytes());
        bytes.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(body);
        bytes
    }
    fn session_response(parameters: &[u8]) -> Vec<u8> {
        let parameter_size = u32::try_from(parameters.len()).expect("TPM test response size");
        let total_size = 10 + 4 + parameters.len() + 5;
        let mut bytes = Vec::with_capacity(total_size);
        bytes.extend_from_slice(&0x8002_u16.to_be_bytes());
        bytes.extend_from_slice(
            &u32::try_from(total_size)
                .expect("TPM test response size")
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&parameter_size.to_be_bytes());
        bytes.extend_from_slice(parameters);
        bytes.extend_from_slice(&[0, 0, 0, 0, 0]);
        bytes
    }
    #[test]
    fn test_read_nv_certificate_chunks_at_nv_buffer_max() {
        let certificate: Vec<_> = (0..1100).map(|byte| (byte % 251) as u8).collect();
        let mut transport = FakeTransport {
            certificate: certificate.clone(),
            commands: Vec::new(),
            reads: Vec::new(),
        };
        let result =
            read_nv_certificate(&mut transport, INDEX, Some(512)).expect("read the NV certificate");
        assert_eq!(result, Some(certificate));
        assert_eq!(
            transport.commands,
            [
                TpmCc::NvReadPublic.value(),
                TpmCc::NvRead.value(),
                TpmCc::NvRead.value(),
                TpmCc::NvRead.value(),
            ]
        );
        assert_eq!(transport.reads, [(512, 0), (512, 512), (76, 1024)]);
    }
    #[test]
    fn test_read_nv_certificate_rejects_missing_or_zero_nv_buffer_max() {
        for nv_buffer_max in [None, Some(0)] {
            let mut transport = FakeTransport {
                certificate: vec![0; 8],
                commands: Vec::new(),
                reads: Vec::new(),
            };
            assert_eq!(
                read_nv_certificate(&mut transport, INDEX, nv_buffer_max),
                Err(DeviceIdentityError::BackendFailure)
            );
            assert_eq!(transport.commands, [TpmCc::NvReadPublic.value()]);
            assert!(transport.reads.is_empty());
        }
    }
}
#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn ignores_unparseable_cached_certificates() {
        let certificates = vec![vec![1, 2, 3]];
        for profile in [Profile::EccP256, Profile::Rsa2048] {
            assert!(find_profile_certificates(&certificates, profile).is_empty());
        }
    }
}
