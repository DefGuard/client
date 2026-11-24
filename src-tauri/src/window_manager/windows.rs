use std::{ffi::OsString, os::windows::ffi::OsStringExt};

use tauri::{Manager, PhysicalPosition, WebviewWindow};
use windows::Win32::{
    Foundation::{LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
    UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
};

use crate::{
    appstate::{AppState, TrayIconRect},
    tray::TRAY_ICON_ID,
    window_manager::{
        fit_full_view_to_work_area, hide_shown_windows,
        placement::{
            clamp_axis, pick_display, place_tray_window, DisplayArea, PhysicalRect, TaskbarPosition,
        },
        WindowManager, WorkArea, COMPACT_WINDOW_HEIGHT, COMPACT_WINDOW_ID, COMPACT_WINDOW_WIDTH,
        FULL_VIEW_WINDOW_HEIGHT, FULL_VIEW_WINDOW_ID, FULL_VIEW_WINDOW_WIDTH,
        WELCOME_WINDOW_HEIGHT, WELCOME_WINDOW_ID, WELCOME_WINDOW_WIDTH, WINDOW_GAP,
    },
};

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub is_primary: bool,
    pub physical_x: i32,
    pub physical_y: i32,
    pub physical_width: u32,
    pub physical_height: u32,
    pub work_area: PhysicalRect,
    pub scale_factor: f64,
    pub taskbar_position: TaskbarPosition,
    pub taskbar_size: u32,
}

impl MonitorInfo {
    #[must_use]
    pub fn display_area(&self) -> DisplayArea {
        DisplayArea {
            monitor: PhysicalRect {
                x: self.physical_x,
                y: self.physical_y,
                width: self.physical_width as i32,
                height: self.physical_height as i32,
            },
            work: self.work_area,
            scale_factor: self.scale_factor,
            taskbar: self.taskbar_position,
            is_primary: self.is_primary,
        }
    }
}

fn work_area(display: &DisplayArea) -> WorkArea {
    WorkArea {
        width: u32::try_from(display.work.width).unwrap_or(0),
        height: u32::try_from(display.work.height).unwrap_or(0),
        scale_factor: display.scale_factor,
    }
}

// Windows reports no monitors during display sleep, hot-plug, driver reset or RDP session switch
fn primary_monitor(monitors: &[MonitorInfo]) -> Option<&MonitorInfo> {
    monitors
        .iter()
        .find(|monitor| monitor.is_primary)
        .or_else(|| monitors.first())
}

fn show_centered(window: &WebviewWindow) -> tauri::Result<()> {
    if let Err(err) = window.center() {
        warn!("Failed to center window without monitor information: {err}");
    }
    window.show()?;
    window.set_focus()
}

fn rect_from_win32(rect: RECT) -> PhysicalRect {
    PhysicalRect {
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    }
}

fn tray_anchor(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    let current = app
        .tray_by_id(TRAY_ICON_ID)
        .and_then(|tray| tray.rect().ok().flatten())
        .map(|rect| TrayIconRect::from_rect(&rect));
    let rect = current.or_else(|| *app.state::<AppState>().tray_icon_rect.lock().unwrap());
    rect.map(|rect| rect.center())
}

// must pick the same display as open_tray
pub(super) fn tray_work_area(app: &tauri::AppHandle) -> Option<WorkArea> {
    let displays: Vec<DisplayArea> = WindowManager::get_monitors()
        .iter()
        .map(MonitorInfo::display_area)
        .collect();
    pick_display(tray_anchor(app), &displays).map(work_area)
}

fn set_tray_window_position(window: &WebviewWindow, (x, y): (i32, i32)) -> tauri::Result<()> {
    let position = PhysicalPosition::new(x, y);
    // stored before moving so the Moved handler in gui.rs recognizes this move as ours
    *window
        .state::<AppState>()
        .tray_window_position
        .lock()
        .unwrap() = Some(position);
    window.set_position(position)
}

// Windows rescales a window after it moves onto a monitor with another DPI, which can push it
// past the work area edge we placed it against
fn keep_tray_window_inside(
    window: &WebviewWindow,
    anchor: Option<(i32, i32)>,
    gap: i32,
    display: &DisplayArea,
) {
    let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) else {
        return;
    };
    let current = PhysicalRect {
        x: position.x,
        y: position.y,
        width: size.width as i32,
        height: size.height as i32,
    };
    if display.work.contains_rect(&current) {
        return;
    }
    let (x, y) = place_tray_window(anchor, (current.width, current.height), gap, display);
    debug!("Tray window at {current:?} left its work area, moving it to ({x}, {y})");
    if let Err(err) = set_tray_window_position(window, (x, y)) {
        warn!("Failed to move tray window back into its work area: {err}");
    }
}

