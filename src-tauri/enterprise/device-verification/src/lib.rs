//! The administrator compares the code this device shows with the one in Core, then verifies
//! or blocks the device. Edge is called with the device polling token.

#[macro_use]
extern crate log;

mod client;
mod error;
mod session;
mod types;

pub use client::{await_decision, start, status};
pub use error::Error;
pub use session::{VerificationCode, VerificationSession};
pub use types::{Outcome, VerificationStatus};
