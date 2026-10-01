use defguard_client_proto::defguard::client_types::{
    mfa_flow_start_response, mfa_flow_step_finish_request, mfa_step_result, mfa_step_started,
    ClientMfaFinishRequest, MfaAdvanced, MfaAwaitingExternal, MfaCompleted, MfaFido2Assertion,
    MfaFido2Challenge, MfaFlowStartAccepted, MfaFlowStartRejected, MfaFlowStartResponse,
    MfaFlowStepFinishRequest, MfaFlowStepFinishResponse, MfaFlowStepStartResponse,
    MfaSignatureChallenge, MfaStepRejection, MfaStepResult, MfaStepStarted,
};
use reqwest::Url;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use wiremock::{
    matchers::{body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::*;
use crate::test_helpers::{start_ws_stub, WsStubCommand};

fn mock_url(server: &MockServer) -> Url {
    Url::parse(&server.uri()).expect("MockServer URI should be valid")
}

async fn mfa_start(proxy_url: Url, request: MfaStartRequest) -> Result<MfaStartResponse, MfaError> {
    super::mfa_start(MfaContract::Legacy, proxy_url, request).await
}

async fn mfa_finish_code(
    proxy_url: Url,
    request: ClientMfaFinishRequest,
) -> Result<MfaFinishResponse, MfaError> {
    super::mfa_finish(
        MfaContract::Legacy,
        proxy_url,
        MfaFinishRequest {
            token: request.token,
            step_attempt_id: None,
            submission: request.code.map(MfaSubmission::Code),
        },
    )
    .await
}

async fn poll_openid_mfa(
    proxy_url: Url,
    token: String,
    step_attempt_id: Option<String>,
    cancel: CancellationToken,
) -> Result<MfaFinishResponse, MfaError> {
    super::poll_openid_mfa(
        MfaContract::Legacy,
        proxy_url,
        token,
        step_attempt_id,
        cancel,
    )
    .await
}

fn derive_ws_url(proxy_base: &Url, token: &str) -> Result<String, MfaError> {
    super::derive_ws_url(MfaContract::Legacy, proxy_base, token, None)
}

fn start_request() -> MfaStartRequest {
    MfaStartRequest {
        location_id: 1,
        pubkey: "pk".into(),
        posture_data: None,
        selected_methods: vec![MfaMethod::Totp],
    }
}

fn start_response_json(token: &str) -> serde_json::Value {
    json!({
        "token": token,
        "challenge": null,
    })
}

fn finish_response_json(key: &str) -> serde_json::Value {
    json!({
        "preshared_key": key,
    })
}

fn finish_response_json_with_result(outcome: mfa_step_result::Outcome) -> serde_json::Value {
    serde_json::to_value(MfaFlowStepFinishResponse {
        result: Some(MfaStepResult {
            outcome: Some(outcome),
        }),
    })
    .expect("MFA flow finish response should serialize")
}

fn flow_start_response_json(token: &str, attempt_id: &str, challenge: &str) -> serde_json::Value {
    serde_json::to_value(MfaFlowStartResponse {
        outcome: Some(mfa_flow_start_response::Outcome::Accepted(
            MfaFlowStartAccepted {
                token: token.into(),
                first_step: Some(MfaStepStarted {
                    step_attempt_id: attempt_id.into(),
                    challenge: Some(mfa_step_started::Challenge::Signature(
                        MfaSignatureChallenge {
                            challenge: challenge.into(),
                        },
                    )),
                }),
            },
        )),
    })
    .expect("MFA flow start response should serialize")
}

fn flow_start_rejected_json(step: u32, reason: MfaStartRejectionReason) -> serde_json::Value {
    serde_json::to_value(MfaFlowStartResponse {
        outcome: Some(mfa_flow_start_response::Outcome::Rejected(
            MfaFlowStartRejected {
                rejections: vec![MfaStepRejection {
                    step,
                    reason: reason as i32,
                }],
            },
        )),
    })
    .expect("MFA flow rejection response should serialize")
}

fn mobile_result_frame(outcome: mfa_step_result::Outcome) -> String {
    serde_json::to_string(&json!({
        "type": "mfa_result",
        "result": MfaStepResult {
            outcome: Some(outcome),
        },
    }))
    .expect("MFA result frame should serialize")
}

#[tokio::test]
async fn test_mfa_start_success() {
    let server = MockServer::start().await;
    let body = start_response_json("mfa-token-1");

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let info = mfa_start(url, start_request()).await.unwrap();
    assert_eq!(info.token, "mfa-token-1");
    assert!(info.first_step.challenge.is_none());
}

#[tokio::test]
async fn test_mfa_start_selects_multi_step_route_from_contract() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/start"))
        .and(body_partial_json(json!({
            "location_id": 1,
            "pubkey": "pk",
            "selected_methods": [MfaMethod::Totp as i32],
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(flow_start_response_json(
                "flow-token",
                "attempt-1",
                "challenge-1",
            )),
        )
        .expect(1)
        .mount(&server)
        .await;

    let response = super::mfa_start(MfaContract::MultiStep, mock_url(&server), start_request())
        .await
        .unwrap();
    assert_eq!(response.token, "flow-token");
    assert_eq!(
        response.first_step.step_attempt_id.as_deref(),
        Some("attempt-1")
    );
    assert_eq!(
        response.first_step.challenge.as_deref(),
        Some("challenge-1")
    );
    server.verify().await;
}

#[tokio::test]
async fn test_mfa_step_start_returns_typed_fido2_challenge() {
    let server = MockServer::start().await;
    let body = serde_json::to_value(MfaFlowStepStartResponse {
        started: Some(MfaStepStarted {
            step_attempt_id: "attempt-2".into(),
            challenge: Some(mfa_step_started::Challenge::Fido2(MfaFido2Challenge {
                challenge: "fido-challenge".into(),
                credential_ids: vec!["credential-1".into()],
            })),
        }),
    })
    .unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-start"))
        .and(body_partial_json(json!({
            "token": "flow-token",
            "method": MfaMethod::Fido2 as i32,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let response = super::mfa_step_start(
        MfaContract::MultiStep,
        mock_url(&server),
        "flow-token".into(),
        MfaMethod::Fido2,
    )
    .await
    .unwrap();
    assert_eq!(response.step_attempt_id.as_deref(), Some("attempt-2"));
    assert_eq!(response.challenge.as_deref(), Some("fido-challenge"));
    assert_eq!(response.credential_ids, vec!["credential-1"]);
}

#[tokio::test]
async fn test_mfa_start_rejected() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "error": "unauthorized" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::MfaRejected { .. }));
}

#[tokio::test]
async fn test_mfa_start_attempt_limit_on_403() {
    let server = MockServer::start().await;
    let message = "Too many failed MFA attempts. Please try connecting again.";

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({ "error": message })))
        .mount(&server)
        .await;

    let err = mfa_start(mock_url(&server), start_request())
        .await
        .unwrap_err();
    match err {
        MfaError::AttemptLimit { message: actual } => assert_eq!(actual, message),
        other => panic!("expected AttemptLimit, got {other:?}"),
    }
}

#[tokio::test]
async fn test_mfa_start_posture_rejected_on_403() {
    // A non-cap 403 must remain a dedicated posture rejection.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(json!({ "error": "firewall enabled" })),
        )
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::PostureRejected { .. }));
}

