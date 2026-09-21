use serde_json::{json, Value};

use super::*;

fn challenge_json() -> String {
    json!({
        "publicKey": {
            "rp": { "name": "Defguard", "id": "vpn.example.com" },
            "user": {
                // "user-handle" in base64url.
                "id": "dXNlci1oYW5kbGU",
                "name": "admin",
                "displayName": "admin",
            },
            "challenge": "Y2hhbGxlbmdl",
            "pubKeyCredParams": [{ "type": "public-key", "alg": -7 }],
            "timeout": 60000,
            "excludeCredentials": [{ "type": "public-key", "id": "Y3JlZC1vbmU" }],
            "attestation": "none",
            "authenticatorSelection": {
                "requireResidentKey": true,
                "userVerification": "required",
                "residentKey": "required",
            },
            "extensions": {},
        }
    })
    .to_string()
}

fn origin() -> Url {
    Url::parse("https://vpn.example.com/").unwrap()
}

#[test]
fn test_client_data_is_what_the_server_recomputes() {
    let ceremony = prepare_registration(&challenge_json(), &origin()).unwrap();
    let client_data: Value = serde_json::from_slice(&ceremony.request.client_data).unwrap();

    assert_eq!(client_data["type"], "webauthn.create");
    // Copied verbatim, the server compares it with the challenge it stored.
    assert_eq!(client_data["challenge"], "Y2hhbGxlbmdl");
    // No trailing slash, or the origin check on the server fails.
    assert_eq!(client_data["origin"], "https://vpn.example.com");
    assert_eq!(client_data["crossOrigin"], false);
}

/// Pins that the assertion is NOT a WebAuthn ceremony. Making it one needs a proto and Core
/// change first, see the doc comment on `assertion_client_data`.
#[test]
fn test_assertion_client_data_is_the_raw_challenge_bytes() {
    let request = prepare_assertion("vpn.example.com", "Y2hhbGxlbmdl", &[]).unwrap();

    assert_eq!(request.client_data, b"Y2hhbGxlbmdl");
    // Emphatically not JSON.
    assert!(serde_json::from_slice::<Value>(&request.client_data).is_err());
}

#[test]
fn test_ceremony_follows_the_options_core_sent() {
    let request = prepare_registration(&challenge_json(), &origin())
        .unwrap()
        .request;

    assert_eq!(request.rp_id, "vpn.example.com");
    assert_eq!(request.rp_name, "Defguard");
    assert_eq!(request.user.id, b"user-handle");
    assert_eq!(request.user.name, "admin");
    assert_eq!(request.exclude_credentials, vec![b"cred-one".to_vec()]);
    assert_eq!(request.resident_key, ResidentKey::Required);
    assert_eq!(request.user_verification, UserVerification::Required);
    assert_eq!(request.algorithms, vec![COSE_ES256]);
    // Only removable keys, so the factor behaves the same on every platform.
    assert_eq!(request.attachment, Attachment::CrossPlatform);
}

/// A key with no free resident slot refuses `required` but makes a non-discoverable credential.
#[test]
fn test_preferred_resident_key_is_not_required() {
    let preferred = challenge_json()
        .replace(
            r#""requireResidentKey":true"#,
            r#""requireResidentKey":false"#,
        )
        .replace(
            r#""residentKey":"required""#,
            r#""residentKey":"preferred""#,
        );

    let request = prepare_registration(&preferred, &origin()).unwrap().request;

    assert_eq!(request.resident_key, ResidentKey::Preferred);
}

