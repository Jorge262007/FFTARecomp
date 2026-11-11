//! Interactive `play` runtime: a winit-driven window with the GPU presenter
//! (or a software fallback), keyboard + gamepad input, the in-game menu, and
//! the audio-clock pacing loop. Extracted from `main.rs` as the one cohesive
//! "interactive shell" unit; everything it needs from the CLI/runtime core
//! (translation, machine construction, audio rings, screen config) it reaches
//! through `crate::` — this module is a child of the crate root and so may
//! use the root's items directly.
//!
//! Pacing principle (load-bearing, see `screen::present` docs): emulation
//! speed is owned by the audio clock, never the display. winit 0.30 is
//! callback-driven, so the former `while window.is_open()` body lives in
//! [`ApplicationHandler::about_to_wait`] under `ControlFlow::Poll`, and the
//! frame is presented *directly* there the instant it is produced — never
//! gated on `RedrawRequested`/vsync, which would couple game speed to the
//! panel.

#[path = "ui_motion.rs"]
mod ui_motion;
#[path = "visual_trace.rs"]
mod visual_trace;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Direct XInput polling path for Windows. This intentionally bypasses both
/// Gilrs and the HID/DirectInput adapter: each frame reads the four XInput
/// slots and compares a complete state snapshot, so one physical press has
/// exactly one rising edge regardless of duplicate event notifications.
#[cfg(windows)]
struct XInputBackend {
    _library: libloading::Library,
    get_state: unsafe extern "system" fn(u32, *mut XInputState) -> u32,
    previous: [Option<XInputState>; 4],
    capture_locked: bool,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct XInputGamepad {
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    left_x: i16,
    left_y: i16,
    right_x: i16,
    right_y: i16,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct XInputState {
    packet: u32,
    gamepad: XInputGamepad,
}

#[cfg(windows)]
impl XInputBackend {
    fn load() -> Option<Self> {
        type GetState = unsafe extern "system" fn(u32, *mut XInputState) -> u32;
        for dll in ["xinput1_4.dll", "xinput1_3.dll", "xinput9_1_0.dll"] {
            let Ok(library) = (unsafe { libloading::Library::new(dll) }) else {
                continue;
            };
            let Ok(symbol) = (unsafe { library.get::<GetState>(b"XInputGetState\0") }) else {
                continue;
            };
            let get_state = *symbol;
            return Some(Self {
                _library: library,
                get_state,
                previous: [None; 4],
                capture_locked: false,
            });
        }
        eprintln!("XInput: no XInput runtime DLL found; using fallback backend");
        None
    }

    fn poll(&self) -> [Option<XInputState>; 4] {
        std::array::from_fn(|slot| {
            let mut state = XInputState::default();
            (unsafe { (self.get_state)(slot as u32, &mut state) } == 0).then_some(state)
        })
    }
}

#[cfg(windows)]
fn xinput_tokens(state: XInputState, deadzone: f32) -> Vec<String> {
    let b = state.gamepad.buttons;
    let mut out = Vec::with_capacity(20);
    for (mask, name) in [
        (0x0001, "DPadUp"),
        (0x0002, "DPadDown"),
        (0x0004, "DPadLeft"),
        (0x0008, "DPadRight"),
        (0x0010, "Start"),
        (0x0020, "Select"),
        (0x0040, "LeftThumb"),
        (0x0080, "RightThumb"),
        (0x0100, "LeftTrigger"),
        (0x0200, "RightTrigger"),
        (0x0400, "Mode"),
        (0x1000, "South"),
        (0x2000, "East"),
        (0x4000, "West"),
        (0x8000, "North"),
    ] {
        if b & mask != 0 {
            out.push(name.to_owned());
        }
    }
    let threshold = (deadzone.clamp(0.0, 1.0) * 255.0).round() as u8;
    if state.gamepad.left_trigger > threshold {
        out.push("LeftTrigger2".into());
    }
    if state.gamepad.right_trigger > threshold {
        out.push("RightTrigger2".into());
    }
    let dz = (deadzone.clamp(0.0, 1.0) * i16::MAX as f32) as i16;
    for (v, positive, negative) in [
        (state.gamepad.left_x, "LeftStickRight", "LeftStickLeft"),
        (state.gamepad.left_y, "LeftStickUp", "LeftStickDown"),
        (state.gamepad.right_x, "RightStickRight", "RightStickLeft"),
        (state.gamepad.right_y, "RightStickUp", "RightStickDown"),
    ] {
        if v > dz {
            out.push(positive.into());
        }
        if v < -dz {
            out.push(negative.into());
        }
    }
    out
}

#[cfg(windows)]
fn xinput_pressed(state: XInputState, token: &str, deadzone: f32) -> bool {
    xinput_tokens(state, deadzone).iter().any(|t| t == token)
}

#[cfg(windows)]
fn xinput_new_press(
    previous: Option<XInputState>,
    current: XInputState,
    deadzone: f32,
) -> Option<String> {
    xinput_tokens(current, deadzone)
        .into_iter()
        .find(|token| !previous.is_some_and(|old| xinput_pressed(old, token, deadzone)))
}

/// Reject duplicate menu edges from the same physical control inside 20 ms.
/// Different controls are never delayed. The explicit timestamp keeps the
/// behavior deterministic in tests without sleeping.
#[derive(Default)]
struct MenuTapFilter(HashMap<String, Instant>);

impl MenuTapFilter {
    fn accept_at(&mut self, control: &str, now: Instant) -> bool {
        if self
            .0
            .get(control)
            .is_some_and(|last| now.saturating_duration_since(*last) < Duration::from_millis(20))
        {
            return false;
        }
        self.0.insert(control.to_owned(), now);
        true
    }
}

use gba_core::{Bus as _, Machine};

/// Current render width (240 + 2 * extra columns). The core renders only
/// what the current zoom shows; the visible part is `PlayApp::view`.
fn wide_w() -> usize {
    gba_core::mem::render_width()
}

/// Current render height (160 + 2 * extra rows), see `wide_w`.
fn wide_h() -> usize {
    gba_core::mem::render_height()
}

/// Match Android's readback when the scene has no expanded terrain rows.
/// Such frames contain only 160 newly rendered rows at the buffer's start;
/// the rest may still contain objects from a previous scene or zoom level.
fn display_frame<'a>(bus: &'a gba_core::mem::MemMap, padding: &'a mut Vec<u16>) -> &'a [u16] {
    let width = wide_w();
    let height = wide_h();
    let ey = gba_core::mem::extra_y();
    if ey == 0 || gba_core::ppu::ffta_zoom_rows_ready(bus) {
        return &bus.wide_framebuffer[..width * height];
    }
    padding.resize(width * height, 0);
    let start = width * ey;
    let end = start + width * 160;
    if bus.ffta_dialogue.menu_pattern {
        for y in (0..ey).chain(ey + 160..height) {
            gba_core::ppu::render_menu_background_row(
                bus,
                y as i32 - ey as i32,
                &mut padding[y * width..(y + 1) * width],
            );
        }
    } else {
        padding[..start].fill(0);
        padding[end..].fill(0);
    }
    padding[start..end].copy_from_slice(&bus.wide_framebuffer[..width * 160]);
    padding
}

/// Normalize one Gilrs event into semantic transitions for the capture
/// router. Pressed and Changed may describe the same input; PadGesture
/// coalesces those transitions by physical held state.
#[cfg(not(target_os = "linux"))]
fn gilrs_control_transitions(event: gilrs::EventType) -> Vec<(String, bool)> {
    use gilrs::{Axis, Button, EventType};
    match event {
        EventType::ButtonPressed(button, _) if button != Button::Unknown => {
            vec![(format!("{button:?}"), true)]
        }
        EventType::ButtonReleased(button, _) if button != Button::Unknown => {
            vec![(format!("{button:?}"), false)]
        }
        EventType::ButtonChanged(button, value, _) if button != Button::Unknown => {
            if value >= 0.6 {
                vec![(format!("{button:?}"), true)]
            } else if value <= 0.4 {
                vec![(format!("{button:?}"), false)]
            } else {
                Vec::new()
            }
        }
        EventType::AxisChanged(axis, value, _) => {
            let pair = match axis {
                Axis::LeftStickX => Some(("LeftStickRight", "LeftStickLeft")),
                Axis::LeftStickY => Some(("LeftStickUp", "LeftStickDown")),
                Axis::RightStickX => Some(("RightStickRight", "RightStickLeft")),
                Axis::RightStickY => Some(("RightStickUp", "RightStickDown")),
                _ => None,
            };
            pair.map_or_else(Vec::new, |(positive, negative)| {
                if value >= 0.6 {
                    vec![(positive.into(), true), (negative.into(), false)]
                } else if value <= -0.6 {
                    vec![(positive.into(), false), (negative.into(), true)]
                } else if value.abs() <= 0.4 {
                    vec![(positive.into(), false), (negative.into(), false)]
                } else {
                    Vec::new()
                }
            })
        }
        _ => Vec::new(),
    }
}

/// Furthest zoom-out, as a multiple of the starting view (x1). The core's
/// buffers allow up to 5x; raise this to see more (costs CPU: the PPU draws
/// zoom^2 times the pixels while zoomed out).
const MAX_ZOOM: f32 = 2.0;
/// `video.scaling` from av.cfg -> shader mode: off = 0 (area average),
/// nearest = 1, sharp = 2. Unknown values fall back to off.
fn scale_mode_of(s: &str) -> u32 {
    match s.trim().to_lowercase().as_str() {
        "nearest" => 1,
        "sharp" => 2,
        "epx" => 3,
        _ => 0,
    }
}

/// Each +/- press multiplies / divides the zoom by this.
const ZOOM_STEP: f32 = 1.122_462; // 2^(1/6): six steps from x1 to x2

/// Extra columns per side. FFTA_WIDESCREEN (if set) overrides the saved
/// `video.aspect` from av.cfg. Accepted: "" / "off" / "0" / "3:2" =
/// original; "16:10" = 8; "16:9" = 22; "20:9" = 58; "21:9" = 66; or a
/// plain number of columns (max 66). Used as the STARTING zoom view only.
fn widescreen_extra(av: &input_config::AvConfig) -> usize {
    let env = std::env::var("FFTA_WIDESCREEN").unwrap_or_default();
    let v = if env.trim().is_empty() {
        av.video_aspect.clone()
    } else {
        env
    };
    aspect_extra(&v)
}

fn aspect_extra(value: &str) -> usize {
    match value.trim() {
        "" | "off" | "0" | "3:2" => 0,
        "16:10" => 8,
        "16:9" => 22,
        "20:9" => 58,
        "21:9" => 66,
        "32:9" => 164, // super ultrawide: experimental
        "auto" => auto_extra(),
        other => other.parse::<usize>().unwrap_or(0),
    }
}

fn zoom_limit(base_extra: usize) -> u32 {
    let by_cfg = (MAX_ZOOM.ln() / ZOOM_STEP.ln() + 1e-3).floor();
    let base_w = (240 + 2 * base_extra) as f32;
    let by_w =
        (((240 + 2 * gba_core::mem::MAX_EXTRA) as f32 / base_w).ln() / ZOOM_STEP.ln()).floor();
    let by_h =
        (((160 + 2 * gba_core::mem::MAX_EXTRA_Y) as f32 / 160.0).ln() / ZOOM_STEP.ln()).floor();
    by_cfg.min(by_w).min(by_h).max(0.0) as u32
}

/// "auto": match the primary monitor's aspect ratio (capped at 21:9; pick
/// 32:9 manually). Falls back to 16:9 if the size can't be read.
fn auto_extra() -> usize {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetSystemMetrics(index: i32) -> i32;
        }
        let (w, h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
        if w > 0 && h > 0 {
            let cols = 160.0 * w as f32 / h as f32;
            return (((cols - 240.0) / 2.0).round().max(0.0) as usize).min(66);
        }
    }
    22
}
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use recomp_core::fnv64;

#[cfg(not(target_os = "linux"))]
use crate::pad_pressed;
use crate::{
    apply_av_change, arm_audio_hle, build_gilrs, ensure_native, load_bios_file, load_native,
    make_machine, print_fallback_census, record_labels, rom_sha256, run_frame_native, start_audio,
    status_line, validate_rom, AudioStreams, BlockTable, FALLBACK_COLLECT, NO_INTERP, RING_TARGET,
};

/// Per-frame down-state of the host-menu navigation inputs, debounced against
/// the previous frame to produce rising-edge events. Kept separate from the
/// GBA key state — menu input never leaks to the game.
#[derive(Clone, Copy, Default)]
struct NavEdges {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    confirm: bool,
    /// Back out of the menu (Escape / pad East). Only consulted while the
    /// menu is open — East is the player's B button during gameplay, so it
    /// must never open the menu.
    cancel: bool,
    /// Open the menu from gameplay: keyboard Escape, or a dedicated pad
    /// control that doesn't collide with any GBA button (the guide/Mode
    /// button).
    open: bool,
}

/// Existing Linux shortcuts remain available in every build. The dedicated
/// Steam Deck binary additionally accepts North (the physical Y button).
fn linux_menu_shortcut(mode: bool, start: bool, select: bool, north: bool) -> bool {
    mode || (start && select) || (cfg!(feature = "steamdeck") && north)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveController {
    Gilrs(gilrs::GamepadId),
    DirectInput,
}

/// A mouse event queued while the pause menu is open (physical window px).
#[derive(Clone, Copy)]
enum MouseEv {
    Release,
    Move((f64, f64)),
    /// `true` = left button, `false` = right.
    Click((f64, f64), bool),
    /// `true` = wheel up.
    Wheel(bool),
}

/// Canonical key name (see input-config) to a winit *physical* key. Physical
/// (scancode-position) keys keep a binding in the same spot regardless of
/// QWERTY/AZERTY — the gamepad-style "this key here" model the old minifb
/// mapping had, which also sidesteps layout-dependent defaults.
fn winit_key(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match name {
        "Escape" => Escape,
        "F10" => F10,
        "F9" => F9,
        "Equal" => Equal,
        "Minus" => Minus,
        "A" => KeyA,
        "B" => KeyB,
        "C" => KeyC,
        "D" => KeyD,
        "E" => KeyE,
        "F" => KeyF,
        "G" => KeyG,
        "H" => KeyH,
        "I" => KeyI,
        "J" => KeyJ,
        "K" => KeyK,
        "L" => KeyL,
        "M" => KeyM,
        "N" => KeyN,
        "O" => KeyO,
        "P" => KeyP,
        "Q" => KeyQ,
        "R" => KeyR,
        "S" => KeyS,
        "T" => KeyT,
        "U" => KeyU,
        "V" => KeyV,
        "W" => KeyW,
        "X" => KeyX,
        "Y" => KeyY,
        "Z" => KeyZ,
        "0" => Digit0,
        "1" => Digit1,
        "2" => Digit2,
        "3" => Digit3,
        "4" => Digit4,
        "5" => Digit5,
        "6" => Digit6,
        "7" => Digit7,
        "8" => Digit8,
        "9" => Digit9,
        "Up" => ArrowUp,
        "Down" => ArrowDown,
        "Left" => ArrowLeft,
        "Right" => ArrowRight,
        "Enter" => Enter,
        "Space" => Space,
        "Tab" => Tab,
        "Backspace" => Backspace,
        "LeftShift" => ShiftLeft,
        "RightShift" => ShiftRight,
        _ => return None,
    })
}

/// Reverse of [`winit_key`]: the canonical config name for a physical key,
/// or `None` for keys the config format can't express (F-keys, Escape, ...).
fn key_name(code: KeyCode) -> Option<&'static str> {
    const NAMES: &[&str] = &[
        "Escape",
        "F9",
        "F10",
        "Equal",
        "Minus",
        "A",
        "B",
        "C",
        "D",
        "E",
        "F",
        "G",
        "H",
        "I",
        "J",
        "K",
        "L",
        "M",
        "N",
        "O",
        "P",
        "Q",
        "R",
        "S",
        "T",
        "U",
        "V",
        "W",
        "X",
        "Y",
        "Z",
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "Up",
        "Down",
        "Left",
        "Right",
        "Enter",
        "Space",
        "Tab",
        "Backspace",
        "LeftShift",
        "RightShift",
    ];
    NAMES.iter().copied().find(|n| winit_key(n) == Some(code))
}

/// Crossing the pause boundary, clear the audio defect counters so the
/// drained-ring underruns during pause don't false-fire the DEGRADED alarm
/// next report window.
fn reset_audio_counters(streams: &Arc<Mutex<AudioStreams>>) {
    let mut st = streams.lock().unwrap();
    st.drops = 0;
    st.holds = 0;
    st.clipped = 0;
}

/// All play-session state. Pre-window state is built in [`cmd_play`] and moved
/// in; the window, GPU presenter, and software-blit surface are created in
/// [`ApplicationHandler::resumed`] (window creation is only valid once the
/// event loop is active).
struct PlayApp {
    // Core machine + persistence.
    machine: Machine,
    sav_path: String,
    autosave: crate::autosave::AutoSave,
    autosave_frame: Vec<u16>,
    sav_seen: Option<u64>,
    record: Option<(std::fs::File, u16)>,
    native: Option<(super::NativeLibrary, BlockTable)>,

    // Audio.
    streams: Arc<Mutex<AudioStreams>>,
    stream: Option<cpal::Stream>,

    // Input.
    icfg: input_config::InputConfig,
    key_pairs: Vec<(KeyCode, u16)>,
    pad: Option<gilrs::Gilrs>,
    directinput: Option<input_config::directinput::DirectInput>,
    directinput_only: bool,
    #[cfg(windows)]
    xinput: Option<XInputBackend>,
    menu_tap_filter: MenuTapFilter,
    active_controller: Option<ActiveController>,
    last_direct_activity: Option<(String, Vec<String>)>,
    pending_pad: Option<String>,
    #[cfg(not(target_os = "linux"))]
    gilrs_gestures: HashMap<gilrs::GamepadId, input_config::PadGesture>,
    #[cfg(target_os = "linux")]
    linux_pads: HashMap<gilrs::GamepadId, crate::linux_pad::Pad>,
    direct_gestures: HashMap<String, input_config::PadGesture>,
    pad_zoom_prev: (bool, bool),
    pad_ff_held: bool,
    pad_ff_toggle_prev: bool,
    /// Physical keys currently held, reconstructed from key press/release
    /// events — winit's event-driven equivalent of minifb's `is_key_down`.
    held: HashSet<KeyCode>,

    // Screen simulation (rebuilt live by the menu).
    av: input_config::AvConfig,
    screen_kind: screen::ScreenKind,
    /// Configured gamut (auto/srgb/display-p3), used for the macOS layer tag.
    display_target: screen::DisplayTarget,
    /// Primaries the LUT actually encodes for, resolved once from
    /// `display_target` + OS detection (may be the measured panel gamut).
    lut_primaries: screen::color::Primaries,
    lut: screen::ColorLut,
    response: screen::ResponseMode,
    temporal: screen::Temporal,
    grid_params: screen::present::GridParams,
    rgba: Vec<[u8; 4]>,
    pause_rgba: Vec<[u8; 4]>,
    ui_pixels: Vec<[u8; 4]>,
    battle_hud_pixels: [Vec<[u8; 4]>; 4],
    battle_hud_motion: ui_motion::BattleHud,
    panel_pixels: Vec<[u8; 4]>,
    ui_auxiliary: [Vec<[u8; 4]>; 2],
    ui_scene: gba_core::ffta_ui::Scene,
    ui_motion: ui_motion::Motion,
    pub_scroll_drag: Option<f32>,
    ui_pack_cache: [gba_core::ffta_dialogue::PackedLayer; 4],
    ui_inspector: crate::ui_inspect::Inspector,
    ui_timing: crate::ui_inspect::Timing,
    padded_frame: Vec<u16>,

    // Window + presentation (created in `resumed`).
    title: String,
    status: bool,
    window: Option<Arc<Window>>,
    presenter: Option<screen::present::Presenter>,
    sb_context: Option<softbuffer::Context<Arc<Window>>>,
    sb_surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,

    // Menu / pause.
    menu_enabled: bool,
    menu: menu::Menu,
    paused: bool,
    menu_dirty: bool,
    cursor_visible: bool,
    suppress: u16,
    nav_prev: NavEdges,

    // Perf + audio watchdog.
    show_stats: bool,
    emu_ema_ms: f64,
    frames_run: u64,
    slow_warned: bool,
    native_run: u64,
    fallback_run: u64,
    primed: bool,
    next_frame: Instant,
    audio_watch: Instant,
    audio_consumed_last: u64,
    audio_stalls: u8,
    audio_retry: Option<Instant>,

    // Fast forward: remappable hold/toggle activation and selected audio mode.
    ff_active: bool,
    ff_latched: bool,
    ff_speed: f32,
    ff_budget: input_config::FrameBudget,
    ff_audio_active: String,
    /// Last key pressed while the menu is open (consumed by key capture).
    pending_key: Option<KeyCode>,

    /// Extra columns per side of the starting view (x1): the aspect chosen
    /// in the launcher. The starting view is (240 + 2 * base_extra) x 160 and
    /// every zoom level keeps exactly that aspect ratio.
    base_extra: usize,
    /// Zoom step index: zoom = ZOOM_STEP ^ zoom_level (0 = x1).
    zoom_level: u32,
    zoom_display: f32,
    zoom_from: f32,
    zoom_started: Option<Instant>,
    zoom_max: u32,
    /// Crop shown, [x0, y0, w, h] in the rendered frame's pixels. Updated
    /// together with the render size (sync_view) so they never disagree.
    view: [f32; 4],
    /// Last cursor position in physical window pixels.
    mouse_pos: Option<(f64, f64)>,
    game_mouse: gba_core::ffta_mouse::LanguageMouse,
    game_keys: u16,
    /// Mouse events received while paused, consumed by the menu each tick.
    mouse_queue: Vec<MouseEv>,

    /// A fatal error raised inside a callback; surfaced after `run_app`.
    error: Option<String>,
}

impl PlayApp {

    /// Width of the starting (x1) view, in frame pixels.
    fn base_w(&self) -> f32 {
        (240 + 2 * self.base_extra) as f32
    }

    /// Zoom by `steps` (positive = zoom out). x1 is the floor: it never goes
    /// below the starting view, so the aspect ratio never changes.
    fn zoom_by(&mut self, steps: i32) {
        let next = (self.zoom_level as i32 + steps).clamp(0, self.zoom_max as i32) as u32;
        if next != self.zoom_level {
            self.zoom_from = self.zoom_display;
            self.zoom_started = Some(Instant::now());
            self.zoom_level = next;
            eprintln!("zoom: x{:.2}", ZOOM_STEP.powi(next as i32));
        }
    }

