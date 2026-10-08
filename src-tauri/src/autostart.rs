//! Launch at login.
//! macOS uses `SMAppService`, since the sandboxed app can't install a Launch Agent.

// TODO: check https://github.com/tauri-apps/plugins-workspace/issues/2342 and switch back to
// the plugin when fixed.

use tauri::AppHandle;

use crate::error::Error;

#[cfg(target_os = "macos")]
fn auto_launch() -> auto_launch::AutoLaunch {
    auto_launch::AutoLaunch::new(
        "",
        "",
        auto_launch::MacOSLaunchMode::SMAppService,
        &[] as &[&str],
        &[] as &[&str],
        "",
    )
}

#[tauri::command]
pub fn get_autostart_enabled(app_handle: AppHandle) -> Result<bool, Error> {
    debug!("Checking if autostart is enabled.");
    #[cfg(target_os = "macos")]
    let enabled = {
        let _ = app_handle;
        auto_launch().is_enabled()
    };
    #[cfg(not(target_os = "macos"))]
    let enabled = {
        use tauri_plugin_autostart::ManagerExt;
        app_handle.autolaunch().is_enabled()
    };
    debug!("Autostart enabled: {enabled:?}");
    enabled.map_err(|err| Error::InternalError(format!("Failed to read autostart state: {err}")))
}

#[tauri::command]
pub fn set_autostart_enabled(app_handle: AppHandle, enabled: bool) -> Result<(), Error> {
    debug!("Setting autostart enabled: {enabled}");
    #[cfg(target_os = "macos")]
    let result = {
        let _ = app_handle;
        let auto_launch = auto_launch();
        if enabled {
            auto_launch.enable()
        } else {
            auto_launch.disable()
        }
    };
    #[cfg(not(target_os = "macos"))]
    let result = {
        use tauri_plugin_autostart::ManagerExt;
        let auto_launch = app_handle.autolaunch();
        if enabled {
            auto_launch.enable()
        } else {
            auto_launch.disable()
        }
    };
    result.map_err(|err| {
        error!("Failed to set autostart enabled to {enabled}: {err}");
        Error::InternalError(format!("Failed to update autostart: {err}"))
    })
}
