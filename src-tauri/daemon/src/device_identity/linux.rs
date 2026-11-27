//! Converts daemon IPC messages to and from the Linux TPM backend.

use defguard_client_device_identity as backend;
use defguard_client_proto::defguard::{client::v1 as ipc, device_identity::v2 as proto};
use tonic::Status;

#[cfg(test)]
mod tests;

pub(super) struct PreparedAttestation {
    session_id: String,
    identity: backend::Identity,
    request: backend::AttestationRequest,
}

pub(super) async fn create() -> Result<ipc::CreateDeviceIdentityResponse, Status> {
    let identity = backend::create_identity().await.map_err(creation_error)?;
    let public = backend::attestation_identity(&identity)
        .await
        .map_err(creation_error)?;
    let backend::AttestationIdentity::Tpm(public) = public else {
        return Err(Status::unimplemented("Only TPM identities are supported"));
    };
    // Return reloadable material to the local caller separately from the public
    // EK and AK metadata it may send to Core.
    Ok(ipc::CreateDeviceIdentityResponse {
        identity: Some(encode_identity(identity)?),
        attestation_identity: Some(proto::AttestationIdentity {
            backend: Some(proto::attestation_identity::Backend::Tpm(
                proto::TpmAttestationIdentity {
                    ek_public: public.ek_public,
                    ek_certificate_chain_der: public.ek_certificate_chain_der,
                    ak_public: public.ak_public,
                },
            )),
        }),
    })
}

fn encode_identity(identity: backend::Identity) -> Result<ipc::DeviceIdentityMaterial, Status> {
    let backend::Identity::Tpm(identity) = identity else {
        return Err(Status::unimplemented("Only TPM identities are supported"));
    };
    Ok(ipc::DeviceIdentityMaterial {
        backend: Some(ipc::device_identity_material::Backend::Tpm(
            ipc::TpmIdentityMaterial {
                ek: Some(ipc::TpmEndorsementKeyMaterial {
                    template: identity.ek.template,
                    public: identity.ek.public,
                    certificate_chain_der: identity.ek.certificate_chain_der,
                }),
                srk_template: identity.srk_template,
                ak: Some(encode_key(identity.ak)),
                identity_key: Some(encode_key(identity.identity_key)),
            },
        )),
    })
}

fn encode_key(key: backend::TpmKeyBlobs) -> ipc::TpmKeyBlobs {
    ipc::TpmKeyBlobs {
        public: key.public,
        private: key.private,
    }
}

fn require_bytes(bytes: Vec<u8>, field: &'static str) -> Result<Vec<u8>, Status> {
    if bytes.is_empty() {
        Err(Status::invalid_argument(format!("Missing {field}")))
    } else {
        Ok(bytes)
    }
}

fn decode_key(key: ipc::TpmKeyBlobs) -> Result<backend::TpmKeyBlobs, Status> {
    Ok(backend::TpmKeyBlobs {
        public: require_bytes(key.public, "TPM key public blob")?,
        private: require_bytes(key.private, "TPM key private blob")?,
    })
}

fn decode_identity(identity: ipc::DeviceIdentityMaterial) -> Result<backend::Identity, Status> {
    let Some(ipc::device_identity_material::Backend::Tpm(identity)) = identity.backend else {
        return Err(Status::invalid_argument("Missing TPM identity material"));
    };
    let ek = identity
        .ek
        .ok_or_else(|| Status::invalid_argument("Missing EK material"))?;
    let ak = identity
        .ak
        .ok_or_else(|| Status::invalid_argument("Missing AK blobs"))?;
    let identity_key = identity
        .identity_key
        .ok_or_else(|| Status::invalid_argument("Missing identity key blobs"))?;
    Ok(backend::Identity::Tpm(backend::TpmIdentity {
        ek: backend::TpmEndorsementKey {
            template: require_bytes(ek.template, "EK template")?,
            public: require_bytes(ek.public, "EK public area")?,
            certificate_chain_der: ek.certificate_chain_der,
        },
        srk_template: require_bytes(identity.srk_template, "SRK template")?,
        ak: decode_key(ak)?,
        identity_key: decode_key(identity_key)?,
    }))
}