    /// Make the core render exactly what the current zoom shows, resize the
    /// buffers if that changed, and recompute the crop. Called at the start
    /// of every emulated frame, never in the middle of one.
    fn sync_view(&mut self) {
        let target = ZOOM_STEP.powi(self.zoom_level as i32);
        let t = self.zoom_started.map_or(1.0, |start| {
            (start.elapsed().as_secs_f32() / 0.20).clamp(0.0, 1.0)
        });
        let ease = t * t * (3.0 - 2.0 * t);
        let z = self.zoom_from + (target - self.zoom_from) * ease;
        self.zoom_display = z;
        if t >= 1.0 {
            self.zoom_started = None;
            self.zoom_from = target;
        }
        let vw = self.base_w() * z;
        let vh = 160.0 * z;
        // Allocate the transition's full extent once. Only the GPU crop
        // changes per frame, rather than reallocating textures at every step.
        let render_zoom = if t < 1.0 {
            self.zoom_from.max(target)
        } else {
            target
        };
        let ex = (((self.base_w() * render_zoom - 240.0) / 2.0)
            .ceil()
            .max(0.0) as usize)
            .min(gba_core::mem::MAX_EXTRA);
        let ey = (((160.0 * render_zoom - 160.0) / 2.0).ceil().max(0.0) as usize)
            .min(gba_core::mem::MAX_EXTRA_Y);
        if ex != gba_core::mem::extra() || ey != gba_core::mem::extra_y() {
            if gba_core::ffta_log::enabled() {
                eprintln!(
                    "FFTA_VIEW frame={} zoom_level={} zoom={z:.6} extra_x={ex} extra_y={ey} render_width={} render_height={} view_width={vw:.6} view_height={vh:.6}",
                    self.machine.bus.frames,
                    self.zoom_level,
                    240 + 2 * ex,
                    160 + 2 * ey
                );
            }
            gba_core::mem::set_extra(ex);
            gba_core::mem::set_extra_y(ey);
            self.rgba = vec![[0u8; 4]; wide_w() * wide_h()];
            self.rebuild_temporal();
        }
        let (rw, rh) = (wide_w() as f32, wide_h() as f32);
        let (vw, vh) = (vw.min(rw), vh.min(rh));
        self.view = [(rw - vw) / 2.0, (rh - vh) / 2.0, vw, vh];
        self.machine.bus.ffta_dialogue.view_zoom = z;
        self.machine.bus.ffta_dialogue.battle_hud_overlay = true;
        self.machine.bus.ffta_dialogue.view_width = vw;
        self.machine.bus.ffta_dialogue.view_height = vh;
        self.machine.bus.ffta_dialogue.style.scale = self.av.bubble_size;
        self.machine.bus.ffta_dialogue.style.length = self.av.bubble_length;
    }

    /// Recreate the temporal-response state for the current frame size (its
    /// history is per pixel, so it must match the rendered size).
    fn rebuild_temporal(&mut self) {
        self.temporal = screen::Temporal::new(
            wide_w() * wide_h(),
            self.response,
            0,
            input_config::AvConfig::knob(&self.av.response_keep)
                .unwrap_or_else(|| screen::blend::default_rho(self.screen_kind)),
        );
    }

    /// Window pixel -> frame pixel (in the full rendered frame's coordinates),
    /// or `None` over the letterbox bars.
    fn menu_dimensions(&self) -> (usize, usize) {
        let Some(window) = self.window.as_ref() else {
            return (1280, 720);
        };
        let size = window.inner_size();
        // Fonts and vector icons are rasterized at physical window pixels.
        // Never enlarge a smaller menu canvas on high-resolution displays.
        (size.width.max(1) as usize, size.height.max(1) as usize)
    }
    fn window_to_frame(&self, pos: (f64, f64)) -> Option<(usize, usize)> {
        let window = self.window.as_ref()?;
        let size = window.inner_size();
        if self.paused {
            let (w, h) = self.menu_dimensions();
            if pos.0 < 0. || pos.1 < 0. || pos.0 >= size.width as f64 || pos.1 >= size.height as f64
            {
                return None;
            }
            return Some((
                (pos.0 * w as f64 / size.width.max(1) as f64) as usize,
                (pos.1 * h as f64 / size.height.max(1) as f64) as usize,
            ));
        }
        let [x0, y0, vw, vh] = self.view;
        let (scale, ox, oy) = screen::present::letterbox_f((size.width, size.height), vw, vh);
        let fx = (pos.0 as f32 - ox) / scale;
        let fy = (pos.1 as f32 - oy) / scale;
        if fx < 0.0 || fy < 0.0 || fx >= vw || fy >= vh {
            return None;
        }
        Some((
            ((x0 + fx) as usize).min(wide_w() - 1),
            ((y0 + fy) as usize).min(wide_h() - 1),
        ))
    }

