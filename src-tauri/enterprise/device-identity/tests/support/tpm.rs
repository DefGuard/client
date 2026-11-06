use aes::Aes128;
use cfb_mode::cipher::{AsyncStreamCipher, KeyIvInit};
use device_identity::{
    CredentialChallenge, SignatureAlgorithm, TpmAttestationIdentity, TpmEvidence,
};
use hmac::{Hmac, Mac};
use p256::{
    ecdh::EphemeralSecret, ecdsa::VerifyingKey, elliptic_curve::sec1::ToEncodedPoint, PublicKey,
};
use rand_core::{OsRng, RngCore};
use rsa::{BigUint, Oaep, RsaPublicKey};
use sha2::{Digest, Sha256};

use super::{check, verify_signature, TestResult};

const SHA256: u16 = 0x000b;
const NULL: u16 = 0x0010;
const ECC: u16 = 0x0023;
const ECDSA: u16 = 0x0018;
const P256: u16 = 0x0003;
const FIXED_TPM: u32 = 1 << 1;
const FIXED_PARENT: u32 = 1 << 4;
const SENSITIVE_DATA_ORIGIN: u32 = 1 << 5;
const RESTRICTED: u32 = 1 << 16;
const DECRYPT: u32 = 1 << 17;
const SIGN: u32 = 1 << 18;

type HmacSha256 = Hmac<Sha256>;

pub struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn take(&mut self, count: usize) -> TestResult<&'a [u8]> {
        check(count <= self.bytes.len(), "truncated binary structure")?;
        let (value, rest) = self.bytes.split_at(count);
        self.bytes = rest;
        Ok(value)
    }

    pub fn u16(&mut self) -> TestResult<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn u32(&mut self) -> TestResult<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn sized(&mut self) -> TestResult<&'a [u8]> {
        let size = self.u16()? as usize;
        self.take(size)
    }

    pub fn finish(self) -> TestResult<()> {
        check(self.bytes.is_empty(), "unexpected trailing binary data")
    }
}

pub fn name(public: &[u8]) -> Vec<u8> {
    [SHA256.to_be_bytes().as_slice(), &Sha256::digest(public)].concat()
}

pub fn signing_key(public: &[u8], restricted: bool) -> TestResult<VerifyingKey> {
    let mut reader = Reader::new(public);
    check(reader.u16()? == ECC, "expected an ECC key")?;
    check(reader.u16()? == SHA256, "expected SHA-256 Name algorithm")?;
    let attributes = reader.u32()?;
    let required = FIXED_TPM | FIXED_PARENT | SENSITIVE_DATA_ORIGIN | SIGN;
    check(
        attributes & required == required,
        "missing TPM key attributes",
    )?;
    check(attributes & DECRYPT == 0, "signing key must not decrypt")?;
    check(
        (attributes & RESTRICTED != 0) == restricted,
        "incorrect restricted attribute",
    )?;
    reader.sized()?;
    check(
        reader.u16()? == NULL,
        "unexpected signing key symmetric algorithm",
    )?;
    check(reader.u16()? == ECDSA, "expected ECDSA signature scheme")?;
    check(reader.u16()? == SHA256, "expected SHA-256 signature hash")?;
    check(reader.u16()? == P256, "expected NIST P-256")?;
    check(reader.u16()? == NULL, "unexpected signing key KDF")?;
    let x = reader.sized()?;
    let y = reader.sized()?;
    let key = public_point(x, y)?;
    reader.finish()?;
    Ok(VerifyingKey::from(key))
}

fn public_point(x: &[u8], y: &[u8]) -> TestResult<PublicKey> {
    check(!x.is_empty() && x.len() <= 32, "invalid P-256 x coordinate")?;
    check(!y.is_empty() && y.len() <= 32, "invalid P-256 y coordinate")?;
    let mut point = [0u8; 65];
    point[0] = 4;
    point[33 - x.len()..33].copy_from_slice(x);
    point[65 - y.len()..].copy_from_slice(y);
    PublicKey::from_sec1_bytes(&point).map_err(|error| error.to_string())
}