#[tokio::test]
async fn test_mfa_start_sends_snake_case_numeric_body() {
    // Guards the wire contract: the proxy expects snake_case fields and a
    // *numeric* `method`. If serde ever serialized camelCase or a string
    // enum, the body matcher fails, the mock returns nothing, and the call
    // errors instead of silently sending a malformed request.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .and(body_partial_json(
            json!({ "location_id": 1, "pubkey": "pk", "method": 0 }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json("t")))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    mfa_start(url, start_request())
        .await
        .expect("request body did not match the expected wire contract");
}

#[tokio::test]
async fn test_mfa_start_network_error() {
    // Nothing listening on this port.
    let url = "http://127.0.0.1:1".parse().unwrap();
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::NetworkError { .. }));
}

#[tokio::test]
async fn test_mfa_start_proxy_error_on_5xx() {
    // 5xx is a server fault (ProxyError), distinct from a 4xx rejection.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({ "error": "boom" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::ProxyError { status: 500, .. }));
}

#[tokio::test]
async fn test_mfa_start_mobile_no_authenticator_guidance() {
    // Mobile-approve start rejected because no authenticator is registered:
    // the generic proxy message becomes actionable guidance.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(json!({ "error": "selected MFA method is not available" })),
        )
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let request = MfaStartRequest {
        selected_methods: vec![MfaMethod::MobileApprove],
        ..start_request()
    };
    match mfa_start(url, request).await.unwrap_err() {
        MfaError::MfaRejected { message } => {
            assert!(
                message.contains("mobile app"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected MfaRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn test_mfa_start_non_mobile_not_rewrapped() {
    // The mobile guidance must not leak into other methods.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(json!({ "error": "selected MFA method is not available" })),
        )
        .mount(&server)
        .await;

    let url = mock_url(&server);
    // start_request() uses method 0 (TOTP).
    match mfa_start(url, start_request()).await.unwrap_err() {
        MfaError::MfaRejected { message } => {
            assert!(
                !message.contains("mobile app"),
                "TOTP got mobile guidance: {message}"
            );
        }
        other => panic!("expected MfaRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn test_mfa_start_sends_frozen_legacy_request_fields() {
    // The legacy wire request stays frozen while the flow route uses the new message.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .and(body_partial_json(json!({
            "method": MfaMethod::Totp as i32,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json("t")))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let request = start_request();
    mfa_start(url, request)
        .await
        .expect("request body did not match the expected wire contract");
}

#[tokio::test]
async fn test_mfa_flow_rejection_keeps_mobile_guidance() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/start"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(flow_start_rejected_json(
                0,
                MfaStartRejectionReason::MfaStartRejectionStepUnavailable,
            )),
        )
        .mount(&server)
        .await;

    let request = MfaStartRequest {
        selected_methods: vec![MfaMethod::MobileApprove],
        ..start_request()
    };
    match super::mfa_start(MfaContract::MultiStep, mock_url(&server), request)
        .await
        .unwrap_err()
    {
        MfaError::MfaRejected { message } => {
            assert!(
                message.contains("mobile app"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected MfaRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn test_mfa_flow_rejection_non_mobile_step_stays_generic() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/start"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(flow_start_rejected_json(
                0,
                MfaStartRejectionReason::MfaStartRejectionStepUnavailable,
            )),
        )
        .mount(&server)
        .await;

    let request = start_request();
    match super::mfa_start(MfaContract::MultiStep, mock_url(&server), request)
        .await
        .unwrap_err()
    {
        MfaError::MfaRejected { message } => {
            assert!(
                !message.contains("mobile app"),
                "TOTP got mobile guidance: {message}"
            );
        }
        other => panic!("expected MfaRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn test_mfa_finish_code_success() {
    let server = MockServer::start().await;
    let body = finish_response_json("psk-123");

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .and(body_partial_json(
            json!({"token": "token", "code": "123456"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let psk = mfa_finish_code(
        url,
        ClientMfaFinishRequest {
            token: "token".into(),
            code: Some("123456".into()),
            auth_pub_key: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(psk.preshared_key, "psk-123");
}

#[tokio::test]
async fn test_mfa_finish_code_rejected() {
    // A wrong code is a 4xx rejection.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "error": "Unauthorized" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_finish_code(
        url,
        ClientMfaFinishRequest {
            token: "token".into(),
            code: Some("000000".into()),
            auth_pub_key: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, MfaError::MfaRejected { .. }));
}

#[tokio::test]
async fn test_mfa_finish_multi_step_sends_typed_code() {
    let server = MockServer::start().await;
    let body = serde_json::to_value(MfaFlowStepFinishRequest {
        token: "flow-token".into(),
        step_attempt_id: "attempt-1".into(),
        submission: Some(mfa_flow_step_finish_request::Submission::Code(
            defguard_client_proto::defguard::client_types::MfaCodeCredential {
                code: "123456".into(),
            },
        )),
    })
    .unwrap();
    let response =
        finish_response_json_with_result(mfa_step_result::Outcome::Completed(MfaCompleted {
            preshared_key: "psk".into(),
        }));

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-finish"))
        .and(body_partial_json(body))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let response = super::mfa_finish(
        MfaContract::MultiStep,
        mock_url(&server),
        MfaFinishRequest {
            token: "flow-token".into(),
            step_attempt_id: Some("attempt-1".into()),
            submission: Some(MfaSubmission::Code("123456".into())),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        response.result.and_then(|result| result.outcome),
        Some(mfa_step_result::Outcome::Completed(completed)) if completed.preshared_key == "psk"
    ));
}

#[tokio::test]
async fn test_mfa_finish_multi_step_sends_typed_fido2_assertion() {
    let server = MockServer::start().await;
    let assertion = MfaFido2Assertion {
        rp_id_hash: vec![1, 2],
        authenticator_data: vec![3, 4],
        signature: vec![5, 6],
        credential_id: vec![7, 8],
    };
    let body = serde_json::to_value(MfaFlowStepFinishRequest {
        token: "flow-token".into(),
        step_attempt_id: "attempt-2".into(),
        submission: Some(mfa_flow_step_finish_request::Submission::Fido2(
            MfaFido2Assertion {
                rp_id_hash: assertion.rp_id_hash.clone(),
                authenticator_data: assertion.authenticator_data.clone(),
                signature: assertion.signature.clone(),
                credential_id: assertion.credential_id.clone(),
            },
        )),
    })
    .unwrap();
    let response =
        finish_response_json_with_result(mfa_step_result::Outcome::Completed(MfaCompleted {
            preshared_key: "psk".into(),
        }));

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-finish"))
        .and(body_partial_json(body))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    super::mfa_finish(
        MfaContract::MultiStep,
        mock_url(&server),
        MfaFinishRequest {
            token: "flow-token".into(),
            step_attempt_id: Some("attempt-2".into()),
            submission: Some(MfaSubmission::Fido2(assertion)),
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_mfa_finish_multi_step_rejects_missing_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let err = super::mfa_finish(
        MfaContract::MultiStep,
        mock_url(&server),
        MfaFinishRequest {
            token: "flow-token".into(),
            step_attempt_id: Some("attempt-1".into()),
            submission: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        MfaError::Other { message } if message.contains("did not include a result")
    ));
}

#[tokio::test]
async fn test_poll_openid_multi_step_advanced_returns_result() {
    let server = MockServer::start().await;
    let body = finish_response_json_with_result(mfa_step_result::Outcome::Advanced(MfaAdvanced {
        next_step: 1,
    }));

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let response = super::poll_openid_mfa(
        MfaContract::MultiStep,
        mock_url(&server),
        "token".into(),
        Some("attempt-1".into()),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert!(response.preshared_key.is_empty());
    assert!(matches!(
        response.result,
        Some(MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Advanced(advanced)),
        }) if advanced.next_step == 1
    ));
}

#[tokio::test]
async fn test_poll_openid_awaiting_external_then_completed() {
    let server = MockServer::start().await;
    let awaiting_body = finish_response_json_with_result(
        mfa_step_result::Outcome::AwaitingExternal(MfaAwaitingExternal {}),
    );
    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&awaiting_body))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let completed_body =
        finish_response_json_with_result(mfa_step_result::Outcome::Completed(MfaCompleted {
            preshared_key: "oidc-psk".into(),
        }));
    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&completed_body))
        .mount(&server)
        .await;

    let response = super::poll_openid_mfa(
        MfaContract::MultiStep,
        mock_url(&server),
        "token".into(),
        Some("attempt-1".into()),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert!(matches!(
        response.result,
        Some(MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Completed(completed)),
        }) if completed.preshared_key == "oidc-psk"
    ));
}

#[tokio::test]
async fn test_poll_openid_sends_step_attempt_id() {
    let server = MockServer::start().await;
    let body =
        finish_response_json_with_result(mfa_step_result::Outcome::Completed(MfaCompleted {
            preshared_key: "oidc-psk".into(),
        }));

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/step-finish"))
        .and(body_partial_json(json!({
            "step_attempt_id": "attempt-123",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let response = super::poll_openid_mfa(
        MfaContract::MultiStep,
        mock_url(&server),
        "token".into(),
        Some("attempt-123".into()),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert!(matches!(
        response.result,
        Some(MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Completed(completed)),
        }) if completed.preshared_key == "oidc-psk"
    ));
}

#[tokio::test]
async fn test_poll_openid_success() {
    let server = MockServer::start().await;
    let body = finish_response_json("oidc-psk");

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let psk = poll_openid_mfa(url, "token".into(), None, cancel)
        .await
        .unwrap();
    assert_eq!(psk.preshared_key, "oidc-psk");
}

#[tokio::test]
async fn test_poll_openid_428_then_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(428))
        .up_to_n_times(2)
        .mount(&server)
        .await;

    let success_body = finish_response_json("oidc-psk");
    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&success_body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let psk = poll_openid_mfa(url, "token".into(), None, cancel)
        .await
        .unwrap();
    assert_eq!(psk.preshared_key, "oidc-psk");
}

#[tokio::test]
async fn test_poll_openid_stops_on_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({ "error": "boom" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let err = poll_openid_mfa(url, "token".into(), None, cancel)
        .await
        .unwrap_err();
    match err {
        MfaError::ProxyError { status, message } => {
            assert_eq!(status, 500);
            assert!(message.contains("boom"));
        }
        other => panic!("expected ProxyError, got {other:?}"),
    }
}

#[tokio::test]
async fn test_poll_openid_timeout() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(428))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let err = poll_openid_mfa(url, "token".into(), None, cancel)
        .await
        .unwrap_err();
    assert!(matches!(err, MfaError::Timeout));
}

#[tokio::test]
async fn test_poll_openid_cancelled() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(428))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let err = poll_openid_mfa(url, "token".into(), None, cancel)
        .await
        .unwrap_err();
    assert!(matches!(err, MfaError::Cancelled));
}

#[tokio::test]
async fn test_mobile_approve_advanced_result() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle = tokio::spawn(async move {
        connect_mobile_approve(MfaContract::MultiStep, &ws_url, cancel).await
    });

    tx.send(WsStubCommand::SendMessage(mobile_result_frame(
        mfa_step_result::Outcome::Advanced(MfaAdvanced { next_step: 1 }),
    )))
    .unwrap();

    let response = handle.await.unwrap().unwrap();
    assert!(response.preshared_key.is_empty());
    assert!(matches!(
        response.result,
        Some(MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Advanced(advanced)),
        }) if advanced.next_step == 1
    ));
}

