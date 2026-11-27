//! The shared secret never leaves the backend, the frontend asks for the current code instead.

use defguard_client_device_verification::{
    await_decision, start, Error as VerificationError, Outcome, VerificationCode,
    VerificationSession,
};
use reqwest::Url;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use crate::{
    appstate::AppState,
    commands::{disconnect_locations, err_to_json},
    database::{
        models::{
            instance::{DeviceStatus, Instance},
            location::Location,
            Id,
        },
        DB_POOL,
    },
    events::{DeviceVerificationErrorPayload, DeviceVerificationResolvedPayload, EventKey},
    periodic::config::sync_service_locations_best_effort,
    tray::reload_tray_menu,
};

pub struct ActiveVerification {
    session: VerificationSession,
    cancel: CancellationToken,
}

/// Mirrors DeviceVerificationInfo in rust-api/types.ts.
#[derive(Serialize)]
pub struct DeviceVerificationInfo {
    pub session_id: i64,
    pub digits: u32,
    /// Seconds.
    pub period: u64,
    /// Unix timestamp in seconds.
    pub expires_at: i64,
    pub code: VerificationCode,
}

impl From<&VerificationSession> for DeviceVerificationInfo {
    fn from(session: &VerificationSession) -> Self {
        Self {
            session_id: session.session_id,
            digits: session.digits,
            period: session.period,
            expires_at: session.expires_at,
            code: session.current_code(),
        }
    }
}

fn other(message: impl Into<String>) -> String {
    err_to_json(VerificationError::Other {
        message: message.into(),
    })
}

/// Starts verifying this device, or joins the session the administrator already started, and
/// waits for the decision in the background.
#[tauri::command(async)]
pub async fn start_device_verification(
    instance_id: Id,
    handle: AppHandle,
) -> Result<DeviceVerificationInfo, String> {
    debug!("Received a command to start device verification for instance {instance_id}");
    let instance = Instance::find_by_id(&*DB_POOL, instance_id)
        .await
        .map_err(|err| other(err.to_string()))?
        .ok_or_else(|| other("Instance not found"))?;
    let token = instance
        .token
        .clone()
        .ok_or_else(|| other("Instance has no config polling token"))?;
    let proxy_url =
        Url::parse(&instance.proxy_url).map_err(|err| other(format!("Invalid Edge URL: {err}")))?;

    let session = match start(&proxy_url, &token).await {
        Ok(session) => session,
        Err(err) => {
            if err == VerificationError::DeviceBlocked {
                set_device_status(&handle, instance_id, DeviceStatus::Blocked).await;
            }
            return Err(err_to_json(err));
        }
    };
    lift_device_block(&handle, instance_id).await;
    let info = DeviceVerificationInfo::from(&session);

    let session_id = session.session_id;
    let cancel = CancellationToken::new();
    let is_new = {
        let state = handle.state::<AppState>();
        let mut verifications = state
            .device_verifications
            .lock()
            .expect("device_verifications mutex poisoned");
        if verifications
            .get(&instance_id)
            .is_some_and(|active| active.session.session_id == session_id)
        {
            false
        } else {
            let previous = verifications.insert(
                instance_id,
                ActiveVerification {
                    session,
                    cancel: cancel.clone(),
                },
            );
            if let Some(previous) = previous {
                previous.cancel.cancel();
            }
            true
        }
    };
    if is_new {
        info!("Waiting for the outcome of device verification {session_id} of {instance}");
        tokio::spawn(wait_for_decision(
            handle,
            instance_id,
            proxy_url,
            token,
            session_id,
            cancel,
        ));
    } else {
        debug!("Already waiting for the outcome of device verification {session_id}");
    }

    Ok(info)
}

/// Codes change every period, so the frontend asks again once expires_in runs out.
#[tauri::command]
pub fn get_device_verification_code(
    instance_id: Id,
    handle: AppHandle,
) -> Result<VerificationCode, String> {
    let state = handle.state::<AppState>();
    let verifications = state
        .device_verifications
        .lock()
        .expect("device_verifications mutex poisoned");
    verifications
        .get(&instance_id)
        .map(|active| active.session.current_code())
        .ok_or_else(|| err_to_json(VerificationError::SessionNotFound))
}

/// Stops waiting for the decision. The session itself lives on in Core until it expires or the
/// administrator decides, starting again joins it.
#[tauri::command]
pub fn cancel_device_verification(instance_id: Id, handle: AppHandle) {
    let removed = handle
        .state::<AppState>()
        .device_verifications
        .lock()
        .expect("device_verifications mutex poisoned")
        .remove(&instance_id);
    if let Some(active) = removed {
        debug!(
            "Stopped waiting for device verification {} of instance {instance_id}",
            active.session.session_id
        );
        active.cancel.cancel();
    }
}

