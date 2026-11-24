use defguard_client_core::version::mark_welcome_shown;
use tauri::{
    async_runtime::block_on, AppHandle, Emitter, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
#[cfg(not(windows))]
use tauri::{Monitor, PhysicalPosition};

use crate::{
    appstate::AppState,
    commands::{build_instance_info, build_location_info},
    connection::active_connections::get_connection_id_by_type,
    database::{
        models::{
            instance::Instance,
            location::{Location, LocationMfaMethod},
            tunnel::Tunnel,
            Id,
        },
        DB_POOL,
    },
    error::Error,
    events::{ConfigureFactorsPayload, EventKey},
    tauri_err_to_app_err, ConnectionType,
};

/// Returns `true` if there are any non-service locations in the database.
pub async fn has_non_service_locations() -> bool {
    Location::exist(&*DB_POOL, false).await.unwrap_or_default()
}

/// Returns `true` if the compact (tray) view has anything to show: at least
/// one non-service location or at least one tunnel.
pub async fn has_tray_content() -> bool {
    if has_non_service_locations().await {
        return true;
    }
    Tunnel::exists(&*DB_POOL).await.unwrap_or_default()
}

pub const COMPACT_WINDOW_ID: &str = "compact-view";
pub const FULL_VIEW_WINDOW_ID: &str = "full-view";
pub const WELCOME_WINDOW_ID: &str = "welcome";
pub const WELCOME_WINDOW_WIDTH: f64 = 640.0;
pub const WELCOME_WINDOW_HEIGHT: f64 = 634.0;
pub const COMPACT_WINDOW_WIDTH: f64 = 380.0;
pub const COMPACT_WINDOW_HEIGHT: f64 = 680.0;
pub const FULL_VIEW_WINDOW_WIDTH: f64 = 800.0;
pub const FULL_VIEW_WINDOW_HEIGHT: f64 = 700.0;
#[cfg(not(target_os = "linux"))]
const WINDOW_GAP: f64 = 20.0;
const WINDOW_TITLE: &str = "Defguard";

#[must_use]
pub fn compact_view_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev)) {
        WebviewUrl::External("http://localhost:5072/compact/".parse().unwrap())
    } else {
        WebviewUrl::App("compact/".into())
    }
}

#[must_use]
pub fn full_view_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev)) {
        WebviewUrl::External("http://localhost:5072/full/".parse().unwrap())
    } else {
        WebviewUrl::App("full/".into())
    }
}

#[must_use]
pub fn welcome_ui_url() -> WebviewUrl {
    if cfg!(any(defguard_client_dev)) {
        WebviewUrl::External("http://localhost:5072/welcome/".parse().unwrap())
    } else {
        WebviewUrl::App("welcome/".into())
    }
}

/// Hides any currently visible webview window other than `skip_label`.
fn hide_shown_windows(app: &AppHandle, skip_label: &str) {
    for (label, window) in app.webview_windows() {
        if label != skip_label && window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        }
    }
}

/// Display area without menu bar, Dock or taskbar, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorkArea {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl WorkArea {
    #[cfg(not(windows))]
    fn from_monitor(monitor: &Monitor) -> Self {
        let size = monitor.work_area().size;
        Self {
            width: size.width,
            height: size.height,
            scale_factor: monitor.scale_factor(),
        }
    }

    fn logical_size(&self) -> Option<(f64, f64)> {
        if !self.scale_factor.is_finite()
            || self.scale_factor <= 0.0
            || self.width == 0
            || self.height == 0
        {
            return None;
        }
        Some((
            f64::from(self.width) / self.scale_factor,
            f64::from(self.height) / self.scale_factor,
        ))
    }
}

/// Unusable display data counts as a fit, so a bad reading never takes the tray away.
#[must_use]
pub(crate) fn compact_window_fits(work: WorkArea) -> bool {
    work.logical_size().is_none_or(|(width, height)| {
        width >= COMPACT_WINDOW_WIDTH && height >= COMPACT_WINDOW_HEIGHT
    })
}

