//! Shared input bindings for the launcher (which edits them) and the play
//! runtime (which honors them). Plain `key = value` text under the user
//! config dir. Native Windows controller support is isolated in directinput.

pub mod storage;

use std::collections::HashSet;
use std::path::PathBuf;
pub mod directinput;
mod combo;
pub use combo::*;

/// The ten pad inputs, in KEYINPUT bit order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
    R,
    L,
}

impl Button {
    pub const ALL: [Button; 10] = [
        Button::A,
        Button::B,
        Button::Select,
        Button::Start,
        Button::Right,
        Button::Left,
        Button::Up,
        Button::Down,
        Button::R,
        Button::L,
    ];

    /// KEYINPUT bit (active-low in hardware; callers handle polarity).
    pub fn bit(self) -> u16 {
        1 << self.index()
    }

    pub fn index(self) -> usize {
        match self {
            Button::A => 0,
            Button::B => 1,
            Button::Select => 2,
            Button::Start => 3,
            Button::Right => 4,
            Button::Left => 5,
            Button::Up => 6,
            Button::Down => 7,
            Button::R => 8,
            Button::L => 9,
        }
    }

    /// Config-file key and display name.
    pub fn name(self) -> &'static str {
        match self {
            Button::A => "a",
            Button::B => "b",
            Button::Select => "select",
            Button::Start => "start",
            Button::Right => "right",
            Button::Left => "left",
            Button::Up => "up",
            Button::Down => "down",
            Button::R => "r",
            Button::L => "l",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Device {
    Keyboard,
    Gamepad,
    MouseKeyboard,
}

/// Routes button transitions for remapping. The event queue owns binding
/// capture; after a binding is accepted, all pad input is locked out until the
/// physical controls are released so the same press cannot activate the menu.
#[derive(Default)]
pub struct PadGesture {
    held: HashSet<String>,
    capture_armed: bool,
    locked_until_release: bool,
}

impl PadGesture {
    pub fn arm_capture(&mut self, armed: bool) {
        self.capture_armed = armed;
    }

    /// Feed a normalized down/up transition. Repeated event variants for one
    /// held control are ignored; a release followed by a press is a new edge.
    pub fn event(&mut self, token: &str, pressed: bool) -> Option<String> {
        if pressed {
            if !self.held.insert(token.to_owned()) {
                return None;
            }
            if self.capture_armed && !self.locked_until_release {
                self.capture_armed = false;
                self.locked_until_release = true;
                return Some(token.to_owned());
            }
        } else {
            self.held.remove(token);
            if self.held.is_empty() {
                self.locked_until_release = false;
            }
        }
        None
    }

    /// Polling fallback for APIs without transition events. It feeds only the
    /// differences from the previous full snapshot through the same router.
    pub fn snapshot<I, S>(&mut self, current: I) -> Option<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let ordered: Vec<String> = current
            .into_iter()
            .map(|token| token.as_ref().to_owned())
            .collect();
        let next: HashSet<String> = ordered.iter().cloned().collect();
        let released: Vec<_> = self.held.difference(&next).cloned().collect();
        let mut seen = HashSet::new();
        let pressed: Vec<_> = ordered
            .into_iter()
            .filter(|token| !self.held.contains(token) && seen.insert(token.clone()))
            .collect();
        for token in released {
            self.event(&token, false);
        }
        let mut captured = None;
        for token in pressed {
            if captured.is_none() {
                captured = self.event(&token, true);
            } else {
                self.event(&token, true);
            }
        }
        captured
    }

    pub fn blocked(&self) -> bool {
        self.locked_until_release
    }

    pub fn disconnect(&mut self) {
        self.held.clear();
        self.capture_armed = false;
        self.locked_until_release = false;
    }
}

#[cfg(test)]
mod pad_gesture_tests {
    use super::PadGesture;

    #[test]
    fn xbox_a_remap_press_is_consumed_and_cannot_confirm_menu() {
        let mut pad = PadGesture::default();

        // A confirms the remap row. Capture starts while this same physical
        // press is still down; duplicate backend events must not assign it.
        assert_eq!(pad.event("South", true), None);
        pad.arm_capture(true);
        assert_eq!(pad.event("South", true), None);
        assert_eq!(pad.event("South", true), None);
        assert!(pad.blocked() == false);

        assert_eq!(pad.event("South", false), None);
        assert_eq!(pad.event("South", true), Some("South".into()));
        assert_eq!(pad.event("South", true), None);
        assert!(pad.blocked());

        // The captured A cannot also be routed as a menu-confirm edge.
        assert_eq!(pad.event("South", true), None);
        assert!(pad.blocked());
        assert_eq!(pad.event("South", false), None);
        assert!(!pad.blocked());
    }

    #[test]
    fn second_control_must_be_released_before_capture_lock_clears() {
        let mut pad = PadGesture::default();
        pad.arm_capture(true);
        assert_eq!(pad.event("South", true), Some("South".into()));
        assert_eq!(pad.event("East", true), None);
        assert_eq!(pad.event("South", false), None);
        assert!(pad.blocked());
        assert_eq!(pad.event("East", false), None);
        assert!(!pad.blocked());
    }
}

/// The UI presents Xbox labels for all gamepads; bindings remain stored in
/// gilrs' canonical positions and can still be remapped on other devices.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GamepadLayout {
    #[default]
    Xbox,
}

impl GamepadLayout {
    pub fn detect(_name: &str) -> Self {
        Self::Xbox
    }

    pub fn detect_with_vendor(_name: &str, _vendor_id: Option<u16>) -> Self {
        Self::Xbox
    }

