//! Windows self-update.
//!
//! The MSI installs per-machine, so installing needs admin rights. The service runs as LocalSystem
//! and installs updates itself, so users without admin rights still get them without a UAC prompt.
//! Any local user can reach the named pipe, so the service decides on its own what to install:
//! callers can't pass a version, URL or path.

use std::{
    collections::HashMap,
    ffi::c_void,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    mem::{size_of, zeroed},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    ptr::{null, null_mut},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use defguard_client_common::{PROGRAM_DATA_DIR, UPDATE_CHECK_PRODUCT, UPDATE_CHECK_URL, VERSION};
use known_folders::{get_known_folder_path, KnownFolder};
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use tonic::Status;
use tracing::{debug, error, info, warn};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_SUCCESS, HANDLE},
    Security::{
        Cryptography::{CertNameToStrW, CERT_X500_NAME_STR, X509_ASN_ENCODING},
        WinTrust::{
            WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData, WinVerifyTrust,
            WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
            WINTRUST_DATA_REVOCATION_CHECKS, WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE,
            WTD_REVOKE_WHOLECHAIN, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
        },
    },
    Storage::FileSystem::{CreateDirectoryW, FILE_SHARE_READ},
    System::{
        Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
        Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD},
        RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken},
        Threading::{
            CreateProcessAsUserW, CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT,
            DETACHED_PROCESS, PROCESS_INFORMATION, STARTUPINFOW,
        },
    },
};

use crate::win_util::{to_wide, SecurityDescriptor};

const DOWNLOAD_URL_BASE: &str = "https://github.com/DefGuard/client/releases/download";
const MAX_PACKAGE_SIZE: u64 = 512 * 1024 * 1024;
const POLICY_KEY: &str = r"SOFTWARE\Policies\Defguard";
const POLICY_DISABLE_VALUE: &str = "DisableAutoUpdate";
const UPDATES_SUBDIR: &str = "updates";
const MARKER_FILE: &str = "pending.json";
const INSTALL_LOG_FILE: &str = "install.log";
const CLIENT_EXE: &str = "defguard-client.exe";
/// Owned by SYSTEM, accessible to SYSTEM and Administrators only, nothing inherited.
const UPDATES_DIR_SDDL: &str = "O:SYG:SYD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";
/// Gives the installer time to finish and release the package after the new service starts.
const POST_UPDATE_DELAY: Duration = Duration::from_secs(15);

