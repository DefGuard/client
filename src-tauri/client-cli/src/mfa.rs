//! Connect-time VPN MFA thin wrapper over `defguard_core::mfa`.
//!
//! Supports TOTP, email, OIDC, and mobile-approve methods.
//!
//! CLI-specific code (method resolution from flags, browser-open, QR
//! rendering, TTY prompting) stays here; all HTTP, WebSocket, and poll
//! logic delegates to `defguard_core::mfa`.

use std::io::{stderr, stdin, Write};

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
    proto::client_types::{
        mfa_step_result, ClientMfaFinishRequest, ClientMfaStartRequest, ClientMfaStepStartRequest,
    },
};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, info, warn};
use url::Url;

use crate::{
    mfa_code::{obtain_code, CodeSource, MfaContext, MfaStepContext},
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

/// CLI-drivable step methods for the code loop below. Mirrors the desktop's
/// `isDesktopDrivable`, minus FIDO2 which has no CLI support.
fn is_cli_code_method(method: LocationMfaMethod) -> bool {
    matches!(method, LocationMfaMethod::Totp | LocationMfaMethod::Email)
}

fn proto_method(method: LocationMfaMethod) -> MfaMethod {
    match method {
        LocationMfaMethod::Totp => MfaMethod::Totp,
        LocationMfaMethod::Email => MfaMethod::Email,
        LocationMfaMethod::Oidc => MfaMethod::Oidc,
        LocationMfaMethod::Biometric => MfaMethod::Biometric,
        LocationMfaMethod::MobileApprove => MfaMethod::MobileApprove,
        // Fido2 has no proto discriminant; unreachable (filtered above).
        LocationMfaMethod::Fido2 => MfaMethod::Totp,
    }
}

fn step_method_label(method: LocationMfaMethod) -> &'static str {
    match method {
        LocationMfaMethod::Totp => "Authenticator app",
        LocationMfaMethod::Email => "Email",
        _ => method.as_str(),
    }
}

/// Resolve one MFA method per verification step, porting the desktop's
/// `resolveMfaStepPlan`: `--mfa-step` one-off, then the saved plan, then the
/// sole usable method, then an interactive pick. Returns the plan plus
/// whether any step was chosen interactively (the caller saves those).
pub(crate) fn resolve_step_plan(
    location: &Location<Id>,
    one_off: &[String],
    interactive: bool,
) -> Result<(Vec<MfaMethod>, bool), CliError> {
    let steps = &location.mfa_steps;
    if one_off.len() > steps.len() {
        return Err(CliError::InvalidInput(format!(
            "Location '{}' has {} verification steps but {} --mfa-step values were given.",
            location.name,
            steps.len(),
            one_off.len()
        )));
    }

    let mut plan = Vec::with_capacity(steps.len());
    let mut interacted = false;
    for (index, step) in steps.iter().enumerate() {
        let mut candidates: Vec<LocationMfaMethod> = step
            .methods
            .iter()
            .filter(|entry| entry.configured && is_cli_code_method(entry.method))
            .map(|entry| entry.method)
            .collect();
        if candidates.is_empty() {
            candidates = step
                .methods
                .iter()
                .filter(|entry| is_cli_code_method(entry.method))
                .map(|entry| entry.method)
                .collect();
        }

        if let Some(raw) = one_off.get(index) {
            let method = parse_method(raw)?;
            let picked = LocationMfaMethod::from(method);
            if !is_cli_code_method(picked) {
                return Err(CliError::InvalidInput(format!(
                    "--mfa-step only supports totp/email; step {} needs a code method.",
                    index + 1
                )));
            }
            if !candidates.contains(&picked) {
                let usable = candidates
                    .iter()
                    .map(|m| m.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(CliError::InvalidInput(format!(
                    "--mfa-step '{raw}' is not available for step {} of '{}' (usable: {usable}).",
                    index + 1,
                    location.name
                )));
            }
            plan.push(method);
            continue;
        }

        match location.mfa_step_plan.get(index) {
            Some(saved) if candidates.contains(saved) => {
                plan.push(proto_method(*saved));
                continue;
            }
            _ => {}
        }

        if candidates.len() == 1 {
            plan.push(proto_method(candidates[0]));
        } else if candidates.is_empty() {
            return Err(CliError::MfaFailed(format!(
                "Step {} of '{}' has no method the CLI can drive (totp/email). \
                 Use the desktop client.",
                index + 1,
                location.name
            )));
        } else if interactive {
            plan.push(prompt_step_method(
                &location.name,
                index,
                steps.len(),
                &candidates,
            )?);
            interacted = true;
        } else {
            let usable = candidates
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(CliError::MfaInputRequired(format!(
                "Step {} of '{}' offers multiple methods ({usable}). \
                 Pass --mfa-step or run on a TTY.",
                index + 1,
                location.name
            )));
        }
    }
    Ok((plan, interacted))
}

/// Ask which method to use for one step.
fn prompt_step_method(
    location_name: &str,
    index: usize,
    step_count: usize,
    candidates: &[LocationMfaMethod],
) -> Result<MfaMethod, CliError> {
    eprintln!(
        "Step {} of {} for '{location_name}': choose MFA method:",
        index + 1,
        step_count
    );
    for (n, method) in candidates.iter().enumerate() {
        eprintln!(
            "  {}) {} ({})",
            n + 1,
            step_method_label(*method),
            method.as_str()
        );
    }
    eprint!("Enter choice [1-{} or name]: ", candidates.len());
    stderr().flush().ok();

    let mut input = String::new();
    stdin()
        .read_line(&mut input)
        .map_err(|e| CliError::MfaFailed(format!("Failed to read choice: {e}")))?;
    let input = input.trim();

    let numbered = input
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=candidates.len()).contains(n));
    if let Some(n) = numbered {
        return Ok(proto_method(candidates[n - 1]));
    }
    match parse_method(input) {
        Ok(method) if is_cli_code_method(LocationMfaMethod::from(method)) => {
            if candidates.contains(&LocationMfaMethod::from(method)) {
                Ok(method)
            } else {
                Err(CliError::InvalidInput(format!(
                    "'{input}' is not available for this step."
                )))
            }
        }
        _ => Err(CliError::InvalidInput(format!(
            "Invalid choice '{input}'. Enter 1-{} or a method name.",
            candidates.len()
        ))),
    }
}

