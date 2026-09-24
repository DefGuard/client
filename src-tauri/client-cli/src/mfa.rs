//! Connect-time VPN MFA thin wrapper over `defguard_core::mfa`.
//!
//! Supports TOTP, email, OIDC, and mobile-approve methods for single- and
//! multi-step verification. The CLI reports FIDO2 and biometric steps, which
//! it cannot run, and stops.
//!
//! CLI-specific code (method resolution from flags, browser-open, QR
//! rendering, TTY prompting) stays here; all HTTP, WebSocket, and poll
//! logic delegates to `defguard_core::mfa`.

use std::{
    future::Future,
    io::{stderr, stdin, Write},
};

use clap::builder::{PossibleValue, PossibleValuesParser};
use defguard_client_proto::defguard::{
    client_types::MfaMethod, enterprise::posture::v2::DevicePostureData,
};
use defguard_core::{
    database::{
        models::{
            instance::Instance,
            location::{
                infer_mfa_method, Location, LocationMfaMethod, LocationMfaMode, LocationMfaStep,
            },
            wireguard_keys::WireguardKeys,
            Id,
        },
        DbPool,
    },
    mfa,
    proto::client_types::{
        mfa_step_result, ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
        ClientMfaStartResponse, ClientMfaStepStartRequest, ClientMfaStepStartResponse,
    },
};
use secrecy::{ExposeSecret, SecretString};
use tokio_util::sync::CancellationToken;
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
        | mfa::MfaError::AttemptLimit { .. }
        | mfa::MfaError::Timeout => CliError::MfaFailed(msg),
        mfa::MfaError::Cancelled => CliError::Cancelled(msg),
    }
}

/// Resolve the effective MFA method for a single-step location.
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
    if let Some(step) = location.mfa_steps.first() {
        if step_candidates(step).is_empty() {
            return Err(CliError::MfaFailed(format!(
                "Location '{}' has no MFA method the CLI can run. \
                 Use the desktop or mobile client.",
                location.name
            )));
        }
    }

    let method = if let Some(raw) = method_override {
        let method = parse_method(raw)?;
        if !is_cli_drivable_method(method) {
            return Err(CliError::InvalidInput(format!(
                "--mfa-method {raw} is not supported by the CLI. \
                 Use the desktop or mobile client."
            )));
        }
        // Edge rejects OIDC for Internal locations, so fail before starting MFA.
        if method == LocationMfaMethod::Oidc
            && location.location_mfa_mode == LocationMfaMode::Internal
        {
            return Err(CliError::InvalidInput(
                "--mfa-method oidc is only valid for locations that use external (OIDC) MFA."
                    .into(),
            ));
        }
        method.into()
    } else {
        infer_method(location)
    };

    Ok(method)
}

/// Validate flags against one method per verification step.
///
/// `--code` accepts one code, so multi-step locations need one code per step.
/// Code input requires TOTP or email; `--qr-file` requires mobile approval.
pub(crate) fn validate_mfa_flags(
    plan: &[MfaMethod],
    location_name: &str,
    code: Option<&str>,
    code_command: Option<&str>,
    qr_file: Option<&str>,
) -> Result<(), CliError> {
    if code.is_some() && plan.len() > 1 {
        return Err(CliError::InvalidInput(
            "--code accepts one code; multi-step locations need one code per step. \
             Use --code-command or a TTY."
                .into(),
        ));
    }

    let takes_code = plan
        .iter()
        .any(|method| matches!(method, MfaMethod::Totp | MfaMethod::Email));
    if !takes_code && (code.is_some() || code_command.is_some()) {
        return Err(CliError::InvalidInput(format!(
            "Location '{location_name}' cannot use --code or --code-command \
             because no verification step accepts a code",
        )));
    }

    if !plan.contains(&MfaMethod::MobileApprove) && qr_file.is_some() {
        return Err(CliError::InvalidInput(
            "--qr-file is only valid with mobile-approve MFA".into(),
        ));
    }

    Ok(())
}