pub fn verify_attestation(
    metadata: &TpmAttestationIdentity,
    data: &[u8],
    evidence: &TpmEvidence,
) -> TestResult<VerifyingKey> {
    check(
        evidence.ek_public == metadata.ek_public,
        "EK changed during attestation",
    )?;
    check(
        evidence.ak_public == metadata.ak_public,
        "AK changed during attestation",
    )?;
    check(
        evidence.ek_certificate_chain_der == metadata.ek_certificate_chain_der,
        "EK certificates changed",
    )?;
    let ak = signing_key(&evidence.ak_public, true)?;
    let identity = signing_key(&evidence.identity_public, false)?;
    check(
        evidence.signature.algorithm == SignatureAlgorithm::EcdsaSha256,
        "expected ECDSA/SHA-256 attestation",
    )?;
    verify_signature(&ak, &evidence.attest, &evidence.signature.bytes)?;

    let mut reader = Reader::new(&evidence.attest);
    check(reader.u32()? == 0xff54_4347, "invalid TPM generated magic")?;
    check(reader.u16()? == 0x8017, "expected TPM_ST_ATTEST_CERTIFY")?;
    reader.sized()?;
    check(
        reader.sized()? == &Sha256::digest(data)[..],
        "attestation does not bind the exact Core data",
    )?;
    reader.take(17)?;
    reader.take(8)?;
    check(
        reader.sized()? == name(&evidence.identity_public),
        "certified key Name mismatch",
    )?;
    reader.sized()?;
    reader.finish()?;
    Ok(identity)
}

pub fn make_credential(
    metadata: &TpmAttestationIdentity,
    secret: &[u8],
) -> TestResult<CredentialChallenge> {
    signing_key(&metadata.ak_public, true)?;
    let mut reader = Reader::new(&metadata.ek_public);
    let algorithm = reader.u16()?;
    check(
        reader.u16()? == SHA256,
        "test verifier requires a SHA-256 EK",
    )?;
    reader.u32()?;
    reader.sized()?;
    check(reader.u16()? == 0x0006, "test verifier requires an AES EK")?;
    check(reader.u16()? == 128, "test verifier requires AES-128")?;
    check(reader.u16()? == 0x0043, "test verifier requires AES-CFB")?;
    check(reader.u16()? == NULL, "expected null EK scheme")?;

    let (seed, encrypted_secret) = match algorithm {
        0x0001 => {
            check(reader.u16()? == 2048, "test verifier requires RSA-2048 EK")?;
            let exponent = match reader.u32()? {
                0 => 65537,
                value => value,
            };
            let modulus = reader.sized()?;
            let public = RsaPublicKey::new(BigUint::from_bytes_be(modulus), exponent.into())
                .map_err(|error| error.to_string())?;
            let mut seed = vec![0u8; 32];
            OsRng.fill_bytes(&mut seed);
            let encrypted = public
                .encrypt(
                    &mut OsRng,
                    Oaep::new_with_label::<Sha256, _>("IDENTITY\0"),
                    &seed,
                )
                .map_err(|error| error.to_string())?;
            (seed, encrypted)
        }
        ECC => {
            check(reader.u16()? == P256, "test verifier requires P-256 EK")?;
            check(reader.u16()? == NULL, "expected null EK KDF")?;
            let x = reader.sized()?;
            let y = reader.sized()?;
            let public = public_point(x, y)?;
            let ephemeral = EphemeralSecret::random(&mut OsRng);
            let ephemeral_public = PublicKey::from(&ephemeral).to_encoded_point(false);
            let shared = ephemeral.diffie_hellman(&public);
            let ek_point = public.to_encoded_point(false);
            let seed = Sha256::digest(
                [
                    1u32.to_be_bytes().as_slice(),
                    &shared.raw_secret_bytes()[..],
                    b"IDENTITY\0",
                    &ephemeral_public.x().unwrap()[..],
                    &ek_point.x().unwrap()[..],
                ]
                .concat(),
            )
            .to_vec();
            let encrypted = [
                sized(ephemeral_public.x().unwrap()),
                sized(ephemeral_public.y().unwrap()),
            ]
            .concat();
            (seed, encrypted)
        }
        _ => return Err("unsupported EK algorithm in test fixture".into()),
    };
    reader.finish()?;

    let ak_name = name(&metadata.ak_public);
    let storage_key = kdfa(&seed, b"STORAGE", &ak_name, 128);
    let mut ciphertext = sized(secret);
    cfb_mode::Encryptor::<Aes128>::new(storage_key.as_slice().into(), &[0; 16].into())
        .encrypt(&mut ciphertext);
    let integrity_key = kdfa(&seed, b"INTEGRITY", &[], 256);
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&integrity_key).unwrap();
    mac.update(&ciphertext);
    mac.update(&ak_name);
    let integrity = mac.finalize().into_bytes();
    Ok(CredentialChallenge {
        credential_blob: [sized(&integrity), ciphertext].concat(),
        encrypted_secret,
    })
}

fn sized(bytes: &[u8]) -> Vec<u8> {
    [(bytes.len() as u16).to_be_bytes().as_slice(), bytes].concat()
}

fn kdfa(seed: &[u8], label: &[u8], context: &[u8], bits: u32) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(seed).unwrap();
    mac.update(&1u32.to_be_bytes());
    mac.update(label);
    mac.update(&[0]);
    mac.update(context);
    mac.update(&bits.to_be_bytes());
    mac.finalize().into_bytes()[..(bits / 8) as usize].to_vec()
}
