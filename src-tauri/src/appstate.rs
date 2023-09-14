use std::sync::Mutex;

use crate::database::DbPool;

#[derive(Default)]
pub struct AppState {
    pub db: Mutex<Option<DbPool>>,
}
impl AppState {
    pub fn get_pool(&self) -> DbPool {
        self.db.lock().unwrap().as_ref().cloned().unwrap()
    }
}