impl WindowManager {
    pub fn get_monitors() -> Vec<MonitorInfo> {
        let mut monitors = Vec::new();

        unsafe extern "system" fn monitor_enum_proc(
            hmonitor: HMONITOR,
            _hdc: HDC,
            _rect: *mut RECT,
            lparam: LPARAM,
        ) -> windows::core::BOOL {
            let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);

            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

            if GetMonitorInfoW(hmonitor, &mut info as *mut _ as *mut _).as_bool() {
                let name_len = info
                    .szDevice
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(info.szDevice.len());
                let name = OsString::from_wide(&info.szDevice[..name_len])
                    .to_string_lossy()
                    .into_owned();

                let is_primary = (info.monitorInfo.dwFlags & 1) != 0;

                let mut dpi_x = 0;
                let mut dpi_y = 0;
                let scale_factor = if GetDpiForMonitor(
                    hmonitor,
                    MDT_EFFECTIVE_DPI,
                    &mut dpi_x,
                    &mut dpi_y,
                )
                .is_ok()
                {
                    dpi_x as f64 / 96.0
                } else {
                    1.0
                };

                let physical_x = info.monitorInfo.rcMonitor.left;
                let physical_y = info.monitorInfo.rcMonitor.top;
                let physical_width = (info.monitorInfo.rcMonitor.right
                    - info.monitorInfo.rcMonitor.left)
                    .unsigned_abs();
                let physical_height = (info.monitorInfo.rcMonitor.bottom
                    - info.monitorInfo.rcMonitor.top)
                    .unsigned_abs();

                let mut taskbar_position = TaskbarPosition::HiddenOrNone;
                let mut taskbar_size = 0;

                let mon = info.monitorInfo.rcMonitor;
                let work = info.monitorInfo.rcWork;

                if work.bottom < mon.bottom {
                    taskbar_position = TaskbarPosition::Bottom;
                    taskbar_size = (mon.bottom - work.bottom).unsigned_abs();
                } else if work.top > mon.top {
                    taskbar_position = TaskbarPosition::Top;
                    taskbar_size = (work.top - mon.top).unsigned_abs();
                } else if work.left > mon.left {
                    taskbar_position = TaskbarPosition::Left;
                    taskbar_size = (work.left - mon.left).unsigned_abs();
                } else if work.right < mon.right {
                    taskbar_position = TaskbarPosition::Right;
                    taskbar_size = (mon.right - work.right).unsigned_abs();
                }

                monitors.push(MonitorInfo {
                    name,
                    is_primary,
                    physical_x,
                    physical_y,
                    physical_width,
                    physical_height,
                    work_area: rect_from_win32(work),
                    scale_factor,
                    taskbar_position,
                    taskbar_size,
                });
            }

            true.into()
        }

        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(monitor_enum_proc),
                LPARAM(&mut monitors as *mut _ as isize),
            );
        }

        monitors
    }

    pub fn open_tray(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
        let anchor = tray_anchor(app);
        let displays: Vec<DisplayArea> = Self::get_monitors()
            .iter()
            .map(MonitorInfo::display_area)
            .collect();

        let window = if let Some(window) = app.get_webview_window(COMPACT_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_tray_window(app)?
        };

        window.set_always_on_top(true)?;

        let placement = pick_display(anchor, &displays).copied().map(|display| {
            let gap = (WINDOW_GAP * display.scale_factor) as i32;
            (display, gap)
        });
        if let Some((display, gap)) = placement {
            let size = (
                (COMPACT_WINDOW_WIDTH * display.scale_factor) as i32,
                (COMPACT_WINDOW_HEIGHT * display.scale_factor) as i32,
            );
            set_tray_window_position(&window, place_tray_window(anchor, size, gap, &display))?;
            keep_tray_window_inside(&window, anchor, gap, &display);
        } else {
            warn!("No monitors found, tray window keeps its previous position");
        }

        window.show()?;

        let window_focus = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if let Some((display, gap)) = placement {
                keep_tray_window_inside(&window_focus, anchor, gap, &display);
            }
            // Toggle always_on_top to force Z-order above the Windows tray overflow popup
            let _ = window_focus.set_always_on_top(false);
            let _ = window_focus.set_always_on_top(true);
            let _ = window_focus.set_focus();
        });

        Ok(window)
    }

    pub fn open_full_view(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
        info!("open_full_view: Getting monitors");
        let monitors = Self::get_monitors();
        info!("open_full_view: Found {} monitors", monitors.len());
        let primary = primary_monitor(&monitors);
        if let Some(primary) = primary {
            info!(
                "open_full_view: Primary monitor scale factor: {}",
                primary.scale_factor
            );
        }

        info!("open_full_view: Checking if full view window exists");
        let window = if let Some(window) = app.get_webview_window(FULL_VIEW_WINDOW_ID) {
            info!("open_full_view: full view window exists, unminimizing");
            let _ = window.unminimize();
            window
        } else {
            info!("open_full_view: full view window does not exist, building it");
            let win = Self::build_full_view_window(app)?;
            info!("open_full_view: full view window built successfully");
            win
        };

        let Some(primary) = primary else {
            warn!("open_full_view: No monitors found, centering the full view window");
            show_centered(&window)?;
            return Ok(window);
        };

        // Shrink before measuring, so the position below is computed for the final size.
        fit_full_view_to_work_area(&window, work_area(&primary.display_area()));

        info!("open_full_view: Querying outer_size");
        let outer_size = window.outer_size().unwrap_or(tauri::PhysicalSize {
            width: (FULL_VIEW_WINDOW_WIDTH * primary.scale_factor) as u32,
            height: (FULL_VIEW_WINDOW_HEIGHT * primary.scale_factor) as u32,
        });
        info!("open_full_view: outer_size = {outer_size:?}");

        info!("open_full_view: Querying inner_size");
        let inner_size = window.inner_size().unwrap_or(tauri::PhysicalSize {
            width: (FULL_VIEW_WINDOW_WIDTH * primary.scale_factor) as u32,
            height: (FULL_VIEW_WINDOW_HEIGHT * primary.scale_factor) as u32,
        });
        info!("open_full_view: inner_size = {inner_size:?}");

        let physical_width = outer_size.width as i32;
        let physical_height = outer_size.height as i32;

        // Windows invisible borders (shadows) are included in outer_size for decorated windows.
        let border_thickness = (physical_width - (inner_size.width as i32)) / 2;
        let visible_height = physical_height - border_thickness;

        let physical_gap = (WINDOW_GAP * primary.scale_factor) as i32;

        let center_x = primary.physical_x + (primary.physical_width as i32 / 2);
        let center_y = primary.physical_y + (primary.physical_height as i32 / 2);

        let mut window_x = center_x - (physical_width / 2);
        let mut window_y = center_y - (visible_height / 2);

        let taskbar_size = primary.taskbar_size as i32;

        match primary.taskbar_position {
            TaskbarPosition::Bottom => {
                let max_y = primary.physical_y + primary.physical_height as i32
                    - taskbar_size
                    - physical_gap;
                if window_y + visible_height > max_y {
                    window_y = max_y - visible_height;
                }
            }
            TaskbarPosition::Top => {
                let min_y = primary.physical_y + taskbar_size + physical_gap;
                if window_y < min_y {
                    window_y = min_y;
                }
            }
            TaskbarPosition::Left => {
                let min_x = primary.physical_x + taskbar_size + physical_gap;
                if window_x + border_thickness < min_x {
                    window_x = min_x - border_thickness;
                }
            }
            TaskbarPosition::Right => {
                let max_x = primary.physical_x + primary.physical_width as i32
                    - taskbar_size
                    - physical_gap;
                if window_x + physical_width - border_thickness > max_x {
                    window_x = max_x - physical_width + border_thickness;
                }
            }
            _ => {}
        }

        // Keep the top-left corner inside the work area, so a window larger than the display
        // still has its top edge reachable.
        let work = primary.work_area;
        window_x = clamp_axis(
            window_x,
            work.x - border_thickness,
            work.right() + border_thickness - physical_width,
        );
        window_y = clamp_axis(window_y, work.y, work.bottom() - visible_height);

        info!("open_full_view: Setting position to ({window_x}, {window_y})");
        window.set_position(tauri::PhysicalPosition::new(window_x, window_y))?;
        info!("open_full_view: Position set, showing window");
        window.show()?;
        info!("open_full_view: Window shown successfully");
        window.set_focus()?;
        Ok(window)
    }

    pub fn open_welcome_view(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
        hide_shown_windows(app, WELCOME_WINDOW_ID);

        let monitors = Self::get_monitors();

        let window = if let Some(window) = app.get_webview_window(WELCOME_WINDOW_ID) {
            let _ = window.unminimize();
            window
        } else {
            Self::build_welcome_window(app)?
        };

        let Some(primary) = primary_monitor(&monitors) else {
            warn!("open_welcome_view: No monitors found, centering the welcome window");
            show_centered(&window)?;
            return Ok(window);
        };

        let outer_size = window.outer_size().unwrap_or(tauri::PhysicalSize {
            width: (WELCOME_WINDOW_WIDTH * primary.scale_factor) as u32,
            height: (WELCOME_WINDOW_HEIGHT * primary.scale_factor) as u32,
        });

        let physical_width = outer_size.width as i32;
        let physical_height = outer_size.height as i32;

        let center_x = primary.physical_x + (primary.physical_width as i32 / 2);
        let center_y = primary.physical_y + (primary.physical_height as i32 / 2);

        let work = primary.work_area;
        let window_x = clamp_axis(
            center_x - (physical_width / 2),
            work.x,
            work.right() - physical_width,
        );
        let window_y = clamp_axis(
            center_y - (physical_height / 2),
            work.y,
            work.bottom() - physical_height,
        );

        window.set_position(tauri::PhysicalPosition::new(window_x, window_y))?;
        window.show()?;
        window.set_focus()?;
        Ok(window)
    }
}