#[must_use]
pub(crate) fn full_view_min_size((width, height): (f64, f64)) -> (f64, f64) {
    let relax = |available: f64, default: f64| {
        if available.is_finite() && available > 0.0 {
            default.min(available)
        } else {
            default
        }
    };
    (
        relax(width, FULL_VIEW_WINDOW_WIDTH),
        relax(height, FULL_VIEW_WINDOW_HEIGHT),
    )
}

#[cfg(not(windows))]
pub(crate) fn monitor_for_point(
    app: &AppHandle,
    point: Option<PhysicalPosition<f64>>,
) -> Option<Monitor> {
    if let Some(point) = point {
        if let Ok(Some(monitor)) = app.monitor_from_point(point.x, point.y) {
            return Some(monitor);
        }
    }
    if let Ok(Some(monitor)) = app.primary_monitor() {
        return Some(monitor);
    }
    // On macOS this is sometimes the only working method, and it lists the current monitor first.
    app.available_monitors()
        .ok()
        .and_then(|monitors| monitors.into_iter().next())
}

#[cfg(not(windows))]
fn tray_work_area(app: &AppHandle) -> Option<WorkArea> {
    let anchor = app
        .state::<AppState>()
        .tray_icon_rect
        .lock()
        .unwrap()
        .map(|rect| rect.position);
    monitor_for_point(app, anchor).map(|monitor| WorkArea::from_monitor(&monitor))
}

#[cfg(windows)]
fn tray_work_area(app: &AppHandle) -> Option<WorkArea> {
    windows::tray_work_area(app)
}

pub(crate) fn tray_window_fits(app: &AppHandle) -> bool {
    let Some(work) = tray_work_area(app) else {
        debug!("No display found for the tray window, assuming it fits");
        return true;
    };
    let fits = compact_window_fits(work);
    if !fits {
        info!(
            "Tray window ({COMPACT_WINDOW_WIDTH}x{COMPACT_WINDOW_HEIGHT}) does not fit the display \
            work area {work:?}, using the full view instead"
        );
    }
    fits
}

pub(crate) fn fit_full_view_to_work_area(window: &WebviewWindow, work: WorkArea) {
    let Some((work_width, work_height)) = work.logical_size() else {
        return;
    };
    let scale = window
        .scale_factor()
        .ok()
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .unwrap_or(work.scale_factor);
    let (Ok(inner), Ok(outer)) = (window.inner_size(), window.outer_size()) else {
        return;
    };
    let inner = inner.to_logical::<f64>(scale);
    let outer = outer.to_logical::<f64>(scale);
    // Decorations (the Linux title bar) take part of the work area too.
    let available = (
        (work_width - (outer.width - inner.width).max(0.0)).max(0.0),
        (work_height - (outer.height - inner.height).max(0.0)).max(0.0),
    );

    let (min_width, min_height) = full_view_min_size(available);
    if let Err(err) = window.set_min_size(Some(LogicalSize::new(min_width, min_height))) {
        warn!("Failed to set full view minimum size: {err}");
    }
    if inner.width > available.0 || inner.height > available.1 {
        let width = inner.width.min(available.0).max(min_width);
        let height = inner.height.min(available.1).max(min_height);
        debug!("Shrinking full view to {width}x{height} to fit work area {work:?}");
        if let Err(err) = window.set_size(LogicalSize::new(width, height)) {
            warn!("Failed to shrink full view to the work area: {err}");
        }
    }
}

pub struct WindowManager;

impl WindowManager {
    pub fn build_tray_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = WebviewWindowBuilder::new(app, COMPACT_WINDOW_ID, compact_view_ui_url())
            .title(WINDOW_TITLE)
            .inner_size(COMPACT_WINDOW_WIDTH, COMPACT_WINDOW_HEIGHT)
            // tao on Linux applies resizable(false) only after the first configure event,
            // pinned size hints keep the window manager from resizing it before that
            .min_inner_size(COMPACT_WINDOW_WIDTH, COMPACT_WINDOW_HEIGHT)
            .max_inner_size(COMPACT_WINDOW_WIDTH, COMPACT_WINDOW_HEIGHT)
            .resizable(false)
            .decorations(false)
            .visible(false)
            .always_on_top(true)
            .skip_taskbar(true);
        #[cfg(target_os = "macos")]
        let window = window.hidden_title(true);

