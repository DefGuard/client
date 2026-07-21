//! Connect-time VPN MFA thin wrapper over `defguard_core::mfa`.
//!
//! Supports TOTP, email, OIDC, and mobile-approve methods.
//!
//! CLI-specific code (method resolution from flags, browser-open, QR
//! rendering, TTY prompting) stays here; all HTTP, WebSocket, and poll
//! logic delegates to `defguard_core::mfa`.

use defguard_client_proto::defguard::{
    client_types::MfaMethod, enterprise::posture::v2::DevicePostureData,
};
use defguard_core::{
    database::{
        models::{
            instance::Instance,
            location::{infer_mfa_method, Location, LocationMfaMethod, LocationMfaMode},
            wireguard_keys::WireguardKeys,
            Id,
        },
        DbPool,
    },
    mfa,
    proto::client_types::{ClientMfaFinishRequest, ClientMfaStartRequest},
};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, info, warn};
use url::Url;

use crate::{
    mfa_code::{obtain_code, CodeSource, MfaContext},
    mfa_qr,
    state::CliError,
};

/// Convert a `defguard_core::mfa::MfaError` into a [`CliError`].
fn into_cli(err: mfa::MfaError) -> CliError {
    let msg = err.to_string();
    match err {
        mfa::MfaError::NetworkError { .. }
        | mfa::MfaError::ProxyError { .. }
        | mfa::MfaError::Other { .. } => CliError::Other(msg),
        mfa::MfaError::MfaRejected { .. }
        | mfa::MfaError::PostureRejected { .. }
        | mfa::MfaError::Timeout => CliError::MfaFailed(msg),
        mfa::MfaError::Cancelled => CliError::Cancelled(msg),
    }
}

/// Resolve the effective MFA method for a location.
///
/// When `method_override` is `Some`, parses it into [`MfaMethod`]; otherwise
/// delegates to [`infer_method`] which respects the location's
/// [`LocationMfaMode`].
///
/// Rejects `--mfa-method oidc` on Internal-mode locations.
pub(crate) fn resolve_method(
    location: &Location<Id>,
    method_override: Option<&str>,
) -> Result<MfaMethod, CliError> {
    let method = if let Some(raw) = method_override {
        let method = parse_method(raw)?;
        // OIDC override on an Internal-mode location will be rejected by the
        // server. Fail early to give the user a clear error before I/O.
        if method == MfaMethod::Oidc && location.location_mfa_mode == LocationMfaMode::Internal {
            return Err(CliError::InvalidInput(
                "--mfa-method oidc is only valid for locations that use external (OIDC) MFA."
                    .into(),
            ));
        }
        method
    } else {
        infer_method(location)
    };

    Ok(method)
}

/// Validate CLI flags against the resolved MFA method.
///
/// * `--code` / `--code-command` are incompatible with OIDC and mobile-approve
///   (neither method accepts textual codes).
/// * `--qr-file` is only valid for mobile-approve MFA.
pub(crate) fn validate_mfa_flags(
    method: MfaMethod,
    location_name: &str,
    code: Option<&str>,
    code_command: Option<&str>,
    qr_file: Option<&str>,
) -> Result<(), CliError> {
    if matches!(method, MfaMethod::Oidc | MfaMethod::MobileApprove)
        && (code.is_some() || code_command.is_some())
    {
        return Err(CliError::InvalidInput(format!(
            "location '{location_name}' cannot use --code / --code-command with {method:?} MFA",
        )));
    }

    if method != MfaMethod::MobileApprove && qr_file.is_some() {
        return Err(CliError::InvalidInput(
            "--qr-file is only valid with mobile-approve MFA".into(),
        ));
    }

    Ok(())
}