#[tokio::test]
async fn test_mobile_approve_completed_result_uses_nested_key() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle = tokio::spawn(async move {
        connect_mobile_approve(MfaContract::MultiStep, &ws_url, cancel).await
    });

    tx.send(WsStubCommand::SendMessage(mobile_result_frame(
        mfa_step_result::Outcome::Completed(MfaCompleted {
            preshared_key: "mobile-psk".into(),
        }),
    )))
    .unwrap();

    let response = handle.await.unwrap().unwrap();
    assert!(response.preshared_key.is_empty());
    assert!(matches!(
        response.result,
        Some(MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Completed(completed)),
        }) if completed.preshared_key == "mobile-psk"
    ));
}

#[tokio::test]
async fn test_mobile_approve_empty_legacy_key_is_rejected() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle =
        tokio::spawn(
            async move { connect_mobile_approve(MfaContract::Legacy, &ws_url, cancel).await },
        );

    tx.send(WsStubCommand::SendMessage(
        r#"{"type":"mfa_success","preshared_key":""}"#.into(),
    ))
    .unwrap();

    let err = handle.await.unwrap().unwrap_err();
    assert!(matches!(
        err,
        MfaError::MfaRejected { message } if message.contains("empty preshared key")
    ));
}

#[tokio::test]
async fn test_mobile_approve_success() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle =
        tokio::spawn(
            async move { connect_mobile_approve(MfaContract::Legacy, &ws_url, cancel).await },
        );

    tx.send(WsStubCommand::SendMessage(
        r#"{"type":"mfa_success","preshared_key":"mobile-psk"}"#.into(),
    ))
    .unwrap();
    tx.send(WsStubCommand::Close).unwrap();

    let psk = handle.await.unwrap().unwrap();
    assert_eq!(psk.preshared_key, "mobile-psk");
}

