use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, Type)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum MfaContract {
    #[default]
    Legacy = 0,
    MultiStep = 1,
}