/// Run the VPN MFA handshake for a location (TOTP or email).
///
/// The HTTP calls are handled by `defguard_core::mfa`; this function
/// handles CLI-specific code sourcing (TTY / --code / --code-command).
pub(crate) async fn authorize(
    location: &Location<Id>,
    source: &CodeSource,
    instance: &Instance<Id>,
    method: MfaMethod,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
) -> Result<SecretString, CliError> {
    // Reject methods not yet supported by the CLI before doing any I/O.
    // OIDC/MobileApprove are not "unsupported" - they have dedicated code
    // paths (authorize_oidc / authorize_mobile_approve). This catch-all is a
    // defense-in-depth barrier that emits a clear error if they land here.
    match method {
        MfaMethod::Biometric => {
            return Err(CliError::MfaFailed(format!(
                "MFA method {method:?} is not supported by the CLI. Use the mobile client."
            )));
        }
        MfaMethod::MobileApprove => {
            return Err(CliError::Other(
                "Internal error: MobileApprove MFA must use authorize_mobile_approve, not authorize"
                    .into(),
            ));
        }
        MfaMethod::Oidc => {
            return Err(CliError::Other(
                "Internal error: OIDC MFA must use authorize_oidc, not authorize".into(),
            ));
        }
        _ => {}
    }

    let wireguard_keys = WireguardKeys::find_by_instance_id(pool, instance.id)
        .await
        .map_err(|e| CliError::Other(e.to_string()))?
        .ok_or_else(|| {
            CliError::Other(format!(
                "WireGuard keys not found for instance {}",
                instance.name
            ))
        })?;

    let proxy_url = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?;
    check_proxy_scheme(&proxy_url);

    debug!("Starting MFA session for location {}", location.name);
    let request = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey,
        method: method as i32,
        posture_data,
    };
    let info = mfa::mfa_start(proxy_url.clone(), request)
        .await
        .map_err(into_cli)?;

    let ctx = MfaContext {
        instance: instance.name.clone(),
        location: location.name.clone(),
    };
    let code = obtain_code(source, &ctx)?;

    let finish_req = ClientMfaFinishRequest {
        token: info.token,
        code: Some(code.expose_secret().to_string()),
        auth_pub_key: None,
    };
    let psk = mfa::mfa_finish_code(proxy_url, finish_req)
        .await
        .map_err(into_cli)?;

    info!("MFA session completed, preshared key obtained");
    Ok(SecretString::from(psk.preshared_key))
}

/// Run the OIDC MFA flow for an external-IdP location.
///
/// Opens the system browser and delegates the HTTP poll to
/// `defguard_core::mfa::poll_openid_mfa`.
///
/// When `json_mode` is true, progress messages on stderr are suppressed so
/// that `--json` output consumers only see the final result/error.
pub(crate) async fn authorize_oidc(
    location: &Location<Id>,
    instance: &Instance<Id>,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
    json_mode: bool,
) -> Result<SecretString, CliError> {
    let wireguard_keys = WireguardKeys::find_by_instance_id(pool, instance.id)
        .await
        .map_err(|e| CliError::Other(e.to_string()))?
        .ok_or_else(|| {
            CliError::Other(format!(
                "WireGuard keys not found for instance {}",
                instance.name
            ))
        })?;

    let proxy_url = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?;
    check_proxy_scheme(&proxy_url);

    debug!("Starting OIDC MFA session for location {}", location.name);
    let request = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey,
        method: MfaMethod::Oidc as i32,
        posture_data,
    };
    let info = mfa::mfa_start(proxy_url.clone(), request)
        .await
        .map_err(into_cli)?;

    let mut browser_url = proxy_url
        .join("openid/mfa")
        .map_err(|e| CliError::Other(format!("Failed to build OIDC MFA URL: {e}")))?;
    browser_url
        .query_pairs_mut()
        .append_pair("token", &info.token);

    if !json_mode {
        eprintln!("Open this URL to authenticate:");
        eprintln!("  {browser_url}");
        eprintln!("Waiting for authentication... (Ctrl-C to cancel)");
    }
    open_url(browser_url.as_ref(), json_mode);

    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_clone = cancel.clone();
    let ctrlc_handle = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
    });

    let result = mfa::poll_openid_mfa(proxy_url, info.token, cancel).await;
    ctrlc_handle.abort();

    let psk = result.map_err(into_cli)?;
    info!("OIDC MFA session completed, preshared key obtained");
    Ok(SecretString::from(psk.preshared_key))
}

