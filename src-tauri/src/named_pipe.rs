use async_stream::stream;
use futures_core::stream::Stream;
use windows_sys::Win32::{
    Foundation::{
        GetLastError, LocalFree, HANDLE, INVALID_HANDLE_VALUE,
    },
    Security::{
        Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SDDL_REVISION_1,
        },
        PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
    },
    System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE},
};
use std::{
    os::windows::io::RawHandle,
    pin::Pin,
};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::windows::named_pipe::NamedPipeServer,
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

/// Converts an str to wide (u16), null-terminated
fn str_to_wide_null_terminated(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn create_secure_pipe() -> Result<HANDLE, std::io::Error> {
    unsafe {
        // Compose SDDL: SYSTEM & Administrators full, users RW.
        let sddl = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;BU)";
        let sddl_wide = str_to_wide_null_terminated(sddl);

        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_wide.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor as *mut PSECURITY_DESCRIPTOR,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }

        // Build SECURITY_ATTRIBUTES pointing to the security descriptor
        let attributes = SECURITY_ATTRIBUTES {
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
            &attributes,
        );

        // Free the security descriptor memory returned by ConvertStringSecurityDescriptorToSecurityDescriptorW
        LocalFree(descriptor as *mut _);

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        Ok(handle)
    }
}

fn create_tokio_secure_pipe() -> NamedPipeServer {
    let raw = create_secure_pipe().unwrap();
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
    stream! {
        let mut server = create_tokio_secure_pipe();

        loop {
            server.connect().await?;
            yield Ok(TonicNamedPipeServer::new(server));
            server = create_tokio_secure_pipe();
        }
    }
}
