use tpm2_protocol::{
    basic::TpmHandle,
    constant::TPM_MAX_COMMAND_SIZE,
    data::{
        Tpm2bAuth, Tpm2bNonce, TpmCc, TpmRc, TpmRcBase, TpmRh, TpmSt, TpmaSession, TpmsAuthCommand,
    },
    frame::{
        tpm_marshal_command, TpmFrame, TpmHeader, TpmMarshalBody, TpmResponse, TpmUnmarshalBody,
    },
    TpmMarshal, TpmResult, TpmSized, TpmUnmarshal, TpmWriter,
};
use tracing::warn;

use crate::{
    AttestationResponse, Capabilities, CredentialChallenge, DeviceIdentityError, PossessionProof,
    TpmIdentity, TpmKeyBlobs,
};

mod attestation;
mod certify;
mod der;
mod ek;
mod keys;
mod possession;
mod raw_capabilities;
mod tbs;

const TPM_HEADER_SIZE: usize = 10;
const TPM_RESPONSE_CODE_OFFSET: usize = 6;

pub(super) fn read_capabilities() -> Result<Capabilities, DeviceIdentityError> {
    let mut transport = tbs::TbsTransport::open()?;
    raw_capabilities::read(&mut transport)
}

pub(super) fn create_identity() -> Result<TpmIdentity, DeviceIdentityError> {
    let mut transport = tbs::TbsTransport::open()?;
    let capabilities = raw_capabilities::read(&mut transport)?;
    let nv_buffer_max = raw_capabilities::nv_buffer_max(&capabilities);
    raw_capabilities::check_hierarchy_auth(&capabilities)?;
    let ek = ek::read(&mut transport, nv_buffer_max)?;
    let keys::CreatedKeys {
        srk_template,
        ak,
        identity_key,
    } = keys::create(&mut transport)?;
    Ok(TpmIdentity {
        ek,
        srk_template,
        ak,
        identity_key,
    })
}

pub(super) fn attest_identity(
    identity: TpmIdentity,
    data: Vec<u8>,
    challenge: CredentialChallenge,
) -> Result<AttestationResponse, DeviceIdentityError> {
    attestation::attest(identity, data, challenge)
}

pub(super) struct PossessionKey {
    pub(super) srk_template: Vec<u8>,
    pub(super) identity_key: TpmKeyBlobs,
}

pub(super) fn prove_possession(
    key: PossessionKey,
    data: Vec<u8>,
) -> Result<PossessionProof, DeviceIdentityError> {
    let mut transport = tbs::TbsTransport::open()?;
    possession::prove(&mut transport, &key.srk_template, &key.identity_key, &data)
}

pub(super) trait Transport {
    fn submit(&mut self, command: &[u8]) -> Result<Vec<u8>, DeviceIdentityError>;
}

#[derive(Debug)]
struct Command<'a> {
    code: TpmCc,
    parameters: &'a [u8],
}

impl TpmSized for Command<'_> {
    const SIZE: usize = 0;

    fn len(&self) -> usize {
        self.parameters.len()
    }
}

impl TpmMarshal for Command<'_> {
    fn marshal(&self, writer: &mut TpmWriter) -> TpmResult<()> {
        self.marshal_handles(writer)?;
        self.marshal_parameters(writer)
    }
}

impl TpmMarshalBody for Command<'_> {
    fn marshal_handles(&self, _writer: &mut TpmWriter) -> TpmResult<()> {
        Ok(())
    }

    fn marshal_parameters(&self, writer: &mut TpmWriter) -> TpmResult<()> {
        writer.write_bytes(self.parameters)
    }
}

impl TpmFrame for Command<'_> {
    fn cc(&self) -> TpmCc {
        self.code
    }

    fn handles(&self) -> usize {
        0
    }
}

fn marshal<T: TpmMarshal + TpmSized>(value: &T) -> Result<Vec<u8>, DeviceIdentityError> {
    let mut bytes = vec![0; value.len()];
    let mut writer = TpmWriter::new(&mut bytes);
    value
        .marshal(&mut writer)
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    Ok(writer.as_bytes().to_vec())
}

