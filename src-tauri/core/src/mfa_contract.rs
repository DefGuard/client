use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum MfaContract {
    Legacy = 0,
    MultiStep = 1,
}