#[tokio::test]
async fn test_mobile_approve_close_without_success() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle =
        tokio::spawn(
            async move { connect_mobile_approve(MfaContract::Legacy, &ws_url, cancel).await },
        );

    tx.send(WsStubCommand::Close).unwrap();

    let err = handle.await.unwrap().unwrap_err();
    assert!(matches!(err, MfaError::MfaRejected { .. }));
}

/// Build an `mfa_result` frame for the WebSocket stub.
fn mfa_result_frame(result: &MfaStepResult) -> String {
    serde_json::to_string(&json!({ "type": "mfa_result", "result": result }))
        .expect("frame serializes")
}

async fn multi_step_mobile_approve_error(command: WsStubCommand) -> MfaError {
    let stub = start_ws_stub().await;
    let ws_url = format!("ws://{}/test", stub.addr);
    let handle = tokio::spawn(async move {
        connect_mobile_approve(MfaContract::MultiStep, &ws_url, CancellationToken::new()).await
    });

    stub.tx.send(command).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), handle)
        .await
        .expect("invalid remote result should fail promptly")
        .unwrap()
        .unwrap_err()
}

#[tokio::test]
async fn test_mobile_approve_advanced_result_is_a_passed_step() {
    // An intermediate step returns no preshared key.
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle = tokio::spawn(async move {
        connect_mobile_approve(MfaContract::MultiStep, &ws_url, cancel).await
    });

    tx.send(WsStubCommand::SendMessage(mfa_result_frame(
        &MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Advanced(MfaAdvanced {
                next_step: 1,
            })),
        },
    )))
    .unwrap();
    tx.send(WsStubCommand::Close).unwrap();

    let response = handle.await.unwrap().unwrap();
    assert!(completed_preshared_key(&response).is_none());
}

