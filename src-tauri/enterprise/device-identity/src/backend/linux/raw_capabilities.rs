//! Raw inventory reads preserve identifiers rejected by the wrapper's closed enums.

use std::{ffi::CString, ptr};

use tss_esapi::{
    constants::{
        tss::{TPM2_CC_FIRST, TPM2_PT_FIXED, TPM2_PT_GROUP, TPM2_PT_VAR, TPMA_CC_V},
        CapabilityType,
    },
    tss2_esys as tss, TctiNameConf,
};

use crate::{
    AlgorithmCapability, AlgorithmId, CommandCapability, DeviceIdentityError, EccCurveId,
    PcrBankCapability, TpmCapabilities, TpmProperty,
};

/// Owns the TCTI transport and ESYS context for one TPM connection.
struct Context {
    tcti: *mut tss::TSS2_TCTI_CONTEXT,
    esys: *mut tss::ESYS_CONTEXT,
}

impl Context {
    /// Opens the selected transport and initializes ESYS for raw inventory reads.
    fn new(configuration: &TctiNameConf) -> Result<Self, DeviceIdentityError> {
        let configuration = CString::try_from(configuration.clone())
            .map_err(|_| DeviceIdentityError::BackendFailure)?;
        let mut context = Self {
            tcti: ptr::null_mut(),
            esys: ptr::null_mut(),
        };
        // The loader borrows the C string only during initialization
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

    /// Reads one capability page and returns whether more entries are available.
    /// Copies the result and releases the ESYS-owned response buffer.
    fn capability(
        &mut self,
        capability: CapabilityType,
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
                capability.into(),
                property,
                127,
                &mut more,
                &mut output,
            )
        };
        let data = if result == 0 {
            // The result is copied before releasing the ESYS allocation
            unsafe { output.as_ref() }
                .copied()
                .ok_or(DeviceIdentityError::BackendFailure)
        } else {
            Err(DeviceIdentityError::BackendFailure)
        };
        unsafe { tss::Esys_Free(output.cast()) };
        let data = data?;
        if data.capability != u32::from(capability) {
            return Err(DeviceIdentityError::BackendFailure);
        }
        Ok((more != 0, data))
    }
}

impl Drop for Context {
    /// Finalizes ESYS before releasing the TCTI transport it borrows.
    fn drop(&mut self) {
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

/// Returns no partial inventory if a capability page fails or cannot advance safely.
pub(super) fn read(configuration: &TctiNameConf) -> Result<TpmCapabilities, DeviceIdentityError> {
    let mut context = Context::new(configuration)?;
    let mut details = TpmCapabilities::default();
    for (capability, start) in [
        (CapabilityType::Algorithms, 0),
        (CapabilityType::EccCurves, 0),
        (CapabilityType::TpmProperties, TPM2_PT_FIXED),
        (CapabilityType::TpmProperties, TPM2_PT_VAR),
        (CapabilityType::Command, TPM2_CC_FIRST),
        (CapabilityType::AssignedPcr, 0),
    ] {
        let mut property = start;
        loop {
            let (more, page) = context.capability(capability, property)?;
            let mut last = None;
            // The validated capability selector determines which union member ESYS populated
            unsafe {
                match capability {
                    CapabilityType::Algorithms => {
                        let list = page.data.algorithms;
                        for entry in entries(&list.algProperties, list.count)? {
                            details.algorithms.push(AlgorithmCapability {
                                id: AlgorithmId(entry.alg),
                                flags: entry.algProperties,
                            });
                            last = Some(u32::from(entry.alg));
                        }
                    }
                    CapabilityType::EccCurves => {
                        let list = page.data.eccCurves;
                        for entry in entries(&list.eccCurves, list.count)? {
                            details.ecc_curves.push(EccCurveId(*entry));
                            last = Some(u32::from(*entry));
                        }
                    }
                    CapabilityType::TpmProperties => {
                        let list = page.data.tpmProperties;
                        let properties = if start == TPM2_PT_FIXED {
                            &mut details.fixed_properties
                        } else {
                            &mut details.variable_properties
                        };
                        for entry in entries(&list.tpmProperty, list.count)? {
                            if entry.property / TPM2_PT_GROUP == start / TPM2_PT_GROUP {
                                properties.push(TpmProperty {
                                    id: entry.property,
                                    value: entry.value,
                                });
                            }
                            last = Some(entry.property);
                        }
                    }
                    CapabilityType::Command => {
                        let list = page.data.command;
                        for entry in entries(&list.commandAttributes, list.count)? {
                            details
                                .commands
                                .push(CommandCapability { attributes: *entry });
                            // Command attributes include the vendor bit alongside the command index
                            last = Some(*entry & (TPMA_CC_V | 0xffff));
                        }
                    }
                    CapabilityType::AssignedPcr => {
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
            if capability == CapabilityType::TpmProperties
                && property / TPM2_PT_GROUP != start / TPM2_PT_GROUP
            {
                break;
            }
        }
    }
    Ok(details)
}

fn entries<T>(values: &[T], count: u32) -> Result<&[T], DeviceIdentityError> {
    values
        .get(..count as usize)
        .ok_or(DeviceIdentityError::BackendFailure)
}
