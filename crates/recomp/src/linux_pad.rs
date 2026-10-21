//! Linux controller snapshots. Gilrs/evdev owns device discovery and canonical
//! mappings; gameplay and remapping consume one stable state per host frame.
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
const BUTTONS: [gilrs::Button; 17] = {
    use gilrs::Button::*;
    [South, East, West, North, LeftTrigger, RightTrigger, LeftTrigger2,
     RightTrigger2, Select, Start, Mode, LeftThumb, RightThumb,
     DPadUp, DPadDown, DPadLeft, DPadRight]
};
const TOKENS: [&str; 25] = [
    "South", "East", "West", "North", "LeftTrigger", "RightTrigger",
    "LeftTrigger2", "RightTrigger2", "Select", "Start", "Mode", "LeftThumb",
    "RightThumb", "DPadUp", "DPadDown", "DPadLeft", "DPadRight",
    "LeftStickRight", "LeftStickLeft", "LeftStickUp", "LeftStickDown",
    "RightStickRight", "RightStickLeft", "RightStickUp", "RightStickDown",
];

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Snapshot(u32);
impl Snapshot {
    #[cfg(target_os = "linux")]
    pub fn read(pad: &gilrs::Gamepad, deadzone: f32, previous: Self) -> Self {
        use gilrs::Axis::*;
        let buttons = BUTTONS.map(|button| {
            if pad.is_pressed(button) { 1. } else {
                pad.button_data(button).map_or(0., |data| data.value())
            }
        });
        Self::sample(buttons, [pad.value(LeftStickX), pad.value(LeftStickY),
            pad.value(RightStickX), pad.value(RightStickY)], deadzone, previous)
    }
    fn sample(buttons: [f32; 17], axes: [f32; 4], deadzone: f32, previous: Self) -> Self {
        let mut mask = 0;
        for (i, value) in buttons.into_iter().enumerate() {
            let threshold = if previous.0 & (1 << i) != 0 { 0.4 } else { 0.55 };
            if value.is_finite() && value > threshold { mask |= 1 << i; }
        }
        let deadzone = if deadzone.is_finite() { deadzone.clamp(0.12, 0.95) } else { 0.5 };
        for (axis, value) in axes.into_iter().enumerate() {
            for (side, signed) in [value, -value].into_iter().enumerate() {
                let bit = 17 + axis * 2 + side;
                let threshold = if previous.0 & (1 << bit) != 0 { deadzone * 0.75 } else { deadzone };
                if signed.is_finite() && signed > threshold { mask |= 1 << bit; }
            }
        }
        Self(mask)
    }
    pub fn pressed(self, token: &str) -> bool {
        TOKENS.iter().position(|name| *name == token).is_some_and(|i| self.0 & (1 << i) != 0)
    }
}

#[derive(Default)]
pub struct Pad {
    pub state: Snapshot,
    locked: bool,
    neutral_since: Option<Instant>,
}
pub struct Update {
    pub capture: Option<&'static str>,
    pub activity: bool,
}
impl Pad {
    pub fn blocked(&self) -> bool { self.locked }
    pub fn poll(&mut self, next: Snapshot, capturing: bool, now: Instant) -> Update {
        let rising = next.0 & !self.state.0;
        self.state = next;
        if self.locked {
            if next.0 != 0 { self.neutral_since = None; }
            else {
                let since = *self.neutral_since.get_or_insert(now);
                if now.saturating_duration_since(since) >= Duration::from_millis(20) {
                    self.locked = false;
                    self.neutral_since = None;
                }
            }
        }
        let capture = if capturing && !self.locked && rising != 0 {
            self.locked = true;
            self.neutral_since = None;
            Some(TOKENS[rising.trailing_zeros() as usize])
        } else { None };
        Update { capture, activity: rising != 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn button(i: usize) -> Snapshot { Snapshot(1 << i) }
    #[test]
    fn held_confirm_cannot_capture_until_a_new_press() {
        let now = Instant::now(); let mut pad = Pad::default();
        assert!(pad.poll(button(0), false, now).activity);
        for n in 1..10 { assert_eq!(pad.poll(button(0), true, now+Duration::from_millis(n*16)).capture, None); }
        pad.poll(Snapshot::default(), true, now+Duration::from_millis(160));
        assert_eq!(pad.poll(button(0), true, now+Duration::from_millis(176)).capture, Some("South"));
    }
    #[test]
    fn capture_is_single_and_blocks_bounce_and_menu_until_neutral_for_20ms() {
        let now = Instant::now(); let mut pad = Pad::default();
        assert_eq!(pad.poll(button(0), true, now).capture, Some("South"));
        assert!(pad.blocked());
        pad.poll(Snapshot::default(), false, now+Duration::from_millis(16));
        assert!(pad.blocked());
        assert_eq!(pad.poll(button(0), true, now+Duration::from_millis(25)).capture, None);
        pad.poll(Snapshot::default(), false, now+Duration::from_millis(32));
        pad.poll(Snapshot::default(), false, now+Duration::from_millis(51));
        assert!(pad.blocked());
        pad.poll(Snapshot::default(), false, now+Duration::from_millis(52));
        assert!(!pad.blocked());
        assert_eq!(pad.poll(button(0), true, now+Duration::from_millis(60)).capture, Some("South"));
    }
    #[test]
    fn all_xbox_buttons_and_stick_directions_have_semantic_tokens() {
        for (i, token) in TOKENS.iter().enumerate() {
            assert!(button(i).pressed(token));
            assert!(!button(i).pressed("Unknown"));
        }
    }
    #[test]
    fn analog_hysteresis_rejects_drift_and_nan_without_sticking_triggers() {
        let neutral = Snapshot::sample([0.;17],[0.1,-0.1,f32::NAN,0.],0.5,Snapshot::default());
        assert_eq!(neutral,Snapshot::default());
        let pushed = Snapshot::sample([0.;17],[0.6,0.,0.,0.],0.5,neutral);
        assert!(pushed.pressed("LeftStickRight"));
        let held = Snapshot::sample([0.;17],[0.45,0.,0.,0.],0.5,pushed);
        assert!(held.pressed("LeftStickRight"));
        let released = Snapshot::sample([0.;17],[0.3,0.,0.,0.],0.5,held);
        assert_eq!(released,neutral);
        let mut triggers=[0.;17];triggers[6]=0.7;triggers[7]=0.9;
        let down=Snapshot::sample(triggers,[0.;4],0.5,neutral);
        assert!(down.pressed("LeftTrigger2") && down.pressed("RightTrigger2"));
        assert_eq!(Snapshot::sample([0.;17],[0.;4],0.5,down),neutral);
    }
}
