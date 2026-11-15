use picky_asn1::wrapper::IntegerAsn1;
use picky_asn1_x509::signature::EcdsaSignatureValue;
use tpm2_protocol::data::{TpmAlgId, TpmtSignature, TpmuSignature};

use crate::{DeviceIdentityError, Signature, SignatureAlgorithm};

pub(super) fn encode_signature(signature: TpmtSignature) -> Result<Signature, DeviceIdentityError> {
    if signature.sig_alg != TpmAlgId::Ecdsa {
        return Err(DeviceIdentityError::Unsupported);
    }
    let TpmuSignature::Ecdsa(signature) = signature.signature else {
        return Err(DeviceIdentityError::Unsupported);
    };
    if signature.hash != TpmAlgId::Sha256 {
        return Err(DeviceIdentityError::Unsupported);
    }

    let value = EcdsaSignatureValue {
        r: IntegerAsn1::from_bytes_be_unsigned(signature.signature_r.as_ref().to_vec()),
        s: IntegerAsn1::from_bytes_be_unsigned(signature.signature_s.as_ref().to_vec()),
    };
    Ok(Signature {
        algorithm: SignatureAlgorithm::EcdsaSha256,
        bytes: picky_asn1_der::to_vec(&value).map_err(|_| DeviceIdentityError::BackendFailure)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpm2_protocol::data::{Tpm2bEccParameter, TpmsSignatureEcc};

    #[test]
    fn test_ecdsa_signature_der_handles_high_bit_and_leading_zero() {
        let signature = TpmtSignature {
            sig_alg: TpmAlgId::Ecdsa,
            signature: TpmuSignature::Ecdsa(TpmsSignatureEcc {
                hash: TpmAlgId::Sha256,
                signature_r: Tpm2bEccParameter::try_from([0, 0x80].as_slice())
                    .expect("valid r scalar"),
                signature_s: Tpm2bEccParameter::try_from([0, 1].as_slice())
                    .expect("valid s scalar"),
            }),
        };

        let encoded = encode_signature(signature).expect("encode ECDSA signature");
        assert_eq!(encoded.algorithm, SignatureAlgorithm::EcdsaSha256);
        assert_eq!(
            encoded.bytes,
            [0x30, 0x07, 0x02, 0x02, 0x00, 0x80, 0x02, 0x01, 0x01]
        );
    }
}
