use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
};

use defguard_client_core::database::db_file_path;
use serde::Deserialize;

use super::UnavailableReason;

/// Path to the kernel's mount table for the current process.
const MOUNTINFO_PATH: &str = "/proc/self/mountinfo";

#[derive(Deserialize)]
struct LsblkDevice {
    /// Kernel device name (e.g. `dm-0`, `nvme0n1p2`), stable across `lsblk`
    /// versions and matching the basename of the canonicalized device path.
    kname: Option<String>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    fstype: Option<String>,
    children: Option<Vec<LsblkDevice>>,
}

impl LsblkDevice {
    /// Whether this device is itself an encryption layer that any data stacked
    /// on top of it passes through: an opened dm-crypt mapping (`type == "crypt"`)
    /// or a LUKS container partition (`fstype == "crypto_LUKS"`).
    #[must_use]
    fn is_encryption_layer(&self) -> bool {
        self.device_type.as_deref() == Some("crypt")
            || self.fstype.as_deref() == Some("crypto_LUKS")
    }
}

#[derive(Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<LsblkDevice>,
}

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

/// Walks a single `lsblk` device subtree looking for the device with kernel
/// name `kname`. Returns `Some(true)`/`Some(false)` for whether that device (or
/// any ancestor it stacks on) is an encryption layer, or `None` when `kname` is
/// not found in this subtree.
fn walk_device(device: &LsblkDevice, kname: &str, encrypted_ancestor: bool) -> Option<bool> {
    let encrypted = encrypted_ancestor || device.is_encryption_layer();
    if device.kname.as_deref() == Some(kname) {
        return Some(encrypted);
    }
    if let Some(children) = &device.children {
        for child in children {
            if let Some(result) = walk_device(child, kname, encrypted) {
                return Some(result);
            }
        }
    }
    None
}

/// Returns whether the block device with kernel name `kname` is encrypted, or
/// `None` if it is not present in the tree (e.g. a non-block-backed filesystem
/// such as tmpfs/overlay/zfs that `lsblk` does not represent).
fn device_is_encrypted(devices: &[LsblkDevice], kname: &str) -> Option<bool> {
    devices
        .iter()
        .find_map(|device| walk_device(device, kname, false))
}

/// Canonicalizes `path`, falling back to its parent directory when the file does
/// not exist yet (the parent lives on the same mount).
fn canonicalize_on_disk(path: &Path) -> Option<PathBuf> {
    path.canonicalize()
        .ok()
        .or_else(|| path.parent().and_then(|parent| parent.canonicalize().ok()))
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
/// than through `lsblk`. A mounted dataset implies its key is loaded, so the
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

/// Runs `lsblk` and returns the parsed block device tree.
fn read_block_devices() -> Result<Vec<LsblkDevice>, UnavailableReason> {
    let output = Command::new("lsblk")
        .args(["-J", "-o", "KNAME,TYPE,FSTYPE"])
        .output()
        .map_err(|_| UnavailableReason::DetectionFailed)?;
    if !output.status.success() {
        return Err(UnavailableReason::DetectionFailed);
    }
    let parsed: LsblkOutput =
        serde_json::from_slice(&output.stdout).map_err(|_| UnavailableReason::DetectionFailed)?;
    Ok(parsed.blockdevices)
}

/// Reports whether the partition that stores the client's database file is
/// encrypted.
///
/// Unlike a global "is any device encrypted?" check, this resolves the specific
/// filesystem that backs the database file and inspects only that, so unrelated
/// encrypted loop/removable/test volumes do not produce a false positive.
///
/// Two encryption mechanisms are recognized:
/// - **LUKS/dm-crypt** for block-backed filesystems (ext4/xfs/btrfs/LVM/…), via
///   the device stack reported by `lsblk`;
/// - **native ZFS encryption**, via the dataset's `encryption` property.
///
/// Other filesystem-internal encryption schemes that are invisible to the block
/// layer — bcachefs native encryption, fscrypt on ext4/f2fs, eCryptfs — are not
/// detected and resolve to `DetectionFailed`. This is fail-safe: a required
/// posture rule fails rather than falsely passing.
pub(super) fn disk_encryption_status() -> Result<bool, UnavailableReason> {
    // Resolve the database file and the mount that backs it.
    let db_path = db_file_path().ok_or(UnavailableReason::DetectionFailed)?;
    let db_path = canonicalize_on_disk(&db_path).ok_or(UnavailableReason::DetectionFailed)?;

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
    // that stack for a LUKS/dm-crypt layer.
    let kname = source_kname(&backing.source).ok_or(UnavailableReason::DetectionFailed)?;
    let devices = read_block_devices()?;
    device_is_encrypted(&devices, &kname).ok_or(UnavailableReason::DetectionFailed)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn devices(json: &str) -> Vec<LsblkDevice> {
        serde_json::from_str::<LsblkOutput>(json)
            .expect("invalid lsblk fixture")
            .blockdevices
    }

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

    #[test]
    fn plain_luks_device_is_encrypted() {
        // Mounted device is the opened crypt mapping itself.
        let json = r#"{"blockdevices":[
            {"kname":"sda","type":"disk","fstype":null,"children":[
                {"kname":"sda2","type":"part","fstype":"crypto_LUKS","children":[
                    {"kname":"dm-0","type":"crypt","fstype":"btrfs","children":null}
                ]}
            ]}
        ]}"#;
        assert_eq!(device_is_encrypted(&devices(json), "dm-0"), Some(true));
    }

    #[test]
    fn luks_under_lvm_device_is_encrypted() {
        // Mounted LVM logical volume stacks on top of a crypt ancestor.
        let json = r#"{"blockdevices":[
            {"kname":"sda","type":"disk","fstype":null,"children":[
                {"kname":"sda2","type":"part","fstype":"crypto_LUKS","children":[
                    {"kname":"dm-0","type":"crypt","fstype":"LVM2_member","children":[
                        {"kname":"dm-1","type":"lvm","fstype":"ext4","children":null}
                    ]}
                ]}
            ]}
        ]}"#;
        assert_eq!(device_is_encrypted(&devices(json), "dm-1"), Some(true));
    }

    #[test]
    fn plaintext_device_is_not_encrypted() {
        let json = r#"{"blockdevices":[
            {"kname":"sda","type":"disk","fstype":null,"children":[
                {"kname":"sda2","type":"part","fstype":"ext4","children":null}
            ]}
        ]}"#;
        assert_eq!(device_is_encrypted(&devices(json), "sda2"), Some(false));
    }

    #[test]
    fn unrelated_encrypted_device_does_not_leak() {
        // An encrypted loop device must not make the plaintext root device report
        // as encrypted (the regression this hardening targets).
        let json = r#"{"blockdevices":[
            {"kname":"loop0","type":"loop","fstype":"crypto_LUKS","children":[
                {"kname":"dm-9","type":"crypt","fstype":"ext4","children":null}
            ]},
            {"kname":"sda","type":"disk","fstype":null,"children":[
                {"kname":"sda2","type":"part","fstype":"ext4","children":null}
            ]}
        ]}"#;
        assert_eq!(device_is_encrypted(&devices(json), "sda2"), Some(false));
        assert_eq!(device_is_encrypted(&devices(json), "dm-9"), Some(true));
    }

    #[test]
    fn missing_device_returns_none() {
        let json = r#"{"blockdevices":[
            {"kname":"sda","type":"disk","fstype":null,"children":[
                {"kname":"sda2","type":"part","fstype":"ext4","children":null}
            ]}
        ]}"#;
        assert_eq!(device_is_encrypted(&devices(json), "dm-42"), None);
    }
}