async fn wait_for_decision(
    handle: AppHandle,
    instance_id: Id,
    proxy_url: Url,
    token: String,
    session_id: i64,
    cancel: CancellationToken,
) {
    let result = await_decision(&proxy_url, &token, session_id, cancel).await;
    {
        let state = handle.state::<AppState>();
        let mut verifications = state
            .device_verifications
            .lock()
            .expect("device_verifications mutex poisoned");
        // A newer session may have taken the place of this one.
        if verifications
            .get(&instance_id)
            .is_some_and(|active| active.session.session_id == session_id)
        {
            verifications.remove(&instance_id);
        }
    }

    match result {
        Ok(status) => {
            info!(
                "Device verification {session_id} of instance {instance_id} ended with outcome \
                {:?}",
                status.outcome
            );
            match status.outcome {
                Outcome::Blocked => {
                    set_device_status(&handle, instance_id, DeviceStatus::Blocked).await;
                }
                Outcome::Verified => {
                    set_device_status(&handle, instance_id, DeviceStatus::Verified).await;
                }
                _ => {}
            }
            DeviceVerificationResolvedPayload {
                instance_id,
                session_id,
                outcome: status.outcome,
                resolved_at: status.resolved_at,
            }
            .emit(&handle);
        }
        Err(VerificationError::Cancelled) => {
            debug!("Stopped waiting for device verification {session_id}");
        }
        Err(err) => {
            warn!("Failed to wait for device verification {session_id}: {err}");
            if err == VerificationError::DeviceBlocked {
                set_device_status(&handle, instance_id, DeviceStatus::Blocked).await;
            }
            DeviceVerificationErrorPayload {
                instance_id,
                session_id,
                error: err,
            }
            .emit(&handle);
        }
    }
}

pub(crate) async fn set_device_status(handle: &AppHandle, instance_id: Id, status: DeviceStatus) {
    update_device_status(handle, instance_id, |_| Some(status)).await;
}

/// Core starts sessions only for enabled devices, so an earlier block was lifted. A verified
/// device stays verified until Core reports otherwise.
async fn lift_device_block(handle: &AppHandle, instance_id: Id) {
    update_device_status(handle, instance_id, |current| {
        (current == Some(DeviceStatus::Blocked)).then_some(DeviceStatus::Unverified)
    })
    .await;
}

/// A blocked device keeps no tunnel up, so blocking disconnects its locations and removes its
/// service locations from the daemon. Unblocking pushes the service locations back.
async fn update_device_status(
    handle: &AppHandle,
    instance_id: Id,
    next: impl FnOnce(Option<DeviceStatus>) -> Option<DeviceStatus>,
) {
    let mut instance = match Instance::find_by_id(&*DB_POOL, instance_id).await {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            warn!("Instance {instance_id} not found, not recording its device status");
            return;
        }
        Err(err) => {
            error!("Failed to load instance {instance_id}: {err}");
            return;
        }
    };
    let Some(status) = next(instance.device_status) else {
        return;
    };
    if instance.device_status == Some(status) {
        return;
    }
    // Saved first, so connects and polls that start from here on see it.
    if let Err(err) = Instance::set_device_status(&*DB_POOL, instance_id, status).await {
        error!("Failed to record the device status of {instance}: {err}");
        return;
    }
    let was_blocked = instance.is_device_blocked();
    instance.device_status = Some(status);
    info!("Device status of {instance} is now {status:?}");

    if was_blocked != instance.is_device_blocked() {
        if was_blocked {
            info!("This device is no longer blocked for {instance}");
        } else {
            disconnect_instance_locations(handle, &instance).await;
        }
        sync_service_locations_best_effort(&DB_POOL, &instance).await;
        reload_tray_menu(handle).await;
    }

    if let Err(err) = handle.emit(EventKey::InstanceUpdate.into(), ()) {
        error!("Event Instance Update was not emitted. Reason: {err}");
    }
}

pub(crate) async fn disconnect_instance_locations(handle: &AppHandle, instance: &Instance<Id>) {
    warn!("The administrator blocked this device for {instance}, disconnecting its locations");
    match Location::find_by_instance_id(&*DB_POOL, instance.id, false).await {
        Ok(locations) => {
            let ids = locations.iter().map(|location| location.id).collect();
            if let Err(err) = disconnect_locations(ids, handle.clone()).await {
                error!("Failed to disconnect the locations of {instance}: {err}");
            }
        }
        Err(err) => error!("Failed to load the locations of {instance}: {err}"),
    }
}