        let window = window.build()?;

        #[cfg(target_os = "macos")]
        if let Err(err) = macos::enable_rounded_corners(&window, false) {
            warn!("Failed to enable rounded corners on tray window: {err}");
        }

        Ok(window)
    }

    pub fn build_full_view_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = WebviewWindowBuilder::new(app, FULL_VIEW_WINDOW_ID, full_view_ui_url())
            .title(WINDOW_TITLE)
            .inner_size(FULL_VIEW_WINDOW_WIDTH, FULL_VIEW_WINDOW_HEIGHT)
            .min_inner_size(FULL_VIEW_WINDOW_WIDTH, FULL_VIEW_WINDOW_HEIGHT)
            .decorations(cfg!(not(any(windows, target_os = "macos"))))
            .visible(false)
            .build()?;

        #[cfg(target_os = "macos")]
        if let Err(err) = macos::enable_rounded_corners(&window, true) {
            warn!("Failed to enable rounded corners on full view window: {err}");
        }

        Ok(window)
    }

    pub fn build_welcome_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = WebviewWindowBuilder::new(app, WELCOME_WINDOW_ID, welcome_ui_url())
            .title(WINDOW_TITLE)
            .inner_size(WELCOME_WINDOW_WIDTH, WELCOME_WINDOW_HEIGHT)
            .resizable(false)
            .maximizable(false)
            .decorations(false)
            .skip_taskbar(false)
            .always_on_top(true)
            .visible(false);
        #[cfg(target_os = "macos")]
        let window = window.hidden_title(true);

        let window = window.build()?;

        #[cfg(target_os = "macos")]
        if let Err(err) = macos::enable_rounded_corners(&window, false) {
            warn!("Failed to enable rounded corners on welcome window: {err}");
        }

        Ok(window)
    }
}

#[cfg(not(windows))]
impl WindowManager {
    pub fn open_tray(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = if let Some(window) = app.get_webview_window(COMPACT_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_tray_window(app)?
        };
        #[cfg(target_os = "macos")]
        {
            macos::position_window_near_tray(app, &window);
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let _ = app.set_dock_visibility(false);
            let _ = app.show();
        }
        let _ = window.show();
        let _ = window.set_focus();
        Ok(window)
    }

    pub fn open_full_view(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        let window = if let Some(window) = app.get_webview_window(FULL_VIEW_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_full_view_window(app)?
        };
        let monitor = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| monitor_for_point(app, None));
        if let Some(monitor) = monitor {
            fit_full_view_to_work_area(&window, WorkArea::from_monitor(&monitor));
        }
        #[cfg(target_os = "macos")]
        {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
            let _ = app.set_dock_visibility(true);
            let _ = app.show();
        }
        let _ = window.show();
        let _ = window.set_focus();
        Ok(window)
    }

    pub fn open_welcome_view(app: &AppHandle) -> tauri::Result<WebviewWindow> {
        hide_shown_windows(app, WELCOME_WINDOW_ID);

        let window = if let Some(window) = app.get_webview_window(WELCOME_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_welcome_window(app)?
        };
        #[cfg(target_os = "macos")]
        let _ = app.set_dock_visibility(true);
        #[cfg(target_os = "macos")]
        let _ = app.show();
        let _ = window.show();
        let _ = window.set_focus();
        Ok(window)
    }
}

#[cfg(any(windows, test))]
pub mod placement;

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

