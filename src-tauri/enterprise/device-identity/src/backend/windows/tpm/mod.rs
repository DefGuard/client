use crate::{Capabilities, DeviceIdentityError};
use tpm2_protocol::{
    constant::TPM_MAX_COMMAND_SIZE,
    data::{TpmCc, TpmRc, TpmRcBase, TpmSt},
    frame::{tpm_marshal_command, TpmFrame, TpmMarshalBody, TpmResponse},
    TpmMarshal, TpmResult, TpmSized, TpmUnmarshal, TpmWriter,
};

mod capabilities;
mod tbs;

const TPM_HEADER_SIZE: usize = 10;

pub(super) fn read_capabilities() -> Result<Capabilities, DeviceIdentityError> {
    let mut transport = tbs::TbsTransport::open()?;
    capabilities::read(&mut transport)
}

trait Transport {
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
    let mut command_buffer = vec![0; TPM_MAX_COMMAND_SIZE as usize];
    let command_len = {
        let mut writer = TpmWriter::new(&mut command_buffer);
        tpm_marshal_command(&command, TpmSt::NoSessions, &[], &mut writer)
            .map_err(|_| DeviceIdentityError::BackendFailure)?;
        writer.len()
    };
    let response_bytes = transport.submit(&command_buffer[..command_len])?;
    let response =
        TpmResponse::cast(&response_bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
    let response_code = response
        .rc()
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    if response_code != TpmRc::Fmt0(TpmRcBase::Success) {
        return Err(map_tpm_rc(response_code));
    }
    if response
        .tag()
        .map_err(|_| DeviceIdentityError::BackendFailure)?
        != TpmSt::NoSessions
    {
        return Err(DeviceIdentityError::BackendFailure);
    }
    receive(&response.as_bytes()[TPM_HEADER_SIZE..])
}

fn receive<T: TpmUnmarshal>(bytes: &[u8]) -> Result<T, DeviceIdentityError> {
    let (value, remaining) =
        T::unmarshal(bytes).map_err(|_| DeviceIdentityError::BackendFailure)?;
    if !remaining.is_empty() {
        return Err(DeviceIdentityError::BackendFailure);
    }
    Ok(value)
}

fn map_tpm_rc(response_code: TpmRc) -> DeviceIdentityError {
    match response_code {
        TpmRc::Warn(TpmRcBase::Retry | TpmRcBase::Yielded | TpmRcBase::Lockout) => {
            DeviceIdentityError::TemporarilyUnavailable
        }
        TpmRc::Fmt1(code)
            if matches!(
                code.base,
                TpmRcBase::Hash | TpmRcBase::Curve | TpmRcBase::Scheme | TpmRcBase::KeySize
            ) =>
        {
            DeviceIdentityError::Unsupported
        }
        _ => DeviceIdentityError::BackendFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpm2_protocol::{basic::TpmUint32, frame::TpmCommand};

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
        let mut bytes = Vec::with_capacity(10 + body.len());
        bytes.extend_from_slice(&0x8001_u16.to_be_bytes());
        bytes.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
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
    fn test_send_rejects_malformed_or_trailing_response_data() {
        let parameters = marshal(&TpmUint32::from(0)).expect("marshal request parameter");
        let mut transport = FakeTransport {
            command: Vec::new(),
            response: vec![0x80, 0x01, 0, 0, 0, 10, 0, 0, 0],
        };
        let result: Result<TpmUint32, _> = send(&mut transport, TpmCc::GetCapability, &parameters);
        assert_eq!(result, Err(DeviceIdentityError::BackendFailure));

        transport.response = response(0, &[0, 0, 0, 1, 0]);
        let result: Result<TpmUint32, _> = send(&mut transport, TpmCc::GetCapability, &parameters);
        assert_eq!(result, Err(DeviceIdentityError::BackendFailure));
    }
}
