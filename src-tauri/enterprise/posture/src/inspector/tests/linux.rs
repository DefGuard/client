use super::super::{disk_encryption_status, os_name, os_version, DiskEncryptionTarget};

#[test]
fn test_os_name() {
    assert!(os_name().is_ok());
}

#[test]
fn test_os_version() {
    assert!(os_version().is_ok());
}

#[test]
#[ignore = "development machine only"]
fn test_disk_encryption() {
    assert!(!disk_encryption_status(DiskEncryptionTarget::ClientDatabase).unwrap());
}

/// A path that does not exist yet still resolves: `canonicalize_on_disk` walks up to the nearest
/// existing ancestor, which is deliberate, since the client database may not have been created yet.
///
/// It is also why the probe must be told which path to report on. Handed a path under a data directory
/// that does not exist - which is what resolving the *user's* database as root produces - it does not
/// fail loudly, it quietly answers for an ancestor instead. The answer looks plausible and is about
/// the wrong filesystem.
#[test]
fn test_nonexistent_path_resolves_to_its_nearest_existing_ancestor() {
    let missing = std::path::Path::new("/nonexistent-defguard-posture-target/data/db.sqlite");
    assert_eq!(
        super::super::linux::disk_encryption_status(missing),
        super::super::linux::disk_encryption_status(std::path::Path::new("/")),
        "a path whose ancestors are all missing must report the same as `/`"
    );
}