fn send<R: TpmUnmarshal>(
    transport: &mut impl Transport,
    command_code: TpmCc,
    command_parameters: &[u8],
) -> Result<R, DeviceIdentityError> {
    let command = Command {
        code: command_code,
        parameters: command_parameters,
    };
    let response_bytes = submit_frame(transport, &command, &[])?;
    let response = checked_response(&response_bytes, TpmSt::NoSessions, map_tpm_rc)?;
    receive(&response.as_bytes()[TPM_HEADER_SIZE..])
}

fn session_tag(sessions: &[TpmsAuthCommand]) -> TpmSt {
    if sessions.is_empty() {
        TpmSt::NoSessions
    } else {
        TpmSt::Sessions
    }
}

fn submit_frame<C: TpmFrame>(
    transport: &mut impl Transport,
    command: &C,
    sessions: &[TpmsAuthCommand],
) -> Result<Vec<u8>, DeviceIdentityError> {
    let tag = session_tag(sessions);
    let mut command_buffer = vec![0; TPM_MAX_COMMAND_SIZE as usize];
    let command_len = {
        let mut writer = TpmWriter::new(&mut command_buffer);
        tpm_marshal_command(command, tag, sessions, &mut writer)
            .map_err(|_| DeviceIdentityError::BackendFailure)?;
        writer.len()
    };
    transport.submit(&command_buffer[..command_len])
}

fn decode_frame_with_mapper<R: TpmUnmarshalBody>(
    bytes: &[u8],
    expected_tag: TpmSt,
    map_response_code: fn(TpmRc) -> DeviceIdentityError,
) -> Result<R, DeviceIdentityError> {
    checked_response(bytes, expected_tag, map_response_code)?
        .unmarshal::<R>()
        .map_err(|_| DeviceIdentityError::BackendFailure)
}

fn checked_response<'a>(
    bytes: &'a [u8],
    expected_tag: TpmSt,
    map_response_code: fn(TpmRc) -> DeviceIdentityError,
) -> Result<&'a TpmResponse, DeviceIdentityError> {
    let response = TpmResponse::cast(bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
    let response_code = response
        .rc()
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    if response_code != TpmRc::Fmt0(TpmRcBase::Success) {
        return Err(map_response_code(response_code));
    }
    if response
        .tag()
        .map_err(|_| DeviceIdentityError::BackendFailure)?
        != expected_tag
    {
        return Err(DeviceIdentityError::BackendFailure);
    }
    Ok(response)
}

pub(super) fn send_frame<C: TpmFrame, R: TpmUnmarshalBody>(
    transport: &mut impl Transport,
    command: &C,
) -> Result<R, DeviceIdentityError> {
    send_with_sessions_and_mapper(transport, command, &[], map_tpm_rc)
}

pub(super) fn send_authenticated<C: TpmFrame, R: TpmUnmarshalBody>(
    transport: &mut impl Transport,
    command: &C,
) -> Result<R, DeviceIdentityError> {
    send_with_sessions_and_mapper(transport, command, &[password_auth()], map_tpm_rc)
}

pub(super) fn send_with_sessions_and_mapper<C: TpmFrame, R: TpmUnmarshalBody>(
    transport: &mut impl Transport,
    command: &C,
    sessions: &[TpmsAuthCommand],
    map_response_code: fn(TpmRc) -> DeviceIdentityError,
) -> Result<R, DeviceIdentityError> {
    let tag = session_tag(sessions);
    let bytes = submit_frame(transport, command, sessions)?;
    decode_frame_with_mapper(&bytes, tag, map_response_code)
}