/// Opens the full view instead when the tray window does not fit, returns the shown window label.
pub(crate) fn show_tray_window(app: &AppHandle) -> &'static str {
    if tray_window_fits(app) {
        if let Err(err) = WindowManager::open_tray(app) {
            error!("Failed to open tray window: {err}");
        }
        COMPACT_WINDOW_ID
    } else {
        if let Err(err) = WindowManager::open_full_view(app) {
            error!("Failed to open full view in place of the tray window: {err}");
        }
        FULL_VIEW_WINDOW_ID
    }
}

fn prefers_full_view(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .app_config
        .lock()
        .unwrap()
        .open_in_full_window
}

pub fn show_tray_or_full_view(app: &AppHandle) {
    if !prefers_full_view(app) && block_on(has_tray_content()) && tray_window_fits(app) {
        if let Some(full_view) = app.get_webview_window(FULL_VIEW_WINDOW_ID) {
            let full_view_visible = full_view.is_visible().ok().unwrap_or(false);
            if full_view_visible {
                let _ = full_view.hide();
            }
        }
        if let Err(err) = WindowManager::open_tray(app) {
            error!("Failed to open tray window: {err}");
        }
    } else {
        let _ = WindowManager::open_full_view(app);
    }
}

fn mfa_trigger_payload(location: &Location<Id>) -> Location<Id> {
    let mut payload = location.clone();
    payload.mfa_steps.0 = location.effective_mfa_steps();
    payload
}

/// Surface the window that should host the MFA flow, and emit `MfaTrigger` targeted at that window.
pub fn trigger_mfa(app: &AppHandle, location: &Location<Id>) {
    let target = if prefers_full_view(app) {
        let _ = WindowManager::open_full_view(app);
        FULL_VIEW_WINDOW_ID
    } else if let Some(window) = app
        .get_webview_window(FULL_VIEW_WINDOW_ID)
        .filter(|w| w.is_visible().unwrap_or(false))
    {
        let _ = window.unminimize();
        let _ = window.set_focus();
        FULL_VIEW_WINDOW_ID
    } else {
        show_tray_window(app)
    };
    let payload = mfa_trigger_payload(location);
    let _ = app.emit_to(target, EventKey::MfaTrigger.into(), &payload);
}

/// Show the full view with the "enable system extension" modal.
pub fn show_system_extension_required(app: &AppHandle) {
    swap_to_full_view(app.clone());
    if let Err(err) = app.emit_to(
        FULL_VIEW_WINDOW_ID,
        EventKey::SystemExtensionRequired.into(),
        (),
    ) {
        error!("Failed to emit system extension required event: {err:?}");
    }
}

#[tauri::command]
pub fn open_tray_window(app: AppHandle) {
    show_tray_window(&app);
}

#[tauri::command]
pub fn open_full_view_window(app: AppHandle) {
    let _ = WindowManager::open_full_view(&app);
}

#[tauri::command]
pub fn swap_to_full_view(app: AppHandle) {
    info!("swap_to_full_view called");
    if let Some(window) = app.get_webview_window(COMPACT_WINDOW_ID) {
        if let Err(err) = window.hide() {
            error!("swap_to_full_view task: Failed to hide new-ui window: {err:?}");
        }
    }
    if let Err(err) = WindowManager::open_full_view(&app) {
        error!("swap_to_full_view task: Failed to open full view: {err:?}");
    } else if let Err(err) = app.emit(EventKey::WindowSwapped.into(), ()) {
        error!("swap_to_full_view task: Failed to emit window swapped event: {err:?}");
    }
}

