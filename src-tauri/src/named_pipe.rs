use async_stream::stream;
use futures_core::stream::Stream;
use windows_sys::Win32::{
    Foundation::{
        GetLastError, LocalFree, ERROR_INSUFFICIENT_BUFFER, HANDLE, INVALID_HANDLE_VALUE,
    },
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SDDL_REVISION_1,
        },
        LookupAccountNameW, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
    },
    System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE, PIPE_WAIT},
};
// use winapi::{shared::{sddl::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1}, winerror::ERROR_INSUFFICIENT_BUFFER},
//     um::{
//         errhandlingapi::GetLastError, handleapi::INVALID_HANDLE_VALUE, minwinbase::SECURITY_ATTRIBUTES, namedpipeapi::CreateNamedPipeW, winbase::{LocalFree, LookupAccountNameW, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT}, winnt::{LPCWSTR, PCWSTR, PSECURITY_DESCRIPTOR, PSID}
//     }};
use std::{
    os::windows::io::{AsRawHandle, RawHandle},
    pin::Pin,
};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
};
use tonic::transport::server::Connected;

pub static PIPE_NAME: &str = r"\\.\pipe\defguard_daemon";

pub struct TonicNamedPipeServer {
    inner: NamedPipeServer,
}

impl TonicNamedPipeServer {
    pub fn new(inner: NamedPipeServer) -> Self {
        Self { inner }
    }
}

impl Connected for TonicNamedPipeServer {
    type ConnectInfo = ();

    fn connect_info(&self) -> Self::ConnectInfo {
        ()
    }
}