static UPDATE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("automatic updates are disabled by policy")]
    DisabledByPolicy,
    #[error("an update is already in progress")]
    InProgress,
    #[error("already running the latest version ({0})")]
    UpToDate(String),
    #[error("no installer is published for architecture {0}")]
    UnsupportedArch(&'static str),
    #[error("update check failed: {0}")]
    Check(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("installer signature rejected: {0}")]
    Signature(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

impl From<UpdateError> for Status {
    fn from(err: UpdateError) -> Self {
        let msg = err.to_string();
        match err {
            UpdateError::DisabledByPolicy
            | UpdateError::InProgress
            | UpdateError::UpToDate(_)
            | UpdateError::UnsupportedArch(_) => Status::failed_precondition(msg),
            UpdateError::Check(_) | UpdateError::Download(_) => Status::unavailable(msg),
            UpdateError::Signature(_) => Status::permission_denied(msg),
            UpdateError::Io(_) => Status::internal(msg),
        }
    }
}

#[derive(Deserialize)]
struct LatestVersion {
    version: String,
}

/// Left behind for the service started by the installer, so it can report and clean up.
#[derive(Serialize, Deserialize)]
struct PendingUpdate {
    version: String,
}

/// Downloads, verifies and launches the installer for the latest release.
///
/// Returns once the installer is running. It stops and restarts this service along the way.
pub async fn start_update() -> Result<String, UpdateError> {
    if disabled_by_policy() {
        return Err(UpdateError::DisabledByPolicy);
    }
    if UPDATE_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return Err(UpdateError::InProgress);
    }
    let result = prepare_and_launch().await;
    // On success the installer watcher clears the flag once msiexec exits.
    if let Err(err) = &result {
        error!("Update failed: {err}");
        UPDATE_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
    result
}

async fn prepare_and_launch() -> Result<String, UpdateError> {
    let arch = msi_arch()?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_secs(60))
        .build()
        .map_err(|err| UpdateError::Check(err.to_string()))?;
    let latest = latest_version(&client).await?;
    let current = Version::parse(VERSION)
        .map_err(|err| UpdateError::Check(format!("invalid current version {VERSION}: {err}")))?;
    if latest <= current {
        return Err(UpdateError::UpToDate(current.to_string()));
    }
    let version = latest.to_string();
    info!("Updating from {current} to {version}");

    let dir = updates_dir()?;
    recreate_updates_dir(&dir)?;

    let msi_name = format!("Defguard_{version}_{arch}_en-US.msi");
    let url = format!("{DOWNLOAD_URL_BASE}/v{version}/{msi_name}");
    let msi_path = dir.join(&msi_name);
    let file = download(&client, &url, &msi_path).await?;

    // Revocation checks block, so keep them off the async workers.
    let verify_path = msi_path.clone();
    let file = tokio::task::spawn_blocking(move || verify_publisher(file, &verify_path))
        .await
        .map_err(|err| UpdateError::Signature(err.to_string()))??;
    info!("Installer {msi_name} is signed by the same publisher as this service");

    let marker = serde_json::to_vec(&PendingUpdate {
        version: version.clone(),
    })
    .map_err(io::Error::other)?;
    fs::write(dir.join(MARKER_FILE), marker)?;

    launch_installer(file, &msi_path, &dir.join(INSTALL_LOG_FILE))?;
    Ok(version)
}

/// Reports the outcome of an update started by the previous service and relaunches the client.
///
/// Called on service start. The marker only exists if this service started the installer.
pub fn finish_pending_update() {
    report_and_relaunch(POST_UPDATE_DELAY);
}

fn report_and_relaunch(delay: Duration) {
    let Ok(dir) = updates_dir() else {
        return;
    };
    let marker = dir.join(MARKER_FILE);
    let Ok(contents) = fs::read(&marker) else {
        return;
    };
    if let Err(err) = fs::remove_file(&marker) {
        // Without this the client would be relaunched on every service start.
        error!("Failed to remove update marker {}: {err}", marker.display());
        return;
    }
    match serde_json::from_slice::<PendingUpdate>(&contents) {
        Ok(pending) if pending.version == VERSION => info!("Updated to version {VERSION}"),
        Ok(pending) => warn!(
            "Update to {} did not complete, still running {VERSION}. Installer log: {}",
            pending.version,
            dir.join(INSTALL_LOG_FILE).display()
        ),
        Err(err) => warn!("Ignoring unreadable update marker: {err}"),
    }

    // The client quit to let the installer replace it, so bring it back either way. The MSI's own
    // launch action can't do this: msiexec runs as SYSTEM and can't reach the user's session.
    thread::spawn(move || {
        thread::sleep(delay);
        relaunch_client();
        remove_packages(&dir);
    });
}

fn msi_arch() -> Result<&'static str, UpdateError> {
    match std::env::consts::ARCH {
        "x86_64" => Ok("x64"),
        "aarch64" => Ok("arm64"),
        other => Err(UpdateError::UnsupportedArch(other)),
    }
}

async fn latest_version(client: &Client) -> Result<Version, UpdateError> {
    let request = HashMap::from([
        ("product", UPDATE_CHECK_PRODUCT),
        ("client_version", VERSION),
        ("operating_system", std::env::consts::OS),
    ]);
    let check = |err: reqwest::Error| UpdateError::Check(err.to_string());
    let response = client
        .post(UPDATE_CHECK_URL)
        .timeout(Duration::from_secs(30))
        .json(&request)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(check)?
        .json::<LatestVersion>()
        .await
        .map_err(check)?;

    parse_release_version(&response.version)
}

/// The version ends up in a URL and a file name, so only accept a plain release version.
fn parse_release_version(value: &str) -> Result<Version, UpdateError> {
    let version = Version::parse(value)
        .map_err(|err| UpdateError::Check(format!("invalid version {value}: {err}")))?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(UpdateError::Check(format!(
            "refusing to install pre-release version {version}"
        )));
    }
    Ok(version)
}

/// Downloads the installer and returns a read-only handle that blocks writes by others.
/// Keep it open until msiexec exits so the file can't be swapped after verification.
async fn download(client: &Client, url: &str, path: &Path) -> Result<File, UpdateError> {
    debug!("Downloading installer from {url}");
    let fail = |err: reqwest::Error| UpdateError::Download(err.to_string());
    let too_large = || UpdateError::Download("installer is too large".into());
    let mut response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(fail)?;
    if response
        .content_length()
        .is_some_and(|len| len > MAX_PACKAGE_SIZE)
    {
        return Err(too_large());
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)?;
    let mut total = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(fail)? {
        total += chunk.len() as u64;
        if total > MAX_PACKAGE_SIZE {
            return Err(too_large());
        }
        file.write_all(&chunk)?;
    }
    debug!("Downloaded {total} bytes to {}", path.display());

    // Only SYSTEM and Administrators can write here between close and reopen.
    let path = path.to_owned();
    let file = tokio::task::spawn_blocking(move || {
        file.sync_all()?;
        drop(file);
        OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&path)
    })
    .await
    .map_err(|err| UpdateError::Download(err.to_string()))??;
    Ok(file)
}

