use super::*;
use crate::database::models::location::{
    legacy_mfa_steps, LocationMfaMode, LocationMfaStep, LocationMfaStepMethod, ServiceLocationMode,
};
use sqlx::types::Json;

fn test_location(location_mfa_mode: LocationMfaMode) -> Location<Id> {
    Location {
        id: 1,
        instance_id: 2,
        network_id: 3,
        name: "loc".into(),
        address: "10.0.0.2/24".into(),
        pubkey: "pk".into(),
        endpoint: "1.2.3.4:51820".into(),
        allowed_ips: "0.0.0.0/0".into(),
        dns: None,
        route_all_traffic: false,
        keepalive_interval: 25,
        location_mfa_mode,
        service_location_mode: ServiceLocationMode::Disabled,
        mfa_method: Some(LocationMfaMethod::Totp),
        posture_check_required: false,
        mfa_steps: Json::default(),
        mfa_step_plan: Json::default(),
        client_mtu: None,
    }
}

#[test]
fn test_mfa_trigger_payload_uses_legacy_steps_for_upgraded_location() {
    let location = test_location(LocationMfaMode::Internal);
    assert!(location.mfa_steps.0.is_empty());

    let payload = mfa_trigger_payload(&location);

    assert_eq!(
        payload.mfa_steps.0,
        legacy_mfa_steps(LocationMfaMode::Internal)
    );
}

#[test]
fn test_mfa_trigger_payload_preserves_saved_steps_and_plan() {
    let mut location = test_location(LocationMfaMode::Internal);
    let saved_steps = vec![
        LocationMfaStep {
            methods: vec![LocationMfaStepMethod {
                method: LocationMfaMethod::Totp,
                configured: true,
            }],
        },
        LocationMfaStep {
            methods: vec![LocationMfaStepMethod {
                method: LocationMfaMethod::Email,
                configured: true,
            }],
        },
    ];
    let saved_plan = vec![LocationMfaMethod::Totp, LocationMfaMethod::Email];
    location.mfa_steps = Json(saved_steps.clone());
    location.mfa_step_plan = Json(saved_plan.clone());

    let payload = mfa_trigger_payload(&location);

    assert_eq!(payload.mfa_steps.0, saved_steps);
    assert_eq!(payload.mfa_step_plan.0, saved_plan);
}
