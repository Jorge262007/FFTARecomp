//! Menu shortcuts and release-to-confirm capture shared by all input backends.
pub const PAD_CONTROLS: &[&str] = &[
    "South",
    "East",
    "West",
    "North",
    "Start",
    "Select",
    "Mode",
    "LeftTrigger",
    "RightTrigger",
    "LeftTrigger2",
    "RightTrigger2",
    "LeftThumb",
    "RightThumb",
    "DPadUp",
    "DPadDown",
    "DPadLeft",
    "DPadRight",
    "LeftStickUp",
    "LeftStickDown",
    "LeftStickLeft",
    "LeftStickRight",
    "RightStickUp",
    "RightStickDown",
    "RightStickLeft",
    "RightStickRight",
];

pub fn normalize_combo(value: &str) -> Option<String> {
    let mut parts: Vec<_> = value.split('+').map(str::trim).collect();
    if parts.is_empty() || parts.iter().any(|s| s.is_empty()) {
        return None;
    }
    parts.sort_unstable();
    parts.dedup();
    Some(parts.join("+"))
}

/// (all members down, at least one member down). Never combines devices.
pub fn combo_state(combo: &str, mut pressed: impl FnMut(&str) -> bool) -> (bool, bool) {
    let mut all = true;
    let mut any = false;
    let mut count = 0;
    for token in combo.split('+').map(str::trim).filter(|s| !s.is_empty()) {
        let down = pressed(token);
        all &= down;
        any |= down;
        count += 1;
    }
    (count > 0 && all, any)
}

/// Keep a shortcut consumed through partial releases; re-arm only at neutral.
#[derive(Default)]
pub struct ComboLatch(bool);
impl ComboLatch {
    pub fn update(&mut self, all: bool, any: bool) -> bool {
        if !any {
            self.0 = false;
        } else if all {
            self.0 = true;
        }
        self.0
    }
}

/// Ignore the activation press, then remember the largest simultaneous set.
/// Releasing every control commits it, so a first press cannot steal a chord.
#[derive(Default)]
pub struct ComboCapture {
    source: String,
    ready: bool,
    peak: Vec<String>,
}
impl ComboCapture {
    pub fn poll(&mut self, source: &str, mut held: Vec<String>) -> Option<String> {
        if self.source != source {
            *self = Self {
                source: source.into(),
                ..Self::default()
            };
        }
        held.sort_unstable();
        held.dedup();
        if !self.ready {
            self.ready = held.is_empty();
            return None;
        }
        if held.is_empty() && !self.peak.is_empty() {
            self.ready = false;
            return Some(std::mem::take(&mut self.peak).join("+"));
        }
        if held.len() > self.peak.len() {
            self.peak = held;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controls(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn waits_for_activation_release_then_captures_overlap_not_sequential_presses() {
        let mut c = ComboCapture::default();
        assert_eq!(c.poll("pad1", controls(&["South"])), None);
        assert_eq!(c.poll("pad1", vec![]), None);
        assert_eq!(c.poll("pad1", controls(&["Start"])), None);
        assert_eq!(c.poll("pad1", controls(&["Select", "Start"])), None);
        assert_eq!(c.poll("pad1", controls(&["Select"])), None);
        assert_eq!(c.poll("pad1", vec![]).as_deref(), Some("Select+Start"));
        let mut latch = ComboLatch::default();
        assert!(!latch.update(false, true));
        assert!(latch.update(true, true));
        assert!(latch.update(false, true));
        assert!(!latch.update(false, false));
    }
    #[test]
    fn disconnect_or_controller_change_cannot_complete_another_devices_combo() {
        let mut c = ComboCapture::default();
        c.poll("pad1", vec![]);
        c.poll("pad1", controls(&["Start"]));
        assert_eq!(c.poll("pad2", controls(&["Select"])), None);
        assert_eq!(c.poll("pad2", vec![]), None);
        assert_eq!(combo_state("Start+Select", |s| s == "Start"), (false, true));
        assert_eq!(
            normalize_combo(" Start + Select + Start ").as_deref(),
            Some("Select+Start")
        );
        assert_eq!(normalize_combo("Start+"), None);
    }

    #[test]
    fn shortcuts_persist_and_legacy_configs_keep_start_back_and_escape() {
        use crate::InputConfig;
        let old = InputConfig::parse("key.a = Z\n");
        assert_eq!(old.menu_key, "Escape");
        assert_eq!(old.menu_pad, "Select+Start");
        let cfg = InputConfig::parse(
            "key.menu = M + LeftControl\npad.menu = RightTrigger + LeftTrigger\n",
        );
        assert_eq!(cfg.menu_key, "LeftControl+M");
        assert_eq!(cfg.menu_pad, "LeftTrigger+RightTrigger");
        assert_eq!(InputConfig::parse(&cfg.serialize()), cfg);
    }
}
