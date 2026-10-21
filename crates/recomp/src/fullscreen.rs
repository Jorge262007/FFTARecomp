//! Borderless requests shared by startup, F11 and the Video menu.
//! Winit/WM own normal placement and maximized restoration. Only repair a
//! formerly visible placement that no longer intersects an available monitor.
use std::time::{Duration, Instant};
use winit::{dpi::{PhysicalPosition, PhysicalSize}, window::{Fullscreen, Window}};

struct Windowed {
    position: Option<PhysicalPosition<i32>>,
    size: PhysicalSize<u32>,
    outer: PhysicalSize<u32>,
    maximized: bool,
}

#[derive(Default)]
pub struct Controller {
    windowed: Option<Windowed>,
    pending: Option<(bool, Instant)>,
}

impl Controller {
    pub fn desired(&self, actual: bool) -> bool {
        self.pending.map_or(actual, |(target, _)| target)
    }

    pub fn request(&mut self, window: &Window, enabled: bool) {
        if enabled && window.fullscreen().is_none() && self.windowed.is_none() {
            self.windowed = Some(Windowed {
                position: window.outer_position().ok(), size: window.inner_size(),
                outer: window.outer_size(), maximized: window.is_maximized(),
            });
        }
        window.set_fullscreen(enabled.then_some(Fullscreen::Borderless(None)));
        self.pending = Some((enabled, Instant::now()));
        if !enabled {
            if let Some(saved) = self.windowed.take() {
                let monitors: Vec<_> = window.available_monitors().collect();
                let visible = saved.position.is_none_or(|position| monitors.iter().any(|m|
                    intersects(position, saved.outer, m.position(), m.size())));
                if !visible {
                    let monitor = window.current_monitor().filter(|m| monitors.contains(m))
                        .or_else(|| window.primary_monitor()).or_else(|| monitors.first().cloned());
                    if let Some(monitor) = monitor {
                        let (position, size) = fallback(monitor.position(), monitor.size(), saved.size);
                        window.set_outer_position(position);
                        if !saved.maximized { let _ = window.request_inner_size(size); }
                        else { window.set_maximized(true); }
                    }
                }
            }
        }
    }

    pub fn observe(&mut self, actual: bool) -> Option<bool> {
        self.observe_at(actual, Instant::now)
    }

    fn observe_at(&mut self, actual: bool, now: impl FnOnce() -> Instant) -> Option<bool> {
        if let Some((target, since)) = self.pending {
            // A compositor can acknowledge after the request returns. Don't
            // overwrite the new preference with that previous window state.
            if target != actual && now().saturating_duration_since(since) < Duration::from_secs(2) {
                return None;
            }
        }
        self.pending = None;
        Some(actual)
    }
}

fn intersects(a: PhysicalPosition<i32>, size: PhysicalSize<u32>,
    b: PhysicalPosition<i32>, monitor: PhysicalSize<u32>) -> bool {
    let overlap = |a: i32, n: u32, b: i32, m: u32| {
        (i64::from(a) + i64::from(n)).min(i64::from(b) + i64::from(m))
            - i64::from(a).max(i64::from(b))
    };
    overlap(a.x, size.width, b.x, monitor.width) >= i64::from(size.width.min(32).max(1))
        && overlap(a.y, size.height, b.y, monitor.height) >= i64::from(size.height.min(32).max(1))
}

fn fallback(origin: PhysicalPosition<i32>, monitor: PhysicalSize<u32>,
    old: PhysicalSize<u32>) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    (PhysicalPosition::new(origin.x.saturating_add(16), origin.y.saturating_add(16)),
        PhysicalSize::new(old.width.min(monitor.width.saturating_sub(64).max(1)).max(1),
            old.height.min(monitor.height.saturating_sub(64).max(1)).max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_ack_cancel_reject_and_external_state_keep_the_right_preference() {
        let now = Instant::now();
        let mut controller = Controller { pending: Some((true, now)), ..Default::default() };
        assert!(controller.desired(false));
        assert_eq!(controller.observe_at(false, || now + Duration::from_millis(500)), None);
        assert_eq!(controller.observe_at(true, || now), Some(true));
        assert_eq!(controller.observe_at(false, || panic!("idle observe must not read a clock")), Some(false));
        controller.pending = Some((false, now));
        assert!(!controller.desired(true));
        assert_eq!(controller.observe_at(true, || now + Duration::from_millis(500)), None);
        assert_eq!(controller.observe_at(true, || now + Duration::from_secs(2)), Some(true));
        assert_eq!(controller.observe_at(false, || now), Some(false));
    }

    #[test]
    fn removed_monitor_fallback_is_visible_and_bounds_oversized_windows() {
        let old = PhysicalPosition::new(-1800, 40);
        let size = PhysicalSize::new(1600, 900);
        assert!(intersects(old, size, PhysicalPosition::new(-1920, 0), PhysicalSize::new(1920, 1080)));
        assert!(!intersects(old, size, PhysicalPosition::new(0, 0), PhysicalSize::new(1280, 720)));
        let (position, restored) = fallback(PhysicalPosition::new(0, 0), PhysicalSize::new(1280, 720), size);
        assert_eq!(restored, PhysicalSize::new(1216, 656));
        assert!(intersects(position, restored, PhysicalPosition::new(0, 0), PhysicalSize::new(1280, 720)));
        assert!(!intersects(PhysicalPosition::new(1279, 0), size,
            PhysicalPosition::new(0, 0), PhysicalSize::new(1280, 720)));
    }
}