/// Run the mobile-approve MFA flow.
///
/// Displays a QR code (terminal and/or `--qr-file` PNG) and delegates the
/// WebSocket connection to `defguard_core::mfa::connect_mobile_approve`.
///
/// When `json_mode` is true, progress messages on stderr are suppressed so
/// that `--json` output consumers only see the final result/error.
pub(crate) async fn authorize_mobile_approve(
    location: &Location<Id>,
    instance: &Instance<Id>,
    posture_data: Option<DevicePostureData>,
    qr_file: Option<&str>,
    pool: &DbPool,
    json_mode: bool,
) -> Result<SecretString, CliError> {
    let wireguard_keys = WireguardKeys::find_by_instance_id(pool, instance.id)
        .await
        .map_err(|e| CliError::Other(e.to_string()))?
        .ok_or_else(|| {
            CliError::Other(format!(
                "WireGuard keys not found for instance {}",
                instance.name
            ))
        })?;

    let proxy_url = Url::parse(&instance.proxy_url)
        .map_err(|e| CliError::Other(format!("Invalid proxy URL: {e}")))?;
    check_proxy_scheme(&proxy_url);

    debug!(
        "Starting mobile-approve MFA session for location {}",
        location.name
    );
    let request = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey,
        method: MfaMethod::MobileApprove as i32,
        posture_data,
    };
    let info = mfa::mfa_start(proxy_url.clone(), request)
        .await
        .map_err(into_cli)?;

    let challenge = info.challenge.ok_or_else(|| {
        CliError::Other("Proxy did not return a challenge for mobile-approve MFA".into())
    })?;

    let payload = mfa_qr::build_qr_payload(&info.token, &challenge, &instance.uuid);
    mfa_qr::render_qr(&payload, qr_file, json_mode)?;
    if !json_mode {
        eprintln!("Waiting for mobile approval... (Ctrl-C to cancel)");
    }

    let ws_url = mfa::derive_ws_url(&proxy_url, &info.token).map_err(into_cli)?;

    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_clone = cancel.clone();
    let ctrlc_handle = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
    });

    let result = mfa::connect_mobile_approve(&ws_url, cancel).await;
    ctrlc_handle.abort();

    let psk = result.map_err(into_cli)?;
    info!("Mobile-approve MFA completed, preshared key obtained");
    Ok(SecretString::from(psk.preshared_key))
}

/// Parse a `--mfa-method` flag string into the proto [`MfaMethod`] enum.
fn parse_method(raw: &str) -> Result<MfaMethod, CliError> {
    match raw.to_lowercase().as_str() {
        "totp" => Ok(MfaMethod::Totp),
        "email" => Ok(MfaMethod::Email),
        "oidc" => Ok(MfaMethod::Oidc),
        "biometric" => Ok(MfaMethod::Biometric),
        "mobile" | "mobile_approve" => Ok(MfaMethod::MobileApprove),
        _ => Err(CliError::Usage(format!(
            "Invalid --mfa-method '{raw}'. Valid: totp, email, oidc, biometric, mobile."
        ))),
    }
}

/// Determine the MFA method to use for a location.
///
/// Delegates to the core's [`infer_mfa_method`] so that [`LocationMfaMode`]
/// is respected - an External-mode location always uses OIDC, while an
/// Internal-mode location respects the stored preference (defaulting to TOTP).
fn infer_method(location: &Location<Id>) -> MfaMethod {
    let method = infer_mfa_method(location.location_mfa_mode, location.mfa_method);
    match method {
        Some(LocationMfaMethod::Totp) => MfaMethod::Totp,
        Some(LocationMfaMethod::Email) => MfaMethod::Email,
        Some(LocationMfaMethod::Oidc) => MfaMethod::Oidc,
        Some(LocationMfaMethod::Biometric) => MfaMethod::Biometric,
        Some(LocationMfaMethod::MobileApprove) => MfaMethod::MobileApprove,
        None => {
            // infer_mfa_method only returns None for Disabled mode, but this is
            // only called when MFA is enabled. Default to TOTP as a safe fallback.
            MfaMethod::Totp
        }
    }
}

