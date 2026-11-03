use std::{env, ffi::CString, ptr};

use tss_esapi_sys as tss;

use crate::{
    AlgorithmCapability, AlgorithmId, AttestationIdentity, AttestationMethod, AttestationRequest,
    AttestationResponse, BackendKind, Capabilities, CommandCapability, DeviceIdentityError,
    EccCurveId, IdentityRef, PcrBankCapability, PlatformCapabilities, PossessionChallenge,
    PossessionProof, ProfileId, ProfileSupport, SupportStatus, TpmCapabilities, TpmProperty,
};

const CAP_ALGORITHMS: u32 = 0;
const CAP_COMMANDS: u32 = 2;
const CAP_PCRS: u32 = 5;
const CAP_PROPERTIES: u32 = 6;
const CAP_CURVES: u32 = 8;
const FIXED_PROPERTIES: u32 = 0x100;
const VARIABLE_PROPERTIES: u32 = 0x200;
const ALG_RSA: u16 = 0x0001;
const ALG_SHA256: u16 = 0x000b;
const ALG_NULL: u16 = 0x0010;
const ALG_RSASSA: u16 = 0x0014;
const ALG_ECDSA: u16 = 0x0018;
const ALG_ECC: u16 = 0x0023;
const CURVE_P256: u16 = 0x0003;

struct Context {
    tcti: *mut tss::TSS2_TCTI_CONTEXT,
    esys: *mut tss::ESYS_CONTEXT,
}

impl Context {
    fn new() -> Result<Self, DeviceIdentityError> {
        let configuration = match env::var("TSS2_TCTI") {
            Ok(value) => value,
            Err(env::VarError::NotPresent) => "device:/dev/tpmrm0".into(),
            Err(env::VarError::NotUnicode(_)) => return Err(DeviceIdentityError::BackendFailure),
        };
        let configuration =
            CString::new(configuration).map_err(|_| DeviceIdentityError::BackendFailure)?;
        let mut context = Self {
            tcti: ptr::null_mut(),
            esys: ptr::null_mut(),
        };
        // the loader borrows the C string only during initialization
        let result =
            unsafe { tss::Tss2_TctiLdr_Initialize(configuration.as_ptr(), &mut context.tcti) };
        if result != 0 {
            return Err(DeviceIdentityError::Unavailable);
        }
        // ESYS borrows the transport, which remains owned by this context
        let result =
            unsafe { tss::Esys_Initialize(&mut context.esys, context.tcti, ptr::null_mut()) };
        if result != 0 {
            return Err(DeviceIdentityError::BackendFailure);
        }
        Ok(context)
    }

    fn capability(
        &mut self,
        capability: u32,
        property: u32,
    ) -> Result<(bool, tss::TPMS_CAPABILITY_DATA), DeviceIdentityError> {
        let mut more = 0;
        let mut output = ptr::null_mut();
        // ESYS allocates the result, which must be released with Esys_Free
        let result = unsafe {
            tss::Esys_GetCapability(
                self.esys,
                tss::ESYS_TR_NONE,
                tss::ESYS_TR_NONE,
                tss::ESYS_TR_NONE,
                capability,
                property,
                127,
                &mut more,
                &mut output,
            )
        };
        let data = if result == 0 {
            // the result is copied before releasing the ESYS allocation
            unsafe { output.as_ref() }
                .copied()
                .ok_or(DeviceIdentityError::BackendFailure)
        } else {
            Err(DeviceIdentityError::BackendFailure)
        };
        unsafe { tss::Esys_Free(output.cast()) };
        let data = data?;
        if data.capability != capability {
            return Err(DeviceIdentityError::BackendFailure);
        }
        Ok((more != 0, data))
    }

    fn profile(&mut self, profile: ProfileId) -> Result<ProfileSupport, DeviceIdentityError> {
        let symmetric = tss::TPMT_SYM_DEF_OBJECT {
            algorithm: ALG_NULL,
            ..Default::default()
        };
        let hash = tss::TPMS_SCHEME_HASH {
            hashAlg: ALG_SHA256,
        };
        let parameters = match profile {
            ProfileId::EcdsaP256Sha256 => tss::TPMT_PUBLIC_PARMS {
                type_: ALG_ECC,
                parameters: tss::TPMU_PUBLIC_PARMS {
                    eccDetail: tss::TPMS_ECC_PARMS {
                        symmetric,
                        scheme: tss::TPMT_ECC_SCHEME {
                            scheme: ALG_ECDSA,
                            details: tss::TPMU_ASYM_SCHEME { ecdsa: hash },
                        },
                        curveID: CURVE_P256,
                        kdf: tss::TPMT_KDF_SCHEME {
                            scheme: ALG_NULL,
                            ..Default::default()
                        },
                    },
                },
            },
            ProfileId::RsaSsa2048Sha256 => tss::TPMT_PUBLIC_PARMS {
                type_: ALG_RSA,
                parameters: tss::TPMU_PUBLIC_PARMS {
                    rsaDetail: tss::TPMS_RSA_PARMS {
                        symmetric,
                        scheme: tss::TPMT_RSA_SCHEME {
                            scheme: ALG_RSASSA,
                            details: tss::TPMU_ASYM_SCHEME { rsassa: hash },
                        },
                        keyBits: 2048,
                        exponent: 0,
                    },
                },
            },
        };
        // TestParms checks the parameter combination without creating or persisting a key
        let result = unsafe {
            tss::Esys_TestParms(
                self.esys,
                tss::ESYS_TR_NONE,
                tss::ESYS_TR_NONE,
                tss::ESYS_TR_NONE,
                &parameters,
            )
        };
        let support = if result == 0 {
            SupportStatus::Supported
        } else if result <= 0xffff
            && matches!(
                result & 0xbf,
                0x81 | 0x82 | 0x83 | 0x84 | 0x87 | 0x89 | 0x8a | 0x8c | 0x92 | 0x96 | 0xa6
            )
        {
            // only TPM parameter rejections mean Unsupported, transport failures are errors
            SupportStatus::Unsupported
        } else {
            return Err(DeviceIdentityError::BackendFailure);
        };
        Ok(ProfileSupport { profile, support })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // ESYS must release its transport reference before the loader destroys the transport
        unsafe {
            if !self.esys.is_null() {
                tss::Esys_Finalize(&mut self.esys);
            }
            if !self.tcti.is_null() {
                tss::Tss2_TctiLdr_Finalize(&mut self.tcti);
            }
        }
    }
}

