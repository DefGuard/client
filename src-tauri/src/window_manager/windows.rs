use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

use crate::window_manager::{
    WindowManager, COMPACT_WINDOW_HEIGHT, COMPACT_WINDOW_ID, COMPACT_WINDOW_WIDTH,
    FULL_VIEW_WINDOW_HEIGHT, FULL_VIEW_WINDOW_ID, FULL_VIEW_WINDOW_WIDTH, WINDOW_GAP,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TaskbarPosition {
    Bottom,
    Top,
    Left,
    Right,
    HiddenOrNone,
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub is_primary: bool,
    pub physical_x: i32,
    pub physical_y: i32,
    pub physical_width: u32,
    pub physical_height: u32,
    pub scale_factor: f64,
    pub taskbar_position: TaskbarPosition,
    pub taskbar_size: u32,
}

impl WindowManager {
    pub fn get_monitors() -> Vec<MonitorInfo> {
        let mut monitors: Vec<MonitorInfo> = Vec::new();

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
                // Name
                let name_len = info
                    .szDevice
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(info.szDevice.len());
                let name = OsString::from_wide(&info.szDevice[..name_len])
                    .to_string_lossy()
                    .into_owned();

                let is_primary = (info.monitorInfo.dwFlags & 1) != 0;

                // DPI and Scaling
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

                // Taskbar position and size
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
        let state = tauri::Manager::state::<crate::appstate::AppState>(app);
        let tray_pos = *state.tray_click_position.lock().unwrap();
        let monitors = Self::get_monitors();
        let primary = monitors
            .iter()
            .find(|m| m.is_primary)
            .unwrap_or(&monitors[0]);

        let window =
            if let Some(window) = tauri::Manager::get_webview_window(app, COMPACT_WINDOW_ID) {
                let _ = window.unminimize();
                window
            } else {
                Self::build_tray_window(app)?
            };

        let logical_width = COMPACT_WINDOW_WIDTH;
        let logical_height = COMPACT_WINDOW_HEIGHT;

        let physical_width = (logical_width * primary.scale_factor) as i32;
        let physical_height = (logical_height * primary.scale_factor) as i32;

        let physical_gap = (WINDOW_GAP * primary.scale_factor) as i32;

        let work_left = primary.physical_x
            + if primary.taskbar_position == TaskbarPosition::Left {
                primary.taskbar_size as i32
            } else {
                0
            };
        let work_top = primary.physical_y
            + if primary.taskbar_position == TaskbarPosition::Top {
                primary.taskbar_size as i32
            } else {
                0
            };
        let work_right = primary.physical_x + primary.physical_width as i32
            - if primary.taskbar_position == TaskbarPosition::Right {
                primary.taskbar_size as i32
            } else {
                0
            };
        let work_bottom = primary.physical_y + primary.physical_height as i32
            - if primary.taskbar_position == TaskbarPosition::Bottom {
                primary.taskbar_size as i32
            } else {
                0
            };

        let (final_x, final_y) = if let Some(pos) = tray_pos {
            let icon_x = pos.x as i32;
            let icon_y = pos.y as i32;
            let icon_width = 0;
            let icon_height = 0;

            let icon_center_x = icon_x + (icon_width / 2);
            let default_x = icon_center_x - (physical_width / 2);
            let max_x = work_right - physical_gap - physical_width;
            let min_x = work_left + physical_gap;
            let clamped_x = default_x.clamp(min_x, max_x);

            let icon_center_y = icon_y + (icon_height / 2);
            let default_y = icon_center_y - (physical_height / 2);
            let max_y = work_bottom - physical_gap - physical_height;
            let min_y = work_top + physical_gap;
            let clamped_y = default_y.clamp(min_y, max_y);

            match primary.taskbar_position {
                TaskbarPosition::Bottom => {
                    (clamped_x, work_bottom - physical_height - physical_gap)
                }
                TaskbarPosition::Top => (clamped_x, work_top + physical_gap),
                TaskbarPosition::Left => (work_left + physical_gap, clamped_y),
                TaskbarPosition::Right => (work_right - physical_width - physical_gap, clamped_y),
                _ => (clamped_x, work_bottom - physical_height - physical_gap),
            }
        } else {
            let x = work_right - physical_width - physical_gap;
            let y = work_bottom - physical_height - physical_gap;
            (x, y)
        };

        window.set_always_on_top(true)?;
        window.set_position(tauri::PhysicalPosition::new(final_x, final_y))?;
        window.show()?;

        let window_focus = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            // Toggle always_on_top to force Z-order above the Windows tray overflow popup
            let _ = window_focus.set_always_on_top(false);
            let _ = window_focus.set_always_on_top(true);
            let _ = window_focus.set_focus();
        });

        Ok(window)
    }

    pub fn open_full_view(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
        log::info!("open_full_view: Getting monitors");
        let monitors = Self::get_monitors();
        log::info!("open_full_view: Found {} monitors", monitors.len());
        let primary = monitors
            .iter()
            .find(|m| m.is_primary)
            .unwrap_or(&monitors[0]);
        log::info!(
            "open_full_view: Primary monitor scale factor: {}",
            primary.scale_factor
        );

        log::info!("open_full_view: Checking if old-ui window exists");
        let window =
            if let Some(window) = tauri::Manager::get_webview_window(app, FULL_VIEW_WINDOW_ID) {
                log::info!("open_full_view: old-ui window exists, unminimizing");
                let _ = window.unminimize();
                window
            } else {
                log::info!("open_full_view: old-ui window does not exist, building it");
                let win = Self::build_full_view_window(app)?;
                log::info!("open_full_view: old-ui window built successfully");
                win
            };

        log::info!("open_full_view: Querying outer_size");
        let outer_size = window.outer_size().unwrap_or(tauri::PhysicalSize {
            width: (FULL_VIEW_WINDOW_WIDTH * primary.scale_factor) as u32,
            height: (FULL_VIEW_WINDOW_HEIGHT * primary.scale_factor) as u32,
        });
        log::info!("open_full_view: outer_size = {outer_size:?}");

        log::info!("open_full_view: Querying inner_size");
        let inner_size = window.inner_size().unwrap_or(tauri::PhysicalSize {
            width: (FULL_VIEW_WINDOW_WIDTH * primary.scale_factor) as u32,
            height: (FULL_VIEW_WINDOW_HEIGHT * primary.scale_factor) as u32,
        });
        log::info!("open_full_view: inner_size = {inner_size:?}");

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

        log::info!("open_full_view: Setting position to ({window_x}, {window_y})");
        window.set_position(tauri::PhysicalPosition::new(window_x, window_y))?;
        log::info!("open_full_view: Position set, showing window");
        window.show()?;
        log::info!("open_full_view: Window shown successfully");
        Ok(window)
    }
}
