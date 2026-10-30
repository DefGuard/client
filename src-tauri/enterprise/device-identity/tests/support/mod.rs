pub mod apple;
pub mod tpm;

use device_identity::{
    attest_identity, attestation_identity, create_identity, AttestationChallenge,
    AttestationIdentity, AttestationRequest, AttestationResponse, IdentityRef, PossessionProof,
    SignatureAlgorithm,
};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use rand_core::{OsRng, RngCore};

pub type TestResult<T> = Result<T, String>;

pub fn check(condition: bool, message: &str) -> TestResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

pub fn challenge_data(purpose: &str) -> Vec<u8> {
    let mut nonce = [0u8; 32];
    OsRng.fill_bytes(&mut nonce);
    [
        b"defguard-contract-v1\0".as_slice(),
        purpose.as_bytes(),
        &[0, 0xff],
        &nonce,
    ]
    .concat()
}

pub fn prepare_attestation(
    metadata: &AttestationIdentity,
) -> (AttestationRequest, Option<[u8; 32]>) {
    let (challenge, secret) = match metadata {
        AttestationIdentity::Tpm(metadata) => {
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            let challenge = tpm::make_credential(metadata, &secret)
                .expect("prepare an independent software MakeCredential challenge");
            (AttestationChallenge::Tpm(challenge), Some(secret))
        }
        AttestationIdentity::AppleAppAttest { .. } => (AttestationChallenge::AppleAppAttest, None),
    };
    (
        AttestationRequest {
            data: challenge_data("attestation"),
            challenge,
        },
        secret,
    )
}

#[derive(Clone)]
pub enum IdentityVerifier {
    Tpm(VerifyingKey),
    Apple(apple::AppleVerifier),
}

impl IdentityVerifier {
    pub fn verify_proof(&mut self, data: &[u8], proof: &PossessionProof) -> TestResult<()> {
        match (self, proof) {
            (Self::Tpm(key), PossessionProof::Signature(signature)) => {
                check(
                    signature.algorithm == SignatureAlgorithm::EcdsaSha256,
                    "expected ECDSA/SHA-256 possession proof",
                )?;
                verify_signature(key, data, &signature.bytes)
            }
            (Self::Apple(verifier), PossessionProof::AppleAssertion(assertion)) => {
                apple::verify_assertion(verifier, data, &assertion.assertion_object)
            }
            _ => Err("possession proof backend differs from the attested backend".into()),
        }
    }
}

pub fn verify_attestation(
    metadata: &AttestationIdentity,
    request: &AttestationRequest,
    expected_secret: Option<&[u8; 32]>,
    response: &AttestationResponse,
) -> TestResult<IdentityVerifier> {
    match (metadata, &request.challenge, response) {
        (
            AttestationIdentity::Tpm(metadata),
            AttestationChallenge::Tpm(_),
            AttestationResponse::Tpm {
                activation_secret,
                evidence,
            },
        ) => {
            let secret = expected_secret.ok_or("missing verifier activation secret")?;
            check(
                activation_secret.as_slice() == secret,
                "credential activation did not recover Core's secret",
            )?;
            tpm::verify_attestation(metadata, &request.data, evidence).map(IdentityVerifier::Tpm)
        }
        (
            AttestationIdentity::AppleAppAttest { key_id },
            AttestationChallenge::AppleAppAttest,
            AttestationResponse::AppleAppAttest(evidence),
        ) => apple::verify_attestation(key_id, &request.data, &evidence.attestation_object)
            .map(IdentityVerifier::Apple),
        _ => Err("attestation backend differs from the requested backend".into()),
    }
}

pub async fn enroll() -> (IdentityRef, IdentityVerifier) {
    let identity = create_identity()
        .await
        .expect("create a hardware-backed identity");
    let metadata = attestation_identity(&identity).expect("read public attestation identity");
    let (request, secret) = prepare_attestation(&metadata);
    let response = attest_identity(&identity, &request)
        .await
        .expect("attest the identity");
    let verifier = verify_attestation(&metadata, &request, secret.as_ref(), &response)
        .unwrap_or_else(|error| panic!("independently verify initial attestation: {error}"));
    (identity, verifier)
}

pub fn verify_signature(key: &VerifyingKey, message: &[u8], signature: &[u8]) -> TestResult<()> {
    let signature = Signature::from_der(signature).map_err(|error| error.to_string())?;
    key.verify(message, &signature)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod verifier_tests;
