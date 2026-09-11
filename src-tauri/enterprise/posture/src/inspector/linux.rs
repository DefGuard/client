use std::{
    collections::HashSet,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
};

use super::UnavailableReason;

/// Path to the kernel's mount table for the current process.
const MOUNTINFO_PATH: &str = "/proc/self/mountinfo";

/// sysfs directory exposing every block device by kernel name.
const SYS_BLOCK: &str = "/sys/class/block";

/// A single mount table entry: its mount point, filesystem type and backing
/// device source.
struct MountEntry {
    mount_point: String,
    /// Filesystem type, e.g. `ext4`, `btrfs`, `zfs`.
    fstype: String,
    /// The mount source as reported by the kernel, e.g. `/dev/mapper/cryptroot`
    /// for a block device or a dataset name like `rpool/USERDATA/x` for ZFS.
    source: String,
}

/// Reports whether the partition that stores the client's database file is
/// encrypted.
///
/// It resolves the specific device backing the database file and inspects only
/// that device's stack, so unrelated encrypted loop/removable/test volumes do
/// not produce a false positive. Two encryption mechanisms are recognized:
/// - **LUKS/dm-crypt** for block-backed filesystems (ext4/xfs/btrfs/LVM/…), via
///   the sysfs device dependency chain (`/sys/class/block/<dev>/slaves`);
/// - **native ZFS encryption**, via the dataset's `encryption` property.
///
/// Other filesystem-internal encryption schemes that leave no block-layer trace
/// (bcachefs native encryption, fscrypt on ext4/f2fs, eCryptfs) are not detected
/// and resolve to `DetectionFailed` - fail-safe: a required posture rule fails
/// rather than falsely passing.
/// Reports whether the device stack backing `path` includes an encryption layer.
///
/// `path` is supplied by the caller rather than derived here, because who is asking changes the
/// answer: a user-initiated check means the partition holding the client database, while the service
/// means `/`. Deriving it internally would silently answer for whichever process happened to call.
pub(super) fn disk_encryption_status(path: &Path) -> Result<bool, UnavailableReason> {
    // Resolve the target and the mount that backs it.
    let db_path = canonicalize_on_disk(path).ok_or(UnavailableReason::DetectionFailed)?;

    let mountinfo =
        read_to_string(MOUNTINFO_PATH).map_err(|_| UnavailableReason::DetectionFailed)?;
    let mounts = parse_mountinfo(&mountinfo);
    let backing =
        find_backing_mount(&mounts, &db_path).ok_or(UnavailableReason::DetectionFailed)?;

    // ZFS encryption is a dataset property, not a block-layer device; the mount
    // source is the dataset name rather than a `/dev` path.
    if backing.fstype == "zfs" {
        return zfs_dataset_encrypted(&backing.source);
    }

    // Otherwise map the mount source to its kernel device name and inspect only
    // that device's stack for a LUKS/dm-crypt layer.
    let kname = source_kname(&backing.source).ok_or(UnavailableReason::DetectionFailed)?;
    device_is_encrypted(&kname).ok_or(UnavailableReason::DetectionFailed)
}