/// Warn if the proxy is not using HTTPS.
///
/// The one-time MFA code and the returned preshared key are sensitive and
/// would travel in cleartext over plain HTTP.
fn check_proxy_scheme(proxy_base: &Url) {
    if proxy_base.scheme() != "https" {
        warn!(
            "Proxy URL '{}' is not HTTPS; secrets will be sent in cleartext.",
            proxy_base.as_str()
        );
    }
}

/// Open a URL in the system browser.
///
/// Production: calls [`webbrowser::open`]; prints a hint to stderr on failure.
/// When `json_mode` is true, the fallback message includes the URL itself since
/// it wasn't already printed above.
/// Tests: no-op (never spawn a browser).
#[cfg(not(test))]
fn open_url(url: &str, json_mode: bool) {
    if webbrowser::open(url).is_err() {
        if json_mode {
            eprintln!("Could not open browser. Open this URL manually: {url}");
        } else {
            eprintln!("Could not open browser. Open the URL above manually.");
        }
    }
}

#[cfg(test)]
fn open_url(_url: &str, _json_mode: bool) {
    // no-op: tests must not spawn a browser
}

#[cfg(test)]
mod tests {
    use defguard_core::database::models::location::ServiceLocationMode;

    use super::*;

    fn location(name: &str, mode: LocationMfaMode) -> Location<Id> {
        Location {
            id: 1,
            instance_id: 1,
            network_id: 1,
            name: name.into(),
            address: "10.0.0.0/24".into(),
            pubkey: "pk".into(),
            endpoint: "1.2.3.4:51820".into(),
            allowed_ips: "0.0.0.0/0".into(),
            dns: None,
            route_all_traffic: false,
            keepalive_interval: 25,
            location_mfa_mode: mode,
            service_location_mode: ServiceLocationMode::Disabled,
            mfa_method: None,
            posture_check_required: false,
        }
    }

    #[test]
    fn test_oidc_location_resolves_to_oidc() {
        let l = location("office", LocationMfaMode::External);
        let method = resolve_method(&l, None).unwrap();
        assert_eq!(method, MfaMethod::Oidc);
    }

    #[test]
    fn test_internal_location_resolves_to_totp() {
        let l = location("office", LocationMfaMode::Internal);
        let method = resolve_method(&l, None).unwrap();
        assert_eq!(method, MfaMethod::Totp);
    }

    #[test]
    fn test_validate_flags_oidc_rejects_code() {
        let err =
            validate_mfa_flags(MfaMethod::Oidc, "office", Some("123456"), None, None).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_validate_flags_oidc_rejects_code_command() {
        let err = validate_mfa_flags(MfaMethod::Oidc, "office", None, Some("pass otp"), None)
            .unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_validate_flags_mobile_approve_rejects_code() {
        let err = validate_mfa_flags(
            MfaMethod::MobileApprove,
            "office",
            Some("123456"),
            None,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_validate_flags_mobile_approve_rejects_code_command() {
        let err = validate_mfa_flags(
            MfaMethod::MobileApprove,
            "office",
            None,
            Some("pass otp"),
            None,
        )
        .unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_validate_flags_qr_file_only_for_mobile_approve() {
        let err =
            validate_mfa_flags(MfaMethod::Totp, "office", None, None, Some("qr.png")).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("qr-file"));
    }

    #[test]
    fn test_validate_flags_qr_file_ok_for_mobile_approve() {
        validate_mfa_flags(
            MfaMethod::MobileApprove,
            "office",
            None,
            None,
            Some("qr.png"),
        )
        .unwrap();
    }

    #[test]
    fn test_validate_flags_pass_through_totp() {
        validate_mfa_flags(MfaMethod::Totp, "office", Some("123456"), None, None).unwrap();
    }

    #[test]
    fn test_no_code_with_oidc_passes() {
        let l = location("office", LocationMfaMode::External);
        let method = resolve_method(&l, None).unwrap();
        assert_eq!(method, MfaMethod::Oidc);
    }

    #[test]
    fn test_mfa_method_oidc_on_internal_rejected() {
        let l = location("office", LocationMfaMode::Internal);
        let err = resolve_method(&l, Some("oidc")).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("oidc"));
    }
}
