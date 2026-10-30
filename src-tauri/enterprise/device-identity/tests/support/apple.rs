use std::{fs, io::Cursor, process::Command};

use base64::{engine::general_purpose::STANDARD, Engine};
use ciborium::Value;
use p256::{ecdsa::VerifyingKey, pkcs8::DecodePublicKey};
use sha2::{Digest, Sha256};
use x509_parser::parse_x509_certificate;

use super::{check, tpm::Reader, verify_signature, TestResult};

#[derive(Clone)]
pub struct AppleVerifier {
    pub key: VerifyingKey,
    pub app_id_hash: Vec<u8>,
    pub counter: u32,
}

pub fn decode(bytes: &[u8]) -> TestResult<Value> {
    let mut cursor = Cursor::new(bytes);
    let value = ciborium::from_reader(&mut cursor).map_err(|error| error.to_string())?;
    check(
        cursor.position() == bytes.len() as u64,
        "trailing CBOR data",
    )?;
    Ok(value)
}

pub fn field<'a>(value: &'a Value, name: &str) -> TestResult<&'a Value> {
    value
        .as_map()
        .and_then(|map| {
            map.iter()
                .find_map(|(key, value)| (key.as_text() == Some(name)).then_some(value))
        })
        .ok_or_else(|| format!("missing CBOR field {name}"))
}

fn bytes(value: &Value) -> TestResult<&[u8]> {
    value
        .as_bytes()
        .map(Vec::as_slice)
        .ok_or_else(|| "expected CBOR bytes".into())
}

fn integer_field(value: &Value, label: i128) -> TestResult<&Value> {
    value
        .as_map()
        .and_then(|map| {
            map.iter().find_map(|(key, value)| {
                (key.as_integer().map(i128::from) == Some(label)).then_some(value)
            })
        })
        .ok_or_else(|| format!("missing COSE key label {label}"))
}

pub fn verify_attestation(key_id: &str, data: &[u8], object: &[u8]) -> TestResult<AppleVerifier> {
    let object = decode(object)?;
    check(
        field(&object, "fmt")?.as_text() == Some("apple-appattest"),
        "unexpected Apple attestation format",
    )?;
    let statement = field(&object, "attStmt")?;
    check(
        !bytes(field(statement, "receipt")?)?.is_empty(),
        "missing Apple receipt",
    )?;
    let chain = field(statement, "x5c")?
        .as_array()
        .ok_or("expected certificate array")?;
    check(
        chain.len() >= 2,
        "expected Apple leaf and intermediate certificates",
    )?;
    let chain: Vec<&[u8]> = chain.iter().map(bytes).collect::<TestResult<_>>()?;
    verify_certificate_chain(&chain)?;
    let (rest, certificate) =
        parse_x509_certificate(chain[0]).map_err(|error| error.to_string())?;
    check(rest.is_empty(), "trailing certificate data")?;
    let key = VerifyingKey::from_public_key_der(certificate.public_key().raw)
        .map_err(|error| error.to_string())?;

    let auth_data = bytes(field(&object, "authData")?)?;
    let mut reader = Reader::new(auth_data);
    let app_id = std::env::var("DEFGUARD_APP_ATTEST_APP_ID")
        .map_err(|_| "set DEFGUARD_APP_ATTEST_APP_ID to TEAM_ID.bundle_id")?;
    let app_id_hash = Sha256::digest(app_id.as_bytes()).to_vec();
    check(
        reader.take(32)? == app_id_hash,
        "App Attest relying party mismatch",
    )?;
    check(
        reader.take(1)?[0] & 0x40 != 0,
        "missing attested credential data",
    )?;
    check(
        reader.u32()? == 0,
        "initial App Attest counter must be zero",
    )?;
    let aaguid = reader.take(16)?;
    let expected_aaguid = match std::env::var("DEFGUARD_APP_ATTEST_ENVIRONMENT").as_deref() {
        Ok("production") => b"appattest\0\0\0\0\0\0\0".as_slice(),
        Ok("development") => b"appattestdevelop".as_slice(),
        _ => return Err("set DEFGUARD_APP_ATTEST_ENVIRONMENT to development or production".into()),
    };
    check(aaguid == expected_aaguid, "App Attest environment mismatch")?;
    let credential_id = reader.sized()?;
    let decoded_key_id = STANDARD.decode(key_id).map_err(|error| error.to_string())?;
    check(
        credential_id == decoded_key_id,
        "App Attest credential ID mismatch",
    )?;
    check(
        decoded_key_id == Sha256::digest(key.to_encoded_point(false).as_bytes())[..],
        "App Attest key ID does not match public key",
    )?;

    let credential_offset = auth_data.len() - (32 + 1 + 4 + 16 + 2 + credential_id.len());
    let cose = decode(reader.take(credential_offset)?)?;
    reader.finish()?;
    check(
        integer_field(&cose, 1)?.as_integer().map(i128::from) == Some(2),
        "expected COSE EC2 key",
    )?;
    check(
        integer_field(&cose, 3)?.as_integer().map(i128::from) == Some(-7),
        "expected COSE ES256",
    )?;
    check(
        integer_field(&cose, -1)?.as_integer().map(i128::from) == Some(1),
        "expected COSE P-256",
    )?;
    let point = key.to_encoded_point(false);
    check(
        bytes(integer_field(&cose, -2)?)? == &point.x().unwrap()[..],
        "COSE x coordinate mismatch",
    )?;
    check(
        bytes(integer_field(&cose, -3)?)? == &point.y().unwrap()[..],
        "COSE y coordinate mismatch",
    )?;

    let nonce = Sha256::digest([auth_data, &Sha256::digest(data)[..]].concat());
    let extension = certificate
        .extensions()
        .iter()
        .find(|extension| extension.oid.to_id_string() == "1.2.840.113635.100.8.2")
        .ok_or("missing App Attest nonce extension")?;
    let mut sequence = DerReader::new(extension.value);
    let value = sequence.element(0x30)?;
    sequence.finish()?;
    let mut explicit = DerReader::new(value);
    let value = explicit.element(0xa1)?;
    explicit.finish()?;
    let mut octets = DerReader::new(value);
    check(
        octets.element(0x04)? == &nonce[..],
        "App Attest nonce does not bind the exact Core data",
    )?;
    octets.finish()?;
    Ok(AppleVerifier {
        key,
        app_id_hash,
        counter: 0,
    })
}

