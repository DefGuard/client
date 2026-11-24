use super::*;

fn device_config(preshared_key: Option<String>) -> DeviceConfig {
    DeviceConfig {
        network_name: "test".to_owned(),
        pubkey: Key::generate().public_key().to_string(),
        endpoint: "127.0.0.1:51820".to_owned(),
        allowed_ips: "10.0.0.0/24".to_owned(),
        preshared_key,
        ..Default::default()
    }
}

#[test]
fn build_peer_sets_preshared_key() {
    let preshared_key = Key::generate();
    let peer = build_peer(&device_config(Some(preshared_key.to_string()))).unwrap();
    assert_eq!(peer.preshared_key, Some(preshared_key));
}

#[test]
fn build_peer_without_preshared_key() {
    let peer = build_peer(&device_config(None)).unwrap();
    assert_eq!(peer.preshared_key, None);
}

#[test]
fn build_peer_rejects_invalid_preshared_key() {
    let result = build_peer(&device_config(Some("not-a-key".to_owned())));
    assert!(matches!(result, Err(CliError::InvalidPresharedKey(_))));
}

#[test]
fn config_without_preshared_key_field_loads() {
    let mut json = serde_json::to_value(CliConfig::default()).unwrap();
    json["device_config"]
        .as_object_mut()
        .unwrap()
        .remove("preshared_key");
    let config: CliConfig = serde_json::from_value(json).unwrap();
    assert_eq!(config.device_config.preshared_key, None);
}

#[test]
fn debug_output_hides_preshared_key() {
    let preshared_key = Key::generate().to_string();
    let config = CliConfig {
        device_config: device_config(Some(preshared_key.clone())),
        ..Default::default()
    };
    assert!(!format!("{config:?}").contains(&preshared_key));
}

#[test]
fn save_replaces_existing_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    CliConfig::default().save(&path).unwrap();
    let config = CliConfig {
        token: Some("polling-token".to_owned()),
        ..Default::default()
    };
    config.save(&path).unwrap();

    assert_eq!(
        CliConfig::load(&path).unwrap().token.as_deref(),
        Some("polling-token")
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn failed_write_keeps_existing_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let config = CliConfig {
        token: Some("polling-token".to_owned()),
        ..Default::default()
    };
    config.save(&path).unwrap();
    let original = std::fs::read(&path).unwrap();

    let result = write_atomically(&path, |file| {
        file.write_all(b"{\"private_key\":")?;
        Err(io::Error::new(io::ErrorKind::StorageFull, "disk full"))
    });

    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn saved_config_is_owner_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    CliConfig::default().save(&path).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}