    pub fn name(self) -> &'static str {
        "XBOX"
    }

    pub fn token(self) -> &'static str {
        "xbox"
    }

    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "xbox" | "nintendo" | "playstation" | "ps" => Some(Self::Xbox),
            _ => None,
        }
    }

    /// Translate a canonical binding into the Xbox label used by the UI.
    pub fn label(self, token: &str) -> String {
        let value = match token {
            "South" => "A",
            "East" => "B",
            "West" => "X",
            "North" => "Y",
            "LeftTrigger" => "LB",
            "RightTrigger" => "RB",
            "LeftTrigger2" => "LT",
            "RightTrigger2" => "RT",
            "Select" => "BACK",
            "Start" => "START",
            "Mode" => "GUIDE",
            "LeftThumb" => "LS CLICK",
            "RightThumb" => "RS CLICK",
            "DPadUp" => "DPAD UP",
            "DPadDown" => "DPAD DOWN",
            "DPadLeft" => "DPAD LEFT",
            "DPadRight" => "DPAD RIGHT",
            "LeftStickUp" => "LS UP",
            "LeftStickDown" => "LS DOWN",
            "LeftStickLeft" => "LS LEFT",
            "LeftStickRight" => "LS RIGHT",
            "RightStickUp" => "RS UP",
            "RightStickDown" => "RS DOWN",
            "RightStickLeft" => "RS LEFT",
            "RightStickRight" => "RS RIGHT",
            _ => return token.to_uppercase(),
        };
        value.to_string()
    }
}

/// Which analog/digital source drives the GBA d-pad. `Both` (the
/// historical behavior) lets the left stick AND the physical d-pad both
/// move the player; `Dpad` ignores the sticks entirely.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DpadSource {
    LeftStick,
    RightStick,
    Dpad,
    Both,
}

impl DpadSource {
    /// Does this source read the left analog stick?
    pub fn uses_left_stick(self) -> bool {
        matches!(self, DpadSource::LeftStick | DpadSource::Both)
    }

    /// Does this source read the right analog stick?
    pub fn uses_right_stick(self) -> bool {
        matches!(self, DpadSource::RightStick)
    }

    fn token(self) -> &'static str {
        match self {
            DpadSource::LeftStick => "leftstick",
            DpadSource::RightStick => "rightstick",
            DpadSource::Dpad => "dpad",
            DpadSource::Both => "both",
        }
    }

    fn from_token(s: &str) -> Option<Self> {
        Some(match s {
            "leftstick" => DpadSource::LeftStick,
            "rightstick" => DpadSource::RightStick,
            "dpad" => DpadSource::Dpad,
            "both" => DpadSource::Both,
            _ => return None,
        })
    }
}

/// Default deadzone for stick-driven directions, and the range the
/// launcher clamps the user's choice into. Below ~0.05 stick drift leaks
/// through; above ~0.95 the stick is effectively unusable.
pub const DEADZONE_DEFAULT: f32 = 0.5;
pub const DEADZONE_MIN: f32 = 0.05;
pub const DEADZONE_MAX: f32 = 0.95;

/// A stick direction usable as a binding source for any of the ten pad
/// inputs (e.g. binding R to "RightStickUp"). The play runtime resolves
/// these against the live axis values past the configured deadzone; the
/// launcher recognizes an axis push as a bindable source during capture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StickToken {
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
    RightStickUp,
    RightStickDown,
    RightStickLeft,
    RightStickRight,
}

impl StickToken {
    pub const ALL: [StickToken; 8] = [
        StickToken::LeftStickUp,
        StickToken::LeftStickDown,
        StickToken::LeftStickLeft,
        StickToken::LeftStickRight,
        StickToken::RightStickUp,
        StickToken::RightStickDown,
        StickToken::RightStickLeft,
        StickToken::RightStickRight,
    ];

    /// The canonical token stored in the config and shown in the UI.
    pub fn name(self) -> &'static str {
        match self {
            StickToken::LeftStickUp => "LeftStickUp",
            StickToken::LeftStickDown => "LeftStickDown",
            StickToken::LeftStickLeft => "LeftStickLeft",
            StickToken::LeftStickRight => "LeftStickRight",
            StickToken::RightStickUp => "RightStickUp",
            StickToken::RightStickDown => "RightStickDown",
            StickToken::RightStickLeft => "RightStickLeft",
            StickToken::RightStickRight => "RightStickRight",
        }
    }

    /// Parse a stored token back into a stick direction.
    pub fn by_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// `true` for the left stick, `false` for the right.
    pub fn left(self) -> bool {
        matches!(
            self,
            StickToken::LeftStickUp
                | StickToken::LeftStickDown
                | StickToken::LeftStickLeft
                | StickToken::LeftStickRight
        )
    }

    /// The axis component this token reads (`(horizontal, positive)`):
    /// horizontal = X axis, else Y axis; positive = push toward +value.
    /// Y is reported stick-up-positive by gilrs, matching this mapping.
    pub fn axis(self) -> (bool, bool) {
        match self {
            StickToken::LeftStickRight | StickToken::RightStickRight => (true, true),
            StickToken::LeftStickLeft | StickToken::RightStickLeft => (true, false),
            StickToken::LeftStickUp | StickToken::RightStickUp => (false, true),
            StickToken::LeftStickDown | StickToken::RightStickDown => (false, false),
        }
    }

    /// The stick token that a `DpadSource` maps onto a given direction
    /// (`(horizontal, positive)` axis geometry), or `None` when the source
    /// reads no stick (`Dpad`). Lets both frontends derive which stick
    /// direction lights a d-pad button from one place.
    pub fn dir(source: DpadSource, horizontal: bool, positive: bool) -> Option<StickToken> {
        let left = match source {
            DpadSource::LeftStick | DpadSource::Both => true,
            DpadSource::RightStick => false,
            DpadSource::Dpad => return None,
        };
        Some(match (left, horizontal, positive) {
            (true, true, true) => StickToken::LeftStickRight,
            (true, true, false) => StickToken::LeftStickLeft,
            (true, false, true) => StickToken::LeftStickUp,
            (true, false, false) => StickToken::LeftStickDown,
            (false, true, true) => StickToken::RightStickRight,
            (false, true, false) => StickToken::RightStickLeft,
            (false, false, true) => StickToken::RightStickUp,
            (false, false, false) => StickToken::RightStickDown,
        })
    }
}

