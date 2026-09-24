//! Attaching a security key prompt to the window that asked for it. An unowned dialog opens
//! behind the application, leaving the user on an apparently frozen panel.

use defguard_client_fido2::PlatformContext;
use tauri::WebviewWindow;

/// Always the window the command was invoked from, since FIDO2 is reachable from both the tray
/// panel and the full view, and the foreground window may belong to another application.
#[cfg(windows)]
pub(crate) fn platform_context(window: &WebviewWindow) -> PlatformContext {
    match window.hwnd() {
        // The receiving crate links a different major `windows` version, where `HWND` is a
        // distinct type even though the value is the same.
        Ok(hwnd) => PlatformContext::new(Some(hwnd.0 as isize)),
        Err(err) => {
            warn!("Could not find the window to attach the security key prompt to: {err}");
            PlatformContext::default()
        }
    }
}

/// Backends that drive the key directly have no prompt to attach.
#[cfg(not(windows))]
pub(crate) fn platform_context(_window: &WebviewWindow) -> PlatformContext {
    PlatformContext::default()
}

/// The tray panel is `always_on_top`, which would cover the platform's security prompt. Drops
/// the flag for the length of the ceremony and restores it on drop.
pub(crate) struct WindowLevelGuard<'a> {
    window: &'a WebviewWindow,
    restore: bool,
}

impl<'a> WindowLevelGuard<'a> {
    pub(crate) fn lower(window: &'a WebviewWindow) -> Self {
        // Elsewhere there is no platform prompt for the panel to compete with.
        if !cfg!(windows) {
            return Self {
                window,
                restore: false,
            };
        }

        let restore = window.is_always_on_top().unwrap_or(false);
        if restore {
            if let Err(err) = window.set_always_on_top(false) {
                warn!("Could not lower the window for the security key prompt: {err}");
                return Self {
                    window,
                    restore: false,
                };
            }
        }
        Self { window, restore }
    }
}

impl Drop for WindowLevelGuard<'_> {
    fn drop(&mut self) {
        if self.restore {
            if let Err(err) = self.window.set_always_on_top(true) {
                warn!("Could not restore the window after the security key prompt: {err}");
            }
        }
    }
}