/// Un-escapes the octal sequences (`\040` space, `\011` tab, `\012` newline,
/// `\134` backslash) that the kernel uses for special characters in
/// `mountinfo` fields.
fn unescape_mountinfo(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let octal: String = chars.clone().take(3).collect();
            if octal.len() == 3 {
                if let Ok(code) = u8::from_str_radix(&octal, 8) {
                    out.push(code as char);
                    // Consume the three octal digits we just parsed.
                    chars.nth(2);
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

/// Parses `/proc/self/mountinfo` content into a list of mount entries.
///
/// Each line has the form
/// `<id> <parent> <maj:min> <root> <mount_point> <opts> <optional...> - <fstype> <source> <super_opts>`.
/// The mount point is field index 4 (before the `-` separator); the filesystem
/// type and backing device source are the first two fields after ` - `.
fn parse_mountinfo(content: &str) -> Vec<MountEntry> {
    content
        .lines()
        .filter_map(|line| {
            let (fields, rest) = line.split_once(" - ")?;
            let mount_point = fields.split(' ').nth(4)?;
            let mut post = rest.split(' ');
            let fstype = post.next()?;
            let source = post.next()?;
            Some(MountEntry {
                mount_point: unescape_mountinfo(mount_point),
                fstype: fstype.to_owned(),
                source: unescape_mountinfo(source),
            })
        })
        .collect()
}

/// Returns the mount entry that backs `path`: the one whose mount point is the
/// longest path-prefix of `path`. Matching is component-aware, so `/var` does
/// not match `/vart`. Among entries sharing the longest mount point (overmounts)
/// the last one wins, matching the kernel's effective mount.
fn find_backing_mount<'a>(mounts: &'a [MountEntry], path: &Path) -> Option<&'a MountEntry> {
    mounts
        .iter()
        .filter(|entry| path.starts_with(&entry.mount_point))
        .max_by_key(|entry| entry.mount_point.len())
}

/// Whether the device `kname` is an opened dm-crypt mapping, per its sysfs
/// `dm/uuid` (dm-crypt devices carry a `CRYPT-` prefix, e.g. `CRYPT-LUKS2-…`).
fn dm_uuid_is_crypt(kname: &str) -> bool {
    read_to_string(Path::new(SYS_BLOCK).join(kname).join("dm/uuid"))
        .is_ok_and(|uuid| uuid.trim_start().starts_with("CRYPT-"))
}

/// Kernel names of the devices `kname` is stacked on top of (its sysfs
/// `slaves/`): e.g. an LVM LV's slave is its dm-crypt device, whose slave is the
/// LUKS partition. Empty when the device has no lower devices (e.g. a plain
/// partition) or the directory is absent.
fn slaves_of(kname: &str) -> Vec<String> {
    let slaves_dir = Path::new(SYS_BLOCK).join(kname).join("slaves");
    let Ok(entries) = std::fs::read_dir(slaves_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Returns whether `kname`, or any device it stacks on, is a dm-crypt mapping,
/// walking the dependency chain via `slaves`. The `is_crypt` and `slaves`
/// readers are injected so the traversal is testable without real sysfs.
fn stack_has_crypt(
    kname: &str,
    is_crypt: &impl Fn(&str) -> bool,
    slaves: &impl Fn(&str) -> Vec<String>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(kname.to_owned()) {
        return false;
    }
    is_crypt(kname)
        || slaves(kname)
            .iter()
            .any(|slave| stack_has_crypt(slave, is_crypt, slaves, visited))
}

/// Returns whether the block device with kernel name `kname` is encrypted
/// (backed by dm-crypt/LUKS anywhere in its stack), or `None` if the device is
/// not present in sysfs (e.g. a non-block-backed filesystem such as
/// tmpfs/overlay/zfs).
fn device_is_encrypted(kname: &str) -> Option<bool> {
    if !Path::new(SYS_BLOCK).join(kname).exists() {
        return None;
    }
    Some(stack_has_crypt(
        kname,
        &dm_uuid_is_crypt,
        &slaves_of,
        &mut HashSet::new(),
    ))
}

/// Canonicalizes `path`, or its nearest existing ancestor when the file/dirs do
/// not exist yet. The nearest existing ancestor lives on the same partition the
/// database will be created on (intermediate dirs are created there; mount
/// points must already exist), so it identifies the correct backing device.
fn canonicalize_on_disk(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(p) = current {
        if let Ok(canonical) = p.canonicalize() {
            return Some(canonical);
        }
        current = p.parent();
    }
    None
}

/// Resolves a mount source to the kernel device name of its block device, e.g.
/// `/dev/mapper/cryptroot` -> `dm-0`. Returns `None` when the source is not a
/// resolvable block device path (e.g. `tmpfs`, a ZFS dataset name).
fn source_kname(source: &str) -> Option<String> {
    Path::new(source)
        .canonicalize()
        .ok()?
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// Interprets the value of a ZFS `encryption` property.
///
/// `off` -> not encrypted; a cipher name (e.g. `aes-256-gcm`) or `on` ->
/// encrypted; an empty or `-` value (unknown/unsupported) -> `None`.
fn parse_zfs_encryption(value: &str) -> Option<bool> {
    match value.trim() {
        "" | "-" => None,
        "off" => Some(false),
        _ => Some(true),
    }
}

/// Reports whether a ZFS dataset uses native encryption.
///
/// ZFS encryption is a per-dataset filesystem property with no block-layer
/// (dm-crypt) representation, so it is queried directly via the `zfs` CLI rather
/// than through sysfs. A mounted dataset implies its key is loaded, so the
/// `encryption` property alone is sufficient (no separate `keystatus` check).
fn zfs_dataset_encrypted(dataset: &str) -> Result<bool, UnavailableReason> {
    let output = Command::new("zfs")
        .args(["get", "-H", "-o", "value", "encryption", dataset])
        .output()
        .map_err(|_| UnavailableReason::DetectionFailed)?;
    if !output.status.success() {
        return Err(UnavailableReason::DetectionFailed);
    }
    let value = String::from_utf8_lossy(&output.stdout);
    parse_zfs_encryption(&value).ok_or(UnavailableReason::DetectionFailed)
}

#[cfg(test)]
mod unit_tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn unescape_handles_octal_sequences() {
        assert_eq!(unescape_mountinfo("/mnt/my\\040disk"), "/mnt/my disk");
        assert_eq!(unescape_mountinfo("/plain/path"), "/plain/path");
        // A lone backslash that is not a valid escape is preserved.
        assert_eq!(unescape_mountinfo("/a\\b"), "/a\\b");
    }

    #[test]
    fn parse_mountinfo_extracts_mount_point_fstype_and_source() {
        let content = "\
36 35 0:30 / / rw,noatime shared:1 - btrfs /dev/mapper/cryptroot rw,subvol=/
38 36 0:32 / /mnt/my\\040disk rw shared:3 - ext4 /dev/sdb1 rw
39 36 0:33 / /data rw shared:4 - zfs rpool/USERDATA/x rw";
        let mounts = parse_mountinfo(content);
        let rows: Vec<(&str, &str, &str)> = mounts
            .iter()
            .map(|m| (m.mount_point.as_str(), m.fstype.as_str(), m.source.as_str()))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("/", "btrfs", "/dev/mapper/cryptroot"),
                ("/mnt/my disk", "ext4", "/dev/sdb1"),
                ("/data", "zfs", "rpool/USERDATA/x"),
            ]
        );
    }

    fn mount(mount_point: &str, source: &str) -> MountEntry {
        MountEntry {
            mount_point: mount_point.to_owned(),
            fstype: "ext4".to_owned(),
            source: source.to_owned(),
        }
    }

    #[test]
    fn parse_zfs_encryption_interprets_property() {
        assert_eq!(parse_zfs_encryption("off"), Some(false));
        assert_eq!(parse_zfs_encryption("on"), Some(true));
        assert_eq!(parse_zfs_encryption("aes-256-gcm"), Some(true));
        assert_eq!(parse_zfs_encryption("aes-256-gcm\n"), Some(true));
        assert_eq!(parse_zfs_encryption("-"), None);
        assert_eq!(parse_zfs_encryption(""), None);
    }

    #[test]
    fn find_backing_mount_picks_longest_prefix() {
        let mounts = vec![
            mount("/", "/dev/sda1"),
            mount("/var", "/dev/sda2"),
            mount("/var/lib", "/dev/sda3"),
        ];
        assert_eq!(
            find_backing_mount(&mounts, Path::new("/var/lib/defguard/db"))
                .map(|m| m.source.as_str()),
            Some("/dev/sda3")
        );
        assert_eq!(
            find_backing_mount(&mounts, Path::new("/var/log/x")).map(|m| m.source.as_str()),
            Some("/dev/sda2")
        );
        assert_eq!(
            find_backing_mount(&mounts, Path::new("/home/x")).map(|m| m.source.as_str()),
            Some("/dev/sda1")
        );
    }

    #[test]
    fn find_backing_mount_is_component_aware() {
        let mounts = vec![mount("/", "/dev/sda1"), mount("/var", "/dev/sda2")];
        // "/vart" must not match the "/var" mount.
        assert_eq!(
            find_backing_mount(&mounts, Path::new("/vart/x")).map(|m| m.source.as_str()),
            Some("/dev/sda1")
        );
    }

    /// Runs `stack_has_crypt` over an in-memory device graph: `crypt` is the set
    /// of dm-crypt kernel names, `slaves` maps each device to the devices it
    /// stacks on (its lower devices).
    fn stack_encrypted(start: &str, crypt: &[&str], slaves: &[(&str, &[&str])]) -> bool {
        let crypt: HashSet<&str> = crypt.iter().copied().collect();
        let slaves: HashMap<&str, Vec<String>> = slaves
            .iter()
            .map(|(k, v)| (*k, v.iter().map(|s| (*s).to_owned()).collect()))
            .collect();
        let is_crypt = |k: &str| crypt.contains(k);
        let slaves_of = |k: &str| slaves.get(k).cloned().unwrap_or_default();
        stack_has_crypt(start, &is_crypt, &slaves_of, &mut HashSet::new())
    }

    #[test]
    fn plain_luks_device_is_encrypted() {
        // Mounted device is the opened crypt mapping itself.
        let slaves = [("dm-0", &["sda2"][..]), ("sda2", &["sda"][..])];
        assert!(stack_encrypted("dm-0", &["dm-0"], &slaves));
    }

    #[test]
    fn luks_under_lvm_device_is_encrypted() {
        // Mounted LVM logical volume stacks on top of a crypt ancestor (dm-0).
        // This is the layered case the previous flat-lsblk walk missed.
        let slaves = [
            ("dm-1", &["dm-0"][..]),
            ("dm-0", &["vda4"][..]),
            ("vda4", &["vda"][..]),
        ];
        assert!(stack_encrypted("dm-1", &["dm-0"], &slaves));
    }

    #[test]
    fn plaintext_device_is_not_encrypted() {
        let slaves = [("sda2", &["sda"][..])];
        assert!(!stack_encrypted("sda2", &[], &slaves));
    }

    #[test]
    fn unrelated_encrypted_device_does_not_leak() {
        // Only the target device's own stack is inspected; an encrypted device in
        // a separate stack must not leak (the regression this hardening targets).
        let slaves = [("sda2", &["sda"][..]), ("dm-9", &["loop0"][..])];
        assert!(!stack_encrypted("sda2", &["dm-9"], &slaves));
        assert!(stack_encrypted("dm-9", &["dm-9"], &slaves));
    }

    #[test]
    fn stack_walk_terminates_on_cycles() {
        // A pathological slaves cycle must not loop forever.
        let slaves = [("a", &["b"][..]), ("b", &["a"][..])];
        assert!(!stack_encrypted("a", &[], &slaves));
    }

    #[test]
    fn canonicalize_ascends_to_nearest_existing_ancestor() {
        // A deep non-existent DB path resolves to its nearest existing ancestor
        // (the default app dir does not exist before first run).
        let base = std::env::temp_dir();
        let deep = base.join("defguard-posture-nonexistent-xyz/a/b/defguard.db");
        assert_eq!(canonicalize_on_disk(&deep), base.canonicalize().ok());
    }
}
