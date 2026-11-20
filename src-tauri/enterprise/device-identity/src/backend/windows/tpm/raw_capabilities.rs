use tpm2_protocol::{
    basic::{TpmUint16, TpmUint32, TpmUint8},
    constant::TPM_PCR_SELECT_MAX,
    data::{TpmAlgId, TpmCap, TpmCc, TpmEccCurve, TpmPt, TpmiYesNo},
    TpmMarshal, TpmResult, TpmSized, TpmUnmarshal, TpmWriter,
};

use super::{marshal, send, Transport};
use crate::{
    AlgorithmCapability, AlgorithmId, AttestationMethod, BackendKind, Capabilities,
    CommandCapability, DeviceIdentityError, EccCurveId, PcrBankCapability, PlatformCapabilities,
    ProfileId, ProfileSupport, SupportStatus, TpmCapabilities, TpmProperty,
};

const CAPABILITY_PAGE_SIZE: u32 = 64;
const MAX_CAPABILITY_PAGES: usize = 64;
const TPM_PT_GROUP: u32 = 0x100;
const TPMA_CC_V: u32 = 1 << 29;
const OWNER_AUTH_SET: u32 = 1 << 0;
const ENDORSEMENT_AUTH_SET: u32 = 1 << 1;

struct GetCapabilityParameters {
    capability: TpmCap,
    property: TpmUint32,
    property_count: TpmUint32,
}

impl TpmSized for GetCapabilityParameters {
    const SIZE: usize = TpmCap::SIZE + TpmUint32::SIZE * 2;

    fn len(&self) -> usize {
        Self::SIZE
    }
}

impl TpmMarshal for GetCapabilityParameters {
    fn marshal(&self, writer: &mut TpmWriter) -> TpmResult<()> {
        self.capability.marshal(writer)?;
        self.property.marshal(writer)?;
        self.property_count.marshal(writer)
    }
}

struct GetCapabilityResponse {
    more_data: bool,
    capability_data: Vec<u8>,
}

impl TpmUnmarshal for GetCapabilityResponse {
    fn unmarshal(bytes: &[u8]) -> TpmResult<(Self, &[u8])> {
        let (more_data, capability_data) = TpmiYesNo::unmarshal(bytes)?;
        Ok((
            Self {
                more_data: bool::from(more_data),
                capability_data: capability_data.to_vec(),
            },
            &[],
        ))
    }
}

struct Reader<'a> {
    remaining: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    fn read<T: TpmUnmarshal>(&mut self) -> Result<T, DeviceIdentityError> {
        let (value, remaining) =
            T::unmarshal(self.remaining).map_err(|_| DeviceIdentityError::BackendFailure)?;
        self.remaining = remaining;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, DeviceIdentityError> {
        Ok(u8::from(self.read::<TpmUint8>()?))
    }

    fn u16(&mut self) -> Result<u16, DeviceIdentityError> {
        Ok(u16::from(self.read::<TpmUint16>()?))
    }

    fn u32(&mut self) -> Result<u32, DeviceIdentityError> {
        Ok(u32::from(self.read::<TpmUint32>()?))
    }

    fn bytes(&mut self, len: usize) -> Result<Vec<u8>, DeviceIdentityError> {
        if len > self.remaining.len() {
            return Err(DeviceIdentityError::BackendFailure);
        }
        let (bytes, remaining) = self.remaining.split_at(len);
        self.remaining = remaining;
        Ok(bytes.to_vec())
    }

    fn finish(self) -> Result<(), DeviceIdentityError> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(DeviceIdentityError::BackendFailure)
        }
    }
}

struct CapabilityPage<T> {
    more_data: bool,
    entries: Vec<T>,
}

