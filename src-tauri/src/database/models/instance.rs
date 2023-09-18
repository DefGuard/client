use crate::error::Error;
use sqlx::{query, FromRow};

#[derive(FromRow)]
pub struct Instance {
    pub id: Option<i64>,
    pub name: String,
    pub uuid: String,
}

impl Instance {
    pub fn new(name: String, uuid: String) -> Self {
        Instance {
            id: None,
            name,
            uuid,
        }
    }

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let result = query!(
            "INSERT INTO instance (name, uuid) VALUES ($1, $2) RETURNING id;",
            self.name,
            self.uuid,
        )
        .fetch_one(executor)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }
}