#[tokio::test]
async fn test_mobile_approve_completed_result_carries_the_key() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle = tokio::spawn(async move {
        connect_mobile_approve(MfaContract::MultiStep, &ws_url, cancel).await
    });

    tx.send(WsStubCommand::SendMessage(mfa_result_frame(
        &MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Completed(MfaCompleted {
                preshared_key: "mobile-psk".into(),
            })),
        },
    )))
    .unwrap();
    tx.send(WsStubCommand::Close).unwrap();

    let response = handle.await.unwrap().unwrap();
    assert_eq!(
        completed_preshared_key(&response).as_deref(),
        Some("mobile-psk")
    );
}

#[tokio::test]
async fn test_mobile_approve_multi_step_close_without_result_fails_promptly() {
    assert!(matches!(
        multi_step_mobile_approve_error(WsStubCommand::Close).await,
        MfaError::MfaRejected { .. }
    ));
}

#[tokio::test]
async fn test_mobile_approve_multi_step_rejects_legacy_frame() {
    assert!(matches!(
        multi_step_mobile_approve_error(WsStubCommand::SendMessage(
            r#"{"type":"mfa_success","preshared_key":"legacy-key"}"#.into()
        ))
        .await,
        MfaError::Other { .. }
    ));
}

#[tokio::test]
async fn test_mobile_approve_multi_step_rejects_missing_outcome() {
    assert!(matches!(
        multi_step_mobile_approve_error(WsStubCommand::SendMessage(
            r#"{"type":"mfa_result","result":{}}"#.into()
        ))
        .await,
        MfaError::Other { .. }
    ));
}