/// Key names are a canonical set understood by both frontends (egui on the
/// launcher side, winit on the play side): letters `A`..`Z`, digits
/// `0`..`9`, `Up` `Down` `Left` `Right`, `Enter`, `Space`, `Tab`,
/// `Backspace`, `LeftShift`, `RightShift`. Pad names are gilrs button
/// names (`South`, `East`, `DPadUp`, `LeftTrigger`, ...).
///
/// Not `Eq` — `stick_deadzone` is an `f32`. Equality is `PartialEq`,
/// which is all the tests and config diffing need.
#[derive(Clone, PartialEq, Debug)]
pub struct InputConfig {
    pub menu_key: String,
    pub menu_pad: String,
    pub device: Device,
    /// Preferred pad by name; empty = first connected.
    pub gamepad_name: String,
    /// Last detected physical button family, including vendor-ID fallback.
    pub gamepad_layout: GamepadLayout,
    pub keys: [String; 10],
    pub mouse_keys: [String; 10],
    pub mouse_keyboard: bool,
    /// Keyboard key that fast-forwards while held (4x, or the menu's speed).
    pub ff_key: String,
    pub ff_toggle_key: String,
    pub ff_pad: String,
    pub ff_toggle_pad: String,
    pub zoom_in_key: String,
    pub zoom_out_key: String,
    pub zoom_in_pad: String,
    pub zoom_out_pad: String,
    /// Pad bindings: a gilrs button name (`South`, `DPadUp`, ...) or a
    /// stick-direction token (`LeftStickUp`, ...). The play runtime
    /// resolves stick tokens against the live axes past `stick_deadzone`.
    pub pads: [String; 10],
    /// Which source(s) move the GBA d-pad on a gamepad.
    pub dpad_source: DpadSource,
    /// Threshold a stick axis must pass to register as a direction.
    pub stick_deadzone: f32,
}

/// Holding the momentary key inverts the latched fast-forward state.
pub fn fast_forward_speed(multiplier: u32, latched: bool, held: bool) -> u32 {
    if latched ^ held {
        match multiplier {
            2 | 4 | 8 => multiplier,
            _ => 4,
        }
    } else {
        1
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            menu_key: "Escape".into(),
            menu_pad: "Select+Start".into(),
            device: Device::Keyboard,
            mouse_keyboard: false,
            mouse_keys: ["Z", "X", "RightShift", "Enter", "D", "A", "W", "S", "E", "Q"].map(str::to_owned),
            gamepad_name: String::new(),
            gamepad_layout: GamepadLayout::Xbox,
            ff_key: s("Tab"),
            ff_toggle_key: s("T"),
            ff_pad: s("LeftThumb"),
            ff_toggle_pad: s("RightThumb"),
            zoom_in_key: s("Equal"),
            zoom_out_key: s("Minus"),
            zoom_in_pad: s("RightTrigger"),
            zoom_out_pad: s("LeftTrigger"),
            // the historical play-command mapping
            keys: [
                s("Z"),
                s("X"),
                s("RightShift"),
                s("Enter"),
                s("Right"),
                s("Left"),
                s("Up"),
                s("Down"),
                s("S"),
                s("A"),
            ],
            pads: [
                s("South"),
                s("East"),
                s("Select"),
                s("Start"),
                s("DPadRight"),
                s("DPadLeft"),
                s("DPadUp"),
                s("DPadDown"),
                s("RightTrigger2"),
                s("LeftTrigger2"),
            ],
            // Default matches the historical hardcoded play behavior:
            // left stick AND the physical d-pad bindings both steer.
            dpad_source: DpadSource::Both,
            stick_deadzone: DEADZONE_DEFAULT,
        }
    }
}