/// Methods the CLI can run. Keep this list aligned with the desktop's
/// `isDesktopDrivable`; FIDO2 and biometric use flows the CLI cannot run.
fn is_cli_drivable_method(method: LocationMfaMethod) -> bool {
    matches!(
        method,
        LocationMfaMethod::Totp
            | LocationMfaMethod::Email
            | LocationMfaMethod::Oidc
            | LocationMfaMethod::MobileApprove
    )
}

/// Format a step prefix such as `[2/4] `.
///
/// Pad the step number so all prefixes in a run have the same width.
fn step_badge(index: usize, total: usize) -> String {
    format!("[{:>w$}/{total}] ", index + 1, w = decimal_digits(total))
}

/// Return the count of decimal digits in `n`.
fn decimal_digits(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        n.ilog10() as usize + 1
    }
}

/// Return the step prefix, or an empty string when no step context exists.
pub(crate) fn opt_step_badge(step: Option<&MfaStepContext>) -> String {
    step.map(|step| step_badge(step.index, step.total))
        .unwrap_or_default()
}

/// Return the indentation for text below a step prefix.
fn step_indent(badge: &str) -> String {
    " ".repeat(badge.len().max(2))
}

pub(crate) fn step_method_label(method: LocationMfaMethod) -> &'static str {
    match method {
        LocationMfaMethod::Totp => "Authenticator app",
        LocationMfaMethod::Email => "Email",
        LocationMfaMethod::Oidc => "OpenID",
        LocationMfaMethod::MobileApprove => "Mobile Client",
        _ => method.as_str(),
    }
}

/// Return methods the CLI can run for one step.
///
/// Prefer configured methods. If none are configured, return every supported
/// method because Edge decides which methods are available.
fn step_candidates(step: &LocationMfaStep) -> Vec<LocationMfaMethod> {
    let configured: Vec<LocationMfaMethod> = step
        .methods
        .iter()
        .filter(|entry| entry.configured && is_cli_drivable_method(entry.method))
        .map(|entry| entry.method)
        .collect();
    if configured.is_empty() {
        return step
            .methods
            .iter()
            .filter(|entry| is_cli_drivable_method(entry.method))
            .map(|entry| entry.method)
            .collect();
    }
    configured
}

pub(crate) fn join_methods(methods: &[LocationMfaMethod]) -> String {
    methods
        .iter()
        .map(LocationMfaMethod::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse a selected method and verify that the CLI can run it for this step.
fn pick_step_method(
    raw: &str,
    candidates: &[LocationMfaMethod],
    index: usize,
    location_name: &str,
) -> Result<LocationMfaMethod, CliError> {
    let picked = parse_method(raw)?;
    if !is_cli_drivable_method(picked) {
        return Err(CliError::InvalidInput(format!(
            "MFA method {raw} is not supported by the CLI for step {}. \
             Use the desktop or mobile client.",
            index + 1
        )));
    }
    if !candidates.contains(&picked) {
        return Err(CliError::InvalidInput(format!(
            "'{raw}' is not available for step {} of '{location_name}' (usable: {}).",
            index + 1,
            join_methods(candidates)
        )));
    }
    Ok(picked)
}

/// Resolve one method for each verification step.
///
/// Precedence is one-off flags, the saved plan, the only usable method, and
/// then an interactive choice. Return whether prompting occurred so the caller
/// can show a command to save the plan.
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
        let candidates = step_candidates(step);
        if candidates.is_empty() {
            return Err(CliError::MfaFailed(format!(
                "Step {} of '{}' has no MFA method the CLI can run. \
                 Use the desktop client.",
                index + 1,
                location.name
            )));
        }

        let saved = location
            .mfa_step_plan
            .get(index)
            .filter(|method| candidates.contains(method));

        let method = if let Some(raw) = one_off.get(index) {
            pick_step_method(raw, &candidates, index, &location.name)?
        } else if let Some(saved) = saved {
            *saved
        } else if candidates.len() == 1 {
            candidates[0]
        } else if interactive {
            interacted = true;
            prompt_step_method(&location.name, index, steps.len(), &candidates)?
        } else {
            return Err(CliError::MfaInputRequired(format!(
                "Step {} of '{}' has multiple methods ({}). \
                 Pass --mfa-step or use a TTY.",
                index + 1,
                location.name,
                join_methods(&candidates)
            )));
        };
        plan.push(method.into());
    }
    Ok((plan, interacted))
}