#[tokio::test]
async fn test_mobile_approve_legacy_ignores_multi_step_frame() {
    let stub = start_ws_stub().await;
    let ws_url = format!("ws://{}/test", stub.addr);
    let handle = tokio::spawn(async move {
        connect_mobile_approve(MfaContract::Legacy, &ws_url, CancellationToken::new()).await
    });

    stub.tx
        .send(WsStubCommand::SendMessage(mfa_result_frame(
            &MfaStepResult {
                outcome: Some(mfa_step_result::Outcome::Advanced(MfaAdvanced {
                    next_step: 1,
                })),
            },
        )))
        .unwrap();
    stub.tx.send(WsStubCommand::Close).unwrap();

    let err = tokio::time::timeout(std::time::Duration::from_secs(1), handle)
        .await
        .expect("legacy decoder should ignore the other contract's frame")
        .unwrap()
        .unwrap_err();
    assert!(matches!(err, MfaError::MfaRejected { .. }));
}

#[tokio::test]
async fn test_mobile_approve_multi_step_rejects_awaiting_external() {
    assert!(matches!(
        multi_step_mobile_approve_error(WsStubCommand::SendMessage(mfa_result_frame(
            &MfaStepResult {
                outcome: Some(mfa_step_result::Outcome::AwaitingExternal(
                    MfaAwaitingExternal {},
                )),
            },
        )))
        .await,
        MfaError::Other { .. }
    ));
}

