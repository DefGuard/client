use ciborium::Value;
use device_identity::{Signature, SignatureAlgorithm, TpmAttestationIdentity, TpmEvidence};
use p256::ecdsa::{signature::Signer, Signature as EcdsaSignature, SigningKey};
use sha2::{Digest, Sha256};

use super::{apple, tpm};

fn key(value: u8) -> SigningKey {
    SigningKey::from_slice(&[value; 32]).unwrap()
}

fn sized(bytes: &[u8]) -> Vec<u8> {
    [(bytes.len() as u16).to_be_bytes().as_slice(), bytes].concat()
}

fn public(key: &SigningKey, restricted: bool) -> Vec<u8> {
    let point = key.verifying_key().to_encoded_point(false);
    let attributes = 0x0004_0072u32 | if restricted { 0x0001_0000 } else { 0 };
    [
        0x0023u16.to_be_bytes().as_slice(),
        0x000bu16.to_be_bytes().as_slice(),
        &attributes.to_be_bytes(),
        &sized(&[]),
        &0x0010u16.to_be_bytes(),
        &0x0018u16.to_be_bytes(),
        &0x000bu16.to_be_bytes(),
        &0x0003u16.to_be_bytes(),
        &0x0010u16.to_be_bytes(),
        &sized(point.x().unwrap()),
        &sized(point.y().unwrap()),
    ]
    .concat()
}

#[test]
fn test_independent_tpm_verifier_checks_certify_signature_name_profile_and_data() {
    let ak = key(1);
    let identity = key(2);
    let data = b"opaque Core data\0\xff";
    let ak_public = public(&ak, true);
    let identity_public = public(&identity, false);
    let attest = [
        0xff54_4347u32.to_be_bytes().as_slice(),
        &0x8017u16.to_be_bytes(),
        &sized(&tpm::name(&ak_public)),
        &sized(&Sha256::digest(data)),
        &[0u8; 17],
        &[0u8; 8],
        &sized(&tpm::name(&identity_public)),
        &sized(&[]),
    ]
    .concat();
    let signature: EcdsaSignature = ak.sign(&attest);
    let metadata = TpmAttestationIdentity {
        ek_public: vec![1],
        ek_certificate_chain_der: vec![],
        ak_public: ak_public.clone(),
    };
    let evidence = TpmEvidence {
        ek_public: metadata.ek_public.clone(),
        ek_certificate_chain_der: vec![],
        ak_public,
        identity_public,
        attest,
        signature: Signature {
            algorithm: SignatureAlgorithm::EcdsaSha256,
            bytes: signature.to_der().as_bytes().to_vec(),
        },
    };
    assert_eq!(
        tpm::verify_attestation(&metadata, data, &evidence).unwrap(),
        *identity.verifying_key()
    );
    assert!(tpm::verify_attestation(&metadata, b"different Core data", &evidence).is_err());

    let mut wrong_key = evidence.clone();
    wrong_key.identity_public = public(&key(3), false);
    assert!(tpm::verify_attestation(&metadata, data, &wrong_key).is_err());

    let mut wrong_profile = evidence.clone();
    wrong_profile.identity_public = public(&identity, true);
    assert!(tpm::verify_attestation(&metadata, data, &wrong_profile).is_err());

    let mut wrong_signature = evidence;
    wrong_signature.signature.bytes[0] ^= 1;
    assert!(tpm::verify_attestation(&metadata, data, &wrong_signature).is_err());
}

#[test]
fn test_independent_apple_assertion_verifier_checks_signature_data_key_and_counter() {
    let key = key(4);
    let data = b"opaque Core data\0\xff";
    let app_id_hash = Sha256::digest(b"TESTTEAM.example.defguard").to_vec();
    let auth_data = [app_id_hash.as_slice(), &[0], &1u32.to_be_bytes()].concat();
    let message = [auth_data.as_slice(), &Sha256::digest(data)].concat();
    let signature: EcdsaSignature = key.sign(&message);
    let object = Value::Map(vec![
        (
            Value::Text("signature".into()),
            Value::Bytes(signature.to_der().as_bytes().to_vec()),
        ),
        (
            Value::Text("authenticatorData".into()),
            Value::Bytes(auth_data),
        ),
    ]);
    let mut encoded = vec![];
    ciborium::into_writer(&object, &mut encoded).unwrap();
    let verifier = apple::AppleVerifier {
        key: *key.verifying_key(),
        app_id_hash,
        counter: 0,
    };
    let mut valid = verifier.clone();
    apple::verify_assertion(&mut valid, data, &encoded).unwrap();
    assert_eq!(valid.counter, 1);
    assert!(apple::verify_assertion(&mut valid, data, &encoded).is_err());
    assert!(
        apple::verify_assertion(&mut verifier.clone(), b"different Core data", &encoded).is_err()
    );

    let mut wrong_key = verifier.clone();
    wrong_key.key = *self::key(5).verifying_key();
    assert!(apple::verify_assertion(&mut wrong_key, data, &encoded).is_err());

    let mut wrong_app = verifier;
    wrong_app.app_id_hash[0] ^= 1;
    assert!(apple::verify_assertion(&mut wrong_app, data, &encoded).is_err());
}
