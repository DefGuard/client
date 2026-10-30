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

The second command is expected to fail on the current stubs. Run a single scenario
by adding its name before `-- --ignored`.

## Contracts

- Capabilities identify the platform backend and its supported identity profile.
- Created identities have nonempty local references and stable public enrollment data.
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

Use a dedicated test TPM or an isolated software TPM. Tests create persistent local
identity records and may leave them behind because the facade has no deletion
operation yet. Do not clear a TPM used for other applications to clean up these tests.

For Linux, `swtpm` is the preferred fixture. The backend defaults to
`device:/dev/tpmrm0`. Set `TSS2_TCTI` explicitly to select another transport:

```sh
export TSS2_TCTI='swtpm:host=127.0.0.1,port=2321'
```

An empty or invalid setting fails without falling back to a hardware device.
Transport initialization is implemented, while capability enumeration remains a stub.
The capabilities contract test still fails with `Unsupported` on a running emulator.

`linux_transport.rs` exercises transport errors in isolated child processes, leaving
the parent environment unchanged. It covers missing devices, unavailable emulators
and invalid configuration. Run its permission test as an unprivileged user:

```sh
cargo test -p device-identity --test linux_transport test_capabilities_reports_permission_denied -- --ignored --exact
```

This test suite does not start an emulator, provision a persistent EK handle, or
configure OS permissions. These are responsibilities of the runner and backend.
Windows needs a suitable TPM accessible through its transport and a writable test
identity store.

The independent software MakeCredential fixture supports standard RSA-2048 and
NIST P-256 EKs with SHA-256 and AES-128-CFB. AK and identity keys must use ECDSA
P-256/SHA-256. The verifier checks key attributes, TPM Names, Certify structure and
signature, challenge binding, and the recovered activation secret. EK manufacturer
CA policy belongs to Core and is outside these facade tests. An absent EK certificate
is allowed for software TPM fixtures.

## macOS

Use a macOS 27+ host where App Attest reports support. The test process needs the
signing, provisioning and application identity required by App Attest. Plain
`cargo test` may need an app-hosted runner or packaging to obtain that execution
context. The Rust facade alone does not provide it.

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