/// Core always names the relying party, but the instance host is the same value.
#[test]
fn test_missing_relying_party_id_falls_back_to_the_instance_host() {
    let without_rp_id = challenge_json().replace(r#""id":"vpn.example.com""#, r#""id":null"#);

    let request = prepare_registration(&without_rp_id, &origin())
        .unwrap()
        .request;

    assert_eq!(request.rp_id, "vpn.example.com");
}

/// Platform dialogs name the relying party and will not accept an empty name.
#[test]
fn test_missing_relying_party_name_falls_back_to_the_id() {
    let without_rp_name = challenge_json().replace(r#""name":"Defguard""#, r#""name":null"#);

    let request = prepare_registration(&without_rp_name, &origin())
        .unwrap()
        .request;

    assert_eq!(request.rp_name, "vpn.example.com");
}

#[test]
fn test_unparseable_challenge_is_reported_as_malformed() {
    let err = prepare_registration("not json", &origin()).err().unwrap();

    assert!(matches!(err, Fido2Error::MalformedChallenge(_)));
}

#[test]
fn test_credential_is_serialized_the_way_webauthn_parses_it() {
    let registration = Registration {
        credential_id: b"credential-id".to_vec(),
        authenticator_data: vec![1, 2, 3],
    };

    let credential: Value =
        serde_json::from_str(&credential_json(&registration, b"client-data").unwrap()).unwrap();

    assert_eq!(credential["type"], "public-key");
    let expected_id = BASE64_URL_SAFE_NO_PAD.encode(b"credential-id");
    assert_eq!(credential["id"], expected_id);
    assert_eq!(credential["rawId"], expected_id);
    assert_eq!(
        credential["response"]["clientDataJSON"],
        BASE64_URL_SAFE_NO_PAD.encode(b"client-data")
    );
    assert!(credential["extensions"].is_object());

    // The attestation object is CBOR, and carries the authenticator data untouched.
    let decoded = BASE64_URL_SAFE_NO_PAD
        .decode(
            credential["response"]["attestationObject"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
    let object: CborValue = ciborium::from_reader(decoded.as_slice()).unwrap();
    let entries = object.as_map().unwrap();
    let field = |name: &str| {
        entries
            .iter()
            .find(|(key, _)| key.as_text() == Some(name))
            .map(|(_, value)| value.clone())
            .unwrap()
    };
    assert_eq!(field("fmt").as_text(), Some("none"));
    assert!(field("attStmt").as_map().unwrap().is_empty());
    assert_eq!(field("authData").as_bytes(), Some(&vec![1, 2, 3]));
}

#[test]
fn test_decode_base64_accepts_every_alphabet() {
    use base64::prelude::{BASE64_STANDARD, BASE64_STANDARD_NO_PAD, BASE64_URL_SAFE};

    // Url-safe (`_-`) and standard (`/+`) encodings differ here, so a one-alphabet decoder fails.
    let raw = vec![0xff_u8, 0xfe, 0xfd, 0x00];

    for encoded in [
        // What webauthn-rs writes for a CredentialID.
        BASE64_URL_SAFE_NO_PAD.encode(&raw),
        BASE64_URL_SAFE.encode(&raw),
        BASE64_STANDARD.encode(&raw),
        BASE64_STANDARD_NO_PAD.encode(&raw),
    ] {
        assert_eq!(
            decode_base64(&encoded).expect("should decode {encoded}"),
            raw,
            "failed to decode {encoded}"
        );
    }
}

#[test]
fn test_decode_base64_rejects_garbage() {
    assert!(decode_base64("not base64!!").is_err());
}

#[test]
fn test_malformed_credential_id_is_reported_as_malformed() {
    let err = prepare_assertion("vpn.example.com", "challenge", &["not base64!".to_string()])
        .err()
        .unwrap();

    assert!(matches!(err, Fido2Error::MalformedChallenge(_)));
}

/// The platform API rejects an empty algorithm list while the direct-to-key backend falls back
/// to ES256, so it is refused here to be refused everywhere.
#[test]
fn test_an_empty_algorithm_list_is_refused_before_any_backend_sees_it() {
    let mut challenge: Value = serde_json::from_str(&challenge_json()).unwrap();
    challenge["publicKey"]["pubKeyCredParams"] = json!([]);

    let err = prepare_registration(&challenge.to_string(), &origin())
        .err()
        .unwrap();

    assert!(matches!(err, Fido2Error::MalformedChallenge(_)));
    assert!(err.to_string().contains("algorithms"));
}
