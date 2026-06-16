//! Mobile-approve MFA QR code payload construction and rendering.
//!
//! QR payload format (matches the desktop client):
//!   Base64(JSON{token, challenge, instance_id})
//!
//! The payload is never logged via tracing.

#[cfg(not(test))]
use std::io::{stderr, IsTerminal};
use std::path::Path;

use base64::{prelude::BASE64_STANDARD, Engine as _};
#[cfg(not(test))]
use image::imageops::{resize, FilterType};
use image::Luma;
#[cfg(not(test))]
use qrcode::render::unicode::Dense1x2;
use qrcode::QrCode;

use crate::state::CliError;

#[cfg(not(test))]
// Target minimum size (in pixels) for QR PNG output.
const QR_PNG_MIN_SIZE: u32 = 300;

/// Build the base64-encoded QR payload for mobile-approve MFA.
///
/// The payload is a JSON object containing the MFA session token, the
/// biometric challenge, and the instance UUID.  This matches the format
/// expected by the Defguard mobile app.
pub(crate) fn build_qr_payload(token: &str, challenge: &str, instance_id: &str) -> String {
    let json = serde_json::json!({
        "token": token,
        "challenge": challenge,
        "instance_id": instance_id,
    });
    let raw = serde_json::to_string(&json).expect("JSON serialization is infallible");
    BASE64_STANDARD.encode(raw.as_bytes())
}

/// Render the QR code for a payload string to available output(s).
///
/// * When **stderr is a TTY**, prints a Unicode `Dense1x2` QR to stderr.
/// * When **`qr_file` is `Some`**, writes a PNG image to that path.
/// * If **neither** output is viable (non-TTY + no `qr_file`), returns
///   [`CliError::InvalidInput`] with guidance to use `--qr-file`.
///
/// Both outputs are rendered independently -- when both are available the
/// user sees the terminal QR *and* gets a PNG file.
#[cfg(not(test))]
pub(crate) fn render_qr(payload: &str, qr_file: Option<&str>) -> Result<(), CliError> {
    let is_tty = stderr().is_terminal();

    if !is_tty && qr_file.is_none() {
        return Err(CliError::InvalidInput(
            "No QR display available (stderr is not a TTY). \
             Use --qr-file <path> to save the QR as a PNG image."
                .into(),
        ));
    }

    if is_tty {
        let code = QrCode::new(payload.as_bytes())
            .map_err(|e| CliError::Other(format!("Failed to generate QR code: {e}")))?;
        let rendered = code.render::<Dense1x2>().build();
        eprintln!("{rendered}");
    }

    if let Some(path) = qr_file {
        let code = QrCode::new(payload.as_bytes())
            .map_err(|e| CliError::Other(format!("Failed to generate QR code: {e}")))?;
        let image = code.render::<Luma<u8>>().build();
        // Scale up so the QR is large enough to scan.
        // Nearest-neighbour preserves sharp module edges.
        let max_dim = image.width().max(image.height());
        let scale = (QR_PNG_MIN_SIZE + max_dim - 1) / max_dim.max(1);
        let scaled = resize(
            &image,
            image.width() * scale,
            image.height() * scale,
            FilterType::Nearest,
        );
        scaled
            .save(Path::new(path))
            .map_err(|e| CliError::Other(format!("Failed to save QR image: {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn render_qr(_payload: &str, qr_file: Option<&str>) -> Result<(), CliError> {
    // Test mode: never render to the terminal.  Write to --qr-file
    // only so that integration tests can verify the file was produced.
    if let Some(path) = qr_file {
        let code = QrCode::new(_payload.as_bytes())
            .map_err(|e| CliError::Other(format!("Failed to generate QR code: {e}")))?;
        let image = code.render::<Luma<u8>>().build();
        image
            .save(Path::new(path))
            .map_err(|e| CliError::Other(format!("Failed to save QR image: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_qr_payload_bytes() {
        let payload = build_qr_payload("tok-abc", "chal-xyz", "uuid-001");
        // Decode and verify the JSON structure.
        let decoded = BASE64_STANDARD.decode(&payload).expect("valid base64");
        let json: serde_json::Value = serde_json::from_slice(&decoded).expect("valid JSON");
        assert_eq!(json["token"], "tok-abc");
        assert_eq!(json["challenge"], "chal-xyz");
        assert_eq!(json["instance_id"], "uuid-001");
    }

    #[test]
    fn test_build_qr_payload_deterministic() {
        // Same inputs must produce identical payloads.
        let a = build_qr_payload("tok", "chal", "inst");
        let b = build_qr_payload("tok", "chal", "inst");
        assert_eq!(a, b);
    }
}