impl InputConfig {
    pub fn active_keys(&self) -> &[String; 10] {if self.mouse_keyboard {&self.mouse_keys} else {&self.keys}}
    pub fn active_keys_mut(&mut self) -> &mut [String; 10] {if self.mouse_keyboard {&mut self.mouse_keys} else {&mut self.keys}}
    pub fn parse(text: &str) -> Self {
        let mut cfg = Self::default();
        let mut toggle_explicit = false;
        let mut gamepad_layout_version = 0;
        let mut gamepad_layout_explicit = false;
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "gamepad_layout_version" => gamepad_layout_version = v.parse::<u32>().unwrap_or(0),
                "device" => {
                    cfg.device = if v.eq_ignore_ascii_case("gamepad") {
                        Device::Gamepad
                    } else if v.eq_ignore_ascii_case("mouse_keyboard") {
                        cfg.mouse_keyboard = true;
                        Device::MouseKeyboard
                    } else {
                        Device::Keyboard
                    }
                }
                "mouse_keyboard" => cfg.mouse_keyboard = v == "true",
                "gamepad_name" => cfg.gamepad_name = v.to_string(),
                "gamepad_layout" => {
                    if let Some(layout) = GamepadLayout::from_token(v) {
                        cfg.gamepad_layout = layout;
                        gamepad_layout_explicit = true;
                    }
                }
                "dpad_source" => {
                    if let Some(d) = DpadSource::from_token(&v.to_ascii_lowercase()) {
                        cfg.dpad_source = d;
                    }
                }
                "stick_deadzone" => {
                    if let Ok(z) = v.parse::<f32>() {
                        if z.is_finite() {
                            cfg.stick_deadzone = z.clamp(DEADZONE_MIN, DEADZONE_MAX);
                        }
                    }
                }
                "key.ff" => cfg.ff_key = v.to_string(),
                "key.menu" => { if let Some(combo) = normalize_combo(v) { cfg.menu_key = combo; } }
                "pad.menu" => { if let Some(combo) = normalize_combo(v) { cfg.menu_pad = combo; } }
                "key.ff_toggle" => {
                    cfg.ff_toggle_key = v.to_string();
                    toggle_explicit = true;
                }
                "key.zoom_in" => cfg.zoom_in_key = v.to_string(),
                "key.zoom_out" => cfg.zoom_out_key = v.to_string(),
                "pad.zoom_in" => cfg.zoom_in_pad = v.to_string(),
                "pad.ff" => cfg.ff_pad = v.to_string(),
                "pad.ff_toggle" => cfg.ff_toggle_pad = v.to_string(),
                "pad.zoom_out" => cfg.zoom_out_pad = v.to_string(),
                _ => {
                    if let Some(name) = k.strip_prefix("key.") {
                        if let Some(b) = button_by_name(name) {
                            cfg.keys[b.index()] = v.to_string();
                        }
                    } else if let Some(name) = k.strip_prefix("mouse_key.") {
                        if let Some(b) = button_by_name(name) { cfg.mouse_keys[b.index()] = v.to_owned(); }
                    } else if let Some(name) = k.strip_prefix("pad.") {
                        if let Some(b) = button_by_name(name) {
                            cfg.pads[b.index()] = v.to_string();
                        }
                    }
                }
            }
        }
        if !gamepad_layout_explicit {
            cfg.gamepad_layout = GamepadLayout::detect(&cfg.gamepad_name);
        }
        // Old profiles assigned GBA L/R to the same bumpers as zoom.
        // Migrate only that exact layout; versioned profiles may deliberately
        // reuse buttons, and DirectInput mappings have device-specific order.
        if gamepad_layout_version < 2
            && !cfg.gamepad_name.starts_with("DirectInput:")
            && cfg.pads[Button::L.index()] == "LeftTrigger"
            && cfg.pads[Button::R.index()] == "RightTrigger"
            && cfg.zoom_out_pad == "LeftTrigger"
            && cfg.zoom_in_pad == "RightTrigger"
        {
            cfg.pads[Button::L.index()] = "LeftTrigger2".into();
            cfg.pads[Button::R.index()] = "RightTrigger2".into();
        }
        if !toggle_explicit {
            if let Some(key) = [
                "T", "F", "G", "H", "J", "K", "L", "U", "I", "O", "P", "0", "1", "2", "3", "4",
                "5", "6", "7", "8", "9",
            ]
            .into_iter()
            .find(|key| {
                !cfg.keys.iter().any(|bound| bound == key)
                    && cfg.ff_key != *key
                    && cfg.zoom_in_key != *key
                    && cfg.zoom_out_key != *key
            }) {
                cfg.ff_toggle_key = key.into();
            }
        }
        cfg
    }

    pub fn serialize(&self) -> String {
        let mut out = String::from("# gba-recomp input bindings\n");
        out.push_str("gamepad_layout_version = 2\n");
        out.push_str(&format!(
            "device = {}\n",
            match self.device {
                Device::Keyboard => "keyboard",
                Device::Gamepad => "gamepad",
                Device::MouseKeyboard => "mouse_keyboard",
            }
        ));
        out.push_str(&format!("mouse_keyboard = {}\n", self.mouse_keyboard));
        for b in Button::ALL {out.push_str(&format!("mouse_key.{} = {}\n",b.name(),self.mouse_keys[b.index()]));}
        out.push_str(&format!("gamepad_name = {}\n", self.gamepad_name));
        out.push_str(&format!(
            "gamepad_layout = {}\n",
            self.gamepad_layout.token()
        ));
        out.push_str(&format!("dpad_source = {}\n", self.dpad_source.token()));
        out.push_str(&format!("stick_deadzone = {:.2}\n", self.stick_deadzone));
        for b in Button::ALL {
            out.push_str(&format!("key.{} = {}\n", b.name(), self.keys[b.index()]));
        }
        out.push_str(&format!("key.ff = {}\n", self.ff_key));
        out.push_str(&format!("key.menu = {}\n", self.menu_key));
        out.push_str(&format!("pad.menu = {}\n", self.menu_pad));
        out.push_str(&format!("key.ff_toggle = {}\n", self.ff_toggle_key));
        out.push_str(&format!("pad.ff = {}\n", self.ff_pad));
        out.push_str(&format!("pad.ff_toggle = {}\n", self.ff_toggle_pad));
        out.push_str(&format!("key.zoom_in = {}\n", self.zoom_in_key));
        out.push_str(&format!("key.zoom_out = {}\n", self.zoom_out_key));
        for b in Button::ALL {
            out.push_str(&format!("pad.{} = {}\n", b.name(), self.pads[b.index()]));
        }
        out.push_str(&format!("pad.zoom_in = {}\n", self.zoom_in_pad));
        out.push_str(&format!("pad.zoom_out = {}\n", self.zoom_out_pad));
        out
    }

    /// Load from the default path; missing or unreadable = defaults.
    pub fn load() -> Self {
        default_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    /// Persist to the default path (creating the directory).
    pub fn save(&self) -> Result<(), String> {
        let path = default_path().ok_or("no config directory")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, self.serialize()).map_err(|e| e.to_string())
    }
}

pub fn button_by_name(name: &str) -> Option<Button> {
    Button::ALL.into_iter().find(|b| b.name() == name)
}

/// A resolved pad binding: either a physical button (by gilrs name) or a
/// stick direction. The play runtime maps the button name through gilrs
/// and the stick token through the live axes; keeping the split here lets
/// the dependency-free model own the token vocabulary.
pub enum PadBinding<'a> {
    Button(&'a str),
    Stick(StickToken),
}

/// Classify a stored pad binding. A recognized stick token resolves to
/// `Stick`; everything else is handed back as a button name for gilrs to
/// interpret (unknown names there simply never fire).
pub fn pad_binding(name: &str) -> PadBinding<'_> {
    match StickToken::by_name(name) {
        Some(t) => PadBinding::Stick(t),
        None => PadBinding::Button(name),
    }
}

