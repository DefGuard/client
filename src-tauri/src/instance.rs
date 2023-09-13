use sqlx::{query, FromRow};
use crate::{database::DbPool, error::Error};

#[derive(FromRow)]
pub struct Instance {
    id: Option<i64>,
    name: String,
}

impl Instance {
    pub fn new(name: String) -> Self {
        Instance {
            id: None,
            name,
        }
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        let result = query!(
            "INSERT INTO instance (name) VALUES ($1) RETURNING id;",
            self.name,
        )
        .fetch_one(pool)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }
}