pub(super) fn prepare(
    request: ipc::AttestDeviceIdentityRequest,
) -> Result<PreparedAttestation, Status> {
    let identity = decode_identity(
        request
            .identity
            .ok_or_else(|| Status::invalid_argument("Missing identity material"))?,
    )?;
    let challenge = request
        .challenge
        .ok_or_else(|| Status::invalid_argument("Missing attestation challenge"))?;
    if challenge.session_id.is_empty() {
        return Err(Status::invalid_argument("Missing attestation challenge ID"));
    }
    // The backend hashes the exact context bytes for Certify. Core owns the
    // deadline check, so the daemon does not use its own clock here.
    let data = require_bytes(challenge.data, "attestation challenge data")?;
    let Some(proto::attestation_challenge::Challenge::Tpm(credential)) = challenge.challenge else {
        return Err(Status::invalid_argument(
            "A TPM identity requires a TPM credential challenge",
        ));
    };
    let credential = backend::CredentialChallenge {
        credential_blob: require_bytes(credential.credential_blob, "credential blob")?,
        encrypted_secret: require_bytes(credential.encrypted_secret, "encrypted secret")?,
    };
    Ok(PreparedAttestation {
        session_id: challenge.session_id,
        identity,
        request: backend::AttestationRequest {
            data,
            challenge: backend::AttestationChallenge::Tpm(credential),
        },
    })
}

pub(super) async fn attest(
    request: PreparedAttestation,
) -> Result<proto::AttestationChallengeSolution, Status> {
    // A backend failure after accepting the request is an enrollment result.
    // Echo the challenge ID so the caller can report that failure to Core.
    let result = match backend::attest_identity(&request.identity, &request.request).await {
        Ok(response) => encode_attestation(response)?,
        Err(error) => {
            proto::attestation_challenge_solution::Result::Error(error_code(&error).into())
        }
    };
    Ok(proto::AttestationChallengeSolution {
        session_id: request.session_id,
        result: Some(result),
    })
}

fn encode_attestation(
    response: backend::AttestationResponse,
) -> Result<proto::attestation_challenge_solution::Result, Status> {
    let backend::AttestationResponse::Tpm {
        activation_secret,
        evidence,
    } = response
    else {
        return Err(Status::internal(
            "TPM attestation returned an unexpected backend",
        ));
    };
    // Preserve the backend's signature encoding. Core verifies it against the
    // original attestation bytes and the declared algorithm.
    let algorithm = match evidence.signature.algorithm {
        backend::SignatureAlgorithm::EcdsaSha256 => proto::SignatureAlgorithm::EcdsaSha256,
        backend::SignatureAlgorithm::RsaSsaSha256 => proto::SignatureAlgorithm::RsaSsaSha256,
    };
    Ok(proto::attestation_challenge_solution::Result::Tpm(
        proto::TpmAttestationResponse {
            activation_secret,
            evidence: Some(proto::TpmEvidence {
                ek_public: evidence.ek_public,
                ek_certificate_chain_der: evidence.ek_certificate_chain_der,
                ak_public: evidence.ak_public,
                identity_public: evidence.identity_public,
                attest: evidence.attest,
                signature: Some(proto::Signature {
                    algorithm: algorithm.into(),
                    signature: evidence.signature.bytes,
                }),
            }),
        },
    ))
}

fn creation_error(error: backend::DeviceIdentityError) -> Status {
    use backend::DeviceIdentityError;
    let message = error.to_string();
    match error {
        DeviceIdentityError::Unavailable | DeviceIdentityError::TemporarilyUnavailable => {
            Status::unavailable(message)
        }
        DeviceIdentityError::Timeout => Status::deadline_exceeded(message),
        DeviceIdentityError::RateLimited => Status::resource_exhausted(message),
        DeviceIdentityError::PermissionDenied => Status::permission_denied(message),
        DeviceIdentityError::Unsupported => Status::unimplemented(message),
        DeviceIdentityError::IdentityLost => Status::failed_precondition(message),
        DeviceIdentityError::InvalidChallenge => Status::invalid_argument(message),
        DeviceIdentityError::BackendFailure => Status::internal(message),
    }
}

fn error_code(error: &backend::DeviceIdentityError) -> proto::IdentityErrorCode {
    use backend::DeviceIdentityError;
    match error {
        DeviceIdentityError::Unavailable => proto::IdentityErrorCode::Unavailable,
        DeviceIdentityError::TemporarilyUnavailable => {
            proto::IdentityErrorCode::TemporarilyUnavailable
        }
        DeviceIdentityError::Timeout => proto::IdentityErrorCode::Timeout,
        DeviceIdentityError::RateLimited => proto::IdentityErrorCode::RateLimited,
        DeviceIdentityError::PermissionDenied => proto::IdentityErrorCode::PermissionDenied,
        DeviceIdentityError::Unsupported => proto::IdentityErrorCode::Unsupported,
        DeviceIdentityError::IdentityLost => proto::IdentityErrorCode::IdentityLost,
        DeviceIdentityError::InvalidChallenge => proto::IdentityErrorCode::InvalidChallenge,
        DeviceIdentityError::BackendFailure => proto::IdentityErrorCode::BackendFailure,
    }
}
