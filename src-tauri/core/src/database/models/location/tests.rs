use sqlx::SqlitePool;

use super::*;
use crate::{
    database::models::instance::{ClientTrafficPolicy, Instance, OpenIdProviderKind},
    mfa_contract::MfaContract,
};

fn new_instance() -> Instance<NoId> {
    Instance {
        id: NoId,
        name: "instance".into(),
        uuid: "uuid-1".into(),
        url: "https://core.example".into(),
        proxy_url: "https://proxy.example".into(),
        username: "alice".into(),
        token: None,
        client_traffic_policy: ClientTrafficPolicy::None,
        enterprise_enabled: false,
        disable_tunnels: false,
        openid_display_name: None,
        openid_provider_kind: OpenIdProviderKind::Custom,
        mfa_configured_methods: None,
        mfa_contract: MfaContract::Legacy,
        mfa_capabilities: None,
        smtp_configured: None,
        openid_available: None,
    }
}

fn new_location(instance_id: Id) -> Location<NoId> {
    Location {
        id: NoId,
        instance_id,
        network_id: 1,
        name: "loc".into(),
        address: "10.0.0.2/24".into(),
        pubkey: "pk".into(),
        endpoint: "1.2.3.4:51820".into(),
        allowed_ips: "0.0.0.0/0".into(),
        dns: None,
        route_all_traffic: false,
        keepalive_interval: 25,
        location_mfa_mode: LocationMfaMode::Disabled,
        service_location_mode: ServiceLocationMode::Disabled,
        mfa_method: None,
        posture_check_required: false,
        mfa_steps: Default::default(),
        mfa_step_plan: Default::default(),
        client_mtu: None,
    }
}

