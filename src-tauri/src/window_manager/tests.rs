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

fn work_area(width: u32, height: u32, scale_factor: f64) -> WorkArea {
    WorkArea {
        width,
        height,
        scale_factor,
    }
}

#[test]
fn test_compact_window_fits_default_macbook_air() {
    // 13" Air default (1470x956 points @2x), menu bar with notch takes ~38pt.
    assert!(compact_window_fits(work_area(2940, 1836, 2.0)));
}

#[test]
fn test_compact_window_does_not_fit_larger_text_macbook_air() {
    // "Larger Text" (1024x665 points @2x), work area without the menu bar.
    assert!(!compact_window_fits(work_area(2048, 1254, 2.0)));
    // Even the whole screen is shorter than the 680pt tray window.
    assert!(!compact_window_fits(work_area(2048, 1330, 2.0)));
}

#[test]
fn test_compact_window_fit_counts_dock_and_menu_bar() {
    // 1147x745 points: menu bar and a visible Dock leave ~640pt.
    assert!(!compact_window_fits(work_area(2294, 1280, 2.0)));
    // Same display with the Dock hidden leaves ~708pt.
    assert!(compact_window_fits(work_area(2294, 1416, 2.0)));
}

#[test]
fn test_compact_window_does_not_fit_small_scaled_windows_display() {
    // 1366x768 @150% with a 48px taskbar: ~911x480 logical.
    assert!(!compact_window_fits(work_area(1366, 720, 1.5)));
    // 1920x1080 @100% with a taskbar fits.
    assert!(compact_window_fits(work_area(1920, 1032, 1.0)));
}

#[test]
fn test_compact_window_fits_on_unusable_display_data() {
    assert!(compact_window_fits(work_area(1920, 1080, 0.0)));
    assert!(compact_window_fits(work_area(1920, 1080, f64::NAN)));
    assert!(compact_window_fits(work_area(1920, 1080, f64::INFINITY)));
    assert!(compact_window_fits(work_area(0, 0, 1.0)));
}

#[test]
fn test_full_view_min_size_keeps_default_on_large_display() {
    assert_eq!(
        full_view_min_size((1470.0, 918.0)),
        (FULL_VIEW_WINDOW_WIDTH, FULL_VIEW_WINDOW_HEIGHT)
    );
}

#[test]
fn test_full_view_min_size_relaxes_on_small_display() {
    // "Larger Text" MacBook Air work area.
    assert_eq!(
        full_view_min_size((1024.0, 627.0)),
        (FULL_VIEW_WINDOW_WIDTH, 627.0)
    );
    // 1366x768 @150% Windows work area.
    assert_eq!(full_view_min_size((750.0, 480.0)), (750.0, 480.0));
}

#[test]
fn test_full_view_min_size_ignores_unusable_space() {
    let default = (FULL_VIEW_WINDOW_WIDTH, FULL_VIEW_WINDOW_HEIGHT);
    assert_eq!(full_view_min_size((0.0, 0.0)), default);
    assert_eq!(full_view_min_size((f64::NAN, -10.0)), default);
}
