use defguard_client_proto::defguard::{
    client::v1::{TpmEndorsementKeyMaterial, TpmIdentityMaterial, TpmKeyBlobs},
    client_types::InstanceInfo,
};
use sqlx::SqlitePool;

use super::*;
use crate::database::models::{instance::Instance, NoId};

async fn instance(pool: &SqlitePool, uuid: &str) -> Instance<Id> {
    Instance::<NoId>::from(InstanceInfo {
        id: uuid.into(),
        name: uuid.into(),
        url: "https://core.example".into(),
        proxy_url: "https://proxy.example".into(),
        username: "alice".into(),
        ..Default::default()
    })
    .save(pool)
    .await
    .unwrap()
}

fn material(seed: u8) -> DeviceIdentityMaterial {
    DeviceIdentityMaterial {
        backend: Some(device_identity_material::Backend::Tpm(
            TpmIdentityMaterial {
                ek: Some(TpmEndorsementKeyMaterial {
                    template: vec![seed, 0],
                    public: vec![seed, 1],
                    certificate_chain_der: vec![vec![seed, 2], vec![seed, 3]],
                }),
                srk_template: vec![seed, 4],
                ak: Some(TpmKeyBlobs {
                    public: vec![seed, 5],
                    private: vec![seed, 6],
                }),
                identity_key: Some(TpmKeyBlobs {
                    public: vec![seed, 7],
                    private: vec![seed, 8],
                }),
            },
        )),
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn test_identity_material_survives_reopening_the_database(pool: SqlitePool) {
    let instance = instance(&pool, "persistent").await;
    let identity = DeviceIdentity::new(instance.id, 42, material(1)).unwrap();
    identity.save(&pool).await.unwrap();
    let options = pool.connect_options().as_ref().clone();
    pool.close().await;
    let reopened = SqlitePool::connect_with(options).await.unwrap();
    let stored = DeviceIdentity::find_by_instance_id(&reopened, instance.id)
        .await
        .unwrap()
        .unwrap();
    assert!(stored == identity);
    assert!(stored.identity_id.is_none());
    reopened.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn test_registrations_keep_independent_keys_even_with_the_same_core_device_id(
    pool: SqlitePool,
) {
    let first = instance(&pool, "core-one").await;
    let second = instance(&pool, "core-two").await;
    let first_identity = DeviceIdentity::new(first.id, 42, material(1)).unwrap();
    let second_identity = DeviceIdentity::new(second.id, 42, material(2)).unwrap();
    first_identity.save(&pool).await.unwrap();
    second_identity.save(&pool).await.unwrap();
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, first.id)
            .await
            .unwrap()
            == Some(first_identity)
    );
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, second.id)
            .await
            .unwrap()
            == Some(second_identity)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn test_saving_new_material_cannot_replace_an_existing_identity(pool: SqlitePool) {
    let instance = instance(&pool, "no-key-replacement").await;
    let identity = DeviceIdentity::new(instance.id, 42, material(1)).unwrap();
    identity.save(&pool).await.unwrap();
    let replacement = DeviceIdentity::new(instance.id, 43, material(2)).unwrap();
    assert!(
        matches!(replacement.save(&pool).await, Err(DeviceIdentityError::Database(sqlx::Error::Database(error))) if error.is_unique_violation())
    );
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, instance.id)
            .await
            .unwrap()
            == Some(identity)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn test_core_acknowledgement_is_bound_to_saved_material_and_registration(pool: SqlitePool) {
    let instance = instance(&pool, "acknowledgement").await;
    let mut identity = DeviceIdentity::new(instance.id, 42, material(1)).unwrap();
    identity.save(&pool).await.unwrap();
    let mut wrong_device = identity.clone();
    wrong_device.core_device_id = 43;
    assert!(matches!(
        wrong_device.record_identity_id(&pool, "wrong-device").await,
        Err(DeviceIdentityError::RegistrationChanged)
    ));
    let mut wrong_key = DeviceIdentity::new(instance.id, 42, material(2)).unwrap();
    assert!(matches!(
        wrong_key.record_identity_id(&pool, "wrong-key").await,
        Err(DeviceIdentityError::RegistrationChanged)
    ));
    assert!(matches!(
        identity.record_identity_id(&pool, "").await,
        Err(DeviceIdentityError::InvalidState(_))
    ));
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, instance.id)
            .await
            .unwrap()
            == Some(identity.clone())
    );
    identity
        .record_identity_id(&pool, "core-identity")
        .await
        .unwrap();
    identity
        .record_identity_id(&pool, "core-identity")
        .await
        .unwrap();
    assert!(matches!(
        identity.record_identity_id(&pool, "another-identity").await,
        Err(DeviceIdentityError::RegistrationChanged)
    ));
    assert_eq!(identity.identity_id.as_deref(), Some("core-identity"));
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, instance.id)
            .await
            .unwrap()
            == Some(identity)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn test_deleting_an_instance_removes_only_its_identity(pool: SqlitePool) {
    let first = instance(&pool, "deleted").await;
    let second = instance(&pool, "retained").await;
    let identity = DeviceIdentity::new(first.id, 42, material(1)).unwrap();
    let other = DeviceIdentity::new(second.id, 43, material(2)).unwrap();
    identity.save(&pool).await.unwrap();
    other.save(&pool).await.unwrap();
    first.delete(&pool).await.unwrap();
    assert!(DeviceIdentity::find_by_instance_id(&pool, first.id)
        .await
        .unwrap()
        .is_none());
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, second.id)
            .await
            .unwrap()
            == Some(other)
    );
    let mut stale = identity;
    assert!(matches!(
        stale.record_identity_id(&pool, "deleted-identity").await,
        Err(DeviceIdentityError::RegistrationChanged)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn test_identity_writes_participate_in_the_callers_transaction(pool: SqlitePool) {
    let instance = instance(&pool, "transaction").await;
    let mut identity = DeviceIdentity::new(instance.id, 42, material(1)).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    identity.save(&mut *transaction).await.unwrap();
    transaction.rollback().await.unwrap();
    assert!(DeviceIdentity::find_by_instance_id(&pool, instance.id)
        .await
        .unwrap()
        .is_none());
    identity.save(&pool).await.unwrap();
    let mut transaction = pool.begin().await.unwrap();
    identity
        .record_identity_id(&mut *transaction, "rolled-back")
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    let stored = DeviceIdentity::find_by_instance_id(&pool, instance.id)
        .await
        .unwrap()
        .unwrap();
    assert!(stored.identity_id.is_none());
    assert!(stored.material == identity.material);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_corrupt_or_unknown_material_is_rejected_instead_of_replaced(pool: SqlitePool) {
    let instance = instance(&pool, "corrupt").await;
    let identity = DeviceIdentity::new(instance.id, 42, material(1)).unwrap();
    identity.save(&pool).await.unwrap();
    sqlx::query("UPDATE device_identity SET material = $1 WHERE instance_id = $2")
        .bind(vec![0xffu8])
        .bind(instance.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        DeviceIdentity::find_by_instance_id(&pool, instance.id).await,
        Err(DeviceIdentityError::Decode(_))
    ));
    sqlx::query(
        "UPDATE device_identity SET material = $1, material_version = 2 WHERE instance_id = $2",
    )
    .bind(identity.material.encode_to_vec())
    .bind(instance.id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        DeviceIdentity::find_by_instance_id(&pool, instance.id).await,
        Err(DeviceIdentityError::UnsupportedMaterialVersion(2))
    ));
}

#[test]
fn test_identity_requires_complete_tpm_material_but_allows_absent_ek_certificates() {
    assert!(matches!(
        DeviceIdentity::new(0, 42, material(1)),
        Err(DeviceIdentityError::InvalidState(_))
    ));
    assert!(matches!(
        DeviceIdentity::new(1, 0, material(1)),
        Err(DeviceIdentityError::InvalidState(_))
    ));
    assert!(matches!(
        DeviceIdentity::new(1, 42, DeviceIdentityMaterial::default()),
        Err(DeviceIdentityError::InvalidState(_))
    ));
    let mut incomplete = material(1);
    let Some(device_identity_material::Backend::Tpm(tpm)) = &mut incomplete.backend else {
        unreachable!()
    };
    tpm.ak.as_mut().unwrap().private.clear();
    assert!(matches!(
        DeviceIdentity::new(1, 42, incomplete),
        Err(DeviceIdentityError::InvalidState(_))
    ));
    let mut without_certificate = material(1);
    let Some(device_identity_material::Backend::Tpm(tpm)) = &mut without_certificate.backend else {
        unreachable!()
    };
    tpm.ek.as_mut().unwrap().certificate_chain_der.clear();
    assert!(DeviceIdentity::new(1, 42, without_certificate).is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_identity_cannot_be_saved_without_its_local_instance(pool: SqlitePool) {
    let identity = DeviceIdentity::new(999, 42, material(1)).unwrap();
    assert!(
        matches!(identity.save(&pool).await, Err(DeviceIdentityError::Database(sqlx::Error::Database(error))) if error.is_foreign_key_violation())
    );
    assert!(
        DeviceIdentity::find_by_instance_id(&pool, identity.instance_id)
            .await
            .unwrap()
            .is_none()
    );
}
