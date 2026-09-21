use defguard_core::database::models::location::{
    LocationMfaMode, LocationMfaStepMethod, ServiceLocationMode,
};
use sqlx::types::Json;

use super::*;

fn make_location(id: Id, instance_id: Id, name: &str, endpoint: &str, mfa: bool) -> Location<Id> {
    Location {
        mfa_steps: Json::default(),
        mfa_step_plan: Json::default(),
        client_mtu: None,
        id,
        instance_id,
        network_id: 1,
        name: name.to_string(),
        address: "10.0.0.0/24".to_string(),
        pubkey: "pk".to_string(),
        endpoint: endpoint.to_string(),
        allowed_ips: "0.0.0.0/0".to_string(),
        dns: None,
        route_all_traffic: false,
        keepalive_interval: 25,
        location_mfa_mode: if mfa {
            LocationMfaMode::Internal
        } else {
            LocationMfaMode::Disabled
        },
        service_location_mode: ServiceLocationMode::Disabled,
        mfa_method: None,
        posture_check_required: false,
    }
}

fn make_instance_details(
    name: &str,
    client_traffic_policy: ClientTrafficPolicy,
) -> InstanceDetails {
    InstanceDetails {
        name: name.to_string(),
        client_traffic_policy,
    }
}

#[test]
fn test_list_human_empty() {
    let result = LocationListResult {
        locations: Vec::new(),
        instance_details: HashMap::new(),
    };
    assert_eq!(
        result.human(),
        "No locations configured. Use the desktop app to enroll an instance first."
    );
}

#[test]
fn test_list_human_with_data() {
    let loc = make_location(1, 10, "office", "1.2.3.4:51820", false);
    let mut instance_details = HashMap::new();
    instance_details.insert(10, make_instance_details("acme", ClientTrafficPolicy::None));
    let result = LocationListResult {
        locations: vec![loc],
        instance_details,
    };
    let s = result.human();
    assert!(s.contains("ID"));
    assert!(s.contains("office"));
    assert!(s.contains("acme"));
    assert!(s.contains("1.2.3.4:51820"));
}

fn routing_column(route_all_traffic: bool, client_traffic_policy: ClientTrafficPolicy) -> String {
    let mut location = make_location(1, 10, "office", "1.2.3.4:51820", false);
    location.route_all_traffic = route_all_traffic;
    let mut instance_details = HashMap::new();
    instance_details.insert(10, make_instance_details("acme", client_traffic_policy));
    LocationListResult {
        locations: vec![location],
        instance_details,
    }
    .human()
}

fn char_col(line: &str, needle: &str) -> usize {
    let byte = line.find(needle).expect("the line holds the value");
    line[..byte].chars().count()
}

#[test]
fn test_list_human_columns_align_with_a_wide_mfa_cell() {
    // Exercise character width for "Kraków" and the wider "2 steps" MFA cell.
    let mut multistep = make_location(1, 10, "Kraków", "1.2.3.4:51820", true);
    multistep.mfa_steps = sqlx::types::Json(steps(&[
        &[LocationMfaMethod::Totp],
        &[LocationMfaMethod::Oidc],
    ]));
    let plain = make_location(2, 10, "office", "5.6.7.8:51820", false);
    let mut instance_details = HashMap::new();
    instance_details.insert(10, make_instance_details("acme", ClientTrafficPolicy::None));

    let table = LocationListResult {
        locations: vec![multistep, plain],
        instance_details,
    }
    .human();

    let lines: Vec<&str> = table.lines().collect();
    let routing_col = char_col(lines[0], "Routing");
    let mfa_col = char_col(lines[0], "MFA");
    for line in &lines[1..] {
        assert_eq!(char_col(line, "Predefined"), routing_col, "line: {line}");
    }
    assert_eq!(char_col(lines[1], "2 steps"), mfa_col);
    assert_eq!(char_col(lines[2], "none"), mfa_col);
}

#[test]
fn test_list_human_force_all_traffic_overrides_location() {
    let table = routing_column(false, ClientTrafficPolicy::ForceAllTraffic);
    assert!(table.contains("All-traffic"));
    assert!(!table.contains("Predefined"));
}

#[test]
fn test_list_human_disable_all_traffic_overrides_location() {
    let table = routing_column(true, ClientTrafficPolicy::DisableAllTraffic);
    assert!(table.contains("Predefined"));
    assert!(!table.contains("All-traffic"));
}

#[test]
fn test_list_human_no_policy_keeps_location_setting() {
    assert!(routing_column(true, ClientTrafficPolicy::None).contains("All-traffic"));
    assert!(routing_column(false, ClientTrafficPolicy::None).contains("Predefined"));
}

