use std::sync::Mutex;

use crate::database::DbPool;

#[derive(Default)]
pub struct AppState {
    pub db: Mutex<Option<DbPool>>,
}