/// Ask which method to use for one step.
fn prompt_step_method(
    location_name: &str,
    index: usize,
    step_count: usize,
    candidates: &[LocationMfaMethod],
) -> Result<LocationMfaMethod, CliError> {
    let badge = step_badge(index, step_count);
    let indent = step_indent(&badge);
    eprintln!("{badge}Choose an MFA method for '{location_name}':");
    for (n, method) in candidates.iter().enumerate() {
        eprintln!(
            "{indent}  {}) {} ({})",
            n + 1,
            step_method_label(*method),
            method.as_str()
        );
    }
    eprint!("{indent}Enter choice [1-{} or name]: ", candidates.len());
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
    match numbered {
        Some(n) => Ok(candidates[n - 1]),
        None => pick_step_method(input, candidates, index, location_name),
    }
}

/// Start an MFA session and return its proxy URL and response.
///
/// `selected_methods` carries one method per verification step. An empty list makes
/// Core use the legacy path, which rejects flows it cannot express through the legacy
/// field and tells the user to update the client. Keep `method` populated for pre-2.2 Edge, which ignores the per-step plan.
async fn start_session(
    location: &Location<Id>,
    instance: &Instance<Id>,
    method: MfaMethod,
    selected_methods: Vec<i32>,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
) -> Result<(Url, ClientMfaStartResponse, bool), CliError> {
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
    #[allow(deprecated)]
    let request = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: wireguard_keys.pubkey,
        method: method as i32,
        posture_data,
        selected_methods,
    };
    let start = mfa::mfa_start_with_capability(proxy_url.clone(), request)
        .await
        .map_err(into_cli)?;

    Ok((proxy_url, start.response, start.multi_step_mfa_capable))
}

/// Reject methods that `authorize` cannot run.
fn check_code_method(method: MfaMethod) -> Result<(), CliError> {
    if !is_cli_drivable_method(method.into()) {
        return Err(CliError::MfaFailed(format!(
            "MFA method {method:?} is not supported by the CLI. Use the desktop or mobile client."
        )));
    }
    if !matches!(method, MfaMethod::Totp | MfaMethod::Email) {
        return Err(CliError::Other(format!(
            "Internal error: {method:?} MFA must use its dedicated flow, not authorize"
        )));
    }
    Ok(())
}

/// Run the VPN MFA handshake for a single-step location (TOTP or email).
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
    check_code_method(method)?;

    let (proxy_url, info, _) = start_session(
        location,
        instance,
        method,
        vec![method as i32],
        posture_data,
        pool,
    )
    .await?;

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

    finish_psk(psk)?.ok_or_else(|| {
        CliError::Other("The server returned an unexpected verification state".into())
    })
}

