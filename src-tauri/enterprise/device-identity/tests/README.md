# Device identity contract tests

`contract.rs` exercises the public facade. Hardware-dependent tests use `#[ignore]`
until the backends are implemented and a suitable runner is configured. They require
successful operations and cryptographically valid results. They do not accept
`Unsupported` as a successful outcome and do not mock the backend.

From `src-tauri`:

```sh
cargo test -p device-identity
cargo test -p device-identity --test contract -- --ignored
```

The full hardware suite still requires possession-proof support, which remains a
stub on Linux. Run a single scenario by adding its name before `-- --ignored`.
Linux creation, public enrollment data, and attestation are implemented.

## Contracts

- Capabilities identify the platform backend and its supported identity profile.
- Created identities return caller-owned key data and stable public enrollment data.
- TPM activation recovers a fresh secret chosen by the test verifier.
- Attestation certifies the required identity key and binds the exact opaque Core bytes.
- Changed attestation data, evidence or activation secrets fail independent verification.
- Possession proofs verify only with the attested key and unchanged Core bytes.
- Apple assertion counters increase and replayed assertions are rejected by the verifier.
- Creating another identity preserves the first identity's key and usability.
- TPM rejects corrupted credential blobs and encrypted protection seeds.

The library receives only opaque challenge data. Session expiry and single-use
challenge bookkeeping belong to Core and need separate Core tests.

## Linux and Windows

Use a dedicated test TPM or an isolated software TPM. TPM identity data consists of
the SRK input template, separate public/private blobs for the AK and identity key,
and EK data: its input template, public area, and available certificates.
The caller retains these fields and supplies them to subsequent operations. Private
blobs are protected by the TPM parent, not exported private keys. The library does
not create identity files, database records, or persistent TPM handles.

On Linux, creation assumes empty Owner and Endorsement authorization. It creates a
transient EK using a default tss-esapi profile, preferring ECC P-256 when its NV
certificate is present, otherwise RSA-2048 when its certificate is present. Without
either certificate it uses ECC P-256 and returns an empty certificate list. Other NV
read errors are propagated. A certificate whose public key differs from the default
EK is unsupported. Manufacturer-specific templates and nonces are not handled.

Only the EK certificate from NV is currently returned. The backend does not fetch CA
certificates or contact manufacturer services. Certificate trust belongs to Core.
The stored EK template allows later recreation for credential activation.
`attestation_identity()` projects the stored public data without accessing the TPM.

Do not clear a TPM used for other applications to clean up these tests.

For Linux, `swtpm` is the preferred fixture. The backend reads `TSS2_TCTI`, defaulting
to `device:/dev/tpmrm0`. To select an emulator:

```sh
export TSS2_TCTI='swtpm:host=127.0.0.1,port=2321'
```

This test suite does not start an emulator, provision a persistent EK handle, or
configure OS permissions. These are responsibilities of the runner and backend.
Windows needs a suitable TPM accessible through its transport.

The independent software MakeCredential fixture supports standard RSA-2048 and
NIST P-256 EKs with SHA-256 and AES-128-CFB. AK and identity keys must use ECDSA
P-256/SHA-256. The verifier checks key attributes, TPM Names, Certify structure and
signature, challenge binding, and the recovered activation secret. EK manufacturer
CA policy belongs to Core and is outside these facade tests. An absent EK certificate
is allowed for software TPM fixtures.

The Linux key-creation test can run before the remaining attestation methods are
implemented. It reloads both keys in new TPM contexts using only the returned blobs
and SRK input template, checks their profiles, and uses empty authorization:

```sh
cargo test -p device-identity --lib test_create_identity_returns_reloadable_key_blobs -- --ignored
```

## macOS

Use a macOS 27+ host where App Attest reports support. The test process needs the
signing, provisioning and application identity required by App Attest. Plain
`cargo test` may need an app-hosted runner or packaging to obtain that execution
context. The Rust facade alone does not provide it.

The caller retains the returned App Attest key ID. The key itself remains managed
by the platform, not by a library-owned store. Tests may leave platform-managed keys
behind and consume service quotas.

Initial attestation needs access to Apple services. Assertions are verified locally.
Set:

```sh
export DEFGUARD_APP_ATTEST_APP_ID='TEAM_ID.bundle_id'
export DEFGUARD_APP_ATTEST_ENVIRONMENT='development' # or production
export DEFGUARD_APP_ATTEST_ROOT_CA='/absolute/path/to/apple-app-attestation-root.pem'
```

Obtain the App Attestation Root CA from Apple's official publication, review it and
pin it in the runner configuration. This is the App Attest trust anchor, distinct
from the Apple Enterprise Attestation Root used by Managed Device Attestation.
Never take the trusted root from the attestation response being tested.

OpenSSL 3 must be available as `openssl` for certificate chain and validity checks.
System trust directories and stores are disabled so only the configured root is trusted.
The Rust verifier also checks the certificate nonce, relying party, environment,
credential ID, P-256 public key and assertion signature/counter.

The hardware scenarios are serialized within the test binary. They create several
keys and may consume Apple attestation quotas. Prefer individual scenarios during
backend development, and apply service-specific retry/backoff in the runner rather
than repeatedly rerunning the full suite.

## Independent verifier

`support/` contains test-only verifier logic and software MakeCredential. It does not
call backend internals or the backend's own verification routines. It is a bounded
fixture for these contracts, not a production Core verifier.

Two ordinary tests use software-generated signed structures to check that the test
verifiers accept valid inputs and reject altered data, keys, signatures and counters.
They run without hardware or Apple service access.

## Linux attestation

```sh
cargo test -p device-identity --test contract test_attestation_verifies_and_rejects_changed_data_or_evidence -- --ignored
cargo test -p device-identity --test contract test_tpm_rejects_a_corrupted_credential_challenge -- --ignored
cargo test -p device-identity --test contract test_attestation_rejects_damaged_identity_without_replacing_keys -- --ignored
```

These scenarios use independent software MakeCredential and attestation verification.
The backend restores the stored SRK and EK templates, loads the AK, activates the
credential using PolicySecret with empty Endorsement authorization, and releases the
EK before loading the identity key. Certify binds SHA-256 of the unchanged request
data, and the returned ECDSA signature uses ASN.1 DER. Invalid challenges and damaged
identity data return errors without creating replacement keys. Failure paths release
TPM resources, and subsequent valid requests must still succeed.

Negative hardware tests intentionally provoke TPM errors, so TSS warnings are expected.

The EK certificate fixtures in `fixtures/` are synthetic, self-signed RSA and ECC
certificates used only to test public-key matching. They are not trust anchors.