/// Audio/video settings, launcher-edited, honored by the play runtime.
/// Lives in its own file (`av.cfg`) so input and A/V concerns stay
/// independently versionable.
///
/// Video fields are stored as plain strings here (this crate stays
/// dependency-free); the `screen` crate owns the vocabulary and both
/// frontends parse through it. "auto" means "the screen model's default".
#[derive(Clone, PartialEq, Debug)]
pub struct AvConfig {
    /// The premium audio path (final name TBD): band-limited sinc
    /// resampling, DC blocking, and buffer rate control instead of the
    /// hardware-faithful nearest-neighbor output. Off until the feature
    /// is complete enough to name and ship on by default.
    pub audio_enhanced: bool,
    /// Master output volume, 0..1 (1 = full).
    pub audio_volume: f32,
    /// Persistent user mute; retains volume and does not stop consumption.
    pub audio_mute: bool,
    /// Persistent fast-forward multiplier: 2, 4 or 8. Activation is session-only.
    pub ff_speed: u32,
    /// silent, fast (accelerated audio), or normal (buffered 1x audio).
    pub ff_audio: String,
    /// Output brightness scale applied to the color table: 1.0 = unchanged.
    pub video_brightness: f32,
    /// Screen simulation model: raw | unlit | frontlit | backlit | classic.
    pub screen: String,
    /// Viewing-darkness knob for the reflective screens: "auto" or 0..1.
    pub screen_darken: String,
    /// Temporal response: off | simple | smart | persistence.
    pub response: String,
    /// Persistence carryover: "auto" or 0..0.9.
    pub response_keep: String,
    /// Pixel-grid strength: "auto" (per-screen default) or 0..1.
    pub grid: String,
    /// Base scaler under the grid: off | nearest | sharp.
    pub scaling: String,
    /// Output colorspace: auto | srgb | display-p3.
    pub display_gamut: String,
    /// Presentation vsync. Off (default) presents immediately
    /// (AutoNoVsync); on trades possible tearing for compositor pacing.
    /// Emulation speed is owned by the audio clock either way.
    pub video_vsync: bool,
    /// Borderless fullscreen at startup; shares the live menu/F11 state.
    pub video_fullscreen: bool,
    /// FPS overlay: off | internal | external | both.
    pub video_fps: String,
    /// Output aspect ratio: auto | 3:2 (original) | 16:10 | 16:9 | 20:9 | 21:9 | 32:9.
    /// Sets how many extra columns the PPU renders; applied at launch.
    pub video_aspect: String,
    pub bubble_size: f32,
    pub bubble_length: f32,
}

impl Default for AvConfig {
    fn default() -> Self {
        Self {
            audio_enhanced: false,
            audio_volume: 1.0,
            audio_mute: false,
            ff_speed: 4,
            ff_audio: "silent".into(),
            video_brightness: 1.0,
            screen: "raw".into(),
            screen_darken: "auto".into(),
            response: "smart".into(),
            response_keep: "auto".into(),
            grid: "auto".into(),
            scaling: "nearest".into(),
            display_gamut: "auto".into(),
            video_vsync: false,
            video_fullscreen: false,
            video_fps: "off".into(),
            video_aspect: "16:9".into(),
            bubble_size: 1.,
            bubble_length: 1.,
        }
    }
}

impl AvConfig {
    pub fn parse(text: &str) -> Self {
        let mut cfg = Self::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "audio.enhanced" => cfg.audio_enhanced = v.eq_ignore_ascii_case("true"),
                "audio.mute" => cfg.audio_mute = v.eq_ignore_ascii_case("true"),
                "ff.speed" => {
                    if let Ok(speed @ (2 | 4 | 8)) = v.parse::<u32>() {
                        cfg.ff_speed = speed;
                    }
                }
                "ff.audio" if matches!(v, "silent" | "fast" | "normal") => {
                    cfg.ff_audio = v.to_string()
                }
                "audio.volume" => {
                    if let Ok(x) = v.parse::<f32>() {
                        if x.is_finite() {
                            cfg.audio_volume = x.clamp(0.0, 1.0);
                        }
                    }
                }
                "video.brightness" => {
                    if let Ok(x) = v.parse::<f32>() {
                        if x.is_finite() {
                            cfg.video_brightness = x.clamp(0.0, 2.0);
                        }
                    }
                }
                "video.screen" => cfg.screen = v.to_ascii_lowercase(),
                "video.darken" => cfg.screen_darken = v.to_ascii_lowercase(),
                "video.response" => cfg.response = v.to_ascii_lowercase(),
                "video.response_keep" => cfg.response_keep = v.to_ascii_lowercase(),
                "video.grid" => cfg.grid = v.to_ascii_lowercase(),
                "video.scaling" => cfg.scaling = v.to_ascii_lowercase(),
                "video.gamut" => cfg.display_gamut = v.to_ascii_lowercase(),
                "video.vsync" => cfg.video_vsync = v.eq_ignore_ascii_case("true"),
                "video.fullscreen" => cfg.video_fullscreen = v.eq_ignore_ascii_case("true"),
                "video.fps" => {
                    let mode = v.to_ascii_lowercase();
                    if matches!(mode.as_str(), "off" | "internal" | "external" | "both") {
                        cfg.video_fps = mode;
                    }
                }
                "bubble.size" | "bubble.length" => {
                    if let Ok(x) = v.parse::<f32>() { if x.is_finite() {
                        if k.trim() == "bubble.size" { cfg.bubble_size=x.clamp(0.5,2.); }
                        else { cfg.bubble_length=x.clamp(1.,2.); }
                    }}
                }
                "video.aspect" => cfg.video_aspect = v.to_ascii_lowercase(),
                _ => {}
            }
        }
        cfg
    }

    pub fn serialize(&self) -> String {
        format!(
            "# gba-recomp a/v settings\n\
             audio.enhanced = {}\n\
             audio.volume = {}\n\
             audio.mute = {}\n\
             ff.speed = {}\n\
             ff.audio = {}\n\
             video.brightness = {}\n\
             video.screen = {}\n\
             video.darken = {}\n\
             video.response = {}\n\
             video.response_keep = {}\n\
             video.grid = {}\n\
             video.scaling = {}\n\
             video.gamut = {}\n\
             video.vsync = {}\n\
             video.fullscreen = {}\n\
             video.fps = {}\n\
             video.aspect = {}\n\
             bubble.size = {}\n\
             bubble.length = {}\n",
            self.audio_enhanced,
            self.audio_volume,
            self.audio_mute,
            self.ff_speed,
            self.ff_audio,
            self.video_brightness,
            self.screen,
            self.screen_darken,
            self.response,
            self.response_keep,
            self.grid,
            self.scaling,
            self.display_gamut,
            self.video_vsync,
            self.video_fullscreen,
            self.video_fps,
            self.video_aspect,
            self.bubble_size,
            self.bubble_length,
        )
    }

    /// "auto" (or anything unparseable) -> None; otherwise the number.
    pub fn knob(value: &str) -> Option<f32> {
        value.parse::<f32>().ok().filter(|v| v.is_finite())
    }

    /// Load from the default path; missing or unreadable = defaults.
    pub fn load() -> Self {
        av_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    /// Persist to the default path (creating the directory).
    pub fn save(&self) -> Result<(), String> {
        let path = av_path().ok_or("no config directory")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, self.serialize()).map_err(|e| e.to_string())
    }
}

