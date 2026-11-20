use std::{mem::size_of, ptr::null_mut, slice};

use picky_asn1::{bit_string::BitString, wrapper::IntegerAsn1};
use picky_asn1_x509::{
    oids, AlgorithmIdentifierParameters, Certificate, EcParameters, PublicKey, SubjectPublicKeyInfo,
};
use tpm2_protocol::{
    basic::{TpmHandle, TpmUint16, TpmUint32},
    data::{
        Tpm2bDigest, Tpm2bEccParameter, Tpm2bPublicKeyRsa, TpmAlgId, TpmEccCurve, TpmPt, TpmRc,
        TpmRcBase, TpmRh, TpmaNv, TpmaObject, TpmsEccParms, TpmsEccPoint, TpmsRsaParms,
        TpmtEccScheme, TpmtKdfScheme, TpmtPublic, TpmtRsaScheme, TpmuAsymScheme, TpmuPublicId,
        TpmuPublicParms,
    },
    frame::{
        TpmNvReadCommand, TpmNvReadPublicCommand, TpmNvReadPublicResponse, TpmNvReadResponse,
        TpmReadPublicCommand, TpmReadPublicResponse,
    },
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

use super::{
    keys, map_tpm_rc, marshal, password_auth, receive, send_frame,
    send_with_sessions_and_tpm_error, with_primary, Transport,
};
use crate::{DeviceIdentityError, TpmEndorsementKey};

const ECC_EK_CERTIFICATE_INDEX: u32 = 0x01c0_000a;
const RSA_EK_CERTIFICATE_INDEX: u32 = 0x01c0_0002;
const MAX_NV_READ_SIZE: usize = 1024;
const MIN_NV_BUFFER_MAX: u32 = 256;
// This is the SHA-256 authorization policy from the TCG default EK template.
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
    transport: &mut impl Transport,
    nv_buffer_max: Option<u32>,
) -> Result<TpmEndorsementKey, DeviceIdentityError> {
    let (profile, certificates) = select_profile(transport, nv_buffer_max, || {
        read_cached_certificates().unwrap_or_else(|error| {
            debug!(?error, "Could not read cached EK certificates");
            Vec::new()
        })
    })?;
    let template = ek_template(profile)?;
    let template_bytes = marshal(&template)?;
    let public_bytes = with_primary(
        transport,
        TpmRh::Endorsement,
        template,
        map_tpm_rc,
        |transport, handle| {
            let command = TpmReadPublicCommand { handles: [handle] };
            let response: TpmReadPublicResponse = send_frame(transport, &command)?;
            marshal(&response.out_public.inner)
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum NvReadStage {
    Public,
    Data,
}

fn allow_unreadable_nv_error<T>(
    result: Result<Result<T, TpmRc>, DeviceIdentityError>,
    stage: NvReadStage,
) -> Result<Option<T>, DeviceIdentityError> {
    match result? {
        Ok(response) => Ok(Some(response)),
        Err(TpmRc::Fmt1(code)) if code.base == TpmRcBase::Handle => Ok(None),
        Err(TpmRc::Fmt0(
            TpmRcBase::NvLocked | TpmRcBase::NvAuthorization | TpmRcBase::NvUninitialized,
        )) if stage == NvReadStage::Data => Ok(None),
        Err(TpmRc::Fmt1(code))
            if stage == NvReadStage::Data
                && matches!(
                    code.base,
                    TpmRcBase::AuthFail | TpmRcBase::BadAuth | TpmRcBase::PolicyFail
                ) =>
        {
            Ok(None)
        }
        Err(response_code) => Err(map_tpm_rc(response_code)),
    }
}

// With no certificate anywhere, use the ECC EK and leave the trust decision to Core.
fn select_profile(
    transport: &mut impl Transport,
    nv_buffer_max: Option<u32>,
    load_cached_certificates: impl FnOnce() -> Vec<Vec<u8>>,
) -> Result<(Profile, Vec<Vec<u8>>), DeviceIdentityError> {
    let cached_certificates = std::cell::LazyCell::new(load_cached_certificates);
    for profile in [Profile::EccP256, Profile::Rsa2048] {
        if let Some(certificate) =
            read_nv_certificate(transport, profile.certificate_index(), nv_buffer_max)?
        {
            return Ok((profile, vec![certificate]));
        }
        let certificates = find_profile_certificates(&cached_certificates, profile);
        if !certificates.is_empty() {
            return Ok((profile, certificates));
        }
    }
    Ok((Profile::EccP256, Vec::new()))
}

fn read_nv_certificate(
    transport: &mut impl Transport,
    index: u32,
    nv_buffer_max: Option<u32>,
) -> Result<Option<Vec<u8>>, DeviceIdentityError> {
    let index_handle = TpmHandle::from(index);
    let public_command = TpmNvReadPublicCommand {
        handles: [index_handle],
    };
    let Some(public): Option<TpmNvReadPublicResponse> = allow_unreadable_nv_error(
        send_with_sessions_and_tpm_error(transport, &public_command, &[]),
        NvReadStage::Public,
    )?
    else {
        return Ok(None);
    };
    let nv_public = public.nv_public.inner;
    if nv_public.nv_index != index_handle {
        return Err(DeviceIdentityError::BackendFailure);
    }
    if !nv_public
        .attributes
        .contains(TpmaNv::AUTHREAD | TpmaNv::NO_DA)
    {
        return Ok(None);
    }
    let data_size = usize::from(u16::from(nv_public.data_size));
    if data_size == 0 {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let Some(nv_buffer_max) = nv_buffer_max.filter(|value| *value >= MIN_NV_BUFFER_MAX) else {
        debug!(
            property = format_args!("{:#010x}", TpmPt::NvBufferMax.value()),
            value = ?nv_buffer_max.map(|value| format!("{value:#010x}")),
            minimum = MIN_NV_BUFFER_MAX,
            "TPM_PT_NV_BUFFER_MAX is missing or below minimum"
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
        let Some(response): Option<TpmNvReadResponse> = allow_unreadable_nv_error(
            send_with_sessions_and_tpm_error(transport, &command, &[password_auth()]),
            NvReadStage::Data,
        )?
        else {
            return Ok(None);
        };
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

struct ProviderHandle(NCRYPT_PROV_HANDLE);

impl Drop for ProviderHandle {
    fn drop(&mut self) {
        unsafe { NCryptFreeObject(self.0) };
    }
}

struct CertificateStore(HCERTSTORE);

impl Drop for CertificateStore {
    fn drop(&mut self) {
        unsafe { CertCloseStore(self.0, 0) };
    }
}

fn read_cached_certificates() -> Result<Vec<Vec<u8>>, DeviceIdentityError> {
    let mut provider = NCRYPT_PROV_HANDLE::default();
    let status =
        unsafe { NCryptOpenStorageProvider(&mut provider, MS_PLATFORM_CRYPTO_PROVIDER, 0) };
    if status == NTE_NOT_FOUND {
        return Ok(Vec::new());
    }
    if status != 0 {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let provider = ProviderHandle(provider);
    let mut store: HCERTSTORE = null_mut();
    let mut returned = 0;
    let status = unsafe {
        NCryptGetProperty(
            provider.0,
            NCRYPT_PCP_EKCERT_PROPERTY,
            (&mut store as *mut HCERTSTORE).cast(),
            u32::try_from(size_of::<HCERTSTORE>())
                .map_err(|_| DeviceIdentityError::BackendFailure)?,
            &mut returned,
            0,
        )
    };
    let store = if store.is_null() {
        None
    } else {
        Some(CertificateStore(store))
    };
    if status == NTE_NOT_FOUND {
        return Ok(Vec::new());
    }
    if status != 0 || returned as usize != size_of::<HCERTSTORE>() {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let store = store.ok_or(DeviceIdentityError::BackendFailure)?;
    let mut certificates = Vec::new();
    let mut previous = null_mut();
    // Each context from CertEnumCertificatesInStore stays valid until it is passed to the next
    // call, which frees it, so the encoded certificate is copied out before then.
    loop {
        unsafe { SetLastError(0) };
        let current = unsafe { CertEnumCertificatesInStore(store.0, previous) };
        if current.is_null() {
            if unsafe { GetLastError() } == CRYPT_E_NOT_FOUND as u32 {
                break;
            }
            return Err(DeviceIdentityError::BackendFailure);
        }
        previous = current;
        let context: &CERT_CONTEXT = unsafe { &*current };
        let length = context.cbCertEncoded as usize;
        if length == 0 || context.pbCertEncoded.is_null() {
            // No further enumeration call will free this context.
            unsafe { CertFreeCertificateContext(current) };
            return Err(DeviceIdentityError::BackendFailure);
        }
        certificates.push(unsafe { slice::from_raw_parts(context.pbCertEncoded, length) }.to_vec());
    }
    Ok(certificates)
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
    let symmetric = keys::aes_128_cfb();
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

pub(super) fn decode_stored_template(bytes: &[u8]) -> Result<TpmtPublic, DeviceIdentityError> {
    let template = keys::decode_public(bytes)?;
    let template_bytes = marshal(&template)?;
    let ecc_template = marshal(&ek_template(Profile::EccP256)?)?;
    let rsa_template = marshal(&ek_template(Profile::Rsa2048)?)?;
    if template_bytes == ecc_template || template_bytes == rsa_template {
        Ok(template)
    } else {
        Err(DeviceIdentityError::IdentityLost)
    }
}

fn certificate_matches(
    public_bytes: &[u8],
    certificate_der: &[u8],
) -> Result<bool, DeviceIdentityError> {
    let certificate: Certificate = picky_asn1_der::from_bytes(certificate_der)
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    let public: TpmtPublic = receive(public_bytes)?;
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
    use std::collections::VecDeque;
    use tpm2_protocol::{data::TpmCc, TpmSized};

    use super::super::{test_response_frame, TpmSt, TPM_HEADER_CODE_OFFSET, TPM_HEADER_SIZE};
    use super::*;

    const INDEX: u32 = ECC_EK_CERTIFICATE_INDEX;

    struct FakeTransport {
        certificate: Vec<u8>,
        nv_attributes: TpmaNv,
        commands: Vec<u32>,
        indexes: Vec<u32>,
        reads: Vec<(u16, u16)>,
        public_error_codes: VecDeque<Option<u32>>,
        read_error_codes: VecDeque<Option<u32>>,
        transport_error: Option<DeviceIdentityError>,
    }

    impl Transport for FakeTransport {
        fn submit(&mut self, command: &[u8]) -> Result<Vec<u8>, DeviceIdentityError> {
            if let Some(error) = self.transport_error.take() {
                return Err(error);
            }
            let command_code = u32::from_be_bytes(
                command
                    .get(TPM_HEADER_CODE_OFFSET..TPM_HEADER_SIZE)
                    .ok_or(DeviceIdentityError::BackendFailure)?
                    .try_into()
                    .map_err(|_| DeviceIdentityError::BackendFailure)?,
            );
            let index = u32::from_be_bytes(
                command
                    .get(TPM_HEADER_SIZE..TPM_HEADER_SIZE + TpmHandle::SIZE)
                    .ok_or(DeviceIdentityError::BackendFailure)?
                    .try_into()
                    .map_err(|_| DeviceIdentityError::BackendFailure)?,
            );
            self.commands.push(command_code);
            self.indexes.push(index);
            if command_code == TpmCc::NvReadPublic.value() {
                if let Some(code) = self.public_error_codes.pop_front().flatten() {
                    return Ok(error_response(code));
                }
                if index != ECC_EK_CERTIFICATE_INDEX && index != RSA_EK_CERTIFICATE_INDEX {
                    return Err(DeviceIdentityError::BackendFailure);
                }
                return Ok(nv_read_public_response(
                    index,
                    self.certificate.len() as u16,
                    self.nv_attributes,
                ));
            }
            if command_code != TpmCc::NvRead.value() {
                panic!("unexpected TPM command: {command_code:#010x}");
            }
            if let Some(code) = self.read_error_codes.pop_front().flatten() {
                return Ok(error_response(code));
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

    fn nv_read_public_response(index: u32, data_size: u16, attributes: TpmaNv) -> Vec<u8> {
        let mut body = Vec::with_capacity(18);
        body.extend_from_slice(&14_u16.to_be_bytes());
        body.extend_from_slice(&index.to_be_bytes());
        body.extend_from_slice(&0x000b_u16.to_be_bytes());
        body.extend_from_slice(&attributes.bits().to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        body.extend_from_slice(&data_size.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        response(&body)
    }

    fn response(body: &[u8]) -> Vec<u8> {
        test_response_frame(TpmSt::NoSessions, 0, body)
    }

    fn error_response(response_code: u32) -> Vec<u8> {
        test_response_frame(TpmSt::NoSessions, response_code, &[])
    }

    fn fake_transport(certificate: Vec<u8>) -> FakeTransport {
        FakeTransport {
            certificate,
            nv_attributes: TpmaNv::AUTHREAD | TpmaNv::NO_DA,
            commands: Vec::new(),
            indexes: Vec::new(),
            reads: Vec::new(),
            public_error_codes: VecDeque::new(),
            read_error_codes: VecDeque::new(),
            transport_error: None,
        }
    }

    fn session_response(parameters: &[u8]) -> Vec<u8> {
        let parameter_size = u32::try_from(parameters.len()).expect("TPM test response size");
        let mut body = Vec::with_capacity(4 + parameters.len() + 5);
        body.extend_from_slice(&parameter_size.to_be_bytes());
        body.extend_from_slice(parameters);
        body.extend_from_slice(&[0; 5]);
        test_response_frame(TpmSt::Sessions, 0, &body)
    }

    #[test]
    fn test_read_nv_certificate_chunks_at_nv_buffer_max() {
        let certificate: Vec<_> = (0..1100).map(|byte| (byte % 251) as u8).collect();
        let mut transport = fake_transport(certificate.clone());
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
    fn test_nv_read_skips_indexes_without_authread_and_no_da() {
        for attributes in [TpmaNv::AUTHREAD, TpmaNv::NO_DA, TpmaNv::empty()] {
            let mut transport = fake_transport(vec![1, 2, 3, 4]);
            transport.nv_attributes = attributes;
            assert_eq!(
                read_nv_certificate(&mut transport, INDEX, Some(512)),
                Ok(None)
            );
            assert_eq!(transport.commands, [TpmCc::NvReadPublic.value()]);
            assert!(transport.reads.is_empty());
        }
    }

    #[test]
    fn test_read_nv_certificate_rejects_missing_or_zero_nv_buffer_max() {
        for nv_buffer_max in [None, Some(0), Some(MIN_NV_BUFFER_MAX - 1)] {
            let mut transport = fake_transport(vec![0; 8]);
            assert_eq!(
                read_nv_certificate(&mut transport, INDEX, nv_buffer_max),
                Err(DeviceIdentityError::BackendFailure)
            );
            assert_eq!(transport.commands, [TpmCc::NvReadPublic.value()]);
            assert!(transport.reads.is_empty());
        }
    }

    #[test]
    fn test_read_nv_certificate_accepts_minimum_nv_buffer_max() {
        let certificate = vec![1, 2, 3, 4];
        let mut transport = fake_transport(certificate.clone());

        assert_eq!(
            read_nv_certificate(&mut transport, INDEX, Some(MIN_NV_BUFFER_MAX)),
            Ok(Some(certificate))
        );
        assert_eq!(transport.reads, [(4, 0)]);
    }

    #[test]
    fn test_unreadable_nv_certificate_errors_are_absent() {
        let mut public_transport = fake_transport(vec![1, 2, 3, 4]);
        public_transport.public_error_codes = VecDeque::from([Some(0x0000_018b)]);
        assert_eq!(
            read_nv_certificate(&mut public_transport, INDEX, Some(512)),
            Ok(None)
        );
        assert_eq!(public_transport.commands, [TpmCc::NvReadPublic.value()]);

        for response_code in [0x0000_0148, 0x0000_0149, 0x0000_014a, 0x0000_098e] {
            let mut read_transport = fake_transport(vec![1, 2, 3, 4]);
            read_transport.read_error_codes = VecDeque::from([Some(response_code)]);
            assert_eq!(
                read_nv_certificate(&mut read_transport, INDEX, Some(512)),
                Ok(None),
                "response code {response_code:#010x}"
            );
            assert_eq!(
                read_transport.commands,
                [TpmCc::NvReadPublic.value(), TpmCc::NvRead.value()]
            );
        }
    }

    #[test]
    fn test_unreadable_ecc_certificate_falls_back_to_rsa() {
        let certificate = vec![1, 2, 3, 4];
        let mut transport = fake_transport(certificate.clone());
        transport.read_error_codes = VecDeque::from([Some(0x0000_014a), None]);

        let (profile, certificates) =
            select_profile(&mut transport, Some(512), Vec::new).expect("select RSA EK");

        assert!(matches!(profile, Profile::Rsa2048));
        assert_eq!(certificates, [certificate]);
        assert_eq!(
            transport.indexes,
            [
                ECC_EK_CERTIFICATE_INDEX,
                ECC_EK_CERTIFICATE_INDEX,
                RSA_EK_CERTIFICATE_INDEX,
                RSA_EK_CERTIFICATE_INDEX
            ]
        );
    }

    #[test]
    fn test_unreadable_ek_certificates_fall_back_to_empty_ecc_chain() {
        let mut transport = fake_transport(vec![1, 2, 3, 4]);
        transport.public_error_codes = VecDeque::from([Some(0x0000_018b), Some(0x0000_018b)]);

        let (profile, certificates) =
            select_profile(&mut transport, Some(512), Vec::new).expect("select empty ECC EK");

        assert!(matches!(profile, Profile::EccP256));
        assert!(certificates.is_empty());
        assert_eq!(
            transport.indexes,
            [ECC_EK_CERTIFICATE_INDEX, RSA_EK_CERTIFICATE_INDEX]
        );
    }

    #[test]
    fn test_nv_certificate_propagates_temporary_and_transport_errors() {
        let mut temporary_tpm_error = fake_transport(vec![1, 2, 3, 4]);
        temporary_tpm_error.public_error_codes = VecDeque::from([Some(0x0000_0922)]);
        assert_eq!(
            read_nv_certificate(&mut temporary_tpm_error, INDEX, Some(512)),
            Err(DeviceIdentityError::TemporarilyUnavailable)
        );

        let mut general_tpm_error = fake_transport(vec![1, 2, 3, 4]);
        general_tpm_error.read_error_codes = VecDeque::from([Some(0x0000_0101)]);
        assert_eq!(
            read_nv_certificate(&mut general_tpm_error, INDEX, Some(512)),
            Err(DeviceIdentityError::BackendFailure)
        );

        let mut public_auth_error = fake_transport(vec![1, 2, 3, 4]);
        public_auth_error.public_error_codes = VecDeque::from([Some(0x0000_098e)]);
        assert_eq!(
            read_nv_certificate(&mut public_auth_error, INDEX, Some(512)),
            Err(DeviceIdentityError::PermissionDenied)
        );

        let mut transport_error = fake_transport(vec![1, 2, 3, 4]);
        transport_error.transport_error = Some(DeviceIdentityError::BackendFailure);
        assert_eq!(
            read_nv_certificate(&mut transport_error, INDEX, Some(512)),
            Err(DeviceIdentityError::BackendFailure)
        );
    }
}
#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn test_ignores_unparseable_cached_certificates() {
        let certificates = vec![vec![1, 2, 3]];
        for profile in [Profile::EccP256, Profile::Rsa2048] {
            assert!(find_profile_certificates(&certificates, profile).is_empty());
        }
    }
}

#[cfg(test)]
mod template_tests {
    use super::*;

    #[test]
    fn test_decode_stored_template_accepts_default_ek_templates() {
        for profile in [Profile::EccP256, Profile::Rsa2048] {
            let template = ek_template(profile).expect("build default EK template");
            let bytes = marshal(&template).expect("marshal default EK template");
            assert!(decode_stored_template(&bytes).is_ok());
        }
    }

    #[test]
    fn test_decode_stored_template_rejects_changed_template() {
        let mut template = ek_template(Profile::EccP256).expect("build default ECC EK template");
        template.name_alg = TpmAlgId::Null;
        let bytes = marshal(&template).expect("marshal changed EK template");

        assert_eq!(
            decode_stored_template(&bytes).map(|_| ()),
            Err(DeviceIdentityError::IdentityLost)
        );
    }
}