pub(super) fn capabilities() -> Result<Capabilities, DeviceIdentityError> {
    let mut context = Context::new()?;
    let mut details = TpmCapabilities::default();
    for (capability, start) in [
        (CAP_ALGORITHMS, 0),
        (CAP_CURVES, 0),
        (CAP_PROPERTIES, FIXED_PROPERTIES),
        (CAP_PROPERTIES, VARIABLE_PROPERTIES),
        (CAP_COMMANDS, 0x011f),
        (CAP_PCRS, 0),
    ] {
        let mut property = start;
        loop {
            let (more, page) = context.capability(capability, property)?;
            let mut last = None;
            // the validated capability selector determines which union member ESYS populated
            unsafe {
                match capability {
                    CAP_ALGORITHMS => {
                        let list = page.data.algorithms;
                        for entry in entries(&list.algProperties, list.count)? {
                            details.algorithms.push(AlgorithmCapability {
                                id: AlgorithmId(entry.alg),
                                flags: entry.algProperties,
                            });
                            last = Some(u32::from(entry.alg));
                        }
                    }
                    CAP_CURVES => {
                        let list = page.data.eccCurves;
                        for entry in entries(&list.eccCurves, list.count)? {
                            details.ecc_curves.push(EccCurveId(*entry));
                            last = Some(u32::from(*entry));
                        }
                    }
                    CAP_PROPERTIES => {
                        let list = page.data.tpmProperties;
                        let properties = if start == FIXED_PROPERTIES {
                            &mut details.fixed_properties
                        } else {
                            &mut details.variable_properties
                        };
                        for entry in entries(&list.tpmProperty, list.count)? {
                            if entry.property / 0x100 == start / 0x100 {
                                properties.push(TpmProperty {
                                    id: entry.property,
                                    value: entry.value,
                                });
                            }
                            last = Some(entry.property);
                        }
                    }
                    CAP_COMMANDS => {
                        let list = page.data.command;
                        for entry in entries(&list.commandAttributes, list.count)? {
                            details
                                .commands
                                .push(CommandCapability { attributes: *entry });
                            // command attributes include the vendor bit alongside the command index
                            last = Some(*entry & 0x2000_ffff);
                        }
                    }
                    CAP_PCRS => {
                        let list = page.data.assignedPCR;
                        for entry in entries(&list.pcrSelections, list.count)? {
                            details.pcr_banks.push(PcrBankCapability {
                                hash_algorithm: AlgorithmId(entry.hash),
                                selection: entries(
                                    &entry.pcrSelect,
                                    u32::from(entry.sizeofSelect),
                                )?
                                .to_vec(),
                            });
                        }
                    }
                    _ => return Err(DeviceIdentityError::BackendFailure),
                }
            }
            if !more {
                break;
            }
            property = last
                .filter(|last| *last >= property)
                .and_then(|last| last.checked_add(1))
                .ok_or(DeviceIdentityError::BackendFailure)?;
            if capability == CAP_PROPERTIES && property / 0x100 != start / 0x100 {
                break;
            }
        }
    }
    details.profiles = [ProfileId::EcdsaP256Sha256, ProfileId::RsaSsa2048Sha256]
        .into_iter()
        .map(|profile| context.profile(profile))
        .collect::<Result<_, _>>()?;
    let attestation_methods = if details.profiles.iter().any(|profile| {
        profile.profile == ProfileId::EcdsaP256Sha256 && profile.support == SupportStatus::Supported
    }) {
        vec![AttestationMethod::TpmKeyAttestation]
    } else {
        Vec::new()
    };
    Ok(Capabilities {
        backend: BackendKind::Tpm,
        attestation_methods,
        details: PlatformCapabilities::Tpm(details),
    })
}

fn entries<T>(values: &[T], count: u32) -> Result<&[T], DeviceIdentityError> {
    values
        .get(..count as usize)
        .ok_or(DeviceIdentityError::BackendFailure)
}

pub(super) async fn create_identity() -> Result<IdentityRef, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(super) fn attestation_identity(
    _identity: &IdentityRef,
) -> Result<AttestationIdentity, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(super) async fn attest_identity(
    _identity: &IdentityRef,
    _request: &AttestationRequest,
) -> Result<AttestationResponse, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}

pub(super) async fn prove_possession(
    _identity: &IdentityRef,
    _challenge: &PossessionChallenge,
) -> Result<PossessionProof, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}
