use async_stream::stream;
use futures_core::stream::Stream;
use winapi::um::{minwinbase::SECURITY_ATTRIBUTES, namedpipeapi::CreateNamedPipeW, winbase::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX}, winnt::LPCWSTR};
use std::{os::windows::io::AsRawHandle, pin::Pin};
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

use windows::{core::{Param, PCWSTR}, Win32::{Foundation::INVALID_HANDLE_VALUE, Security::{
    InitializeSecurityDescriptor, SetSecurityDescriptorDacl, ACL, PSECURITY_DESCRIPTOR, SECURITY_DESCRIPTOR
}}};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1
};
// use windows::Win32::System::Pipes::CreateNamedPipeW;
use windows::Win32::System::Pipes::{
    // PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE,
    PIPE_TYPE_BYTE,
    PIPE_READMODE_BYTE, PIPE_WAIT, PIPE_UNLIMITED_INSTANCES
};
// use windows::Win32::System::IO::FILE_FLAG_FIRST_PIPE_INSTANCE;
use windows::core::PWSTR;

static PIPE_NAME_W: &str = r"\\.\pipe\defguard_daemon\0";

/// Converts an str to wide (u16), null-terminated
fn str_to_wide_null_terminated(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn create_secure_pipe() -> winapi::um::winnt::HANDLE {
    // ✅ SDDL: Grant RW access to group "defguard", no access to others
    // - O: = Owner (not set -> default)
    // - G: = Group (set to "defguard")
    // - D: = DACL:
    //   - (A;;0x12019f;;;SY)      → SYSTEM full access
    //   - (A;;0x12019f;;;BA)      → Builtin administrators full access
    //   - (A;;0x12019f;;;defguard) → "defguard" group RW access
    let security_descriptor= PCWSTR::from_raw(
        str_to_wide_null_terminated("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;defguard)").as_mut_ptr()
    );

    let mut sd_ptr = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            security_descriptor,
            SDDL_REVISION_1,
            &mut sd_ptr,
            // std::ptr::null_mut(),
            None,
        )
        .expect("Failed to create security descriptor");
        let pipe_name = LPCWSTR::from(str_to_wide_null_terminated(PIPE_NAME_W).as_ptr());
        let handle: winapi::um::winnt::HANDLE = CreateNamedPipeW(
            pipe_name,
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE.0 | PIPE_READMODE_BYTE.0 | PIPE_WAIT.0,
            PIPE_UNLIMITED_INSTANCES,
            65536,
            65536,
            0,
            sd_ptr.0 as *mut SECURITY_ATTRIBUTES,
        );

        // TODO: handle invalid handle
        // if handle.is_invalid() {
        //     panic!("Failed to create secure named pipe. Access denied / invalid group?");
        // }

        // if handle == INVALID_HANDLE_VALUE.0 || handle.is_null() {
        //     panic!("Failed to create secure named pipe. Access denied / invalid group?");
        // }

        handle
    }
}

use std::os::windows::io::{FromRawHandle, OwnedHandle};

fn create_tokio_secure_pipe() -> NamedPipeServer {
    let raw = create_secure_pipe();
    let owned = unsafe { OwnedHandle::from_raw_handle(raw as _) };
    unsafe {NamedPipeServer::from_raw_handle(owned.as_raw_handle()).unwrap()}
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