/// `<config dir>/gba-recomp/av.cfg`.
pub fn av_path() -> Option<PathBuf> {
    Some(default_path()?.with_file_name("av.cfg"))
}

/// `<config dir>/gba-recomp/input.cfg`, resolved per-platform without
/// pulling in a dependency.
pub fn default_path() -> Option<PathBuf> {
    Some(config_root()?.join("input.cfg"))
}

/// `<config dir>/gba-recomp`, the per-user config/state directory,
/// resolved per-platform without pulling in a dependency.
pub fn config_root() -> Option<PathBuf> {
    if let Some(root) = storage::portable_root() {
        return Some(root.join("Data").join("config"));
    }
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?)
    } else {
        match std::env::var_os("XDG_CONFIG_HOME") {
            Some(x) if !x.is_empty() => PathBuf::from(x),
            _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
        }
    };
    Some(base.join("FFTARecomp"))
}

// ── BIOS image (real-BIOS boot) ─────────────────────────────────────
//
// The product boots the real BIOS: the launcher collects the user's
// dump once (first-launch setup), installs it where the whole release
// can find it, and the play runtime recompiles it alongside each
// cartridge. No image installed = the runtime falls back to BIOS HLE.

/// Canonical on-disk name for the installed BIOS image. Lowercase and
/// used verbatim everywhere — Linux filesystems are case-sensitive, so
/// resolution and install must never disagree on case.
pub const BIOS_FILE_NAME: &str = "gba_bios.bin";

/// Expected image size: the BIOS mask ROM is exactly 16 KB.
pub const BIOS_SIZE: usize = 0x4000;

/// SHA-256 of the canonical BIOS dump. Other images (homebrew
/// replacements, bad dumps) are warned about loudly but still tried —
/// the user owns the choice.
pub const BIOS_SHA256: &str = "fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570";

/// Directory of the running executable — the "release directory" for
/// portable installs. The exe path is canonicalized first so symlinked
/// launches (Linux desktop entries, dev shims) resolve to the real
/// install directory rather than the symlink's.
pub fn exe_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    exe.parent().map(|d| d.to_path_buf())
}