pub fn verify_assertion(
    verifier: &mut AppleVerifier,
    data: &[u8],
    object: &[u8],
) -> TestResult<()> {
    let object = decode(object)?;
    let signature = bytes(field(&object, "signature")?)?;
    let auth_data = bytes(field(&object, "authenticatorData")?)?;
    let mut reader = Reader::new(auth_data);
    check(
        reader.take(32)? == verifier.app_id_hash,
        "assertion relying party mismatch",
    )?;
    reader.take(1)?;
    let counter = reader.u32()?;
    check(
        counter > verifier.counter,
        "assertion counter did not increase",
    )?;
    let message = [auth_data, &Sha256::digest(data)[..]].concat();
    verify_signature(&verifier.key, &message, signature)?;
    verifier.counter = counter;
    Ok(())
}

fn verify_certificate_chain(chain: &[&[u8]]) -> TestResult<()> {
    let root = std::env::var_os("DEFGUARD_APP_ATTEST_ROOT_CA").ok_or(
        "set DEFGUARD_APP_ATTEST_ROOT_CA to the trusted Apple App Attestation Root CA PEM",
    )?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let leaf = directory.path().join("leaf.pem");
    let intermediates = directory.path().join("intermediates.pem");
    fs::write(&leaf, pem(chain[0])).map_err(|error| error.to_string())?;
    fs::write(
        &intermediates,
        chain[1..].iter().map(|cert| pem(cert)).collect::<String>(),
    )
    .map_err(|error| error.to_string())?;
    let output = Command::new("openssl")
        .arg("verify")
        .arg("-no-CApath")
        .arg("-no-CAstore")
        .arg("-CAfile")
        .arg(root)
        .arg("-untrusted")
        .arg(intermediates)
        .arg(leaf)
        .output()
        .map_err(|error| format!("run OpenSSL to validate Apple certificates: {error}"))?;
    check(
        output.status.success(),
        &format!(
            "Apple certificate validation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

fn pem(der: &[u8]) -> String {
    let encoded = STANDARD.encode(der);
    let body = encoded
        .as_bytes()
        .chunks(64)
        .map(|line| format!("{}\n", std::str::from_utf8(line).unwrap()))
        .collect::<String>();
    format!("-----BEGIN CERTIFICATE-----\n{body}-----END CERTIFICATE-----\n")
}

struct DerReader<'a> {
    bytes: &'a [u8],
}

impl<'a> DerReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn element(&mut self, tag: u8) -> TestResult<&'a [u8]> {
        check(
            self.bytes.len() >= 2 && self.bytes[0] == tag,
            "unexpected DER tag",
        )?;
        let length = self.bytes[1] as usize;
        check(
            length < 128 && self.bytes.len() >= 2 + length,
            "invalid DER extension length",
        )?;
        let value = &self.bytes[2..2 + length];
        self.bytes = &self.bytes[2 + length..];
        Ok(value)
    }

    fn finish(self) -> TestResult<()> {
        check(self.bytes.is_empty(), "trailing DER extension data")
    }
}