#[test]
fn test_list_json_empty() {
    let result = LocationListResult {
        locations: Vec::new(),
        instance_details: HashMap::new(),
    };
    let json = result.json();
    assert_eq!(json["locations"].as_array().unwrap().len(), 0);
    assert!(json["message"].is_null());
}

#[test]
fn test_list_json_with_data() {
    let loc = make_location(1, 10, "office", "1.2.3.4:51820", false);
    let mut instance_details = HashMap::new();
    instance_details.insert(10, make_instance_details("acme", ClientTrafficPolicy::None));
    let result = LocationListResult {
        locations: vec![loc],
        instance_details,
    };
    let json = result.json();
    let locations = json["locations"].as_array().unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0]["id"], 1);
    assert_eq!(locations[0]["name"], "office");
    assert_eq!(locations[0]["instance"], "acme");
}

#[test]
fn test_show_human() {
    let result = LocationShowResult {
        name: "office".to_string(),
        address: "10.0.0.0/24".to_string(),
        endpoint: "1.2.3.4:51820".to_string(),
        pubkey: "pk".to_string(),
        allowed_ips: "0.0.0.0/0".to_string(),
        dns: Some("8.8.8.8".to_string()),
        mfa_method: "totp".to_string(),
        mfa_steps: Vec::new(),
        mfa_step_plan: Vec::new(),
        route_all_traffic: false,
        keepalive_interval: 25,
    };
    let s = result.human();
    assert!(s.contains("Name:              office"));
    assert!(s.contains("Address:           10.0.0.0/24"));
    assert!(s.contains("DNS:               8.8.8.8"));
    assert!(s.contains("MFA method:        totp"));
}

#[test]
fn test_show_human_without_dns() {
    let result = LocationShowResult {
        name: "office".to_string(),
        address: "10.0.0.0/24".to_string(),
        endpoint: "1.2.3.4:51820".to_string(),
        pubkey: "pk".to_string(),
        allowed_ips: "0.0.0.0/0".to_string(),
        dns: None,
        mfa_method: "none".to_string(),
        mfa_steps: Vec::new(),
        mfa_step_plan: Vec::new(),
        route_all_traffic: true,
        keepalive_interval: 30,
    };
    let s = result.human();
    assert!(!s.contains("DNS"));
    assert!(s.contains("Route all traffic: true"));
}

#[test]
fn test_show_json() {
    let result = LocationShowResult {
        name: "office".to_string(),
        address: "10.0.0.0/24".to_string(),
        endpoint: "1.2.3.4:51820".to_string(),
        pubkey: "pk".to_string(),
        allowed_ips: "0.0.0.0/0".to_string(),
        dns: Some("8.8.8.8".to_string()),
        mfa_method: "totp".to_string(),
        mfa_steps: Vec::new(),
        mfa_step_plan: Vec::new(),
        route_all_traffic: false,
        keepalive_interval: 25,
    };
    let json = result.json();
    assert_eq!(json["name"], "office");
    assert_eq!(json["dns"], "8.8.8.8");
    assert_eq!(json["mfa_method"], "totp");
    assert!(json["message"].is_null());
}

#[test]
fn test_show_json_without_dns() {
    let result = LocationShowResult {
        name: "office".to_string(),
        address: "10.0.0.0/24".to_string(),
        endpoint: "1.2.3.4:51820".to_string(),
        pubkey: "pk".to_string(),
        allowed_ips: "0.0.0.0/0".to_string(),
        dns: None,
        mfa_method: "none".to_string(),
        mfa_steps: Vec::new(),
        mfa_step_plan: Vec::new(),
        route_all_traffic: true,
        keepalive_interval: 30,
    };
    let json = result.json();
    assert!(json["dns"].is_null());
}

#[test]
fn test_exit_code_zero() {
    assert_eq!(
        LocationListResult {
            locations: Vec::new(),
            instance_details: HashMap::new(),
        }
        .exit_code(),
        0
    );
    assert_eq!(
        LocationShowResult {
            name: "x".to_string(),
            address: "a".to_string(),
            endpoint: "e".to_string(),
            pubkey: "p".to_string(),
            allowed_ips: "0.0.0.0/0".to_string(),
            dns: None,
            mfa_method: "n".to_string(),
            mfa_steps: Vec::new(),
            mfa_step_plan: Vec::new(),
            route_all_traffic: false,
            keepalive_interval: 25,
        }
        .exit_code(),
        0
    );
    assert_eq!(
        LocationSetResult {
            name: "x".to_string(),
            changes: Vec::new(),
        }
        .exit_code(),
        0
    );
}

#[test]
fn test_set_human_no_changes() {
    let result = LocationSetResult {
        name: "office".to_string(),
        changes: Vec::new(),
    };
    assert_eq!(result.human(), "No changes for location 'office'.");
}