#[test]
fn test_effective_mfa_steps() {
    let mut location = new_location(1);
    location.location_mfa_mode = LocationMfaMode::Internal;
    assert_eq!(
        location.effective_mfa_steps(),
        legacy_mfa_steps(LocationMfaMode::Internal)
    );
    location.location_mfa_mode = LocationMfaMode::External;
    assert_eq!(
        location.effective_mfa_steps(),
        legacy_mfa_steps(LocationMfaMode::External)
    );
    location.location_mfa_mode = LocationMfaMode::Disabled;
    assert_eq!(
        location.effective_mfa_steps(),
        Vec::<LocationMfaStep>::new()
    );

    let saved_steps = vec![LocationMfaStep {
        methods: vec![LocationMfaStepMethod {
            method: LocationMfaMethod::Totp,
            configured: false,
        }],
    }];
    location.location_mfa_mode = LocationMfaMode::Internal;
    location.mfa_steps = Json(saved_steps.clone());
    assert_eq!(location.effective_mfa_steps(), saved_steps);
}
#[test]
fn test_mfa_enabled_uses_effective_mfa_steps() {
    let mut location = new_location(1);
    location.location_mfa_mode = LocationMfaMode::Internal;
    assert!(location.mfa_enabled());
    location.location_mfa_mode = LocationMfaMode::External;
    assert!(location.mfa_enabled());
    location.location_mfa_mode = LocationMfaMode::Disabled;
    assert!(!location.mfa_enabled());

    location.mfa_steps = Json(vec![LocationMfaStep {
        methods: vec![LocationMfaStepMethod {
            method: LocationMfaMethod::Totp,
            configured: false,
        }],
    }]);
    location.location_mfa_mode = LocationMfaMode::Internal;
    assert!(location.mfa_enabled());
    location.location_mfa_mode = LocationMfaMode::External;
    assert!(location.mfa_enabled());
    location.location_mfa_mode = LocationMfaMode::Disabled;
    assert!(location.mfa_enabled());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_location_crud_round_trip(pool: SqlitePool) {
    let instance = new_instance().save(&pool).await.unwrap();
    let location = new_location(instance.id).save(&pool).await.unwrap();

    let found = Location::find_by_id(&pool, location.id)
        .await
        .unwrap()
        .expect("location should exist");
    assert_eq!(found.name, "loc");
    assert_eq!(found.instance_id, instance.id);

    location.delete(&pool).await.unwrap();
    assert!(Location::find_by_id(&pool, location.id)
        .await
        .unwrap()
        .is_none());
}

#[test]
fn test_mfa_method_serde_matches_frontend_contract() {
    // The frontend compares these strings, and `as_str` says "mobile" for the same variant.
    assert_eq!(
        serde_json::to_string(&LocationMfaMethod::MobileApprove).unwrap(),
        "\"mobileapprove\""
    );
    assert_eq!(
        serde_json::to_string(&LocationMfaMethod::Fido2).unwrap(),
        "\"fido2\""
    );
}

#[test]
fn test_effective_mtu_precedence() {
    let mut location = new_location(1);

    // Nothing configured anywhere: let the system pick.
    assert_eq!(location.effective_mtu(None), None);

    // Core-provided MTU is used when settings leave it unset.
    location.client_mtu = Some(1380);
    assert_eq!(location.effective_mtu(None), Some(1380));

    // An MTU pinned in the settings overrides the core-provided one.
    assert_eq!(location.effective_mtu(Some(1420)), Some(1420));

    // ...and is used even when the core sent nothing.
    location.client_mtu = None;
    assert_eq!(location.effective_mtu(Some(1420)), Some(1420));
}

#[sqlx::test(migrations = "../migrations")]
async fn test_client_mtu_round_trip(pool: SqlitePool) {
    let instance = new_instance().save(&pool).await.unwrap();
    let mut location = new_location(instance.id);
    location.client_mtu = Some(1380);
    let mut location = location.save(&pool).await.unwrap();

    let found = Location::find_by_id(&pool, location.id)
        .await
        .unwrap()
        .expect("location should exist");
    assert_eq!(found.client_mtu, Some(1380));

    // The core dropping the value clears it on the next save.
    location.client_mtu = None;
    location.save(&pool).await.unwrap();
    let found = Location::find_by_id(&pool, location.id)
        .await
        .unwrap()
        .expect("location should exist");
    assert_eq!(found.client_mtu, None);
}

#[test]
fn test_infer_mfa_method() {
    use LocationMfaMethod::{Biometric, Email, Oidc, Totp};
    use LocationMfaMode::{Disabled, External, Internal};

    // Disabled mode passes the configured method through unchanged.
    assert_eq!(infer_mfa_method(Disabled, None), None);
    assert_eq!(infer_mfa_method(Disabled, Some(Totp)), Some(Totp));

    // Internal mode forces Totp when no method or OIDC is configured.
    assert_eq!(infer_mfa_method(Internal, None), Some(Totp));
    assert_eq!(infer_mfa_method(Internal, Some(Oidc)), Some(Totp));
    // Internal mode keeps any other explicit method.
    assert_eq!(infer_mfa_method(Internal, Some(Email)), Some(Email));
    assert_eq!(infer_mfa_method(Internal, Some(Biometric)), Some(Biometric));

    // External mode always resolves to OIDC, ignoring the configured method.
    assert_eq!(infer_mfa_method(External, None), Some(Oidc));
    assert_eq!(infer_mfa_method(External, Some(Totp)), Some(Oidc));
}

#[test]
fn test_location_mfa_mode_from_proto() {
    assert_eq!(
        LocationMfaMode::from(ProtoLocationMfaMode::Unspecified),
        LocationMfaMode::Disabled
    );
    assert_eq!(
        LocationMfaMode::from(ProtoLocationMfaMode::Disabled),
        LocationMfaMode::Disabled
    );
    assert_eq!(
        LocationMfaMode::from(ProtoLocationMfaMode::Internal),
        LocationMfaMode::Internal
    );
    assert_eq!(
        LocationMfaMode::from(ProtoLocationMfaMode::External),
        LocationMfaMode::External
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn test_effective_route_all_traffic(pool: SqlitePool) {
    use ClientTrafficPolicy::{DisableAllTraffic, ForceAllTraffic, None as NoPolicy};

    // (policy, stored flag, per-call override, expected)
    let cases = [
        (NoPolicy, false, None, false),
        (NoPolicy, false, Some(true), true),
        (NoPolicy, true, None, true),
        (NoPolicy, true, Some(false), false),
        (ForceAllTraffic, false, None, true),
        (ForceAllTraffic, false, Some(false), true),
        (DisableAllTraffic, true, None, false),
        (DisableAllTraffic, true, Some(true), false),
    ];

    let mut instance = new_instance().save(&pool).await.unwrap();
    let mut location = new_location(instance.id).save(&pool).await.unwrap();

    for (policy, route_all_traffic, override_value, expected) in cases {
        instance.client_traffic_policy = policy.clone();
        instance.save(&pool).await.unwrap();
        location.route_all_traffic = route_all_traffic;

        let effective = location
            .effective_route_all_traffic(&pool, override_value)
            .await
            .unwrap();
        assert_eq!(
            effective, expected,
            "policy {policy:?}, flag {route_all_traffic}, override {override_value:?}"
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn test_routed_networks(pool: SqlitePool) {
    let instance = new_instance().save(&pool).await.unwrap();
    let mut location = new_location(instance.id);
    location.address = "10.6.0.2/24, fd00::2/64".into();
    location.allowed_ips = "10.0.0.0/8, bad-entry, 192.168.1.0/24".into();
    let location = location.save(&pool).await.unwrap();

    let networks = |list: &[&str]| -> Vec<IpAddrMask> {
        list.iter().map(|entry| entry.parse().unwrap()).collect()
    };
    assert_eq!(
        location.routed_networks(&pool, None).await.unwrap(),
        networks(&["10.6.0.2/24", "fd00::2/64", "10.0.0.0/8", "192.168.1.0/24"])
    );
    assert_eq!(
        location.routed_networks(&pool, Some(true)).await.unwrap(),
        networks(&["10.6.0.2/24", "fd00::2/64", "0.0.0.0/0", "::/0"])
    );
}

#[test]
fn test_service_location_mode_from_proto() {
    assert_eq!(
        ServiceLocationMode::from(ProtoServiceLocationMode::Unspecified),
        ServiceLocationMode::Disabled
    );
    assert_eq!(
        ServiceLocationMode::from(ProtoServiceLocationMode::Disabled),
        ServiceLocationMode::Disabled
    );
    assert_eq!(
        ServiceLocationMode::from(ProtoServiceLocationMode::Prelogon),
        ServiceLocationMode::PreLogon
    );
    assert_eq!(
        ServiceLocationMode::from(ProtoServiceLocationMode::Alwayson),
        ServiceLocationMode::AlwaysOn
    );
}