/// Run the VPN MFA handshake for a location (TOTP or email).
///
/// The HTTP calls are handled by `defguard_core::mfa`; this function
/// handles CLI-specific code sourcing (TTY / --code / --code-command).
#[allow(deprecated)]
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
        MfaMethod::Fido2 => {
            return Err(CliError::MfaFailed(
                "FIDO2 MFA is not supported by the CLI. Use the desktop client.".into(),
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
        // empty = legacy path; the CLI has no multi-step MFA
        selected_methods: Vec::new(),
    };
    let info = mfa::mfa_start(proxy_url.clone(), request)
        .await
        .map_err(into_cli)?;

    let ctx = MfaContext {
        instance: instance.name.clone(),
        location: location.name.clone(),
        step: None,
    };
    let code = obtain_code(source, &ctx)?;

    let finish_req = ClientMfaFinishRequest {
        token: info.token,
        code: Some(code.expose_secret().to_string()),
        auth_pub_key: None,
        step_attempt_id: None,
        auth_data: None,
        credential_id: None,
    };
    let psk = mfa::mfa_finish_code(proxy_url, finish_req)
        .await
        .map_err(into_cli)?;

    info!("MFA session completed, preshared key obtained");
    Ok(SecretString::from(psk.preshared_key))
}

/// Run the VPN MFA handshake for a multi-step location (TOTP / email steps).
///
/// Returns the preshared key once the server reports the plan completed.
pub(crate) async fn authorize_multistep(
    location: &Location<Id>,
    code_command: Option<&str>,
    plan: &[MfaMethod],
    instance: &Instance<Id>,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
) -> Result<SecretString, CliError> {
    let Some((first, _)) = plan.split_first() else {
        return Err(CliError::Other("MFA step plan is empty".into()));
    };

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
        "Starting multi-step MFA session for location {} ({} steps)",
        location.name,
        plan.len()
    );
    #[allow(deprecated)]
    let request = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey,
        method: *first as i32,
        posture_data,
        selected_methods: plan.iter().map(|method| *method as i32).collect(),
    };
    let info = mfa::mfa_start(proxy_url.clone(), request)
        .await
        .map_err(into_cli)?;

    let ctx = MfaContext {
        instance: instance.name.clone(),
        location: location.name.clone(),
        step: None,
    };
    let token = info.token;
    for (index, method) in plan.iter().enumerate() {
        let step_attempt_id = if index == 0 {
            None
        } else {
            debug!(
                "Starting MFA step {}/{} ({method:?})",
                index + 1,
                plan.len()
            );
            let step = mfa::mfa_step_start(
                proxy_url.clone(),
                ClientMfaStepStartRequest {
                    token: token.clone(),
                    method: *method as i32,
                },
            )
            .await
            .map_err(into_cli)?;
            Some(step.step_attempt_id)
        };

        let source = match code_command {
            Some(cmd) => CodeSource::Command(cmd.to_string()),
            // Interactive; obtain_code errors clearly without a TTY.
            None => CodeSource::Interactive,
        };
        let step_ctx = MfaContext {
            instance: ctx.instance.clone(),
            location: ctx.location.clone(),
            step: Some(MfaStepContext {
                index,
                total: plan.len(),
                method_label: step_method_label(LocationMfaMethod::from(*method)).to_string(),
            }),
        };
        let code = obtain_code(&source, &step_ctx)?;

        let finish = mfa::mfa_finish_code(
            proxy_url.clone(),
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(code.expose_secret().to_string()),
                auth_pub_key: None,
                step_attempt_id,
                auth_data: None,
                credential_id: None,
            },
        )
        .await
        .map_err(into_cli)?;

        match finish.result.and_then(|result| result.outcome) {
            Some(mfa_step_result::Outcome::Advanced(advanced)) => {
                debug!(
                    "MFA step passed, advancing to step {}",
                    advanced.next_step + 1
                );
            }
            Some(mfa_step_result::Outcome::Completed(completed)) => {
                info!("MFA session completed, preshared key obtained");
                return Ok(SecretString::from(completed.preshared_key));
            }
            Some(mfa_step_result::Outcome::AwaitingExternal(_)) => {
                return Err(CliError::Other(
                    "The server returned an unexpected verification state".into(),
                ));
            }
            // Legacy single-response path.
            None => {
                info!("MFA session completed, preshared key obtained");
                #[allow(deprecated)]
                return Ok(SecretString::from(finish.preshared_key));
            }
        }
    }

    Err(CliError::Other(
        "MFA finished without a preshared key".into(),
    ))
}

