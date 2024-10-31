use crate::database::models::Id;
use crate::database::Instance;
use crate::error::Error;
use sqlx::SqliteExecutor;

impl Instance<Id> {
    pub async fn disable_enterprise_features<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        debug!(
            "Disabling enterprise features for instance {}({})",
            self.name, self.id
        );
        self.enterprise_enabled = false;
        self.disable_all_traffic = false;
        self.save(executor).await?;
        debug!(
            "Disabled enterprise features for instance {}({})",
            self.name, self.id
        );
        Ok(())
    }
}
