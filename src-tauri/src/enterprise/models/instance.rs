use sqlx::SqliteExecutor;

use crate::{
    database::models::{
        instance::{ClientTrafficPolicy, Instance},
        Id,
    },
    error::Error,
};

pub async fn disable_enterprise_features<'e, E>(
    instance: &mut Instance<Id>,
    executor: E,
) -> Result<(), Error>
where
    E: SqliteExecutor<'e>,
{
    debug!(
        "Disabling enterprise features for instance {}({})",
        instance.name, instance.id
    );
    instance.client_traffic_policy = ClientTrafficPolicy::None;
    instance.save(executor).await?;
    debug!(
        "Disabled enterprise features for instance {}({})",
        instance.name, instance.id
    );
    Ok(())
}
