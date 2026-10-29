use crate::{Capabilities, HardwareIdentityError};

pub(crate) fn probe() -> Result<Capabilities, HardwareIdentityError> {
    Err(HardwareIdentityError::Unsupported)
}
