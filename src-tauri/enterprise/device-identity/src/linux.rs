use crate::{Capabilities, DeviceIdentityError};

pub(crate) fn probe() -> Result<Capabilities, DeviceIdentityError> {
    Err(DeviceIdentityError::Unsupported)
}