/// Accepts the installer only if its Authenticode signer matches this service's.
///
/// Matching the subject instead of pinning a certificate keeps updates working across certificate
/// renewals. Unsigned (development) builds can't update themselves.
fn verify_publisher(file: File, path: &Path) -> Result<File, UpdateError> {
    // Only the subject is needed from our own signature, so skip its revocation round trip.
    let service_exe = std::env::current_exe()?;
    let expected = signer_subject(&service_exe, None, WTD_REVOKE_NONE)
        .map_err(|err| UpdateError::Signature(format!("this service is not signed ({err})")))?;
    let actual = signer_subject(path, Some(file.as_raw_handle()), WTD_REVOKE_WHOLECHAIN)?;
    if actual != expected {
        return Err(UpdateError::Signature(format!(
            "signed by \"{actual}\", expected \"{expected}\""
        )));
    }
    Ok(file)
}

fn signer_subject(
    path: &Path,
    handle: Option<HANDLE>,
    revocation: WINTRUST_DATA_REVOCATION_CHECKS,
) -> Result<String, UpdateError> {
    let wide_path = to_wide(path);
    // SAFETY: all pointers passed to WinTrust point to locals that outlive the calls, and the
    // verification state is closed before returning.
    unsafe {
        let mut file_info: WINTRUST_FILE_INFO = zeroed();
        file_info.cbStruct = size_of::<WINTRUST_FILE_INFO>() as u32;
        file_info.pcwszFilePath = wide_path.as_ptr();
        file_info.hFile = handle.unwrap_or_default();

        let mut data: WINTRUST_DATA = zeroed();
        data.cbStruct = size_of::<WINTRUST_DATA>() as u32;
        data.dwUIChoice = WTD_UI_NONE;
        data.fdwRevocationChecks = revocation;
        data.dwUnionChoice = WTD_CHOICE_FILE;
        data.Anonymous = WINTRUST_DATA_0 {
            pFile: &mut file_info,
        };
        data.dwStateAction = WTD_STATEACTION_VERIFY;

        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let status = WinVerifyTrust(null_mut(), &mut action, (&raw mut data).cast::<c_void>());
        let result = if status == 0 {
            subject_from_state(data.hWVTStateData)
        } else {
            Err(UpdateError::Signature(format!(
                "{} failed verification (0x{:08x})",
                path.display(),
                status as u32
            )))
        };

        data.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(null_mut(), &mut action, (&raw mut data).cast::<c_void>());
        result
    }
}

/// # Safety
/// `state` must come from a successful `WinVerifyTrust` call that hasn't been closed yet.
unsafe fn subject_from_state(state: HANDLE) -> Result<String, UpdateError> {
    let missing = || UpdateError::Signature("signer certificate not found".into());
    let provider = WTHelperProvDataFromStateData(state);
    if provider.is_null() {
        return Err(missing());
    }
    let signer = WTHelperGetProvSignerFromChain(provider, 0, 0, 0);
    if signer.is_null() || (*signer).csCertChain == 0 || (*signer).pasCertChain.is_null() {
        return Err(missing());
    }
    let cert = (*(*signer).pasCertChain).pCert;
    if cert.is_null() || (*cert).pCertInfo.is_null() {
        return Err(missing());
    }
    let subject = &raw const (*(*cert).pCertInfo).Subject;
    let len = CertNameToStrW(
        X509_ASN_ENCODING,
        subject,
        CERT_X500_NAME_STR,
        null_mut(),
        0,
    );
    if len <= 1 {
        return Err(missing());
    }
    let mut buf = vec![0u16; len as usize];
    CertNameToStrW(
        X509_ASN_ENCODING,
        subject,
        CERT_X500_NAME_STR,
        buf.as_mut_ptr(),
        len,
    );
    Ok(String::from_utf16_lossy(&buf[..len as usize - 1]))
}

fn launch_installer(package: File, msi_path: &Path, log_path: &Path) -> Result<(), UpdateError> {
    let system = get_known_folder_path(KnownFolder::System)
        .ok_or_else(|| io::Error::other("System32 folder not found"))?;
    info!(
        "Starting installer {}, log: {}",
        msi_path.display(),
        log_path.display()
    );
    // Detached so it keeps running after the installer stops this service.
    let mut child = Command::new(system.join("msiexec.exe"))
        .arg("/i")
        .arg(msi_path)
        .args(["/qn", "/norestart", "/l*v"])
        .arg(log_path)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()?;

    // Only reached if the installer fails before it stops this service.
    thread::spawn(move || {
        match child.wait() {
            Ok(status) => {
                warn!("Installer exited with {status} while the service was still running")
            }
            Err(err) => error!("Failed to wait for the installer: {err}"),
        }
        drop(package);
        UPDATE_IN_PROGRESS.store(false, Ordering::SeqCst);
        // The client already quit, so report the failure and bring it back now.
        report_and_relaunch(Duration::ZERO);
    });
    Ok(())
}

