#![cfg(target_os = "linux")]

use std::{env, fs, os::unix::fs::PermissionsExt, process::Command};

use device_identity::{capabilities, DeviceIdentityError};

fn assert_capabilities_error(test_name: &str, transport: &str, expected: DeviceIdentityError) {
    if env::var_os("DEFGUARD_TRANSPORT_TEST_CHILD").is_some() {
        assert_eq!(capabilities(), Err(expected));
        return;
    }

    let output = Command::new(env::current_exe().expect("locate test executable"))
        .args(["--exact", test_name, "--include-ignored", "--nocapture"])
        .env("DEFGUARD_TRANSPORT_TEST_CHILD", "1")
        .env("TSS2_TCTI", transport)
        .output()
        .expect("run transport test in an isolated process");
    assert!(
        output.status.success(),
        "transport test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn test_capabilities_reports_missing_device() {
    let directory = tempfile::tempdir().expect("create isolated device path");
    let transport = format!("device:{}", directory.path().join("missing-tpm").display());
    assert_capabilities_error(
        "test_capabilities_reports_missing_device",
        &transport,
        DeviceIdentityError::Unavailable,
    );
}

#[test]
fn test_capabilities_reports_unavailable_emulator() {
    let directory = tempfile::tempdir().expect("create isolated socket path");
    let transport = format!(
        "swtpm:path={}",
        directory.path().join("missing.sock").display()
    );
    assert_capabilities_error(
        "test_capabilities_reports_unavailable_emulator",
        &transport,
        DeviceIdentityError::Unavailable,
    );
}

#[test]
fn test_capabilities_rejects_empty_transport_without_fallback() {
    assert_capabilities_error(
        "test_capabilities_rejects_empty_transport_without_fallback",
        "",
        DeviceIdentityError::BackendFailure,
    );
}

#[test]
fn test_capabilities_rejects_unknown_transport_without_fallback() {
    assert_capabilities_error(
        "test_capabilities_rejects_unknown_transport_without_fallback",
        "defguard-nonexistent-tcti",
        DeviceIdentityError::BackendFailure,
    );
}

#[test]
#[ignore = "Requires an unprivileged Linux process"]
fn test_capabilities_reports_permission_denied() {
    let file = tempfile::NamedTempFile::new().expect("create inaccessible device fixture");
    fs::set_permissions(file.path(), fs::Permissions::from_mode(0o000))
        .expect("remove fixture access permissions");
    let transport = format!("device:{}", file.path().display());
    assert_capabilities_error(
        "test_capabilities_reports_permission_denied",
        &transport,
        DeviceIdentityError::PermissionDenied,
    );
}
