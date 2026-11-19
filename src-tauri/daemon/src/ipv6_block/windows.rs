use std::{io, iter::once, ptr};

use windows_sys::Win32::{
    Foundation::{ERROR_SUCCESS, HANDLE},
    NetworkManagement::{
        IpHelper::ConvertInterfaceAliasToLuid,
        Ndis::NET_LUID_LH,
        WindowsFilteringPlatform::{
            FwpmEngineClose0, FwpmEngineOpen0, FwpmFilterAdd0, FWPM_ACTION0,
            FWPM_CONDITION_IP_LOCAL_INTERFACE, FWPM_DISPLAY_DATA0, FWPM_FILTER0,
            FWPM_FILTER_CONDITION0, FWPM_FILTER_FLAG_CLEAR_ACTION_RIGHT,
            FWPM_LAYER_ALE_AUTH_CONNECT_V6, FWPM_SESSION0, FWPM_SESSION_FLAG_DYNAMIC,
            FWP_ACTION_BLOCK, FWP_CONDITION_VALUE0, FWP_CONDITION_VALUE0_0, FWP_MATCH_EQUAL,
            FWP_UINT64,
        },
    },
    System::Rpc::RPC_C_AUTHN_WINNT,
};

/// Blocks IPv6 connections that leave through a tunnel interface.
///
/// The WFP session is dynamic, so Windows removes the filter when this value is dropped or the
/// service process exits.
pub(crate) struct Ipv6Block(HANDLE);

unsafe impl Send for Ipv6Block {}

impl Ipv6Block {
    pub(crate) fn new(ifname: &str) -> Result<Self, io::Error> {
        let alias: Vec<u16> = ifname.encode_utf16().chain(once(0)).collect();
        let mut luid = NET_LUID_LH::default();
        let err = unsafe { ConvertInterfaceAliasToLuid(alias.as_ptr(), &mut luid) };
        if err != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(err as i32));
        }
        let mut luid = unsafe { luid.Value };

        let session = FWPM_SESSION0 {
            flags: FWPM_SESSION_FLAG_DYNAMIC,
            ..Default::default()
        };
        let mut engine = ptr::null_mut();
        let err = unsafe {
            FwpmEngineOpen0(
                ptr::null(),
                RPC_C_AUTHN_WINNT,
                ptr::null(),
                &session,
                &mut engine,
            )
        };
        if err != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(err as i32));
        }
        // Bound before the filter is added, so an early return drops it and closes the session.
        let block = Self(engine);

        let mut condition = FWPM_FILTER_CONDITION0 {
            fieldKey: FWPM_CONDITION_IP_LOCAL_INTERFACE,
            matchType: FWP_MATCH_EQUAL,
            conditionValue: FWP_CONDITION_VALUE0 {
                r#type: FWP_UINT64,
                Anonymous: FWP_CONDITION_VALUE0_0 { uint64: &mut luid },
            },
        };
        let mut name: Vec<u16> = "Defguard IPv6 block"
            .encode_utf16()
            .chain(once(0))
            .collect();
        // A hard block, so that permit filters in other sublayers cannot override it.
        let filter = FWPM_FILTER0 {
            displayData: FWPM_DISPLAY_DATA0 {
                name: name.as_mut_ptr(),
                description: ptr::null_mut(),
            },
            flags: FWPM_FILTER_FLAG_CLEAR_ACTION_RIGHT,
            layerKey: FWPM_LAYER_ALE_AUTH_CONNECT_V6,
            numFilterConditions: 1,
            filterCondition: &mut condition,
            action: FWPM_ACTION0 {
                r#type: FWP_ACTION_BLOCK,
                ..Default::default()
            },
            ..Default::default()
        };
        let err = unsafe { FwpmFilterAdd0(engine, &filter, ptr::null_mut(), ptr::null_mut()) };
        if err != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(err as i32));
        }

        Ok(block)
    }
}

impl Drop for Ipv6Block {
    fn drop(&mut self) {
        unsafe { FwpmEngineClose0(self.0) };
    }
}