/// Run the OIDC MFA flow for an external-IdP location.
///
/// Opens the system browser and delegates the HTTP poll to
/// `defguard_core::mfa::poll_openid_mfa`.
///
/// When `json_mode` is true, progress messages on stderr are suppressed so
/// that `--json` output consumers only see the final result/error.
#[allow(deprecated)]
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
        // empty = legacy path; the CLI has no multi-step MFA
        selected_methods: Vec::new(),
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
#[allow(deprecated)]
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
        // empty = legacy path; the CLI has no multi-step MFA
        selected_methods: Vec::new(),
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
        Some(LocationMfaMethod::Fido2) => MfaMethod::Fido2,
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
            mfa_steps: Default::default(),
            mfa_step_plan: Default::default(),
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

    use defguard_core::database::models::location::{LocationMfaStep, LocationMfaStepMethod};
    use sqlx::types::Json;

    fn step(methods: &[(LocationMfaMethod, bool)]) -> LocationMfaStep {
        LocationMfaStep {
            methods: methods
                .iter()
                .map(|(method, configured)| LocationMfaStepMethod {
                    method: *method,
                    configured: *configured,
                })
                .collect(),
        }
    }

    fn multistep_location(
        steps: Vec<LocationMfaStep>,
        saved: Vec<LocationMfaMethod>,
    ) -> Location<Id> {
        let mut l = location("office", LocationMfaMode::Internal);
        l.mfa_steps = Json(steps);
        l.mfa_step_plan = Json(saved);
        l
    }

    #[test]
    fn test_step_plan_one_off_beats_saved() {
        let l = multistep_location(
            vec![
                step(&[
                    (LocationMfaMethod::Totp, true),
                    (LocationMfaMethod::Email, true),
                ]),
                step(&[(LocationMfaMethod::Email, true)]),
            ],
            vec![LocationMfaMethod::Totp, LocationMfaMethod::Email],
        );
        let (plan, interacted) = resolve_step_plan(&l, &["email".to_string()], false).unwrap();
        assert_eq!(plan, vec![MfaMethod::Email, MfaMethod::Email]);
        assert!(!interacted);
    }

    #[test]
    fn test_step_plan_saved_used_without_one_off() {
        let l = multistep_location(
            vec![
                step(&[
                    (LocationMfaMethod::Totp, true),
                    (LocationMfaMethod::Email, true),
                ]),
                step(&[(LocationMfaMethod::Email, true)]),
            ],
            vec![LocationMfaMethod::Email],
        );
        let (plan, interacted) = resolve_step_plan(&l, &[], false).unwrap();
        assert_eq!(plan, vec![MfaMethod::Email, MfaMethod::Email]);
        assert!(!interacted);
    }

    #[test]
    fn test_step_plan_single_usable_auto_picked() {
        let l = multistep_location(vec![step(&[(LocationMfaMethod::Totp, true)])], vec![]);
        let (plan, _) = resolve_step_plan(&l, &[], false).unwrap();
        assert_eq!(plan, vec![MfaMethod::Totp]);
    }

    #[test]
    fn test_step_plan_ambiguous_headless_errors() {
        let l = multistep_location(
            vec![step(&[
                (LocationMfaMethod::Totp, true),
                (LocationMfaMethod::Email, true),
            ])],
            vec![],
        );
        let err = resolve_step_plan(&l, &[], false).unwrap_err();
        assert!(matches!(err, CliError::MfaInputRequired(_)));
        assert!(err.to_string().contains("--mfa-step"));
    }

    #[test]
    fn test_step_plan_too_many_one_offs_rejected() {
        let l = multistep_location(vec![step(&[(LocationMfaMethod::Totp, true)])], vec![]);
        let err =
            resolve_step_plan(&l, &["totp".to_string(), "email".to_string()], false).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
    }

    #[test]
    fn test_step_plan_unavailable_one_off_rejected() {
        let l = multistep_location(vec![step(&[(LocationMfaMethod::Totp, true)])], vec![]);
        let err = resolve_step_plan(&l, &["email".to_string()], false).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("step 1"));
    }

    #[test]
    fn test_step_plan_no_cli_method_uses_desktop() {
        let l = multistep_location(vec![step(&[(LocationMfaMethod::Biometric, true)])], vec![]);
        let err = resolve_step_plan(&l, &[], false).unwrap_err();
        assert!(matches!(err, CliError::MfaFailed(_)));
        assert!(err.to_string().contains("desktop"));
    }
}