/// Run a multi-step MFA handshake.
///
/// Start one session with `plan`, complete each step, and return the key after
/// the server reports completion. Code steps use `--code-command` or a TTY;
/// OIDC uses the browser, and mobile approval uses a QR code.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn authorize_multistep(
    location: &Location<Id>,
    code_command: Option<&str>,
    plan: &[MfaMethod],
    instance: &Instance<Id>,
    posture_data: Option<DevicePostureData>,
    pool: &DbPool,
    qr_file: Option<&str>,
    json_mode: bool,
) -> Result<SecretString, CliError> {
    let Some((first, _)) = plan.split_first() else {
        return Err(CliError::Other("MFA step plan is empty".into()));
    };

    debug!(
        "Starting multi-step MFA session for location {} ({} steps)",
        location.name,
        plan.len()
    );
    let (proxy_url, info, capable) = start_session(
        location,
        instance,
        *first,
        plan.iter().map(|method| *method as i32).collect(),
        posture_data,
        pool,
    )
    .await?;
    if !capable {
        debug!("Edge or Core does not support multi-step MFA; using the legacy flow");
    }

    let token = info.token;
    for (index, method) in plan.iter().enumerate() {
        debug!("Running MFA step {}/{} ({method:?})", index + 1, plan.len());

        let step = if capable {
            Some(step_start(&proxy_url, &token, *method).await?)
        } else {
            None
        };

        let step_ctx = MfaStepContext {
            index,
            total: plan.len(),
            method: LocationMfaMethod::from(*method),
        };

        let finish = match method {
            MfaMethod::Totp | MfaMethod::Email => {
                let source = match code_command {
                    Some(cmd) => CodeSource::Command(cmd.to_string()),
                    // Let obtain_code handle the TTY check and error.
                    None => CodeSource::Interactive,
                };
                let ctx = MfaContext {
                    instance: instance.name.clone(),
                    location: location.name.clone(),
                    step: Some(step_ctx),
                };
                let code = obtain_code(&source, &ctx)?;

                mfa::mfa_finish_code(
                    proxy_url.clone(),
                    ClientMfaFinishRequest {
                        token: token.clone(),
                        code: Some(code.expose_secret().to_string()),
                        auth_pub_key: None,
                        step_attempt_id: step.map(|step| step.step_attempt_id),
                        auth_data: None,
                        credential_id: None,
                    },
                )
                .await
                .map_err(into_cli)?
            }
            MfaMethod::Oidc => {
                run_oidc_step(
                    &proxy_url,
                    &token,
                    step.map(|step| step.step_attempt_id),
                    Some(&step_ctx),
                    json_mode,
                )
                .await?
            }
            MfaMethod::MobileApprove => {
                let challenge = match &step {
                    Some(step) => step.challenge.clone(),
                    None => info.challenge.clone(),
                }
                .ok_or_else(|| {
                    CliError::Other("Edge did not return a challenge for mobile-approve MFA".into())
                })?;
                run_mobile_step(
                    &proxy_url,
                    &token,
                    &challenge,
                    &instance.uuid,
                    qr_file,
                    Some(&step_ctx),
                    json_mode,
                )
                .await?
            }
            _ => {
                return Err(CliError::MfaFailed(format!(
                    "MFA method {method:?} is not supported by the CLI for step {}. \
                     Use the desktop or mobile client.",
                    index + 1
                )));
            }
        };

        if let Some(psk) = finish_psk(finish)? {
            return Ok(psk);
        }
    }

    Err(CliError::Other(
        "MFA completed without a preshared key".into(),
    ))
}

/// Open a step on an existing session.
async fn step_start(
    proxy_url: &Url,
    token: &str,
    method: MfaMethod,
) -> Result<ClientMfaStepStartResponse, CliError> {
    mfa::mfa_step_start(
        proxy_url.clone(),
        ClientMfaStepStartRequest {
            token: token.to_string(),
            method: method as i32,
        },
    )
    .await
    .map_err(into_cli)
}

/// Extract a preshared key from a finish response.
///
/// Return `None` when the server advanced to another step.
fn finish_psk(finish: ClientMfaFinishResponse) -> Result<Option<SecretString>, CliError> {
    if let Some(mfa_step_result::Outcome::AwaitingExternal(_)) = finish
        .result
        .as_ref()
        .and_then(|result| result.outcome.as_ref())
    {
        return Err(CliError::Other(
            "The server returned an unexpected verification state".into(),
        ));
    }
    if let Some(psk) = mfa::completed_preshared_key(&finish) {
        info!("MFA session completed, preshared key obtained");
        Ok(Some(SecretString::from(psk)))
    } else {
        debug!("MFA step passed, the session advanced to the next step");
        Ok(None)
    }
}