pub(super) fn send_with_sessions_and_tpm_error<C: TpmFrame, R: TpmUnmarshalBody>(
    transport: &mut impl Transport,
    command: &C,
    sessions: &[TpmsAuthCommand],
) -> Result<Result<R, TpmRc>, DeviceIdentityError> {
    let bytes = submit_frame(transport, command, sessions)?;
    let response = TpmResponse::cast(&bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
    let response_code = response
        .rc()
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    if response_code != TpmRc::Fmt0(TpmRcBase::Success) {
        return Ok(Err(response_code));
    }
    decode_frame_with_mapper(&bytes, session_tag(sessions), map_tpm_rc).map(Ok)
}

pub(super) fn with_primary<T: Transport, R>(
    transport: &mut T,
    hierarchy: TpmRh,
    public: tpm2_protocol::data::TpmtPublic,
    map_response_code: fn(TpmRc) -> DeviceIdentityError,
    operation: impl FnOnce(&mut T, TpmHandle) -> Result<R, DeviceIdentityError>,
) -> Result<R, DeviceIdentityError> {
    use tpm2_protocol::{
        data::{
            Tpm2bData, Tpm2bPublic, Tpm2bSensitiveCreate, TpmlPcrSelection, TpmsSensitiveCreate,
        },
        frame::{TpmCreatePrimaryCommand, TpmCreatePrimaryResponse},
    };

    let command = TpmCreatePrimaryCommand {
        handles: [TpmHandle::from(hierarchy.value())],
        in_sensitive: Tpm2bSensitiveCreate {
            inner: TpmsSensitiveCreate::default(),
        },
        in_public: Tpm2bPublic { inner: public },
        outside_info: Tpm2bData::new(),
        creation_pcr: TpmlPcrSelection::default(),
    };
    with_transient_response::<T, _, TpmCreatePrimaryResponse, _>(
        transport,
        &command,
        &[password_auth()],
        map_response_code,
        |transport, handle, _| operation(transport, handle),
    )
}

pub(super) fn with_transient_response<T: Transport, C: TpmFrame, R: TpmUnmarshalBody, O>(
    transport: &mut T,
    command: &C,
    sessions: &[TpmsAuthCommand],
    map_response_code: fn(TpmRc) -> DeviceIdentityError,
    operation: impl FnOnce(&mut T, TpmHandle, R) -> Result<O, DeviceIdentityError>,
) -> Result<O, DeviceIdentityError>
where
    R: TpmHeader,
{
    if R::HANDLES != 1 {
        return Err(DeviceIdentityError::BackendFailure);
    }
    let bytes = submit_frame(transport, command, sessions)?;
    let response_code_bytes = bytes
        .get(TPM_RESPONSE_CODE_OFFSET..TPM_HEADER_SIZE)
        .ok_or(DeviceIdentityError::BackendFailure)?;
    if response_code_bytes != [0; 4] {
        let response =
            TpmResponse::cast(&bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
        let response_code = response
            .rc()
            .map_err(|_| DeviceIdentityError::BackendFailure)?;
        return Err(map_response_code(response_code));
    }
    let handle_bytes = bytes
        .get(TPM_HEADER_SIZE..TPM_HEADER_SIZE + TpmHandle::SIZE)
        .ok_or(DeviceIdentityError::BackendFailure)?;
    let (handle, remaining) =
        TpmHandle::unmarshal(handle_bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
    if !remaining.is_empty() {
        return Err(DeviceIdentityError::BackendFailure);
    }
    with_transient(transport, handle, |transport, handle| {
        let response =
            decode_frame_with_mapper::<R>(&bytes, session_tag(sessions), map_response_code)?;
        operation(transport, handle, response)
    })
}

pub(super) fn with_transient<T: Transport, R>(
    transport: &mut T,
    handle: TpmHandle,
    operation: impl FnOnce(&mut T, TpmHandle) -> Result<R, DeviceIdentityError>,
) -> Result<R, DeviceIdentityError> {
    let result = operation(transport, handle);
    let flush_result = flush_context(transport, handle);
    if let Err(ref flush_error) = flush_result {
        warn!(?flush_error, "Failed to flush TPM transient object");
    }
    match result {
        Ok(value) => flush_result.map(|()| value),
        Err(error) => Err(error),
    }
}

fn flush_context(
    transport: &mut impl Transport,
    handle: TpmHandle,
) -> Result<(), DeviceIdentityError> {
    use tpm2_protocol::frame::{TpmFlushContextCommand, TpmFlushContextResponse};

    let command = TpmFlushContextCommand {
        handles: [],
        flush_handle: handle,
    };
    send_frame::<_, TpmFlushContextResponse>(transport, &command).map(|_| ())
}

fn password_auth() -> TpmsAuthCommand {
    TpmsAuthCommand {
        session_handle: TpmHandle::from(TpmRh::Pw.value()),
        nonce: Tpm2bNonce::new(),
        session_attributes: TpmaSession::default(),
        hmac: Tpm2bAuth::new(),
    }
}

fn receive<T: TpmUnmarshal>(bytes: &[u8]) -> Result<T, DeviceIdentityError> {
    let (value, remaining) =
        T::unmarshal(bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
    if !remaining.is_empty() {
        return Err(DeviceIdentityError::BackendFailure);
    }
    Ok(value)
}

pub(super) fn map_identity_rc(response_code: TpmRc) -> DeviceIdentityError {
    match response_code {
        TpmRc::Fmt1(code)
            if matches!(
                code.base,
                TpmRcBase::Integrity
                    | TpmRcBase::Binding
                    | TpmRcBase::Parent
                    | TpmRcBase::Key
                    | TpmRcBase::Sensitive
                    | TpmRcBase::Size
                    | TpmRcBase::Value
            ) =>
        {
            DeviceIdentityError::IdentityLost
        }
        other => map_tpm_rc(other),
    }
}

pub(super) fn map_activation_rc(response_code: TpmRc) -> DeviceIdentityError {
    match response_code {
        TpmRc::Fmt1(code)
            if matches!(
                code.base,
                TpmRcBase::Integrity
                    | TpmRcBase::Value
                    | TpmRcBase::Size
                    | TpmRcBase::EccPoint
                    | TpmRcBase::Insufficient
            ) =>
        {
            DeviceIdentityError::InvalidChallenge
        }
        other => map_tpm_rc(other),
    }
}

fn map_tpm_rc(response_code: TpmRc) -> DeviceIdentityError {
    match response_code {
        TpmRc::Warn(TpmRcBase::Retry | TpmRcBase::Yielded | TpmRcBase::Lockout) => {
            DeviceIdentityError::TemporarilyUnavailable
        }
        TpmRc::Fmt1(code) if matches!(code.base, TpmRcBase::AuthFail | TpmRcBase::BadAuth) => {
            DeviceIdentityError::PermissionDenied
        }
        TpmRc::Fmt1(code)
            if matches!(
                code.base,
                TpmRcBase::Hash
                    | TpmRcBase::Curve
                    | TpmRcBase::Scheme
                    | TpmRcBase::KeySize
                    | TpmRcBase::Asymmetric
                    | TpmRcBase::Symmetric
                    | TpmRcBase::Kdf
                    | TpmRcBase::Mode
            ) =>
        {
            DeviceIdentityError::Unsupported
        }
        _ => DeviceIdentityError::BackendFailure,
    }
}

#[cfg(test)]
mod tests {
    use tpm2_protocol::{basic::TpmUint32, frame::TpmCommand};

    use super::*;

    #[test]
    #[ignore = "Requires a Windows TPM"]
    fn test_created_keys_reload_in_fresh_tbs_connection() {
        let identity = create_identity().expect("create TPM identity");
        let mut transport = tbs::TbsTransport::open().expect("open a fresh TBS context");
        keys::reload(&mut transport, &identity).expect("reload both keys under the recreated SRK");
    }

    struct FakeTransport {
        command: Vec<u8>,
        response: Vec<u8>,
    }

    impl Transport for FakeTransport {
        fn submit(&mut self, command: &[u8]) -> Result<Vec<u8>, DeviceIdentityError> {
            self.command = command.to_vec();
            Ok(self.response.clone())
        }
    }

    fn response(response_code: u32, body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(TPM_HEADER_SIZE + body.len());
        bytes.extend_from_slice(&0x8001_u16.to_be_bytes());
        bytes.extend_from_slice(&((TPM_HEADER_SIZE + body.len()) as u32).to_be_bytes());
        bytes.extend_from_slice(&response_code.to_be_bytes());
        bytes.extend_from_slice(body);
        bytes
    }

    #[test]
    fn test_send_marshals_command_and_parses_typed_response() {
        let request = TpmUint32::from(0x0102_0304);
        let parameters = marshal(&request).expect("marshal request parameter");
        let mut transport = FakeTransport {
            command: Vec::new(),
            response: response(0, &0x1122_3344_u32.to_be_bytes()),
        };

        let result: TpmUint32 =
            send(&mut transport, TpmCc::GetCapability, &parameters).expect("valid TPM response");

        let command = TpmCommand::cast(&transport.command).expect("valid command frame");
        assert_eq!(command.cc().expect("command code"), TpmCc::GetCapability);
        let parsed = receive::<TpmUint32>(command.parameters().expect("command parameters"))
            .expect("valid command body");
        assert_eq!(u32::from(parsed), 0x0102_0304);
        assert_eq!(u32::from(result), 0x1122_3344);
    }

    #[test]
    fn test_send_maps_tpm_response_codes() {
        let parameters = marshal(&TpmUint32::from(0)).expect("marshal request parameter");
        let mut transport = FakeTransport {
            command: Vec::new(),
            response: Vec::new(),
        };

        for (raw_code, expected) in [
            (0x0000_0083, DeviceIdentityError::Unsupported),
            (0x0000_008e, DeviceIdentityError::PermissionDenied),
            (0x0000_00a2, DeviceIdentityError::PermissionDenied),
            (0x0000_0081, DeviceIdentityError::Unsupported),
            (0x0000_0096, DeviceIdentityError::Unsupported),
            (0x0000_008c, DeviceIdentityError::Unsupported),
            (0x0000_0089, DeviceIdentityError::Unsupported),
            (0x0000_0101, DeviceIdentityError::BackendFailure),
            (0x0000_0922, DeviceIdentityError::TemporarilyUnavailable),
            (0x0000_0921, DeviceIdentityError::TemporarilyUnavailable),
        ] {
            transport.response = response(raw_code, &[]);
            let result: Result<TpmUint32, _> =
                send(&mut transport, TpmCc::GetCapability, &parameters);
            assert_eq!(result, Err(expected));
        }
    }

    #[test]
    fn test_with_transient_flushes_after_operation_error() {
        let mut transport = FakeTransport {
            command: Vec::new(),
            response: response(0, &[]),
        };
        let result: Result<(), DeviceIdentityError> =
            with_transient(&mut transport, TpmHandle::from(0x8000_0001), |_, _| {
                Err(DeviceIdentityError::Unsupported)
            });
        assert_eq!(result, Err(DeviceIdentityError::Unsupported));
        let command = TpmCommand::cast(&transport.command).expect("valid flush command");
        assert_eq!(command.cc().expect("command code"), TpmCc::FlushContext);
    }

    #[test]
    fn test_send_rejects_malformed_or_trailing_response_data() {
        let parameters = marshal(&TpmUint32::from(0)).expect("marshal request parameter");
        let mut transport = FakeTransport {
            command: Vec::new(),
            response: vec![0x80, 0x01, 0, 0, 0, TPM_HEADER_SIZE as u8, 0, 0, 0],
        };
        let result: Result<TpmUint32, _> = send(&mut transport, TpmCc::GetCapability, &parameters);
        assert_eq!(result, Err(DeviceIdentityError::BackendFailure));

        transport.response = response(0, &[0, 0, 0, 1, 0]);
        let result: Result<TpmUint32, _> = send(&mut transport, TpmCc::GetCapability, &parameters);
        assert_eq!(result, Err(DeviceIdentityError::BackendFailure));
    }
}
