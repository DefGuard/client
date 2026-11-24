#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskbarPosition {
    Bottom,
    Top,
    Left,
    Right,
    HiddenOrNone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl PhysicalRect {
    #[must_use]
    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    #[must_use]
    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    #[must_use]
    pub fn contains_point(&self, (x, y): (i32, i32)) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    #[must_use]
    pub fn contains_rect(&self, other: &Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    fn distance_squared_to(&self, (x, y): (i32, i32)) -> i64 {
        let dx = i64::from((self.x - x).max(x - (self.right() - 1)).max(0));
        let dy = i64::from((self.y - y).max(y - (self.bottom() - 1)).max(0));
        dx * dx + dy * dy
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayArea {
    pub monitor: PhysicalRect,
    pub work: PhysicalRect,
    pub scale_factor: f64,
    pub taskbar: TaskbarPosition,
    pub is_primary: bool,
}

#[must_use]
pub fn pick_display(anchor: Option<(i32, i32)>, displays: &[DisplayArea]) -> Option<&DisplayArea> {
    match anchor {
        Some(point) => displays
            .iter()
            .find(|display| display.monitor.contains_point(point))
            .or_else(|| {
                displays
                    .iter()
                    .min_by_key(|display| display.monitor.distance_squared_to(point))
            }),
        None => displays
            .iter()
            .find(|display| display.is_primary)
            .or_else(|| displays.first()),
    }
}

// min wins over max, so a window larger than the work area sticks to its left or top edge
pub(crate) fn clamp_axis(value: i32, min: i32, max: i32) -> i32 {
    value.min(max).max(min)
}

#[must_use]
pub fn place_tray_window(
    anchor: Option<(i32, i32)>,
    (width, height): (i32, i32),
    gap: i32,
    display: &DisplayArea,
) -> (i32, i32) {
    let work = display.work;
    let min_x = work.x + gap;
    let max_x = work.right() - gap - width;
    let min_y = work.y + gap;
    let max_y = work.bottom() - gap - height;
    let far_x = clamp_axis(max_x, min_x, max_x);
    let far_y = clamp_axis(max_y, min_y, max_y);

    let Some((anchor_x, anchor_y)) = anchor else {
        return (far_x, far_y);
    };
    let centered_x = clamp_axis(anchor_x - width / 2, min_x, max_x);
    let centered_y = clamp_axis(anchor_y - height / 2, min_y, max_y);

    match display.taskbar {
        TaskbarPosition::Top => (centered_x, min_y),
        TaskbarPosition::Left => (min_x, centered_y),
        TaskbarPosition::Right => (far_x, centered_y),
        TaskbarPosition::Bottom | TaskbarPosition::HiddenOrNone => (centered_x, far_y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAP: i32 = 20;
    const TASKBAR: i32 = 48;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> PhysicalRect {
        PhysicalRect {
            x,
            y,
            width,
            height,
        }
    }

    fn display(monitor: PhysicalRect, taskbar: TaskbarPosition, is_primary: bool) -> DisplayArea {
        let work = match taskbar {
            TaskbarPosition::Bottom => rect(
                monitor.x,
                monitor.y,
                monitor.width,
                monitor.height - TASKBAR,
            ),
            TaskbarPosition::Top => rect(
                monitor.x,
                monitor.y + TASKBAR,
                monitor.width,
                monitor.height - TASKBAR,
            ),
            TaskbarPosition::Left => rect(
                monitor.x + TASKBAR,
                monitor.y,
                monitor.width - TASKBAR,
                monitor.height,
            ),
            TaskbarPosition::Right => rect(
                monitor.x,
                monitor.y,
                monitor.width - TASKBAR,
                monitor.height,
            ),
            TaskbarPosition::HiddenOrNone => monitor,
        };
        DisplayArea {
            monitor,
            work,
            scale_factor: 1.0,
            taskbar,
            is_primary,
        }
    }

    fn placed_rect(anchor: (i32, i32), size: (i32, i32), displays: &[DisplayArea]) -> PhysicalRect {
        let target = pick_display(Some(anchor), displays).unwrap();
        let (x, y) = place_tray_window(Some(anchor), size, GAP, target);
        rect(x, y, size.0, size.1)
    }

    fn assert_on_single_monitor(window: PhysicalRect, displays: &[DisplayArea]) {
        let hosts = displays
            .iter()
            .filter(|display| display.work.contains_rect(&window))
            .count();
        assert_eq!(hosts, 1, "{window:?} is not inside exactly one work area");
    }

    fn side_by_side() -> [DisplayArea; 2] {
        [
            display(rect(0, 0, 1920, 1080), TaskbarPosition::Bottom, true),
            display(rect(1920, 0, 2560, 1440), TaskbarPosition::Bottom, false),
        ]
    }

    #[test]
    fn click_near_right_edge_of_left_monitor_sticks_to_that_edge() {
        let displays = side_by_side();
        let window = placed_rect((1910, 1060), (380, 680), &displays);
        assert_eq!(window.right(), 1920 - GAP);
        assert_eq!(window.bottom(), 1080 - TASKBAR - GAP);
        assert_on_single_monitor(window, &displays);
    }

    #[test]
    fn click_near_left_edge_of_right_monitor_sticks_to_that_edge() {
        let displays = side_by_side();
        let window = placed_rect((1925, 1420), (380, 680), &displays);
        assert_eq!(window.x, 1920 + GAP);
        assert_eq!(window.bottom(), 1440 - TASKBAR - GAP);
        assert_on_single_monitor(window, &displays);
    }

    #[test]
    fn click_in_the_middle_centers_on_the_icon() {
        let displays = side_by_side();
        let window = placed_rect((960, 1060), (380, 680), &displays);
        assert_eq!(window.x, 960 - 190);
        assert_on_single_monitor(window, &displays);
    }

    #[test]
    fn scaled_secondary_monitor_with_negative_origin() {
        let mut left = display(rect(-2880, 0, 2880, 1620), TaskbarPosition::Bottom, false);
        left.scale_factor = 1.5;
        let displays = [
            left,
            display(rect(0, 0, 1920, 1080), TaskbarPosition::Bottom, true),
        ];
        let size = (570, 1020);
        let window = placed_rect((-5, 1600), size, &displays);
        assert_eq!(window.right(), -GAP);
        assert_on_single_monitor(window, &displays);
    }

    #[test]
    fn monitor_above_primary_with_top_taskbar() {
        let displays = [
            display(rect(0, -1080, 1920, 1080), TaskbarPosition::Top, false),
            display(rect(0, 0, 1920, 1080), TaskbarPosition::Bottom, true),
        ];
        let window = placed_rect((1900, -1060), (380, 680), &displays);
        assert_eq!(window.y, -1080 + TASKBAR + GAP);
        assert_eq!(window.right(), 1920 - GAP);
        assert_on_single_monitor(window, &displays);
    }

    #[test]
    fn side_taskbars_place_window_next_to_them() {
        let left = [display(rect(0, 0, 1920, 1080), TaskbarPosition::Left, true)];
        let window = placed_rect((20, 1070), (380, 680), &left);
        assert_eq!(window.x, TASKBAR + GAP);
        assert_eq!(window.bottom(), 1080 - GAP);

        let right = [display(
            rect(0, 0, 1920, 1080),
            TaskbarPosition::Right,
            true,
        )];
        let window = placed_rect((1900, 5), (380, 680), &right);
        assert_eq!(window.right(), 1920 - TASKBAR - GAP);
        assert_eq!(window.y, GAP);
    }

    #[test]
    fn anchor_outside_every_monitor_picks_the_nearest() {
        let displays = side_by_side();
        let target = pick_display(Some((5000, 100)), &displays).unwrap();
        assert_eq!(target.monitor.x, 1920);
        let target = pick_display(Some((-300, 100)), &displays).unwrap();
        assert_eq!(target.monitor.x, 0);
    }

    #[test]
    fn missing_anchor_uses_primary_bottom_right() {
        let displays = [
            display(rect(-1920, 0, 1920, 1080), TaskbarPosition::Bottom, false),
            display(rect(0, 0, 1920, 1080), TaskbarPosition::Bottom, true),
        ];
        let target = pick_display(None, &displays).unwrap();
        assert!(target.is_primary);
        let (x, y) = place_tray_window(None, (380, 680), GAP, target);
        assert_eq!((x + 380, y + 680), (1920 - GAP, 1080 - TASKBAR - GAP));
    }

    #[test]
    fn window_larger_than_work_area_pins_to_top_left() {
        let small = [display(
            rect(100, 100, 300, 400),
            TaskbarPosition::Bottom,
            true,
        )];
        let window = placed_rect((390, 490), (380, 680), &small);
        assert_eq!((window.x, window.y), (100 + GAP, 100 + GAP));
    }

    #[test]
    fn clamp_axis_pins_oversized_window_to_the_start_edge() {
        // full view (700 tall) on a 512 tall work area starting at y = 0
        assert_eq!(clamp_axis(-200, 0, 512 - 700), 0);
        assert_eq!(clamp_axis(100, 0, 1080 - 700), 100);
        assert_eq!(clamp_axis(500, 0, 1080 - 700), 380);
    }

    #[test]
    fn no_displays_yields_none() {
        assert!(pick_display(Some((0, 0)), &[]).is_none());
        assert!(pick_display(None, &[]).is_none());
    }
}
