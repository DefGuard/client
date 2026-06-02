use std::{
    os::windows::io::RawHandle,
    pin::Pin,
    task::{Context, Poll},
};

use async_stream::stream;
use futures_core::stream::Stream;
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::windows::named_pipe::NamedPipeServer,
};
use tonic::transport::server::Connected;
use windows_sys::Win32::{
    Foundation::{LocalFree, HANDLE, INVALID_HANDLE_VALUE},
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX},
    System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE},
};

// Named-pipe name used for IPC between defguard client and windows service.
pub(super) static PIPE_NAME: &str = r"\\.\pipe\defguard_daemon";

/// SDDL defining named pipe ACL:
/// - `SY` (LocalSystem) - full control
/// - `BA` (Administrators) - full control
/// - `BU` (Built-in Users) - read/write
static SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;BU)";

/// Tonic-compatible wrapper around a Windows named pipe server handle.
pub(crate) struct TonicNamedPipeServer {
    inner: NamedPipeServer,
}

impl TonicNamedPipeServer {
    pub fn new(inner: NamedPipeServer) -> Self {
        Self { inner }
    }
}

impl Connected for TonicNamedPipeServer {
    type ConnectInfo = ();

    fn connect_info(&self) -> Self::ConnectInfo {}
}

impl AsyncRead for TonicNamedPipeServer {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for TonicNamedPipeServer {
    /// Delegate async write to the underlying pipe.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    /// Delegate flush to the underlying pipe.
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    /// Delegate shutdown to the underlying pipe.
    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Convert `&str` to a null-terminated UTF-16 buffer suitable for Win32 APIs.
fn str_to_wide_null_terminated(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Create a secure Windows named pipe handle with appropriate ACL.
/// Uses `FILE_FLAG_OVERLAPPED` for Tokio compatibility and sets `nMaxInstances = 2`
/// (one client + one service instance).
fn create_secure_pipe() -> Result<HANDLE, std::io::Error> {
    debug!("Creating secure named pipe {PIPE_NAME}");

    // Compose SDDL: SYSTEM & Administrators full access, users read-write.
    let sddl_wide = str_to_wide_null_terminated(SDDL);

    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

    let result = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_wide.as_ptr(),
            SDDL_REVISION_1,
            (&raw mut descriptor).cast::<PSECURITY_DESCRIPTOR>(),
            std::ptr::null_mut(),
        )
    };
    if result == 0 {
        error!("Error calling ConvertStringSecurityDescriptorToSecurityDescriptorW");
        return Err(std::io::Error::last_os_error());
    }

    // Build SECURITY_ATTRIBUTES pointing to the security descriptor
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.cast(),
        bInheritHandle: 0,
    };

    let name_wide = str_to_wide_null_terminated(PIPE_NAME);

    let handle = unsafe {
        CreateNamedPipeW(
            name_wide.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_BYTE,
            // 1 client + 1 service
            2,
            65536,
            65536,
            0,
            &raw const attributes,
        )
    };
    unsafe {
        // Free memory allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
        LocalFree(descriptor);
    }

    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        error!("CreateNamedPipeW returned invalid handle: {handle:?}");
        return Err(std::io::Error::last_os_error());
    }

    info!("Created secure named pipe {PIPE_NAME}");
    Ok(handle)
}

/// Wrap a raw pipe `HANDLE` into a Tokio `NamedPipeServer`.
fn create_tokio_secure_pipe() -> Result<NamedPipeServer, std::io::Error> {
    debug!("Creating tokio secure pipe");
    let raw = create_secure_pipe()?;
    let pipe = unsafe { NamedPipeServer::from_raw_handle(raw as RawHandle)? };

    info!("Created tokio secure pipe");
    Ok(pipe)
}

/// Produce a `Stream` of connected pipe servers for `tonic::transport::Server::serve_with_incoming`.
///
/// Each loop:
/// 1. Creates a fresh listening instance.
/// 2. Awaits a client connection (`connect().await`).
/// 3. Yields the connected `TonicNamedPipeServer`.
pub(crate) fn get_named_pipe_server_stream() -> impl Stream<Item = io::Result<TonicNamedPipeServer>>
{
    debug!("Creating named pipe server stream");
    let stream = stream! {
        let mut server;
        loop {
            server = create_tokio_secure_pipe()?;
            server.connect().await?;
            yield Ok(TonicNamedPipeServer::new(server));
        }
    };
    info!("Created named pipe server stream");
    stream
}
