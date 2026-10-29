#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "macos")]
mod macos;

pub enum Algorithm {
    Test,
}

pub struct TpmCapabilities {
    pub algorithms: Vec<AlgorithmCapability>,
    pub ecc_curves: Vec<EccCurveId>,

    pub fixed_properties: Vec<TpmProperty>,
    pub variable_properties: Vec<TpmProperty>,
    pub commands: Vec<CommandCapability>,
    pub pcr_banks: Vec<PcrBankCapability>,

    pub profiles: Vec<ProfileSupport>,
}

/// An algorithm reported by TPM2_GetCapability.
pub struct AlgorithmCapability {
    /// TPM_ALG_ID, preserved even if unknown to this library.
    pub id: AlgorithmId,
    /// Raw TPMA_ALGORITHM flags describing the algorithm's capabilities.
    pub flags: u32,
}

pub struct AlgorithmId(pub u16);

impl AlgorithmId {
    pub fn name(&self) -> Option<&'static str> {
        match self.0 {
            0x0001 => Some("RSA"),
            0x000B => Some("SHA-256"),
            0x0018 => Some("ECDSA"),
            0x0023 => Some("ECC"),
            // TODO
            // ...
            _ => None,
        }
    }
}

pub struct EccCurveId(pub u16);
impl EccCurveId {
    pub fn name(&self) -> Option<&'static str> {
        match self.0 {
            0x0001 => Some("RSA"),
            0x000B => Some("SHA-256"),
            0x0018 => Some("ECDSA"),
            0x0023 => Some("ECC"),
            // TODO
            // ...
            _ => None,
        }
    }
}
pub struct TpmConfig {

}

pub struct StoredDeviceKeys {

}

pub fn probe() {
    #[cfg(target_os = "linux")]
    linux::probe();

    #[cfg(windows)]
    windows::probe();

    #[cfg(target_os = "macos")]
    macos::probe();
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let caps = TpmCapabilities {
            algorithms: vec![Algorithm::Test],
        };
    }
}
