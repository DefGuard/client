use sqlx::SqlitePool;
use tonic::Status;
use wiremock::ResponseTemplate;

use super::{attest_device, AttestationOutcome};
use defguard_client_core::database::models::device_identity::DeviceIdentity;
use defguard_client_proto::defguard::device_identity::v2::{
    attestation_challenge_solution::Result as Proof, AttestationChallengeSolution,
    IdentityErrorCode,
};

mod support;
use support::*;

#[sqlx::test(migrations = "../../migrations")]
async fn test_full_attestation_for_new_and_already_enrolled_users(pool: SqlitePool) {
    for (uuid, enrolled, seed) in [("new-user", false, 1), ("enrolled-user", true, 2)] {
        let registration = Registration::new(&pool, uuid, enrolled).await;
        let daemon = Daemon::new(
            &pool,
            registration.instance.id,
            seed,
            Ok(()),
            Ok(solution()),
        )
        .await;
        registration
            .expect_attestation(start_response(), solution(), verified())
            .await;
        assert_eq!(
            attest_device(
                &registration.session,
                &pool,
                registration.instance.id,
                daemon.client.clone()
            )
            .await,
            AttestationOutcome::Attested
        );
        let stored = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.core_device_id, 701);
        assert_eq!(stored.identity_id.as_deref(), Some("core-identity"));
        assert!(stored.material == material(seed));
        registration.assert_registration_retained(&pool).await;
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_unavailable_or_old_daemon_does_not_start_attestation(pool: SqlitePool) {
    for (uuid, error) in [
        ("unavailable", Status::unavailable("no TPM")),
        ("old-daemon", Status::unimplemented("old daemon")),
    ] {
        let registration = Registration::new(&pool, uuid, true).await;
        let daemon = Daemon::new(
            &pool,
            registration.instance.id,
            1,
            Err(error),
            Ok(solution()),
        )
        .await;
        assert_eq!(
            attest_device(
                &registration.session,
                &pool,
                registration.instance.id,
                daemon.client.clone()
            )
            .await,
            AttestationOutcome::Skipped
        );
        assert!(
            DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
                .await
                .unwrap()
                .is_none()
        );
        registration.assert_no_attestation().await;
        registration.assert_registration_retained(&pool).await;
        assert_eq!(daemon.counts(), (1, 0));
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_failed_material_persistence_prevents_start_and_preserves_device_config(
    pool: SqlitePool,
) {
    let registration = Registration::new(&pool, "write-failure", true).await;
    sqlx::query("CREATE TRIGGER fail_material_write BEFORE INSERT ON device_identity BEGIN SELECT RAISE(ABORT, 'simulated disk failure'); END")
        .execute(&pool).await.unwrap();
    let daemon = Daemon::new(&pool, registration.instance.id, 1, Ok(()), Ok(solution())).await;
    assert_eq!(
        attest_device(
            &registration.session,
            &pool,
            registration.instance.id,
            daemon.client.clone()
        )
        .await,
        AttestationOutcome::Failed
    );
    registration.assert_no_attestation().await;
    registration.assert_registration_retained(&pool).await;
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(daemon.counts(), (1, 0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_core_start_denial_retains_keys_without_invoking_the_tpm_solver(pool: SqlitePool) {
    for (uuid, status) in [("expired-session", 403), ("unimplemented-core", 501)] {
        let registration = Registration::new(&pool, uuid, true).await;
        registration
            .expect_start(ResponseTemplate::new(status))
            .await;
        let daemon = Daemon::new(&pool, registration.instance.id, 1, Ok(()), Ok(solution())).await;
        assert_eq!(
            attest_device(
                &registration.session,
                &pool,
                registration.instance.id,
                daemon.client.clone()
            )
            .await,
            AttestationOutcome::Failed
        );
        let saved = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .unwrap();
        assert!(saved.material == material(1));
        assert!(saved.identity_id.is_none());
        assert_eq!(daemon.counts(), (1, 0));
        registration.assert_registration_retained(&pool).await;
        assert!(registration
            .proxy
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| !request.url.path().ends_with("attestation/finish")));
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_backend_and_ipc_errors_after_start_are_reported_without_losing_keys(
    pool: SqlitePool,
) {
    for (uuid, proof, expected) in [
        (
            "lost-tpm-key",
            Ok(AttestationChallengeSolution {
                session_id: SESSION_ID.into(),
                result: Some(Proof::Error(IdentityErrorCode::IdentityLost.into())),
            }),
            IdentityErrorCode::IdentityLost,
        ),
        (
            "ipc-failure",
            Err(Status::unavailable(
                "private diagnostic must not enter HTTP",
            )),
            IdentityErrorCode::TemporarilyUnavailable,
        ),
    ] {
        let registration = Registration::new(&pool, uuid, true).await;
        let daemon = Daemon::new(&pool, registration.instance.id, 1, Ok(()), proof).await;
        let reported = AttestationChallengeSolution {
            session_id: SESSION_ID.into(),
            result: Some(Proof::Error(expected.into())),
        };
        // A success response cannot acknowledge an identity when the client sent no proof.
        registration
            .expect_attestation(start_response(), reported, verified())
            .await;
        assert_eq!(
            attest_device(
                &registration.session,
                &pool,
                registration.instance.id,
                daemon.client.clone()
            )
            .await,
            AttestationOutcome::Failed
        );
        let saved = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .unwrap();
        assert!(saved.material == material(1));
        assert!(saved.identity_id.is_none());
        registration.assert_registration_retained(&pool).await;
        assert_eq!(daemon.counts(), (1, 1));
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_failed_or_lost_finish_does_not_discard_or_automatically_replace_keys(
    pool: SqlitePool,
) {
    for (uuid, response) in [
        ("finish-http-error", ResponseTemplate::new(503)),
        (
            "lost-success-acknowledgement",
            ResponseTemplate::new(200).set_body_string("truncated success response"),
        ),
        (
            "missing-core-state",
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"identity_state": null})),
        ),
    ] {
        let registration = Registration::new(&pool, uuid, true).await;
        registration
            .expect_attestation(start_response(), solution(), response)
            .await;
        let daemon = Daemon::new(&pool, registration.instance.id, 1, Ok(()), Ok(solution())).await;
        assert_eq!(
            attest_device(
                &registration.session,
                &pool,
                registration.instance.id,
                daemon.client.clone()
            )
            .await,
            AttestationOutcome::Failed
        );
        let saved = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .unwrap();
        assert!(saved.material == material(1));
        assert!(saved.identity_id.is_none());
        assert_eq!(
            attest_device(
                &registration.session,
                &pool,
                registration.instance.id,
                daemon.client.clone()
            )
            .await,
            AttestationOutcome::Skipped
        );
        assert_eq!(daemon.counts(), (1, 1));
        let retained = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .unwrap();
        assert!(retained == saved);
        registration.assert_registration_retained(&pool).await;
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_successful_identity_is_not_recreated_or_reattested(pool: SqlitePool) {
    let registration = Registration::new(&pool, "no-reattest", true).await;
    registration
        .expect_attestation(start_response(), solution(), verified())
        .await;
    let daemon = Daemon::new(&pool, registration.instance.id, 1, Ok(()), Ok(solution())).await;
    assert_eq!(
        attest_device(
            &registration.session,
            &pool,
            registration.instance.id,
            daemon.client.clone()
        )
        .await,
        AttestationOutcome::Attested
    );
    let saved = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        attest_device(
            &registration.session,
            &pool,
            registration.instance.id,
            daemon.client.clone()
        )
        .await,
        AttestationOutcome::Skipped
    );
    assert_eq!(daemon.counts(), (1, 1));
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .unwrap()
            == saved
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_registration_binding_rejects_another_instance_before_ipc(pool: SqlitePool) {
    let first = Registration::new(&pool, "core-one", true).await;
    let second = Registration::new(&pool, "core-two", true).await;
    let first_daemon = Daemon::new(&pool, first.instance.id, 1, Ok(()), Ok(solution())).await;
    let second_daemon = Daemon::new(&pool, second.instance.id, 2, Ok(()), Ok(solution())).await;
    assert_eq!(
        attest_device(
            &first.session,
            &pool,
            second.instance.id,
            first_daemon.client.clone()
        )
        .await,
        AttestationOutcome::Failed
    );
    assert_eq!(first_daemon.counts(), (0, 0));
    first.assert_no_attestation().await;
    second.assert_no_attestation().await;
    for (registration, daemon, seed) in [(&first, &first_daemon, 1), (&second, &second_daemon, 2)] {
        registration
            .expect_attestation(start_response(), solution(), verified())
            .await;
        assert_eq!(
            attest_device(
                &registration.session,
                &pool,
                registration.instance.id,
                daemon.client.clone()
            )
            .await,
            AttestationOutcome::Attested
        );
        let saved = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.core_device_id, 701);
        assert!(saved.material == material(seed));
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_corrupt_identity_is_not_silently_regenerated(pool: SqlitePool) {
    let registration = Registration::new(&pool, "corrupt-material", true).await;
    DeviceIdentity::new(registration.instance.id, DEVICE_ID, material(1))
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE device_identity SET material = $1 WHERE instance_id = $2")
        .bind(vec![0xffu8])
        .bind(registration.instance.id)
        .execute(&pool)
        .await
        .unwrap();
    let daemon = Daemon::new(&pool, registration.instance.id, 2, Ok(()), Ok(solution())).await;
    assert_eq!(
        attest_device(
            &registration.session,
            &pool,
            registration.instance.id,
            daemon.client.clone()
        )
        .await,
        AttestationOutcome::Failed
    );
    assert_eq!(daemon.counts(), (0, 0));
    registration.assert_no_attestation().await;
    registration.assert_registration_retained(&pool).await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_failed_acknowledgement_write_retains_material_after_core_success(pool: SqlitePool) {
    let registration = Registration::new(&pool, "ack-write-failure", true).await;
    sqlx::query("CREATE TRIGGER fail_ack_write BEFORE UPDATE OF identity_id ON device_identity BEGIN SELECT RAISE(ABORT, 'simulated disk failure'); END")
        .execute(&pool).await.unwrap();
    registration
        .expect_attestation(start_response(), solution(), verified())
        .await;
    let daemon = Daemon::new(&pool, registration.instance.id, 1, Ok(()), Ok(solution())).await;
    assert_eq!(
        attest_device(
            &registration.session,
            &pool,
            registration.instance.id,
            daemon.client.clone()
        )
        .await,
        AttestationOutcome::Failed
    );
    let saved = DeviceIdentity::find_by_instance_id(&pool, registration.instance.id)
        .await
        .unwrap()
        .unwrap();
    assert!(saved.material == material(1));
    assert!(saved.identity_id.is_none());
    registration.assert_registration_retained(&pool).await;
}