/// Locate the installed BIOS image. Resolution order:
/// 1. `$GBA_RECOMP_BIOS` (explicit override, dev/CI),
/// 2. next to the executable (portable release directory),
/// 3. the per-user config directory (fallback installs — read-only
///    release dirs like /Applications or Program Files).
pub fn find_bios() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("GBA_RECOMP_BIOS") {
        if !v.is_empty() {
            let p = PathBuf::from(v);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    if let Some(dir) = exe_dir() {
        let p = dir.join(BIOS_FILE_NAME);
        if p.is_file() {
            return Some(p);
        }
    }
    let p = config_root()?.join(BIOS_FILE_NAME);
    p.is_file().then_some(p)
}

/// Install a user-selected BIOS image where `find_bios` will see it.
/// Prefers the release directory (portable layout: the whole product
/// travels as one folder); falls back to the per-user config directory
/// when the release directory isn't writable — macOS app translocation
/// and /Applications, Windows Program Files, system-wide Linux installs.
/// The copy is staged to a temp file and renamed so a crash mid-write
/// can never leave a torn image at the resolved name.
pub fn install_bios(src: &std::path::Path) -> Result<PathBuf, String> {
    let bytes = std::fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?;
    if bytes.len() != BIOS_SIZE {
        return Err(format!(
            "{}: expected a {BIOS_SIZE}-byte BIOS image, got {} bytes",
            src.display(),
            bytes.len()
        ));
    }
    let mut targets = Vec::new();
    if let Some(dir) = exe_dir() {
        targets.push(dir.join(BIOS_FILE_NAME));
    }
    if let Some(dir) = config_root() {
        targets.push(dir.join(BIOS_FILE_NAME));
    }
    let mut last_err = "no writable install location".to_string();
    for dest in targets {
        if let Some(dir) = dest.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Same-directory temp + rename: atomic on POSIX, and never
        // crosses a volume boundary (fs::rename can't).
        let tmp = dest.with_extension("bin.tmp");
        let write = std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, &dest));
        match write {
            Ok(()) => return Ok(dest),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                last_err = format!("{}: {e}", dest.display());
            }
        }
    }
    Err(format!("could not install BIOS image ({last_err})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_layout_uses_xbox_labels_for_every_gamepad() {
        assert_eq!(
            GamepadLayout::detect("Xbox Wireless Controller"),
            GamepadLayout::Xbox
        );
        assert_eq!(
            GamepadLayout::detect("Nintendo Switch Pro Controller"),
            GamepadLayout::Xbox
        );
        assert_eq!(GamepadLayout::detect("Joy-Con (L)"), GamepadLayout::Xbox);
        assert_eq!(
            GamepadLayout::detect_with_vendor("Wireless Gamepad", Some(0x057e)),
            GamepadLayout::Xbox
        );
        assert_eq!(
            GamepadLayout::detect_with_vendor("Wireless Gamepad", Some(0x054c)),
            GamepadLayout::Xbox
        );
        assert_eq!(GamepadLayout::detect("PS5 Controller"), GamepadLayout::Xbox);
        assert_eq!(
            GamepadLayout::detect("DualSense Wireless Controller"),
            GamepadLayout::Xbox
        );
        assert_eq!(
            GamepadLayout::detect("Sony DualShock 4"),
            GamepadLayout::Xbox
        );

        assert_eq!(GamepadLayout::Xbox.name(), "XBOX");
        assert_eq!(GamepadLayout::Xbox.label("South"), "A");
        assert_eq!(GamepadLayout::Xbox.label("East"), "B");
        assert_eq!(GamepadLayout::Xbox.label("West"), "X");
        assert_eq!(GamepadLayout::Xbox.label("North"), "Y");
        assert_eq!(GamepadLayout::Xbox.label("LeftTrigger2"), "LT");
        assert_eq!(GamepadLayout::Xbox.label("RightTrigger2"), "RT");
    }

    #[test]
    fn controller_layout_roundtrips_and_legacy_layout_names_use_xbox() {
        let cfg = InputConfig {
            gamepad_name: "Wireless Gamepad".into(),
            gamepad_layout: GamepadLayout::Xbox,
            ..InputConfig::default()
        };
        assert_eq!(InputConfig::parse(&cfg.serialize()), cfg);
        assert_eq!(
            InputConfig::parse(
                "gamepad_name = Nintendo Switch Pro Controller\ngamepad_layout = nintendo\n"
            )
            .gamepad_layout,
            GamepadLayout::Xbox
        );
        assert_eq!(
            GamepadLayout::from_token("playstation"),
            Some(GamepadLayout::Xbox)
        );
    }

    #[test]
    fn fast_forward_toggle_and_hold_are_independent_of_multiplier() {
        for speed in [2, 4, 8] {
            assert_eq!(fast_forward_speed(speed, false, false), 1);
            assert_eq!(fast_forward_speed(speed, false, true), speed);
            assert_eq!(fast_forward_speed(speed, true, false), speed);
            assert_eq!(fast_forward_speed(speed, true, true), 1);
        }
        let cfg = InputConfig::parse("key.ff = Space\nkey.ff_toggle = Y\n");
        assert_eq!(cfg.ff_key, "Space");
        assert_eq!(cfg.ff_toggle_key, "Y");
        assert_eq!(InputConfig::parse(&cfg.serialize()), cfg);
        assert_eq!(AvConfig::parse("ff.speed = 1\n").ff_speed, 4);
        let previous = InputConfig::parse("key.a = T\n");
        assert_eq!(previous.keys[Button::A.index()], "T");
        assert_eq!(previous.ff_toggle_key, "F");
    }

    #[test]
    fn roundtrip() {
        let mut cfg = InputConfig::default();
        cfg.device = Device::Gamepad;
        cfg.gamepad_name = "Pro Pad 2".into();
        cfg.keys[Button::A.index()] = "Space".into();
        cfg.pads[Button::L.index()] = "LeftTrigger2".into();
        // stick-as-button binding survives the round-trip too
        cfg.pads[Button::R.index()] = "RightStickUp".into();
        cfg.ff_pad = "West".into();
        cfg.ff_toggle_pad = "North".into();
        cfg.dpad_source = DpadSource::RightStick;
        cfg.stick_deadzone = 0.30;
        assert_eq!(InputConfig::parse(&cfg.serialize()), cfg);
    }
    #[test]
    fn mouse_keyboard_profile_is_independent_and_persistent() {
        let mut cfg=InputConfig::default();
        let keyboard=cfg.keys.clone();
        cfg.device=Device::MouseKeyboard;
        cfg.mouse_keyboard=true;
        assert_eq!(cfg.active_keys()[Button::Up.index()],"W");
        cfg.active_keys_mut()[Button::A.index()]="Space".into();
        assert_eq!(cfg.keys,keyboard);
        let restored=InputConfig::parse(&cfg.serialize());
        assert_eq!(restored,cfg);
        cfg.mouse_keyboard=false;
        assert_eq!(cfg.active_keys(),&keyboard);
    }

    #[test]
    fn legacy_bumpers_shared_with_zoom_migrate_only_once() {
        let old = "pad.l = LeftTrigger\npad.r = RightTrigger\npad.zoom_out = LeftTrigger\npad.zoom_in = RightTrigger\npad.a = West\n";
        let cfg = InputConfig::parse(old);
        assert_eq!(cfg.pads[Button::L.index()], "LeftTrigger2");
        assert_eq!(cfg.pads[Button::R.index()], "RightTrigger2");
        assert_eq!(cfg.zoom_out_pad, "LeftTrigger");
        assert_eq!(cfg.zoom_in_pad, "RightTrigger");
        assert_eq!(cfg.pads[Button::A.index()], "West");
        assert_eq!(InputConfig::parse(&cfg.serialize()), cfg);

        let deliberate = InputConfig::parse(&format!("gamepad_layout_version = 2\n{old}"));
        assert_eq!(deliberate.pads[Button::L.index()], "LeftTrigger");
        assert_eq!(deliberate.pads[Button::R.index()], "RightTrigger");
        let direct = InputConfig::parse(&format!("gamepad_name = DirectInput: Legacy Pad\n{old}"));
        assert_eq!(direct.pads[Button::L.index()], "LeftTrigger");
    }

    #[test]
    fn parse_tolerates_noise() {
        let cfg =
            InputConfig::parse("garbage\nkey.a = Q # comment\nkey.zz = NO\ndevice = GAMEPAD\n");
        assert_eq!(cfg.keys[Button::A.index()], "Q");
        assert_eq!(cfg.device, Device::Gamepad);
        assert_eq!(cfg.keys[Button::B.index()], "X"); // untouched default
    }

    #[test]
    fn serialize_spells_analog_keys_as_specified() {
        // Lock the on-disk format so launcher and play never disagree.
        let s = InputConfig::default().serialize();
        assert!(s.contains("dpad_source = both\n"), "got:\n{s}");
        assert!(s.contains("stick_deadzone = 0.50\n"), "got:\n{s}");
        let cfg = InputConfig {
            dpad_source: DpadSource::RightStick,
            stick_deadzone: 0.25,
            ..InputConfig::default()
        };
        let s = cfg.serialize();
        assert!(s.contains("dpad_source = rightstick\n"), "got:\n{s}");
        assert!(s.contains("stick_deadzone = 0.25\n"), "got:\n{s}");
    }

    #[test]
    fn new_fields_default_and_clamp() {
        // Old config files (no analog keys) load with the defaults that
        // reproduce the historical behavior.
        let old = InputConfig::parse("device = gamepad\npad.a = South\n");
        assert_eq!(old.dpad_source, DpadSource::Both);
        assert_eq!(old.stick_deadzone, DEADZONE_DEFAULT);

        // dpad_source parses case-insensitively; bad values keep default.
        assert_eq!(
            InputConfig::parse("dpad_source = LEFTSTICK\n").dpad_source,
            DpadSource::LeftStick
        );
        assert_eq!(
            InputConfig::parse("dpad_source = nonsense\n").dpad_source,
            DpadSource::Both
        );

        // deadzone clamps into the launcher's range; garbage keeps default.
        assert_eq!(
            InputConfig::parse("stick_deadzone = 0.01\n").stick_deadzone,
            DEADZONE_MIN
        );
        assert_eq!(
            InputConfig::parse("stick_deadzone = 9.0\n").stick_deadzone,
            DEADZONE_MAX
        );
        assert_eq!(
            InputConfig::parse("stick_deadzone = abc\n").stick_deadzone,
            DEADZONE_DEFAULT
        );
    }

    #[test]
    fn stick_tokens_classify_and_resolve() {
        // A stick token resolves to Stick; anything else is a button.
        assert!(matches!(
            pad_binding("LeftStickUp"),
            PadBinding::Stick(StickToken::LeftStickUp)
        ));
        assert!(matches!(pad_binding("South"), PadBinding::Button("South")));
        // axis geometry: horizontal flag + positive direction.
        assert_eq!(StickToken::LeftStickRight.axis(), (true, true));
        assert_eq!(StickToken::RightStickDown.axis(), (false, false));
        assert!(StickToken::LeftStickUp.left());
        assert!(!StickToken::RightStickUp.left());
    }

    #[test]
    fn av_roundtrip_and_default_off() {
        assert!(!AvConfig::default().audio_enhanced);
        let cfg = AvConfig {
            audio_enhanced: true,
            ..AvConfig::default()
        };
        assert_eq!(AvConfig::parse(&cfg.serialize()), cfg);
        assert_eq!(
            AvConfig::parse("junk\naudio.enhanced = TRUE # ok\n"),
            cfg,
            "unknown keys keep defaults"
        );
    }

    #[test]
    fn user_mute_is_default_off_and_roundtrips_without_losing_volume() {
        let old = AvConfig::parse("audio.volume = 0.6\naudio.enhanced = true\n");
        assert!(!old.audio_mute);
        let muted = AvConfig::parse("audio.mute = TRUE # user choice\naudio.volume = 0.6\n");
        assert!(muted.audio_mute);
        assert_eq!(muted.audio_volume, 0.6);
        assert_eq!(AvConfig::parse(&muted.serialize()), muted);
        assert!(!AvConfig::parse("audio.mute = invalid\n").audio_mute);
        assert!(!AvConfig::parse("audio.mute = false\naudio.volume = 0\n").audio_mute);
    }

    #[test]
    fn av_volume_brightness_and_ff_multiplier_roundtrip() {
        let d = AvConfig::default();
        assert_eq!(
            (d.audio_volume, d.ff_speed, d.video_brightness),
            (1.0, 4, 1.0)
        );
        let cfg = AvConfig {
            audio_volume: 0.6,
            video_brightness: 1.2,
            ..AvConfig::default()
        };
        assert_eq!(AvConfig::parse(&cfg.serialize()), cfg);
        // The multiplier is persistent; activation remains session-only.
        let ff = AvConfig {
            ff_speed: 8,
            ..AvConfig::default()
        };
        assert_eq!(AvConfig::parse(&ff.serialize()).ff_speed, 8);
        assert_eq!(AvConfig::parse("ff.speed = 8\n").ff_speed, 8);
        let wild = AvConfig::parse("audio.volume = 9\nvideo.brightness = -3\n");
        assert_eq!((wild.audio_volume, wild.video_brightness), (1.0, 0.0));
    }

    #[test]
    fn install_bios_rejects_wrong_size() {
        let dir = std::env::temp_dir();
        let src = dir.join("gba-recomp-test-not-a-bios.bin");
        std::fs::write(&src, vec![0u8; 123]).unwrap();
        let err = install_bios(&src).unwrap_err();
        assert!(err.contains("16384-byte"), "got: {err}");
        let _ = std::fs::remove_file(&src);
    }

    #[test]
    fn fullscreen_preference_roundtrips_and_old_configs_remain_windowed() {
        assert!(!AvConfig::parse("video.vsync = true\n").video_fullscreen);
        let cfg = AvConfig::parse("video.fullscreen = TRUE # borderless\n");
        assert!(cfg.video_fullscreen);
        assert_eq!(AvConfig::parse(&cfg.serialize()), cfg);
        assert!(!AvConfig::parse("video.fullscreen = invalid\n").video_fullscreen);
    }

    #[test]
    fn av_video_roundtrip() {
        let cfg = AvConfig {
            screen: "backlit".into(),
            screen_darken: "0.5".into(),
            response: "persistence".into(),
            response_keep: "0.42".into(),
            grid: "0".into(),
            scaling: "sharp".into(),
            display_gamut: "display-p3".into(),
            ..AvConfig::default()
        };
        assert_eq!(AvConfig::parse(&cfg.serialize()), cfg);
        // Old config files (audio-only) parse to video defaults.
        let old = AvConfig::parse("audio.enhanced = true\n");
        assert_eq!(old.screen, "raw");
        assert_eq!(old.response, "smart");
        // Knob parsing: auto and garbage are None, numbers are Some.
        assert_eq!(AvConfig::knob("auto"), None);
        assert_eq!(AvConfig::knob("0.5"), Some(0.5));
        assert_eq!(AvConfig::knob("nan"), None);
    }

    #[test]
    fn fps_modes_persist_and_old_configs_keep_the_overlay_off() {
        assert_eq!(AvConfig::parse("video.vsync = true\n").video_fps, "off");
        assert_eq!(AvConfig::parse("video.fps = invalid\n").video_fps, "off");
        for mode in ["off", "internal", "external", "both"] {
            let cfg = AvConfig::parse(&format!("video.fps = {}\n", mode.to_uppercase()));
            assert_eq!(cfg.video_fps, mode);
            assert_eq!(AvConfig::parse(&cfg.serialize()), cfg);
        }
    }
}

#[cfg(test)]
mod bubble_config_tests {
    use super::*;
    #[test]
    fn bubble_controls_persist_and_reject_nonfinite_values() {
        let cfg=AvConfig::parse("bubble.size = 1.3\nbubble.length = 1.7\n");
        assert_eq!(cfg.bubble_size,1.3);
        assert_eq!(cfg.bubble_length,1.7);
        assert_eq!(AvConfig::parse(&cfg.serialize()),cfg);
        assert_eq!(AvConfig::parse("bubble.size = NaN").bubble_size,1.);
    }
}
