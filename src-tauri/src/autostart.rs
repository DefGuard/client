use serde::Serialize;

/// Registration state reported by the operating system for launch at login.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutostartStatus {
    Enabled,
    Disabled,
    RequiresApproval,
    NotFound,
}

#[cfg(target_os = "macos")]
fn macos_service() -> objc2::rc::Retained<objc2_service_management::SMAppService> {
    // SAFETY: mainAppService is an Objective-C class method returning an owned object.
    unsafe { objc2_service_management::SMAppService::mainAppService() }
}

#[cfg(target_os = "macos")]
fn macos_status(service: &objc2_service_management::SMAppService) -> AutostartStatus {
    use objc2_service_management::SMAppServiceStatus;

    // SAFETY: status reads the ServiceManagement registration for the main app.
    match unsafe { service.status() } {
        SMAppServiceStatus::Enabled => AutostartStatus::Enabled,
        SMAppServiceStatus::NotRegistered => AutostartStatus::Disabled,
        SMAppServiceStatus::RequiresApproval => AutostartStatus::RequiresApproval,
        _ => AutostartStatus::NotFound,
    }
}

#[tauri::command]
pub fn get_autostart_status(app: tauri::AppHandle) -> Result<AutostartStatus, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        Ok(macos_status(&macos_service()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        use tauri_plugin_autostart::ManagerExt;

        app.autolaunch()
            .is_enabled()
            .map(|enabled| {
                if enabled {
                    AutostartStatus::Enabled
                } else {
                    AutostartStatus::Disabled
                }
            })
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub fn set_autostart_enabled(
    app: tauri::AppHandle,
    enabled: bool,
) -> Result<AutostartStatus, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        let service = macos_service();
        let status = macos_status(&service);
        if enabled && matches!(status, AutostartStatus::Disabled) {
            // SAFETY: ServiceManagement registers the calling signed main application.
            unsafe { service.registerAndReturnError() }
                .map_err(|error| error.localizedDescription().to_string())?;
        } else if !enabled
            && matches!(
                status,
                AutostartStatus::Enabled | AutostartStatus::RequiresApproval
            )
        {
            // SAFETY: ServiceManagement unregisters only this main application.
            unsafe { service.unregisterAndReturnError() }
                .map_err(|error| error.localizedDescription().to_string())?;
        } else if matches!(status, AutostartStatus::NotFound) {
            return Err("Main app login service is unavailable".into());
        }
        Ok(macos_status(&service))
    }
    #[cfg(not(target_os = "macos"))]
    {
        use tauri_plugin_autostart::ManagerExt;

        if enabled {
            app.autolaunch()
                .enable()
                .map_err(|error| error.to_string())?;
        } else {
            app.autolaunch()
                .disable()
                .map_err(|error| error.to_string())?;
        }
        get_autostart_status(app)
    }
}