/// Run an MFA operation with Ctrl-C cancellation.
async fn with_ctrl_c<T, F>(run: impl FnOnce(CancellationToken) -> F) -> T
where
    F: Future<Output = T>,
{
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let ctrlc_handle = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
    });

    let result = run(cancel).await;
    ctrlc_handle.abort();
    result
}

/// Build the browser URL for an OIDC step.
fn oidc_browser_url(
    proxy_url: &Url,
    token: &str,
    step_attempt_id: Option<&str>,
) -> Result<Url, CliError> {
    let mut url = proxy_url
        .join("openid/mfa")
        .map_err(|e| CliError::Other(format!("Failed to build OIDC MFA URL: {e}")))?;
    let mut query = url.query_pairs_mut();
    query.append_pair("token", token);
    if let Some(id) = step_attempt_id {
        query.append_pair("step_attempt_id", id);
    }
    drop(query);
    Ok(url)
}

/// Open the OIDC page and poll for the result.
async fn run_oidc_step(
    proxy_url: &Url,
    token: &str,
    step_attempt_id: Option<String>,
    step: Option<&MfaStepContext>,
    json_mode: bool,
) -> Result<ClientMfaFinishResponse, CliError> {
    let browser_url = oidc_browser_url(proxy_url, token, step_attempt_id.as_deref())?;

    let badge = opt_step_badge(step);
    if !json_mode {
        eprintln!("{badge}Open this URL to authenticate:");
        eprintln!("{}{browser_url}", step_indent(&badge));
        eprintln!("{badge}Waiting for browser authentication... (Ctrl-C to cancel)");
    }
    open_url(browser_url.as_ref(), &badge, json_mode);

    with_ctrl_c(|cancel| {
        mfa::poll_openid_mfa(
            proxy_url.clone(),
            token.to_string(),
            step_attempt_id,
            cancel,
        )
    })
    .await
    .map_err(into_cli)
}

/// Show a mobile-approval QR code and wait for the WebSocket result.
async fn run_mobile_step(
    proxy_url: &Url,
    token: &str,
    challenge: &str,
    instance_uuid: &str,
    qr_file: Option<&str>,
    step: Option<&MfaStepContext>,
    json_mode: bool,
) -> Result<ClientMfaFinishResponse, CliError> {
    let badge = opt_step_badge(step);
    let payload = mfa_qr::build_qr_payload(token, challenge, instance_uuid);
    mfa_qr::render_qr(&payload, qr_file, &badge, json_mode)?;
    if !json_mode {
        eprintln!("{badge}Waiting for mobile approval... (Ctrl-C to cancel)");
    }

    let ws_url = mfa::derive_ws_url(proxy_url, token).map_err(into_cli)?;

    with_ctrl_c(|cancel| mfa::connect_mobile_approve(&ws_url, cancel))
        .await
        .map_err(into_cli)
}

/// Run the OIDC MFA flow for a single-step external-IdP location.
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
    let (proxy_url, info, _) = start_session(
        location,
        instance,
        MfaMethod::Oidc,
        vec![MfaMethod::Oidc as i32],
        posture_data,
        pool,
    )
    .await?;

    let finish = run_oidc_step(&proxy_url, &info.token, None, None, json_mode).await?;
    finish_psk(finish)?.ok_or_else(|| {
        CliError::Other("The server returned an unexpected verification state".into())
    })
}

