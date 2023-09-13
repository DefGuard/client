use chrono::NaiveDateTime;
use sqlx::{FromRow, query};

use crate::{database::DbPool, error::Error};

#[derive(FromRow)]
pub struct Connection {
    id: Option<i64>,
    location_id: i64,
    connected_from: String,
    start: NaiveDateTime,
    end: NaiveDateTime,
}

impl Connection {
    pub fn new(
        location_id: i64,
        connected_from: String,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Self {
        Connection {
            id: None,
            location_id,
            connected_from,
            start,
            end,
        }
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        let result = query!(
            "INSERT INTO connection (location_id, connected_from, start, end) \
            VALUES ($1, $2, $3, $4) \
            RETURNING id;",
            self.location_id,
            self.connected_from,
            self.start,
            self.end,
        )
        .fetch_one(pool)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }
}