fn read_pages<T>(
    transport: &mut impl Transport,
    capability: TpmCap,
    mut property: u32,
    mut parse_item: impl for<'a> FnMut(&mut Reader<'a>) -> Result<T, DeviceIdentityError>,
    item_property: impl Fn(&T) -> u32,
    item_group_size: Option<u32>,
) -> Result<Vec<T>, DeviceIdentityError> {
    let mut entries = Vec::new();
    let start = property;
    for _ in 0..MAX_CAPABILITY_PAGES {
        let page = read_page(transport, capability, property, &mut parse_item)?;
        let mut previous = None;
        for entry in &page.entries {
            let current = item_property(entry);
            if current < property || previous.is_some_and(|previous| current <= previous) {
                return Err(DeviceIdentityError::BackendFailure);
            }
            previous = Some(current);
        }
        let crossed_group = item_group_size.is_some_and(|size| {
            page.entries
                .iter()
                .any(|entry| item_property(entry) / size != property / size)
        });
        entries.extend(page.entries.into_iter().filter(|entry| {
            item_group_size.is_none_or(|size| item_property(entry) / size == property / size)
        }));
        if crossed_group || !page.more_data {
            return Ok(entries);
        }
        let last_property = previous.ok_or(DeviceIdentityError::BackendFailure)?;
        property = last_property
            .checked_add(1)
            .ok_or(DeviceIdentityError::BackendFailure)?;
        if item_group_size.is_some_and(|size| property / size != start / size) {
            return Ok(entries);
        }
    }
    Err(DeviceIdentityError::BackendFailure)
}