/// Run the mobile-approve MFA flow for a single-step location.
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
    let (proxy_url, info, _) = start_session(
        location,
        instance,
        MfaMethod::MobileApprove,
        vec![MfaMethod::MobileApprove as i32],
        posture_data,
        pool,
    )
    .await?;

    let challenge = info.challenge.ok_or_else(|| {
        CliError::Other("Edge did not return a challenge for mobile-approve MFA".into())
    })?;

    let finish = run_mobile_step(
        &proxy_url,
        &info.token,
        &challenge,
        &instance.uuid,
        qr_file,
        None,
        json_mode,
    )
    .await?;
    finish_psk(finish)?.ok_or_else(|| {
        CliError::Other("The server returned an unexpected verification state".into())
    })
}

/// MFA methods that a CLI flag can name, in `--help` order.
const MFA_METHODS: [LocationMfaMethod; 6] = [
    LocationMfaMethod::Totp,
    LocationMfaMethod::Email,
    LocationMfaMethod::Oidc,
    LocationMfaMethod::Biometric,
    LocationMfaMethod::MobileApprove,
    LocationMfaMethod::Fido2,
];

fn method_value(method: LocationMfaMethod) -> PossibleValue {
    let value = PossibleValue::new(method.as_str());
    if method == LocationMfaMethod::MobileApprove {
        value.alias("mobile_approve")
    } else {
        value
    }
}

/// Value parser that lists the MFA methods in `--help`.
///
/// With `cli_drivable_only`, `--help` omits the methods that the CLI cannot run.
/// The parser still accepts them, so the command can tell the user which client to use.
pub(crate) fn method_parser(cli_drivable_only: bool) -> PossibleValuesParser {
    PossibleValuesParser::new(MFA_METHODS.map(|method| {
        method_value(method).hide(cli_drivable_only && !is_cli_drivable_method(method))
    }))
}

/// Parse an MFA method name from a CLI flag.
pub(crate) fn parse_method(raw: &str) -> Result<LocationMfaMethod, CliError> {
    MFA_METHODS
        .into_iter()
        .find(|method| method_value(*method).matches(raw, true))
        .ok_or_else(|| {
            CliError::Usage(format!(
                "Invalid MFA method '{raw}'. Valid: {}.",
                join_methods(&MFA_METHODS)
            ))
        })
}

