use defguard_client_proto::defguard::client::v1::{
    device_identity_material, DeviceIdentityMaterial,
};
use prost::Message;
use sqlx::SqliteExecutor;
use thiserror::Error;

use super::Id;

#[cfg(test)]
mod tests;

const MATERIAL_VERSION: i64 = 1;

// private key blobs must not enter debug logs or webview responses
#[derive(Clone, PartialEq)]
pub struct DeviceIdentity {
    pub instance_id: Id,
    pub core_device_id: Id,
    /// missing after a lost finish response, even when Core accepted attestation
    pub identity_id: Option<String>,
    pub material: DeviceIdentityMaterial,
}

#[derive(Debug, Error)]
pub enum DeviceIdentityError {
    #[error("Invalid device identity state: {0}")]
    InvalidState(&'static str),
    #[error("Unsupported device identity material version: {0}")]
    UnsupportedMaterialVersion(i64),
    #[error("Stored device identity material could not be decoded")]
    Decode(#[from] prost::DecodeError),
    #[error("Device identity registration has changed or no longer exists")]
    RegistrationChanged,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

struct StoredIdentity {
    instance_id: Id,
    core_device_id: Id,
    identity_id: Option<String>,
    material_version: i64,
    material: Vec<u8>,
}

impl DeviceIdentity {
    pub fn new(
        instance_id: Id,
        core_device_id: Id,
        material: DeviceIdentityMaterial,
    ) -> Result<Self, DeviceIdentityError> {
        let identity = Self {
            instance_id,
            core_device_id,
            identity_id: None,
            material,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub async fn save<'e, E: SqliteExecutor<'e>>(
        &self,
        executor: E,
    ) -> Result<(), DeviceIdentityError> {
        self.validate()?;
        if self.identity_id.is_some() {
            return Err(DeviceIdentityError::InvalidState(
                "new material cannot already have a Core identity ID",
            ));
        }
        let material = self.material.encode_to_vec();
        sqlx::query!(
            "INSERT INTO device_identity (instance_id, core_device_id, material_version, material) \
             VALUES ($1, $2, $3, $4)",
            self.instance_id,
            self.core_device_id,
            MATERIAL_VERSION,
            material,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn find_by_instance_id<'e, E: SqliteExecutor<'e>>(
        executor: E,
        instance_id: Id,
    ) -> Result<Option<Self>, DeviceIdentityError> {
        let stored = sqlx::query_as!(
            StoredIdentity,
            "SELECT instance_id, core_device_id, identity_id, material_version, material \
             FROM device_identity WHERE instance_id = $1",
            instance_id,
        )
        .fetch_optional(executor)
        .await?;
        stored.map(Self::try_from).transpose()
    }

    pub async fn record_identity_id<'e, E: SqliteExecutor<'e>>(
        &mut self,
        executor: E,
        identity_id: &str,
    ) -> Result<(), DeviceIdentityError> {
        self.validate()?;
        if identity_id.is_empty() {
            return Err(DeviceIdentityError::InvalidState(
                "Core identity ID is empty",
            ));
        }
        let material = self.material.encode_to_vec();
        let result = sqlx::query!(
            "UPDATE device_identity SET identity_id = $1 \
             WHERE instance_id = $2 AND core_device_id = $3 \
             AND material_version = $4 AND material = $5 \
             AND (identity_id IS NULL OR identity_id = $1)",
            identity_id,
            self.instance_id,
            self.core_device_id,
            MATERIAL_VERSION,
            material,
        )
        .execute(executor)
        .await?;
        if result.rows_affected() != 1 {
            return Err(DeviceIdentityError::RegistrationChanged);
        }
        self.identity_id = Some(identity_id.into());
        Ok(())
    }

    fn validate(&self) -> Result<(), DeviceIdentityError> {
        if self.instance_id <= 0 || self.core_device_id <= 0 {
            return Err(DeviceIdentityError::InvalidState(
                "local instance and Core device IDs must be positive",
            ));
        }
        if self.identity_id.as_deref() == Some("") {
            return Err(DeviceIdentityError::InvalidState(
                "Core identity ID is empty",
            ));
        }
        let Some(device_identity_material::Backend::Tpm(identity)) = &self.material.backend else {
            return Err(DeviceIdentityError::InvalidState("TPM material is missing"));
        };
        let (Some(ek), Some(ak), Some(key)) = (&identity.ek, &identity.ak, &identity.identity_key)
        else {
            return Err(DeviceIdentityError::InvalidState(
                "EK, AK and identity key material are required",
            ));
        };
        if [
            &ek.template,
            &ek.public,
            &identity.srk_template,
            &ak.public,
            &ak.private,
            &key.public,
            &key.private,
        ]
        .iter()
        .any(|bytes| bytes.is_empty())
        {
            return Err(DeviceIdentityError::InvalidState(
                "TPM templates and key blobs must not be empty",
            ));
        }
        Ok(())
    }
}

impl TryFrom<StoredIdentity> for DeviceIdentity {
    type Error = DeviceIdentityError;

    fn try_from(stored: StoredIdentity) -> Result<Self, Self::Error> {
        if stored.material_version != MATERIAL_VERSION {
            return Err(DeviceIdentityError::UnsupportedMaterialVersion(
                stored.material_version,
            ));
        }
        let material = DeviceIdentityMaterial::decode(stored.material.as_slice())?;
        let identity = Self {
            instance_id: stored.instance_id,
            core_device_id: stored.core_device_id,
            identity_id: stored.identity_id,
            material,
        };
        identity.validate()?;
        Ok(identity)
    }
}