#[tokio::test]
async fn test_mobile_approve_multi_step_rejects_empty_completed_key() {
    assert!(matches!(
        multi_step_mobile_approve_error(WsStubCommand::SendMessage(mfa_result_frame(
            &MfaStepResult {
                outcome: Some(mfa_step_result::Outcome::Completed(MfaCompleted {
                    preshared_key: String::new(),
                })),
            },
        )))
        .await,
        MfaError::MfaRejected { message } if message.contains("empty preshared key")
    ));
}

#[tokio::test]
async fn test_mobile_approve_close_frame_reaches_the_error() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle =
        tokio::spawn(
            async move { connect_mobile_approve(MfaContract::Legacy, &ws_url, cancel).await },
        );

    tx.send(WsStubCommand::CloseWith(4001, "unknown mfa token".into()))
        .unwrap();

    let err = handle.await.unwrap().unwrap_err();
    let MfaError::MfaRejected { message } = err else {
        panic!("expected MfaRejected, got {err:?}");
    };
    assert!(message.contains("4001"), "{message}");
    assert!(message.contains("unknown mfa token"), "{message}");
}

#[tokio::test]
async fn test_mobile_approve_cancelled() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    cancel.cancel();
    let err = connect_mobile_approve(MfaContract::Legacy, &ws_url, cancel)
        .await
        .unwrap_err();
    assert!(matches!(err, MfaError::Cancelled));
}

#[tokio::test]
async fn test_mobile_approve_connect_error_does_not_leak_token() {
    // Nothing is listening, so the WebSocket connect fails. The error must
    // be a NetworkError whose message never contains the MFA token (the
    // token rides in the ws_url query string).
    let base: Url = "http://127.0.0.1:1".parse().unwrap();
    let token = "super-secret-mfa-token";
    let ws_url = derive_ws_url(&base, token).unwrap();

    let cancel = CancellationToken::new();
    let err = connect_mobile_approve(MfaContract::Legacy, &ws_url, cancel)
        .await
        .unwrap_err();

    assert!(matches!(err, MfaError::NetworkError { .. }));
    assert!(
        !err.to_string().contains(token),
        "error leaked the MFA token: {err}"
    );
}

#[test]
fn test_derive_ws_url_http_to_ws() {
    let base = Url::parse("http://proxy.example.com/").unwrap();
    let ws = derive_ws_url(&base, "tok").unwrap();
    assert!(ws.starts_with("ws://proxy.example.com/api/v1/client-mfa/remote"));
    assert!(ws.contains("token=tok"));
}

#[test]
fn test_derive_ws_url_https_to_wss() {
    let base = Url::parse("https://proxy.example.com/").unwrap();
    let ws = derive_ws_url(&base, "tok").unwrap();
    assert!(ws.starts_with("wss://proxy.example.com/api/v1/client-mfa/remote"));
}