#[test]
fn test_set_human_with_changes() {
    let result = LocationSetResult {
        name: "office".to_string(),
        changes: vec![
            "MFA method → totp".to_string(),
            "route-all-traffic → on".to_string(),
        ],
    };
    let s = result.human();
    assert!(s.contains("Updated location 'office'"));
    assert!(s.contains("MFA method → totp"));
    assert!(s.contains("route-all-traffic → on"));
}

#[test]
fn test_set_json() {
    let result = LocationSetResult {
        name: "office".to_string(),
        changes: vec!["MFA method → totp".to_string()],
    };
    let json = result.json();
    assert_eq!(json["location"], "office");
    assert_eq!(json["changes"].as_array().unwrap().len(), 1);
}

#[test]
fn test_set_json_empty_changes() {
    let result = LocationSetResult {
        name: "office".to_string(),
        changes: Vec::new(),
    };
    let json = result.json();
    assert_eq!(json["location"], "office");
    assert_eq!(json["changes"].as_array().unwrap().len(), 0);
    assert!(json["message"].is_null());
}

fn steps(offered: &[&[LocationMfaMethod]]) -> Vec<LocationMfaStep> {
    offered
        .iter()
        .map(|methods| LocationMfaStep {
            methods: methods
                .iter()
                .map(|method| LocationMfaStepMethod {
                    method: *method,
                    configured: true,
                })
                .collect(),
        })
        .collect()
}

#[test]
fn test_parse_step_plan_ok() {
    let steps = steps(&[
        &[LocationMfaMethod::Email, LocationMfaMethod::Totp],
        &[LocationMfaMethod::Totp],
    ]);
    let plan =
        parse_step_plan("office", &["email".to_string(), "totp".to_string()], &steps).unwrap();
    assert_eq!(
        plan,
        vec![LocationMfaMethod::Email, LocationMfaMethod::Totp]
    );
}

#[test]
fn test_parse_step_plan_wrong_count_rejected() {
    let steps = steps(&[&[LocationMfaMethod::Totp], &[LocationMfaMethod::Email]]);
    let err = parse_step_plan("office", &["totp".to_string()], &steps).unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert!(err.to_string().contains("2 verification steps"));
}

#[test]
fn test_parse_step_plan_bad_name_rejected() {
    let steps = steps(&[&[LocationMfaMethod::Totp], &[LocationMfaMethod::Email]]);
    let err = parse_step_plan(
        "office",
        &["totp".to_string(), "smoke-signals".to_string()],
        &steps,
    )
    .unwrap_err();
    assert!(err.to_string().contains("smoke-signals"));
}

#[test]
fn test_parse_step_plan_method_absent_from_step_rejected() {
    let steps = steps(&[&[LocationMfaMethod::Totp], &[LocationMfaMethod::Email]]);
    let err =
        parse_step_plan("office", &["totp".to_string(), "oidc".to_string()], &steps).unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert!(err.to_string().contains("step 2"));
}

#[test]
fn test_parse_step_plan_keeps_fido2_for_the_desktop() {
    let steps = steps(&[&[LocationMfaMethod::Fido2]]);
    let plan = parse_step_plan("office", &["fido2".to_string()], &steps).unwrap();
    assert_eq!(plan, vec![LocationMfaMethod::Fido2]);
}

#[test]
fn test_location_mfa_label_reports_step_count() {
    let mut location = make_location(1, 1, "office", "1.2.3.4:51820", true);
    location.mfa_steps = sqlx::types::Json(steps(&[
        &[LocationMfaMethod::Totp],
        &[LocationMfaMethod::Oidc],
    ]));
    assert_eq!(location_mfa_label(&location), "2 steps");
}

#[test]
fn test_check_set_mfa_flags_rejects_method_on_multistep_location() {
    let err = check_set_mfa_flags(Some("totp"), &[], 2).unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert!(err.to_string().contains("--mfa-step"));
}

#[test]
fn test_check_set_mfa_flags_accepts_matching_shape() {
    assert!(check_set_mfa_flags(Some("totp"), &[], 1).is_ok());
    assert!(check_set_mfa_flags(Some("totp"), &[], 0).is_ok());
    assert!(check_set_mfa_flags(None, &["totp".to_string(), "email".to_string()], 2).is_ok());
    assert!(check_set_mfa_flags(None, &[], 2).is_ok());
}

#[test]
fn test_check_set_mfa_flags_rejects_steps_on_single_step_location() {
    let err = check_set_mfa_flags(None, &["totp".to_string()], 1).unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    let err = check_set_mfa_flags(Some("totp"), &["totp".to_string()], 2).unwrap_err();
    assert!(err.to_string().contains("conflicts"));
}