    fn roster_scrollbar(&self) -> Option<screen::ui_scrollbar::Bar> {
        if !self.icfg.mouse_keyboard {
            return None;
        }
        let list = gba_core::ffta_roster_mouse::scrollbar_list(&self.machine.bus)?;
        screen::ui_scrollbar::Bar::centered(list.bounds, list.first, list.visible, list.total)
    }
    fn roster_scrollbar_pointer(&mut self, pos: (f64, f64), pressed: bool) -> bool {
        let Some(bar) = self.roster_scrollbar() else {
            return false;
        };
        let Some(window) = &self.window else {
            return false;
        };
        let size = window.inner_size();
        let (scale, ox, oy) = screen::present::letterbox_f(
            (size.width.max(1), size.height.max(1)),
            self.view[2],
            self.view[3],
        );
        let x = self.view[0] + (pos.0 as f32 - ox) / scale - gba_core::mem::extra() as f32;
        let y = self.view[1] + (pos.1 as f32 - oy) / scale - gba_core::mem::extra_y() as f32;
        if self.pub_scroll_drag.is_none() && !bar.hit(x, y) {
            return false;
        }
        if pressed {
            self.pub_scroll_drag = Some(bar.grab(y));
        }
        if let Some(grab) = self.pub_scroll_drag {
            gba_core::ffta_roster_mouse::scroll_to(&mut self.machine.bus, bar.first_at(y, grab));
        }
        true
    }
    fn pub_scrollbar_slot(&self) -> usize {
        if gba_core::ffta_battle_mouse::battle_list(&self.machine.bus).is_some() {
            1
        } else {
            0
        }
    }
    fn pub_scrollbar(&self) -> Option<screen::ui_scrollbar::Bar> {
        if !self.icfg.mouse_keyboard {
            return None;
        }
        if !gba_core::ffta_battle_mouse::pub_scroll_visible(&self.machine.bus) {
            return None;
        }
        let battle = gba_core::ffta_battle_mouse::battle_list(&self.machine.bus);
        let list = gba_core::ffta_battle_mouse::pub_list(&self.machine.bus)
            .or(battle)
            .or_else(|| gba_core::ffta_shop_mouse::list(&self.machine.bus))?;
        let node = self.ui_scene.nodes[if battle.is_some() { 1 } else { 0 }]?;
        let mut bounds = node.bounds;
        if battle.is_some() {
            let menu = gba_core::ffta_battle_mouse::MenuScene::read(&self.machine.bus)?;
            bounds[1] = (menu.origin[1] - 4).max(node.bounds[1]);
            bounds[3] = (menu.origin[1] - 4 + list.visible as i32 * menu.pitch).min(node.bounds[3]);
        }
        if self.machine.bus.ffta_dialogue.shop_layout {
            bounds = [8, 0, 216, 88];
        } else if gba_core::ffta_surface::Descriptor::read(&self.machine.bus)
            .is_some_and(|s| matches!(s.id, 66 | 67))
        {
            let window = gba_core::ffta_ui_window::pub_windows(&self.machine.bus, [None; 3])[1]?;
            bounds = window.frame_bounds;
            if gba_core::ffta_surface::Descriptor::read(&self.machine.bus)
                .is_some_and(|s| s.id == 67)
            {
                bounds[2] = (bounds[2] + 8).min(240);
            }
        }
        if battle.is_some() {
            screen::ui_scrollbar::Bar::centered(bounds, list.first, list.visible, list.total)
        } else {
            screen::ui_scrollbar::Bar::new(bounds, list.first, list.visible, list.total)
        }
    }
    fn pub_scrollbar_pointer(&mut self, pos: (f64, f64), pressed: bool) -> bool {
        let Some(bar) = self.pub_scrollbar() else {
            self.pub_scroll_drag = None;
            return false;
        };
        let Some(window) = &self.window else {
            return false;
        };
        let size = window.inner_size();
        let Some(layout) = self.ui_layout(
            (size.width.max(1), size.height.max(1)),
            self.pub_scrollbar_slot(),
        ) else {
            return false;
        };
        // Keep dragging when the pointer moves above/below the track.
        let k = layout.rect[2] / 480.;
        let x = (pos.0 as f32 - layout.rect[0]) / k - 120.;
        let y = (pos.1 as f32 - layout.rect[1]) / k;
        if self.pub_scroll_drag.is_none() && !bar.hit(x, y) {
            return false;
        }
        if pressed {
            self.pub_scroll_drag = Some(bar.grab(y));
        }
        if let Some(grab) = self.pub_scroll_drag {
            let first = bar.first_at(y, grab);
            if !gba_core::ffta_battle_mouse::pub_scroll_to(&mut self.machine.bus, first) {
                gba_core::ffta_shop_mouse::scroll_to(&mut self.machine.bus, first);
            }
        }
        true
    }
    fn game_mouse_aim(&mut self, pos: (f64, f64), confirm: bool) -> bool {
        if confirm {
            visual_trace::capture(
                &self.machine.bus,
                &self.ui_scene,
                Some(pos),
                self.view,
                true,
            );
        }
        if !self.icfg.mouse_keyboard {
            return false;
        }
        if self.autosave.screen_active(&self.machine.bus) {
            if let Some((x, y)) = self.window_to_frame(pos) {
                let x = x as f32 - gba_core::mem::extra() as f32;
                let y = y as f32 - gba_core::mem::extra_y() as f32;
                if crate::autosave::confirming_save(&self.machine.bus) {
                    return gba_core::ffta_battle_mouse::aim(&mut self.machine.bus, x, y, confirm);
                }
                let saving = crate::autosave::is_saving(&self.machine.bus);
                if self.autosave.delete_confirm.is_some() {
                    if confirm {
                        let accept = x >= 131. && x < 160. && y >= 100. && y < 121.;
                        let cancel = x >= 79. && x < 108. && y >= 100. && y < 121.;
                        if accept || cancel {
                            if let Err(e) = self.autosave.finish_delete(accept) {
                                self.autosave.message = Some(e);
                            }
                        }
                    }
                    return true;
                }
                if let Some(id) = self.autosave.trash_hit(saving, x, y) {
                    if confirm {
                        self.autosave.request_delete(id);
                    }
                    return true;
                }

                if let Some(right) = self.autosave.arrow_hit(saving, x, y) {
                    if confirm {
                        self.autosave.scroll_carousel(saving, right);
                    }
                    return true;
                }
                if let Some(slot) = self
                    .autosave
                    .hit_card(saving, x, y)
                    .filter(|slot| crate::autosave::selectable(&self.machine.bus, *slot))
                {
                    self.autosave.selected = slot;
                    if confirm {
                        self.activate_save_card();
                    }
                }
            }
            return true;
        }

        // Party is a gallery event, not a text-menu or frontend selector.
        if gba_core::ffta_roster_mouse::party_gallery(&self.machine.bus) {
            // The gallery owns its native canvas, including its scrollbar.
            // Consume drag/click before testing troop rectangles underneath.
            if self.roster_scrollbar_pointer(pos, confirm) {
                return true;
            }
            let hit = self.window_to_frame(pos).is_some_and(|(x, y)| {
                let native = (
                    x as f32 - gba_core::mem::extra() as f32,
                    y as f32 - gba_core::mem::extra_y() as f32,
                );
                let accepted = gba_core::ffta_roster_mouse::aim(
                    &mut self.machine.bus,
                    native.0,
                    native.1,
                    confirm,
                );
                if confirm {
                    eprintln!(
                        "FFTA_PARTY_CLICK native=({:.1},{:.1}) accepted={accepted}",
                        native.0, native.1
                    );
                }
                accepted
            });
            return hit;
        }
        // Naming is drawn on its own UI canvas; world zoom is not its transform.
        if self.machine.bus.ffta_dialogue.naming_layout {
            return self
                .window
                .as_ref()
                .and_then(|w| {
                    let s = w.inner_size();
                    self.ui_layout((s.width.max(1), s.height.max(1)), 0)
                })
                .and_then(|l| l.source(pos.0 as f32, pos.1 as f32))
                .is_some_and(|p| {
                    self.game_mouse.aim(
                        &self.machine.bus,
                        (p % 480) as f32 - 120.,
                        (p / 480) as f32,
                        confirm,
                    )
                });
        }
        // Frontend/save screens own input before stale gameplay selectors.
        if self.game_mouse.scene(&self.machine.bus).is_some() {
            return self.window_to_frame(pos).is_some_and(|(x, y)| {
                self.game_mouse.aim(
                    &self.machine.bus,
                    x as f32 - gba_core::mem::extra() as f32,
                    y as f32 - gba_core::mem::extra_y() as f32,
                    confirm,
                )
            });
        }
        // A verified text wait owns the click even if a service keeps its
        // root selector alive behind the dialogue. Never turn a failed text
        // request into a consumed click or a menu confirmation.
        let menu_present = gba_core::ffta_roster_mouse::active(&self.machine.bus)
            || gba_core::ffta_battle_mouse::MenuScene::read(&self.machine.bus).is_some();
        let speech_present = self.machine.bus.ffta_dialogue.service_speech.active()
            || self.machine.bus.ffta_dialogue.split_speech
                && self.ui_scene.nodes.iter().enumerate().any(|(slot, node)| {
                    node.is_some_and(|n| n.role == gba_core::ffta_ui::Role::Dialogue)
                        && self
                            .ui_pixels_at(slot)
                            .iter()
                            .any(|p| p[2] != 0 || p[3] != 16)
                });
        if confirm
            && (!menu_present || speech_present)
            && self.window_to_frame(pos).is_some()
            && gba_core::ffta_battle_mouse::advance_dialogue(&mut self.machine.bus)
        {
            self.game_mouse.cancel();
            return true;
        }
        if let Some(choice) = gba_core::ffta_battle_mouse::DialogueChoice::read(&self.machine.bus) {
            // A question owns input before an inactive service menu behind it.
            // Yes/No belongs to its rendered canvas, including centered NOTICE
            // prompts such as Begin Battle. World coordinates are only valid
            // on frontend prompts that have no detached UI canvas.
            if let Some((slot, _)) = self
                .ui_scene
                .nodes
                .iter()
                .enumerate()
                .find(|(_, n)| n.is_some_and(|n| dialogue_choice_canvas(n.role)))
            {
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    if let Some(index) = self
                        .ui_layout((size.width.max(1), size.height.max(1)), slot)
                        .and_then(|l| l.source(pos.0 as f32, pos.1 as f32))
                    {
                        let x = (index % 480) as f32 - 120.;
                        let y = (index / 480) as f32;
                        if let Some(target) = choice.hit(x, y) {
                            return choice.select(&mut self.machine.bus, target, confirm);
                        }
                    }
                }
            } else if let Some((x, y)) = self.window_to_frame(pos) {
                // Save-file NOTICE uses the native frontend planes, without a
                // detached dialogue canvas. Its choice model is still the
                // same original text engine (state 9).
                if let Some(target) = choice.hit(
                    x as f32 - gba_core::mem::extra() as f32,
                    y as f32 - gba_core::mem::extra_y() as f32,
                ) {
                    return choice.select(&mut self.machine.bus, target, confirm);
                }
            }
            return true; // Outside Yes/No: do not confirm a hidden menu.
        }
        if self.roster_scrollbar_pointer(pos, confirm) {
            return true;
        }
        // Native Party/Clan/Area menus own their own canvas; never transform
        // their pointer through a stale battle/service panel behind them.
        if gba_core::ffta_roster_mouse::Scene::read(&self.machine.bus).is_some() {
            return self.window_to_frame(pos).is_some_and(|(x, y)| {
                gba_core::ffta_roster_mouse::aim(
                    &mut self.machine.bus,
                    x as f32 - gba_core::mem::extra() as f32,
                    y as f32 - gba_core::mem::extra_y() as f32,
                    confirm,
                )
            });
        }
        if gba_core::ffta_battle_mouse::FacingPrompt::read(&self.machine.bus).is_some() {
            // PC uses keyboard/gamepad for facing. Keep ordinary mouse A,
            // while the calibrated actor-relative selector remains in the core
            // for Android's experimental touch path.
            if confirm {
                gba_core::ffta_battle_mouse::press(&mut self.machine.bus, 0x3fe);
            }
            return true;
        }
        if self.pub_scrollbar_pointer(pos, confirm) {
            return true;
        }
        let scene = gba_core::ffta_battle_mouse::MenuScene::read(&self.machine.bus);
        let state = &self.machine.bus.ffta_dialogue;
        let ui_index = if state.service_layout {
            // Submenus/inventory belong to the right canvas even while their
            // inactive root MENU still exists on the independent left canvas.
            Some(
                if !state.shop_layout
                    && scene.is_some_and(|m| {
                        state.extra.bounds.is_some_and(|[l, t, r, b]| {
                            m.origin[0] >= l
                                && m.origin[0] < r
                                && m.origin[1] >= t
                                && m.origin[1] < b
                        })
                    })
                {
                    1
                } else {
                    0
                },
            )
        } else {
            battle_menu_ui_layer_at(state, &self.ui_scene, scene.map(|m| m.origin))
        };
        if (scene.is_some() || state.shop_layout) && ui_index.is_some() {
            if let Some(window) = &self.window {
                let size = window.inner_size();
                if let Some(layout) =
                    self.ui_layout((size.width.max(1), size.height.max(1)), ui_index.unwrap())
                {
                    if let Some(index) = layout.source(pos.0 as f32, pos.1 as f32) {
                        let state = &self.machine.bus.ffta_dialogue;
                        if let Some(node) = self.ui_scene.nodes[ui_index.unwrap()] {
                            let mut bounds = node.bounds;
                            if node.length {
                                let amount = ((bounds[2] - bounds[0]) as f32
                                    * (state.style.length.clamp(1., 2.) - 1.))
                                    .round() as i32;
                                match node.anchor {
                                    gba_core::ffta_ui::Anchor::Corner { right, .. }
                                    | gba_core::ffta_ui::Anchor::Edge { right, .. } => {
                                        if right {
                                            bounds[0] -= amount;
                                        } else {
                                            bounds[2] += amount;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if matches!(
                                node.role,
                                gba_core::ffta_ui::Role::ServiceMenu
                                    | gba_core::ffta_ui::Role::ServiceContent
                                    | gba_core::ffta_ui::Role::ActionMenu
                                    | gba_core::ffta_ui::Role::Notice
                            ) && (!pointer_in_ui_bounds(index, bounds)
                                || !self
                                    .ui_pixels_at(ui_index.unwrap())
                                    .get(index)
                                    .is_some_and(|p| p[2] != 0 || p[3] != 16))
                            {
                                self.machine.bus.ffta_battle_mouse.cancel();
                                return false;
                            }
                        }
                        if state.town_layout
                            || state.shop_layout
                            || state.site_layout
                            || state.extra.kind == gba_core::ffta_dialogue::PanelKind::Notice
                        {
                            let x = (index % 480) as f32 - 120.;
                            let y = (index / 480) as f32;
                            return gba_core::ffta_battle_mouse::aim(
                                &mut self.machine.bus,
                                x,
                                y,
                                confirm,
                            );
                        }
                        let bounds = self.ui_scene.nodes[ui_index.unwrap()].map(|n| n.bounds);
                        if let Some(b) = bounds {
                            let (x, y) = menu_panel_native(
                                index,
                                b,
                                if state.town_layout || state.shop_layout {
                                    1.
                                } else {
                                    state.style.length
                                },
                            );
                            return gba_core::ffta_battle_mouse::aim(
                                &mut self.machine.bus,
                                x,
                                y,
                                confirm,
                            );
                        }
                    }
                    self.machine.bus.ffta_battle_mouse.cancel();
                    return false;
                }
            }
        }
        if let Some((x, y)) = self.window_to_frame(pos) {
            let x = x as f32 - gba_core::mem::extra() as f32;
            let y = y as f32 - gba_core::mem::extra_y() as f32;
            if self.game_mouse.scene(&self.machine.bus).is_some() {
                self.game_mouse.aim(&self.machine.bus, x, y, confirm)
            } else {
                gba_core::ffta_battle_mouse::aim(&mut self.machine.bus, x, y, confirm)
            }
        } else {
            self.game_mouse.cancel();
            self.machine.bus.ffta_battle_mouse.cancel();
            false
        }
    }

    fn choose_rom(&mut self) -> bool {
        let Some(path) = crate::rom_switch::pick(&self.sav_path) else {
            return false;
        };
        let result = crate::rom_switch::validate(&path).and_then(|_| {
            // Flush the current cartridge before the replacement process reads
            // any save. A failed save or launch keeps this paused game alive.
            crate::rom_switch::launch(&path, &self.sav_path)
        });
        match result {
            Ok(()) => true,
            Err(e) => {
                crate::rom_switch::show_error(&e);
                false
            }
        }
    }
    fn activate_save_card(&mut self) {
        let slot = self.autosave.selected;
        if !crate::autosave::selectable(&self.machine.bus, slot) {
            return;
        }
        if crate::autosave::is_saving(&self.machine.bus) {
            match self.autosave.save_and_return(slot - 1) {
                Ok(()) => {
                    self.game_mouse.cancel();
                    self.suppress = 0x3ff;
                }
                Err(e) => {
                    eprintln!("FFTA_STATE_SAVE error={e}");
                    self.autosave.message = Some(e);
                }
            }
        } else if slot == 0 {
            if self.autosave.available() {
                self.autosave.queue_load(0);
            }
        } else if self.autosave.new_slot(slot - 1) {
            self.autosave.queue_load(slot);
        } else if !self.autosave.deleted(slot) {
            match crate::autosave::activate_manual(&mut self.machine, slot - 1) {
                Ok(()) => {
                    self.autosave.import_after_load(slot - 1);
                    self.game_mouse.cancel();
                    self.suppress = 0x3ff;
                }
                Err(e) => {
                    eprintln!("FFTA_IMPORT error={e}");
                    self.autosave.message = Some(e);
                }
            }
        }
    }
    fn load_auto_state(&mut self) {
        self.autosave.pending_load = false;
        match self.autosave.load(&mut self.machine) {
            Ok(()) => {
                arm_audio_hle(&mut self.machine, None);
                self.game_mouse.cancel();
                self.machine.bus.ffta_battle_mouse.cancel();
                self.machine.bus.keys = 0x3ff;
                self.game_keys = 0x3ff;
                self.suppress = 0x3ff;
                self.held.clear();
                self.ui_motion = Default::default();
                self.battle_hud_motion = Default::default();
                self.ui_pixels.clear();
                self.panel_pixels.clear();
                for p in &mut self.ui_auxiliary {
                    p.clear();
                }
                for c in &mut self.ui_pack_cache {
                    c.invalidate();
                }
                self.ui_scene = Default::default();
                self.rebuild_temporal();
                reset_audio_counters(&self.streams);
                self.next_frame = Instant::now();
                self.sav_seen = self.machine.bus.save_data().map(fnv64);
                eprintln!("FFTA_AUTOSAVE restored frame={}", self.machine.bus.frames);
            }
            Err(e) => {
                eprintln!("FFTA_AUTOSAVE load_failed={e}");
                self.autosave.message = Some(e);
            }
        }
    }
    fn ui_layout(&self, physical: (u32, u32), index: usize) -> Option<screen::ui_layer::Layout> {
        if !self.ui_motion.panels[index].input() {
            return None;
        }
        let node = self.ui_scene.nodes.get(index).copied().flatten()?;
        Some(self.ui_motion.layout(
            index,
            host_ui_layout(
                physical,
                self.base_w(),
                self.machine.bus.ffta_dialogue.style,
                node,
            ),
        ))
    }
    fn ui_render_layout(
        &self,
        physical: (u32, u32),
        index: usize,
    ) -> Option<screen::ui_layer::Layout> {
        let node = self.ui_motion.drawing(index)?;
        Some(self.ui_motion.layout(
            index,
            host_ui_layout(
                physical,
                self.base_w(),
                self.machine.bus.ffta_dialogue.style,
                node,
            ),
        ))
    }
    fn ui_pixels_at(&self, index: usize) -> &[[u8; 4]] {
        match index {
            0 => &self.ui_pixels,
            1 => &self.panel_pixels,
            2 => &self.ui_auxiliary[0],
            3 => &self.ui_auxiliary[1],
            _ => &[],
        }
    }
    fn ui_world_rect(&self, physical: (u32, u32)) -> [f32; 4] {
        let (s, x, y) = screen::present::letterbox_f(physical, self.base_w(), 160.);
        [x, y, self.base_w() * s, 160. * s]
    }
    fn ui_frame(
        &self,
        layout: screen::ui_layer::Layout,
        index: usize,
    ) -> screen::ui_layer::Frame<'_> {
        screen::ui_layer::Frame {
            pixels: self.ui_motion.panels[index].pixels(),
            raw_world: if index == 1
                && self.machine.bus.ffta_dialogue.extra.kind
                    == gba_core::ffta_dialogue::PanelKind::Stats
            {
                &self.machine.bus.ffta_dialogue.extra.background
            } else {
                &self.machine.bus.wide_framebuffer
            },
            world_dims: (wide_w() as u32, wide_h() as u32),
            lut: self.lut.table.as_slice(),
            layout,
            world_view: self.view,
            world_rect: self.ui_world_rect(
                self.window
                    .as_ref()
                    .map(|w| {
                        let s = w.inner_size();
                        (s.width, s.height)
                    })
                    .unwrap_or((1, 1)),
            ),
        }
    }

    /// Present `self.rgba` at the window's current size: the GPU grid/scaling
    /// pass when available, otherwise the software letterbox blit (still fully
    /// color-corrected, no grid). Cloning the `Arc` avoids borrowing
    /// `self.window` while `self.presenter`/`self.sb_surface` are mutated.
    fn present_rgba(&mut self) {
        let save_ui = !self.paused && self.autosave.screen_active(&self.machine.bus);
        if save_ui {
            let (w, h) = self.menu_dimensions();
            let mut canvas = std::mem::take(&mut self.pause_rgba);
            crate::autosave::compose_display(
                &self.machine.bus,
                &self.autosave,
                &self.rgba,
                &mut canvas,
                (wide_w(), wide_h()),
                (w, h),
                self.view,
                self.lut.table.as_slice(),
            );
            std::mem::swap(&mut self.rgba, &mut canvas);
            let view = self.view;
            self.view = [0., 0., w as f32, h as f32];
            self.present_rgba_content(true);
            self.view = view;
            std::mem::swap(&mut self.rgba, &mut canvas);
            self.pause_rgba = canvas;
        } else {
            self.present_rgba_content(false);
        }
    }
    fn present_rgba_content(&mut self, save_ui: bool) {
        let ui_presented = self.ui_timing.start();
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let phys = (size.width.max(1), size.height.max(1));
        let frame_dims = if self.paused || save_ui {
            let (w, h) = self.menu_dimensions();
            (w as u32, h as u32)
        } else {
            (wide_w() as u32, wide_h() as u32)
        };
        if let Some(mut p) = self.presenter.take() {
            if !self.paused && !save_ui {
                let mut layers: Vec<_> = (0..gba_core::ffta_ui::LAYERS)
                    .map(|i| {
                        self.ui_render_layout(phys, i)
                            .map(|layout| self.ui_frame(layout, i))
                    })
                    .collect();
                for i in 0..4 {
                    let base = screen::ui_layer::Layout::centered(phys, self.base_w(), 1.);
                    layers.push(self.battle_hud_motion.layout(i, base).map(|layout| {
                        screen::ui_layer::Frame {
                            pixels: self.battle_hud_motion.pixels(i),
                            raw_world: &self.machine.bus.wide_framebuffer,
                            world_dims: (wide_w() as u32, wide_h() as u32),
                            lut: self.lut.table.as_slice(),
                            layout,
                            world_view: self.view,
                            world_rect: self.ui_world_rect(phys),
                        }
                    }));
                }
                p.set_ui_layers(&layers);
            } else {
                p.set_ui_layer(None);
            }
            let mut grid = self.grid_params;
            if self.paused || save_ui {
                grid.strength = 0.;
                grid.scale_mode = 1;
            }
            match p.present(&self.rgba, frame_dims, phys, &grid, self.view) {
                Ok(()) => {
                    self.presenter = Some(p);
                    self.ui_timing.presented(ui_presented);
                    return;
                }
                Err(e) => {
                    eprintln!("video: GPU presenter failed ({e}); CPU scaling active");
                    self.presenter = None;
                }
            }
        }
        #[cfg(target_os = "linux")]
        if self.presenter.is_none() && self.sb_surface.is_none() {
            eprintln!("video: software presenter active; CPU scaling will be used");
        }
        self.present_software(&window, phys, save_ui);
        self.ui_timing.presented(ui_presented);
    }

    /// Software fallback (no GPU / presenter lost): nearest-neighbor, aspect-
    /// correct letterbox of the visible crop into the window via softbuffer,
    /// matching the GPU presenter's centering. Lazily creates the surface.
    fn present_software(&mut self, window: &Arc<Window>, (w, h): (u32, u32), save_ui: bool) {
        let ui_layouts = std::array::from_fn::<_, { gba_core::ffta_ui::LAYERS }, _>(|i| {
            if self.paused || save_ui {
                None
            } else {
                self.ui_render_layout((w, h), i)
            }
        });
        // Zoom crop, computed before `surf` borrows `self` mutably.
        let [vx0, vy0, vw, vh] = self.view;
        let (stride, frame_h) = if self.paused || save_ui {
            self.menu_dimensions()
        } else {
            (wide_w(), wide_h())
        };
        if self.sb_surface.is_none() {
            match softbuffer::Context::new(window.clone()) {
                Ok(ctx) => match softbuffer::Surface::new(&ctx, window.clone()) {
                    Ok(surf) => {
                        self.sb_context = Some(ctx);
                        self.sb_surface = Some(surf);
                    }
                    Err(e) => {
                        eprintln!("video: software surface unavailable ({e})");
                        return;
                    }
                },
                Err(e) => {
                    eprintln!("video: software context unavailable ({e})");
                    return;
                }
            }
        }
        let ui_world_rect = self.ui_world_rect((w, h));
        let Some(surf) = self.sb_surface.as_mut() else {
            return;
        };
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) else {
            return;
        };
        if let Err(e) = surf.resize(nw, nh) {
            eprintln!("video: software resize failed ({e})");
            return;
        }
        let mut buffer = match surf.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("video: software buffer unavailable ({e})");
                return;
            }
        };
        let scale = ((w as f32) / vw).min((h as f32) / vh);
        let dw = (vw * scale) as u32;
        let dh = (vh * scale) as u32;
        let ox = w.saturating_sub(dw) / 2;
        let oy = h.saturating_sub(dh) / 2;
        let draw_w = dw.min(w) as usize;
        let draw_h = dh.min(h) as usize;
        let ox = ox as usize;
        let oy = oy as usize;
        // Match the previous nearest-neighbor mapping while doing its float
        // divisions once per output axis instead of once per output pixel.
        let source_x: Vec<usize> = (0..draw_w)
            .map(|x| ((vx0 + (x as f32) / scale) as usize).min(stride - 1))
            .collect();
        let source_y: Vec<usize> = (0..draw_h)
            .map(|y| ((vy0 + (y as f32) / scale) as usize).min(frame_h - 1))
            .collect();
        let row_len = w as usize;
        for (y, row) in buffer.chunks_exact_mut(row_len).enumerate() {
            let Some(local_y) = y.checked_sub(oy).filter(|&y| y < draw_h) else {
                row.fill(0);
                continue;
            };
            row[..ox].fill(0);
            let right = (ox + draw_w).min(row_len);
            row[right..].fill(0);
            let src = &self.rgba[source_y[local_y] * stride..][..stride];
            for (pixel, &sx) in row[ox..right].iter_mut().zip(&source_x) {
                let s = src[sx];
                *pixel = (s[0] as u32) << 16 | (s[1] as u32) << 8 | s[2] as u32;
            }
        }
        for (i, layout) in ui_layouts.into_iter().enumerate() {
            if let Some(layout) = layout {
                let ui = screen::ui_layer::Frame {
                    pixels: self.ui_motion.panels[i].pixels(),
                    raw_world: if i == 1
                        && self.machine.bus.ffta_dialogue.extra.kind
                            == gba_core::ffta_dialogue::PanelKind::Stats
                    {
                        &self.machine.bus.ffta_dialogue.extra.background
                    } else {
                        &self.machine.bus.wide_framebuffer
                    },
                    world_dims: (wide_w() as u32, wide_h() as u32),
                    lut: self.lut.table.as_slice(),
                    layout,
                    world_view: self.view,
                    world_rect: ui_world_rect,
                };
                let x0 = layout.rect[0].max(layout.clip[0]).max(0.) as usize;
                let y0 = layout.rect[1].max(layout.clip[1]).max(0.) as usize;
                let x1 = (layout.rect[0] + layout.rect[2]).min(w as f32).max(0.) as usize;
                let y1 = (layout.rect[1] + layout.rect[3]).min(h as f32).max(0.) as usize;
                for y in y0..y1 {
                    for x in x0..x1 {
                        if let Some(c) = ui.sample(x as f32 + 0.5, y as f32 + 0.5) {
                            buffer[y * w as usize + x] =
                                (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
                        }
                    }
                }
            }
        }
        if !self.paused && !save_ui {
            for i in 0..4 {
                let base = screen::ui_layer::Layout::centered(
                    (w, h),
                    (240 + 2 * self.base_extra) as f32,
                    1.,
                );
                let Some(layout) = self.battle_hud_motion.layout(i, base) else {
                    continue;
                };
                let ui = screen::ui_layer::Frame {
                    pixels: self.battle_hud_motion.pixels(i),
                    raw_world: &self.machine.bus.wide_framebuffer,
                    world_dims: (wide_w() as u32, wide_h() as u32),
                    lut: self.lut.table.as_slice(),
                    layout,
                    world_view: self.view,
                    world_rect: ui_world_rect,
                };
                let x0 = layout.clip[0].max(0.) as usize;
                let y0 = layout.clip[1].max(0.) as usize;
                let x1 = (layout.clip[0] + layout.clip[2]).min(w as f32).max(0.) as usize;
                let y1 = (layout.clip[1] + layout.clip[3]).min(h as f32).max(0.) as usize;
                for y in y0..y1 {
                    for x in x0..x1 {
                        if let Some(c) = ui.sample(x as f32 + 0.5, y as f32 + 0.5) {
                            buffer[y * w as usize + x] =
                                (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
                        }
                    }
                }
            }
        }
        let _ = buffer.present();
    }

    /// Gather this frame's down-state into the active-low GBA `keys` mask and
    /// the host-menu [`NavEdges`], from the held-key set plus the gamepad.
    fn gather_input(&mut self) -> (u16, NavEdges) {
        // KEYINPUT is active-low.
        let mut keys = 0x3FFu16;
        let mut pad_zoom_in = false;
        let mut pad_zoom_out = false;
        let mut pad_ff_held = false;
        let mut pad_ff_toggle = false;
        for (code, bit) in &self.key_pairs {
            if self.held.contains(code) {
                keys &= !bit;
            }
        }
        // Host-menu navigation down-states. On the keyboard Escape both opens
        // and backs out (it never aliases a GBA key).
        let mut nav = NavEdges {
            up: self.held.contains(&KeyCode::ArrowUp),
            down: self.held.contains(&KeyCode::ArrowDown),
            left: self.held.contains(&KeyCode::ArrowLeft),
            right: self.held.contains(&KeyCode::ArrowRight),
            confirm: self.held.contains(&KeyCode::Enter),
            cancel: self.held.contains(&KeyCode::Escape),
            open: winit_key(&self.icfg.menu_key).is_some_and(|k| self.held.contains(&k)),
        };
        let capture_pad = matches!(
            self.menu.capturing(),
            Some(
                menu::Bind::Gamepad(_)
                    | menu::Bind::GamepadZoomIn
                    | menu::Bind::GamepadZoomOut
                    | menu::Bind::GamepadFastForward
                    | menu::Bind::GamepadMenu
                    | menu::Bind::GamepadFastForwardToggle
            )
        );
        self.pending_pad = None;
        let mut standard_selected = false;
        let activity_threshold = self.icfg.stick_deadzone.max(0.12);
        #[cfg(windows)]
        let native_xinput = self.xinput.is_some();
        #[cfg(not(windows))]
        let native_xinput = false;
        let mut gilrs_activity = false;
        let mut pointer_pad_activity = false;
        let mut gilrs_active_name = None;
        let mut gilrs_active_layout = None;

        // Pump every gilrs device, not only the one selected when the game
        // started. Any deliberate button/axis activity makes that controller
        // the active one immediately; this also handles hot-swapping pads.
        if !native_xinput {
            if let Some(g) = self.pad.as_mut() {
                while let Some(event) = g.next_event() {
                    #[cfg(target_os = "linux")]
                    if matches!(event.event, gilrs::EventType::Disconnected) {
                        self.linux_pads.remove(&event.id);
                        if self.active_controller == Some(ActiveController::Gilrs(event.id)) {
                            self.active_controller = None;
                        }
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        let gesture = self.gilrs_gestures.entry(event.id).or_default();
                        gesture.arm_capture(capture_pad && !self.directinput_only);
                        for (token, pressed) in gilrs_control_transitions(event.event) {
                            if let Some(captured) = gesture.event(&token, pressed) {
                                if self.pending_pad.is_none() {
                                    self.pending_pad = Some(captured);
                                }
                            }
                        }
                        if matches!(event.event, gilrs::EventType::Disconnected) {
                            self.gilrs_gestures.remove(&event.id);
                        }
                        match event.event {
                            gilrs::EventType::Disconnected => {
                                if self.active_controller == Some(ActiveController::Gilrs(event.id))
                                {
                                    self.active_controller = None;
                                }
                            }
                            gilrs::EventType::ButtonPressed(button, _)
                            | gilrs::EventType::ButtonRepeated(button, _)
                                if button != gilrs::Button::Unknown =>
                            {
                                self.active_controller = Some(ActiveController::Gilrs(event.id));
                                gilrs_activity = true;
                                let gamepad = g.gamepad(event.id);
                                if gamepad.is_connected() {
                                    gilrs_active_name = Some(gamepad.name().to_owned());
                                    gilrs_active_layout =
                                        Some(input_config::GamepadLayout::detect_with_vendor(
                                            gamepad.name(),
                                            gamepad.vendor_id(),
                                        ));
                                }
                            }
                            gilrs::EventType::ButtonReleased(button, _)
                                if button != gilrs::Button::Unknown => {}
                            gilrs::EventType::ButtonChanged(button, value, _)
                                if button != gilrs::Button::Unknown
                                    && value >= activity_threshold =>
                            {
                                self.active_controller = Some(ActiveController::Gilrs(event.id));
                                gilrs_activity = true;
                                let gamepad = g.gamepad(event.id);
                                if gamepad.is_connected() {
                                    gilrs_active_name = Some(gamepad.name().to_owned());
                                    gilrs_active_layout =
                                        Some(input_config::GamepadLayout::detect_with_vendor(
                                            gamepad.name(),
                                            gamepad.vendor_id(),
                                        ));
                                }
                            }
                            gilrs::EventType::AxisChanged(_, value, _)
                                if value.abs() >= activity_threshold =>
                            {
                                self.active_controller = Some(ActiveController::Gilrs(event.id));
                                gilrs_activity = true;
                                let gamepad = g.gamepad(event.id);
                                if gamepad.is_connected() {
                                    gilrs_active_name = Some(gamepad.name().to_owned());
                                    gilrs_active_layout =
                                        Some(input_config::GamepadLayout::detect_with_vendor(
                                            gamepad.name(),
                                            gamepad.vendor_id(),
                                        ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                #[cfg(target_os = "linux")]
                {
                    // Events update Gilrs' cached device state only. Consume a
                    // single complete snapshot, never Pressed/Changed/Repeated
                    // variants independently for the same physical press.
                    let now = std::time::Instant::now();
                    let connected: Vec<_> = g.gamepads().map(|(id, _)| id).collect();
                    self.linux_pads.retain(|id, _| connected.contains(id));
                    for (id, gamepad) in g.gamepads() {
                        let pad = self.linux_pads.entry(id).or_default();
                        let snapshot = crate::linux_pad::Snapshot::read(
                            &gamepad,
                            self.icfg.stick_deadzone,
                            pad.state,
                        );
                        let update = pad.poll(snapshot, capture_pad, now);
                        if self.pending_pad.is_none() {
                            self.pending_pad = update.capture.map(str::to_owned);
                        }
                        if update.activity {
                            self.active_controller = Some(ActiveController::Gilrs(id));
                            gilrs_activity = true;
                            gilrs_active_name = Some(gamepad.name().to_owned());
                            gilrs_active_layout = Some(input_config::GamepadLayout::Xbox);
                        }
                    }
                }
            }
        }
        if !native_xinput {
            if let Some(name) = gilrs_active_name {
                let layout = gilrs_active_layout
                    .unwrap_or_else(|| input_config::GamepadLayout::detect(&name));
                if self.icfg.gamepad_name != name
                    || self.icfg.device != input_config::Device::Gamepad
                    || self.icfg.gamepad_layout != layout
                {
                    self.icfg.gamepad_name = name;
                    self.icfg.device = input_config::Device::Gamepad;
                    self.icfg.gamepad_layout = layout;
                    if let Err(error) = self.icfg.save() {
                        eprintln!("input: could not save active controller ({error})");
                    }
                }
            }
        }

        // DirectInput has no event queue, so poll each attached pad and use
        // whichever currently reports intentional activity. This avoids
        // getting stuck on a saved controller name after the user swaps pads.
        let direct_states = if native_xinput {
            Vec::new()
        } else {
            self.directinput
                .as_mut()
                .map(|direct| direct.poll_all())
                .unwrap_or_default()
        };
        let direct_activity = direct_states
            .iter()
            .find(|pad| pad.has_activity(activity_threshold))
            .map(|pad| (pad.name.clone(), pad.activity_tokens(activity_threshold)));
        if let Some((name, tokens)) = direct_activity.as_ref() {
            let new_activity =
                self.last_direct_activity.as_ref() != Some(&(name.clone(), tokens.clone()));
            if !gilrs_activity && new_activity {
                pointer_pad_activity = true;
                self.active_controller = Some(ActiveController::DirectInput);
                let layout = input_config::GamepadLayout::detect(name);
                if self.icfg.gamepad_name != *name
                    || self.icfg.device != input_config::Device::Gamepad
                    || self.icfg.gamepad_layout != layout
                {
                    self.icfg.gamepad_name = name.clone();
                    self.icfg.device = input_config::Device::Gamepad;
                    self.icfg.gamepad_layout = layout;
                    if let Err(error) = self.icfg.save() {
                        eprintln!("input: could not save active controller ({error})");
                    }
                }
            }
        }
        if self.last_direct_activity != direct_activity {
            self.last_direct_activity = direct_activity;
        }
        if direct_states.is_empty()
            && self.active_controller == Some(ActiveController::DirectInput)
            && !self.directinput_only
        {
            self.active_controller = None;
            self.last_direct_activity = None;
        }

        let direct_requested = native_xinput
            || self.directinput_only
            || match self.active_controller {
                Some(ActiveController::DirectInput) => true,
                Some(ActiveController::Gilrs(_)) => false,
                None => {
                    self.directinput.is_some()
                        && self.icfg.gamepad_name.starts_with("DirectInput:")
                        && !direct_states.is_empty()
                }
            };
        if direct_requested {
            // Only the selected backend can own a binding; never let a
            // parallel Gilrs event for the same physical pad win the race.
            self.pending_pad = None;
        }
        for pad in &direct_states {
            let gesture = self.direct_gestures.entry(pad.name.clone()).or_default();
            gesture.arm_capture(capture_pad && direct_requested);
            let captured = gesture.snapshot(pad.capture_tokens());
            if direct_requested && self.pending_pad.is_none() {
                self.pending_pad = captured;
            }
        }
        #[cfg(windows)]
        if native_xinput {
            let (current, previous, capture_locked) = {
                let backend = self.xinput.as_mut().expect("native XInput selected");
                let current = backend.poll();
                let previous = backend.previous;
                backend.previous = current;
                (current, previous, backend.capture_locked)
            };
            let mut activity_slot = None;
            for slot in 0..4 {
                if let Some(state) = current[slot] {
                    if !xinput_tokens(state, activity_threshold).is_empty()
                        && current[slot] != previous[slot]
                    {
                        activity_slot = Some(slot);
                    }
                }
            }
            let active_slot = activity_slot
                .or_else(|| {
                    self.icfg
                        .gamepad_name
                        .strip_prefix("XInput Controller ")
                        .and_then(|n| n.parse::<usize>().ok())
                        .and_then(|n| n.checked_sub(1))
                        .filter(|&n| n < 4 && current[n].is_some())
                })
                .or_else(|| current.iter().position(Option::is_some));
            pointer_pad_activity |= activity_slot.is_some();
            if let Some(slot) = activity_slot.or(active_slot) {
                let name = format!("XInput Controller {}", slot + 1);
                if self.icfg.gamepad_name != name
                    || self.icfg.device != input_config::Device::Gamepad
                    || self.icfg.gamepad_layout != input_config::GamepadLayout::Xbox
                {
                    self.icfg.gamepad_name = name;
                    self.icfg.device = input_config::Device::Gamepad;
                    self.icfg.gamepad_layout = input_config::GamepadLayout::Xbox;
                    if let Err(error) = self.icfg.save() {
                        eprintln!("input: could not save active XInput controller ({error})");
                    }
                }
            }
            let all_tokens: Vec<(usize, String)> = current
                .iter()
                .enumerate()
                .filter_map(|(slot, state)| state.map(|s| (slot, s)))
                .flat_map(|(slot, state)| {
                    xinput_tokens(state, activity_threshold)
                        .into_iter()
                        .map(move |token| (slot, token))
                })
                .collect();
            let any_down = !all_tokens.is_empty();
            let mut captured = None;
            if capture_pad && !capture_locked {
                for slot in 0..4 {
                    if let Some(state) = current[slot] {
                        if let Some(token) =
                            xinput_new_press(previous[slot], state, activity_threshold)
                        {
                            captured = Some(token);
                            break;
                        }
                    }
                }
            }
            if let Some(token) = captured {
                self.pending_pad = Some(token);
                if let Some(backend) = self.xinput.as_mut() {
                    backend.capture_locked = true;
                }
            } else if capture_locked && !any_down {
                if let Some(backend) = self.xinput.as_mut() {
                    backend.capture_locked = false;
                }
            }
            if let Some(slot) = active_slot {
                if let Some(state) = current[slot] {
                    let blocked =
                        capture_locked || self.xinput.as_ref().is_some_and(|x| x.capture_locked);
                    if !blocked {
                        let dz = self.icfg.stick_deadzone;
                        let pressed = |name: &str| xinput_pressed(state, name, dz);
                        pad_zoom_in = pressed(&self.icfg.zoom_in_pad);
                        pad_zoom_out = pressed(&self.icfg.zoom_out_pad);
                        pad_ff_held = pressed(&self.icfg.ff_pad);
                        pad_ff_toggle = pressed(&self.icfg.ff_toggle_pad);
                        for b in input_config::Button::ALL {
                            if pressed(&self.icfg.pads[b.index()]) {
                                keys &= !b.bit();
                            }
                        }
                        let src = self.icfg.dpad_source;
                        let (sx, sy) = if src.uses_right_stick() {
                            (state.gamepad.right_x, state.gamepad.right_y)
                        } else {
                            (state.gamepad.left_x, state.gamepad.left_y)
                        };
                        let dz_axis = (dz.clamp(0.0, 1.0) * i16::MAX as f32) as i16;
                        if (src.uses_left_stick() || src.uses_right_stick()) && sx > dz_axis {
                            keys &= !(1 << 4);
                        }
                        if (src.uses_left_stick() || src.uses_right_stick()) && sx < -dz_axis {
                            keys &= !(1 << 5);
                        }
                        if (src.uses_left_stick() || src.uses_right_stick()) && sy > dz_axis {
                            keys &= !(1 << 6);
                        }
                        if (src.uses_left_stick() || src.uses_right_stick()) && sy < -dz_axis {
                            keys &= !(1 << 7);
                        }
                        nav.up |= pressed("DPadUp") || state.gamepad.left_y > dz_axis;
                        nav.down |= pressed("DPadDown") || state.gamepad.left_y < -dz_axis;
                        nav.left |= pressed("DPadLeft") || state.gamepad.left_x < -dz_axis;
                        nav.right |= pressed("DPadRight") || state.gamepad.left_x > dz_axis;
                        nav.confirm |= pressed("South");
                        nav.cancel |= pressed("East");
                        nav.open |=
                            pressed(&self.icfg.menu_pad) || (pressed("Start") && pressed("Select"));
                    }
                }
            }
        }
        if let Some(g) = self.pad.as_mut().filter(|_| !direct_requested) {
            let active = match self.active_controller {
                Some(ActiveController::Gilrs(id)) => {
                    let gamepad = g.gamepad(id);
                    gamepad.is_connected().then_some((id, gamepad))
                }
                _ => None,
            };
            let chosen = active
                .or_else(|| {
                    g.gamepads()
                        .find(|(_, gp)| gp.name() == self.icfg.gamepad_name)
                })
                .or_else(|| g.gamepads().next());
            if let Some((id, gp)) = chosen {
                standard_selected = true;
                #[cfg(not(target_os = "linux"))]
                let pad_blocked = self
                    .gilrs_gestures
                    .get(&id)
                    .is_some_and(input_config::PadGesture::blocked);
                #[cfg(target_os = "linux")]
                let pad_blocked = self
                    .linux_pads
                    .get(&id)
                    .is_some_and(crate::linux_pad::Pad::blocked);
                #[cfg(not(target_os = "linux"))]
                let dz = self.icfg.stick_deadzone;
                if !pad_blocked {
                    #[cfg(not(target_os = "linux"))]
                    let pressed = |name: &str| pad_pressed(&gp, name, dz);
                    #[cfg(target_os = "linux")]
                    let pressed = |name: &str| {
                        self.linux_pads
                            .get(&id)
                            .is_some_and(|pad| pad.state.pressed(name))
                    };
                    pad_zoom_in = pressed(&self.icfg.zoom_in_pad);
                    pad_zoom_out = pressed(&self.icfg.zoom_out_pad);
                    pad_ff_held = pressed(&self.icfg.ff_pad);
                    pad_ff_toggle = pressed(&self.icfg.ff_toggle_pad);
                    for b in input_config::Button::ALL {
                        if pressed(&self.icfg.pads[b.index()]) {
                            keys &= !b.bit();
                        }
                    }
                    // A configured stick drives the GBA d-pad per dpad_source (the
                    // physical d-pad still works through the bindings above). Y is
                    // up-positive in gilrs.
                    let src = self.icfg.dpad_source;
                    if src.uses_left_stick() || src.uses_right_stick() {
                        #[cfg(target_os = "linux")]
                        {
                            let directions = if src.uses_right_stick() {
                                [
                                    ("RightStickRight", 4),
                                    ("RightStickLeft", 5),
                                    ("RightStickUp", 6),
                                    ("RightStickDown", 7),
                                ]
                            } else {
                                [
                                    ("LeftStickRight", 4),
                                    ("LeftStickLeft", 5),
                                    ("LeftStickUp", 6),
                                    ("LeftStickDown", 7),
                                ]
                            };
                            for (token, bit) in directions {
                                if pressed(token) {
                                    keys &= !(1 << bit);
                                }
                            }
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            let (ax, ay) = if src.uses_right_stick() {
                                (gilrs::Axis::RightStickX, gilrs::Axis::RightStickY)
                            } else {
                                (gilrs::Axis::LeftStickX, gilrs::Axis::LeftStickY)
                            };
                            let (x, y) = (gp.value(ax), gp.value(ay));
                            if x > dz {
                                keys &= !(1 << 4);
                            } // Right
                            if x < -dz {
                                keys &= !(1 << 5);
                            } // Left
                            if y > dz {
                                keys &= !(1 << 6);
                            } // Up
                            if y < -dz {
                                keys &= !(1 << 7);
                            } // Down
                        }
                    }
                    // Menu nav from the pad: d-pad and sticks move the cursor;
                    // South confirms, East backs out. These steer the menu only
                    // while it is open, so reusing the gameplay face buttons here
                    // is safe.
                    #[cfg(not(target_os = "linux"))]
                    use gilrs::{Axis, Button as Gb};
                    #[cfg(not(target_os = "linux"))]
                    let pad_nav = NavEdges {
                        up: gp.is_pressed(Gb::DPadUp) || gp.value(Axis::LeftStickY) > dz,
                        down: gp.is_pressed(Gb::DPadDown) || gp.value(Axis::LeftStickY) < -dz,
                        left: gp.is_pressed(Gb::DPadLeft) || gp.value(Axis::LeftStickX) < -dz,
                        right: gp.is_pressed(Gb::DPadRight) || gp.value(Axis::LeftStickX) > dz,
                        confirm: gp.is_pressed(Gb::South),
                        cancel: gp.is_pressed(Gb::East),
                        open: super::gilrs_button(&self.icfg.menu_pad)
                            .is_some_and(|b| gp.is_pressed(b)),
                    };
                    #[cfg(target_os = "linux")]
                    let pad_nav = NavEdges {
                        up: pressed("DPadUp") || pressed("LeftStickUp"),
                        down: pressed("DPadDown") || pressed("LeftStickDown"),
                        left: pressed("DPadLeft") || pressed("LeftStickLeft"),
                        right: pressed("DPadRight") || pressed("LeftStickRight"),
                        confirm: pressed("South"),
                        cancel: pressed("East"),
                        // The Steam Deck package adds Y (Gilrs North); the
                        // standard Linux binary keeps its existing shortcuts.
                        open: linux_menu_shortcut(
                            pressed(&self.icfg.menu_pad),
                            pressed("Start"),
                            pressed("Select"),
                            pressed("North"),
                        ),
                    };
                    nav.up |= pad_nav.up;
                    nav.down |= pad_nav.down;
                    nav.left |= pad_nav.left;
                    nav.right |= pad_nav.right;
                    nav.confirm |= pad_nav.confirm;
                    nav.cancel |= pad_nav.cancel;
                    // Linux also accepts Start+Back when the desktop/Steam
                    // intercepts Guide. Keyboard Escape remains available.
                    nav.open |= pad_nav.open;
                }
            }
        }
        if !standard_selected {
            let active = direct_states
                .iter()
                .find(|pad| pad.has_activity(activity_threshold));
            let preferred = direct_states
                .iter()
                .find(|pad| pad.name == self.icfg.gamepad_name);
            if let Some(gp) = active.or(preferred).or_else(|| direct_states.first()) {
                let pad_blocked = self
                    .direct_gestures
                    .get(&gp.name)
                    .is_some_and(input_config::PadGesture::blocked);
                let dz = self.icfg.stick_deadzone;
                if !pad_blocked {
                    pad_zoom_in = gp.pressed(&self.icfg.zoom_in_pad, dz);
                    pad_zoom_out = gp.pressed(&self.icfg.zoom_out_pad, dz);
                    pad_ff_held = gp.pressed(&self.icfg.ff_pad, dz);
                    pad_ff_toggle = gp.pressed(&self.icfg.ff_toggle_pad, dz);
                    for button in input_config::Button::ALL {
                        let pressed = gp.active(
                            &self.icfg.pads[button.index()],
                            self.icfg.dpad_source,
                            dz,
                            button,
                        );
                        if pressed {
                            keys &= !button.bit();
                        }
                    }
                    let pad_nav = NavEdges {
                        up: gp.pressed("DPadUp", dz) || gp.axes[1] > dz,
                        down: gp.pressed("DPadDown", dz) || gp.axes[1] < -dz,
                        left: gp.pressed("DPadLeft", dz) || gp.axes[0] < -dz,
                        right: gp.pressed("DPadRight", dz) || gp.axes[0] > dz,
                        confirm: gp.pressed("South", dz),
                        cancel: gp.pressed("East", dz),
                        open: gp.pressed(&self.icfg.menu_pad, dz),
                    };
                    nav.up |= pad_nav.up;
                    nav.down |= pad_nav.down;
                    nav.left |= pad_nav.left;
                    nav.right |= pad_nav.right;
                    nav.confirm |= pad_nav.confirm;
                    nav.cancel |= pad_nav.cancel;
                    nav.open |= pad_nav.open;
                }
            }
        }
        if gilrs_activity || pointer_pad_activity {
            gba_core::ffta_battle_mouse::set_mouse_visual(&mut self.machine.bus, false);
        }
        if capture_pad {
            // Remapping is an exclusive input mode. Do not let the physical
            // button being captured also control the game or host zoom.
            // Keep nav's actual held state so nav_prev is primed; clearing it
            // here makes a still-held confirm button look newly pressed on
            // the first frame after capture ends.
            keys = 0x3FF;
            pad_zoom_in = false;
            pad_zoom_out = false;
            pad_ff_held = false;
            pad_ff_toggle = false;
        }
        if !self.paused && pad_zoom_in && !self.pad_zoom_prev.0 {
            self.zoom_by(-1);
        }
        if !self.paused && pad_zoom_out && !self.pad_zoom_prev.1
        {
            self.zoom_by(1);
        }
        self.pad_zoom_prev = (pad_zoom_in, pad_zoom_out);
        if !self.paused

            && pad_ff_toggle
            && !self.pad_ff_toggle_prev
        {
            self.ff_latched = !self.ff_latched;
        }
        self.pad_ff_toggle_prev = pad_ff_toggle;
        self.pad_ff_held = pad_ff_held;
        (keys, nav)
    }

    /// Rebuild the key -> GBA button table after the menu rebinds a key.
    fn rebuild_key_pairs(&mut self) {
        let defaults = input_config::InputConfig::default();
        let pairs: Vec<(KeyCode, u16)> = input_config::Button::ALL
            .iter()
            .map(|b| {
                let key = winit_key(&self.icfg.active_keys()[b.index()])
                    .unwrap_or_else(|| winit_key(&defaults.keys[b.index()]).unwrap());
                (key, b.bit())
            })
            .collect();
        self.key_pairs = pairs;
    }

    /// Change fast-forward activation or audio speed. Flush stale audio on
    /// transitions and re-prime when returning to normal play.
    fn set_fast_forward(&mut self, on: bool, speed: f32) {
        self.ff_budget.reset();
        self.ff_active = on;
        self.ff_audio_active = self.av.ff_audio.clone();
        {
            let mut st = self.streams.lock().unwrap();
            st.muted = on && self.av.ff_audio == "silent";
            st.playback_speed = if on && self.av.ff_audio == "fast" {
                speed as f64
            } else {
                1.0
            };
            st.normal_audio = None;
            if on && self.av.ff_audio == "normal" {
                match super::ff_audio::NormalAudio::new() {
                    Ok(queue) => st.normal_audio = Some(queue),
                    Err(error) => {
                        eprintln!(
                            "audio: cannot create normal-speed backlog ({error}); muting fast-forward audio"
                        );
                        st.muted = true;
                    }
                }
            }
            st.mixed.clear();
            st.psg.clear();
            st.hle.clear();
            st.fifo[0].clear();
            st.fifo[1].clear();
            st.drops = 0;
            st.holds = 0;
            st.clipped = 0;
        }
        self.primed = false;
        self.audio_watch = Instant::now();
        self.audio_stalls = 0;
        self.next_frame = Instant::now();
    }

    /// One emulated frame plus its audio handoff to the rings. Counts native
    /// vs fallback dispatch, advances the screen's temporal response, surfaces
    /// the realtime / audio-defect alarms, and flushes backup media.
    fn run_one_frame(&mut self) {
        let ui_started = self.ui_timing.start();
        if self.autosave.pending_load {
            self.load_auto_state();
        }
        self.autosave.route_entry(&mut self.machine);
        // Apply a pending zoom change before the frame is emulated, so the
        // core renders (and we read back) the size the zoom needs.
        self.sync_view();
        gba_core::ffta_roster_mouse::publish_party_click(&mut self.machine);
        self.autosave
            .capture_before_menu(&self.machine, self.game_keys);
        let keys = self.game_mouse.keys(&mut self.machine.bus, self.game_keys);
        let keys = gba_core::ffta_battle_mouse::keys(&mut self.machine.bus, keys);
        self.machine.bus.keys = keys;
        if let Some(index) = self.machine.bus.ffta_battle_mouse.take_event() {
            if let Some((file, _)) = &mut self.record {
                use std::io::Write;
                let _ = writeln!(file, "{} mouse:{index}", self.machine.bus.frames);
            }
        }
        if let Some(index) = self.game_mouse.take_selection_event() {
            if let Some((file, _)) = &mut self.record {
                use std::io::Write;
                let _ = writeln!(file, "{} mouse:{index}", self.machine.bus.frames);
            }
        }
        if let Some((file, last)) = &mut self.record {
            if keys != *last {
                use std::io::Write;
                let _ = writeln!(file, "{} {keys:03x}", self.machine.bus.frames);
                *last = keys;
            }
        }
        let emu_t0 = Instant::now();
        gba_core::ffta_debug::tick(&mut self.machine.bus);
        match &self.native {
            Some((_lib, table)) => {
                let mptr = &mut self.machine as *mut Machine as *mut std::ffi::c_void;
                let (n, f) = run_frame_native(&mut self.machine, table, mptr, 5_000_000);
                self.native_run += n;
                self.fallback_run += f;
            }
            None => self.machine.run_frame(5_000_000),
        }
        self.autosave.route_entry(&mut self.machine);
        let ui_emulated = self.ui_timing.start();
        gba_core::ffta_battle_hud::capture_layers(&self.machine.bus, &mut self.battle_hud_pixels);
        self.battle_hud_motion
            .sync(&self.machine.bus, &self.battle_hud_pixels);
        gba_core::ffta_sprites::trace_frame(&self.machine.bus);
        if let Some((pixels, _)) = self.machine.bus.ffta_dialogue.layer() {
            self.ui_pixels.resize(pixels.len(), [0; 4]);
            let state = &self.machine.bus.ffta_dialogue;
            if state.action_layout {
                self.ui_pack_cache[0].update_with_bounds(
                    pixels,
                    &state.backing,
                    if state.action_banner.is_some() {
                        1.
                    } else {
                        state.style.length
                    },
                    false,
                    Some([0, 48, 160, 160]),
                    &mut self.ui_pixels,
                );
            } else if state.town_layout || state.shop_layout {
                self.ui_pack_cache[0].update(pixels, &[], 1., true, &mut self.ui_pixels);
            } else {
                self.ui_pack_cache[0].update(pixels, &[], 1., true, &mut self.ui_pixels);
            }
        } else {
            self.ui_pixels.clear();
            self.ui_pack_cache[0].invalidate();
        }
        let extra = &self.machine.bus.ffta_dialogue.extra;
        if self.machine.bus.ffta_dialogue.separate && extra.active() {
            self.panel_pixels.resize(extra.pixels.len(), [0; 4]);
            let b = self
                .machine
                .bus
                .ffta_dialogue
                .stats_target
                .or(extra.bounds)
                .or(self.ui_pack_cache[1].bounds)
                .unwrap_or([0; 4]);
            let state = &self.machine.bus.ffta_dialogue;
            let geometry = gba_core::ffta_ui::Scene::capture(state).nodes[1]
                .map(|n| n.bounds)
                .or(state.stats_target.or(extra.bounds));
            self.ui_pack_cache[1].update_with_bounds(
                &extra.pixels,
                &extra.backing,
                if state.town_layout || extra.kind == gba_core::ffta_dialogue::PanelKind::Notice {
                    1.
                } else {
                    state.style.length
                },
                state.town_layout || b[0] + b[2] >= 240,
                geometry,
                &mut self.panel_pixels,
            );
        } else {
            self.panel_pixels.clear();
            self.ui_pack_cache[1].invalidate();
        }
        self.ui_scene = gba_core::ffta_ui::Scene::capture_with_bounds(
            &self.machine.bus.ffta_dialogue,
            [self.ui_pack_cache[0].bounds, self.ui_pack_cache[1].bounds],
        );
        let mut auxiliary_used = [false; 2];
        let target = &self.machine.bus.ffta_dialogue.battle_stats;
        if target.active() {
            if let Some(bounds) = self
                .machine
                .bus
                .ffta_dialogue
                .stats_target
                .or(target.bounds)
            {
                let right = self.machine.bus.ffta_dialogue.stats_right;
                self.ui_pack_cache[2].update_with_bounds(
                    &target.pixels,
                    &target.backing,
                    self.machine.bus.ffta_dialogue.style.length,
                    right,
                    Some(bounds),
                    &mut self.ui_auxiliary[0],
                );
                auxiliary_used[0] = true;
                self.ui_scene.nodes[2] = Some(gba_core::ffta_ui::Node {
                    slot: 2,
                    role: gba_core::ffta_ui::Role::Stats,
                    anchor: gba_core::ffta_ui::Anchor::Corner {
                        right,
                        bottom: true,
                    },
                    bounds,
                    length: true,
                });
            }
        }
        if let Some(bounds) = self.machine.bus.ffta_dialogue.funds_bounds {
            self.ui_pack_cache[2].update_with_bounds(
                &self.machine.bus.ffta_dialogue.funds_pixels,
                &[],
                1.,
                true,
                Some(bounds),
                &mut self.ui_auxiliary[0],
            );
            auxiliary_used[0] = true;
            self.ui_scene.nodes[2] = Some(gba_core::ffta_ui::Node {
                slot: 2,
                role: gba_core::ffta_ui::Role::Funds,
                anchor: gba_core::ffta_ui::Anchor::Edge {
                    right: true,
                    vertical: 160.,
                },
                bounds,
                length: false,
            });
        }
        let speech = &self.machine.bus.ffta_dialogue.service_speech;
        if speech.active() {
            if let Some(bounds) = speech.bounds {
                self.ui_pack_cache[3].update_with_bounds(
                    &speech.pixels,
                    &[],
                    1.,
                    false,
                    Some(bounds),
                    &mut self.ui_auxiliary[1],
                );
                auxiliary_used[1] = true;
                self.ui_scene.nodes[3] = Some(gba_core::ffta_ui::Node {
                    slot: 3,
                    role: gba_core::ffta_ui::Role::Dialogue,
                    anchor: gba_core::ffta_ui::Anchor::Edge {
                        right: self.machine.bus.ffta_dialogue.service_speech_right,
                        vertical: if bounds[1] + bounds[3] >= 160 {
                            160.
                        } else {
                            0.
                        },
                    },
                    bounds,
                    length: false,
                });
            }
        }
        if let Some(bounds) = self.machine.bus.ffta_dialogue.action_banner {
            extract_ui_rectangle(&mut self.ui_pixels, &mut self.ui_auxiliary[1], bounds);
            self.ui_pack_cache[3].invalidate();
            auxiliary_used[1] = true;
            self.ui_scene.nodes[3] = Some(gba_core::ffta_ui::Node {
                slot: 3,
                role: gba_core::ffta_ui::Role::AttackName,
                anchor: gba_core::ffta_ui::Anchor::TopCenter,
                bounds,
                length: false,
            });
        }
        for (i, used) in auxiliary_used.into_iter().enumerate() {
            if !used {
                self.ui_auxiliary[i].clear();
                self.ui_pack_cache[i + 2].invalidate();
            }
        }
        if self.machine.bus.ffta_dialogue.action_banner.is_some() {
            self.ui_pack_cache[0].invalidate();
        }
        if self.pub_scrollbar_slot() == 1 && self.ui_scene.nodes[1].is_some() {
            if let Some(bar) = self.pub_scrollbar() {
                bar.paint_over(&mut self.panel_pixels);
                self.ui_pack_cache[1].invalidate();
            }
        } else if self.ui_scene.nodes[1].is_none() {
            if let Some(bar) = self.pub_scrollbar() {
                bar.paint(&mut self.panel_pixels);
                let mut node = self.ui_scene.nodes[0].unwrap();
                node.slot = 1;
                self.ui_scene.nodes[1] = Some(node);
            }
        }
        self.ui_motion.sync(
            &self.machine.bus,
            self.ui_scene.nodes,
            [
                &self.ui_pixels,
                &self.panel_pixels,
                &self.ui_auxiliary[0],
                &self.ui_auxiliary[1],
            ],
        );
        self.ui_inspector.observe(&self.machine.bus, &self.ui_scene);
        visual_trace::capture(
            &self.machine.bus,
            &self.ui_scene,
            self.mouse_pos,
            self.view,
            false,
        );
        self.ui_timing.prepared(
            ui_started,
            ui_emulated,
            self.machine.bus.frames,
            self.ui_scene.nodes.iter().flatten().count(),
        );
        // Advance the screen's temporal response once per EMULATED frame —
        // never per presented frame, or flicker-transparency titles lock onto
        // one phase (the classic parity bug).
        let bar = self.roster_scrollbar();
        let wf = display_frame(&self.machine.bus, &mut self.padded_frame);
        let wf = if self.autosave.screen_active(&self.machine.bus) {
            crate::autosave::paint(
                &self.machine.bus,
                wf,
                &mut self.autosave_frame,
                &self.autosave,
            );
            &self.autosave_frame[..]
        } else {
            wf
        };

        if let Some(bar) = bar {
            let mut pixels = wf.to_vec();
            for (rect, color) in [
                (bar.track, 2u16 | (5 << 5) | (13 << 10)),
                (bar.thumb, 10u16 | (24 << 5) | (31 << 10)),
            ] {
                for y in rect[1].max(0)..rect[3].min(160) {
                    for x in rect[0].max(0)..rect[2].min(240) {
                        let p = (y as usize + gba_core::mem::extra_y()) * wide_w()
                            + x as usize
                            + gba_core::mem::extra();
                        if let Some(pixel) = pixels.get_mut(p) {
                            *pixel = color;
                        }
                    }
                }
            }
            self.temporal.push(&pixels, &self.lut);
        } else {
            self.temporal.push(wf, &self.lut);
        }

        // Realtime alarm: the frame budget is 16.7 ms. If smoothed emulation
        // cost approaches it, the product promise is broken — say so once.
        let dt_ms = emu_t0.elapsed().as_secs_f64() * 1e3;
        self.emu_ema_ms = if self.frames_run == 0 {
            dt_ms
        } else {
            self.emu_ema_ms * 0.95 + dt_ms * 0.05
        };
        self.frames_run += 1;
        if !self.slow_warned && self.frames_run > 120 && self.emu_ema_ms > 15.0 {
            self.slow_warned = true;
            eprintln!(
                "DEGRADED: emulation averaging {:.1} ms/frame (budget 16.7 ms) — below native speed",
                self.emu_ema_ms
            );
        }

        // Live perf readout in the title, once a second. Developer tooling —
        // hidden from the release out-of-box experience unless requested.
        if self.show_stats && self.frames_run % 60 == 0 {
            let total = self.native_run + self.fallback_run;
            let share = if total == 0 {
                0.0
            } else {
                self.native_run as f64 * 100.0 / total as f64
            };
            if let Some(w) = self.window.as_ref() {
                w.set_title(&format!(
                    "{} · {:.2} ms/frame ({:.1}x headroom) · native {share:.0}%",
                    self.title,
                    self.emu_ema_ms,
                    16.7 / self.emu_ema_ms.max(0.01),
                ));
            }
            self.native_run = 0;
            self.fallback_run = 0;
        }

        // Audio defect surfacing: ring drops and underrun holds are silent
        // corruption — say so, with numbers, per the product bar.
        if self.frames_run % 300 == 0 {
            let (d, h, c) = {
                let mut st = self.streams.lock().unwrap();
                let v = (st.drops, st.holds, st.clipped);
                st.drops = 0;
                st.holds = 0;
                st.clipped = 0;
                v
            };
            if d + h > 0 && !self.ff_active {
                eprintln!(
                    "DEGRADED: audio ring dropped {d} / held {h} sample pairs in the last 5 s"
                );
            }
            // Soft-knee engagement is canon clipping handled better, not a
            // defect — visible only in developer stats.
            if self.show_stats && c > 0 {
                eprintln!("audio: soft clip engaged on {c} sample pairs in the last 5 s");
            }
        }

        // New state saves own persistence; never recreate the archived .sav.
        let auto_eligible = self.autosave.eligible(&self.machine.bus);
        self.autosave.tick(
            &self.machine,
            auto_eligible && !self.paused && !self.machine.bus.skip_render,
        );
        self.handoff_audio();
    }

    /// Hand this frame's audio taps to the consumer rings, refresh the routing
    /// snapshot, and count anything dropped. With no consumer, the taps are
    /// discarded (not a defect — reported when the device died) so the rings
    /// can't overflow into permanent DEGRADED spam.
    fn handoff_audio(&mut self) {
        let bus = &mut self.machine.bus;
        if self.stream.is_none() || (self.ff_active && self.av.ff_audio == "silent") {
            bus.audio_buf.clear();
            bus.psg_tap.clear();
            bus.hle_tap.clear();
            bus.fifo_tap[0].clear();
            bus.fifo_tap[1].clear();
            return;
        }
        if self.ff_active && self.av.ff_audio == "normal" {
            let mut st = self.streams.lock().unwrap();
            if let Some(queue) = st.normal_audio.as_mut() {
                if let Err(error) = queue.push(&bus.audio_buf) {
                    eprintln!(
                        "audio: normal-speed backlog write failed ({error}); muting fast-forward audio"
                    );
                    st.normal_audio = None;
                    st.muted = true;
                }
            }
            bus.audio_buf.clear();
            bus.psg_tap.clear();
            bus.hle_tap.clear();
            bus.fifo_tap[0].clear();
            bus.fifo_tap[1].clear();
            return;
        }
        let cnt_h = bus.read16(0x0400_0082);
        let mut st = self.streams.lock().unwrap();
        let audio_capacity = 65536 * st.playback_speed.max(1.0) as usize;
        st.route = [(cnt_h >> 8 & 3) as u8, (cnt_h >> 12 & 3) as u8];
        st.vol_half = [cnt_h & 4 == 0, cnt_h & 8 == 0];
        // Faithful ring: always produced (the menu may crossfade to it at any
        // moment), so it must always be primed.
        for pair in bus.audio_buf.chunks_exact(2) {
            if st.mixed.len() < audio_capacity {
                st.mixed.push_back(pair[0]);
                st.mixed.push_back(pair[1]);
            } else {
                st.drops += 1;
            }
        }
        bus.audio_buf.clear();
        // Enhanced rings: also always produced (tap_channels is on), so the
        // enhanced path stays warm for a live switch.
        for pair in bus.psg_tap.chunks_exact(2) {
            if st.psg.len() < audio_capacity {
                st.psg.push_back(pair[0] as f32 / 32768.0);
                st.psg.push_back(pair[1] as f32 / 32768.0);
            } else {
                st.drops += 1;
            }
        }
        bus.psg_tap.clear();
        // MP2K/engine HLE stream and liveness. A pause is a diagnostic event
        // (the crossfade keeps it inaudible) — say so, with the reason, but
        // don't let a hopeless variant spam the log forever.
        let live = bus.mp2k.as_deref().is_some_and(|h| h.live())
            || bus.gax.as_deref().is_some_and(|g| g.live())
            || bus.rdrv.as_deref().is_some_and(|r| r.live());
        let pause_msg = if let Some(h) = bus.mp2k.as_deref_mut() {
            h.vf.reverted.take().map(|m| (m, h.vf.pauses))
        } else if let Some(g) = bus.gax.as_deref_mut() {
            g.vf.reverted.take().map(|m| (m, g.vf.pauses))
        } else if let Some(r) = bus.rdrv.as_deref_mut() {
            r.vf.reverted.take().map(|m| (m, r.vf.pauses))
        } else {
            None
        };
        if let Some((msg, pauses)) = pause_msg {
            if pauses <= 3 {
                eprintln!("DEGRADED: engine HLE paused (probing continues): {msg}");
            } else if pauses == 4 {
                eprintln!("DEGRADED: engine HLE paused again: {msg} (further pauses unlogged)");
            }
        }
        // Feed the HLE ring only while live — an unproven stream must neither
        // pile up (drop spam) nor leave stale audio for the switchover.
        if live {
            for pair in bus.hle_tap.chunks_exact(2) {
                if st.hle.len() < audio_capacity {
                    st.hle.push_back(pair[0]);
                    st.hle.push_back(pair[1]);
                } else {
                    st.drops += 1;
                }
            }
        } else {
            st.hle.clear();
        }
        bus.hle_tap.clear();
        // Switch over only once the HLE ring is primed; the callback
        // crossfades both directions, and the per-channel path stays warm.
        if !live {
            st.hle_on = false;
        } else if st.hle.len() / 2 >= RING_TARGET / 2 {
            st.hle_on = true;
        }
        for f in 0..2 {
            let events = std::mem::take(&mut bus.fifo_tap[f]);
            for e in events {
                if st.fifo[f].len() < audio_capacity / 2 {
                    st.fifo[f].push_back(e);
                } else {
                    st.drops += 1;
                }
            }
        }
    }

    /// The audio-device watchdog (runs on the outer tick, not the production
    /// loop — a dead consumer leaves the ring full and the production loop
    /// never executes, which is exactly the case this must catch).
    fn audio_watchdog(&mut self) {
        if self.stream.is_some() && self.audio_watch.elapsed().as_secs() >= 1 {
            self.audio_watch = Instant::now();
            let (consumed, dead) = {
                let st = self.streams.lock().unwrap();
                (st.consumed, st.dead)
            };
            if dead || consumed == self.audio_consumed_last {
                self.audio_stalls += 1;
            } else {
                self.audio_stalls = 0;
            }
            self.audio_consumed_last = consumed;
            if dead || self.audio_stalls >= 2 {
                eprintln!(
                    "DEGRADED: audio device {} — wall-clock pacing, retrying every 5 s",
                    if dead { "errored" } else { "stopped consuming" }
                );
                self.stream = None;
                {
                    let mut st = self.streams.lock().unwrap();
                    st.dead = false;
                    st.mixed.clear();
                    st.psg.clear();
                    st.hle.clear();
                    st.fifo[0].clear();
                    st.fifo[1].clear();
                }
                self.audio_stalls = 0;
                self.audio_retry = Some(Instant::now());
                self.next_frame = Instant::now();
            }
        } else if self.stream.is_none() {
            if let Some(t) = self.audio_retry {
                if t.elapsed().as_secs() >= 5 {
                    self.audio_retry = Some(Instant::now());
                    if let Some(s) = start_audio(self.streams.clone()) {
                        eprintln!("audio: output device restored");
                        self.stream = Some(s);
                        self.primed = false;
                        self.audio_consumed_last = 0;
                        self.audio_stalls = 0;
                        reset_audio_counters(&self.streams);
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for PlayApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create the window + presenter exactly once. `resumed` can fire again
        // on some platforms (e.g. Android lifecycle); on desktop, once.
        if self.window.is_some() {
            return;
        }
        // Self-paced: poll continuously and produce frames as the audio ring
        // needs them. Never Wait (would block the emulator on an OS event).
        event_loop.set_control_flow(ControlFlow::Poll);
        let attrs = Window::default_attributes()
            .with_title(self.title.clone())
            .with_window_icon(load_window_icon())
            .with_inner_size(LogicalSize::new(self.base_w() as f64 * 3.0, 160.0 * 3.0))
            .with_resizable(true)
            .with_fullscreen(if self.av.video_fullscreen {
                Some(Fullscreen::Borderless(None))
            } else {
                None
            });
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.error = Some(format!("window: {e}"));
                event_loop.exit();
                return;
            }
        };
        if let Ok(image) = image::load_from_memory(include_bytes!("../../../assets/cursor.png")) {
            let image = image.into_rgba8();
            if let Ok(source) =
                winit::window::CustomCursor::from_rgba(image.into_raw(), 32, 40, 2, 2)
            {
                window.set_cursor(event_loop.create_custom_cursor(source));
            }
        }
        // The texture is allocated for the largest frame; each present uploads
        // only the current (zoom-dependent) frame size and crop.
        self.presenter = match screen::present::Presenter::new(
            window.clone(),
            (240 + 2 * gba_core::mem::MAX_EXTRA) as u32,
            (160 + 2 * gba_core::mem::MAX_EXTRA_Y) as u32,
            self.display_target,
            self.av.video_vsync,
        ) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("video: GPU presenter unavailable ({e}); CPU scaling active");
                None
            }
        };
        self.window = Some(window);
        // The window now exists: tell a supervising frontend we're live.
        if self.status {
            status_line("playing");
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        self.menu_dirty = true;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            // Lost focus: a key held during alt-tab might never deliver its
            // release, so drop the held set defensively (no stuck inputs).
            WindowEvent::Focused(false) => {
                self.held.clear();
                self.game_keys = 0x3ff;
                self.game_mouse.cancel();
                self.machine.bus.ffta_battle_mouse.cancel();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat,
                        text,
                        ..
                    },
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        if self
                            .key_pairs
                            .iter()
                            .any(|(key, bit)| *key == code && bit & 0xf0 != 0)
                        {
                            gba_core::ffta_battle_mouse::set_mouse_visual(
                                &mut self.machine.bus,
                                false,
                            );
                        }
                        // One toggle per physical press. This is presentation-only and never pauses.
                        if code == KeyCode::F9
                            && !repeat
                            && std::env::var("FFTA_DEBUG").as_deref() == Ok("1")
                        {
                            visual_trace::toggle();
                            return;
                        }
                        if code == KeyCode::F8 && std::env::var("FFTA_DEBUG").as_deref() == Ok("1")
                        {
                            if !repeat
                                && self.held.insert(code)
                                && gba_core::ffta_rom::version(&self.machine.bus.rom)
                                    == Some(gba_core::ffta_rom::Version::Europe)
                            {
                                self.paused = true;

                                self.game_mouse.cancel();
                                self.machine.bus.ffta_battle_mouse.cancel();
                                self.machine.bus.keys = 0x3ff;
                                self.menu.open_debug();

                                self.menu_dirty = true;
                                self.mouse_queue.clear();
                                self.pending_key = None;
                                self.pending_pad = None;
                                reset_audio_counters(&self.streams);
                            }
                            return;
                        }

                        if !self.paused && self.game_mouse.is_name_input(&self.machine.bus) {
                            let consumed = match code {
                                KeyCode::Backspace => {
                                    self.game_mouse.name_command(&self.machine.bus, false)
                                }
                                KeyCode::Enter | KeyCode::NumpadEnter => {
                                    self.game_mouse.name_command(&self.machine.bus, true)
                                }
                                _ => text.as_ref().is_some_and(|t| {
                                    self.game_mouse.type_text(&self.machine.bus, t.as_str())
                                }),
                            };
                            if consumed {
                                self.held.remove(&code);
                                return;
                            }
                        }
                        self.held.insert(code);
                        if !self.paused
                            && !repeat
                            && winit_key(&self.icfg.ff_toggle_key) == Some(code)
                        {
                            self.ff_latched = !self.ff_latched;
                        }
                        if self.paused && !repeat {
                            self.pending_key = Some(code);
                        }
                        // F11 toggles fullscreen (the OS's own fullscreen
                        // affordance — macOS green button, Linux/Windows compositor
                        // — also works under winit). Act on the initial press, not
                        // auto-repeat.
                        if code == KeyCode::F11 && !repeat && self.menu.capturing().is_none() {
                            if let Some(w) = self.window.as_ref() {
                                let next = if w.fullscreen().is_some() {
                                    None
                                } else {
                                    Some(Fullscreen::Borderless(None))
                                };
                                self.av.video_fullscreen = next.is_some();
                                w.set_fullscreen(next);
                                if let Err(e) = self.av.save() {
                                    eprintln!("menu: cannot save window mode ({e})");
                                }
                            }
                        }
                        if !self.paused {
                            // Zoom is gameplay-only; prevent it changing behind menus.
                            match code {
                                _ if key_name(code).is_some_and(|n| n == self.icfg.zoom_in_key) => {
                                    self.zoom_by(-1)
                                }
                                _ if key_name(code)
                                    .is_some_and(|n| n == self.icfg.zoom_out_key) =>
                                {
                                    self.zoom_by(1)
                                }
                                KeyCode::Equal | KeyCode::NumpadAdd => self.zoom_by(-1),
                                KeyCode::Minus | KeyCode::NumpadSubtract => self.zoom_by(1),
                                _ => {}
                            }
                        }
                    }
                    ElementState::Released => {
                        self.held.remove(&code);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if !self.paused && self.icfg.mouse_keyboard {
                    gba_core::ffta_battle_mouse::set_mouse_visual(&mut self.machine.bus, true);
                }
                let p = (position.x, position.y);
                self.mouse_pos = Some(p);

                if self.paused {
                    self.mouse_queue.push(MouseEv::Move(p));
                } else {
                    self.game_mouse_aim(p, false);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.pub_scroll_drag = None;
                self.mouse_pos = None;
                self.game_mouse.cancel();
                self.machine.bus.ffta_battle_mouse.cancel();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.pub_scroll_drag = None;
                if self.paused {
                    self.mouse_queue.push(MouseEv::Release);
                } else {
                    }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if let (true, Some(p)) = (self.paused, self.mouse_pos) {
                    match button {
                        MouseButton::Left => self.mouse_queue.push(MouseEv::Click(p, true)),
                        MouseButton::Right => self.mouse_queue.push(MouseEv::Click(p, false)),
                        _ => {}
                    }
                } else if !self.icfg.mouse_keyboard {
                    return;
                } else if button == MouseButton::Right {
                    gba_core::ffta_battle_mouse::set_mouse_visual(&mut self.machine.bus, true);
                    self.game_mouse.cancel();
                    gba_core::ffta_battle_mouse::back(&mut self.machine.bus);
                } else if button == MouseButton::Left {
                    gba_core::ffta_battle_mouse::set_mouse_visual(&mut self.machine.bus, true);
                    // Capture context BEFORE aiming: a miss cancels pending
                    // selection but must never turn into a blind confirm.
                    let menu_context = self.game_mouse.scene(&self.machine.bus).is_some()
                        || gba_core::ffta_battle_mouse::MenuScene::read(&self.machine.bus)
                            .is_some()
                        || {
                            let ui = &self.machine.bus.ffta_dialogue;
                            !ui.service_speech.active()
                                && (ui.service_layout
                                    || ui.town_layout
                                    || ui.shop_layout
                                    || ui.site_layout
                                    || ui.extra.kind == gba_core::ffta_dialogue::PanelKind::Notice)
                        };
                    let menu_context = menu_context
                        || gba_core::ffta_roster_mouse::active(&self.machine.bus)
                        || gba_core::ffta_battle_mouse::map_award_bounds(&self.machine.bus)
                            .is_some();
                    let handled = self.mouse_pos.is_some_and(|p| self.game_mouse_aim(p, true));
                    if allow_background_confirm(
                        handled,
                        menu_context,
                        self.mouse_pos
                            .and_then(|p| self.window_to_frame(p))
                            .is_some(),
                    ) {
                        self.game_mouse.cancel();
                        gba_core::ffta_battle_mouse::press(&mut self.machine.bus, 0x3fe);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {

                if self.paused {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => (p.y / 30.0) as f32,
                    };
                    if dy > 0.0 {
                        self.mouse_queue.push(MouseEv::Wheel(true));
                    } else if dy < 0.0 {
                        self.mouse_queue.push(MouseEv::Wheel(false));
                    }
                } else {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => (p.y / 30.) as f32,
                    };
                    if !self.icfg.mouse_keyboard {
                        return;
                    }
                    gba_core::ffta_battle_mouse::set_mouse_visual(&mut self.machine.bus, true);
                    if dy != 0.
                        && !gba_core::ffta_roster_mouse::scroll_by(&mut self.machine.bus, dy > 0.)
                        && !gba_core::ffta_battle_mouse::pub_scroll_by(
                            &mut self.machine.bus,
                            dy > 0.,
                        )
                    {
                        gba_core::ffta_shop_mouse::scroll(&mut self.machine.bus, dy > 0.);
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        // Pointer policy, reconciled live: visible in windowed mode (users
        // need it to drag/resize/close the window) and while the pause menu is
        // up; hidden only when the game owns a fullscreen screen. Call through
        // only on a change.
        let fullscreen = self
            .window
            .as_ref()
            .is_some_and(|w| w.fullscreen().is_some());
        let want_cursor = self.paused
            || !fullscreen
            || self.machine.bus.ffta_battle_mouse.enabled
            || self.game_mouse.scene(&self.machine.bus).is_some();
        if want_cursor != self.cursor_visible {
            if let Some(w) = self.window.as_ref() {
                w.set_cursor_visible(want_cursor);
            }
            self.cursor_visible = want_cursor;
        }

        let (mut keys, mut nav) = self.gather_input();
        let save_screen = !self.paused && self.autosave.screen_active(&self.machine.bus);
        if save_screen && !crate::autosave::confirming_save(&self.machine.bus) {
            nav.left |= keys & 0x20 == 0;
            nav.right |= keys & 0x10 == 0;
            nav.confirm |= keys & 1 == 0;
            if !self.autosave.screen {
                self.autosave.delete_confirm = None;
                self.autosave.selected = 1;
                self.autosave.carousel_start = 0;
                self.autosave.message = None;
            }
            let delete_modal = self.autosave.delete_confirm.is_some();
            if delete_modal {
                if (nav.left && !self.nav_prev.left) || (nav.right && !self.nav_prev.right) {
                    self.autosave.delete_accept = !self.autosave.delete_accept;
                }
                if nav.cancel && !self.nav_prev.cancel {
                    let _ = self.autosave.finish_delete(false);
                } else if nav.confirm && !self.nav_prev.confirm {
                    let accept = self.autosave.delete_accept;
                    let _ = self.autosave.finish_delete(accept);
                }
            } else {
                if nav.left && !self.nav_prev.left {
                    self.autosave
                        .move_carousel(crate::autosave::is_saving(&self.machine.bus), false);
                }
                if nav.right && !self.nav_prev.right {
                    self.autosave
                        .move_carousel(crate::autosave::is_saving(&self.machine.bus), true);
                }
                if nav.confirm && !self.nav_prev.confirm {
                    self.activate_save_card();
                }
            }
            // The host owns all three cards; do not also navigate the two-slot guest menu.
            keys |= if delete_modal { 0x03ff } else { 0x00f9 };
        }
        self.autosave.screen = save_screen;

        // Drop any button from the suppression mask once observed released
        // (active-low bit set → cleared from `!keys`), then force the buttons
        // still held over from a menu interaction to read as released.
        self.suppress &= !keys;
        keys |= self.suppress;

        // The open control (Escape, or the pad's guide button) toggles the
        // pause overlay. With the menu disabled (a package pin), its rising
        // edge quits instead, preserving the historical Escape = quit.
        // While the menu waits for a key, Escape cancels the capture
        // instead of closing the menu.
        let capturing_now = self.paused && self.menu.capturing().is_some();
        let toggle_edge = !capturing_now && nav.open && !self.nav_prev.open;
        if toggle_edge {
            if !self.menu_enabled {
                event_loop.exit();
                return;
            }
            self.paused = !self.paused;
            if self.paused {
                self.game_mouse.cancel();
                self.machine.bus.ffta_battle_mouse.cancel();
                self.menu.reset();
                self.menu_dirty = true;
                // Drop stale pointer events from before the menu opened.
                self.mouse_queue.clear();
            }
            reset_audio_counters(&self.streams);
            // A toggle press is fully consumed here — while paused, fall
            // through to the overlay redraw; the same press must not also be
            // fed to the menu as a Cancel.
            if !self.paused {
                self.suppress = !keys & 0x3FF;
                self.nav_prev = nav;
                return;
            }
        }

        if self.paused {
            let previous_aspect = self.av.video_aspect.clone();
            self.menu_dirty |= self.pending_key.is_some()
                || self.pending_pad.is_some()
                || !self.mouse_queue.is_empty();
            // Feed the menu the rising edges and apply any setting change live
            // (video) or to av.cfg only (restart-required audio/gamut). The
            // GBA sees all keys released while the menu is up.
            self.machine.bus.keys = 0x3FF;
            // Key capture: the Inputs page asked for a key; take the next
            // press (Escape cancels). Navigation is frozen meanwhile and for
            // the rest of this tick, so the chosen key (an arrow, Enter...)
            // can't also move the cursor.
            let was_capturing = self.menu.capturing().is_some();
            let captured = self.pending_key.take();
            if let (Some(btn), Some(code)) = (self.menu.capturing(), captured) {
                if code == KeyCode::Escape {
                    self.menu.end_capture();
                } else if let Some(name) = key_name(code) {
                    // A key already bound elsewhere swaps with this binding.
                    match btn {
                        menu::Bind::Button(b) => {
                            let idx = b.index();
                            let old = self.icfg.active_keys()[idx].clone();
                            if let Some(j) = (0..10)
                                .find(|&j| j != idx && self.icfg.active_keys_mut()[j] == name)
                            {
                                self.icfg.active_keys_mut()[j] = old;
                            } else if self.icfg.ff_key == name {
                                self.icfg.ff_key = old;
                            } else if self.icfg.ff_toggle_key == name {
                                self.icfg.ff_toggle_key = old;
                            }
                            self.icfg.active_keys_mut()[idx] = name.to_string();
                        }
                        menu::Bind::FastForward => {
                            let old = self.icfg.ff_key.clone();
                            if let Some(j) =
                                (0..10).find(|&j| self.icfg.active_keys_mut()[j] == name)
                            {
                                self.icfg.active_keys_mut()[j] = old;
                            } else if self.icfg.ff_toggle_key == name {
                                self.icfg.ff_toggle_key = old;
                            }
                            self.icfg.ff_key = name.to_string();
                        }
                        menu::Bind::FastForwardToggle => {
                            let old = self.icfg.ff_toggle_key.clone();
                            if let Some(j) =
                                (0..10).find(|&j| self.icfg.active_keys_mut()[j] == name)
                            {
                                self.icfg.active_keys_mut()[j] = old;
                            } else if self.icfg.ff_key == name {
                                self.icfg.ff_key = old;
                            }
                            self.icfg.ff_toggle_key = name.to_string();
                        }
                        menu::Bind::ZoomIn => self.icfg.zoom_in_key = name.to_string(),
                        menu::Bind::ZoomOut => self.icfg.zoom_out_key = name.to_string(),
                        menu::Bind::MenuKey => self.icfg.menu_key = name.to_string(),
                        menu::Bind::GamepadZoomIn
                        | menu::Bind::GamepadZoomOut
                        | menu::Bind::GamepadFastForward
                        | menu::Bind::GamepadMenu
                        | menu::Bind::GamepadFastForwardToggle => {}
                        menu::Bind::Gamepad(_) => {}
                    }
                    if !matches!(btn, menu::Bind::Gamepad(_)) {
                        self.rebuild_key_pairs();
                        if let Err(e) = self.icfg.save() {
                            eprintln!("menu: could not save input.cfg ({e})");
                        }
                        self.menu.end_capture();
                    }
                }
            }
            if let (Some(binding), Some(name)) = (self.menu.capturing(), self.pending_pad.take()) {
                if !self
                    .menu_tap_filter
                    .accept_at(&format!("pad:{name}"), Instant::now())
                {
                    self.nav_prev = nav;
                    return;
                }
                if let menu::Bind::Gamepad(button) = binding {
                    let index = button.index();
                    let old = self.icfg.pads[index].clone();
                    if let Some(other) = (0..10).find(|&i| i != index && self.icfg.pads[i] == name)
                    {
                        self.icfg.pads[other] = old;
                    }
                    self.icfg.pads[index] = name;
                } else if binding == menu::Bind::GamepadMenu {
                    self.icfg.menu_pad = name;
                } else if binding == menu::Bind::GamepadZoomIn {
                    self.icfg.zoom_in_pad = name;
                } else if binding == menu::Bind::GamepadZoomOut {
                    self.icfg.zoom_out_pad = name;
                } else if binding == menu::Bind::GamepadFastForward {
                    self.icfg.ff_pad = name;
                } else if binding == menu::Bind::GamepadFastForwardToggle {
                    self.icfg.ff_toggle_pad = name;
                }
                if let Err(error) = self.icfg.save() {
                    eprintln!("menu: could not save input.cfg ({error})");
                }
                self.menu.end_capture();
            }
            let events = [
                (nav.up && !self.nav_prev.up, menu::MenuInput::Up),
                (nav.down && !self.nav_prev.down, menu::MenuInput::Down),
                (nav.left && !self.nav_prev.left, menu::MenuInput::Left),
                (nav.right && !self.nav_prev.right, menu::MenuInput::Right),
                (
                    nav.confirm && !self.nav_prev.confirm,
                    menu::MenuInput::Confirm,
                ),
                // East backs out; Escape already toggled above, so only feed
                // Cancel when this frame wasn't a toggle.
                (
                    !toggle_edge && nav.cancel && !self.nav_prev.cancel,
                    menu::MenuInput::Cancel,
                ),
            ];
            self.nav_prev = nav;
            let mut quit = false;
            // Actions from keyboard/pad edges and from the mouse, applied
            // together below.
            let mut acts: Vec<menu::MenuAction> = Vec::new();
            for (fired, ev) in events {
                if !fired || was_capturing {
                    continue;
                }
                self.menu_dirty = true;
                let control = match ev {
                    menu::MenuInput::Up => "menu:up",
                    menu::MenuInput::Down => "menu:down",
                    menu::MenuInput::Left => "menu:left",
                    menu::MenuInput::Right => "menu:right",
                    menu::MenuInput::Confirm => "menu:confirm",
                    menu::MenuInput::Cancel => "menu:cancel",
                };
                if !self.menu_tap_filter.accept_at(control, Instant::now()) {
                    continue;
                }
                if let Some(a) = self.menu.handle(ev, &mut self.av) {
                    acts.push(a);
                }
            }
            // Mouse: hover highlights, left/right click act, wheel moves.
            let mq = std::mem::take(&mut self.mouse_queue);
            let (fw, fh) = self.menu_dimensions();
            for m in mq {
                if was_capturing {
                    // Any click abandons a pending key capture.
                    if matches!(m, MouseEv::Click(..)) {
                        self.menu.end_capture();
                    }
                    continue;
                }
                match m {
                    MouseEv::Release => {},
                    MouseEv::Move(p) => {
                        if let Some((x, y)) = self.window_to_frame(p) {

                            self.menu.hover(x, y, fw, fh);
                        }
                    }
                    MouseEv::Click(p, left) => {
                        if let Some((x, y)) = self.window_to_frame(p) {
                            if let Some(a) = self.menu.click(x, y, fw, fh, left, &mut self.av) {
                                acts.push(a);
                            }
                        }
                    }
                    MouseEv::Wheel(up) => {
                        if let Some(a) = self.menu.scroll(up, &mut self.av) {
                            acts.push(a);
                        }
                    }
                }
            }
            for action in acts {
                match action {
                    menu::MenuAction::DebugVictory => {
                        let result = gba_core::ffta_debug::request_victory(&mut self.machine.bus);
                        eprintln!(
                            "FFTA_DEBUG_WIN frame={} result={result:?}",
                            self.machine.bus.frames
                        );
                        match result {
                            Ok(()) => self.paused = false,
                            Err(error) => self.menu.debug_status(error),
                        }
                        self.menu_dirty = true;
                    }
                    menu::MenuAction::Resume => self.paused = false,
                    menu::MenuAction::Quit => quit = true,
                    menu::MenuAction::SwitchRom => {
                        if self.choose_rom() {
                            quit = true;
                            break;
                        }
                    }
                    menu::MenuAction::InputDevice(device) => {
                        self.icfg.device = device;
                        if device != input_config::Device::Gamepad {
                            self.icfg.mouse_keyboard =
                                device == input_config::Device::MouseKeyboard;
                            let enabled = self.icfg.mouse_keyboard;
                            self.machine.bus.ffta_battle_mouse.set_enabled(enabled);
                            self.game_mouse = gba_core::ffta_mouse::LanguageMouse::new(
                                &self.machine.bus.rom,
                                enabled,
                            );
                        }
                        self.rebuild_key_pairs();
                        if device == input_config::Device::Gamepad {
                            if self.pad.is_none() {
                                self.pad = build_gilrs();
                            }
                            if self.directinput.is_none()
                                && std::env::var("FFTA_GAMEPAD_BACKEND").as_deref() != Ok("gilrs")
                            {
                                self.directinput = input_config::directinput::DirectInput::new();
                            }
                        }
                        if let Err(error) = self.icfg.save() {
                            eprintln!("menu: could not save input.cfg ({error})");
                        }
                    }
                    menu::MenuAction::ResetGamepad => {
                        let defaults = input_config::InputConfig::default();
                        self.icfg.pads = defaults.pads;
                        self.icfg.menu_pad = defaults.menu_pad;
                        self.icfg.zoom_in_pad = defaults.zoom_in_pad;
                        self.icfg.ff_pad = defaults.ff_pad;
                        self.icfg.ff_toggle_pad = defaults.ff_toggle_pad;
                        self.icfg.zoom_out_pad = defaults.zoom_out_pad;
                        self.icfg.dpad_source = defaults.dpad_source;
                        self.icfg.stick_deadzone = defaults.stick_deadzone;
                        if let Err(error) = self.icfg.save() {
                            eprintln!("menu: could not save input.cfg ({error})");
                        }
                    }
                    menu::MenuAction::Changed(what) => {
                        // Vsync is the presenter's own knob; everything
                        // else routes through apply_av_change.
                        if what == menu::Changed::Vsync {
                            if let Some(p) = self.presenter.as_mut() {
                                p.set_vsync(self.av.video_vsync);
                            }
                        }
                        apply_av_change(
                            what,
                            &self.av,
                            &self.streams,
                            &mut self.screen_kind,
                            &mut self.response,
                            &mut self.lut,
                            &mut self.temporal,
                            &mut self.grid_params,
                            self.lut_primaries,
                        );
                        // apply_av_change may rebuild the temporal state for
                        // a 160-row frame; make it match the zoomed size.
                        if what == menu::Changed::Video {
                            self.rebuild_temporal();
                            self.grid_params.scale_mode = scale_mode_of(&self.av.scaling);
                        }
                        if what == menu::Changed::WindowMode {
                            if let Some(w) = self.window.as_ref() {
                                w.set_fullscreen(if self.av.video_fullscreen {
                                    Some(Fullscreen::Borderless(None))
                                } else {
                                    None
                                });
                            }
                        }
                        if let Err(e) = self.av.save() {
                            eprintln!("menu: could not save av.cfg ({e})");
                        }
                    }
                }
            }
            if self.av.video_aspect != previous_aspect {
                // FFTA_WIDESCREEN sets the starting view only. An explicit
                // in-session menu choice takes precedence over that override.
                self.base_extra = aspect_extra(&self.av.video_aspect).min(gba_core::mem::MAX_EXTRA);
                self.zoom_max = zoom_limit(self.base_extra);
                self.zoom_level = self.zoom_level.min(self.zoom_max);
                // sync_view applies dimensions and resets temporal history at
                // the next emulated frame; the paused frame stays valid here.
            }
            if quit {
                event_loop.exit();
                return;
            }
            if !self.paused {
                // Resume: clear the counters again so the pause tail doesn't
                // bleed into the next report, and neutralize the buttons held
                // to select Resume/back out (South=A, East=B) so they don't
                // buffer into the game. The cursor is re-hidden by the next
                // reconcile if fullscreen.
                reset_audio_counters(&self.streams);
                self.suppress = !keys & 0x3FF;
                return;
            }
            // Poll input at the normal menu cadence, but rasterize the native-
            // resolution canvas only when input or window events changed it.
            if !self.menu_dirty {
                std::thread::sleep(Duration::from_millis(16));
                return;
            }
            // Redraw the overlay over the last emulated frame at a modest fixed
            // cadence. Re-compose from the frozen `temporal` each time
            // (emulation, temporal, and the frame counter all stand still while
            // paused) so the overlay's darkening never compounds across
            // redraws.
            self.temporal.compose(&self.lut, &mut self.rgba);
            let (mw, mh) = self.menu_dimensions();
            self.pause_rgba.resize(mw * mh, [0, 0, 0, 255]);
            let [vx, vy, vw, vh] = self.view;
            let (scale, ox, oy) = screen::present::letterbox_f((mw as u32, mh as u32), vw, vh);
            for y in 0..mh {
                for x in 0..mw {
                    let fx = (x as f32 - ox) / scale;
                    let fy = (y as f32 - oy) / scale;
                    self.pause_rgba[y * mw + x] = if fx >= 0. && fy >= 0. && fx < vw && fy < vh {
                        self.rgba[((vy + fy) as usize).min(wide_h() - 1) * wide_w()
                            + ((vx + fx) as usize).min(wide_w() - 1)]
                    } else {
                        [0, 0, 0, 255]
                    };
                }
            }
            for i in 0..gba_core::ffta_ui::LAYERS {
                if let Some(layout) = self.ui_layout((mw as u32, mh as u32), i) {
                    let mut canvas = std::mem::take(&mut self.pause_rgba);
                    let ui = self.ui_frame(layout, i);
                    for y in 0..mh {
                        for x in 0..mw {
                            if let Some(c) = ui.sample(x as f32 + 0.5, y as f32 + 0.5) {
                                canvas[y * mw + x] = c;
                            }
                        }
                    }
                    self.pause_rgba = canvas;
                }
            }
            self.menu
                .draw_inputs(&self.av, &self.icfg, &mut self.pause_rgba, mw, mh);
            std::mem::swap(&mut self.rgba, &mut self.pause_rgba);
            let game_view = self.view;
            self.view = [0., 0., mw as f32, mh as f32];
            self.present_rgba();
            self.view = game_view;
            std::mem::swap(&mut self.rgba, &mut self.pause_rgba);
            self.menu_dirty = false;
            std::thread::sleep(Duration::from_millis(16));
            return;
        }
        self.nav_prev = nav;

        self.game_keys = keys;

        // Audio-clock pacing: the device callback consumes at a fixed rate, so
        // run exactly as many frames as the ring needs — production follows
        // consumption, immune to display refresh rate (ProMotion) and window-
        // server throttling. Without a device, wall-clock pace at the hardware
        // frame rate (handled below).
        // The multiplier never activates fast-forward by itself. Holding the
        // momentary key inverts the session's permanent-toggle state.
        let ff_held = self.pad_ff_held
            || winit_key(&self.icfg.ff_key).is_some_and(|k| self.held.contains(&k));
        let ff_speed = input_config::fast_forward_speed(self.av.ff_speed, self.ff_latched, ff_held);
        let ff_now = ff_speed > 1.0;
        if ff_now != self.ff_active
            || ff_speed != self.ff_speed
            || self.av.ff_audio != self.ff_audio_active
        {
            self.set_fast_forward(ff_now, ff_speed);
        }
        self.ff_speed = ff_speed;
        // ALSA/PipeWire may consume audio in large callback batches. Using
        // those batches as video deadlines skips intermediate presentations.
        // Prime the audio normally, then present one frame per hardware tick.
        // Keep the original audio-driven path available for A/B comparison.
        static CLOCK_PACING: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let clock_pacing = *CLOCK_PACING.get_or_init(|| {
            cfg!(any(target_os = "linux", windows))
                && std::env::var("FFTA_FRAME_PACING").as_deref() != Ok("audio")
        });
        let smooth_normal = clock_pacing && self.primed && !self.ff_active;
        let frame_cap = if self.ff_active {
            self.ff_budget.next(self.ff_speed)
        } else if smooth_normal {
            1
        } else {
            4
        };
        let mut ran = 0u32;
        while ran < frame_cap {
            if self.ff_active {
                // Fast forward: no ring pacing; run the whole batch.
            } else if self.stream.is_some() && !smooth_normal {
                // Both rings are live (the menu can crossfade between them), so
                // pace on the emptier one — neither faithful nor enhanced may
                // be allowed to starve.
                let fill = {
                    let st = self.streams.lock().unwrap();
                    (st.mixed.len() / 2).min(st.psg.len() / 2)
                };
                if fill >= RING_TARGET {
                    break;
                }
            } else if ran >= 1 {
                break;
            }
            ran += 1;
            self.run_one_frame();
        }

        // Once both rings first reach target, the buffer has primed: discard
        // the cold-start underruns so they don't report as a defect.
        if !self.primed && self.stream.is_some() && !self.ff_active {
            let st = self.streams.lock().unwrap();
            if (st.mixed.len() / 2).min(st.psg.len() / 2) >= RING_TARGET {
                drop(st);
                self.primed = true;
                if clock_pacing {
                    self.next_frame = Instant::now();
                }
                reset_audio_counters(&self.streams);
            }
        }

        if !self.ff_active || self.av.ff_audio != "silent" {
            self.audio_watchdog();
        }

        if ran == 0 {
            // Ring is full: let the device drain a little before the next tick.
            // (winit pumps OS events between ticks; no explicit pump needed.)
            std::thread::sleep(Duration::from_millis(2));
            return;
        }
        if self.stream.is_none() || self.ff_active || smooth_normal {
            self.next_frame += Duration::from_nanos(16_742_706); // 59.7275 Hz
            let now = Instant::now();
            if self.next_frame > now {
                std::thread::sleep(self.next_frame - now);
            } else {
                self.next_frame = now;
            }
        }

        // Resolve the visible image (temporal response + color LUT) and
        // present it (downstream of production, never the pacing source).
        self.temporal.compose(&self.lut, &mut self.rgba);
        self.present_rgba();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Final teardown on a clean exit (window closed / menu Quit): this is
        // winit's guaranteed last callback, so it runs whether or not
        // `run_app` returns on a given platform. The in-loop periodic save
        // bounds loss on an abnormal (signal) kill. Skip if we're already
        // failing (e.g. window creation never succeeded).
        if self.error.is_some() {
            return;
        }
        print_fallback_census();
        if FALLBACK_COLLECT.load(Ordering::Relaxed) {
            if let Err(e) = record_labels(
                self.machine.bus.rom.len(),
                &rom_sha256(&self.machine.bus.rom),
            ) {
                self.error = Some(e);
            }
        }
    }
}

/// Interactive play: resolve the image/BIOS/translation, set up audio, input,
/// and screen simulation, then run the winit event loop until the window
/// closes. Teardown (final save, fallback census, label recording) happens
/// after the loop returns.
pub fn cmd_play(args: &[String]) -> Result<(), String> {
    let mut rom_path = None;
    let mut interp_only = false;
    // Machine-readable lifecycle lines on stdout for a supervising frontend
    // (the launcher): `STATUS building <pct>` / `STATUS playing`.
    let mut status = false;
    // Translate the image into the cache, then exit without opening a window.
    // A launcher uses it to do the one-time build as a setup step.
    let mut prepare = false;
    // Perf instrumentation is developer tooling: always on in debug builds,
    // opt-in (--stats or GBA_RECOMP_STATS=1) in release.
    let mut show_stats = cfg!(debug_assertions) || std::env::var_os("GBA_RECOMP_STATS").is_some();
    let mut bios_arg: Option<String> = None;
    let mut no_bios = false;
    let mut gamedb_arg: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--interp" => interp_only = true,
            "--status" => status = true,
            "--prepare" => prepare = true,
            "--stats" => show_stats = true,
            "--record-labels" => FALLBACK_COLLECT.store(true, Ordering::Relaxed),
            "--bios" => bios_arg = Some(it.next().ok_or("--bios needs a value")?.to_string()),
            "--no-bios" => no_bios = true,
            "--gamedb" => gamedb_arg = Some(it.next().ok_or("--gamedb needs a value")?.to_string()),
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    // The function-boundary source for a first-launch build: open it once
    // here so on-demand translation seeds exhaustively from the map (and
    // --ram profiling captures RAM-resident code the map can't carry).
    let gamedb = gamedb_arg
        .as_deref()
        .map(|p| gamedb::GameDb::open(std::path::Path::new(p)))
        .transpose()?;
    // Packaged mode: a manifest beside the executable pins this binary to one
    // title. The ROM is resolved by content hash (the package ships none), the
    // BIOS pin is enforced, and the translation loads from the package.
    let manifest = match crate::packfile::load() {
        Some(Ok(m)) => Some(m),
        Some(Err(e)) => return Err(e),
        None => None,
    };
    let rom_path = match (&rom_path, &manifest) {
        (Some(p), _) => p.clone(),
        (None, Some(m)) => {
            let p = crate::packfile::find_rom(m)?;
            eprintln!("{}: playing {}", m.name, p.display());
            p.to_string_lossy().into_owned()
        }
        (None, None) => return Err("missing ROM path".into()),
    };
    let rom = std::fs::read(&rom_path).map_err(|e| format!("{rom_path}: {e}"))?;
    validate_rom(&rom_path, &rom)?;
    if let Some(m) = &manifest {
        let sha = rom_sha256(&rom);
        if sha != m.rom_sha256 {
            return Err(format!(
                "this package plays exactly one cartridge image \
(sha256 {}…); {rom_path} hashes to {}…",
                &m.rom_sha256[..16],
                &sha[..16]
            ));
        }
    }
    let sav_path = crate::save_path::resolve(Path::new(&rom_path))?
        .to_string_lossy()
        .into_owned();
    // Debug changes may later reach the game's normal save routine. Always
    // isolate this session, including autosave and teardown, before loading it.
    let sav_path = if std::env::var("FFTA_DEBUG").as_deref() == Ok("1") {
        let original = std::path::Path::new(&sav_path);
        let folder = original
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("ffta-debug-saves");
        std::fs::create_dir_all(&folder).map_err(|e| format!("debug save folder: {e}"))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let copy = folder.join(format!("session-{}-{stamp}.sav", std::process::id()));
        if original.is_file() {
            std::fs::copy(original, &copy).map_err(|e| format!("debug save copy: {e}"))?;
        }
        eprintln!("FFTA_DEBUG_SAVE isolated={}", copy.display());
        copy.to_string_lossy().into_owned()
    } else {
        sav_path
    };

    // Product boot path: real BIOS when an image is installed (explicit --bios
    // > the launcher-installed image via the shared resolution), BIOS HLE
    // otherwise. An explicit --bios that fails to load is a hard error; a
    // discovered image that fails is loud but non-fatal.
    let bios: Option<Vec<u8>> = if no_bios {
        eprintln!("boot: BIOS HLE (--no-bios)");
        None
    } else if let Some(p) = &bios_arg {
        let b = load_bios_file(p)?;
        eprintln!("boot: real BIOS ({p})");
        Some(b)
    } else if let Some(p) = input_config::find_bios() {
        match load_bios_file(&p) {
            Ok(b) => {
                eprintln!("boot: real BIOS ({})", p.display());
                Some(b)
            }
            Err(e) => {
                eprintln!("DEGRADED: installed BIOS unusable ({e}); using BIOS HLE");
                None
            }
        }
    } else {
        eprintln!("boot: BIOS HLE (no BIOS image installed)");
        None
    };
    if let Some(b) = &bios {
        use sha2::{Digest, Sha256};
        let sha = Sha256::digest(b)
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect::<String>();
        if let Some(m) = &manifest {
            if sha != m.bios_sha256 {
                return Err(format!(
                    "this package requires the pinned BIOS (sha256 {}…); the installed \
image hashes to {}…",
                    &m.bios_sha256[..16],
                    &sha[..16]
                ));
            }
        } else if sha != input_config::BIOS_SHA256 {
            eprintln!("bios: image is not the canonical dump (sha256 {sha}) — trying it as-is");
        }
    } else if let Some(m) = &manifest {
        return Err(format!(
            "this package requires a BIOS image (sha256 {}…) and none is installed.\n\
Place your own dump as gba_bios.bin next to the executable:\n  {}",
            &m.bios_sha256[..16],
            m.dir.display()
        ));
    }

    // Native translation. Packaged: the prebuilt library ships in the package
    // — under interpreter=false (a *full recomp*) failing to load it is fatal,
    // and so is any dispatch miss later. Developer path: per-user cache, built
    // on first launch. The product bar is full speed at full accuracy.
    let native = if let Some(m) = &manifest {
        if interp_only && !m.interpreter {
            return Err("--interp is not available in a full-recomp package".into());
        }
        if !m.interpreter {
            NO_INTERP.store(true, Ordering::Relaxed);
        }
        let lib_path = m.dir.join(&m.translation);
        match load_native(&lib_path).inspect(|v| {
            eprintln!("native translation: {} blocks (packaged)", v.1.len);
        }) {
            Ok(v) if !interp_only => Some(v),
            Ok(_) => None,
            Err(e) if !m.interpreter => return Err(e),
            Err(e) => {
                eprintln!("DEGRADED: packaged translation unavailable ({e}); interpreter only");
                None
            }
        }
    } else if interp_only {
        None
    } else {
        match ensure_native(&rom_path, &rom, status, bios.as_deref(), gamedb.as_ref()) {
            Ok(v) => Some(v),
            // A setup step must fail loudly; only a real play session may fall back.
            Err(e) if prepare => return Err(format!("native translation failed: {e}")),
            Err(e) => {
                eprintln!("DEGRADED: native translation unavailable ({e}); interpreter only");
                None
            }
        }
    };
    if prepare {
        return Ok(());
    }

    let mut machine = make_machine(rom, bios.as_deref());
    if let Ok(sav) =
        std::fs::read(&sav_path).or_else(|_| std::fs::read(format!("{sav_path}.legacy-backup")))
    {
        machine.bus.load_save_data(&sav);
        eprintln!("loaded {sav_path}");
    }

    // Triage (RECOMP_RECORD_INPUT=<path>): record this session's input as a
    // replayable script (frames/runc/verify --input). The save state at
    // session start is snapshotted next to it so the replay boots from
    // identical backup media.
    let record: Option<(std::fs::File, u16)> = match std::env::var_os("RECOMP_RECORD_INPUT") {
        None => None,
        Some(p) => {
            use std::io::Write;
            let p = p.to_string_lossy().into_owned();
            if Path::new(&sav_path).is_file() {
                let _ = std::fs::copy(&sav_path, format!("{p}.sav"));
            }
            let mut f = std::fs::File::create(&p).map_err(|e| format!("{p}: {e}"))?;
            writeln!(f, "gba-input v1").map_err(|e| e.to_string())?;
            eprintln!("recording input to {p}");
            Some((f, 0x3FF))
        }
    };

    let title = String::from("FFTARecomp");

    // Audio. Both paths run at once so the in-game menu can crossfade between
    // faithful and enhanced live (the callback slews between them, the producer
    // fills both rings). tap_channels gates only the extra per-channel work;
    // the faithful mix is produced regardless. The MP2K/engine HLE shadow is
    // armed when the stock driver is present so the enhanced path is ready the
    // instant it's selected; it's read-only, so arming it always is free of
    // correctness risk.
    let mut av = input_config::AvConfig::load();
    if let Ok(aspect) = std::env::var("FFTA_WIDESCREEN") {
        if !aspect.trim().is_empty() {
            // Show the actual initial ratio in the menu, including when a
            // command-line environment override supplied it.
            av.video_aspect = match aspect.trim() {
                "off" | "0" => "3:2".into(),
                value => value.to_owned(),
            };
        }
    }
    // The aspect setting picks the STARTING view (x1): (240 + 2 * extra) x 160.
    // Zoom (+/-) then renders and shows more of the world around it, always in
    // that same aspect ratio, and never less than x1.
    let base_extra = widescreen_extra(&av).min(gba_core::mem::MAX_EXTRA);
    gba_core::mem::set_extra(base_extra);
    gba_core::mem::set_extra_y(0);
    #[cfg(any(windows, target_os = "linux"))]
    gba_core::ppu::set_parallel_zoom(std::env::var("FFTA_PARALLEL_ZOOM").as_deref() != Ok("0"));
    let zoom_max = zoom_limit(base_extra);
    machine.bus.tap_channels = true;
    let desc = arm_audio_hle(
        &mut machine,
        manifest.as_ref().and_then(|m| m.engine_hle.as_deref()),
    );
    eprintln!("audio engine: {desc}");
    let streams = Arc::new(Mutex::new(AudioStreams::default()));
    {
        // Apply persisted gain before the device callback can consume samples.
        let mut audio = streams.lock().unwrap();
        audio.enhanced_on = av.audio_enhanced;
        audio.vol_cut = 1.0 - av.audio_volume.clamp(0.0, 1.0);
    }
    let stream = start_audio(streams.clone());

    // Bindings come from the launcher-managed config (defaults match the
    // historical hardcoded map). Keyboard always works; a configured gamepad is
    // OR-ed in on top.
    let mut icfg = input_config::InputConfig::load();
    let directinput_only = std::env::var("FFTA_GAMEPAD_BACKEND").as_deref() == Ok("directinput");
    if directinput_only {
        icfg.device = input_config::Device::Gamepad;
    }
    let defaults = input_config::InputConfig::default();
    let key_pairs: Vec<(KeyCode, u16)> = input_config::Button::ALL
        .iter()
        .map(|b| {
            let name = &icfg.active_keys()[b.index()];
            let key = winit_key(name).unwrap_or_else(|| {
                eprintln!(
                    "input.cfg: unknown key {name:?} for {} — using default",
                    b.name()
                );
                winit_key(&defaults.keys[b.index()]).unwrap()
            });
            (key, b.bit())
        })
        .collect();
    // Keep both detectors alive even when the saved preference is keyboard.
    // Device preference controls the binding page; actual controller input
    // switches the active device at runtime.
    let pad = build_gilrs();
    #[cfg(target_os = "linux")]
    if let Some(gilrs) = &pad {
        eprintln!(
            "gamepad: Linux native snapshot polling (gilrs/evdev), {} connected",
            gilrs.gamepads().count()
        );
    } else {
        eprintln!("gamepad: Linux input backend unavailable; check device access and libudev");
    }
    let directinput = if std::env::var("FFTA_GAMEPAD_BACKEND").as_deref() != Ok("gilrs") {
        input_config::directinput::DirectInput::new()
    } else {
        None
    };
    #[cfg(windows)]
    let xinput = if matches!(
        std::env::var("FFTA_GAMEPAD_BACKEND").as_deref(),
        Ok("directinput" | "gilrs")
    ) {
        None
    } else {
        let backend = XInputBackend::load();
        if backend.is_some() {
            eprintln!("XInput: native snapshot polling enabled");
        }
        backend
    };

    // Screen simulation (see crates/screen): temporal response on the emulated-
    // frame stream, color LUT to the display's colorspace, and a GPU grid pass.
    // Settings come from the launcher-managed A/V config; unknown values fall
    // back loudly. `av` is mutable: the in-game menu edits it live, so the
    // screen objects derived from it rebuild in place on a change.
    let screen_kind = screen::ScreenKind::by_name(&av.screen).unwrap_or_else(|| {
        eprintln!(
            "av.cfg: unknown video.screen {:?} — using frontlit",
            av.screen
        );
        screen::ScreenKind::Frontlit
    });
    let display_target = screen::DisplayTarget::by_name(&av.display_gamut).unwrap_or_else(|| {
        eprintln!(
            "av.cfg: unknown video.gamut {:?} — using auto",
            av.display_gamut
        );
        screen::DisplayTarget::Auto
    });
    // Resolve the LUT's target primaries from the configured gamut plus OS
    // detection (a wide-gamut panel the compositor isn't managing resolves to
    // the panel's own primaries — see screen::display). EDID is sysfs and the
    // probe opens its own Wayland connection, so this needs no window.
    let lut_primaries = screen::display::resolve(display_target);
    let lut = screen::ColorLut::build_with_brightness(
        &screen::ColorSettings {
            screen: screen_kind,
            darken: input_config::AvConfig::knob(&av.screen_darken)
                .map(f64::from)
                .unwrap_or(f64::NAN),
            target: lut_primaries,
        },
        f64::from(av.video_brightness),
    );
    let response = screen::ResponseMode::by_name(&av.response).unwrap_or_else(|| {
        eprintln!(
            "av.cfg: unknown video.response {:?} — using smart",
            av.response
        );
        screen::ResponseMode::Smart
    });
    let temporal = screen::Temporal::new(
        wide_w() * wide_h(),
        response,
        0,
        input_config::AvConfig::knob(&av.response_keep)
            .unwrap_or_else(|| screen::blend::default_rho(screen_kind)),
    );
    let mut grid_params = screen::present::GridParams::with_strength(
        input_config::AvConfig::knob(&av.grid)
            .unwrap_or_else(|| screen_kind.default_grid_strength()),
    );
    grid_params.scale_mode = scale_mode_of(&av.scaling);

    // In-game menu (Escape / pad Mode). Available unless a package pins it off,
    // in which case Escape quits as it did before the menu existed.
    let menu_enabled = manifest.as_ref().map(|m| m.menu).unwrap_or(true);

    let sav_seen = machine.bus.save_data().map(fnv64);
    let save_dir = std::env::var_os("FFTA_SAVES_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let cwd = std::env::current_dir().unwrap_or_default();
            if cwd.join("Cargo.toml").is_file() {
                cwd.join("Saves")
            } else {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("Saves")))
                    .unwrap_or(cwd.join("Saves"))
            }
        });
    let autosave =
        crate::autosave::AutoSave::new_at(&sav_path, &rom_sha256(&machine.bus.rom), save_dir);
    let game_mouse =
        gba_core::ffta_mouse::LanguageMouse::new(&machine.bus.rom, icfg.mouse_keyboard);
    machine
        .bus
        .ffta_battle_mouse
        .set_enabled(icfg.mouse_keyboard);
    let mut app = PlayApp {
        machine,
        autosave,
        autosave_frame: Vec::new(),
        sav_path,
        sav_seen,
        record,
        native,
        streams,
        stream,
        icfg,
        key_pairs,
        pad,
        directinput,
        directinput_only,
        #[cfg(windows)]
        xinput,
        menu_tap_filter: MenuTapFilter::default(),
        active_controller: None,
        last_direct_activity: None,
        pending_pad: None,
        #[cfg(not(target_os = "linux"))]
        gilrs_gestures: HashMap::new(),
        #[cfg(target_os = "linux")]
        linux_pads: HashMap::new(),
        direct_gestures: HashMap::new(),
        pad_zoom_prev: (false, false),
        pad_ff_held: false,
        pad_ff_toggle_prev: false,
        held: HashSet::new(),
        av,
        screen_kind,
        display_target,
        lut_primaries,
        lut,
        response,
        temporal,
        grid_params,
        rgba: vec![[0u8; 4]; wide_w() * wide_h()],
        pause_rgba: Vec::new(),
        ui_pixels: Vec::new(),
        battle_hud_pixels: Default::default(),
        battle_hud_motion: Default::default(),
        panel_pixels: Vec::new(),
        ui_auxiliary: Default::default(),
        ui_scene: Default::default(),
        ui_motion: Default::default(),
        pub_scroll_drag: None,
        ui_pack_cache: Default::default(),
        ui_inspector: crate::ui_inspect::Inspector::from_env(),
        ui_timing: crate::ui_inspect::Timing::from_env(),
        padded_frame: Vec::new(),
        title,
        status,
        window: None,
        presenter: None,
        sb_context: None,
        sb_surface: None,
        menu_enabled,
        menu: menu::Menu::new(true, false),
        paused: false,
        menu_dirty: true,
        // Starts visible (windowed); the loop reconciles against fullscreen.
        cursor_visible: true,
        suppress: 0,
        nav_prev: NavEdges::default(),
        show_stats,
        emu_ema_ms: 0.0,
        frames_run: 0,
        slow_warned: false,
        native_run: 0,
        fallback_run: 0,
        primed: false,
        next_frame: Instant::now(),
        audio_watch: Instant::now(),
        audio_consumed_last: 0,
        audio_stalls: 0,
        audio_retry: None,
        ff_active: false,
        ff_latched: false,
        ff_speed: 4.0,
        ff_budget: input_config::FrameBudget::default(),
        ff_audio_active: String::new(),
        pending_key: None,
        base_extra,
        zoom_level: 0,
        zoom_display: 1.0,
        zoom_from: 1.0,
        zoom_started: None,
        zoom_max,
        view: [0.0, 0.0, (240 + 2 * base_extra) as f32, 160.0],
        mouse_pos: None,
        game_mouse,
        game_keys: 0x3ff,
        mouse_queue: Vec::new(),
        error: None,
    };

    // Teardown (final save, fallback census, label recording) runs in the
    // app's `exiting` callback — winit's guaranteed last hook — so it fires on
    // a clean window close regardless of whether `run_app` returns. Anything
    // it raised surfaces through `app.error`.
    let event_loop = EventLoop::new().map_err(|e| format!("event loop: {e}"))?;
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("event loop: {e}"))?;
    if let Some(e) = app.error {
        return Err(e);
    }
    Ok(())
}

fn load_window_icon() -> Option<winit::window::Icon> {
    let img = image::load_from_memory(include_bytes!("../../installer/assets/icon.ico"))
        .ok()?
        .to_rgba8();
    let (w, h) = img.dimensions();
    winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

// The action list is the independent foreground panel. The main action HUD
// may be active simultaneously and must not steal its mouse coordinates.
fn battle_menu_ui_layer(state: &gba_core::ffta_dialogue::DialogueState) -> Option<usize> {
    if matches!(
        state.extra.kind,
        gba_core::ffta_dialogue::PanelKind::Actions | gba_core::ffta_dialogue::PanelKind::Notice
    ) && state.extra.active()
    {
        Some(1)
    } else if state.action_layout || state.town_layout || state.shop_layout {
        Some(0)
    } else {
        None
    }
}
fn dialogue_choice_canvas(role: gba_core::ffta_ui::Role) -> bool {
    matches!(
        role,
        gba_core::ffta_ui::Role::Dialogue | gba_core::ffta_ui::Role::Notice
    )
}
fn battle_menu_ui_layer_at(
    state: &gba_core::ffta_dialogue::DialogueState,
    scene: &gba_core::ffta_ui::Scene,
    origin: Option<[i32; 2]>,
) -> Option<usize> {
    if state.extra.kind == gba_core::ffta_dialogue::PanelKind::Notice && state.extra.active() {
        return Some(1);
    }
    if let Some([x, y]) = origin {
        for slot in [1, 0] {
            if scene.nodes[slot].is_some_and(|n| {
                let [l, t, r, b] = n.bounds;
                x >= l && x < r && y >= t && y < b
            }) {
                return Some(slot);
            }
        }
    }
    battle_menu_ui_layer(state)
}
fn extract_ui_rectangle(source: &mut Vec<[u8; 4]>, out: &mut Vec<[u8; 4]>, b: [i32; 4]) {
    out.resize(480 * 160, [0, 0, 0, 16]);
    out.fill([0, 0, 0, 16]);
    if source.len() != 480 * 160 {
        return;
    }
    for y in b[1].max(0)..b[3].min(160) {
        for x in b[0].max(-120)..b[2].min(360) {
            let i = y as usize * 480 + (x + 120) as usize;
            out[i] = source[i];
            source[i] = [0, 0, 0, 16];
        }
    }
}
fn allow_background_confirm(handled: bool, menu: bool, in_game: bool) -> bool {
    !handled && !menu && in_game
}
fn pointer_in_ui_bounds(index: usize, bounds: [i32; 4]) -> bool {
    let x = (index % 480) as i32 - 120;
    let y = (index / 480) as i32;
    x >= bounds[0] && x < bounds[2] && y >= bounds[1] && y < bounds[3]
}
#[cfg(test)]
mod mouse_confirm_guard_tests {
    #[test]
    fn begin_battle_choice_uses_notice_canvas_instead_of_world() {
        use gba_core::ffta_ui::{Anchor, Node, Role};
        let node = Node {
            slot: 1,
            role: Role::Notice,
            anchor: Anchor::Modal,
            bounds: [16, 16, 224, 64],
            length: false,
        };
        assert!(super::dialogue_choice_canvas(node.role));
        assert!(!super::dialogue_choice_canvas(Role::ActionInfo));
        let style = gba_core::ffta_dialogue::DialogueStyle {
            scale: 0.9,
            length: 1.1,
            offset_x: 0.,
            offset_y: 0.,
        };
        let layout = super::host_ui_layout((1704, 480), 568., style, node);
        // Actual visible Yes button in the user's 1704x480 battle window.
        let index = layout.source(645., 261.).unwrap();
        let x = (index % 480) as i32 - 120;
        let y = (index / 480) as i32;
        assert!((27..65).contains(&x) && (44..60).contains(&y), "{x},{y}");
    }

    #[test]
    fn confirmation_uses_the_canvas_containing_its_native_origin() {
        use gba_core::ffta_ui::{Anchor, Node, Role, Scene};
        let mut state = gba_core::ffta_dialogue::DialogueState::default();
        state.action_layout = true;
        state.extra.kind = gba_core::ffta_dialogue::PanelKind::Actions;
        state.extra.bounds = Some([176, 64, 240, 152]);
        let mut scene = Scene::default();
        scene.nodes[0] = Some(Node {
            slot: 0,
            role: Role::ActionInfo,
            anchor: Anchor::Corner {
                right: false,
                bottom: true,
            },
            bounds: [0, 48, 160, 160],
            length: true,
        });
        scene.nodes[1] = Some(Node {
            slot: 1,
            role: Role::ActionMenu,
            anchor: Anchor::Corner {
                right: true,
                bottom: true,
            },
            bounds: [168, 64, 240, 160],
            length: true,
        });
        assert_eq!(
            super::battle_menu_ui_layer_at(&state, &scene, Some([80, 104])),
            Some(0)
        );
        assert_eq!(
            super::battle_menu_ui_layer_at(&state, &scene, Some([184, 84])),
            Some(1)
        );
        for length in [1., 1.5, 2.] {
            let shifted = 100. + 160. * (length - 1.);
            let index = 112 * 480 + (shifted + 120.) as usize;
            assert_eq!(
                super::menu_panel_native(index, scene.nodes[0].unwrap().bounds, length),
                (100., 112.)
            );
        }
    }
    #[test]
    fn menus_and_consumed_scroll_clicks_never_fall_through_to_a() {
        for handled in [false, true] {
            for inside in [false, true] {
                assert!(!super::allow_background_confirm(handled, true, inside));
            }
        }
        assert!(!super::allow_background_confirm(true, false, true));
        assert!(!super::allow_background_confirm(false, false, false));
        assert!(super::allow_background_confirm(false, false, true));
    }
    #[test]
    fn service_menu_hit_does_not_extend_outside_its_visible_frame() {
        let bounds = [8, 72, 96, 160];
        for x in 0..240 {
            let index = 100 * 480 + (x + 120) as usize;
            assert_eq!(
                super::pointer_in_ui_bounds(index, bounds),
                (8..96).contains(&x)
            );
        }
    }
    #[test]
    fn funds_canvas_stays_fixed_while_guest_objects_slide_through_viewport_edges() {
        let style = gba_core::ffta_dialogue::DialogueStyle::default();
        let mut previous = None;
        for bounds in [
            [152, 144, 240, 160],
            [168, 152, 256, 168],
            [184, 160, 272, 176],
        ] {
            let node = gba_core::ffta_ui::Node {
                slot: 2,
                role: gba_core::ffta_ui::Role::Funds,
                anchor: gba_core::ffta_ui::Anchor::Edge {
                    right: true,
                    vertical: 160.,
                },
                bounds,
                length: false,
            };
            let layout = super::host_ui_layout((1704, 960), 284., style, node);
            if let Some(rect) = previous {
                assert_eq!(layout.rect, rect);
            }
            previous = Some(layout.rect);
        }
    }
}
fn centered_ui_panel(
    window: (u32, u32),
    base: f32,
    size: f32,
    b: [i32; 4],
    top: bool,
) -> screen::ui_layer::Layout {
    let mut l = screen::ui_layer::Layout::centered(window, base, size);
    let k = l.rect[2] / 480.;
    let s = l.clip[3] / 160.;
    l.rect[0] = (l.clip[0] + l.clip[2] * 0.5 - (120. + (b[0] + b[2]) as f32 * 0.5) * k).round();
    l.rect[1] = if top {
        (l.clip[1] + 6. * s - b[1] as f32 * k).round()
    } else {
        (l.clip[1] + l.clip[3] * 0.5 - (b[1] + b[3]) as f32 * 0.5 * k).round()
    };
    l
}
fn host_ui_layout(
    window: (u32, u32),
    base: f32,
    style: gba_core::ffta_dialogue::DialogueStyle,
    node: gba_core::ffta_ui::Node,
) -> screen::ui_layer::Layout {
    use gba_core::ffta_ui::Anchor;
    use screen::ui_layer::Layout;
    let length = if node.length { style.length } else { 1. };
    let mut size = node.fitted_scale_with_length(style.scale, base, length);
    if matches!(
        node.role,
        gba_core::ffta_ui::Role::Stats
            | gba_core::ffta_ui::Role::ActionInfo
            | gba_core::ffta_ui::Role::Funds
    ) {
        size = size
            .min(148. / (node.bounds[3] - node.bounds[1]).max(1) as f32)
            .max(0.5);
    }
    let mut layout = match node.anchor {
        Anchor::Edge { right, vertical } => Layout::new_with_anchor(
            window,
            base,
            right,
            size,
            if node.role == gba_core::ffta_ui::Role::Dialogue {
                [style.offset_x, style.offset_y]
            } else {
                [0., 0.]
            },
            vertical,
        ),
        Anchor::Corner { right, bottom } => {
            Layout::at_corner(window, base, right, bottom, size, length, node.bounds)
        }
        Anchor::Center => Layout::centered(window, base, size),
        Anchor::Modal | Anchor::PanelCenter => {
            centered_ui_panel(window, base, size, node.bounds, false)
        }
        Anchor::TopCenter => centered_ui_panel(window, base, size, node.bounds, true),
    };
    if matches!(
        node.role,
        gba_core::ffta_ui::Role::Stats
            | gba_core::ffta_ui::Role::ActionInfo
            | gba_core::ffta_ui::Role::Funds
    ) {
        let margin = 6. * layout.clip[3] / 160.;
        let bottom = matches!(
            node.anchor,
            Anchor::Corner { bottom: true, .. } | Anchor::Edge { vertical: 160., .. }
        );
        layout.rect[1] += if bottom { -margin } else { margin };
    }
    layout
}
// Fixed native canvases preserve the original entry/exit animation within
// each panel, without moving its host origin when visible bounds change.
#[cfg(test)]
fn service_ui_layout(
    physical: (u32, u32),
    base_width: f32,
    size: f32,
    root: bool,
) -> screen::ui_layer::Layout {
    screen::ui_layer::Layout::new_with_anchor(
        physical,
        base_width,
        !root,
        size,
        [0., 0.],
        if root { 160. } else { 0. },
    )
}
#[cfg(test)]
fn pub_ui_layout(physical: (u32, u32), base_width: f32, size: f32) -> screen::ui_layer::Layout {
    screen::ui_layer::Layout::new_with_anchor(physical, base_width, true, size, [0., 0.], 0.)
}
fn menu_panel_native(index: usize, bounds: [i32; 4], length: f32) -> (f32, f32) {
    let distance = ((bounds[2] - bounds[0]) as f32 * (length.clamp(1., 2.) - 1.)).round();
    let shift = if bounds[0] + bounds[2] >= 240 {
        distance
    } else {
        -distance
    };
    ((index % 480) as f32 - 120. + shift, (index / 480) as f32)
}
#[cfg(test)]
mod gamepad_capture_simulation {
    use super::MenuTapFilter;
    use std::time::{Duration, Instant};

    #[test]
    fn steam_deck_y_shortcut_is_build_specific_and_mapping_is_present() {
        let linux_deck_rows: Vec<_> = crate::CONTROLLER_DB
            .lines()
            .filter(|line| line.contains(",Valve Steam Deck,") && line.ends_with("platform:Linux,"))
            .collect();
        assert_eq!(linux_deck_rows.len(), 2);
        assert!(linux_deck_rows.iter().all(|line| line.contains(",y:b6,")));

        let steam_virtual_linux = crate::CONTROLLER_DB
            .lines()
            .find(|line| {
                line.contains(",Steam Virtual Gamepad,") && line.ends_with("platform:Linux,")
            })
            .expect("Steam Input virtual pad mapping for Linux");
        assert!(steam_virtual_linux.contains(",y:b3,"));

        assert!(super::linux_menu_shortcut(true, false, false, false));
        assert!(super::linux_menu_shortcut(false, true, true, false));
        #[cfg(feature = "steamdeck")]
        assert!(super::linux_menu_shortcut(false, false, false, true));
        #[cfg(not(feature = "steamdeck"))]
        assert!(!super::linux_menu_shortcut(false, false, false, true));
    }

    #[test]
    fn shop_mouse_maps_categories_items_and_quantity_through_the_same_ui_canvas() {
        let mut state = gba_core::ffta_dialogue::DialogueState::default();
        state.shop_layout = true;
        assert_eq!(super::battle_menu_ui_layer(&state), Some(0));
        for window in [(960, 640), (1920, 1080), (2560, 720)] {
            for scale in [0.5, 1.] {
                let layout = super::pub_ui_layout(window, 568., scale);
                for (x, y) in [(24usize, 8usize), (40, 36), (170, 96)] {
                    let px = layout.rect[0] + (120. + x as f32 + 0.5) * layout.rect[2] / 480.;
                    let py = layout.rect[1] + (y as f32 + 0.5) * layout.rect[3] / 160.;
                    let index = layout.source(px, py).unwrap();
                    assert_eq!(
                        super::menu_panel_native(index, [0, 0, 238, 155], 1.),
                        (x as f32, y as f32)
                    );
                }
            }
        }
    }
    #[test]
    fn service_root_owns_left_edge_even_when_native_window_origin_is_centered() {
        use gba_core::ffta_ui::{Anchor, Node, Role};
        for x in [0, 32, 72, 128] {
            let bounds = [x, 76, x + 80, 160];
            let n = Node {
                slot: 1,
                role: Role::ServiceMenu,
                anchor: Anchor::Corner {
                    right: false,
                    bottom: true,
                },
                bounds,
                length: false,
            };
            let l = super::host_ui_layout((1920, 1080), 284., Default::default(), n);
            let edge = l.rect[0] + (120 + x) as f32 * l.rect[2] / 480.;
            assert!((edge - l.clip[0]).abs() < 1.);
        }
    }
    #[test]
    fn centered_cards_top_attack_names_and_bottom_margin_are_viewport_relative() {
        use gba_core::ffta_ui::{Anchor, Node, Role};
        let style = gba_core::ffta_dialogue::DialogueStyle::default();
        for (window, base) in [((1920, 1080), 284.), ((2560, 720), 568.)] {
            for (role, anchor, bounds) in [
                (Role::AttackName, Anchor::TopCenter, [0, 16, 96, 32]),
                (Role::Notice, Anchor::Modal, [16, 48, 224, 96]),
                (Role::Notice, Anchor::Modal, [40, 48, 200, 112]),
                (Role::ServiceContent, Anchor::PanelCenter, [72, 8, 216, 96]),
            ] {
                let l = super::host_ui_layout(
                    window,
                    base,
                    style,
                    Node {
                        slot: 0,
                        role,
                        anchor,
                        bounds,
                        length: false,
                    },
                );
                let k = l.rect[2] / 480.;
                let center = l.rect[0] + (120. + (bounds[0] + bounds[2]) as f32 * 0.5) * k;
                assert!((center - l.clip[0] - l.clip[2] * 0.5).abs() < 1.);
                if role == Role::AttackName {
                    assert!(
                        (l.rect[1] + bounds[1] as f32 * k - l.clip[1] - 6. * l.clip[3] / 160.)
                            .abs()
                            < 1.
                    );
                } else {
                    assert!(
                        (l.rect[1] + (bounds[1] + bounds[3]) as f32 * 0.5 * k
                            - l.clip[1]
                            - l.clip[3] * 0.5)
                            .abs()
                            < 1.
                    );
                }
            }
            let bounds = [152, 104, 240, 160];
            let n = Node {
                slot: 0,
                role: Role::ActionInfo,
                anchor: Anchor::Corner {
                    right: false,
                    bottom: true,
                },
                bounds,
                length: false,
            };
            let l = super::host_ui_layout(window, base, style, n);
            let bottom = l.rect[1] + 160. * l.rect[3] / 160.;
            assert!(bottom < l.clip[1] + l.clip[3] - 1.);
        }
    }
    #[test]
    fn auxiliary_partition_moves_pixels_once_and_clears_old_images() {
        let mut src = vec![[0, 0, 0, 16]; 480 * 160];
        src[120 * 480 + 120 + 200] = [31, 0, 16, 0];
        src[40 * 480 + 120 + 40] = [1, 0, 16, 0];
        let mut aux = Vec::new();
        super::extract_ui_rectangle(&mut src, &mut aux, [176, 112, 240, 136]);
        assert_eq!(src[120 * 480 + 320], [0, 0, 0, 16]);
        assert_eq!(aux[120 * 480 + 320], [31, 0, 16, 0]);
        assert_eq!(src[40 * 480 + 160], [1, 0, 16, 0]);
        super::extract_ui_rectangle(&mut src, &mut aux, [176, 112, 240, 136]);
        assert!(aux.iter().all(|p| *p == [0, 0, 0, 16]));
    }
    #[test]
    fn service_canvases_keep_menu_left_and_remaining_ui_right_at_every_size() {
        for window in [(1280, 720), (1920, 1080), (2560, 720)] {
            for size in [0.5, 1., 1.5, 2.] {
                let root = super::service_ui_layout(window, 568., size, true);
                let rest = super::service_ui_layout(window, 568., size, false);
                let k = root.rect[2] / 480.;
                assert!((root.rect[0] + 120. * k - root.clip[0]).abs() <= 1.);
                assert!((rest.rect[0] + 360. * k - rest.clip[0] - rest.clip[2]).abs() <= 1.);
                assert!((root.rect[1] + 160. * k - root.clip[1] - root.clip[3]).abs() <= 1.);
                // Native row movement stays inside the fixed canvas; no new
                // origin is derived from the changing visible pixel bounds.
                for y in [80., 92., 144.] {
                    let x = root.rect[0] + (120. + 24.5) * k;
                    let py = root.rect[1] + (y + 0.5) * k;
                    if let Some(index) = root.source(x, py) {
                        assert_eq!(
                            super::menu_panel_native(index, [0, 76, 96, 160], 1.),
                            (24., y)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn pub_menu_mouse_keeps_fixed_right_origin_through_animation() {
        use gba_core::ffta_dialogue::{pack_panel, DialogueState, UiPixel};
        let mut state = DialogueState::default();
        state.town_layout = true;
        assert_eq!(super::battle_menu_ui_layer(&state), Some(0));
        let bounds = [0, 0, 240, 160];
        let mut pixels = vec![UiPixel::default(); 480 * 160];
        pixels[132 * 480 + 120 + 100] = UiPixel::opaque(31);
        let mut packed = Vec::new();
        pack_panel(&pixels, &[], 1., true, &mut packed);
        for size in [(1280, 720), (2560, 720)] {
            let layout = super::pub_ui_layout(size, 568., 1.);
            let [left, top, width, height] = layout.rect;
            let native_right = left + 360. * width / 480.;
            assert!((native_right - layout.clip[0] - layout.clip[2]).abs() < 1.);
            let index = layout
                .source(left + 220.5 / 480. * width, top + 132.5 / 160. * height)
                .unwrap();
            assert_eq!(super::menu_panel_native(index, bounds, 1.), (100., 132.));
            assert_ne!(packed[index], [0; 4]);
        }
    }

    #[test]
    fn battle_menu_mouse_uses_foreground_panel_when_action_hud_is_also_present() {
        use gba_core::ffta_dialogue::{pack_panel, DialogueState, PanelKind, UiPixel};
        let mut state = DialogueState::default();
        state.action_layout = true;
        assert_eq!(super::battle_menu_ui_layer(&state), Some(0)); // confirmation path
        state.extra.kind = PanelKind::Actions;
        let bounds = [160, 84, 240, 148];
        state.extra.bounds = Some(bounds);
        assert_eq!(super::battle_menu_ui_layer(&state), Some(1));
        let mut backing = vec![UiPixel::default(); 480 * 160];
        for y in 84..148 {
            for x in 160..240 {
                backing[y * 480 + x + 120] = UiPixel::opaque(31);
            }
        }
        let mut pixels = backing.clone();
        let target = (192usize, 140usize);
        pixels[target.1 * 480 + target.0 + 120] = UiPixel::opaque(0x7fff);
        for window in [(960, 640), (1920, 1080), (2560, 720)] {
            for width in [240., 284., 568.] {
                for size in [0.75, 1., 1.5] {
                    for length in [1., 1.5, 2.] {
                        let mut packed = Vec::new();
                        pack_panel(&pixels, &backing, length, true, &mut packed);
                        let index = packed
                            .iter()
                            .position(|p| *p == UiPixel::opaque(0x7fff).packed())
                            .unwrap();
                        let layout = screen::ui_layer::Layout::at_corner(
                            window, width, true, true, size, length, bounds,
                        );
                        let point = (
                            layout.rect[0]
                                + (index % 480) as f32 * layout.rect[2] / 480.
                                + layout.rect[2] / 960.,
                            layout.rect[1]
                                + (index / 480) as f32 * layout.rect[3] / 160.
                                + layout.rect[3] / 320.,
                        );
                        let source = layout
                            .source(point.0, point.1)
                            .expect("drawn option must be hittable");
                        let (x, y) = super::menu_panel_native(source, bounds, length);
                        assert_eq!((x as usize, y as usize), target);
                        assert_eq!(((y - 84.) / 16.) as usize, 3);
                    }
                }
            }
        }
        state.extra.kind = PanelKind::None;
        assert_eq!(super::battle_menu_ui_layer(&state), Some(0));
    }

    #[test]
    fn patterned_menu_fills_extra_rows_with_background_only() {
        gba_core::mem::set_extra(164);
        gba_core::mem::set_extra_y(80);
        let mut bus = gba_core::MemMap::new(vec![]);
        bus.ffta_dialogue.menu_pattern = true;
        bus.io[8..10].copy_from_slice(&0xad0bu16.to_le_bytes());
        for e in bus.vram[0x6800..0x7800].chunks_exact_mut(2) {
            e.copy_from_slice(&0xa03cu16.to_le_bytes());
        }
        bus.vram[0x8780..0x87a0].fill(0x11);
        bus.palette[0x142..0x144].copy_from_slice(&31u16.to_le_bytes());
        let width = super::wide_w();
        bus.wide_framebuffer.fill(0x3e0);
        bus.wide_framebuffer[..width * 160].fill(0x7fff);
        let original = bus.wide_framebuffer.clone();
        let oam = bus.oam.clone();
        let mut padding = Vec::new();
        let frame = super::display_frame(&bus, &mut padding);
        assert!(frame[..width * 80].iter().all(|&p| p == 31));
        assert!(frame[width * 80..width * 240].iter().all(|&p| p == 0x7fff));
        assert!(frame[width * 240..].iter().all(|&p| p == 31));
        assert_eq!(bus.wide_framebuffer, original);
        assert_eq!(bus.oam, oam);
        gba_core::mem::set_extra(0);
        gba_core::mem::set_extra_y(0);
    }
    #[test]
    fn scene_without_zoom_rows_cannot_show_stale_object_pixels() {
        gba_core::mem::set_extra(164);
        gba_core::mem::set_extra_y(80);
        let mut bus = gba_core::mem::MemMap::new(vec![]);
        let width = super::wide_w();
        bus.wide_framebuffer.fill(31);
        bus.wide_framebuffer[..width * 160].fill(0x7fff);
        let original = bus.wide_framebuffer.clone();
        let oam = bus.oam.clone();
        let mut padding = Vec::new();
        let frame = super::display_frame(&bus, &mut padding);
        assert!(frame[..width * 80].iter().all(|&p| p == 0));
        assert!(frame[width * 80..width * 240].iter().all(|&p| p == 0x7fff));
        assert!(frame[width * 240..].iter().all(|&p| p == 0));
        assert_eq!(bus.wide_framebuffer, original);
        assert_eq!(bus.oam, oam);
        bus.ffta_stage.enabled = true;
        bus.io[..2].copy_from_slice(&0x0300u16.to_le_bytes());
        bus.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        bus.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
        let frame = super::display_frame(&bus, &mut padding);
        assert_eq!(frame.as_ptr(), bus.wide_framebuffer.as_ptr());
        gba_core::mem::set_extra(0);
        gba_core::mem::set_extra_y(0);
        let frame = super::display_frame(&bus, &mut padding);
        assert_eq!(frame, &bus.wide_framebuffer[..240 * 160]);
    }

    #[test]
    fn aspect_choices_recompute_render_limits_without_restarting() {
        for (aspect, extra) in [("3:2", 0), ("16:9", 22), ("32:9", 164), ("16:10", 8)] {
            let base = super::aspect_extra(aspect);
            assert_eq!(base, extra);
            let limit = super::zoom_limit(base);
            let zoom = super::ZOOM_STEP.powi(limit as i32);
            assert!((240 + 2 * base) as f32 * zoom <= (240 + 2 * gba_core::mem::MAX_EXTRA) as f32);
            assert!(160.0 * zoom <= (160 + 2 * gba_core::mem::MAX_EXTRA_Y) as f32);
        }
    }

    #[test]
    fn same_menu_control_is_ignored_for_20ms_but_other_controls_are_not() {
        let start = Instant::now();
        let mut filter = MenuTapFilter::default();
        assert!(filter.accept_at("menu:confirm", start));
        assert!(!filter.accept_at("menu:confirm", start + Duration::from_millis(19)));
        assert!(filter.accept_at("menu:down", start + Duration::from_millis(19)));
        assert!(filter.accept_at("menu:confirm", start + Duration::from_millis(20)));
    }

    #[cfg(windows)]
    #[test]
    fn simulated_native_xinput_a_press_assigns_once_and_does_not_confirm_menu() {
        use super::{xinput_new_press, xinput_pressed, XInputGamepad, XInputState};

        let mut av = input_config::AvConfig::default();
        let mut menu = menu::Menu::new(true, false);
        for _ in 0..5 {
            menu.handle(menu::MenuInput::Down, &mut av);
        }
        menu.handle(menu::MenuInput::Confirm, &mut av); // Input page.
        menu.handle(menu::MenuInput::Right, &mut av); // Gamepad tab.
        menu.handle(menu::MenuInput::Confirm, &mut av); // Capture GBA A.
        assert!(menu.capturing().is_some());

        let mut assignments = Vec::new();
        let mut confirms = 0;

        // XInput supplies a full state snapshot, not competing Pressed and
        // Changed event variants. A held A can only create one rising edge.
        let released = XInputState::default();
        let pressed = XInputState {
            gamepad: XInputGamepad {
                buttons: 0x1000,
                ..Default::default()
            },
            ..Default::default()
        };
        let captured = xinput_new_press(Some(released), pressed, 0.2);
        eprintln!(
            "SIM_XINPUT frame=1 new_assignment={} menu_confirm=false",
            captured.as_deref().unwrap_or("none")
        );
        if let Some(token) = captured {
            assignments.push(token);
            menu.end_capture();
        }
        for frame in 2..=4 {
            let captured = xinput_new_press(Some(pressed), pressed, 0.2);
            let confirm =
                captured.as_deref().is_some_and(|t| t == "South") && menu.capturing().is_none();
            confirms += usize::from(confirm);
            eprintln!(
                "SIM_XINPUT frame={frame} new_assignment={} menu_confirm={confirm}",
                captured.as_deref().unwrap_or("none")
            );
        }
        assert_eq!(assignments, ["South"]);
        assert_eq!(confirms, 0);
        assert!(menu.capturing().is_none());
        assert!(xinput_pressed(pressed, "South", 0.2));
        assert!(xinput_new_press(Some(pressed), released, 0.2).is_none());
    }
}
