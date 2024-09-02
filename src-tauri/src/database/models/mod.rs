use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;

pub mod connection;
pub mod instance;
pub mod location;
pub mod settings;
pub mod tunnel;
pub mod wireguard_keys;

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[sqlx(transparent)]
pub struct Id(pub i64);
#[derive(Debug, Clone)]
pub struct NoId;

pub trait HasId {
    fn id(&self) -> u64;
}