/// Surface the full view and hand it an MFA configuration request. Callable from either window,
/// so the tray gets out of the way the way `swap_to_full_view` does.
///
/// Both webviews are pre-built hidden at startup, so the full view is already listening by the
/// time this runs and the event cannot race window creation.
#[tauri::command(async)]
pub async fn initiate_configure_factor_screen(
    app: AppHandle,
    instance_id: Id,
    methods: Option<Vec<LocationMfaMethod>>,
    source: String,
    location_id: Option<Id>,
) -> Result<(), Error> {
    debug!("Received a command to open the configure factors screen for instance {instance_id} (source: {source})");
    let Some(instance) = Instance::find_by_id(&*DB_POOL, instance_id).await? else {
        error!("Configure factors requested for unknown instance {instance_id}");
        return Err(Error::NotFound);
    };
    let connected_location_ids = get_connection_id_by_type(ConnectionType::Location).await;
    let instance = build_instance_info(instance, &connected_location_ids).await?;
    // A location that went away in the meantime is not worth refusing the screen over, it just
    // loses the steps it would have spoken to.
    let location = match location_id {
        Some(location_id) => {
            if let Some(location) = Location::find_by_id(&*DB_POOL, location_id).await? {
                Some(build_location_info(location, &connected_location_ids))
            } else {
                warn!("Configure factors requested from unknown location {location_id}");
                None
            }
        }
        None => None,
    };

    if let Some(window) = app.get_webview_window(COMPACT_WINDOW_ID) {
        if let Err(err) = window.hide() {
            error!("initiate_configure_factor_screen: Failed to hide new-ui window: {err:?}");
        }
    }
    WindowManager::open_full_view(&app).map_err(tauri_err_to_app_err)?;
    app.emit_to(
        FULL_VIEW_WINDOW_ID,
        EventKey::ConfigureFactorsTrigger.into(),
        ConfigureFactorsPayload {
            instance,
            methods: methods.unwrap_or_default(),
            source,
            location,
        },
    )
    .map_err(tauri_err_to_app_err)?;
    info!("Configure factors screen requested for instance {instance_id}");
    Ok(())
}

#[tauri::command]
pub fn close_tray_window(app: AppHandle) {
    info!("close_tray_window called");

    if let Some(window) = app.get_webview_window(COMPACT_WINDOW_ID) {
        info!("close_tray_window task: Hiding new-ui window");
        if let Err(err) = window.hide() {
            error!("close_tray_window task: Failed to hide new-ui window: {err:?}");
        }
    } else {
        warn!("close_tray_window task: new-ui window not found");
    }
}

#[tauri::command]
pub fn close_welcome_window(app: AppHandle) {
    info!("close_welcome_window called");

    match app.path().app_data_dir() {
        Ok(config_dir) => mark_welcome_shown(&config_dir, &app.package_info().version),
        Err(err) => error!(
            "close_welcome_window: Failed to access app data directory, the welcome screen will \
            show again on next launch: {err}"
        ),
    }

    show_tray_or_full_view(&app);

    // Destroy rather than hide: a hidden webview keeps running, and the looping welcome
    // video holds a media power request that stops the system from sleeping.
    let destroy_handle = app.clone();
    if let Err(err) = app.run_on_main_thread(move || {
        if let Some(window) = destroy_handle.get_webview_window(WELCOME_WINDOW_ID) {
            if let Err(err) = window.destroy() {
                error!("close_welcome_window task: Failed to destroy welcome window: {err:?}");
            }
        } else {
            warn!("close_welcome_window task: welcome window not found");
        }
    }) {
        error!("close_welcome_window task: Failed to schedule welcome window teardown: {err:?}");
    }
}

#[tauri::command]
pub fn swap_to_tray(app: AppHandle) {
    info!("swap_to_tray called");
    if !tray_window_fits(&app) {
        info!("swap_to_tray: Tray window does not fit the display, keeping the full view");
        return;
    }
    if let Err(err) = WindowManager::open_tray(&app) {
        error!("swap_to_tray task: Failed to open tray window: {err:?}");
    }
    if let Some(window) = app.get_webview_window(FULL_VIEW_WINDOW_ID) {
        if let Err(err) = window.hide() {
            error!("swap_to_tray task: Failed to hide full-view window: {err:?}");
        }
    }
    if let Err(err) = app.emit(EventKey::WindowSwapped.into(), ()) {
        error!("swap_to_tray task: Failed to emit window swapped event: {err:?}");
    }
}

#[cfg(test)]
mod tests;