#[test]
fn test_derive_ws_url_preserves_path_prefix() {
    let base = Url::parse("https://proxy.example.com/defguard/").unwrap();
    let ws = derive_ws_url(&base, "tok").unwrap();
    assert!(ws.starts_with("wss://proxy.example.com/defguard/api/v1/client-mfa/remote"));
}

#[test]
fn test_derive_ws_url_multi_step_includes_attempt_id() {
    let base = Url::parse("https://proxy.example.com/").unwrap();
    let ws = super::derive_ws_url(MfaContract::MultiStep, &base, "tok", Some("attempt-1")).unwrap();
    let ws = Url::parse(&ws).unwrap();
    assert_eq!(ws.scheme(), "wss");
    assert_eq!(ws.path(), "/api/v1/mfa-flow/remote");
    let query = ws.query_pairs().collect::<Vec<_>>();
    assert_eq!(query[0], ("token".into(), "tok".into()));
    assert_eq!(query[1], ("step_attempt_id".into(), "attempt-1".into()));
}

#[test]
fn test_derive_ws_url_multi_step_requires_attempt_id() {
    let base = Url::parse("https://proxy.example.com/").unwrap();
    assert!(super::derive_ws_url(MfaContract::MultiStep, &base, "tok", None).is_err());
}

#[test]
fn test_derive_ws_url_rejects_non_http_scheme() {
    let base = Url::parse("ftp://proxy.example.com/").unwrap();
    let err = derive_ws_url(&base, "tok").unwrap_err();
    assert!(matches!(err, MfaError::Other { .. }));
}

#[tokio::test]
async fn test_mfa_flow_start_reads_fido2_credential_ids() {
    let server = MockServer::start().await;
    let body = serde_json::to_value(MfaFlowStartResponse {
        outcome: Some(mfa_flow_start_response::Outcome::Accepted(
            MfaFlowStartAccepted {
                token: "fido2-token".into(),
                first_step: Some(MfaStepStarted {
                    step_attempt_id: "attempt-1".into(),
                    challenge: Some(mfa_step_started::Challenge::Fido2(MfaFido2Challenge {
                        challenge: "chal".into(),
                        credential_ids: vec!["a-b_c".into(), "ZmlkbzI".into()],
                    })),
                }),
            },
        )),
    })
    .unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/mfa-flow/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let info = super::mfa_start(MfaContract::MultiStep, mock_url(&server), start_request())
        .await
        .unwrap();
    assert_eq!(info.first_step.challenge.as_deref(), Some("chal"));
    assert_eq!(info.first_step.credential_ids, vec!["a-b_c", "ZmlkbzI"]);
}

fn finish_response(preshared_key: &str, result: Option<MfaStepResult>) -> MfaFinishResponse {
    MfaFinishResponse {
        preshared_key: preshared_key.into(),
        result,
    }
}

#[test]
fn test_completed_preshared_key_reads_the_legacy_field() {
    let response = finish_response("psk", None);
    assert_eq!(completed_preshared_key(&response).as_deref(), Some("psk"));
}

#[test]
fn test_completed_preshared_key_rejects_an_empty_legacy_field() {
    // An empty legacy key represents an incomplete intermediate step.
    let response = finish_response("", None);
    assert!(completed_preshared_key(&response).is_none());
}

#[test]
fn test_completed_preshared_key_reads_the_completed_outcome() {
    let response = finish_response(
        "",
        Some(MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Completed(MfaCompleted {
                preshared_key: "psk".into(),
            })),
        }),
    );
    assert_eq!(completed_preshared_key(&response).as_deref(), Some("psk"));
}

#[test]
fn test_completed_preshared_key_rejects_an_advanced_outcome() {
    let response = finish_response(
        "leftover",
        Some(MfaStepResult {
            outcome: Some(mfa_step_result::Outcome::Advanced(MfaAdvanced {
                next_step: 1,
            })),
        }),
    );
    assert!(completed_preshared_key(&response).is_none());
}