/// Determine the MFA method to use for a single-step location.
///
/// Delegates to the core's [`infer_mfa_method`] so that [`LocationMfaMode`]
/// is respected - an External-mode location always uses OIDC, while an
/// Internal-mode location respects the stored preference (defaulting to TOTP).
fn infer_method(location: &Location<Id>) -> MfaMethod {
    // infer_mfa_method returns None only for Disabled mode, and this runs only
    // when MFA is enabled. TOTP is the safe fallback.
    infer_mfa_method(location.location_mfa_mode, location.mfa_method)
        .map_or(MfaMethod::Totp, Into::into)
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
fn open_url(url: &str, badge: &str, json_mode: bool) {
    if webbrowser::open(url).is_err() {
        if json_mode {
            eprintln!("Could not open browser. Open this URL manually: {url}");
        } else {
            eprintln!("{badge}Could not open browser. Open the above URL manually.");
        }
    }
}

#[cfg(test)]
fn open_url(_url: &str, _badge: &str, _json_mode: bool) {
    // no-op: tests must not spawn a browser
}

#[cfg(test)]
mod tests {
    use defguard_core::database::models::location::ServiceLocationMode;
    use sqlx::types::Json;

    use super::*;

    #[test]
    fn decimal_digits_matches_string_length() {
        for n in [0, 1, 9, 10, 99, 100, 999, 1000, usize::MAX] {
            assert_eq!(decimal_digits(n), n.to_string().len());
        }
    }

    fn location(name: &str, mode: LocationMfaMode) -> Location<Id> {
        Location {
            mfa_steps: Json::default(),
            mfa_step_plan: Json::default(),
            client_mtu: None,
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
    fn test_oidc_browser_url_carries_step_attempt_id() {
        let proxy = Url::parse("https://proxy.example.com/").unwrap();
        let url = oidc_browser_url(&proxy, "tok", Some("attempt-1")).unwrap();
        assert_eq!(
            url.as_str(),
            "https://proxy.example.com/openid/mfa?token=tok&step_attempt_id=attempt-1"
        );
    }

    #[test]
    fn test_oidc_browser_url_legacy_omits_step_attempt_id() {
        let proxy = Url::parse("https://proxy.example.com/").unwrap();
        let url = oidc_browser_url(&proxy, "tok", None).unwrap();
        assert_eq!(
            url.as_str(),
            "https://proxy.example.com/openid/mfa?token=tok"
        );
    }

    #[test]
    fn test_check_code_method_rejects_undrivable_as_mfa_failed() {
        for method in [MfaMethod::Biometric, MfaMethod::Fido2] {
            let err = check_code_method(method).unwrap_err();
            assert!(matches!(err, CliError::MfaFailed(_)), "{method:?}");
        }
    }

    #[test]
    fn test_check_code_method_rejects_own_flow_methods_as_internal() {
        for method in [MfaMethod::Oidc, MfaMethod::MobileApprove] {
            let err = check_code_method(method).unwrap_err();
            assert!(matches!(err, CliError::Other(_)), "{method:?}");
        }
        assert!(check_code_method(MfaMethod::Totp).is_ok());
        assert!(check_code_method(MfaMethod::Email).is_ok());
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
        let err = validate_mfa_flags(&[MfaMethod::Oidc], "office", Some("123456"), None, None)
            .unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_validate_flags_oidc_rejects_code_command() {
        let err = validate_mfa_flags(&[MfaMethod::Oidc], "office", None, Some("pass otp"), None)
            .unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_validate_flags_mobile_approve_rejects_code() {
        let err = validate_mfa_flags(
            &[MfaMethod::MobileApprove],
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
            &[MfaMethod::MobileApprove],
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
        let err = validate_mfa_flags(&[MfaMethod::Totp], "office", None, None, Some("qr.png"))
            .unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("qr-file"));
    }

    #[test]
    fn test_validate_flags_qr_file_ok_for_mobile_approve() {
        validate_mfa_flags(
            &[MfaMethod::MobileApprove],
            "office",
            None,
            None,
            Some("qr.png"),
        )
        .unwrap();
    }

    #[test]
    fn test_validate_flags_pass_through_totp() {
        validate_mfa_flags(&[MfaMethod::Totp], "office", Some("123456"), None, None).unwrap();
    }

    #[test]
    fn test_validate_flags_qr_file_ok_for_mobile_step_of_plan() {
        validate_mfa_flags(
            &[MfaMethod::Totp, MfaMethod::MobileApprove],
            "office",
            None,
            Some("pass otp"),
            Some("qr.png"),
        )
        .unwrap();
    }

    #[test]
    fn test_validate_flags_qr_file_rejected_without_mobile_step() {
        let err = validate_mfa_flags(
            &[MfaMethod::Totp, MfaMethod::Oidc],
            "office",
            None,
            None,
            Some("qr.png"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("qr-file"));
    }

    #[test]
    fn test_validate_flags_code_command_rejected_without_code_step() {
        let err = validate_mfa_flags(
            &[MfaMethod::Oidc, MfaMethod::MobileApprove],
            "office",
            None,
            Some("pass otp"),
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("--code"));
    }

    #[test]
    fn test_validate_flags_code_rejected_for_multistep() {
        let err = validate_mfa_flags(
            &[MfaMethod::Totp, MfaMethod::Email],
            "office",
            Some("123456"),
            None,
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("one code per step"));
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
    fn test_step_badge_pads_to_a_fixed_width() {
        assert_eq!(step_badge(0, 4), "[1/4] ");
        assert_eq!(step_badge(0, 12), "[ 1/12] ");
        assert_eq!(step_badge(9, 12), "[10/12] ");
        assert_eq!(step_badge(0, 12).len(), step_badge(9, 12).len());
    }

    #[test]
    fn test_step_indent_aligns_under_the_badge() {
        assert_eq!(step_indent(""), "  ");
        assert_eq!(step_indent(&step_badge(0, 4)), " ".repeat(6));
    }

    #[test]
    fn test_opt_step_badge_is_empty_for_a_single_step() {
        assert_eq!(opt_step_badge(None), "");
        let step = MfaStepContext {
            index: 1,
            total: 4,
            method: LocationMfaMethod::Email,
        };
        assert_eq!(opt_step_badge(Some(&step)), "[2/4] ");
    }

    #[test]
    fn test_single_step_without_a_drivable_method_is_rejected() {
        let l = multistep_location(vec![step(&[(LocationMfaMethod::Fido2, true)])], vec![]);
        let err = resolve_method(&l, None).unwrap_err();
        assert!(matches!(err, CliError::MfaFailed(_)));
        assert!(err.to_string().contains("desktop"));
    }

    #[test]
    fn test_mfa_method_fido2_override_is_rejected() {
        let l = location("office", LocationMfaMode::Internal);
        let err = resolve_method(&l, Some("fido2")).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("fido2"));
    }

    #[test]
    fn test_parse_method_accepts_alias_in_any_case() {
        assert_eq!(parse_method("TOTP").unwrap(), LocationMfaMethod::Totp);
        assert_eq!(
            parse_method("Mobile_Approve").unwrap(),
            LocationMfaMethod::MobileApprove
        );
        let err = parse_method("sms").unwrap_err();
        assert!(matches!(err, CliError::Usage(_)));
        assert!(err
            .to_string()
            .contains("Valid: totp, email, oidc, biometric, mobile, fido2."));
    }

    #[test]
    fn test_step_plan_one_off_fido2_is_rejected() {
        let l = multistep_location(
            vec![step(&[
                (LocationMfaMethod::Totp, true),
                (LocationMfaMethod::Fido2, true),
            ])],
            vec![],
        );
        let err = resolve_step_plan(&l, &["fido2".to_string()], false).unwrap_err();
        assert!(matches!(err, CliError::InvalidInput(_)));
        assert!(err.to_string().contains("not supported by the CLI"));
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

    #[test]
    fn test_step_plan_oidc_and_mobile_are_candidates() {
        let l = multistep_location(
            vec![step(&[
                (LocationMfaMethod::Totp, true),
                (LocationMfaMethod::Email, true),
                (LocationMfaMethod::Oidc, true),
                (LocationMfaMethod::MobileApprove, true),
            ])],
            vec![LocationMfaMethod::Oidc],
        );
        let (plan, _) = resolve_step_plan(&l, &[], false).unwrap();
        assert_eq!(plan, vec![MfaMethod::Oidc]);
    }

    #[test]
    fn test_step_plan_one_off_oidc_accepted() {
        let l = multistep_location(
            vec![step(&[
                (LocationMfaMethod::Totp, true),
                (LocationMfaMethod::Oidc, true),
            ])],
            vec![],
        );
        let (plan, _) = resolve_step_plan(&l, &["oidc".to_string()], false).unwrap();
        assert_eq!(plan, vec![MfaMethod::Oidc]);
    }

    #[test]
    fn test_step_plan_headless_lists_all_usable() {
        let l = multistep_location(
            vec![step(&[
                (LocationMfaMethod::Totp, true),
                (LocationMfaMethod::Email, true),
                (LocationMfaMethod::Oidc, true),
                (LocationMfaMethod::MobileApprove, true),
            ])],
            vec![],
        );
        let err = resolve_step_plan(&l, &[], false).unwrap_err();
        assert!(matches!(err, CliError::MfaInputRequired(_)));
        let msg = err.to_string();
        assert!(msg.contains("totp"));
        assert!(msg.contains("oidc"));
        assert!(msg.contains("mobile"));
    }
}
