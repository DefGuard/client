use sqlx::SqliteExecutor;

use crate::{
    database::models::{instance::{ClientTrafficPolicy, Instance}, Id},
    error::Error,
};

impl Instance<Id> {
    pub async fn disable_enterprise_features<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        debug!(
            "Disabling enterprise features for instance {}({})",
            self.name, self.id
        );
        self.client_traffic_policy = ClientTrafficPolicy::None;
        self.save(executor).await?;
        debug!(
            "Disabled enterprise features for instance {}({})",
            self.name, self.id
        );
        Ok(())
    }
}