impl AsyncRead for TonicNamedPipeServer {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for TonicNamedPipeServer {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

// use windows::{core::{Param, PCWSTR}, Win32::{Foundation::INVALID_HANDLE_VALUE, Security::{
//     InitializeSecurityDescriptor, SetSecurityDescriptorDacl, ACL, PSECURITY_DESCRIPTOR, SECURITY_DESCRIPTOR
// }}};
// use windows::Win32::Security::Authorization::{
//     ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1
// };
// // use windows::Win32::System::Pipes::CreateNamedPipeW;
// use windows::Win32::System::Pipes::{
//     // PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE,
//     PIPE_TYPE_BYTE,
//     PIPE_READMODE_BYTE, PIPE_WAIT, PIPE_UNLIMITED_INSTANCES
// };
// // use windows::Win32::System::IO::FILE_FLAG_FIRST_PIPE_INSTANCE;
// use windows::core::PWSTR;

static PIPE_NAME_W: &str = r"\\.\pipe\defguard_daemon\0";

/// Converts an str to wide (u16), null-terminated
fn str_to_wide_null_terminated(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

// fn create_secure_pipe() -> winapi::um::winnt::HANDLE {
//     // ✅ SDDL: Grant RW access to group "defguard", no access to others
//     // - O: = Owner (not set -> default)
//     // - G: = Group (set to "defguard")
//     // - D: = DACL:
//     //   - (A;;0x12019f;;;SY)      → SYSTEM full access
//     //   - (A;;0x12019f;;;BA)      → Builtin administrators full access
//     //   - (A;;0x12019f;;;defguard) → "defguard" group RW access
//     let security_descriptor= PCWSTR::from(
//         str_to_wide_null_terminated("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;defguard)").as_mut_ptr()
//     );

//     // let mut sd_ptr = PSECURITY_DESCRIPTOR::default();
//     let mut sd_ptr = PSECURITY_DESCRIPTOR::default();
//     let mut sds_ptr = 0u32;
//     unsafe {
//         ConvertStringSecurityDescriptorToSecurityDescriptorW(
//             security_descriptor,
//             SDDL_REVISION_1 as u32,
//             &mut sd_ptr,
//             // std::ptr::null_mut(),
//             &mut sds_ptr,
//         );
//         // .expect("Failed to create security descriptor");
//         let pipe_name = LPCWSTR::from(str_to_wide_null_terminated(PIPE_NAME_W).as_ptr());
//         let handle: winapi::um::winnt::HANDLE = CreateNamedPipeW(
//             pipe_name,
//             PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
//             PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
//             PIPE_UNLIMITED_INSTANCES,
//             65536,
//             65536,
//             0,
//             // sd_ptr.0 as *mut SECURITY_ATTRIBUTES,
//             sd_ptr as *mut SECURITY_ATTRIBUTES,
//         );

//         // TODO: handle invalid handle
//         // if handle.is_invalid() {
//         //     panic!("Failed to create secure named pipe. Access denied / invalid group?");
//         // }

//         if handle == INVALID_HANDLE_VALUE || handle.is_null() {
//             panic!("Failed to create secure named pipe. Access denied / invalid group?");
//         }
//         info!("Handle: {:?}", handle);

//         handle
//     }
// }

// fn create_secure_pipe() -> winapi::um::winnt::HANDLE {
//     unsafe {
//         // SDDL string - allow SYSTEM, Administrators, and group "defguard"
//         // let sddl = to_wide("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;defguard)");
//         let sddl = str_to_wide_null_terminated("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;defguard)");

//         let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
//         if ConvertStringSecurityDescriptorToSecurityDescriptorW(
//             sddl.as_ptr(),
//             SDDL_REVISION_1 as u32,
//             &mut sd,
//             std::ptr::null_mut(),
//         ) == 0 {
//             panic!("Failed to convert SDDL: {}", std::io::Error::last_os_error());
//         }

//         // Build SECURITY_ATTRIBUTES properly
//         let mut sa = SECURITY_ATTRIBUTES {
//             nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
//             lpSecurityDescriptor: sd as *mut _,
//             bInheritHandle: 0,
//         };

//         // let name = to_wide(r"\\.\pipe\defguard_daemon");
//         let name = str_to_wide_null_terminated(r"\\.\pipe\defguard_daemon");

//         let handle = CreateNamedPipeW(
//             name.as_ptr(),
//             PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
//             PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
//             PIPE_UNLIMITED_INSTANCES,
//             65536,
//             65536,
//             0,
//             &mut sa,
//         );

//         if handle == INVALID_HANDLE_VALUE || handle.is_null() {
//             panic!(
//                 "CreateNamedPipeW failed: {}",
//                 std::io::Error::last_os_error()
//             );
//         }

//         handle
//     }
// }

// fn to_wide(s: &str) -> Vec<u16> {
//     std::ffi::OsStr::new(s)
//         .encode_wide()
//         .chain(std::iter::once(0))
//         .collect()
// }

/// Resolve account name (e.g. "defguard") -> SID string like "S-1-5-21-...".
fn account_name_to_sid_string(account: &str) -> Result<String, std::io::Error> {
    unsafe {
        let name_wide = str_to_wide_null_terminated(account);
        // First call to get buffer sizes
        let mut sid_size: u32 = 0;
        let mut domain_size: u32 = 0;
        let mut pe_use = 0i32;
        // let mut pe_use: *mut u32 = std::ptr::null_mut();

        let ok = LookupAccountNameW(
            std::ptr::null(), // local system
            name_wide.as_ptr(),
            std::ptr::null_mut(), // PSID buffer
            &mut sid_size,
            std::ptr::null_mut(), // domain buffer
            &mut domain_size,
            &mut pe_use,
        );

        if ok != 0 {
            // Shouldn't succeed with null buffers, but handle defensively
        } else {
            // TODO handle error != ERROR_INSUFFICIENT_BUFFER
            // panic!("ok == 0");
            let err = GetLastError();
            if err != ERROR_INSUFFICIENT_BUFFER {
                error!("Other error");
                return Err(std::io::Error::from_raw_os_error(err as i32));
            }
        }

        // allocate buffers
        let mut sid_buf: Vec<u8> = vec![0u8; sid_size as usize];
        let mut domain_buf: Vec<u16> = vec![0u16; domain_size as usize];

        let ok2 = LookupAccountNameW(
            std::ptr::null(),
            name_wide.as_ptr(),
            sid_buf.as_mut_ptr() as PSID,
            &mut sid_size,
            domain_buf.as_mut_ptr(),
            &mut domain_size,
            &mut pe_use,
        );

        if ok2 == 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Convert SID to string
        let mut sid_string_ptr: *mut u16 = std::ptr::null_mut();
        if ConvertSidToStringSidW(
            sid_buf.as_mut_ptr() as PSID,
            &mut sid_string_ptr as *mut *mut u16,
        ) == 0
        {
            error!("ConvertSidToStringSidW");
            return Err(std::io::Error::last_os_error());
        }

        // sid_string_ptr is a LPWSTR allocated by LocalAlloc/LocalFree. Convert to Rust String.
        let mut len = 0usize;
        while *sid_string_ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(sid_string_ptr, len);
        let sid_string = String::from_utf16_lossy(slice);

        // free returned string
        LocalFree(sid_string_ptr as *mut _);

        Ok(sid_string)
    }
}

fn create_secure_pipe() -> Result<HANDLE, std::io::Error> {
    unsafe {
        // Compose SDDL: SYSTEM & Administrators full, users RW.
        let sddl = format!("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;BU)");
        let sddl_wide = str_to_wide_null_terminated(&sddl);

        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        warn!("DESCRIPTOR BEFORE: {descriptor:?}");

        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_wide.as_ptr(),
            SDDL_REVISION_1 as u32,
            &mut descriptor as *mut PSECURITY_DESCRIPTOR as *mut *mut _,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        warn!("DESCRIPTOR AFTER: {descriptor:?}");

        // Build SECURITY_ATTRIBUTES pointing to the security descriptor
        let mut attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor as *mut _,
            bInheritHandle: 0,
        };

        let name_wide = str_to_wide_null_terminated(r"\\.\pipe\defguard_daemon");

        let handle = CreateNamedPipeW(
            name_wide.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_BYTE,
            2,
            65536,
            65536,
            0,
            &mut attributes,
        );

        // Free the security descriptor memory returned by ConvertStringSecurityDescriptorToSecurityDescriptorW
        LocalFree(descriptor as *mut _);

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        Ok(handle)
    }
}

use std::os::windows::io::{FromRawHandle, OwnedHandle};

fn create_tokio_secure_pipe() -> NamedPipeServer {
    // let raw = create_secure_pipe();
    // let owned = unsafe { OwnedHandle::from_raw_handle(raw as _) };
    // unsafe {NamedPipeServer::from_raw_handle(owned.as_raw_handle()).unwrap()}
    let raw = create_secure_pipe().unwrap();
    info!("Raw handle: {raw:?}");
    unsafe {
        let result = NamedPipeServer::from_raw_handle(raw as RawHandle);
        match result {
            Ok(server) => server,
            Err(err) => {
                let error = GetLastError();
                error!("Windows error: {error:}");
                panic!("Other error: {err:?}");
            }
        }
    }
}

pub fn get_named_pipe_server_stream() -> impl Stream<Item = io::Result<TonicNamedPipeServer>> {
    // stream! {
    //     let mut server = ServerOptions::new()
    //         .first_pipe_instance(true)
    //         .create(PIPE_NAME)?;

    //     loop {
    //         server.connect().await?;

    //         let client = TonicNamedPipeServer::new(server);

    //         yield Ok(client);

    //         server = ServerOptions::new().create(PIPE_NAME)?;
    //     }
    // }
    stream! {
        let mut server = create_tokio_secure_pipe();

        loop {
            server.connect().await?;
            yield Ok(TonicNamedPipeServer::new(server));
            server = create_tokio_secure_pipe();
        }
    }
}