fn disabled_by_policy() -> bool {
    let key = to_wide(POLICY_KEY);
    let value = to_wide(POLICY_DISABLE_VALUE);
    let mut data = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `data` and `size` describe a valid u32 buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&raw mut data).cast::<c_void>(),
            &mut size,
        )
    };
    status == ERROR_SUCCESS && data != 0
}

fn updates_dir() -> Result<PathBuf, UpdateError> {
    let mut path = get_known_folder_path(KnownFolder::ProgramData)
        .ok_or_else(|| io::Error::other("ProgramData folder not found"))?;
    path.push(PROGRAM_DATA_DIR);
    path.push(UPDATES_SUBDIR);
    Ok(path)
}

/// Starts from an empty directory, created with its restrictive ACL in one step.
///
/// Users can create folders in ProgramData, so an existing directory can't be trusted.
fn recreate_updates_dir(dir: &Path) -> Result<(), UpdateError> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    if let Some(parent) = dir.parent() {
        fs::create_dir_all(parent)?;
    }

    let descriptor = SecurityDescriptor::from_sddl(UPDATES_DIR_SDDL)?;
    let attributes = descriptor.attributes();
    let wide_dir = to_wide(dir);
    // SAFETY: both pointers are valid for the duration of the call.
    // Fails if someone recreated the directory in the meantime.
    if unsafe { CreateDirectoryW(wide_dir.as_ptr(), &attributes) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(())
}

/// Removes downloaded packages but keeps the installer log for troubleshooting.
fn remove_packages(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("msi"))
        {
            if let Err(err) = fs::remove_file(&path) {
                warn!("Failed to remove installer {}: {err}", path.display());
            }
        }
    }
}

/// Starts the client in the console user's session, since the service runs in session 0.
fn relaunch_client() {
    let exe = match std::env::current_exe() {
        Ok(path) => path.with_file_name(CLIENT_EXE),
        Err(err) => {
            warn!("Not relaunching the client: {err}");
            return;
        }
    };
    let Some(workdir) = exe.parent() else {
        return;
    };
    let application = to_wide(&exe);
    // CreateProcessAsUserW may write to the command line buffer.
    let mut command_line = to_wide(format!("\"{}\"", exe.display()));
    let mut desktop = to_wide(r"winsta0\default");
    let workdir = to_wide(workdir);

    // SAFETY: handles and the environment block are released on every path, and all strings are
    // NUL-terminated buffers that outlive the calls.
    unsafe {
        let session = WTSGetActiveConsoleSessionId();
        if session == u32::MAX {
            info!("No active console session, not relaunching the client");
            return;
        }
        let mut token: HANDLE = null_mut();
        if WTSQueryUserToken(session, &mut token) == 0 {
            info!(
                "No user logged on to the console, not relaunching the client (error {})",
                GetLastError()
            );
            return;
        }
        // Stays null on failure, which makes the process inherit this service's environment.
        let mut environment: *mut c_void = null_mut();
        CreateEnvironmentBlock(&mut environment, token, 0);

        let mut startup: STARTUPINFOW = zeroed();
        startup.cb = size_of::<STARTUPINFOW>() as u32;
        startup.lpDesktop = desktop.as_mut_ptr();
        let mut process: PROCESS_INFORMATION = zeroed();

        let created = CreateProcessAsUserW(
            token,
            application.as_ptr(),
            command_line.as_mut_ptr(),
            null(),
            null(),
            0,
            CREATE_UNICODE_ENVIRONMENT,
            environment,
            workdir.as_ptr(),
            &startup,
            &mut process,
        );
        if created == 0 {
            warn!("Failed to relaunch the client (error {})", GetLastError());
        } else {
            info!("Relaunched the client in session {session}");
            CloseHandle(process.hProcess);
            CloseHandle(process.hThread);
        }
        if !environment.is_null() {
            DestroyEnvironmentBlock(environment);
        }
        CloseHandle(token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_release_versions() {
        assert_eq!(
            parse_release_version("2.4.0").unwrap(),
            Version::new(2, 4, 0)
        );
    }

    #[test]
    fn rejects_versions_unsafe_for_urls_and_paths() {
        for value in [
            "2.4.0-rc1",
            "2.4.0+build",
            "2.4.0/../../x",
            "v2.4.0",
            "",
            "2.4",
        ] {
            assert!(parse_release_version(value).is_err(), "accepted {value:?}");
        }
    }
}