fn read_page<T>(
    transport: &mut impl Transport,
    capability: TpmCap,
    property: u32,
    parse_item: &mut impl for<'a> FnMut(&mut Reader<'a>) -> Result<T, DeviceIdentityError>,
) -> Result<CapabilityPage<T>, DeviceIdentityError> {
    let parameters = GetCapabilityParameters {
        capability,
        property: TpmUint32::from(property),
        property_count: TpmUint32::from(CAPABILITY_PAGE_SIZE),
    };
    let command_parameters = marshal(&parameters)?;
    let response: GetCapabilityResponse =
        send(transport, TpmCc::GetCapability, &command_parameters)?;
    let mut reader = Reader::new(&response.capability_data);
    if reader.u32()? != capability.value() {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let count = usize::try_from(reader.u32()?).map_err(|_| DeviceIdentityError::BackendFailure)?;
    if count > CAPABILITY_PAGE_SIZE as usize {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(parse_item(&mut reader)?);
    }
    reader.finish()?;
    Ok(CapabilityPage {
        more_data: response.more_data,
        entries,
    })
}

pub(super) fn read(transport: &mut impl Transport) -> Result<Capabilities, DeviceIdentityError> {
    let algorithms = read_pages(
        transport,
        TpmCap::Algs,
        0,
        |reader| {
            Ok(AlgorithmCapability {
                id: AlgorithmId(reader.u16()?),
                flags: reader.u32()?,
            })
        },
        |algorithm| u32::from(algorithm.id.0),
        None,
    )?;
    let ecc_curves = read_pages(
        transport,
        TpmCap::EccCurves,
        0,
        |reader| Ok(EccCurveId(reader.u16()?)),
        |curve| u32::from(curve.0),
        None,
    )?;
    let fixed_properties = read_pages(
        transport,
        TpmCap::TpmProperties,
        TpmPt::FamilyIndicator.value(),
        |reader| {
            Ok(TpmProperty {
                id: reader.u32()?,
                value: reader.u32()?,
            })
        },
        |property| property.id,
        Some(TPM_PT_GROUP),
    )?;
    let variable_properties = read_pages(
        transport,
        TpmCap::TpmProperties,
        TpmPt::Permanent.value(),
        |reader| {
            Ok(TpmProperty {
                id: reader.u32()?,
                value: reader.u32()?,
            })
        },
        |property| property.id,
        Some(TPM_PT_GROUP),
    )?;
    let commands = read_pages(
        transport,
        TpmCap::Commands,
        0,
        |reader| {
            Ok(CommandCapability {
                attributes: reader.u32()?,
            })
        },
        |command| command.attributes & (TPMA_CC_V | 0x0000_FFFF),
        None,
    )?;
    let pcr_banks = read_pages(
        transport,
        TpmCap::Pcrs,
        0,
        |reader| {
            let hash_algorithm = AlgorithmId(reader.u16()?);
            let selection_len = usize::from(reader.u8()?);
            if selection_len == 0 || selection_len > TPM_PCR_SELECT_MAX as usize {
                return Err(DeviceIdentityError::BackendFailure);
            }
            Ok(PcrBankCapability {
                hash_algorithm,
                selection: reader.bytes(selection_len)?,
            })
        },
        |bank| u32::from(bank.hash_algorithm.0),
        None,
    )?;

    let has_algorithm = |id| algorithms.iter().any(|algorithm| algorithm.id.0 == id);
    let ecdsa_supported = has_algorithm(TpmAlgId::Ecc.value())
        && has_algorithm(TpmAlgId::Ecdsa.value())
        && has_algorithm(TpmAlgId::Sha256.value())
        && ecc_curves
            .iter()
            .any(|curve| curve.0 == TpmEccCurve::NistP256.value());
    let ecdsa_status = if ecdsa_supported {
        SupportStatus::Supported
    } else {
        SupportStatus::Unsupported
    };
    let details = TpmCapabilities {
        algorithms,
        ecc_curves,
        fixed_properties,
        variable_properties,
        commands,
        pcr_banks,
        profiles: vec![
            ProfileSupport {
                profile: ProfileId::EcdsaP256Sha256,
                support: ecdsa_status,
            },
            ProfileSupport {
                profile: ProfileId::RsaSsa2048Sha256,
                support: SupportStatus::NotChecked,
            },
        ],
    };
    Ok(Capabilities {
        backend: BackendKind::Tpm,
        attestation_method: ecdsa_supported.then_some(AttestationMethod::TpmKeyAttestation),
        details: PlatformCapabilities::Tpm(details),
    })
}

pub(super) fn nv_buffer_max(capabilities: &Capabilities) -> Option<u32> {
    let PlatformCapabilities::Tpm(details) = &capabilities.details else {
        return None;
    };
    details
        .fixed_properties
        .iter()
        .find(|property| property.id == TpmPt::NvBufferMax.value())
        .map(|property| property.value)
}
pub(super) fn check_hierarchy_auth(capabilities: &Capabilities) -> Result<(), DeviceIdentityError> {
    let PlatformCapabilities::Tpm(details) = &capabilities.details else {
        return Err(DeviceIdentityError::BackendFailure);
    };
    let permanent = details
        .variable_properties
        .iter()
        .find(|property| property.id == TpmPt::Permanent.value())
        .ok_or(DeviceIdentityError::BackendFailure)?;
    if permanent.value & (OWNER_AUTH_SET | ENDORSEMENT_AUTH_SET) != 0 {
        return Err(DeviceIdentityError::Unsupported);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    struct FakeTransport {
        responses: VecDeque<Vec<u8>>,
        requested_properties: Vec<u32>,
    }

    impl Transport for FakeTransport {
        fn submit(&mut self, command: &[u8]) -> Result<Vec<u8>, DeviceIdentityError> {
            let property = command
                .get(14..18)
                .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
                .map(u32::from_be_bytes)
                .ok_or(DeviceIdentityError::BackendFailure)?;
            self.requested_properties.push(property);
            self.responses
                .pop_front()
                .ok_or(DeviceIdentityError::BackendFailure)
        }
    }

    fn response(more_data: bool, capability: TpmCap, entries: &[[u8; 6]]) -> Vec<u8> {
        let payload: Vec<_> = entries.iter().flatten().copied().collect();
        response_with_payload(more_data, capability, entries.len() as u32, &payload)
    }

    fn response_with_payload(
        more_data: bool,
        capability: TpmCap,
        count: u32,
        payload: &[u8],
    ) -> Vec<u8> {
        let mut body = vec![u8::from(more_data)];
        body.extend_from_slice(&capability.value().to_be_bytes());
        body.extend_from_slice(&count.to_be_bytes());
        body.extend_from_slice(payload);
        let mut response = Vec::with_capacity(10 + body.len());
        response.extend_from_slice(&0x8001_u16.to_be_bytes());
        response.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        response.extend_from_slice(&0_u32.to_be_bytes());
        response.extend_from_slice(&body);
        response
    }

    #[test]
    fn test_read_uses_variable_properties_for_permanent_auth_bits() {
        let mut transport = FakeTransport {
            responses: VecDeque::from([
                response(
                    false,
                    TpmCap::Algs,
                    &[
                        [0x00, 0x0b, 0, 0, 0, 4],
                        [0x00, 0x18, 0, 0, 1, 0],
                        [0x00, 0x23, 0, 0, 0, 9],
                    ],
                ),
                response_with_payload(false, TpmCap::EccCurves, 1, &[0x00, 0x03]),
                response_with_payload(
                    false,
                    TpmCap::TpmProperties,
                    2,
                    &[
                        0x00, 0x00, 0x01, 0x00, 0x31, 0x2e, 0x30, 0x00, 0x00, 0x00, 0x01, 0x2c,
                        0x00, 0x00, 0x02, 0x00,
                    ],
                ),
                response_with_payload(
                    false,
                    TpmCap::TpmProperties,
                    1,
                    &[0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x03],
                ),
                response_with_payload(false, TpmCap::Commands, 0, &[]),
                response_with_payload(false, TpmCap::Pcrs, 0, &[]),
            ]),
            requested_properties: Vec::new(),
        };

        let capabilities = read(&mut transport).expect("complete capability inventory");
        assert_eq!(
            capabilities.attestation_method,
            Some(AttestationMethod::TpmKeyAttestation)
        );
        assert_eq!(
            check_hierarchy_auth(&capabilities),
            Err(DeviceIdentityError::Unsupported)
        );
        assert_eq!(nv_buffer_max(&capabilities), Some(512));
        let PlatformCapabilities::Tpm(details) = capabilities.details else {
            panic!("TPM details expected");
        };
        assert_eq!(details.fixed_properties.len(), 2);
        assert_eq!(
            details.fixed_properties[0].id,
            TpmPt::FamilyIndicator.value()
        );
        assert_eq!(details.fixed_properties[1].id, TpmPt::NvBufferMax.value());
        assert_eq!(details.variable_properties.len(), 1);
        assert_eq!(details.variable_properties[0].id, TpmPt::Permanent.value());
        assert!(details.profiles.iter().any(|profile| {
            profile.profile == ProfileId::EcdsaP256Sha256
                && profile.support == SupportStatus::Supported
        }));
        assert!(transport.responses.is_empty());
    }

    #[test]
    fn test_read_pages_preserves_unknown_algorithm_ids_and_flags() {
        let mut transport = FakeTransport {
            responses: VecDeque::from([
                response(true, TpmCap::Algs, &[[0x00, 0x01, 0, 0, 0, 1]]),
                response(false, TpmCap::Algs, &[[0xff, 0xff, 0xde, 0xad, 0xbe, 0xef]]),
            ]),
            requested_properties: Vec::new(),
        };

        let algorithms = read_pages(
            &mut transport,
            TpmCap::Algs,
            0,
            |reader| {
                Ok(AlgorithmCapability {
                    id: AlgorithmId(reader.u16()?),
                    flags: reader.u32()?,
                })
            },
            |algorithm| u32::from(algorithm.id.0),
            None,
        )
        .expect("valid capability pages");

        assert_eq!(algorithms[0].id, AlgorithmId(0x0001));
        assert_eq!(algorithms[0].flags, 1);
        assert_eq!(algorithms[1].id, AlgorithmId(0xffff));
        assert_eq!(algorithms[1].flags, 0xdead_beef);
    }

    #[test]
    fn test_read_pages_keeps_vendor_bit_in_command_paging_key() {
        let standard_commands: Vec<_> = (1_u32..=CAPABILITY_PAGE_SIZE)
            .flat_map(|command| command.to_be_bytes())
            .collect();
        let vendor_command_1 = (TPMA_CC_V | 1).to_be_bytes();
        let vendor_command_2 = (TPMA_CC_V | 2).to_be_bytes();
        let mut transport = FakeTransport {
            responses: VecDeque::from([
                response(false, TpmCap::Algs, &[]),
                response_with_payload(false, TpmCap::EccCurves, 0, &[]),
                response_with_payload(false, TpmCap::TpmProperties, 0, &[]),
                response_with_payload(false, TpmCap::TpmProperties, 0, &[]),
                response_with_payload(
                    true,
                    TpmCap::Commands,
                    CAPABILITY_PAGE_SIZE,
                    &standard_commands,
                ),
                response_with_payload(true, TpmCap::Commands, 1, &vendor_command_1),
                response_with_payload(false, TpmCap::Commands, 1, &vendor_command_2),
                response_with_payload(false, TpmCap::Pcrs, 0, &[]),
            ]),
            requested_properties: Vec::new(),
        };

        let capabilities = read(&mut transport).expect("page standard and vendor commands");
        let PlatformCapabilities::Tpm(details) = capabilities.details else {
            panic!("TPM details expected");
        };
        let commands = details.commands;

        assert_eq!(commands.len(), 66);
        assert_eq!(commands[64].attributes, TPMA_CC_V | 1);
        assert_eq!(commands[65].attributes, TPMA_CC_V | 2);
        assert_eq!(
            transport.requested_properties,
            [0, 0, 0x100, 0x200, 0, 65, TPMA_CC_V | 2, 0]
        );
    }

    #[test]
    fn test_read_pages_filters_properties_from_another_group() {
        let mut transport = FakeTransport {
            responses: VecDeque::from([response_with_payload(
                true,
                TpmCap::TpmProperties,
                2,
                &[
                    0x00, 0x00, 0x01, 0xff, 0, 0, 0, 1, 0x00, 0x00, 0x02, 0x00, 0, 0, 0, 2,
                ],
            )]),
            requested_properties: Vec::new(),
        };

        let properties = read_pages(
            &mut transport,
            TpmCap::TpmProperties,
            TpmPt::FamilyIndicator.value(),
            |reader| {
                Ok(TpmProperty {
                    id: reader.u32()?,
                    value: reader.u32()?,
                })
            },
            |property| property.id,
            Some(TPM_PT_GROUP),
        )
        .expect("fixed property page");

        assert_eq!(
            properties
                .iter()
                .map(|property| property.id)
                .collect::<Vec<_>>(),
            [0x1ff]
        );
        assert_eq!(
            transport.requested_properties,
            [TpmPt::FamilyIndicator.value()]
        );
        assert!(transport.responses.is_empty());
    }

    #[test]
    fn test_read_pages_stops_when_next_property_leaves_group() {
        let mut transport = FakeTransport {
            responses: VecDeque::from([response_with_payload(
                true,
                TpmCap::TpmProperties,
                1,
                &[0x00, 0x00, 0x01, 0xff, 0, 0, 0, 1],
            )]),
            requested_properties: Vec::new(),
        };
        let properties = read_pages(
            &mut transport,
            TpmCap::TpmProperties,
            TpmPt::FamilyIndicator.value(),
            |reader| {
                Ok(TpmProperty {
                    id: reader.u32()?,
                    value: reader.u32()?,
                })
            },
            |property| property.id,
            Some(TPM_PT_GROUP),
        )
        .expect("stop at group boundary");

        assert_eq!(properties.len(), 1);
        assert_eq!(properties[0].id, 0x1ff);
        assert_eq!(
            transport.requested_properties,
            [TpmPt::FamilyIndicator.value()]
        );
    }

    #[test]
    fn test_read_pages_rejects_empty_or_non_advancing_pages() {
        for pages in [
            vec![response(true, TpmCap::Algs, &[])],
            vec![
                response(true, TpmCap::Algs, &[[0x00, 0x01, 0, 0, 0, 1]]),
                response(false, TpmCap::Algs, &[[0x00, 0x01, 0, 0, 0, 1]]),
            ],
        ] {
            let mut transport = FakeTransport {
                responses: VecDeque::from(pages),
                requested_properties: Vec::new(),
            };
            let result = read_pages(
                &mut transport,
                TpmCap::Algs,
                0,
                |reader| {
                    Ok(AlgorithmCapability {
                        id: AlgorithmId(reader.u16()?),
                        flags: reader.u32()?,
                    })
                },
                |algorithm| u32::from(algorithm.id.0),
                None,
            );
            assert_eq!(result, Err(DeviceIdentityError::BackendFailure));
        }
    }
}
