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

#[path = "play_benchmark.rs"]
mod benchmark;

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use crate::perf::{Phase, PlayProfile};
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
    padding[..start].fill(0);
    padding[end..].fill(0);
    padding[start..end].copy_from_slice(&bus.wide_framebuffer[..width * 160]);
    padding
}

/// Snapshots do not encode the global stride of their wide scratch framebuffer.
/// Present the authoritative native image until a completed frame rebuilds the
/// current view; never run or mutate the restored machine to manufacture a preview.
fn restored_native_frame<'a>(bus: &gba_core::mem::MemMap, padding: &'a mut Vec<u16>) -> &'a [u16] {
    let (width, height) = (wide_w(), wide_h());
    let (x, y) = ((width - 240) / 2, (height - 160) / 2);
    padding.resize(width * height, 0);
    padding.fill(0);
    for (row, pixels) in bus.framebuffer.chunks_exact(240).take(160).enumerate() {
        let start = (y + row) * width + x;
        padding[start..start + 240].copy_from_slice(pixels);
    }
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
use winit::window::{Window, WindowId};

use recomp_core::fnv64;

use crate::{
    AudioStreams, BlockTable, FALLBACK_COLLECT, NO_INTERP, RING_TARGET, apply_av_change,
    arm_audio_hle, build_gilrs, ensure_native, load_bios_file, load_native, make_machine,
    print_fallback_census, record_labels, rom_sha256, run_frame_native, start_audio,
    status_line, validate_rom, write_sav,
};
#[cfg(not(target_os = "linux"))]
use crate::pad_pressed;

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
    /// Open the menu from gameplay: configurable keyboard/pad shortcuts,
    /// with Escape and Guide/Mode retained as fallback controls.
    open: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveController {
    Gilrs(gilrs::GamepadId),
    DirectInput,
}

/// A mouse event queued while the pause menu is open (physical window px).
#[derive(Clone, Copy)]
enum MouseEv {
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
        "Escape" => Escape,
        "LeftControl" => ControlLeft,
        "RightControl" => ControlRight,
        "LeftAlt" => AltLeft,
        "RightAlt" => AltRight,
        _ => return None,
    })
}

/// Reverse of [`winit_key`]: the canonical config name for a physical key,
/// or `None` for keys the config format can't express (F-keys, Escape, ...).
fn key_name(code: KeyCode) -> Option<&'static str> {
    const NAMES: &[&str] = &[
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
        "LeftControl", "RightControl", "LeftAlt", "RightAlt",
    ];
    NAMES.iter().copied().find(|n| winit_key(n) == Some(code))
}

/// All play-session state. Pre-window state is built in [`cmd_play`] and moved
/// in; the window, GPU presenter, and software-blit surface are created in
/// [`ApplicationHandler::resumed`] (window creation is only valid once the
/// event loop is active).
use crate::debug_control::ReplayMode as ReplayKind;

struct PlayApp {
    // Core machine + persistence.
    machine: Machine,
    sav_path: String,
    diagnosis: crate::diagnostics::Request,
    diagnosis_clipboard: Option<arboard::Clipboard>,
    debug_session: bool,
    debug_catalog: Option<crate::debug_catalog::Catalog>,
    content_picker: crate::content_picker::Picker,
    workbench: crate::debug_workbench::Workbench,
    replay_path: Option<std::path::PathBuf>,
    replay_job: Option<crate::debug_replay::Job>,
    replay_terminal: Option<Result<crate::debug_replay::Report,String>>,
    cheat_refresh: Option<crate::debug_cheats::Refresh>,
    cheat_snapshot: Option<crate::debug_cheats::Snapshot>,
    cheat_preview: Option<serde_json::Value>,
    last_cheat_once: Option<serde_json::Value>,
    replay_preparation_seen:usize,
    replay_message: String,
    game_saves: crate::game_saves::GameSaves,
    state_slots: crate::save_state::Slots,
    save_backups: crate::save_backups::Backups,
    boot_state: Vec<u8>,
    sav_seen: Option<u64>,
    record: Option<(std::fs::File, u16)>,
    capture: Option<crate::debug_recording::Capture>,
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
    modal_input: crate::modal_input::ModalInput,
    active_controller: Option<ActiveController>,
    last_direct_activity: Option<(String, Vec<String>)>,
    pending_pad: Option<String>,
    menu_key_capture: input_config::ComboCapture,
    menu_pad_capture: input_config::ComboCapture,
    menu_key_latch: input_config::ComboLatch,
    menu_pad_latch: input_config::ComboLatch,
    menu_shortcut_held: bool,
    #[cfg(not(target_os = "linux"))]
    gilrs_gestures: HashMap<gilrs::GamepadId, input_config::PadGesture>,
    #[cfg(target_os = "linux")]
    linux_pads: HashMap<gilrs::GamepadId, crate::linux_pad::Pad>,
    direct_gestures: HashMap<String, input_config::PadGesture>,
    pad_zoom_prev: (bool, bool),
    pad_ff_held: bool,
    pad_ff_toggle_prev: bool,
    atlas_down: [bool;7],
    atlas_focus_prev: bool,
    atlas_reference_prev: bool,
    atlas_information_prev: bool,
    atlas_shortcuts: menu::shortcuts::ControllerShortcuts,
    atlas_clock: Instant,
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
    ui_pixels: crate::ui_pixels::Pixels,
    panel_pixels: crate::ui_pixels::Pixels,
    padded_frame: Vec<u16>,

    // Window + presentation (created in `resumed`).
    title: String,
    status: bool,
    window: Option<Arc<Window>>,
    fullscreen: crate::fullscreen::Controller,
    presenter: Option<screen::present::Presenter>,
    sb_context: Option<softbuffer::Context<Arc<Window>>>,
    sb_surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,

    // Menu / pause.
    menu_enabled: bool,
    menu: menu::Menu,
    paused: bool,
    frame_control: crate::debug_control::Control,
    probe: Option<crate::debug_probe::Search>,
    probe_result: Option<serde_json::Value>,
    inspection: Option<serde_json::Value>,
    graphics_report: Option<String>,
    ui_trace: crate::debug_ui::Trace,
    events: crate::debug_events::Hub,
    event_console: std::rc::Rc<std::cell::RefCell<crate::debug_console::Window>>,
    event_console_panel: Option<crate::debug_console_panel::Panel>,
    command_console_panel: Option<crate::debug_command_panel::Panel>,
    command_pad_handoff: Option<u8>,
    selections: crate::debug_events::Selections,
    lifecycle: crate::debug_lifecycle::Jobs,
    audio_diagnostic: bool,
    last_audio_report: Option<String>,
    code_trace: Option<crate::debug_code::Trace>,
    gameplay_trace: Option<crate::debug_gameplay::Trace>,
    last_gameplay_report: Option<String>,
    recent: Option<crate::debug_recent::Buffer>,
    last_recent_report: Option<String>,
    last_code_report: Option<String>,
    last_issue_report: Option<String>,
    control_server: Option<crate::debug_control::Server>,
    menu_dirty: bool,
    cursor_visible: bool,
    suppress: u16,
    nav_prev: NavEdges,

    // Perf + audio watchdog.
    show_stats: bool,
    fps: crate::fps::Meter,
    profile: PlayProfile,
    benchmark: Option<benchmark::Benchmark>,
    emu_ema_ms: f64,
    frames_run: u64,
    slow_warned: bool,
    native_run: u64,
    fallback_run: u64,
    primed: bool,
    clock_pacing: bool,
    next_frame: Instant,
    audio_watch: Instant,
    audio_consumed_last: u64,
    audio_stalls: u8,
    audio_retry: Option<Instant>,

    // Fast forward: remappable hold/toggle activation and selected audio mode.
    ff_active: bool,
    ff_latched: bool,
    ff_speed: u32,
    ff_clock: input_config::fast_forward::FrameClock,
    ff_audio_active: String,
    /// Last key pressed while the menu is open (consumed by key capture).
    pending_key: Option<KeyCode>,

    /// Extra columns per side of the starting view (x1): the aspect chosen
    /// in the launcher. The starting view is (240 + 2 * base_extra) x 160 and
    /// every zoom level keeps exactly that aspect ratio.
    base_extra: usize,
    /// Zoom step index: zoom = ZOOM_STEP ^ zoom_level (0 = x1).
    zoom_level: u32,
    zoom_max: u32,
    /// Crop shown, [x0, y0, w, h] in the rendered frame's pixels. Updated
    /// together with the render size (sync_view) so they never disagree.
    view: [f32; 4],
    /// Last cursor position in physical window pixels.
    mouse_pos: Option<(f64, f64)>,
    game_mouse: gba_core::ffta_mouse::LanguageMouse,
    game_keys: u16,
    /// Mouse events received while paused, consumed by the menu each tick.
    mouse_queue: Vec<(MouseEv,Option<crate::debug_events::Correlation>)>,

    /// A fatal error raised inside a callback; surfaced after `run_app`.
    error: Option<String>,
}

impl PlayApp {
    fn menu_text(&self,key:&str,args:&[(&str,String)])->String {
        menu::i18n::format(self.av.menu_language,key,args)
    }

    fn set_fullscreen(&mut self, enabled: bool) {
        let Some(window) = &self.window else { return; };
        self.fullscreen.request(window, enabled);
        self.av.video_fullscreen = enabled;
        self.menu_dirty = true;
        self.profile.presentation_boundary();
        if let Err(error) = self.av.save() {
            eprintln!("menu: could not save fullscreen preference ({error})");
        }
    }

    /// Width of the starting (x1) view, in frame pixels.
    fn base_w(&self) -> f32 {
        (240 + 2 * self.base_extra) as f32
    }

    /// Zoom by `steps` (positive = zoom out). x1 is the floor: it never goes
    /// below the starting view, so the aspect ratio never changes.
    fn zoom_by(&mut self, steps: i32) {
        let next = (self.zoom_level as i32 + steps).clamp(0, self.zoom_max as i32) as u32;
        if next != self.zoom_level {
            self.zoom_level = next;
            eprintln!("zoom: x{:.2}", ZOOM_STEP.powi(next as i32));
        }
    }

    /// Make the core render exactly what the current zoom shows, resize the
    /// buffers if that changed, and recompute the crop. Called at the start
    /// of every emulated frame, never in the middle of one.
    fn sync_view(&mut self) {
        let z = if self.zoom_level == 0 {
            1.0
        } else {
            ZOOM_STEP.powi(self.zoom_level as i32)
        };
        let vw = self.base_w() * z;
        let vh = 160.0 * z;
        let ex = (((vw - 240.0) / 2.0).ceil().max(0.0) as usize).min(gba_core::mem::MAX_EXTRA);
        let ey = (((vh - 160.0) / 2.0).ceil().max(0.0) as usize).min(gba_core::mem::MAX_EXTRA_Y);
        if ex != gba_core::mem::extra() || ey != gba_core::mem::extra_y() {
            self.profile.presentation_boundary();
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

    fn game_mouse_aim(&mut self,pos:(f64,f64),confirm:bool)->bool {
        let ui_index=battle_menu_ui_layer(&self.machine.bus.ffta_dialogue);
        if (gba_core::ffta_battle_mouse::MenuScene::read(&self.machine.bus).is_some() || self.machine.bus.ffta_dialogue.shop_layout) && ui_index.is_some() {
            if let Some(window)=&self.window {
                let size=window.inner_size();
                if let Some(layout)=self.ui_layout((size.width.max(1),size.height.max(1)),ui_index.unwrap()) {
                    if let Some(index)=layout.source(pos.0 as f32,pos.1 as f32) {
                        let state=&self.machine.bus.ffta_dialogue;
                        if state.town_layout || state.shop_layout || state.site_layout || state.extra.kind==gba_core::ffta_dialogue::PanelKind::Notice {
                            let x=(index%480) as f32-120.;
                            let y=(index/480) as f32;
                            return gba_core::ffta_battle_mouse::aim(&mut self.machine.bus,x,y,confirm);
                        }
                        let bounds=if ui_index==Some(0) {gba_core::ffta_dialogue::panel_bounds(&state.pixels,&state.backing)}else{gba_core::ffta_dialogue::panel_bounds(&state.extra.pixels,&state.extra.backing)};
                        if let Some(b)=bounds {
                            let (x,y)=menu_panel_native(index,b,if state.town_layout || state.shop_layout {1.}else{state.style.length});
                            return gba_core::ffta_battle_mouse::aim(&mut self.machine.bus,x,y,confirm);
                        }
                    }
                    self.machine.bus.ffta_battle_mouse.cancel();
                    return false;
                }
            }
        }
        if let Some((x,y))=self.window_to_frame(pos) {
            let x=x as f32-gba_core::mem::extra()as f32;let y=y as f32-gba_core::mem::extra_y()as f32;
            if self.game_mouse.scene(&self.machine.bus).is_some(){self.game_mouse.aim(&self.machine.bus,x,y,confirm)}
            else{gba_core::ffta_battle_mouse::aim(&mut self.machine.bus,x,y,confirm)}
        }else{self.game_mouse.cancel();self.machine.bus.ffta_battle_mouse.cancel();false}
    }

    fn choose_rom(&mut self)->bool {
        if self.debug_session {
            self.menu.session_result("Close the debug session to return to the normal game before switching ROM.".into());
            return false;
        }
        let Some(path)=crate::rom_switch::pick(&self.sav_path)else{return false;};
        let result=crate::rom_switch::validate(&path).and_then(|_| {
            // Flush the current cartridge before the replacement process reads
            // any save. A failed save or launch keeps this paused game alive.
            if let Some(data)=self.machine.bus.save_data() {
                write_sav(&self.sav_path,data).map_err(|e|format!("Cannot save the current game: {e}"))?;
            }
            crate::rom_switch::launch(&path,&self.sav_path)
        });
        match result {Ok(())=>true,Err(e)=>{crate::rom_switch::show_error(&e);false}}
    }
    fn ui_layout(&self,physical:(u32,u32),index:usize)->Option<screen::ui_layer::Layout> {
        let pixels = if index == 1 { &self.panel_pixels } else { &self.ui_pixels };
        if pixels.is_empty() { return None; }
        let state=&self.machine.bus.ffta_dialogue;
        if index==1 {
            if !state.separate || !state.extra.active() {return None;}
            if state.extra.kind==gba_core::ffta_dialogue::PanelKind::Notice {
                return Some(screen::ui_layer::Layout::centered_panel(physical,self.base_w(),state.style.scale,state.extra.bounds?));
            }
            let b=if state.extra.kind==gba_core::ffta_dialogue::PanelKind::Actions {gba_core::ffta_dialogue::panel_bounds(&state.extra.pixels,&state.extra.backing)?}else{gba_core::ffta_dialogue::pixel_bounds(&state.extra.pixels)?};
            return Some(screen::ui_layer::Layout::at_corner(physical,self.base_w(),state.town_layout || b[0]+b[2]>=240,b[1]+b[3]>=160,state.style.scale,if state.town_layout {1.}else{state.style.length},b));
        }
        let (_,right)=state.layer()?;
        if state.action_layout {
            let b=gba_core::ffta_dialogue::panel_bounds(&state.pixels,&state.backing)?;
            Some(screen::ui_layer::Layout::at_corner(physical,self.base_w(),false,true,state.style.scale,state.style.length,b))
        } else if state.shop_layout || state.site_layout {
            Some(screen::ui_layer::Layout::centered(physical,self.base_w(),state.style.scale))
        } else if state.town_layout || state.shop_layout {
            // One fixed native canvas: animated portraits/cursor highlights
            // must never change the origin of the Pub UI or its mouse targets.
            Some(pub_ui_layout(physical,self.base_w(),state.style.scale))
        } else {
            Some(screen::ui_layer::Layout::new_with_anchor(physical,self.base_w(),right,state.style.scale,
                [state.style.offset_x,state.style.offset_y],state.vertical_anchor()))
        }
    }
    fn ui_frame(&self,layout:screen::ui_layer::Layout,index:usize)->screen::ui_layer::Frame<'_> {
        screen::ui_layer::Frame {grid:Some(self.grid_params),pixels_revision:if index==0 {self.ui_pixels.revision()}else{self.panel_pixels.revision()},pixels:if index==0 {&self.ui_pixels}else{&self.panel_pixels},
            raw_world:if index==1 && self.machine.bus.ffta_dialogue.extra.kind==gba_core::ffta_dialogue::PanelKind::Stats {&self.machine.bus.ffta_dialogue.extra.background}else{&self.machine.bus.wide_framebuffer},
            world_dims:(wide_w() as u32,wide_h() as u32),lut:self.lut.table.as_slice(),
            layout,world_view:self.view}
    }

    /// Present `self.rgba` at the window's current size: the GPU grid/scaling
    /// pass when available, otherwise the software letterbox blit (still fully
    /// color-corrected, no grid). Cloning the `Arc` avoids borrowing
    /// `self.window` while `self.presenter`/`self.sb_surface` are mutated.
    fn present_rgba(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        self.profile.prepare_presentation(self.machine.bus.frames,
            !self.paused && (self.primed || self.stream.is_none()) && !self.ff_active && size.width != 0 && size.height != 0);
        // Do not enqueue UI/world copies or construct software surfaces while
        // minimized to zero size. Guest/audio progress remains independent.
        if size.width == 0 || size.height == 0 {
            if self.benchmark.is_some() {
                self.error = Some("GUI benchmark skipped a presentation".into());
            }
            return;
        }
        let phys = (size.width, size.height);
        let frame_dims = if self.paused {
            // The window can resize while its menu canvas is being drawn.
            // Upload the dimensions of that completed canvas, then scale to
            // the latest physical window size; never reinterpret its stride.
            (self.view[2] as u32, self.view[3] as u32)
        } else {
            (wide_w() as u32, wide_h() as u32)
        };
        if let Some(mut p) = self.presenter.take() {
            p.enable_cpu_profiling(self.profile.enabled() && !self.paused);
            if !self.paused {
                let ui_start = self.profile.start();
                let layers = [
                    self.ui_layout(phys, 0).map(|layout| self.ui_frame(layout, 0)),
                    self.ui_layout(phys, 1).map(|layout| self.ui_frame(layout, 1)),
                    self.fps.frame(phys, window.scale_factor()),
                ];
                p.set_ui_layers(&layers);
                self.profile.record_ui_upload(p.ui_uploaded_bytes());
                self.profile.finish(Phase::UiUpload, ui_start);
            } else {p.set_ui_layer(None);}
            let mut grid = self.grid_params;
            if self.paused {
                grid.strength = 0.;
                grid.scale_mode = 1;
            }
            let result = p.present(&self.rgba, frame_dims, (size.width, size.height), &grid, self.view);
            if let Some(times) = p.cpu_timings() {
                self.profile.record(Phase::GpuSetupUpload, times.setup_upload);
                self.profile.record(Phase::GpuAcquire, times.acquire);
                self.profile.record(Phase::GpuEncode, times.encode);
                self.profile.record(Phase::GpuSubmit, times.submit);
                self.profile.record(Phase::GpuPresent, times.present);
                self.profile.record(Phase::GpuReadback, times.timestamp_readback);
            }
            if let Some(batch) = p.take_gpu_timings() { self.profile.record_gpu(batch); }
            match result {
                Ok(outcome) => {
                    if self.benchmark.is_some() && outcome == screen::present::PresentOutcome::Skipped {
                        self.error = Some("GUI benchmark skipped a presentation".into());
                    }
                    self.presenter = Some(p);
                    if outcome == screen::present::PresentOutcome::Skipped { self.profile.presentation_skipped(); }
                    if outcome == screen::present::PresentOutcome::Presented && !self.paused {
                        let now = Instant::now();
                        self.fps.presented(now);
                        self.profile.presented(now, false);
                    }
                    return;
                },
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
        self.present_software(&window, phys);
    }

    /// Software fallback (no GPU / presenter lost): nearest-neighbor, aspect-
    /// correct letterbox of the visible crop into the window via softbuffer,
    /// matching the GPU presenter's centering. Lazily creates the surface.
    fn present_software(&mut self, window: &Arc<Window>, (w, h): (u32, u32)) {
        let ui_layouts=std::array::from_fn::<_,2,_>(|i|if self.paused {None}else{self.ui_layout((w,h),i)});
        // Zoom crop, computed before `surf` borrows `self` mutably.
        let [vx0, vy0, vw, vh] = self.view;
        let (stride, frame_h) = if self.paused {
            (vw as usize, vh as usize)
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
        for (i,layout) in ui_layouts.into_iter().enumerate() {if let Some(layout)=layout {
            let ui=screen::ui_layer::Frame {grid:None,pixels_revision:None,pixels:if i==0 {&self.ui_pixels}else{&self.panel_pixels},raw_world:if i==1 && self.machine.bus.ffta_dialogue.extra.kind==gba_core::ffta_dialogue::PanelKind::Stats {&self.machine.bus.ffta_dialogue.extra.background}else{&self.machine.bus.wide_framebuffer},
                world_dims:(wide_w() as u32,wide_h() as u32),lut:self.lut.table.as_slice(),layout,world_view:self.view};
            let x0=layout.rect[0].max(layout.clip[0]).max(0.) as usize;
            let y0=layout.rect[1].max(layout.clip[1]).max(0.) as usize;
            let x1=(layout.rect[0]+layout.rect[2]).min(w as f32).max(0.) as usize;
            let y1=(layout.rect[1]+layout.rect[3]).min(h as f32).max(0.) as usize;
            for y in y0..y1 {for x in x0..x1 {
                if let Some(c)=ui.sample(x as f32+0.5,y as f32+0.5) {
                    buffer[y*w as usize+x]=(c[0] as u32)<<16|(c[1] as u32)<<8|c[2] as u32;
                }
            }}
        }
        }
        if !self.paused {
            if let Some(hud) = self.fps.frame((w, h), window.scale_factor()) {
                let [x, y, width, height] = hud.layout.clip;
                for py in y as usize..((y + height) as usize).min(h as usize) {
                    for px in x as usize..((x + width) as usize).min(w as usize) {
                        if let Some(c) = hud.sample(px as f32 + 0.5, py as f32 + 0.5) {
                            buffer[py * w as usize + px] = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
                        }
                    }
                }
            }
        }
        let presented = buffer.present().is_ok();
        if presented && !self.paused {
            let now = Instant::now();
            self.fps.presented(now);
            self.profile.presented(now, true);
        } else if !presented { self.profile.presentation_skipped(); }
    }

    /// Gather this frame's down-state into the active-low GBA `keys` mask and
    /// the host-menu [`NavEdges`], from the held-key set plus the gamepad.
    fn gather_input(&mut self) -> (u16, NavEdges) {
        let capture_combo = self.menu.capturing() == Some(menu::Bind::GamepadMenu);
        let mut combo_snapshot = None;
        let mut pad_shortcut = (false, false);
        let key_shortcut = input_config::combo_state(&self.icfg.menu_key, |token| {
            winit_key(token).is_some_and(|key| self.held.contains(&key))
        });
        let key_menu_held = self.menu_key_latch.update(key_shortcut.0, key_shortcut.1);
        // KEYINPUT is active-low.
        let mut keys = 0x3FFu16;
        let mut pad_zoom_in = false;
        let mut pad_zoom_out = false;
        let mut pad_ff_held = false;
        let mut pad_ff_toggle = false;
        self.atlas_down=[false;7];
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
            open: self.held.contains(&KeyCode::Escape) || key_menu_held,
        };
        let capture_pad = matches!(
            self.menu.capturing(),
            Some(
                menu::Bind::Gamepad(_)
                    | menu::Bind::GamepadMenu
                    | menu::Bind::GamepadZoomIn
                    | menu::Bind::GamepadZoomOut
                    | menu::Bind::GamepadFastForward
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
                    gesture.arm_capture(capture_pad && !capture_combo && !self.directinput_only);
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
                            if self.active_controller == Some(ActiveController::Gilrs(event.id)) {
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
                            if button != gilrs::Button::Unknown && value >= activity_threshold =>
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
                    let connected: Vec<_> = g.gamepads().map(|(id,_)|id).collect();
                    self.linux_pads.retain(|id,_|connected.contains(id));
                    for (id, gamepad) in g.gamepads() {
                        let pad=self.linux_pads.entry(id).or_default();
                        let snapshot=crate::linux_pad::Snapshot::read(&gamepad,self.icfg.stick_deadzone,pad.state);
                        let update=pad.poll(snapshot,capture_pad && !capture_combo,now);
                        if self.pending_pad.is_none() {
                            self.pending_pad=update.capture.map(str::to_owned);
                        }
                        if update.activity {
                            self.active_controller=Some(ActiveController::Gilrs(id));
                            gilrs_activity=true;
                            gilrs_active_name=Some(gamepad.name().to_owned());
                            gilrs_active_layout=Some(input_config::GamepadLayout::Xbox);
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

        let mut hint_pad_activity=gilrs_activity;

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
                hint_pad_activity=true;
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
            gesture.arm_capture(capture_pad && !capture_combo && direct_requested);
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
            hint_pad_activity |= activity_slot.is_some();
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
            if self.events_enabled(){self.ui_event("controller-poll",serde_json::json!({"backend":"XInput","tokens":all_tokens}));}
            let any_down = !all_tokens.is_empty();
            let mut captured = None;
            if capture_pad && !capture_combo && !capture_locked {
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
                    if capture_combo {
                        combo_snapshot = Some((format!("xinput:{slot}"), xinput_tokens(state, activity_threshold)));
                    }
                    let blocked =
                        capture_locked || self.xinput.as_ref().is_some_and(|x| x.capture_locked);
                    if !blocked {
                        let dz = self.icfg.stick_deadzone;
                        let pressed = |name: &str| xinput_pressed(state, name, dz);
                        self.atlas_down=[pressed("LeftTrigger"),pressed("RightTrigger"),pressed("LeftTrigger2"),pressed("RightTrigger2"),pressed("West"),pressed("North"),pressed("RightThumb")];
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
                        pad_shortcut = input_config::combo_state(&self.icfg.menu_pad, pressed);
                        pad_shortcut.0 |= pressed("Mode");
                        pad_shortcut.1 |= pressed("Mode");
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
            if let Some((id, _gp)) = chosen {
                #[cfg(not(target_os = "linux"))]
                let gp = _gp;
                standard_selected = true;
                #[cfg(not(target_os = "linux"))]
                let pad_blocked = self
                    .gilrs_gestures
                    .get(&id)
                    .is_some_and(input_config::PadGesture::blocked);
                #[cfg(target_os = "linux")]
                let pad_blocked=self.linux_pads.get(&id).is_some_and(crate::linux_pad::Pad::blocked);
                #[cfg(not(target_os = "linux"))]
                let dz = self.icfg.stick_deadzone;
                    #[cfg(not(target_os = "linux"))]
                    let pressed=|name:&str|pad_pressed(&gp,name,dz);
                    #[cfg(target_os = "linux")]
                    let pressed=|name:&str|self.linux_pads.get(&id).is_some_and(|pad|pad.state.pressed(name));
                if capture_combo {
                    combo_snapshot = Some((format!("gilrs:{id:?}"), input_config::PAD_CONTROLS.iter().filter(|token| pressed(token)).map(|s| s.to_string()).collect()));
                }
                if !pad_blocked {
                    self.atlas_down=[pressed("LeftTrigger"),pressed("RightTrigger"),pressed("LeftTrigger2"),pressed("RightTrigger2"),pressed("West"),pressed("North"),pressed("RightThumb")];
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
                            let directions=if src.uses_right_stick(){
                                [("RightStickRight",4),("RightStickLeft",5),("RightStickUp",6),("RightStickDown",7)]
                            }else{
                                [("LeftStickRight",4),("LeftStickLeft",5),("LeftStickUp",6),("LeftStickDown",7)]
                            };
                            for (token,bit) in directions {
                                if pressed(token){keys &= !(1<<bit);}
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
                        open: false,
                    };
                    #[cfg(target_os = "linux")]
                    let pad_nav=NavEdges {
                        up:pressed("DPadUp")||pressed("LeftStickUp"),
                        down:pressed("DPadDown")||pressed("LeftStickDown"),
                        left:pressed("DPadLeft")||pressed("LeftStickLeft"),
                        right:pressed("DPadRight")||pressed("LeftStickRight"),
                        confirm:pressed("South"),cancel:pressed("East"),
                        open:false,
                    };
                    nav.up |= pad_nav.up;
                    nav.down |= pad_nav.down;
                    nav.left |= pad_nav.left;
                    nav.right |= pad_nav.right;
                    nav.confirm |= pad_nav.confirm;
                    nav.cancel |= pad_nav.cancel;
                    // The configured shortcut works even when the desktop
                    // intercepts Guide. Escape remains available too.
                    pad_shortcut = input_config::combo_state(&self.icfg.menu_pad, pressed);
                    pad_shortcut.0 |= pressed("Mode");
                    pad_shortcut.1 |= pressed("Mode");
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
                if capture_combo { combo_snapshot = Some((gp.name.clone(), gp.capture_tokens())); }
                let pad_blocked = self
                    .direct_gestures
                    .get(&gp.name)
                    .is_some_and(input_config::PadGesture::blocked);
                let dz = self.icfg.stick_deadzone;
                if !pad_blocked {
                    self.atlas_down=[gp.pressed("LeftTrigger",dz),gp.pressed("RightTrigger",dz),gp.pressed("LeftTrigger2",dz),gp.pressed("RightTrigger2",dz),gp.pressed("West",dz),gp.pressed("North",dz),gp.pressed("RightThumb",dz)];
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
                        open: false,
                    };
                    nav.up |= pad_nav.up;
                    nav.down |= pad_nav.down;
                    nav.left |= pad_nav.left;
                    nav.right |= pad_nav.right;
                    nav.confirm |= pad_nav.confirm;
                    nav.cancel |= pad_nav.cancel;
                    pad_shortcut = input_config::combo_state(&self.icfg.menu_pad, |token| gp.pressed(token, dz));
                    pad_shortcut.0 |= gp.pressed("Mode", dz);
                    pad_shortcut.1 |= gp.pressed("Mode", dz);
                }
            }
        }
        if capture_combo {
            self.pending_pad = None;
            if let Some((source, controls)) = combo_snapshot {
                self.pending_pad = self.menu_pad_capture.poll(&source, controls);
            } else { self.menu_pad_capture = input_config::ComboCapture::default(); }
        } else { self.menu_pad_capture = input_config::ComboCapture::default(); }
        let pad_menu_held = self.menu_pad_latch.update(pad_shortcut.0, pad_shortcut.1);
        self.menu_shortcut_held = key_menu_held || pad_menu_held;
        // Capture already suppresses toggle handling. Preserve the held state
        // here so cancelling capture cannot turn it into a fresh menu press.
        nav.open |= pad_menu_held;
        if key_menu_held || pad_menu_held {
            nav.up = false; nav.down = false; nav.left = false; nav.right = false;
            nav.confirm = false; nav.cancel = false;
        }
        let command_focus=self.command_console_panel.as_ref().is_some_and(|p|p.focused());
        if command_focus && !self.modal_input.blocked() {
            let input=crate::debug_command::controller::Input{up:nav.up,down:nav.down,left:nav.left,right:nav.right,confirm:nav.confirm,cancel:nav.cancel};
            let action=self.command_console_panel.as_mut().unwrap().controller_input(input);
            match action {
                crate::debug_command_panel::ControllerEvent::Complete=>{
                    let profiles=self.content_picker.completion_ids(menu::ContentSlot::Replay);let catalogs=self.content_picker.completion_ids(menu::ContentSlot::Catalog);let cheats=self.content_picker.completion_ids(menu::ContentSlot::CheatCatalog);
                    let requests=self.control_server.as_ref().and_then(|s|s.pending_status()["request_id"].as_str().map(str::to_owned)).into_iter().collect::<Vec<_>>();
                    self.command_console_panel.as_mut().unwrap().complete(&profiles,&catalogs,&cheats,&requests);
                },
                crate::debug_command_panel::ControllerEvent::Hide=>{
                    self.command_console_panel.as_mut().unwrap().hide();self.command_pad_handoff=Some(0);
                },
                crate::debug_command_panel::ControllerEvent::None=>{},
            }
        }
        if command_focus || self.command_pad_handoff.is_some() {
            let neutral=keys==0x3ff && !(nav.up||nav.down||nav.left||nav.right||nav.confirm||nav.cancel||nav.open)
                && !(pad_zoom_in||pad_zoom_out||pad_ff_held||pad_ff_toggle) && !self.atlas_down.iter().any(|v|*v);
            if !command_focus {
                let polls=if neutral{self.command_pad_handoff.unwrap_or(0).saturating_add(1)}else{0};
                self.command_pad_handoff=if polls>=2{None}else{Some(polls)};
            }
            keys=0x3ff;nav=NavEdges::default();self.atlas_down=[false;7];self.menu_shortcut_held=false;
            self.pending_pad=None;self.pad_zoom_prev=(pad_zoom_in,pad_zoom_out);self.pad_ff_toggle_prev=pad_ff_toggle;
            pad_zoom_in=false;pad_zoom_out=false;pad_ff_held=false;pad_ff_toggle=false;
        }
        if capture_pad || key_menu_held || pad_menu_held {
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
        if !self.paused && pad_zoom_out && !self.pad_zoom_prev.1 {
            self.zoom_by(1);
        }
        if !command_focus && self.command_pad_handoff.is_none() {self.pad_zoom_prev = (pad_zoom_in, pad_zoom_out);}
        if !self.paused && pad_ff_toggle && !self.pad_ff_toggle_prev {
            self.ff_latched = !self.ff_latched;
        }
        if !command_focus && self.command_pad_handoff.is_none() {self.pad_ff_toggle_prev = pad_ff_toggle;}
        self.pad_ff_held = pad_ff_held;
        if self.events_enabled(){
            let mut held=self.held.iter().map(|k|format!("{k:?}")).collect::<Vec<_>>();held.sort();
            self.ui_event("mapped-poll",serde_json::json!({"held_keyboard":held,"mapped_keys":keys,"controller_backend":if native_xinput{"XInput"}else if direct_requested{"DirectInput"}else if self.pad.is_some(){"Gilrs"}else{"unavailable"},"controller_device":self.icfg.gamepad_name,"navigation_source":"Keyboard OR selected controller; simultaneous sources not causally separated","nav":[nav.up,nav.down,nav.left,nav.right,nav.confirm,nav.cancel,nav.open],"rising_confirm":nav.confirm&&!self.nav_prev.confirm,"paused":self.paused,"suppression":self.suppress,"binding_capture":format!("{:?}",self.menu.capturing()),"menu_shortcut":self.menu_shortcut_held,"menu":self.ui_focus()}));
        }
        if hint_pad_activity {self.menu_dirty |= self.menu.note_input(input_config::Device::Gamepad);}
        (keys, nav)
    }

    /// Rebuild the key -> GBA button table after the menu rebinds a key.
    fn rebuild_key_pairs(&mut self) {
        if self.events_enabled(){self.ui_event("bindings-changed",serde_json::json!({"input_config":self.icfg.serialize()}));}
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

    fn refresh_ui_layers(&mut self) {
        let state = &self.machine.bus.ffta_dialogue;
        if let Some((pixels, _)) = state.layer() {
            if state.action_layout {
                self.ui_pixels.panel(pixels, &state.backing, state.style.length, false);
            } else if state.town_layout || state.shop_layout {
                self.ui_pixels.panel(pixels, &[], 1., true);
            } else {
                self.ui_pixels.raw(pixels);
            }
        } else {
            self.ui_pixels.clear();
        }
        let extra = &state.extra;
        if state.separate && extra.active() {
            let b = gba_core::ffta_dialogue::panel_bounds(&extra.pixels, &extra.backing).unwrap_or([0; 4]);
            self.panel_pixels.panel(
                &extra.pixels, &extra.backing,
                if state.town_layout || extra.kind == gba_core::ffta_dialogue::PanelKind::Notice { 1. } else { state.style.length },
                state.town_layout || b[0] + b[2] >= 240,
            );
        } else {
            self.panel_pixels.clear();
        }
    }

    fn start_debug_session(&mut self) {
        self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"copy-requested",None,serde_json::json!({"scope":"Existing isolated-copy handler; no secondary guest executor"}));
        if self.recording_active() {
            self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"copy-refused",None,serde_json::json!({"outcome":"refused","reason":"Input recording active"}));
            self.menu.diagnosis_result("Stop input recording before starting a debug session.".into());
            return;
        }
        let result = (|| -> Result<(), String> {
            let state = self.machine.save_state()?;
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let destination = crate::debug_session::default_destination()?;
            let root = crate::debug_session::create(&destination, crate::debug_session::Source {
                rom: &self.machine.bus.rom,
                bios: self.machine.bus.real_bios.then_some(self.machine.bus.bios.as_slice()),
                save: self.machine.bus.save_data(), state: Some(&state),
                av: self.av.clone(), input: self.icfg.clone(), interp: self.native.is_none(),
                dataset: self.diagnosis.dataset.as_deref(), executable: &exe,
            })?;
            crate::debug_session::launch(&root)?;
            eprintln!("Isolated debug session: {}", root.display());
            Ok(())
        })();
        self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"copy-result",None,serde_json::json!({"outcome":if result.is_ok(){"completed"}else{"refused"},"error":result.as_ref().err().map(|e|e.chars().take(1024).collect::<String>())}));
        self.menu.diagnosis_result(match result {
            Ok(()) => "Debug copy opened; normal game stays paused.".into(),
            Err(e) => format!("Cannot start debug copy: {e}"),
        });
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn lifecycle_emit(&mut self,family:crate::debug_lifecycle::Family,phase:&str,context:Option<crate::debug_events::Correlation>,details:serde_json::Value) {
        if !self.events_enabled(){return;}
        let cause=self.events.clock.current().request_id;
        let _context=context.map(|context|self.events.clock.context(context));
        let mut data=crate::debug_lifecycle::event(family,phase,details);
        data["cause_request_id"]=serde_json::json!(cause);
        self.ui_event("runtime-event",data);
    }
    fn lifecycle_begin(&mut self,family:crate::debug_lifecycle::Family,details:serde_json::Value) {
        let context=self.lifecycle.begin(family,&self.events.clock,self.events_enabled());
        match context {
            Ok(context)=>self.lifecycle_emit(family,"started",Some(context),details),
            Err(error)=>self.lifecycle_emit(family,"metadata-unavailable",None,serde_json::json!({"outcome":"refused","error":error,"scope":"Existing operation was not redispatched or cancelled by metadata refusal"})),
        }
    }
    fn lifecycle_finish(&mut self,family:crate::debug_lifecycle::Family,details:serde_json::Value) {
        let context=self.lifecycle.finish(family);
        self.lifecycle_emit(family,"finished",context,details);
    }
    fn lifecycle_baseline(&mut self) {
        if !self.events_enabled(){return;}
        self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"baseline",None,serde_json::json!({"debug_copy":self.debug_session,"title":self.title.chars().take(128).collect::<String>(),"rom_sha256":gba_core::ffta_rom::digest(&self.machine.bus.rom),"backend":if self.native.is_some(){"native-with-fallback"}else{"interpreter"},"dataset":self.machine.bus.rom_resources().map(|r|serde_json::json!({"source_sha256":r.source_sha256(),"dataset_sha256":r.dataset_sha256()})),"paused":self.paused,"scope":"Current authenticated run/ROM boundary at explicit capture start, not a reconstructed startup history"}));
    }
    fn lifecycle_action(&mut self,action:&crate::debug_control::Action,result:&Result<(),String>,report:Option<String>) {
        use crate::debug_control::Action as A;use crate::debug_lifecycle::Family as F;
        let operation=action.operation_id();
        let capture=match action{A::UiStart|A::UiReport=>Some(F::Ui),A::RecentStart{..}|A::RecentFreeze|A::RecentReport=>Some(F::Recent),A::CodeStart{..}|A::CodeReport=>Some(F::Code),A::GameplayStart{..}|A::GameplayReport=>Some(F::Gameplay),A::AudioStart{..}|A::AudioReport=>Some(F::Audio),_=>None};
        if let Err(error)=result {
            if !matches!(action,A::Status){self.lifecycle_emit(capture.unwrap_or(if matches!(action,A::Graphics{..}|A::IssueReport{..}){F::Report}else if matches!(action,A::CheatPrepare{..}|A::CheatApply{..}){F::Cheat}else{F::Machine}),"refused",None,serde_json::json!({"operation_id":operation,"outcome":"refused","error":error.chars().take(1024).collect::<String>(),"artifact":report}));}
            return;
        }
        match action {
            A::CheatPrepare{id,..}=>self.lifecycle_emit(F::Cheat,"prepared",None,serde_json::json!({"operation_id":operation,"id":id,"outcome":"completed","scope":"Bounded paused preview; no guest writes"})),
            A::CheatApply{id,..}=>self.lifecycle_emit(F::Cheat,"applied-once",None,serde_json::json!({"operation_id":operation,"id":id,"outcome":"completed","result":self.last_cheat_once})),
            A::UiStart|A::RecentStart{..}|A::CodeStart{..}|A::GameplayStart{..}|A::AudioStart{..}=>{
                let family=capture.unwrap();
                if self.lifecycle.context(family).is_some(){self.lifecycle_finish(family,serde_json::json!({"outcome":"rearmed","scope":"Previous observer ownership replaced by an accepted explicit start"}));}
                self.lifecycle_begin(family,serde_json::json!({"operation_id":operation,"scope":"Existing bounded observer armed; no guest step"}));self.lifecycle_baseline();
            },
            A::UiReport|A::RecentReport|A::CodeReport|A::GameplayReport|A::AudioReport=>{
                let family=capture.unwrap();let output=match family{F::Ui=>self.ui_trace.report.clone(),F::Recent=>self.last_recent_report.clone(),F::Code=>self.last_code_report.clone(),F::Gameplay=>self.last_gameplay_report.clone(),F::Audio=>self.last_audio_report.clone(),_=>None};
                self.lifecycle_finish(family,serde_json::json!({"operation_id":operation,"outcome":"completed","artifact":output,"scope":"Existing explicit report published; own collector may already be frozen"}));
            },
            A::Graphics{..}|A::IssueReport{..}=>self.lifecycle_emit(F::Report,"published",None,serde_json::json!({"operation_id":operation,"outcome":"completed","artifact":report})),
            A::RecentFreeze=>{let context=self.lifecycle.context(F::Recent);self.lifecycle_emit(F::Recent,"frozen",context,serde_json::json!({"operation_id":operation}));},
            A::Step{..}|A::Find{..}=>{
                for family in [F::Step,F::Search]{if self.lifecycle.context(family).is_some(){self.lifecycle_finish(family,serde_json::json!({"outcome":"superseded","by_operation_id":operation}));}}
                let (family,details)=match action{A::Step{frames,keys}=>(F::Step,serde_json::json!({"operation_id":operation,"frames":frames,"keys":keys})),_=>(F::Search,serde_json::json!({"operation_id":operation,"remaining_steps":self.probe.as_ref().map(|p|p.remaining())}))};
                self.lifecycle_begin(family,details);
            },
            A::Pause|A::Resume|A::Cancel{..}=>{
                self.lifecycle_emit(F::Machine,"control",None,serde_json::json!({"operation_id":operation,"paused":self.paused,"reason":self.frame_control.reason,"remaining":self.frame_control.remaining}));
                if self.frame_control.remaining==0&&self.probe.is_none(){for family in [F::Step,F::Search]{if self.lifecycle.context(family).is_some(){self.lifecycle_finish(family,serde_json::json!({"outcome":self.frame_control.reason,"by_operation_id":operation}));}}}
            },
            A::Status=>{},
            _=>self.lifecycle_emit(F::Machine,"operation",None,serde_json::json!({"operation_id":operation,"outcome":"completed"})),
        }
    }
    fn lifecycle_poll(&mut self) {
        use crate::debug_lifecycle::Family as F;
        // End metadata jobs even with all consumers off. No guest state dump.
        for family in [F::Step,F::Search] {
            if self.lifecycle.context(family).is_some()&&self.frame_control.remaining==0&&self.probe.is_none(){
                self.lifecycle_finish(family,serde_json::json!({"outcome":self.frame_control.reason,"completed_frame":self.machine.bus.frames,"clock":self.machine.bus.clock}));
            }
        }
        if !self.events_enabled(){return;}
        for family in [F::Step,F::Search,F::Ui,F::Recent,F::Code,F::Gameplay,F::Audio,F::Recording] {
            if !self.lifecycle.due(family){continue;}
            let details=match family {
                F::Step|F::Search=>serde_json::json!({"frame":self.machine.bus.frames,"remaining_frames":self.frame_control.remaining,"remaining_steps":self.probe.as_ref().map(|p|p.remaining()),"reason":self.frame_control.reason}),
                F::Recording=>self.capture.as_ref().map_or_else(||serde_json::json!({"legacy":self.record.is_some()}),|c|c.lifecycle_status()),
                F::Ui=>self.ui_trace.lifecycle_status(),
                F::Recent=>self.recent.as_ref().map_or(serde_json::Value::Null,|b|b.lifecycle_status()),
                F::Code=>self.code_trace.as_ref().map_or(serde_json::Value::Null,|t|t.status()),
                F::Gameplay=>self.gameplay_trace.as_ref().map_or(serde_json::Value::Null,|t|t.status()),
                F::Audio=>{let streams=self.streams.lock().unwrap();streams.diagnostic.as_ref().map_or(serde_json::Value::Null,|d|d.status())},
                _=>unreachable!(),
            };
            if let Some((context,details))=self.lifecycle.progress(family,details){self.lifecycle_emit(family,"progress",Some(context),details);}
        }
    }
    fn host_consumed(&mut self,reason:&str) {
        if self.events_enabled(){self.ui_event("host-input-consumed",serde_json::json!({"reason":reason,"guest_input":false}));}
    }
    fn host_action_returned(&mut self,action:menu::MenuAction) {
        if self.events_enabled(){self.ui_event("host-action-result",serde_json::json!({"action":format!("{action:?}"),"outcome":"handler-returned","feedback":self.menu.diagnosis_message().chars().take(512).collect::<String>(),"scope":"Return from existing menu handler; semantic feedback changes carry UI outcomes, asynchronous completion is separate"}));}
    }
    fn host_refused(&mut self,error:&str) {
        if self.events_enabled(){self.ui_event("host-action-refused",serde_json::json!({"outcome":"refused","error":error.chars().take(512).collect::<String>()}));}
    }
    fn inline_action(&mut self,action:Option<menu::MenuAction>) {
        if let Some(action)=action {if self.events_enabled(){self.ui_event("host-action",serde_json::json!({"action":format!("{action:?}"),"operation_id":crate::debug_operations::menu_operation(&action),"scope":"Existing inline cancel handler; no second dispatch"}));}}
    }
    fn observe_menu(&mut self,cause:&str) {
        if !self.events_enabled(){self.selections.reset();return;}
        let properties=self.menu.selection_state(&self.av,&self.icfg);
        for (stage,data) in self.selections.changes(properties,self.paused,cause){self.ui_event(stage,data);}
    }
    fn queue_mouse(&mut self,event:MouseEv) {
        let context=self.events_enabled().then(||self.events.clock.current());
        self.mouse_queue.push((event,context));
    }
    fn toggle_event_console(&mut self, event_loop:&ActiveEventLoop) {
        if let Some(panel)=self.event_console_panel.as_mut() {
            if panel.is_visible() { panel.hide(&mut self.event_console.borrow_mut()); }
            else { panel.show(&mut self.event_console.borrow_mut()); }
        } else {
            match crate::debug_console_panel::Panel::open(event_loop,&mut self.event_console.borrow_mut()) {
                Ok(panel)=>self.event_console_panel=Some(panel),
                Err(e)=>{self.menu.diagnosis_result(format!("Cannot open event console: {e}"));self.menu_dirty=true;}
            }
        }
    }
    fn open_command_console(&mut self,event_loop:&ActiveEventLoop) {
        let Some((root,session))=self.control_server.as_ref().map(|s|(s.root().to_owned(),s.event_session())) else {
            self.menu.diagnosis_result("Open an isolated debug copy before using the command console.".into());self.menu_dirty=true;return;
        };
        if let Some(panel)=self.command_console_panel.as_mut(){panel.show();return;}
        let Some(manifest_sha256)=session.manifest_sha256.as_deref() else {self.menu.diagnosis_result("Missing session identity".into());self.menu_dirty=true;return;};
        match crate::debug_command_panel::Panel::open(event_loop,root,&session.run_id,manifest_sha256) {
            Ok(panel)=>self.command_console_panel=Some(panel),
            Err(error)=>{self.menu.diagnosis_result(format!("Command console: {error}"));self.menu_dirty=true;},
        }
    }
    fn console_key(&self, code:KeyCode)->bool {
        matches!((self.icfg.event_console_key.as_str(),code),
            ("F2",KeyCode::F2)|("F3",KeyCode::F3)|("F4",KeyCode::F4)|("F5",KeyCode::F5)|
            ("F9",KeyCode::F9)|("F10",KeyCode::F10)|("F12",KeyCode::F12))
    }
    fn events_enabled(&self)->bool {
        self.ui_trace.enabled()||self.recent.as_ref().is_some_and(|b|b.enabled())||self.event_console.borrow().enabled()
    }
    fn ui_event(&mut self,stage:&str,data:serde_json::Value) {
        if !self.events_enabled(){return;}
        let coalesce={let console=self.event_console.borrow(); !console.enabled() || console.coalesce_polls};
        if let Some(event)=self.events.publish_with_poll_coalescing(self.machine.bus.frames,self.machine.bus.clock,stage,data,coalesce){
            self.ui_trace.accept(&event);
            if let Some(buffer)=self.recent.as_mut(){buffer.accept(&event);}
            self.event_console.borrow_mut().accept(&event);
        }
    }
    fn ui_focus(&self)->serde_json::Value {
        let(w,h)=self.menu_dimensions();crate::debug_ui::focus(&self.menu,w,h)
    }
    fn save_ui_report(&mut self)->Result<(),String> {
        let root=input_config::exe_dir().ok_or("Missing isolated root")?;
        crate::debug_ui::capacity(&root)?;
        let(w,h)=self.menu_dimensions();
        if w>1600||h>1000{return Err("UI report supports windows up to1600x1000".into());}
        let before=self.machine.save_state()?;
        let mut pixels=self.preview_frame(w,h,true);
        let capture=menu::diagnostics::capture(||self.menu.draw_inputs(&self.av,&self.icfg,&mut pixels,w,h)).map_err(str::to_owned)?;
        let state=&self.machine.bus.ffta_dialogue;
        let layouts=(0..2).map(|i|self.ui_layout((w as u32,h as u32),i).map(|l|serde_json::json!({"rect":l.rect,"clip":l.clip}))).collect::<Vec<_>>();
        let scene=self.game_mouse.scene(&self.machine.bus).map(|s|serde_json::json!({"selected":s.selected,"targets":s.targets.iter().map(|t|serde_json::json!({"index":t.index,"bounds":t.bounds})).collect::<Vec<_>>()}));
        let frozen=serde_json::json!({"rom_sha256":gba_core::ffta_rom::digest(&self.machine.bus.rom),"runtime_sha256":gba_core::ffta_rom::digest(&crate::diagnostics::read(&std::env::current_exe().map_err(|e|e.to_string())?,256*1024*1024)?),
            "frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"state_sha256":gba_core::ffta_rom::digest(&before),
            "host_menu":self.ui_focus(),"text":crate::debug_ui::text(capture),"canvas":[w,h],"input_config":self.icfg.serialize(),
            "guest_ui":{"recognized_selector":scene,"name_input":self.game_mouse.is_name_input(&self.machine.bus),"battle_mouse_enabled":self.machine.bus.ffta_battle_mouse.enabled,
                "dialogue_flags":{"separate":state.separate,"action":state.action_layout,"town":state.town_layout,"shop":state.shop_layout,"site":state.site_layout,"laws":state.laws_layout},"panel_kind":format!("{:?}",state.extra.kind),"layouts":layouts,"pixel_bounds":[gba_core::ffta_dialogue::pixel_bounds(&state.pixels),gba_core::ffta_dialogue::pixel_bounds(&state.extra.pixels)],"unknown_menu":"Other guest menu/focus/text metrics are unknown"}});
        if self.machine.save_state()?!=before{return Err("UI observation changed guest state".into());}
        let path=self.ui_trace.save(&root,frozen,&pixels,w,h)?;
        eprintln!("UI report: {}",root.join(&path).display());
        self.menu.diagnosis_result("UI report saved. Open Data/config/ui/<run>/index.html".into());self.menu_dirty=true;Ok(())
    }
    fn recent_scene(&self)->serde_json::Value{
        let state=&self.machine.bus.ffta_dialogue;
        let recognized=self.game_mouse.scene(&self.machine.bus).map(|scene|serde_json::json!({"selected":scene.selected,"target_ids":scene.targets.iter().take(128).map(|t|t.index).collect::<Vec<_>>(),"targets_total":scene.targets.len()}));
        serde_json::json!({"recognized_selector":recognized,"name_input":self.game_mouse.is_name_input(&self.machine.bus),
            "dialogue_flags":{"separate":state.separate,"action":state.action_layout,"town":state.town_layout,"shop":state.shop_layout,"site":state.site_layout,"laws":state.laws_layout},
            "panel_kind":format!("{:?}",state.extra.kind),"scope":"Partial readonly frontend UI signature; disabled/unrecognized is unknown, no global scene ID"})
    }
    fn recent_error(&mut self,error:&str){
        if self.events_enabled(){self.ui_event("error",serde_json::json!({"message":error.chars().take(1024).collect::<String>(),"message_truncated":error.chars().count()>1024,"pc":self.machine.cpu.regs[15],"keys":self.machine.bus.keys}));}
        let triggered=self.recent.as_mut().is_some_and(|b|b.trigger_error(&self.machine));
        if triggered{
            let result=(||{
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;
                self.recent.as_mut().unwrap().save(&root,&self.machine)
            })();
            match result{Ok(path)=>{eprintln!("Recent error context: {path}");self.last_recent_report=Some(path);},
                Err(error)=>eprintln!("Recent context frozen but not saved: {error}")}
        }
    }
    fn debug_action(&mut self, action: &crate::debug_control::Action) -> Result<(),String> {
        let mut context=self.events.clock.current();
        if context.request_id.is_none(){context=self.events.clock.request("shared-action");}
        if matches!(action,crate::debug_control::Action::Step{..}|crate::debug_control::Action::Find{..}|crate::debug_control::Action::SelectId{..}|crate::debug_control::Action::ContentRefresh|crate::debug_control::Action::CheatRefresh|crate::debug_control::Action::Replay{..}){context.job_id=context.request_id.clone();}
        let _context=self.events.clock.context(context);
        self.observe_menu("control-before");
        let observed=!matches!(action,crate::debug_control::Action::Status);
        if observed&&self.events_enabled(){self.ui_event("control-request",serde_json::json!({"command":action,"paused":self.paused}));}
        let before_report=match action{crate::debug_control::Action::Graphics{..}=>self.graphics_report.clone(),crate::debug_control::Action::IssueReport{..}=>self.last_issue_report.clone(),_=>None};
        let result=self.debug_action_inner(action);
        let report=match action{crate::debug_control::Action::Graphics{..}=>self.graphics_report.clone(),crate::debug_control::Action::IssueReport{..}=>self.last_issue_report.clone(),_=>None}.filter(|path|Some(path)!=before_report.as_ref());
        self.lifecycle_action(action,&result,report);
        self.observe_menu("control-handler");
        if observed&&self.events_enabled(){self.ui_event("control-result",serde_json::json!({"command":action,"outcome":if result.is_ok(){"accepted"}else{"refused"},"error":result.as_ref().err(),"scope":"Shared handler return; asynchronous work may still be pending"}));}
        if let Err(error)=&result{self.recent_error(error);}
        result
    }
    fn debug_action_inner(&mut self, action: &crate::debug_control::Action) -> Result<(),String> {
        if !self.debug_session { return Err("Open an isolated debug copy first".into()); }
        match action {
            crate::debug_control::Action::CheatRefresh=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some()||self.cheat_refresh.is_some(){return Err("Cheat refresh requires an idle paused copy".into());}
                let reference=self.content_picker.reference(menu::ContentSlot::CheatCatalog).cloned().ok_or("Select a cheat catalog by ID first")?;
                let rom=self.content_picker.rom_context().ok_or("No ROM context")?.to_owned();
                self.cheat_refresh=Some(crate::debug_cheats::Refresh::start(reference,rom)?);return Ok(());
            },
            crate::debug_control::Action::CheatList{offset,limit,generation}=>{action.validate()?;self.current_cheat_snapshot()?.page(*offset,*limit,generation.as_deref())?;return Ok(());},
            crate::debug_control::Action::CheatShow{id,offset,generation}=>{action.validate()?;self.current_cheat_snapshot()?.show(id,*offset,generation.as_deref())?;return Ok(());},
            crate::debug_control::Action::CheatPrepare{id,provenance,master,generation}=>{
                action.validate()?;self.cheat_preview=None;
                let (_,preview)=self.prepare_cheat_once(id,*provenance,master,generation)?;
                self.cheat_preview=Some(preview);self.menu.diagnosis_result("Cheat preview prepared; inspect its plan and token before applying once.".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::CheatApply{id,provenance,master,generation,preview}=>{
                action.validate()?;
                if self.cheat_preview.as_ref().and_then(|v|v["token"].as_str())!=Some(preview.as_str()){return Err("Prepare this exact one-shot preview first".into());}
                let (plan,current)=self.prepare_cheat_once(id,*provenance,master,generation)?;
                if current["token"].as_str()!=Some(preview.as_str()){return Err("Cheat preview, source, ROM or paused state changed; prepare again".into());}
                let before=(self.machine.bus.frames,self.machine.bus.clock);let count=plan.apply_once(&mut self.machine.bus)?;
                self.cheat_preview=None;self.last_cheat_once=Some(serde_json::json!({"id":id,"preview":preview,"writes":count,"frame":before.0,"clock":before.1,"scope":"Manual paused one-shot, no hardware hook or maintained toggle"}));
                self.menu.diagnosis_result(format!("Applied {count} writes once in the isolated paused copy."));self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::Replay{mode}=>{
                if !self.paused{return Err("Replay requires an idle paused copy".into());}
                return self.start_replay_request(false,*mode,false).and_then(|started|if started{Ok(())}else{Err("Choose a replay profile first".into())});
            },
            crate::debug_control::Action::ContentRefresh=>{
                if !self.paused || self.probe.is_some() || self.frame_control.remaining>0 || self.replay_job.is_some(){return Err("Content refresh requires an idle paused copy".into());}
                self.content_picker.refresh_control()?;
                if self.menu.content_active(){self.menu.content_busy(true);self.menu_dirty=true;}
                return Ok(());
            },
            crate::debug_control::Action::ContentList{offset,limit,generation}=>{
                action.validate()?;self.content_picker.page(*offset,*limit,*generation)?;return Ok(());
            },
            crate::debug_control::Action::SelectId{slot,id}=>{
                action.validate()?;
                if !self.paused || self.probe.is_some() || self.frame_control.remaining>0 || self.replay_job.is_some(){return Err("Content selection requires an idle paused copy".into());}
                self.content_picker.select_id(slot.content_slot(),id)?;
                if self.menu.content_active(){self.menu.content_busy(true);self.menu_dirty=true;}
                return Ok(());
            },
            crate::debug_control::Action::IssueReport{profile,symptom}=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Issue report requires an idle paused copy".into());}
                let selected=if profile.is_none(){Some(self.selected_replay_profile()?)}else{None};
                let path=profile.as_ref().or(selected.as_ref()).ok_or("Choose a replay profile first")?;
                let report=crate::debug_issue::create(path,symptom)?;
                self.replay_path=Some(path.clone());self.last_issue_report=Some(report.path.to_string_lossy().into_owned());
                self.menu.diagnosis_result(if report.compatible{"Issue saved. Open its index.html; replay was not started."}else{"Error issue saved. Inspect preflight in its index.html."}.into());self.menu_dirty=true;
                return if report.compatible{Ok(())}else{Err("Issue report saved with incompatible inputs".into())};
            },
            crate::debug_control::Action::RecentStart{settings}=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Recent capture requires an idle paused copy".into());}
                if self.recent.as_ref().is_some_and(|b|!b.saved()){return Err("Save the previous recent window first".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;
                let scene=self.recent_scene();
                let origin=serde_json::json!({"runtime_sha256":gba_core::ffta_rom::digest(&crate::diagnostics::read(&std::env::current_exe().map_err(|e|e.to_string())?,256*1024*1024)?),
                    "backend":if self.native.is_some(){"native-with-fallback"}else{"interpreter"},"native_table_entries":self.native.as_ref().map(|(_,t)|t.len),
                    "native_scope":"Loaded code-memory identity and block interiors are not authenticated here",
                    "av":self.av.serialize(),"input_config":self.icfg.serialize(),
                    "active_dataset":self.machine.bus.rom_resources().map(|r|serde_json::json!({"source_sha256":r.source_sha256(),"dataset_sha256":r.dataset_sha256()})),
                    "build":{"revision":env!("FFTA_BUILD_REVISION"),"dirty_at_build":env!("FFTA_BUILD_DIRTY"),"target":env!("FFTA_BUILD_TARGET"),"profile":env!("FFTA_BUILD_PROFILE"),"rom_data_read_audit":cfg!(feature="rom-data-read-audit")}});
                self.recent=Some(crate::debug_recent::Buffer::start_shared_observing(&root,&self.machine,settings.clone(),origin,scene,self.events.clock.clone(),Some(std::rc::Rc::downgrade(&self.event_console)))?);self.events.reset_polls();self.selections.reset();
                self.menu.diagnosis_result("Recent events rolling. Errors freeze/save; use Freeze / save for review.".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::RecentFreeze=>{
                self.recent.as_mut().ok_or("Start recent events first")?.freeze(&self.machine,"freeze-request");
                self.menu.diagnosis_result("Recent window frozen; pause and Save recent events.".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::RecentReport=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Recent report requires an idle paused copy".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;let before=self.machine.save_state()?;
                let path=self.recent.as_mut().ok_or("Start recent events first")?.save(&root,&self.machine)?;
                if self.machine.save_state()?!=before{return Err("Recent report changed guest state".into());}
                self.last_recent_report=Some(path);self.menu.diagnosis_result("Recent events saved. Open Data/config/recent/<run>/index.html".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::GameplayStart{settings}=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Gameplay trace requires an idle paused copy".into());}
                if self.gameplay_trace.is_some(){return Err("Save the previous gameplay trace first".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;
                let origin=serde_json::json!({"runtime_sha256":gba_core::ffta_rom::digest(&crate::diagnostics::read(&std::env::current_exe().map_err(|e|e.to_string())?,256*1024*1024)?),
                    "backend":if self.native.is_some(){"native-with-fallback"}else{"interpreter"},"av":self.av.serialize(),"input_config":self.icfg.serialize(),"keys":self.machine.bus.keys,
                    "active_dataset":self.machine.bus.rom_resources().map(|r|serde_json::json!({"source_sha256":r.source_sha256(),"dataset_sha256":r.dataset_sha256()})),
                    "build":{"revision":env!("FFTA_BUILD_REVISION"),"dirty_at_build":env!("FFTA_BUILD_DIRTY"),"target":env!("FFTA_BUILD_TARGET"),"profile":env!("FFTA_BUILD_PROFILE"),"rom_data_read_audit":cfg!(feature="rom-data-read-audit")}});
                self.gameplay_trace=Some(crate::debug_gameplay::Trace::start(&root,&self.machine,settings.clone(),origin)?);
                self.menu.diagnosis_result("Gameplay trace armed. Reproduce, pause, Save gameplay report.".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::GameplayReport=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Pause and finish current budget before gameplay report".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;let before=self.machine.save_state()?;
                let current=serde_json::json!({"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"keys":self.machine.bus.keys,"state_sha256":gba_core::ffta_rom::digest(&before)});
                let path=self.gameplay_trace.as_mut().ok_or("Start a gameplay trace first")?.save(&root,current)?;
                if self.machine.save_state()?!=before{return Err("Gameplay report changed guest state".into());}
                self.gameplay_trace=None;self.last_gameplay_report=Some(path);self.menu.diagnosis_result("Gameplay saved. Open Data/config/gameplay/<run>/index.html".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::CodeStart{settings}=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Code trace requires an idle paused copy".into());}
                if self.code_trace.is_some(){return Err("Save the previous code trace first".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;
                let origin=serde_json::json!({"runtime_sha256":gba_core::ffta_rom::digest(&crate::diagnostics::read(&std::env::current_exe().map_err(|e|e.to_string())?,256*1024*1024)?),
                    "av":self.av.serialize(),"input_config":self.icfg.serialize(),"keys":self.machine.bus.keys,
                    "active_dataset":self.machine.bus.rom_resources().map(|r|serde_json::json!({"source_sha256":r.source_sha256(),"dataset_sha256":r.dataset_sha256()})),
                    "build":{"revision":env!("FFTA_BUILD_REVISION"),"dirty_at_build":env!("FFTA_BUILD_DIRTY"),"target":env!("FFTA_BUILD_TARGET"),"profile":env!("FFTA_BUILD_PROFILE"),"rom_data_read_audit":cfg!(feature="rom-data-read-audit")}});
                let labels=if self.content_picker.has_slot(menu::ContentSlot::Labels){Some(self.content_picker.slot_path(menu::ContentSlot::Labels,&gba_core::ffta_rom::digest(&self.machine.bus.rom))?)}else{None};
                self.code_trace=Some(crate::debug_code::Trace::start(&root,&self.machine,self.native.as_ref().map(|(_,t)|t),settings.clone(),origin,labels.as_deref())?);
                self.menu.diagnosis_result("Code trace armed. Reproduce, pause, Save code report.".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::CodeReport=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Pause and finish current budget before code report".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;let before=self.machine.save_state()?;
                let current=serde_json::json!({"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"keys":self.machine.bus.keys,"state_sha256":gba_core::ffta_rom::digest(&before)});
                let path=self.code_trace.as_mut().ok_or("Start a code trace first")?.save(&root,current)?;
                if self.machine.save_state()?!=before{return Err("Code report changed guest state".into());}
                self.code_trace=None;self.last_code_report=Some(path);self.menu.diagnosis_result("Code saved. Open Data/config/code/<run>/index.html".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::AudioStart{settings}=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Audio capture requires an idle paused copy".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;crate::debug_audio::capacity(&root)?;
                let manifest=crate::diagnostics::read(&root.join("debug-session.json"),1024*1024)?;
                let session_manifest:serde_json::Value=serde_json::from_slice(&manifest).map_err(|e|e.to_string())?;
                let origin=serde_json::json!({"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"state_sha256":gba_core::ffta_rom::digest(&self.machine.save_state()?),
                    "keys":self.machine.bus.keys,"input_config":self.icfg.serialize(),"backend":if self.native.is_some(){"native-with-fallback"}else{"interpreter"},
                    "bios":{"real":self.machine.bus.real_bios,"sha256":gba_core::ffta_rom::digest(&self.machine.bus.bios)},
                    "active_dataset":self.machine.bus.rom_resources().map(|r|serde_json::json!({"source_sha256":r.source_sha256(),"dataset_sha256":r.dataset_sha256()})),
                    "session_manifest_sha256":gba_core::ffta_rom::digest(&manifest),"session_manifest":session_manifest,
                    "recording_directory":self.capture.as_ref().map(|c|c.directory.strip_prefix(&root).unwrap_or(&c.directory).to_string_lossy()),
                    "build":{"version":env!("CARGO_PKG_VERSION"),"revision":env!("FFTA_BUILD_REVISION"),"dirty_at_build":env!("FFTA_BUILD_DIRTY"),"target":env!("FFTA_BUILD_TARGET"),"profile":env!("FFTA_BUILD_PROFILE"),"rom_data_read_audit":cfg!(feature="rom-data-read-audit")},
                    "av":self.av.serialize(),"device_stream_present":self.stream.is_some(),"pending_audio":[self.machine.bus.audio_buf.len(),self.machine.bus.psg_tap.len(),self.machine.bus.hle_tap.len(),self.machine.bus.fifo_tap[0].len(),self.machine.bus.fifo_tap[1].len()]});
                let mut st=self.streams.lock().unwrap();if st.diagnostic.is_some(){return Err("Save the previous audio capture first".into());}
                st.diagnostic=Some(crate::debug_audio::Capture::start(settings.clone(),origin,self.machine.bus.frames)?);self.audio_diagnostic=true;
                self.menu.diagnosis_result("Audio capture armed. Reproduce, then Save report.".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::AudioReport=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("Pause and finish current budget before audio report".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;crate::debug_audio::capacity(&root)?;
                let before=self.machine.save_state()?;
                let runtime_sha=gba_core::ffta_rom::digest(&crate::diagnostics::read(&std::env::current_exe().map_err(|e|e.to_string())?,256*1024*1024)?);
                let mut capture=self.streams.lock().unwrap().diagnostic.take().ok_or("Start an audio capture first")?;capture.freeze("report-request");self.audio_diagnostic=false;
                let metadata=serde_json::json!({"rom_sha256":gba_core::ffta_rom::digest(&self.machine.bus.rom),"runtime_sha256":runtime_sha,
                    "current":{"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"state_sha256":gba_core::ffta_rom::digest(&before),"av":self.av.serialize(),"device_stream_present":self.stream.is_some()}});
                // Publish with the callback mutex released. Failed output retains the frozen capture.
                let result=capture.save(&root,metadata);
                if self.machine.save_state()?!=before{self.streams.lock().unwrap().diagnostic=Some(capture);return Err("Audio reporting changed guest state".into());}
                match result{Ok(path)=>{eprintln!("Audio report: {}",root.join(&path).display());self.last_audio_report=Some(path);self.menu.diagnosis_result("Audio saved. Open Data/config/audio/<run>/index.html".into());self.menu_dirty=true;return Ok(());},Err(error)=>{self.streams.lock().unwrap().diagnostic=Some(capture);return Err(error);}}
            },
            crate::debug_control::Action::UiStart|crate::debug_control::Action::UiReport=>{
                if !self.paused||self.probe.is_some()||self.frame_control.remaining>0||self.replay_job.is_some(){return Err("UI trace/report requires an idle paused copy".into());}
                if matches!(action,crate::debug_control::Action::UiStart){
                    let root=input_config::exe_dir().ok_or("Missing isolated root")?;crate::debug_ui::capacity(&root)?;
                    let state=gba_core::ffta_rom::digest(&self.machine.save_state()?);
                    self.ui_trace.start_shared(serde_json::json!({"state_sha256":state,"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"menu":self.ui_focus(),"input_config":self.icfg.serialize()}),self.events.clock.clone())?;self.events.reset_polls();self.selections.reset();
                    self.menu.diagnosis_result("UI/input trace active:30s /1024events /600frames. Save UI report to retain.".into());self.menu_dirty=true;return Ok(());
                }
                return self.save_ui_report();
            },
            crate::debug_control::Action::Graphics{compare_last}=>{
                if !self.paused || self.probe.is_some() || self.frame_control.remaining>0 || self.replay_job.is_some(){return Err("Pause and finish current search/step/replay before graphics capture".into());}
                let root=input_config::exe_dir().ok_or("Missing isolated root")?;crate::debug_graphics::capacity(&root)?;
                let size=self.window.as_ref().ok_or("Graphics capture needs an active window")?.inner_size();
                let(w,h)=(size.width as usize,size.height as usize);if w==0||h==0||w>1600||h>1000{return Err("Graphics capture supports window sizes up to1600x1000".into());}
                let world=self.preview_frame(w,h,false);let composed=self.preview_frame(w,h,true);
                let layouts=(0..2).map(|i|self.ui_layout((w as u32,h as u32),i).map(|l|serde_json::json!({"rect":l.rect,"clip":l.clip}))).collect::<Vec<_>>();
                let front=crate::debug_graphics::Frontend{width:w,height:h,world,composed,settings:serde_json::json!({"view":self.view,"base_extra":self.base_extra,"zoom_level":self.zoom_level,"ui_layouts":layouts,"av":self.av.serialize()})};
                let report=crate::debug_graphics::capture(&root,&self.machine,front,*compare_last)?;
                eprintln!("Graphics report: {}",root.join(&report).display());self.graphics_report=Some(report);
                self.menu.diagnosis_result("Graphics saved. Open Data/config/graphics/<run>/index.html".into());self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::Inspect{address,length}=>{
                if !self.paused || self.probe.is_some() || self.frame_control.remaining>0 {return Err("Pause and finish the current budget before inspection".into());}
                let root=input_config::exe_dir();
                let value=crate::debug_probe::inspect(&self.machine,*address,*length,root.as_deref(),self.diagnosis.dataset.as_deref())?;
                if let Some(server)=&self.control_server {server.inspection(&value)?;}
                self.menu.diagnosis_result(format!("PC {:08X} | CPSR {:08X} | inspection.json",self.machine.cpu.regs[15],self.machine.cpu.cpsr));
                self.inspection=Some(value);self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::Find{settings}=>{
                if !self.paused || self.probe.is_some() || self.frame_control.remaining>0 || self.replay_job.is_some() || self.recording_active() {return Err("Search requires an idle paused copy without recording or replay".into());}
                let search=crate::debug_probe::Search::start(settings.clone(),self.native.is_some())?;
                self.frame_control.stop("searching");self.probe_result=None;self.probe=Some(search);self.debug_audio_boundary();self.menu_dirty=true;return Ok(());
            },
            crate::debug_control::Action::Cancel{request_id}=>{
                if !self.control_server.as_ref().is_some_and(|s|s.pending_matches(request_id)){
                    return Err("No pending command with that exact request ID".into());
                }
                if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::SelectId{..})) {
                    return if self.content_picker.cancel_id(){Ok(())}else{Err("Content selection is no longer pending".into())};
                }
                if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::ContentRefresh)) {
                    return if self.content_picker.cancel_refresh_control(){Ok(())}else{Err("Content scan is no longer pending".into())};
                }
                if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::CheatRefresh)) {
                    if let Some(job)=self.cheat_refresh.as_mut(){job.cancel();return Ok(());}return Err("Cheat refresh is no longer pending".into());
                }
                if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::Replay{..})) {
                    return self.cancel_replay_request();
                }
                if self.replay_job.is_some(){return Err("Use replay cancellation for a separate worker".into());}
                self.interrupt_probe();
            },
            crate::debug_control::Action::Step{..} if self.probe.is_some()=>return Err("Interrupt or finish the search before stepping frames".into()),
            crate::debug_control::Action::Pause | crate::debug_control::Action::Resume=>self.interrupt_probe(),
            _=>{},
        }
        let before=self.paused;
        let paused=self.frame_control.apply(action,before,self.replay_job.is_some())?;
        if matches!(action,crate::debug_control::Action::Status) {return Ok(());}
        self.paused=paused;
        if paused && !before {self.menu.reset();self.refresh_state_slots();}
        self.pending_key=None;
        self.mouse_queue.clear();
        self.game_mouse.cancel();self.machine.bus.ffta_battle_mouse.cancel();
        self.suppress |= !self.game_keys & 0x3ff;
        self.debug_audio_boundary();
        self.menu_dirty=true;
        Ok(())
    }

    fn interrupt_probe(&mut self) {
        if let Some(probe)=self.probe.take() {
            self.probe_result=Some(serde_json::json!({"outcome":"interrupted","remaining_steps":probe.remaining()}));
        }
    }

    fn probe_tick(&mut self) {
        let _context=self.lifecycle.context(crate::debug_lifecycle::Family::Search).map(|c|self.events.clock.context(c));
        let completed=self.probe.as_mut().unwrap().tick(&mut self.machine);
        if completed {
            self.refresh_ui_layers();
            let frame=display_frame(&self.machine.bus,&mut self.padded_frame);
            self.temporal.push(frame,&self.lut);
            self.handoff_audio();
        }
        if self.probe.as_ref().unwrap().result.is_some() {
            let probe=self.probe.take().unwrap();
            let mut result=probe.result.clone().unwrap();result["settings"]=serde_json::to_value(&probe.settings).expect("search settings");self.probe_result=Some(result);
            self.frame_control.stop(if self.probe_result.as_ref().unwrap()["outcome"]=="hit" {"search-hit"}else{"not-reached"});
            // A partial-step stop keeps its guest audio buffers: they are guest
            // state, drained at the next completed frame as ordinary playback.
            self.menu.diagnosis_result(format!("{} | PC {:08X} | inspect via debug-control",self.frame_control.reason.to_uppercase(),self.machine.cpu.regs[15]));
            self.finish_control();
        }
        self.menu_dirty=true;
    }

    fn debug_audio_boundary(&mut self) {
        self.set_fast_forward(false,1);
        self.streams.lock().unwrap().muted=self.paused;
        self.fps.reset(Instant::now());
        self.profile.presentation_boundary();
        self.reset_audio_counters();
    }

    fn debug_status(&self) -> Result<serde_json::Value,String> {
        let state=gba_core::ffta_rom::digest(&self.machine.save_state()?);
        let st=self.streams.lock().unwrap();
        let mut value=serde_json::json!({"paused":self.paused,"remaining":self.frame_control.remaining,
            "reason":self.frame_control.reason,"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,
            "pc":self.machine.cpu.regs[15],"keys":self.machine.bus.keys,"state_sha256":state,
            "backend":if self.native.is_some(){"native"}else{"interp"},
            "registers":self.machine.cpu.regs,"cpsr":self.machine.cpu.cpsr,"thumb":self.machine.cpu.thumb(),
            "probe":self.probe_result,"search_remaining_steps":self.probe.as_ref().map(|p|p.remaining()),"inspection":self.inspection,"graphics_report":self.graphics_report,"ui_trace":self.ui_trace.status(),"event_clock":self.events.clock.status(),"event_coverage":crate::debug_events::coverage(),"host_menu":self.ui_focus(),"audio_diagnostic":st.diagnostic.as_ref().map(|d|d.status()),"audio_report":self.last_audio_report,
            "pending_control":self.control_server.as_ref().map(|s|s.pending_status()),"content_picker":self.content_picker.status(),"workbench":self.workbench.status(),"host_modal_input":self.modal_input.blocked(),
            "recent":self.recent.as_ref().map(|b|b.status()),"recent_report":self.last_recent_report,
            "gameplay_trace":self.gameplay_trace.as_ref().map(|t|t.status()),"gameplay_report":self.last_gameplay_report,
            "code_trace":self.code_trace.as_ref().map(|t|t.status()),"code_report":self.last_code_report,
            "issue_report":self.last_issue_report,
            "host_audio":{"mixed":st.mixed.len(),"psg":st.psg.len(),"hle":st.hle.len(),
                "fifo":[st.fifo[0].len(),st.fifo[1].len()],"normal":st.normal_audio.is_some(),"muted":st.muted,"epoch":st.epoch},
            "guest_audio":[self.machine.bus.audio_buf.len(),self.machine.bus.psg_tap.len(),self.machine.bus.hle_tap.len(),self.machine.bus.fifo_tap[0].len(),self.machine.bus.fifo_tap[1].len()]});
        value["replay"]=serde_json::json!({"active":self.replay_job.is_some(),"progress":self.replay_job.as_ref().and_then(|j|j.progress()),"result":self.replay_terminal.as_ref().map(|r|match r{Ok(report)=>serde_json::json!({"report":report}),Err(error)=>serde_json::json!({"error":error})})});
        value["cheat_snapshot"]=self.current_cheat_snapshot().ok().map(|s|serde_json::json!({"generation":s.generation,"source_sha256":s.source_sha256,"rom_sha256":s.rom_sha256,"scope":"Immutable snapshot verified at explicit refresh"})).unwrap_or(serde_json::Value::Null);
        value["cheat_preview"]=self.cheat_preview.clone().unwrap_or(serde_json::Value::Null);value["cheat_once"]=self.last_cheat_once.clone().unwrap_or(serde_json::Value::Null);
        value["runtime_lifecycles"]=self.lifecycle.status();
        value["request_ledger"]=self.control_server.as_ref().map_or(serde_json::Value::Null,|s|s.retry_status());
        value["event_console"]=self.event_console.borrow().status();
        Ok(value)
    }

    fn poll_control(&mut self) {
        // Deadline/cancel/status run on the frontend thread, between existing
        // bounded guest frame/search ticks. No thread interrupts guest execution.
        if self.control_server.as_ref().is_some_and(|s|s.deadline_expired()){
            if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::SelectId{..})) {
                self.content_picker.cancel_id();
            } else if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::ContentRefresh)) {
                self.content_picker.cancel_refresh_control();
            } else if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::CheatRefresh)) {
                if let Some(job)=self.cheat_refresh.as_mut(){job.cancel();}
            } else if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::Replay{..})) {
                if let Some(job)=self.replay_job.as_mut(){let _=job.cancel();}
            } else {
                self.interrupt_probe();self.frame_control.stop("control-timeout");
                self.paused=true;self.debug_audio_boundary();self.menu_dirty=true;
            }
            self.finish_control();
        }
        // Separate query response cannot overwrite a waiting command's result.
        for query in [true,false] {
            let request=match self.control_server.as_mut().map(|s|if query{s.poll_query()}else{s.poll()}){
                Some(Ok(request))=>request,
                Some(Err(error))=>{let context=self.control_server.as_ref().and_then(|s|s.publication_context(query));let _context=context.map(|c|self.events.clock.context(c));self.recent_error(&format!("Control channel: {error}"));eprintln!("debug control: {error}");None},
                None=>None,
            };
            if let Some(request)=request{
                let _context=self.events.clock.context(request.event_context());
                let outer_refusal=request.expired()||(!query&&self.control_server.as_ref().is_some_and(|s|s.pending.is_some()));
                if outer_refusal&&!matches!(request.command,crate::debug_control::Action::Status)&&self.events_enabled(){self.ui_event("control-request",serde_json::json!({"command":request.command,"paused":self.paused}));}
                let result=if request.expired(){
                    Err("Command deadline expired before execution".into())
                }else if !query && self.control_server.as_ref().is_some_and(|s|s.pending.is_some()){
                    Err("A command is still pending; use query status or targeted cancel".into())
                }else{self.debug_action(&request.command)};
                if outer_refusal&&!matches!(request.command,crate::debug_control::Action::Status)&&self.events_enabled(){self.ui_event("control-result",serde_json::json!({"command":request.command,"outcome":"refused","error":result.as_ref().err()}));}
                if result.is_ok() && matches!(request.command,crate::debug_control::Action::Step{..}|crate::debug_control::Action::Find{..}|crate::debug_control::Action::SelectId{..}|crate::debug_control::Action::ContentRefresh|crate::debug_control::Action::CheatRefresh|crate::debug_control::Action::Replay{..}){
                    self.control_server.as_mut().unwrap().pending=Some(request);
                }else{
                    self.finish_control();
                    let mut status=self.debug_status();let error=result.err().or_else(||status.as_ref().err().cloned());
                    if error.is_none(){if let Ok(value)=&mut status{match &request.command{
                        crate::debug_control::Action::ContentList{offset,limit,generation}=>value["content_page"]=self.content_picker.page(*offset,*limit,*generation).unwrap(),
                        crate::debug_control::Action::CheatList{offset,limit,generation}=>value["cheat_page"]=self.current_cheat_snapshot().unwrap().page(*offset,*limit,generation.as_deref()).unwrap(),
                        crate::debug_control::Action::CheatShow{id,offset,generation}=>value["cheat_detail"]=self.current_cheat_snapshot().unwrap().show(id,*offset,generation.as_deref()).unwrap(),
                        _=>{},
                    }}}
                    if !matches!(request.command,crate::debug_control::Action::Status)&&self.events_enabled(){self.ui_event("control-complete",serde_json::json!({"command":request.command,"outcome":if error.is_some(){"refused"}else{"complete"},"error":error}));}
                    if let Err(error)=self.control_server.as_mut().unwrap().reply(&request,status.unwrap_or_default(),error){self.recent_error(&format!("Control reply: {error}"));eprintln!("debug control: {error}");}
                }
            }
        }
        self.finish_control();
    }

    fn finish_control(&mut self) {
        if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::CheatRefresh)) {
            if !self.cheat_refresh.as_ref().is_some_and(|j|j.is_finished()){return;}
            let result=self.cheat_refresh.take().unwrap().finish().and_then(|snapshot|{
                let valid=self.content_picker.rom_context().is_some_and(|rom|snapshot.matches(self.content_picker.reference(menu::ContentSlot::CheatCatalog),rom));
                if !valid{return Err("Cheat selection or ROM context changed during refresh".into());}self.cheat_snapshot=Some(snapshot);Ok(())
            });
            let request=self.control_server.as_ref().unwrap().pending.as_ref().unwrap().clone();let _context=self.events.clock.context(request.event_context());
            let status=self.debug_status();let error=result.err().or_else(||status.as_ref().err().cloned());
            if self.events_enabled(){self.ui_event("control-complete",serde_json::json!({"command":request.command,"outcome":if error.is_some(){"refused"}else{"complete"},"error":error}));}
            if let Err(error)=self.control_server.as_mut().unwrap().finish(status.unwrap_or_default(),error){self.recent_error(&format!("Control completion reply: {error}"));eprintln!("debug control: {error}");}return;
        }
        if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::Replay{..})) {
            if self.replay_job.is_some(){return;}
            let (outcome,error)=match self.replay_terminal.as_ref(){
                Some(Ok(report))=>match report.outcome.as_str(){
                    "passed"|"captured"|"matched"=>("ok",None),
                    "interrupted"=>("cancelled",Some(report.message.clone())),
                    "timeout"=>("timeout",Some(report.message.clone())),
                    "incompatible"=>("incompatible",Some(report.message.clone())),
                    "diverged"=>("diverged",Some(report.message.clone())),
                    "not-reached"=>("not-reached",Some(report.message.clone())),
                    _=>("failed",Some(report.message.clone())),
                },
                Some(Err(error))=>("failed",Some(error.clone())),
                None=>("failed",Some("Replay worker ended without a result".into())),
            };
            let request=self.control_server.as_ref().unwrap().pending.as_ref().unwrap().clone();
            let _context=self.events.clock.context(request.event_context());
            let status=self.debug_status();let error=error.or_else(||status.as_ref().err().cloned());
            let outcome=if error.is_some()&&outcome=="ok"{"failed"}else{outcome};
            if self.events_enabled(){self.ui_event("control-complete",serde_json::json!({"command":request.command,"outcome":outcome,"error":error}));}
            if let Err(error)=self.control_server.as_mut().unwrap().finish_classified(status.unwrap_or_default(),error,Some(outcome)){self.recent_error(&format!("Control completion reply: {error}"));eprintln!("debug control: {error}");}
            return;
        }
        if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::ContentRefresh)) {
            if self.control_server.as_ref().is_some_and(|s|s.deadline_expired()){self.content_picker.cancel_refresh_control();}
            let Some(result)=self.content_picker.poll_refresh_control(&mut self.menu) else{return;};
            if self.menu.content_active(){self.menu.content_busy(false);self.menu_dirty=true;}
            let request=self.control_server.as_ref().unwrap().pending.as_ref().unwrap().clone();let _context=self.events.clock.context(request.event_context());
            let status=self.debug_status();let error=result.err().or_else(||status.as_ref().err().cloned());
            if self.events_enabled(){self.ui_event("control-complete",serde_json::json!({"command":request.command,"outcome":if error.is_some(){"refused"}else{"complete"},"error":error}));}
            if let Err(error)=self.control_server.as_mut().unwrap().finish(status.unwrap_or_default(),error){self.recent_error(&format!("Control completion reply: {error}"));eprintln!("debug control: {error}");}
            return;
        }
        if self.control_server.as_ref().and_then(|s|s.pending.as_ref()).is_some_and(|r|matches!(r.command,crate::debug_control::Action::SelectId{..})) {
            if self.control_server.as_ref().is_some_and(|s|s.deadline_expired()){self.content_picker.cancel_id();}
            let Some(result)=self.content_picker.poll_id(&mut self.menu) else{return;};
            self.menu_dirty=true;
            let request=self.control_server.as_ref().unwrap().pending.as_ref().unwrap().clone();
            let _context=self.events.clock.context(request.event_context());
            let status=self.debug_status();
            let error=result.err().or_else(||status.as_ref().err().cloned());
            if self.events_enabled(){self.ui_event("control-complete",serde_json::json!({"command":request.command,"outcome":if error.is_some(){"refused"}else{"complete"},"error":error}));}
            if let Err(error)=self.control_server.as_mut().unwrap().finish(status.unwrap_or_default(),error){self.recent_error(&format!("Control completion reply: {error}"));eprintln!("debug control: {error}");}
            return;
        }
        if self.frame_control.remaining==0 && self.probe.is_none() && self.control_server.as_ref().is_some_and(|s|s.pending.is_some()) {
            let request=self.control_server.as_ref().unwrap().pending.as_ref().unwrap().clone();
            let _context=self.events.clock.context(request.event_context());
            let status=self.debug_status();
            let error=if !["step-complete","search-hit","not-reached"].contains(&self.frame_control.reason) {Some(format!("Step stopped: {}",self.frame_control.reason))} else {status.as_ref().err().cloned()};
            if self.events_enabled(){self.ui_event("control-complete",serde_json::json!({"command":request.command,"outcome":if error.is_some(){"refused"}else{"complete"},"error":error,"reason":self.frame_control.reason}));}
            if let Err(error)=self.control_server.as_mut().unwrap().finish(status.unwrap_or_default(),error){self.recent_error(&format!("Control completion reply: {error}"));eprintln!("debug control: {error}");}
        }
    }

    fn workbench_request(&mut self,request:menu::ToolRequest)->Result<(),String>{
        use crate::debug_lifecycle::Family as F;
        let tool=self.menu.tool_values().0;let was_busy=self.workbench.busy();
        let result=self.workbench_request_inner(request);
        let family=if tool.library(){F::Script}else{F::Workbench};
        if result.is_ok()&&!was_busy&&self.workbench.busy(){
            self.lifecycle_begin(family,serde_json::json!({"tool":format!("{tool:?}"),"operation":format!("{request:?}"),"scope":"One existing host worker; no new guest executor"}));
        }else{
            let context=self.lifecycle.context(family);
            self.lifecycle_emit(family,if request==menu::ToolRequest::Prepare&&was_busy&&tool.library(){"cancellation-requested"}else if request==menu::ToolRequest::Back{"view-left"}else{"handler-result"},context,
                serde_json::json!({"tool":format!("{tool:?}"),"operation":format!("{request:?}"),"outcome":if result.is_ok(){"accepted"}else{"refused"},"error":result.as_ref().err().map(|e|e.chars().take(1024).collect::<String>()),"worker_continues":self.workbench.busy()}));
        }
        result
    }
    fn workbench_request_inner(&mut self,request:menu::ToolRequest)->Result<(),String>{
        use menu::ToolRequest;
        if request==ToolRequest::Back{self.workbench.remember(&self.menu);self.workbench.cancel_view();return Ok(());}
        if self.menu.tool_values().0.library() && request==ToolRequest::Prepare {
            if self.menu.tool_values().0==menu::DebugTool::GuestScripts {self.workbench.open(&mut self.menu,menu::DebugTool::Gameplay);return Ok(());}
            self.workbench.cancel_process()?;
            self.menu.tool_feedback(self.workbench.last.clone(),String::new(),true);return Ok(());
        }
        if !self.paused || self.probe.is_some() || self.frame_control.remaining>0 || self.replay_job.is_some() || self.recording_active(){return Err("Tool draft actions require an idle paused copy without recording/replay/step/search.".into());}
        let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);
        let draft=self.workbench.draft(&self.menu,&self.content_picker,rom)?;
        if request==ToolRequest::Run && draft.tool.library() {return self.workbench.request(&mut self.menu,draft,request);}
        if request==ToolRequest::Run{
            if self.workbench.busy(){return Err("A workbench host operation is still finishing".into());}
            if draft.tool.creates_profile(){
                let path=self.workbench.prepared_for(draft.tool)?;
                let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);
                self.content_picker.adopt(menu::ContentSlot::Replay,&path,&rom,&mut self.menu)?;
                let kind=match draft.tool{menu::DebugTool::Audit=>ReplayKind::Audit,menu::DebugTool::Capture=>ReplayKind::Capture,menu::DebugTool::Compare=>ReplayKind::Comparison,_=>ReplayKind::Plain};
                self.start_replay(false,kind);
                if self.replay_job.is_none(){return Err(self.menu.diagnosis_message().into());}
                self.workbench.last="Explicit prepared-profile request dispatched; inspect existing replay progress/result.".into();
            }else{
                let action=draft.control()?;self.debug_action(&action)?;
                self.workbench.last=format!("Accepted by shared handler: {action:?}. Observers require explicit steps/resume and existing report action.");
            }
            self.menu.tool_feedback(self.workbench.last.clone(),String::new(),false);return Ok(());
        }
        self.workbench.request(&mut self.menu,draft,request)
    }
    fn choose_tool_input(&mut self,slot:menu::ContentSlot,message:String){
        let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);
        if let Err(error)=self.content_picker.open_for(&mut self.menu,rom,slot,message){self.menu.diagnosis_result(error);}
        self.menu_dirty=true;
    }
    fn selected_replay_profile(&mut self)->Result<std::path::PathBuf,String>{
        let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);
        if !self.content_picker.has_slot(menu::ContentSlot::Replay){
            if let Some(path)=self.replay_path.as_ref(){self.content_picker.adopt(menu::ContentSlot::Replay,path,&rom,&mut self.menu)?;}
        }
        let path=self.content_picker.slot_path(menu::ContentSlot::Replay,&rom)?;
        self.replay_path=Some(path.clone());Ok(path)
    }
    fn start_replay(&mut self,choose_only:bool,kind:ReplayKind) {
        let _=self.start_replay_request(choose_only,kind,true);
    }
    fn start_replay_request(&mut self,choose_only:bool,kind:ReplayKind,interactive:bool)->Result<bool,String> {
        let already_running=self.replay_job.is_some();
        let result=self.start_replay_inner(choose_only,kind,interactive);
        if let Err(error)=&result{self.menu.diagnosis_result(error.clone());self.menu_dirty=true;}
        if !already_running&&self.replay_job.is_some(){
            let job=self.replay_job.as_ref().unwrap();let run=job.run.clone();let profile=job.profile.to_string_lossy().chars().take(1024).collect::<String>();
            self.replay_preparation_seen=0;
            self.lifecycle_begin(crate::debug_lifecycle::Family::Replay,serde_json::json!({"worker_run":run,"profile":profile,"scope":"Separate pinned worker; its frame counts are not the main guest frame/clock"}));
        }else if !already_running{
            self.lifecycle_emit(crate::debug_lifecycle::Family::Replay,if choose_only||self.menu.content_active(){"input-selection"}else{"not-started"},None,serde_json::json!({"outcome":if choose_only||self.menu.content_active(){"requires-input"}else{"refused"},"feedback":self.menu.diagnosis_message().chars().take(512).collect::<String>(),"worker_started":false}));
        }
        result
    }
    fn start_replay_inner(&mut self, choose_only: bool, kind: ReplayKind,interactive:bool)->Result<bool,String> {
        if self.recent.as_ref().is_some_and(|b|!b.saved()){return Err("Save the recent window before starting a separate replay.".into());}
        if self.gameplay_trace.is_some(){return Err("Save the gameplay trace before starting a separate replay.".into());}
        if self.code_trace.is_some(){return Err("Save the code trace before starting a separate replay.".into());}
        if self.audio_diagnostic || self.streams.lock().unwrap().diagnostic.is_some() {return Err("Save the audio capture before starting a separate replay.".into());}
        if self.frame_control.remaining>0 || self.probe.is_some() {return Err("Finish the frame step/search before starting a replay.".into());}
        if self.replay_job.is_some() {return Err("A replay is already running. Use Cancel replay.".into());}
        if self.recording_active() {return Err("Stop input recording before replaying a case.".into());}
        if choose_only {self.choose_tool_input(menu::ContentSlot::Replay,"Choose a profile; this does not start a replay.".into());return Ok(false);}
        let path=match self.selected_replay_profile(){Ok(path)=>path,Err(e) if interactive=>{self.choose_tool_input(menu::ContentSlot::Replay,e);return Ok(false);},Err(e)=>return Err(e)};
        let result=match kind {ReplayKind::Comparison=>crate::debug_replay::Job::start_comparison(&path),ReplayKind::Capture=>crate::debug_replay::Job::start_capture(&path),ReplayKind::Audit=>crate::debug_replay::Job::start_audit(&path),ReplayKind::Plain=>crate::debug_replay::Job::start(&path)};
        let job=result.map_err(|e|format!("Cannot replay: {e}"))?;
        self.replay_path=Some(job.profile.clone());self.replay_message.clear();self.replay_terminal=None;self.menu.diagnosis_result("Replay running in an isolated worker. Cancel replay stops it.".into());self.replay_job=Some(job);
        self.menu_dirty=true;
        Ok(true)
    }

    fn poll_replay(&mut self) {
        let Some(job)=&mut self.replay_job else{return;};
        let result=job.poll();let terminal=matches!(result,Ok(Some(_))|Err(_));
        let report=match &result{Ok(Some(report))=>Some(report.clone()),_=>job.progress().cloned()};
        let message=match &result {Ok(Some(report))=>{eprintln!("Replay result: {}",serde_json::to_string(report).unwrap_or_default());report.short()},Ok(None)=>report.as_ref().map_or_else(||"Replay starting.".into(),|r|r.short()),Err(error)=>format!("Replay error: {error}")};
        if let Some(report)=&report {
            let context=self.lifecycle.context(crate::debug_lifecycle::Family::Replay);
            if self.replay_preparation_seen==0&&report.preparation.len()>8{self.lifecycle_emit(crate::debug_lifecycle::Family::Replay,"preparation-limit",context.clone(),serde_json::json!({"omitted_records":report.preparation.len()-8,"max_records":8,"scope":"Unexpected extra worker metadata was not forwarded"}));}
            for preparation in report.preparation.iter().take(8).skip(self.replay_preparation_seen){
                let family=match preparation["family"].as_str(){Some("dataset")=>crate::debug_lifecycle::Family::Dataset,Some("cache")=>crate::debug_lifecycle::Family::Cache,_=>continue};
                self.lifecycle_emit(family,"worker-preparation",context.clone(),serde_json::json!({"worker_run":report.run,"worker_phase":preparation,"outcome":preparation["outcome"],"scope":"Retained worker preparation observations received at this host poll; not instantaneous guest chronology"}));
            }
            self.replay_preparation_seen=report.preparation.len().min(8);
            if !terminal&&self.events_enabled()&&self.lifecycle.due(crate::debug_lifecycle::Family::Replay){
                if let Some((context,details))=self.lifecycle.progress(crate::debug_lifecycle::Family::Replay,serde_json::json!({"worker_run":report.run,"outcome":report.outcome,"completed":report.completed,"frames":report.frames,"message":report.message.chars().take(512).collect::<String>()})){self.lifecycle_emit(crate::debug_lifecycle::Family::Replay,"progress",Some(context),details);}
            }
        }
        if message!=self.replay_message{self.replay_message=message.clone();self.menu.diagnosis_result(message);self.menu_dirty=true;}
        if terminal {
            self.replay_terminal=Some(match &result{Ok(Some(report))=>Ok(report.clone()),Err(error)=>Err(error.clone()),_=>unreachable!()});
            let details=match result {Ok(Some(report))=>serde_json::json!({"worker_run":report.run,"outcome":report.outcome,"completed":report.completed,"frames":report.frames,"message":report.message.chars().take(1024).collect::<String>(),"audit_report":report.audit_report,"capture_catalog":report.capture_catalog,"comparison_report":report.comparison_report}),Err(error)=>serde_json::json!({"outcome":"error","error":error.chars().take(1024).collect::<String>()}),_=>unreachable!()};
            self.lifecycle_finish(crate::debug_lifecycle::Family::Replay,details);self.replay_job=None;
        }
    }

    fn cancel_replay(&mut self) {
        let _=self.cancel_replay_request();
    }
    fn cancel_replay_request(&mut self)->Result<(),String> {
        let context=self.lifecycle.context(crate::debug_lifecycle::Family::Replay);
        let result=match &mut self.replay_job {Some(job)=>job.cancel(),None=>Err("No replay is running.".into())};
        let (message,outcome)=match &result {
            Ok(())=>("Replay cancellation requested.".into(),"accepted"),
            Err(error)=>(format!("Cannot cancel: {error}"),"refused"),
        };
        self.lifecycle_emit(crate::debug_lifecycle::Family::Replay,"cancellation-request",context,serde_json::json!({"outcome":outcome,"message":message,"scope":"Request outcome; final worker interruption is a separate terminal observation"}));
        self.menu.diagnosis_result(message);self.menu_dirty=true;
        result
    }

    fn recording_title(&self) -> String { format!("{}{}", self.title, if self.capture.is_some() { " [REC]" } else { "" }) }

    fn recording_active(&self) -> bool { self.record.is_some() || self.capture.is_some() }

    fn start_input_recording(&mut self) {
        let result = (|| -> Result<(), String> {
            if self.probe.is_some() {return Err("Finish or interrupt the search before recording.".into());}
            if !self.debug_session { return Err("Open an isolated debug copy before recording.".into()); }
            if self.last_cheat_once.is_some(){return Err("Recording after a cheat one-shot needs provenance support; restart the isolated copy first.".into());}
            if self.replay_job.is_some() {return Err("Finish or cancel the replay before recording.".into());}
            if self.recording_active() { return Err("An input recording is already active.".into()); }
            self.sync_view();
            self.game_mouse.cancel(); self.machine.bus.ffta_battle_mouse.cancel();
            let directory = crate::debug_recording::destination()?;
            self.capture = Some(crate::debug_recording::Capture::start(&directory, &self.machine,
                self.native.is_none(), self.diagnosis.dataset.as_deref(), &self.av, &self.icfg)?);
            eprintln!("Input recording started: {}", directory.display());
            if let Some(window) = &self.window { window.set_title(&format!("{} [REC]", self.title)); }
            Ok(())
        })();
        match &result{Ok(())=>self.lifecycle_begin(crate::debug_lifecycle::Family::Recording,serde_json::json!({"directory":self.capture.as_ref().map(|c|c.directory.to_string_lossy().chars().take(1024).collect::<String>()),"scope":"Snapshot-relative input writer started by the existing handler"})),Err(error)=>self.lifecycle_emit(crate::debug_lifecycle::Family::Recording,"refused",None,serde_json::json!({"outcome":"refused","error":error.chars().take(1024).collect::<String>()}))};
        self.menu.diagnosis_result(match result { Ok(()) => "Recording active. Resume to reproduce.".into(), Err(e) => format!("Cannot record: {e}") });
        self.menu_dirty = true;
    }

    fn stop_input_recording(&mut self, reason: &str, complete: bool) -> bool {
        let mut saved=false;let mut terminal=serde_json::json!({"outcome":"not-running","reason":reason});
        let message=if let Some(capture)=self.capture.take(){
            match capture.finish(&self.machine,reason,complete){
                Ok(path)=>{saved=true;eprintln!("Input recording completed: {}",path.display());terminal=serde_json::json!({"outcome":"completed","reason":reason,"complete":complete,"directory":path.to_string_lossy().chars().take(1024).collect::<String>(),"manifest":path.join("recording.json").to_string_lossy().chars().take(1024).collect::<String>()});format!("Recording saved: {}",path.file_name().unwrap_or_default().to_string_lossy())},
                Err(error)=>{eprintln!("Input recording: {error}");terminal=serde_json::json!({"outcome":"error","reason":reason,"error":error.chars().take(1024).collect::<String>()});format!("Recording error: {error}")},
            }
        }else if let Some((file,_))=self.record.take(){
            match file.sync_all(){Ok(())=>{terminal=serde_json::json!({"outcome":"completed","legacy":true,"reason":reason});"Legacy input recording stopped; no snapshot-relative manifest was created.".into()},Err(error)=>{terminal=serde_json::json!({"outcome":"error","legacy":true,"error":error.to_string().chars().take(1024).collect::<String>()});format!("Recording error: {error}")}}
        }else{"No input recording is active.".into()};
        self.lifecycle_finish(crate::debug_lifecycle::Family::Recording,terminal);
        self.menu.diagnosis_result(message);if let Some(window)=&self.window{window.set_title(&self.title);}self.menu_dirty=true;saved
    }

    fn current_cheat_snapshot(&self)->Result<&crate::debug_cheats::Snapshot,String>{
        let snapshot=self.cheat_snapshot.as_ref().ok_or("Refresh the selected cheat catalog first")?;
        let rom=self.content_picker.rom_context().ok_or("No ROM context")?;
        if !snapshot.matches(self.content_picker.reference(menu::ContentSlot::CheatCatalog),rom){return Err("Cheat selection or ROM context changed; refresh the catalog".into());}
        Ok(snapshot)
    }

    fn prepare_cheat_once(&self,id:&str,provenance:u16,master:&str,generation:&str)->Result<(crate::debug_cheat_decoder::Plan,serde_json::Value),String>{
        if !self.paused || self.probe.is_some() || self.frame_control.remaining>0 || self.replay_job.is_some() || self.recording_active(){return Err("One-shot requires an idle paused isolated copy without recording or replay".into());}
        if self.native.is_some(){return Err("One-shot currently requires the interpreter; native cache invalidation is unsupported".into());}
        let reference=self.content_picker.reference(menu::ContentSlot::CheatCatalog).ok_or("Select a cheat catalog first")?;
        let (plan,info)=self.current_cheat_snapshot()?.prepare_once(reference,&self.machine.bus.rom,id,usize::from(provenance),master,generation)?;
        let state=gba_core::ffta_rom::digest(&self.machine.save_state()?);
        let body=serde_json::json!({"state_sha256":state,"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"details":info});
        let token=gba_core::ffta_rom::digest(&serde_json::to_vec(&body).map_err(|e|e.to_string())?);
        Ok((plan,serde_json::json!({"token":token,"body":body,"scope":"Preview only; source behavior and original hardware hook remain unverified"})))
    }

    fn open_cheat_catalog(&mut self) {
        let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);
        let result=(||->Result<Option<crate::debug_cheats::Catalog>,String>{
            if !self.content_picker.has_slot(menu::ContentSlot::CheatCatalog){
                let path=input_config::exe_dir().ok_or("No executable directory")?.join("cheats").join("catalogo.json");
                if !path.exists(){return Ok(None);}
                self.content_picker.adopt(menu::ContentSlot::CheatCatalog,&path,&rom,&mut self.menu)?;
            }
            let path=self.content_picker.slot_path(menu::ContentSlot::CheatCatalog,&rom)?;
            let expected=self.content_picker.references().get(&menu::ContentSlot::CheatCatalog).and_then(|r|r.entry.sha256.clone()).ok_or("Catalog selection has no source identity")?;
            let catalog=crate::debug_cheats::Catalog::load(&path)?;
            if catalog.source_sha256!=expected{return Err("Catalog changed during import; select it again".into());}
            Ok(Some(catalog))
        })();
        match result {
            Ok(Some(catalog))=>{let entries=catalog.menu_entries(Some(&rom));let message=format!("{} voci · tutte disattivate · compatibilità non verificata",entries.len());self.menu.open_cheats(entries,message);},
            Ok(None)=>self.menu.open_cheats(Vec::new(),"Catalogo assente. Scegli Importa catalogo; nessuna voce è attiva.".into()),
            Err(error)=>self.menu.open_cheats(Vec::new(),format!("Catalogo non disponibile: {error}. Importa una sorgente valida.")),
        }
        self.menu_dirty=true;
    }

    fn open_debug_catalog(&mut self, choose: bool) {
        if choose {self.choose_tool_input(menu::ContentSlot::Catalog,"Choose a save catalog; then open Debug save catalog separately.".into());return;}
        let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);
        let selected=(||{
            if !self.content_picker.has_slot(menu::ContentSlot::Catalog){
                let path=self.debug_catalog.as_ref().map(|c|c.path.clone()).or_else(crate::debug_catalog::default_path).filter(|p|p.is_file()).ok_or("Select a save catalog; Refresh lists declared roots.")?;
                self.content_picker.adopt(menu::ContentSlot::Catalog,&path,&rom,&mut self.menu)?;
            }
            self.content_picker.slot_path(menu::ContentSlot::Catalog,&rom)
        })();
        let path=match selected{Ok(path)=>path,Err(error)=>{self.choose_tool_input(menu::ContentSlot::Catalog,error);return;}};
        match crate::debug_catalog::Catalog::load(&path) {
            Ok(catalog)=>{
                let entries=catalog.entries.iter().map(|e|menu::CatalogEntry {key:e.key.clone(),name:e.name.clone(),kind:e.kind.clone(),case:e.case.clone(),region:e.region.clone(),status:e.status.clone(),slot:e.slot,source:e.source.clone(),evidence:e.evidence.clone(),available:e.available,error:e.error.clone().unwrap_or_default()}).collect();
                let region=match gba_core::ffta_rom::version(&self.machine.bus.rom) {Some(gba_core::ffta_rom::Version::Europe)=>"EUR",Some(gba_core::ffta_rom::Version::Usa)=>"USA",None=>""};
                self.debug_catalog=Some(catalog);self.menu.set_catalog(entries,region);
                if let Some(index)=self.menu.catalog_selection(){self.catalog_preview(index);}
            },
            Err(e)=>{self.menu.diagnosis_result(format!("Cannot open catalog: {e}"));self.menu.catalog_result(format!("Cannot open catalog: {e}"));},
        }
        self.menu_dirty=true;
    }

    fn catalog_preview(&mut self,index:usize) {
        if let Some(catalog)=&self.debug_catalog {
            match catalog.preview(index) {Ok(preview)=>self.menu.catalog_preview(preview),Err(e)=>{self.menu.catalog_preview(Vec::new());self.menu.catalog_result(e);}}
        }
        self.menu_dirty=true;
    }

    fn catalog_import(&mut self,index:usize) {
        if !self.debug_session {self.menu.catalog_result("Import requires a debug copy; use Open isolated case.".into());return;}
        let result=(||->Result<u8,String>{
            let catalog=self.debug_catalog.as_ref().ok_or("Open a catalog first")?;
            // Use the live owner so the import remains the last undoable file action.
            self.state_slots.set_extended(true);
            catalog.import(index,&self.machine,&mut self.state_slots,None)
        })();
        match result {
            Ok(slot)=>{
                if slot>9 {self.av.debug_extended_saves=true;let _=self.av.save();}
                self.refresh_state_slots();self.menu.reveal_imported_slot(slot);
                self.menu.state_result(format!("Catalog checkpoint imported into slot {slot}. Select Load state to use it."));
            },Err(e)=>self.menu.catalog_result(format!("Cannot import: {e}")),
        }
        self.menu_dirty=true;
    }

    fn catalog_open(&mut self,index:usize) {
        if self.debug_session {self.menu.catalog_result("Close this debug copy to open another; import a checkpoint here instead.".into());return;}
        if self.recording_active(){self.menu.catalog_result("Stop input recording before opening a case.".into());return;}
        let result=(||->Result<(),String>{
            let catalog=self.debug_catalog.as_ref().ok_or("Open a catalog first")?;
            let e=catalog.entries.get(index).ok_or("Select a catalog entry")?;
            let destination=crate::debug_session::default_destination()?;
            let suffix=destination.file_name().unwrap_or_default().to_string_lossy();
            let label=format!("{}-{}-{suffix}",if e.id.is_empty(){"variant"}else{&e.id},e.region);
            let destination=destination.with_file_name(label);
            let root=catalog.create(index,&destination,&self.machine.bus.rom,
                self.machine.bus.real_bios.then_some(self.machine.bus.bios.as_slice()),self.native.is_none(),
                self.av.clone(),self.icfg.clone(),self.diagnosis.dataset.as_deref())?;
            crate::debug_session::launch(&root)?;
            eprintln!("Catalog isolated case: {}",root.display());Ok(())
        })();
        self.menu.catalog_result(match result{Ok(())=>"Catalog copy opened; this game stays paused.".into(),Err(e)=>format!("Cannot open case: {e}")});self.menu_dirty=true;
    }

    fn copy_diagnosis(&mut self) {
        self.diagnosis.save = Some(self.sav_path.clone().into());
        self.diagnosis.slot = Some(self.menu.selected_state_slot());
        self.diagnosis.interp = self.native.is_none();
        self.diagnosis.recording = self.recording_active();
        if let Some(capture) = &self.capture { self.diagnosis.input = Some(capture.directory.join("input.input")); }
        let result = (|| -> Result<String, String> {
            let path = crate::diagnostics::default_report_path()?;
            self.diagnosis.out = Some(path.clone());
            let mut report = crate::diagnostics::collect(&self.diagnosis, Some(&self.machine));
            report["live_settings"] = serde_json::json!({
                "debug_session": self.debug_session,
                "aspect": self.av.video_aspect, "extended_saves": self.av.debug_extended_saves,
                "audio_enhanced": self.av.audio_enhanced,
                "note": "Live settings can differ from the config file while applying a change."
            });
            crate::diagnostics::write_report(&path, &report)?;
            eprintln!("Diagnosis report: {}", path.display());
            let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
            let copied = (|| -> Result<(), String> {
                if self.diagnosis_clipboard.is_none() {
                    self.diagnosis_clipboard = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
                }
                self.diagnosis_clipboard.as_mut().unwrap().set_text(text).map_err(|e| e.to_string())
            })();
            let status = if report["ok"] == true { "OK" } else { "check issues" };
            match copied {
                Ok(()) => Ok(format!("Diagnosis copied ({status}); report saved.")),
                Err(e) => {
                    eprintln!("Diagnosis clipboard unavailable: {e}; report retained at {}", path.display());
                    Ok(format!("Report saved ({status}); clipboard unavailable."))
                }
            }
        })();
        self.menu.diagnosis_result(result.unwrap_or_else(|e| format!("Diagnosis failed: {e}")));
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn refresh_state_slots(&mut self) {
        self.state_slots.set_extended(self.av.debug_extended_saves);
        let slots = (1..=self.state_slots.count()).map(|slot| {
            let info = self.state_slots.info(slot);
            menu::StateSlot {
                occupied: info.occupied,
                preview: info.preview,
                status: info.status,
            }
        }).collect();
        self.menu.set_state_slots(slots);
        let times=(1..=self.state_slots.count()).map(|slot|self.state_slots.path(slot).ok().and_then(|path|std::fs::metadata(path).ok()).and_then(|m|m.modified().ok()).and_then(|time|time.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_secs())).collect();
        self.menu.set_state_timestamps(times);
        match std::fs::metadata(&self.sav_path) {
            Ok(metadata) => {
                if let Some(stamp) = metadata.modified().ok().and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()) {
                    self.menu.set_standard_save_timestamp(stamp);
                } else {
                    self.menu.set_standard_save_status("Ultima modifica file .sav: data non disponibile".into());
                }
            }
            Err(e) if e.kind()==std::io::ErrorKind::NotFound => self.menu.set_standard_save_status("Nessun salvataggio standard trovato".into()),
            Err(_) => self.menu.set_standard_save_status("File .sav: metadati non disponibili".into()),
        }
        self.menu.set_state_undo_description(self.state_slots.undo_description());
        let expected = self.machine.bus.save_data().map_or(0, |data| data.len());
        if self.av.debug_extended_saves {
            self.menu.set_game_saves(self.game_saves.choices(expected));
        }
        let choices = self.save_backups.choices(expected);
        self.menu
            .set_save_backups(choices.map(|choice| menu::BackupInfo {
                generation: choice.generation,
                status: choice.status,
            }));
    }

    /// Capture the displayed game view plus native UI, excluding the pause menu and FPS HUD.
    fn state_preview(&mut self) -> Vec<[u8; 4]> {
        self.preview_frame(crate::save_state::PREVIEW_W,crate::save_state::PREVIEW_H,true)
    }
    fn preview_frame(&mut self,w:usize,h:usize,include_ui:bool)->Vec<[u8;4]>{
        self.temporal.compose(&self.lut, &mut self.rgba);
        let mut pixels = vec![[0, 0, 0, 255]; w * h];
        let [vx, vy, vw, vh] = self.view;
        let (scale, ox, oy) = screen::present::letterbox_f((w as u32, h as u32), vw, vh);
        for y in 0..h {
            for x in 0..w {
                let fx = (x as f32 - ox) / scale;
                let fy = (y as f32 - oy) / scale;
                if fx >= 0. && fy >= 0. && fx < vw && fy < vh {
                    pixels[y * w + x] = self.rgba[((vy + fy) as usize).min(wide_h() - 1)
                        * wide_w()
                        + ((vx + fx) as usize).min(wide_w() - 1)];
                }
            }
        }
        for layer in 0..if include_ui{2}else{0} {
            if let Some(layout) = self.ui_layout((w as u32, h as u32), layer) {
                let ui = self.ui_frame(layout, layer);
                for y in 0..h {
                    for x in 0..w {
                        if let Some(color) = ui.sample(x as f32 + 0.5, y as f32 + 0.5) {
                            pixels[y * w + x] = color;
                        }
                    }
                }
            }
        }
        pixels
    }

    fn switch_game_save(&mut self, slot: u8) {
        if !self.av.debug_extended_saves { return; }
        if self.recording_active() {
            self.menu.game_save_result("Stop input recording before switching saves.".into());
            self.menu_dirty = true;
            return;
        }
        let message = match self.game_saves.switch(&mut self.machine, &self.boot_state, slot) {
            Ok(path) => {
                self.sav_path = path.to_string_lossy().into_owned();
                self.sav_seen = self.machine.bus.save_data().map(fnv64);
                self.save_backups = crate::save_backups::Backups::new(&path);
                self.state_slots = crate::save_state::Slots::new(&path, &self.machine.bus.rom);
                self.av.debug_game_save_slot = slot;
                crate::save_path::remember(&path);
                let persisted = self.av.save();
                let enabled = self.icfg.mouse_keyboard || std::env::var("FFTA_MOUSE").as_deref() == Ok("1");
                self.game_mouse = gba_core::ffta_mouse::LanguageMouse::new(&self.machine.bus.rom, enabled);
                self.machine.bus.ffta_battle_mouse.set_enabled(self.icfg.mouse_keyboard);
                self.after_state_restore();
                match persisted {
                    Ok(()) => self.menu_text("Slot {slot:02} active. Resume, then load inside the game.", &[("slot", format!("{:02}",slot))]),
                    Err(e) => self.menu_text("Slot {slot:02} active; cannot remember selection: {e}", &[("slot", format!("{:02}",slot)), ("e", (e).to_string())]),
                }
            },
            Err(e) => self.menu_text("Cannot switch save: {e}", &[("e", (e).to_string())]),
        };
        self.refresh_state_slots();
        self.menu.game_save_result(message);
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn copy_game_save(&mut self, slot: u8, overwrite: bool) {
        if !self.av.debug_extended_saves { return; }
        let message = match self.game_saves.copy_current(&self.machine, slot, overwrite) {
            Ok(()) => self.menu_text("Copied current game save to slot {slot:02}.", &[("slot", format!("{:02}",slot))]),
            Err(e) => self.menu_text("Cannot copy save: {e}", &[("e", (e).to_string())]),
        };
        self.refresh_state_slots();
        self.menu.game_save_result(message);
        self.menu_dirty = true;
    }

    fn restart_rom(&mut self) {
        self.lifecycle_emit(crate::debug_lifecycle::Family::State,"requested",None,serde_json::json!({"operation":"restart"}));
        if self.recording_active() {
            self.lifecycle_emit(crate::debug_lifecycle::Family::State,"refused",None,serde_json::json!({"operation":"restart","outcome":"refused","reason":"Input recording active"}));
            self.menu.session_result("Stop recording before restarting.".into());
            self.menu_dirty = true;
            return;
        }
        match crate::save_backups::restart(&mut self.machine, &self.boot_state, Path::new(&self.sav_path)) {
            Ok(()) => {
                self.lifecycle_emit(crate::debug_lifecycle::Family::State,"completed",None,serde_json::json!({"operation":"restart","outcome":"completed","resume_requested":true}));
                self.sav_seen = self.machine.bus.save_data().map(fnv64);
                self.state_slots = crate::save_state::Slots::new(Path::new(&self.sav_path), &self.machine.bus.rom);
                let enabled = self.icfg.mouse_keyboard || std::env::var("FFTA_MOUSE").as_deref() == Ok("1");
                self.game_mouse = gba_core::ffta_mouse::LanguageMouse::new(&self.machine.bus.rom, enabled);
                self.machine.bus.ffta_battle_mouse.set_enabled(self.icfg.mouse_keyboard);
                self.after_state_restore();
                self.refresh_state_slots();
                self.menu.reset();
                self.paused = false;
            }
            Err(error) => {
                self.lifecycle_emit(crate::debug_lifecycle::Family::State,"refused",None,serde_json::json!({"operation":"restart","outcome":"refused","error":error.chars().take(1024).collect::<String>()}));
                eprintln!("Cannot restart ROM: {error}");
                self.menu.session_result(self.menu_text("Cannot restart ROM: {error}", &[("error", (error).to_string())]));
            }
        }
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn restore_save_backup(&mut self, generation: u64) {
        self.lifecycle_emit(crate::debug_lifecycle::Family::State,"requested",None,serde_json::json!({"operation":"restore-backup","generation":generation}));
        if self.recording_active() {
            self.lifecycle_emit(crate::debug_lifecycle::Family::State,"refused",None,serde_json::json!({"operation":"restore-backup","outcome":"refused","reason":"Input recording active"}));
            self.menu
                .backup_result("Stop input recording before restoring a backup.".into());
            self.menu_dirty = true;
            return;
        }
        let message =
            match self
                .save_backups
                .restore(generation, &mut self.machine, &self.boot_state)
            {
                Ok(()) => {
                    self.lifecycle_emit(crate::debug_lifecycle::Family::State,"completed",None,serde_json::json!({"operation":"restore-backup","generation":generation,"outcome":"completed"}));
                    self.sav_seen = self.machine.bus.save_data().map(fnv64);
                    self.state_slots = crate::save_state::Slots::new(
                        Path::new(&self.sav_path),
                        &self.machine.bus.rom,
                    );
                    let enabled = self.icfg.mouse_keyboard
                        || std::env::var("FFTA_MOUSE").as_deref() == Ok("1");
                    self.game_mouse =
                        gba_core::ffta_mouse::LanguageMouse::new(&self.machine.bus.rom, enabled);
                    self.machine
                        .bus
                        .ffta_battle_mouse
                        .set_enabled(self.icfg.mouse_keyboard);
                    self.after_state_restore();
                    "Backup restored. Resume to start the game.".into()
                }
                Err(error) => {self.lifecycle_emit(crate::debug_lifecycle::Family::State,"refused",None,serde_json::json!({"operation":"restore-backup","outcome":"refused","error":error.chars().take(1024).collect::<String>()}));self.menu_text("Cannot restore backup: {error}", &[("error", (error).to_string())])},
            };
        self.refresh_state_slots();
        self.menu.backup_result(message);
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn after_state_restore(&mut self) {
        self.lifecycle_emit(crate::debug_lifecycle::Family::State,"timeline-restored",None,serde_json::json!({"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"scope":"Existing frontend restore boundary before collector discontinuity freeze"}));
        if let Some(buffer)=self.recent.as_mut(){buffer.freeze(&self.machine,"state-restored");}
        if let Some(trace)=self.code_trace.as_mut(){trace.freeze("state-restored");}
        if let Some(trace)=self.gameplay_trace.as_mut(){trace.freeze("state-restored");}
        if self.audio_diagnostic{if let Some(capture)=self.streams.lock().unwrap().diagnostic.as_mut(){capture.freeze("state-restored");}self.audio_diagnostic=false;}
        if self.events_enabled(){self.ui_event("timeline-restored",serde_json::json!({"scope":"trace prefix ended; report may show a later restored state"}));self.ui_trace.freeze("state-restored");}
        self.interrupt_probe();
        self.frame_control.stop("state-restored");
        self.profile.presentation_restored(self.machine.bus.frames);
        // Discard consumer resamplers and queues from the old timeline.
        drop(self.stream.take());
        self.ff_latched = false;
        {
            let mut st = self.streams.lock().unwrap();
            st.dead = false;
            st.taps_unavailable = !self.machine.bus.tap_channels;
            if let Some(profile) = st.profile.as_mut() {
                profile.restored_frame(self.machine.bus.frames);
            }
        }
        self.set_fast_forward(false, 1);
        self.streams.lock().unwrap().muted=self.paused;
        self.stream = start_audio(self.streams.clone());
        self.audio_retry = if self.stream.is_none() {
            Some(Instant::now())
        } else {
            None
        };
        self.game_mouse.cancel();
        self.mouse_queue.clear();
        self.machine.bus.keys = 0x3ff;
        self.sync_view();
        self.rebuild_temporal();
        let frame = restored_native_frame(&self.machine.bus, &mut self.padded_frame);
        self.temporal.push(frame, &self.lut);
        // Native preview already includes the saved UI. Rebuild separated layers
        // only after a completed frame has rendered them for the current view.
        self.ui_pixels.clear();
        self.panel_pixels.clear();
        self.fps.reset(Instant::now());
        self.profile.presentation_boundary();
    }

    fn undo_state_operation(&mut self) {
        self.lifecycle_emit(crate::debug_lifecycle::Family::State,"requested",None,serde_json::json!({"operation":"undo"}));
        if self.recording_active() && self.state_slots.undo_restores_machine() {
            let message = if self.capture.is_some() {
                if self.stop_input_recording("Undo load requested; retry after recording stopped", true) {
                    "Recording saved. Choose Undo load again."
                } else { "Recording failed; see Debug for details. Undo load was refused." }
            } else { "Stop input recording before undoing a load." };
            self.lifecycle_emit(crate::debug_lifecycle::Family::State,"not-applied",None,serde_json::json!({"operation":"undo","outcome":"refused","reason":"Recording guard; explicit retry required"}));
            self.menu.state_result(message.into());
            self.menu_dirty = true;
            return;
        }
        let message = match self.state_slots.undo(&mut self.machine) {
            Ok(crate::save_state::UndoEffect::Save(slot)) => {self.lifecycle_emit(crate::debug_lifecycle::Family::State,"completed",None,serde_json::json!({"operation":"undo-save","slot":slot,"outcome":"completed"}));self.menu_text("Undid save in slot {slot}.", &[("slot", (slot).to_string())])},
            Ok(crate::save_state::UndoEffect::Load(slot)) => {
                self.lifecycle_emit(crate::debug_lifecycle::Family::State,"completed",None,serde_json::json!({"operation":"undo-load","slot":slot,"outcome":"completed","scope":"Restored state boundary before collectors freeze"}));
                self.after_state_restore();
                self.menu_text("Undid load from slot {slot}.", &[("slot", (slot).to_string())])
            }
            Err(error) => {self.lifecycle_emit(crate::debug_lifecycle::Family::State,"refused",None,serde_json::json!({"operation":"undo","outcome":"refused","error":error.chars().take(1024).collect::<String>()}));self.menu_text("Cannot undo: {error}", &[("error", (error).to_string())])},
        };
        self.refresh_state_slots();
        self.menu.state_result(message);
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn state_operation(&mut self, saving: bool, slot: u8, overwrite: bool) {
        let operation=if saving{"save"}else{"load"};let before=(self.machine.bus.frames,self.machine.bus.clock);
        self.lifecycle_emit(crate::debug_lifecycle::Family::State,"requested",None,serde_json::json!({"operation":operation,"slot":slot,"overwrite":overwrite}));
        if saving && self.last_cheat_once.is_some(){self.menu.state_result("Saving after a cheat one-shot needs provenance support; restart the isolated copy first.".into());self.menu_dirty=true;return;}
        if !saving && self.recording_active() {
            let message = if self.capture.is_some() {
                if self.stop_input_recording("Load requested; retry after recording stopped", true) {
                    "Recording saved. Choose Load again."
                } else { "Recording failed; see Debug for details. Load was refused." }
            } else { "Stop input recording before loading a state." };
            self.lifecycle_emit(crate::debug_lifecycle::Family::State,"not-applied",None,serde_json::json!({"operation":operation,"slot":slot,"outcome":"refused","reason":"Recording guard; explicit retry required"}));
            self.menu.state_result(message.into());
            self.menu_dirty = true;
            return;
        }
        let result = if saving {
            let preview = self.state_preview();
            self.state_slots
                .save(&self.machine, slot, &preview, overwrite)
        } else {
            self.state_slots.load(&mut self.machine, slot)
        };
        let message = match result {
            Ok(()) => {
                self.lifecycle_emit(crate::debug_lifecycle::Family::State,"completed",None,serde_json::json!({"operation":operation,"slot":slot,"outcome":"completed","before":{"frame":before.0,"clock":before.1},"after":{"frame":self.machine.bus.frames,"clock":self.machine.bus.clock}}));
                if !saving {
                    self.after_state_restore();
                }
                if saving {self.menu_text("Slot {slot} saved.", &[("slot", (slot).to_string())])} else {self.menu_text("Slot {slot} loaded.", &[("slot", (slot).to_string())])}
            }
            Err(error) => {self.lifecycle_emit(crate::debug_lifecycle::Family::State,"refused",None,serde_json::json!({"operation":operation,"slot":slot,"outcome":"refused","error":error.chars().take(1024).collect::<String>()}));if saving{self.menu_text("Cannot save slot {slot}: {error}", &[("slot", (slot).to_string()), ("error", (error).to_string())])}else{self.menu_text("Cannot load slot {slot}: {error}", &[("slot", (slot).to_string()), ("error", (error).to_string())])}},
        };
        self.refresh_state_slots();
        self.menu.state_result(message);
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    /// Developer workload cycle; normal interactive pacing stays in about_to_wait.
    fn benchmark_tick(&mut self, event_loop: &ActiveEventLoop) {
        if self.presenter.is_none() {
            self.error = Some("GUI benchmark requires a GPU presenter; inspect adapter/surface errors".into());
            event_loop.exit(); return;
        }
        let measuring = self.benchmark.as_ref().unwrap().measuring();
        self.profile.set_measuring(measuring);
        let start = Instant::now();
        if let Err(error) = self.benchmark.as_ref().unwrap().apply_input(&mut self.machine) {
            self.error = Some(error); event_loop.exit(); return;
        }
        let tick_start = self.profile.start();
        if !self.run_one_frame() { event_loop.exit(); return; }
        let compose_start = self.profile.start();
        self.temporal.compose(&self.lut, &mut self.rgba);
        self.profile.finish(Phase::TemporalCompose, compose_start);
        let present_start = self.profile.start();
        self.present_rgba();
        self.profile.finish(Phase::Present, present_start);
        self.profile.finish(Phase::Tick, tick_start);
        if self.error.is_some() || self.presenter.is_none() {
            self.error.get_or_insert_with(|| "GUI benchmark lost GPU presentation".into());
            event_loop.exit(); return;
        }
        match self.benchmark.as_mut().unwrap().finish_frame(&self.machine, start) {
            Ok(true) => event_loop.exit(),
            Ok(false) => {},
            Err(error) => { self.error = Some(error); event_loop.exit(); }
        }
    }

    /// Pause/priming boundaries reset legacy alarms, but the optional session
    /// profile retains independent totals labelled with the current context.
    fn reset_audio_counters(&self) {
        let mut st = self.streams.lock().unwrap();
        st.drops = 0;
        st.holds = 0;
        st.clipped = 0;
        if let Some(profile) = st.profile.as_mut() {
            profile.set_context(crate::audio_profile::Context::of(self.paused, self.primed, self.ff_active));
            profile.publish_frame(self.machine.bus.frames);
        }
    }

    /// Change fast-forward activation or audio speed. Flush stale audio on
    /// transitions and re-prime when returning to normal play.
    fn set_fast_forward(&mut self, on: bool, speed: u32) {
        self.profile.presentation_boundary();
        self.ff_active = on;
        self.ff_clock.reset();
        let percent=if on {input_config::fast_forward::selected_percent(speed)}else{100};
        self.ff_audio_active = self.av.ff_audio.clone();
        {
            let mut st = self.streams.lock().unwrap();
            st.discard_pending();
            st.muted = on && self.av.ff_audio == "silent";
            st.playback_speed = if on && self.av.ff_audio == "fast" {
                percent as f64/100.0
            } else {
                1.0
            };
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
            st.drops = 0;
            st.holds = 0;
            st.clipped = 0;
            if let Some(profile) = st.profile.as_mut() {
                profile.set_context(crate::audio_profile::Context::of(self.paused, false, on));
                profile.publish_frame(self.machine.bus.frames);
            }
        }
        self.primed = false;
        self.audio_watch = Instant::now();
        self.audio_stalls = 0;
        self.next_frame = Instant::now();
    }

    /// One emulated frame plus its audio handoff to the rings. Counts native
    /// vs fallback dispatch, advances the screen's temporal response, surfaces
    /// the realtime / audio-defect alarms, and flushes backup media.
    fn run_one_frame(&mut self) -> bool {
        let _context=self.lifecycle.context(crate::debug_lifecycle::Family::Step).map(|c|self.events.clock.context(c));
        // Apply a pending zoom change before the frame is emulated, so the
        // core renders (and we read back) the size the zoom needs.
        self.sync_view();
        if self.benchmark.is_none() {
            let keys = if self.paused { self.frame_control.keys } else {
                let keys=self.game_mouse.keys(&mut self.machine.bus,self.game_keys);
                gba_core::ffta_battle_mouse::keys(&mut self.machine.bus,keys)
            };
            self.machine.bus.keys = keys;
            if self.events_enabled(){self.ui_event("guest-input-latch",serde_json::json!({"keys":keys,"source":if self.paused{"controlled-step"}else{"mapped-host-and-mouse"},"host_menu_paused":self.paused,"recording":self.capture.is_some()}));}
            let battle_event = self.machine.bus.ffta_battle_mouse.take_event();
            let language_event = self.game_mouse.take_selection_event();
            if let Some(index) = battle_event {
                if let Some((file, _)) = &mut self.record {
                    use std::io::Write;
                    let _ = writeln!(file, "{} mouse:{index}", self.machine.bus.frames);
                }
            }
            if let Some(index) = language_event {
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
            if let Some(capture) = &mut self.capture {
                let events: Vec<_> = battle_event.into_iter().chain(language_event.map(u16::from)).collect();
                if let Err(error) = capture.before_frame(&self.machine, &events) {
                    self.stop_input_recording(&error, false);
                    self.paused = true;
                    return true;
                }
            }
        }
        let emu_t0 = Instant::now();
        self.fps.configure(&self.av.video_fps, emu_t0);
        let frames_before = self.machine.bus.frames;
        let core_start = self.profile.start();
        let ppu_capture = gba_core::ppu_profile::Capture::start_detailed(
            self.profile.enabled(), self.profile.detailed_ppu(),
        );
        let recent_on=self.recent.as_ref().is_some_and(|b|b.enabled());
        if recent_on{let scene=self.recent_scene();self.recent.as_mut().unwrap().scene(&self.machine,scene);}
        let mut recent_frame=if recent_on{Some(crate::debug_recent::Frame::start(self.recent.as_mut().unwrap(),&self.machine,self.native.is_some()))}else{None};
        let code_observed=self.code_trace.as_ref().is_some_and(|t|t.enabled());
        let gameplay_observed=self.gameplay_trace.as_ref().is_some_and(|t|t.enabled());
        match &self.native {
            Some((_lib, table)) => {
                let mptr = &mut self.machine as *mut Machine as *mut std::ffi::c_void;
                let (n, f) = if code_observed || gameplay_observed || recent_on {
                    let code=&mut self.code_trace;let gameplay=&mut self.gameplay_trace;
                    super::run_frame_native_traced(&mut self.machine,table,mptr,5_000_000,|m,k|{
                        if let Some(t)=code.as_mut(){t.native(m,k);}if let Some(t)=gameplay.as_mut(){t.native(m,k);}
                        if let Some(frame)=recent_frame.as_mut(){frame.native(k);}
                    },|_,_,_|{})
                }else{run_frame_native(&mut self.machine, table, mptr, 5_000_000)};
                self.native_run += n;
                self.fallback_run += f;
            }
            None => if gameplay_observed {
                let code=&mut self.code_trace;let gameplay=self.gameplay_trace.as_mut().unwrap();
                super::run_frame_interpreter_observed(&mut self.machine,5_000_000,|m|{if let Some(t)=code.as_mut(){t.interpreter(m);}gameplay.observe(m,"interpreter");});
            }else if code_observed{self.code_trace.as_mut().unwrap().run_interpreter(&mut self.machine,5_000_000)}else{self.machine.run_frame(5_000_000)},
        }
        let ppu_timings = ppu_capture.finish();
        self.profile.finish(Phase::Core, core_start);
        // A guard hit is an incomplete frame, not a new temporal-history entry.
        // Stop cleanly instead of presenting/counting partially advanced state.
        if let Some(frame)=recent_frame.take(){frame.finish(self.recent.as_mut().unwrap(),&self.machine,self.machine.bus.frames!=frames_before);}
        if self.machine.bus.frames == frames_before {
            self.error = Some("emulation step guard exceeded before a complete frame".into());
            return false;
        }
        if code_observed{self.code_trace.as_mut().unwrap().completed_frame();}
        if gameplay_observed{self.gameplay_trace.as_mut().unwrap().completed_frame(&self.machine);}
        if self.events_enabled(){self.ui_event("guest-frame",serde_json::json!({"keys":self.machine.bus.keys,"from_frame":frames_before,"completed_frame":self.machine.bus.frames,"recording":self.capture.is_some()}));}
        self.profile.record_ppu(ppu_timings);
        self.fps.emulated(self.machine.bus.frames.saturating_sub(frames_before));
        gba_core::ffta_sprites::trace_frame(&self.machine.bus);
        let ui_start = self.profile.start();
        self.refresh_ui_layers();
        if recent_on{let scene=self.recent_scene();self.recent.as_mut().unwrap().scene(&self.machine,scene);}
        self.profile.finish(Phase::UiPrepare, ui_start);
        // Advance the screen's temporal response once per EMULATED frame —
        // never per presented frame, or flicker-transparency titles lock onto
        // one phase (the classic parity bug).
        let temporal_start = self.profile.start();
        let wf = display_frame(&self.machine.bus, &mut self.padded_frame);
        self.temporal.push(wf, &self.lut);
        self.profile.finish(Phase::TemporalPush, temporal_start);

        // Realtime alarm: the frame budget is 16.7 ms. If smoothed emulation
        // cost approaches it, the product promise is broken — say so once.
        let dt_ms = emu_t0.elapsed().as_secs_f64() * 1e3;
        self.emu_ema_ms = if self.frames_run == 0 {
            dt_ms
        } else {
            self.emu_ema_ms * 0.95 + dt_ms * 0.05
        };
        self.frames_run += 1;
        if self.benchmark.is_none() && !self.slow_warned && self.frames_run > 120 && self.emu_ema_ms > 15.0 {
            self.slow_warned = true;
            eprintln!(
                "DEGRADED: emulation averaging {:.1} ms/frame (budget 16.7 ms) — below native speed",
                self.emu_ema_ms
            );
        }

        // Live perf readout in the title, once a second. Developer tooling —
        // hidden from the release out-of-box experience unless requested.
        if self.show_stats && self.frames_run % 60 == 0 {
            if let Some(w) = self.window.as_ref() {
                w.set_title(&format!(
                    "{} · {:.2} ms/frame ({:.1}x headroom) · native {} blocks · fallback {} steps",
                    self.recording_title(),
                    self.emu_ema_ms,
                    16.7 / self.emu_ema_ms.max(0.01),
                    self.native_run,
                    self.fallback_run,
                ));
            }
            self.native_run = 0;
            self.fallback_run = 0;
        }

        // Audio defect surfacing: ring drops and underrun holds are silent
        // corruption — say so, with numbers, per the product bar.
        if self.benchmark.is_none() && self.frames_run % 300 == 0 {
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

        let autosave_start = (self.benchmark.is_none() && self.frames_run % 300 == 0).then(|| self.profile.start()).flatten();
        // Flush backup media periodically when it changed, so an external
        // teardown (launcher kills us) can't lose more than ~5 s of progress.
        if self.benchmark.is_none() && self.frames_run % 300 == 0 {
            if let Some(data) = self.machine.bus.save_data() {
                let h = fnv64(data);
                if self.sav_seen != Some(h) && write_sav(&self.sav_path, data).is_ok() {
                    self.sav_seen = Some(h);
                }
            }
        }

        self.profile.finish(Phase::Autosave, autosave_start);
        let audio_start = self.profile.start();
        self.handoff_audio();
        self.profile.finish(Phase::AudioHandoff, audio_start);
        true
    }

    /// Hand this frame's audio taps to the consumer rings, refresh the routing
    /// snapshot, and count anything dropped. With no consumer, the taps are
    /// discarded (not a defect — reported when the device died) so the rings
    /// can't overflow into permanent DEGRADED spam.
    fn handoff_audio(&mut self) {
        if self.audio_diagnostic {
            let decision=if self.paused{"discard-paused"}else if self.stream.is_none(){"discard-no-stream"}else if self.ff_active&&self.av.ff_audio=="silent"{"discard-fast-silent"}else if self.ff_active&&self.av.ff_audio=="normal"{"normal-queue"}else{"rings"};
            if let Some(capture)=self.streams.lock().unwrap().diagnostic.as_mut(){capture.generated(&self.machine,decision);}
        }
        let bus = &mut self.machine.bus;
        if self.paused || self.stream.is_none() || (self.ff_active && self.av.ff_audio == "silent") {
            // Silent fast-forward still advances guest frames. Only diagnostics
            // need to publish them; keep the default discard path lock-free.
            if self.benchmark.is_none() && self.profile.enabled() {
                if let Some(profile) = self.streams.lock().unwrap().profile.as_mut() {
                    profile.publish_frame(bus.frames);
                }
            }
            bus.audio_buf.clear();
            bus.psg_tap.clear();
            bus.hle_tap.clear();
            bus.fifo_tap[0].clear();
            bus.fifo_tap[1].clear();
            return;
        }
        if self.ff_active && self.av.ff_audio == "normal" {
            let mut st = self.streams.lock().unwrap();
            if let Some(profile) = st.profile.as_mut() { profile.publish_frame(bus.frames); }
            if let Some(queue) = st.normal_audio.as_mut() {
                if let Err(error) = queue.push(&bus.audio_buf) {
                    eprintln!(
                        "audio: invalid normal-speed PCM ({error}); muting fast-forward audio"
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
        if let Some(profile) = st.profile.as_mut() { profile.publish_frame(bus.frames); }
        let drops_before = st.profile.as_ref().map(|_| st.drops);
        if st.playback_speed > 1. {
            st.fast_produced = st.fast_produced.wrapping_add((bus.audio_buf.len() / 2) as u64);
        }
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
            // Keep the producer's allocation for the next frame, preserving
            // the same event order and overflow/drop behavior.
            for e in bus.fifo_tap[f].drain(..) {
                if st.fifo[f].len() < audio_capacity / 2 {
                    st.fifo[f].push_back(e);
                } else {
                    st.drops += 1;
                }
            }
        }
        if let Some(before) = drops_before {
            let dropped = st.drops.saturating_sub(before);
            if let Some(profile) = st.profile.as_mut() { profile.dropped(dropped); }
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
                self.profile.presentation_boundary();
                self.streams.lock().unwrap().dead = false;
                // Recreate even Normal's backlog from the current timeline.
                self.set_fast_forward(self.ff_active, self.ff_speed);
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
                        self.reset_audio_counters();
                    }
                }
            }
        }
    }
}

impl PlayApp {
    fn window_event_observed(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if matches!(&event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }) {
            self.profile.presentation_boundary();
        }
        if self.benchmark.is_some() {
            if matches!(event, WindowEvent::CloseRequested) { event_loop.exit(); }
            return; // Physical input must not alter a deterministic benchmark.
        }
        self.menu_dirty = true;
        if self.modal_input.blocked() && matches!(&event,WindowEvent::KeyboardInput{..}|WindowEvent::MouseInput{..}|WindowEvent::MouseWheel{..}|WindowEvent::CursorMoved{..}) {
            self.held.clear();self.pending_key=None;self.pending_pad=None;self.mouse_queue.clear();
            if self.events_enabled(){self.ui_event("modal-input-consumed",serde_json::json!({"guest_input":false,"reason":"Returning from explicit host dialog"}));}
            return;
        }
        if matches!(&event,
            WindowEvent::KeyboardInput{event:KeyEvent{state:ElementState::Pressed,repeat:false,..},is_synthetic:false,..}
            | WindowEvent::MouseInput{state:ElementState::Pressed,..}
            | WindowEvent::MouseWheel{..}) {
            self.menu.note_input(input_config::Device::Keyboard);
        }
        match event {
            WindowEvent::Focused(true)=>self.modal_input.focus_return(Instant::now()),
            WindowEvent::CloseRequested => event_loop.exit(),
            // Lost focus: a key held during alt-tab might never deliver its
            // release, so drop the held set defensively (no stuck inputs).
            WindowEvent::Focused(false) => {self.held.clear();self.game_keys=0x3ff;self.game_mouse.cancel();if !self.event_console_panel.as_ref().is_some_and(|p|p.focus_handoff()) && !self.command_console_panel.as_ref().is_some_and(|p|p.focus_handoff()){self.machine.bus.ffta_battle_mouse.cancel();}},
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat,
                        text,
                        ..
                    },
                is_synthetic,
                ..
            } => {
                if self.events_enabled(){self.ui_event("keyboard",serde_json::json!({"key":format!("{code:?}"),"pressed":state==ElementState::Pressed,"repeat":repeat,"paused":self.paused,"synthetic":is_synthetic}));}
                if is_synthetic && state==ElementState::Pressed && self.command_console_panel.as_ref().is_some_and(|p|p.hide_handoff()) {
                    self.held.remove(&code);self.pending_key=None;self.host_consumed("Synthetic focus-return key after command console");return;
                }
                let menu_key_member=self.icfg.menu_key.split('+').any(|token|winit_key(token)==Some(code));
                if self.paused && self.menu.host_panel_active() && is_synthetic {self.host_consumed("Synthetic keyboard event while host panel owns input");self.held.remove(&code);return;}
                if self.paused && self.menu.host_panel_active() {
                    if state==ElementState::Released {self.held.remove(&code);return;}
                    if self.menu.content_text_focus() {
                        self.host_consumed("Host text editor owns keyboard input");
                        match code {
                            KeyCode::Enter|KeyCode::NumpadEnter if !repeat=>self.menu.content_finish_search(false),
                            KeyCode::Escape if !repeat=>self.menu.content_finish_search(true),
                            KeyCode::Backspace=>self.menu.content_backspace(),
                            KeyCode::Tab if !repeat=>self.menu.content_tab(false),
                            KeyCode::ArrowUp|KeyCode::ArrowDown|KeyCode::ArrowLeft|KeyCode::ArrowRight=>{let ev=match code{KeyCode::ArrowUp=>menu::MenuInput::Up,KeyCode::ArrowDown=>menu::MenuInput::Down,KeyCode::ArrowLeft=>menu::MenuInput::Left,_=>menu::MenuInput::Right};self.menu.handle(ev,&mut self.av);},
                            _=>{if let Some(text)=text.as_ref(){self.menu.content_text(text.as_str());}},
                        }
                        self.held.remove(&code);self.pending_key=None;return;
                    }
                    if repeat {self.host_consumed("Host panel ignores repeated navigation");return;}
                    if code==KeyCode::Tab && !menu_key_member {self.host_consumed("Host panel tab navigation");if self.held.contains(&KeyCode::ControlLeft)||self.held.contains(&KeyCode::ControlRight){self.menu.focus_next();}else{self.menu.content_tab(false);}return;}
                    if code==KeyCode::Escape {self.host_consumed("Host panel cancel");let action=self.menu.handle(menu::MenuInput::Cancel,&mut self.av);self.inline_action(action);if action==Some(menu::MenuAction::ToolRequest(menu::ToolRequest::Back)){self.workbench.remember(&self.menu);self.workbench.cancel_view();}self.content_picker.leave_view(&self.menu);self.observe_menu("keyboard-cancel");if let Some(action)=action{self.host_action_returned(action);}self.held.clear();return;}
                    if code==KeyCode::KeyF && (self.held.contains(&KeyCode::ControlLeft)||self.held.contains(&KeyCode::ControlRight)){self.host_consumed("Host panel search shortcut");self.menu.content_start_search();self.held.clear();return;}
                }
                if self.paused && !repeat && state==ElementState::Pressed && code==KeyCode::F1 && !menu_key_member && self.menu.capturing().is_none() && !self.menu.content_text_focus(){self.menu.information(&self.av,&self.icfg);self.menu_dirty=true;return;}
                if self.paused && repeat && code==KeyCode::Escape && self.menu.capturing().is_none(){self.host_consumed("Escape repeat stays consumed by menu");return;}
                // Escape cancels the current menu context before the recovery
                // shortcut can resume the game. Binding capture stays exclusive.
                if self.paused && !repeat && state==ElementState::Pressed && code==KeyCode::Escape && self.menu.capturing().is_none() && self.menu.escape_goes_back() {
                    self.host_consumed("Escape cancels menu context");
                    let action=self.menu.handle(menu::MenuInput::Cancel,&mut self.av);
                    self.inline_action(action);self.observe_menu("keyboard-cancel");
                    self.held.remove(&code);self.pending_key=None;self.menu_dirty=true;return;
                }
                if self.av.debug && !is_synthetic && !repeat && state==ElementState::Pressed && self.console_key(code)
                    && self.menu.capturing().is_none() && !self.menu.content_text_focus() {
                    self.held.remove(&code);self.pending_key=None;self.toggle_event_console(event_loop);return;
                }
                match state {
                ElementState::Pressed => {
                    if self.paused && !repeat && code==KeyCode::Tab && !menu_key_member && self.menu.capturing().is_none() && !self.menu.content_text_focus() && (!self.menu.host_panel_active() || self.held.contains(&KeyCode::ControlLeft)||self.held.contains(&KeyCode::ControlRight)) {if self.held.contains(&KeyCode::ShiftLeft)||self.held.contains(&KeyCode::ShiftRight){self.menu.atlas_reference_tab();}else{self.menu.focus_next();}self.menu_dirty=true;return;}
                    // Consume debugger shortcuts before text input, bindings, Fast or menu capture.
                    if self.debug_session && self.av.debug && matches!(code,KeyCode::F6|KeyCode::F7|KeyCode::F8) {
                        if self.events_enabled(){self.ui_event("shortcut-consumed",serde_json::json!({"key":format!("{code:?}"),"repeat":repeat,"guest_input":false}));}
                        if !repeat {
                            let action=match code {
                                KeyCode::F6=>if self.paused {crate::debug_control::Action::Resume}else{crate::debug_control::Action::Pause},
                                KeyCode::F7=>crate::debug_control::Action::Step{frames:1,keys:0x3ff},
                                _=>crate::debug_control::Action::Step{frames:10,keys:0x3ff},
                            };
                            if let Err(error)=self.debug_action(&action){self.menu.diagnosis_result(error);self.menu_dirty=true;}
                        }
                        return;
                    }
                    if !self.paused && self.game_mouse.is_name_input(&self.machine.bus) {
                        let consumed=match code {
                            KeyCode::Backspace=>self.game_mouse.name_command(&self.machine.bus,false),
                            KeyCode::Enter|KeyCode::NumpadEnter=>self.game_mouse.name_command(&self.machine.bus,true),
                            _=>text.as_ref().is_some_and(|t|self.game_mouse.type_text(&self.machine.bus,t.as_str())),
                        };
                        if consumed {if self.events_enabled(){self.ui_event("text-input-consumed",serde_json::json!({"key":format!("{code:?}"),"guest_button":false}));}self.held.remove(&code);return;}
                    }
                    self.held.insert(code);
                    let menu_combo_down = input_config::combo_state(&self.icfg.menu_key, |token| {
                        winit_key(token).is_some_and(|key| self.held.contains(&key))
                    }).0;
                    if !self.paused && !menu_combo_down && !repeat && winit_key(&self.icfg.ff_toggle_key) == Some(code)
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
                    if code == KeyCode::F11 && !repeat {
                        if let Some(w) = self.window.as_ref() {
                            let next = !self.fullscreen.desired(w.fullscreen().is_some());
                            self.set_fullscreen(next);
                        }
                    }
                    if !self.paused && !menu_combo_down {
                        // Zoom is gameplay-only; prevent it changing behind menus.
                        match code {
                            _ if key_name(code).is_some_and(|n| n == self.icfg.zoom_in_key) => {
                                self.zoom_by(-1)
                            }
                            _ if key_name(code).is_some_and(|n| n == self.icfg.zoom_out_key) => {
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
            },
            WindowEvent::Ime(winit::event::Ime::Preedit(text,_)) if self.paused && self.menu.content_text_focus()=>{self.menu.content_preedit(&text);},
            WindowEvent::Ime(winit::event::Ime::Commit(text)) if self.paused && self.menu.content_text_focus()=>{self.menu.content_preedit("");self.menu.content_text(&text);},
            WindowEvent::CursorMoved { position, .. } => {
                let p = (position.x, position.y);
                self.mouse_pos = Some(p);
                if self.paused {
                    self.queue_mouse(MouseEv::Move(p));
                } else {
                    self.game_mouse_aim(p,false);
                }
            }
            WindowEvent::CursorLeft { .. } => {self.mouse_pos=None;self.game_mouse.cancel();self.machine.bus.ffta_battle_mouse.cancel();},
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if let (true, Some(p)) = (self.paused, self.mouse_pos) {
                    match button {
                        MouseButton::Left => self.queue_mouse(MouseEv::Click(p, true)),
                        MouseButton::Right => self.queue_mouse(MouseEv::Click(p, false)),
                        _ => {}
                    }
                }else if button==MouseButton::Right {
                    self.game_mouse.cancel();
                    gba_core::ffta_battle_mouse::back(&mut self.machine.bus);
                }else if button==MouseButton::Left {
                    let handled=self.mouse_pos.is_some_and(|p|self.game_mouse_aim(p,true));
                    if !handled {
                        self.game_mouse.cancel();
                        gba_core::ffta_battle_mouse::press(&mut self.machine.bus,0x3fe);
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
                        self.queue_mouse(MouseEv::Wheel(true));
                    } else if dy < 0.0 {
                        self.queue_mouse(MouseEv::Wheel(false));
                    }
                } else {
                    let dy=match delta {MouseScrollDelta::LineDelta(_,y)=>y,MouseScrollDelta::PixelDelta(p)=>(p.y/30.)as f32};
                    if dy!=0. {gba_core::ffta_shop_mouse::scroll(&mut self.machine.bus,dy>0.);}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait_observed(&mut self, event_loop: &ActiveEventLoop) {
        self.poll_replay();
        if self.paused&&!self.content_picker.initialized(){let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);if let Err(e)=self.content_picker.initialize(&mut self.menu,rom){self.menu.diagnosis_result(e);}}
        self.menu_dirty |= self.content_picker.poll(&mut self.menu);
        if self.paused{self.menu_dirty|=self.content_picker.persist_tick(&mut self.menu);}
        let workbench_was_busy=self.workbench.busy();
        if let Some(path)=self.workbench.poll(&mut self.menu){
            let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);
            if let Err(error)=self.content_picker.adopt(menu::ContentSlot::Replay,&path,&rom,&mut self.menu){self.workbench.last=format!("Profile prepared; selection refused: {error}");self.menu.tool_feedback(self.workbench.last.clone(),String::new(),false);}
        }
        self.menu_dirty |= workbench_was_busy!=self.workbench.busy();
        if workbench_was_busy&&!self.workbench.busy(){
            use crate::debug_lifecycle::Family as F;
            let family=if self.lifecycle.context(F::Script).is_some(){F::Script}else{F::Workbench};
            let context=self.lifecycle.finish(family);
            let observed=self.workbench.observation.clone().unwrap_or_else(||serde_json::json!({"outcome":"unknown","coverage_gap":"Worker ended without terminal metadata"}));
            let error=if observed["outcome"].as_str().is_some_and(|v|["refused","timeout","error"].contains(&v)){Some(observed.pointer("/result/error").and_then(|v|v.as_str()).unwrap_or("Host worker failed; inspect selected result").to_owned())}else{None};
            self.lifecycle_emit(family,"finished",context.clone(),observed);
            if let Some(error)=error{let _context=context.map(|c|self.events.clock.context(c));self.recent_error(&error);}
        }
        if self.benchmark.is_some() && self.window.is_some() {
            self.benchmark_tick(event_loop); return;
        }
        if self.window.is_none() {
            return;
        }
        self.poll_control();
        if let Some(trace)=self.code_trace.as_mut(){trace.tick();}
        if let Some(trace)=self.gameplay_trace.as_mut(){trace.tick();}
        if self.audio_diagnostic {let mut st=self.streams.lock().unwrap();if let Some(capture)=st.diagnostic.as_mut(){capture.tick();self.audio_diagnostic=capture.enabled();}}

        // Pointer policy, reconciled live: visible in windowed mode (users
        // need it to drag/resize/close the window) and while the pause menu is
        // up; hidden only when the game owns a fullscreen screen. Call through
        // only on a change.
        let fullscreen = self
            .window
            .as_ref()
            .is_some_and(|w| w.fullscreen().is_some());
        if let Some(actual) = self.fullscreen.observe(fullscreen) {
            if actual != self.av.video_fullscreen {
                self.av.video_fullscreen = actual;
                self.menu_dirty = true;
                if let Err(error) = self.av.save() {
                    eprintln!("menu: could not save fullscreen state ({error})");
                }
            }
        }
        let want_cursor = self.paused || !fullscreen || self.machine.bus.ffta_battle_mouse.enabled || self.game_mouse.scene(&self.machine.bus).is_some();
        if want_cursor != self.cursor_visible {
            if let Some(w) = self.window.as_ref() {
                w.set_cursor_visible(want_cursor);
            }
            self.cursor_visible = want_cursor;
        }

        if let Some(window)=self.window.as_ref(){window.set_ime_allowed(self.paused && self.menu.content_text_focus());}
        if self.paused && self.menu.content_text_focus(){self.held.clear();self.pending_key=None;}
        if self.modal_input.blocked(){
            self.held.clear();self.pending_key=None;self.pending_pad=None;self.mouse_queue.clear();
            self.menu_key_latch=Default::default();self.menu_pad_latch=Default::default();self.menu_shortcut_held=false;
            let (_,nav)=self.gather_input();let neutral=!(nav.up||nav.down||nav.left||nav.right||nav.confirm||nav.cancel||nav.open);
            self.nav_prev=nav;self.modal_input.observe(Instant::now(),neutral);return;
        }
        let (mut keys, nav) = self.gather_input();

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
        let selector_open=self.paused && self.menu.host_panel_active();
        if selector_open && nav.open && !self.nav_prev.open {
            let _context=self.events_enabled().then(||self.events.clock.context(self.events.clock.request("menu-cancel")));
            self.host_consumed("Mapped open control cancels the active host panel");
            let action=self.menu.handle(menu::MenuInput::Cancel,&mut self.av);self.inline_action(action);
            if action==Some(menu::MenuAction::ContentCancel){self.content_picker.leave_view(&self.menu);}
            if action==Some(menu::MenuAction::ToolRequest(menu::ToolRequest::Back)){self.workbench.remember(&self.menu);self.workbench.cancel_view();}
            self.nav_prev=nav;self.menu_dirty=true;
            self.observe_menu("mapped-panel-cancel");if let Some(action)=action{self.host_action_returned(action);}
        }
        let capturing_now = self.paused && (self.menu.capturing().is_some() || selector_open);
        let toggle_edge = self.replay_job.is_none() && !capturing_now && nav.open && !self.nav_prev.open;
        if toggle_edge {
            let _context=self.events_enabled().then(||self.events.clock.context(self.events.clock.request("menu-toggle")));
            if !self.menu_enabled {
                event_loop.exit();
                return;
            }
            self.paused = !self.paused;
            self.interrupt_probe();
            self.frame_control.stop(if self.paused {"menu-pause"}else{"running"});
            self.debug_audio_boundary();
            self.fps.reset(Instant::now());
            self.profile.presentation_boundary();
            if self.paused {
                // End the current audio timeline immediately. Keep the user's
                // Fast latch; the effective speed is resolved again on resume.
                self.game_mouse.cancel();self.machine.bus.ffta_battle_mouse.cancel();
                self.menu.reset();
                self.refresh_state_slots();
                self.menu_dirty = true;
                // Drop stale pointer events from before the menu opened.
                self.mouse_queue.clear();
            }
            self.lifecycle_emit(crate::debug_lifecycle::Family::Machine,"menu-toggle",None,serde_json::json!({"paused":self.paused,"reason":self.frame_control.reason}));
            for family in [crate::debug_lifecycle::Family::Step,crate::debug_lifecycle::Family::Search]{if self.lifecycle.context(family).is_some(){self.lifecycle_finish(family,serde_json::json!({"outcome":self.frame_control.reason,"scope":"Existing menu toggle stopped the budget"}));}}
            self.observe_menu("menu-toggle");
            self.reset_audio_counters();
            // A toggle press is fully consumed here — while paused, fall
            // through to the overlay redraw; the same press must not also be
            // fed to the menu as a Cancel.
            if !self.paused {
                self.suppress = !keys & 0x3FF;
                self.nav_prev = nav;
                return;
            }
        }

        let atlas_edges=self.atlas_shortcuts.poll([self.atlas_down[0],self.atlas_down[1],self.atlas_down[2],self.atlas_down[3]],self.paused && !self.menu_shortcut_held && self.menu.capturing().is_none() && !self.menu.content_text_focus(),self.atlas_clock.elapsed().as_millis().min(u64::MAX as u128) as u64);
        if self.paused && !self.menu_shortcut_held && self.atlas_down[4] && !self.atlas_focus_prev && self.menu.capturing().is_none() && !self.menu.content_text_focus(){self.menu.focus_next();self.menu_dirty=true;}
        self.atlas_focus_prev=self.atlas_down[4];
        if self.paused && !self.menu_shortcut_held && self.atlas_down[5] && !self.atlas_reference_prev && self.menu.capturing().is_none() && !self.menu.content_text_focus(){self.menu.atlas_reference_tab();self.menu_dirty=true;}
        self.atlas_reference_prev=self.atlas_down[5];
        if self.paused && !self.menu_shortcut_held && self.atlas_down[6] && !self.atlas_information_prev && self.menu.capturing().is_none() && !self.menu.content_text_focus(){self.menu.information(&self.av,&self.icfg);self.menu_dirty=true;}
        self.atlas_information_prev=self.atlas_down[6];
        if self.paused {
            let previous_aspect = self.av.video_aspect.clone();
            self.menu_dirty |= self.pending_key.is_some()
                || self.pending_pad.is_some()
                || !self.mouse_queue.is_empty();
            // Feed the menu the rising edges and apply any setting change live
            // (video) or to av.cfg only (restart-required audio/gamut). The
            // No guest step runs until requested; preserve its saved input latch.
            // Key capture: the Inputs page asked for a key; take the next
            // press (Escape cancels). Navigation is frozen meanwhile and for
            // the rest of this tick, so the chosen key (an arrow, Enter...)
            // can't also move the cursor.
            let binding_before=self.menu.capturing();
            let was_capturing = binding_before.is_some();
            let capture_context=(was_capturing&&self.events_enabled()).then(||self.events.clock.context(self.events.clock.request("binding-capture")));
            let captured_pad=if was_capturing&&self.events_enabled(){self.pending_pad.clone()}else{None};
            let captured = self.pending_key.take();
            let capture_key_combo = self.menu.capturing() == Some(menu::Bind::Menu);
            if capture_key_combo {
                if captured == Some(KeyCode::Escape) {
                    self.menu.end_capture();
                    self.menu_key_capture = input_config::ComboCapture::default();
                } else if let Some(combo) = self.menu_key_capture.poll("keyboard", self.held.iter().filter_map(|code| key_name(*code).map(str::to_owned)).collect()) {
                    self.icfg.menu_key = combo;
                    if let Err(error) = self.icfg.save() { eprintln!("menu: could not save input.cfg ({error})"); }
                    self.menu.end_capture();
                }
            } else { self.menu_key_capture = input_config::ComboCapture::default(); }
            if let (Some(btn), Some(code)) = (self.menu.capturing(), captured.filter(|_| !capture_key_combo)) {
                if code == KeyCode::Escape {
                    self.menu.end_capture();
                } else if let Some(name) = key_name(code) {
                    // A key already bound elsewhere swaps with this binding.
                    match btn {
                        menu::Bind::Button(b) => {
                            let idx = b.index();
                            let old = self.icfg.active_keys()[idx].clone();
                            if let Some(j) =
                                (0..10).find(|&j| j != idx && self.icfg.active_keys_mut()[j] == name)
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
                            if let Some(j) = (0..10).find(|&j| self.icfg.active_keys_mut()[j] == name) {
                                self.icfg.active_keys_mut()[j] = old;
                            } else if self.icfg.ff_toggle_key == name {
                                self.icfg.ff_toggle_key = old;
                            }
                            self.icfg.ff_key = name.to_string();
                        }
                        menu::Bind::FastForwardToggle => {
                            let old = self.icfg.ff_toggle_key.clone();
                            if let Some(j) = (0..10).find(|&j| self.icfg.active_keys_mut()[j] == name) {
                                self.icfg.active_keys_mut()[j] = old;
                            } else if self.icfg.ff_key == name {
                                self.icfg.ff_key = old;
                            }
                            self.icfg.ff_toggle_key = name.to_string();
                        }
                        menu::Bind::ZoomIn => self.icfg.zoom_in_key = name.to_string(),
                        menu::Bind::ZoomOut => self.icfg.zoom_out_key = name.to_string(),
                        menu::Bind::GamepadZoomIn
                        | menu::Bind::GamepadZoomOut
                        | menu::Bind::GamepadFastForward
                        | menu::Bind::GamepadFastForwardToggle => {}
                        menu::Bind::Gamepad(_) => {}
                        menu::Bind::Menu | menu::Bind::GamepadMenu => {}
                    }
                    if !matches!(btn, menu::Bind::Gamepad(_) | menu::Bind::GamepadMenu | menu::Bind::GamepadZoomIn | menu::Bind::GamepadZoomOut | menu::Bind::GamepadFastForward | menu::Bind::GamepadFastForwardToggle) {
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
                } else if binding == menu::Bind::GamepadZoomIn {
                    self.icfg.zoom_in_pad = name;
                } else if binding == menu::Bind::GamepadZoomOut {
                    self.icfg.zoom_out_pad = name;
                } else if binding == menu::Bind::GamepadFastForward {
                    self.icfg.ff_pad = name;
                } else if binding == menu::Bind::GamepadFastForwardToggle {
                    self.icfg.ff_toggle_pad = name;
                } else if binding == menu::Bind::GamepadMenu {
                    self.icfg.menu_pad = name;
                }
                if self.events_enabled(){self.ui_event("pad-bindings-changed",serde_json::json!({"input_config":self.icfg.serialize()}));}
                if let Err(error) = self.icfg.save() {
                    eprintln!("menu: could not save input.cfg ({error})");
                }
                self.menu.end_capture();
            }
            if was_capturing&&self.menu.capturing().is_none()&&self.events_enabled(){self.ui_event("binding-capture-result",serde_json::json!({"binding":format!("{binding_before:?}"),"keyboard_token":captured.map(|k|format!("{k:?}")),"pad_token":captured_pad,"outcome":if captured==Some(KeyCode::Escape){"cancelled"}else{"finished"},"input_config":self.icfg.serialize()}));}
            self.observe_menu("binding-capture");
            drop(capture_context);
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
            let mut acts: Vec<(menu::MenuAction,Option<crate::debug_events::Correlation>)> = Vec::new();
            for edge in atlas_edges {if let Some(action)=self.menu.global_shortcut(edge,&mut self.av){acts.push((action,None));self.menu_dirty=true;}}
            for (fired, ev) in events {
                if !fired {continue;}
                if was_capturing {
                    if self.events_enabled(){self.ui_event("menu-consumed-by-remapping",serde_json::json!({"input":format!("{ev:?}"),"menu":self.ui_focus()}));}
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
                    if self.events_enabled(){self.ui_event("menu-debounce-refused",serde_json::json!({"input":format!("{ev:?}"),"menu":self.ui_focus()}));}
                    continue;
                }
                let correlation=self.events_enabled().then(||self.events.clock.request("menu-navigation"));
                let _context=correlation.clone().map(|c|self.events.clock.context(c));
                let before=if self.events_enabled(){self.ui_event("host-input-request",serde_json::json!({"input":format!("{ev:?}")}));Some(self.ui_focus())}else{None};
                let action=self.menu.handle(ev,&mut self.av);
                self.observe_menu("mapped-navigation");
                if let Some(before)=before{self.ui_event("host-menu-dispatch",serde_json::json!({"input":format!("{ev:?}"),"before":before,"after":self.ui_focus(),"action":action.map(|a|format!("{a:?}")),"guest_step":false}));}
                if let Some(a)=action {acts.push((a,correlation));}
            }
            // Mouse: hover highlights, left/right click act, wheel moves.
            let mq = std::mem::take(&mut self.mouse_queue);
            let (fw, fh) = self.menu_dimensions();
            for (m,correlation) in mq {
                let _context=correlation.clone().map(|c|self.events.clock.context(c));
                if was_capturing {
                    // Any click abandons a pending key capture.
                    if matches!(m, MouseEv::Click(..)) {
                        self.menu.end_capture();
                    }
                    if matches!(m,MouseEv::Click(..)|MouseEv::Wheel(_)){self.host_consumed("Mouse input during binding capture");}
                    self.observe_menu("mouse-capture-cancel");
                    continue;
                }
                if !matches!(m,MouseEv::Move(_)){self.menu.cancel_debug_unlock();}
                match m {
                    MouseEv::Move(p) => {
                        if let Some((x, y)) = self.window_to_frame(p) {
                            self.menu.hover(x, y, fw, fh);
                        }
                    }
                    MouseEv::Click(p, left) => {
                        if let Some((x, y)) = self.window_to_frame(p) {
                            if left && self.menu.information_hit(x,y,fw,fh){self.menu.information(&self.av,&self.icfg);self.menu_dirty=true;continue;}
                            if let Some(a) = self.menu.click(x, y, fw, fh, left, &mut self.av) {
                                acts.push((a,correlation.clone()));
                            }
                        }
                    }
                    MouseEv::Wheel(up) => {
                        let (fw,fh)=self.menu_dimensions();
                        if let Some((x,y))=self.mouse_pos.and_then(|p|self.window_to_frame(p)){if self.menu.atlas_reference_wheel(up,x,y,fw,fh){continue;}}
                        if self.menu.host_panel_active(){self.menu.content_wheel(up);self.host_consumed("Host panel consumes mouse wheel");self.observe_menu("mouse-wheel");continue;}
                        let ev = if up {
                            menu::MenuInput::Up
                        } else {
                            menu::MenuInput::Down
                        };
                        if let Some(a) = self.menu.handle(ev, &mut self.av) {
                            acts.push((a,correlation.clone()));
                        }
                        self.menu.cancel_debug_unlock();
                    }
                }
                if !matches!(m,MouseEv::Move(_))&&self.events_enabled(){self.ui_event("host-pointer-dispatch",serde_json::json!({"kind":if matches!(m,MouseEv::Click(..)){"click"}else{"wheel"},"outcome":"routed-to-existing-menu-handler","guest_input":false}));}
                self.observe_menu(match m{MouseEv::Move(_)=>"mouse-hover",MouseEv::Click(..)=>"mouse-click",MouseEv::Wheel(_)=>"mouse-wheel"});
            }
            for (action,correlation) in acts {
                let correlation=correlation.unwrap_or_else(||self.events.clock.request("menu-action"));
                let _context=self.events.clock.context(correlation);
                self.observe_menu("menu-action-before");
                if self.events_enabled(){self.ui_event("host-action",serde_json::json!({"action":format!("{action:?}"),"operation_id":crate::debug_operations::menu_operation(&action),"menu":self.ui_focus()}));}
                match action {
                    menu::MenuAction::OpenDebugTool(tool)=>self.workbench.open(&mut self.menu,tool),
                    menu::MenuAction::ToolRequest(request)=>{if let Err(e)=self.workbench_request(request){self.host_refused(&e);self.workbench.last=format!("Refused: {e}");self.menu.tool_feedback(self.workbench.last.clone(),String::new(),false);}},
                    menu::MenuAction::ChooseToolInput(slot)=>{if self.menu.tool_active(){self.workbench.remember(&self.menu);self.workbench.cancel_view();}self.choose_tool_input(slot,"Choose the input reference. Actual action preflight still applies.".into());},
                    menu::MenuAction::ContentOpen=>{let rom=gba_core::ffta_rom::digest(&self.machine.bus.rom);if let Err(e)=self.content_picker.open(&mut self.menu,rom){self.host_refused(&e);self.menu.diagnosis_result(e);}},
                    menu::MenuAction::ContentRefresh=>{if let Err(e)=self.content_picker.refresh(&mut self.menu){self.host_refused(&e);self.menu.content_message(e);}},
                    menu::MenuAction::ContentBrowseFile|menu::MenuAction::ContentAddFolder=>{
                        {
                            self.modal_input.begin();
                            let result=if action==menu::MenuAction::ContentBrowseFile{self.content_picker.browse_file(&mut self.menu)}else{self.content_picker.browse_folder(&mut self.menu)};
                            self.modal_input.finish(Instant::now());if let Err(e)=result{self.host_refused(&e);self.menu.content_message(e);}
                            self.held.clear();self.pending_key=None;self.pending_pad=None;self.mouse_queue.clear();self.nav_prev=Default::default();
                        }
                        self.observe_menu("external-dialog-return");self.host_action_returned(action);
                        break;
                    },
                    menu::MenuAction::ContentSelect{index,generation}|menu::MenuAction::ContentPreview{index,generation}=>{if let Err(e)=self.content_picker.choose(&mut self.menu,index,generation,matches!(action,menu::MenuAction::ContentPreview{..})){self.host_refused(&e);self.menu.content_message(e);}},
                    menu::MenuAction::ContentCancel=>self.content_picker.leave_view(&self.menu),
                    menu::MenuAction::CreateIssueReport=>self.workbench.open(&mut self.menu,menu::DebugTool::Issue),
                    menu::MenuAction::StartRecentEvents=>self.workbench.open(&mut self.menu,menu::DebugTool::Recent),
                    menu::MenuAction::SaveRecentEvents=>{if let Err(e)=self.debug_action(&crate::debug_control::Action::RecentReport){self.menu.diagnosis_result(e);self.menu_dirty=true;}},
                    menu::MenuAction::StartGameplayTrace=>self.workbench.open(&mut self.menu,menu::DebugTool::Gameplay),
                    menu::MenuAction::SaveGameplayReport=>{if let Err(e)=self.debug_action(&crate::debug_control::Action::GameplayReport){self.menu.diagnosis_result(e);self.menu_dirty=true;}},
                    menu::MenuAction::StartCodeTrace=>self.workbench.open(&mut self.menu,menu::DebugTool::Code),
                    menu::MenuAction::SaveCodeReport=>{if let Err(e)=self.debug_action(&crate::debug_control::Action::CodeReport){self.menu.diagnosis_result(e);self.menu_dirty=true;}},
                    menu::MenuAction::StartAudioCapture=>self.workbench.open(&mut self.menu,menu::DebugTool::Audio),
                    menu::MenuAction::SaveAudioReport=>{if let Err(e)=self.debug_action(&crate::debug_control::Action::AudioReport){self.menu.diagnosis_result(e);self.menu_dirty=true;}},
                    menu::MenuAction::ToggleEventConsole=>self.toggle_event_console(event_loop),
                    menu::MenuAction::OpenCommandConsole=>self.open_command_console(event_loop),
                    menu::MenuAction::StartUiTrace=>{if let Err(e)=self.debug_action(&crate::debug_control::Action::UiStart){self.menu.diagnosis_result(e);self.menu_dirty=true;}},
                    menu::MenuAction::SaveUiReport=>{if let Err(e)=self.debug_action(&crate::debug_control::Action::UiReport){self.menu.diagnosis_result(e);self.menu_dirty=true;}},
                    menu::MenuAction::ToggleFastForward=>{self.ff_latched=!self.ff_latched;self.menu_dirty=true;},
                    menu::MenuAction::Resume => {
                        if self.replay_job.is_some() {self.menu.diagnosis_result("Finish or cancel the replay before resuming.".into());}
                        else {self.paused=false;self.lifecycle_emit(crate::debug_lifecycle::Family::Machine,"menu-resume",None,serde_json::json!({"paused":false}));}
                    },
                    menu::MenuAction::CaptureGraphics=>self.workbench.open(&mut self.menu,menu::DebugTool::Graphics),
                    menu::MenuAction::InspectPC=>self.workbench.open(&mut self.menu,menu::DebugTool::Inspect),
                    menu::MenuAction::DebugStep(frames) => {
                        if let Err(error)=self.debug_action(&crate::debug_control::Action::Step{frames,keys:0x3ff}){self.menu.diagnosis_result(error);}
                    },
                    menu::MenuAction::DebugResume => {
                        if let Err(error)=self.debug_action(&crate::debug_control::Action::Resume){self.menu.diagnosis_result(error);}
                    },
                    menu::MenuAction::RestartRom => self.restart_rom(),
                    menu::MenuAction::SaveState { slot, overwrite } => self.state_operation(true, slot, overwrite),
                    menu::MenuAction::LoadState(slot) => self.state_operation(false, slot, false),
                    menu::MenuAction::UndoState => self.undo_state_operation(),
                    menu::MenuAction::OpenDebugCatalog => self.open_debug_catalog(false),
                    menu::MenuAction::OpenCheatCatalog | menu::MenuAction::CheatReload => self.open_cheat_catalog(),
                    menu::MenuAction::CheatChoose => self.choose_tool_input(menu::ContentSlot::CheatCatalog,"Seleziona il catalogo, torna indietro e premi Ricarica. Nessun codice viene eseguito.".into()),
                    menu::MenuAction::ChooseDebugCatalog => self.open_debug_catalog(true),
                    menu::MenuAction::CatalogSelection(index) => self.catalog_preview(index),
                    menu::MenuAction::CatalogImport(index) => self.catalog_import(index),
                    menu::MenuAction::CatalogOpen(index) => self.catalog_open(index),
                    menu::MenuAction::CompareReplay => self.start_replay(false,ReplayKind::Comparison),
                    menu::MenuAction::CaptureReplay => self.start_replay(false,ReplayKind::Capture),
                    menu::MenuAction::AuditReplay => self.start_replay(false,ReplayKind::Audit),
                    menu::MenuAction::ReplayCase => self.start_replay(false,ReplayKind::Plain),
                    menu::MenuAction::ChooseReplayProfile => self.start_replay(true,ReplayKind::Plain),
                    menu::MenuAction::CancelReplay => self.cancel_replay(),
                    menu::MenuAction::StartInputRecording => self.start_input_recording(),
                    menu::MenuAction::StopInputRecording => { self.stop_input_recording("user stopped", true); },
                    menu::MenuAction::CopyDiagnosis => self.copy_diagnosis(),
                    menu::MenuAction::StartDebugSession => self.start_debug_session(),
                    menu::MenuAction::CloseDebugSession => {self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"close-requested",None,serde_json::json!({"source":"close-debug-copy","scope":"No implicit collector export"}));quit=true;},
                    menu::MenuAction::RestoreBackup(id) => self.restore_save_backup(id),
                    menu::MenuAction::SwitchGameSave(slot) => self.switch_game_save(slot),
                    menu::MenuAction::CopyGameSave { slot, overwrite } => self.copy_game_save(slot, overwrite),
                    menu::MenuAction::Quit => {self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"close-requested",None,serde_json::json!({"source":"quit","scope":"No implicit collector export"}));quit=true;},
                    menu::MenuAction::SwitchRom => {
                        if self.choose_rom() {self.observe_menu("switch-rom");self.host_action_returned(action);quit=true;break;}
                    },
                    menu::MenuAction::InputDevice(device) => {
                        self.icfg.device = device;
                        if device != input_config::Device::Gamepad {
                            self.icfg.mouse_keyboard = device == input_config::Device::MouseKeyboard;
                            let enabled=self.icfg.mouse_keyboard;
                            self.machine.bus.ffta_battle_mouse.set_enabled(enabled);
                            self.game_mouse=gba_core::ffta_mouse::LanguageMouse::new(&self.machine.bus.rom,enabled);
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
                        self.icfg.zoom_in_pad = defaults.zoom_in_pad;
                        self.icfg.ff_pad = defaults.ff_pad;
                        self.icfg.ff_toggle_pad = defaults.ff_toggle_pad;
                        self.icfg.zoom_out_pad = defaults.zoom_out_pad;
                        self.icfg.menu_pad = defaults.menu_pad;
                        self.icfg.dpad_source = defaults.dpad_source;
                        self.icfg.stick_deadzone = defaults.stick_deadzone;
                        if let Err(error) = self.icfg.save() {
                            eprintln!("menu: could not save input.cfg ({error})");
                        }
                    }
                    menu::MenuAction::Changed(what) => {
                        if what == menu::Changed::Debug {
                            self.refresh_state_slots();
                        }
                        if what == menu::Changed::Fullscreen {
                            self.set_fullscreen(self.av.video_fullscreen);
                        }
                        // Vsync is the presenter's own knob; everything
                        // else routes through apply_av_change.
                        if what == menu::Changed::Vsync {
                            let result = self.presenter.as_mut().map(|p| p.set_vsync(self.av.video_vsync));
                            if let Some(Err(error)) = result {
                                eprintln!("video: GPU VSync configuration failed ({error}); CPU scaling active");
                                drop(self.presenter.take());
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
                        if !matches!(what, menu::Changed::Overlay | menu::Changed::Audio | menu::Changed::Fullscreen | menu::Changed::Debug) {
                            self.rebuild_temporal();
                        }
                        // apply_av_change rebuilds grid_params from scratch.
                        self.grid_params.scale_mode = scale_mode_of(&self.av.scaling);
                        if what != menu::Changed::Fullscreen {
                            if let Err(e) = self.av.save() {
                                eprintln!("menu: could not save av.cfg ({e})");
                            }
                        }
                    }
                }
                self.observe_menu("menu-action-handler");
                self.host_action_returned(action);
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
                self.interrupt_probe();
                self.frame_control.stop("running");
                self.debug_audio_boundary();
                self.suppress = !keys & 0x3FF;
                return;
            }
            if self.probe.is_some() {self.probe_tick();}
            if self.frame_control.remaining>0 {
                let before=self.machine.bus.frames;
                if !self.run_one_frame() {self.frame_control.stop("incomplete-frame");self.finish_control();event_loop.exit();return;}
                if let Err(error)=self.frame_control.completed_frame(before,self.machine.bus.frames){self.error=Some(error);self.finish_control();event_loop.exit();return;}
                self.menu_dirty=true;
                self.finish_control();
            }
            self.menu.set_debug_position(self.machine.bus.frames,self.machine.bus.clock,self.frame_control.reason,self.probe.as_ref().map_or(self.frame_control.remaining,|p|p.remaining() as u32));
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
            for i in 0..2 {if let Some(layout)=self.ui_layout((mw as u32,mh as u32),i) {
                let mut canvas=std::mem::take(&mut self.pause_rgba);
                let ui=self.ui_frame(layout,i);
                for y in 0..mh {for x in 0..mw {
                    if let Some(c)=ui.sample(x as f32+0.5,y as f32+0.5) {canvas[y*mw+x]=c;}
                }}
                self.pause_rgba=canvas;
            }}
            self.menu.set_live_status(self.machine.bus.frames,self.ff_latched,input_config::fast_forward::effective_percent(self.av.ff_speed,self.ff_latched,false));
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

        self.game_keys=keys;

        // Audio-clock pacing: the device callback consumes at a fixed rate, so
        // run exactly as many frames as the ring needs — production follows
        // consumption, immune to display refresh rate (ProMotion) and window-
        // server throttling. Without a device, wall-clock pace at the hardware
        // frame rate (handled below).
        // The multiplier never activates fast-forward by itself. Holding the
        // momentary key inverts the session's permanent-toggle state.
        let ff_held = !self.menu_shortcut_held && (self.pad_ff_held
            || winit_key(&self.icfg.ff_key).is_some_and(|k| self.held.contains(&k)));
        let ff_speed = input_config::fast_forward::effective_percent(self.av.ff_speed, self.ff_latched, ff_held);
        let ff_now = self.ff_latched ^ ff_held;
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
        let clock_pacing = self.clock_pacing;
        let smooth_normal = clock_pacing && self.primed && !self.ff_active;
        let frame_cap = if self.ff_active {
            self.ff_clock.next_batch(self.ff_speed)
        } else if smooth_normal {
            1
        } else {
            4
        };
        // A tick can include multiple guest frames; keep its inclusive time
        // separate from per-frame core/UI/audio phases. Pause polling is excluded.
        let tick_start = self.profile.start();
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
                    st.pacing_fill()
                };
                if fill >= RING_TARGET {
                    break;
                }
            } else if ran >= 1 {
                break;
            }
            ran += 1;
            if !self.run_one_frame() {
                event_loop.exit();
                return;
            }
        }

        // Once both rings first reach target, the buffer has primed: discard
        // the cold-start underruns so they don't report as a defect.
        if !self.primed && self.stream.is_some() && !self.ff_active {
            let st = self.streams.lock().unwrap();
            if st.pacing_fill() >= RING_TARGET {
                drop(st);
                self.primed = true;
                if clock_pacing {
                    self.next_frame = Instant::now();
                }
                self.reset_audio_counters();
            }
        }

        if !self.ff_active || self.av.ff_audio != "silent" {
            self.audio_watchdog();
        }

        if ran == 0 {
            // Ring is full: let the device drain a little before the next tick.
            // (winit pumps OS events between ticks; no explicit pump needed.)
            let wait_start = self.profile.start();
            std::thread::sleep(Duration::from_millis(2));
            self.profile.finish(Phase::PacingWait, wait_start);
            self.profile.finish(Phase::Tick, tick_start);
            return;
        }
        if self.stream.is_none() || self.ff_active || smooth_normal {
            self.next_frame += Duration::from_nanos(16_742_706); // 59.7275 Hz
            let now = Instant::now();
            if self.next_frame > now {
                let wait_start = self.profile.start();
                std::thread::sleep(self.next_frame - now);
                self.profile.finish(Phase::PacingWait, wait_start);
            } else {
                self.next_frame = now;
            }
        }

        // Resolve the visible image (temporal response + color LUT) and
        // present it (downstream of production, never the pacing source).
        let compose_start = self.profile.start();
        self.temporal.compose(&self.lut, &mut self.rgba);
        self.profile.finish(Phase::TemporalCompose, compose_start);
        let present_start = self.profile.start();
        self.present_rgba();
        self.profile.finish(Phase::Present, present_start);
        self.profile.finish(Phase::Tick, tick_start);
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
            .with_title(self.recording_title())
            .with_window_icon(load_window_icon())
            .with_inner_size(LogicalSize::new(self.base_w() as f64 * 3.0, 160.0 * 3.0))
            .with_resizable(true);
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.error = Some(format!("window: {e}"));
                event_loop.exit();
                return;
            }
        };
        if let Ok(image)=image::load_from_memory(include_bytes!("../../../assets/cursor.png")) {
            let image=image.into_rgba8();
            if let Ok(source)=winit::window::CustomCursor::from_rgba(image.into_raw(),32,40,2,2) {
                window.set_cursor(event_loop.create_custom_cursor(source));
            }
        }
        // The texture is allocated for the largest frame; each present uploads
        // only the current (zoom-dependent) frame size and crop.
        self.presenter = match screen::present::Presenter::new_profiled(
            window.clone(),
            (240 + 2 * gba_core::mem::MAX_EXTRA) as u32,
            (160 + 2 * gba_core::mem::MAX_EXTRA_Y) as u32,
            self.display_target,
            self.av.video_vsync,
            self.profile.gpu_enabled(),
        ) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("video: GPU presenter unavailable ({e}); CPU scaling active");
                None
            }
        };
        self.window = Some(window);
        if self.benchmark.is_none() && self.av.video_fullscreen { self.set_fullscreen(true); }
        // The window now exists: tell a supervising frontend we're live.
        if self.status {
            status_line("playing");
        }
    }

    fn window_event(&mut self,event_loop:&ActiveEventLoop,id:WindowId,event:WindowEvent) {
        if self.command_console_panel.as_ref().is_some_and(|panel|panel.owns(id)) {
            let completing=matches!(&event,WindowEvent::KeyboardInput{event:KeyEvent{physical_key:PhysicalKey::Code(KeyCode::Tab),state:ElementState::Pressed,repeat:false,..},is_synthetic:false,..});
            let profiles=if completing{self.content_picker.completion_ids(menu::ContentSlot::Replay)}else{Vec::new()};
            let catalogs=if completing{self.content_picker.completion_ids(menu::ContentSlot::Catalog)}else{Vec::new()};
            let cheats=if completing{self.content_picker.completion_ids(menu::ContentSlot::CheatCatalog)}else{Vec::new()};
            let requests=if completing{self.control_server.as_ref().and_then(|s|s.pending_status()["request_id"].as_str().map(str::to_owned)).into_iter().collect::<Vec<_>>()}else{Vec::new()};
            if matches!(event,WindowEvent::Focused(false)){self.command_pad_handoff=Some(0);}
            if let Some(panel)=self.command_console_panel.as_mut(){if panel.event(&event,&profiles,&catalogs,&cheats,&requests){panel.hide();self.command_pad_handoff=Some(0);}}
            return;
        }
        if self.event_console_panel.as_ref().is_some_and(|panel|panel.owns(id)) {
            let shortcut=matches!(&event,WindowEvent::KeyboardInput{event:winit::event::KeyEvent{physical_key:PhysicalKey::Code(code),state:ElementState::Pressed,repeat:false,..},is_synthetic:false,..} if self.console_key(*code));
            let root=input_config::exe_dir();
            if let Some(panel)=self.event_console_panel.as_mut() {
                let hide=shortcut || panel.event(&event,&mut self.event_console.borrow_mut(),root.as_deref());
                if hide {panel.hide(&mut self.event_console.borrow_mut());}
            }
            return;
        }
        let cause=match &event {
            WindowEvent::KeyboardInput{..}=>"keyboard",WindowEvent::Ime(..)=>"ime",
            WindowEvent::CursorMoved{..}=>"mouse-hover",WindowEvent::MouseInput{..}=>"mouse-button",
            WindowEvent::MouseWheel{..}=>"mouse-wheel",WindowEvent::Focused(..)=>"window-focus",_=>"window-event"
        };
        self.observe_menu("input-before");
        let _context=self.events_enabled().then(||self.events.clock.context(self.events.clock.request(cause)));
        if self.events_enabled(){
            match &event {
                WindowEvent::MouseInput{state,button,..}=>self.ui_event("mouse-input",serde_json::json!({"backend":"Winit","state":format!("{state:?}"),"button":format!("{button:?}"),"position":self.mouse_pos,"paused":self.paused})),
                WindowEvent::MouseWheel{delta,..}=>self.ui_event("mouse-input",serde_json::json!({"backend":"Winit","wheel":format!("{delta:?}"),"position":self.mouse_pos,"paused":self.paused})),
                WindowEvent::Ime(winit::event::Ime::Preedit(text,..))=>self.ui_event("host-text-input",serde_json::json!({"phase":"preedit","text":text.chars().take(256).collect::<String>(),"chars":text.chars().count(),"consumed_by_editor":self.paused&&self.menu.content_text_focus()})),
                WindowEvent::Ime(winit::event::Ime::Commit(text))=>self.ui_event("host-text-input",serde_json::json!({"phase":"commit","text":text.chars().take(256).collect::<String>(),"chars":text.chars().count(),"consumed_by_editor":self.paused&&self.menu.content_text_focus()})),
                _=>{}
            }
        }
        if matches!(&event,WindowEvent::CloseRequested){self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"close-requested",None,serde_json::json!({"scope":"Frontend close request; buffers are not automatically exported"}));}
        self.window_event_observed(event_loop,id,event);
        self.observe_menu(cause);
    }
    fn about_to_wait(&mut self,event_loop:&ActiveEventLoop) {
        self.event_console.borrow_mut().begin_tick();
        self.observe_menu("frontend-before");
        self.about_to_wait_observed(event_loop);
        self.observe_menu("frontend-tick");
        self.lifecycle_poll();
        if let Some(panel)=self.event_console_panel.as_mut() {
            if let Err(error)=panel.draw_if_due(&self.event_console.borrow()){eprintln!("event console: {error}");}
        }
        if self.command_console_panel.as_ref().is_some_and(|p|p.wants_observation()){
            let pending=self.control_server.as_ref().map_or(serde_json::Value::Null,|s|s.pending_status());
            let remaining=self.frame_control.remaining;let search_remaining=self.probe.as_ref().map(|p|p.remaining());
            self.command_console_panel.as_mut().unwrap().observe(&pending,remaining,search_remaining);
        }
        if let Some(panel)=self.command_console_panel.as_mut(){panel.tick();if let Err(error)=panel.draw_if_due(){eprintln!("command console: {error}");}}
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.lifecycle_emit(crate::debug_lifecycle::Family::Session,"frontend-exiting",None,serde_json::json!({"frame":self.machine.bus.frames,"clock":self.machine.bus.clock,"error":self.error.as_ref().map(|e|e.chars().take(1024).collect::<String>()),"scope":"Only already-active in-memory collectors; no implicit export"}));
        if self.replay_job.is_some(){self.lifecycle_finish(crate::debug_lifecycle::Family::Replay,serde_json::json!({"outcome":"interrupted","scope":"Existing supervisor drops/kills its worker on session exit"}));}
        self.content_picker.persist_now(&self.menu);
        drop(self.replay_job.take());
        self.interrupt_probe();
        drop(self.control_server.take());
        if self.capture.is_some() {
            let complete = self.error.is_none();
            if !self.stop_input_recording(if complete { "clean session close" } else { "runtime error" }, complete) && complete {
                self.error = Some("Cannot finish input recording; inspect the retained recording folder and session log".into());
            }
        }
        if let Some(error)=self.error.clone(){self.recent_error(&error);}
        // run_app consumes/drops the event loop before PlayApp is dropped.
        // In particular, wgpu's EGL backend must release its Wayland objects
        // while that display connection still exists (even with Vulkan selected).
        // Tear down here on both success and error, with surfaces before windows.
        drop(self.stream.take());
        drop(self.presenter.take());
        drop(self.sb_surface.take());
        drop(self.sb_context.take());
        drop(self.window.take());
        crate::rom_data::report_usage(&self.machine);
        self.profile.report();
        let audio_profile = self.streams.lock().unwrap().profile.take();
        if let Some(profile) = audio_profile { profile.report(); }
        if let Some(benchmark) = self.benchmark.as_mut() {
            if self.error.is_none() {
                if let Err(error) = benchmark.report(&self.machine, self.native_run, self.fallback_run) { self.error = Some(error); }
            }
            if let Some(error)=self.error.clone(){self.recent_error(&error);}
            return; // Developer replay never writes cartridge saves or backups.
        }

        // Final teardown on a clean exit (window closed / menu Quit): this is
        // winit's guaranteed last callback, so it runs whether or not
        // `run_app` returns on a given platform. The in-loop periodic save
        // bounds loss on an abnormal (signal) kill. Skip if we're already
        // failing (e.g. window creation never succeeded).
        if self.error.is_some() {
            return;
        }
        if let Some(data) = self.machine.bus.save_data() {
            match write_sav(&self.sav_path, data) {
                Ok(()) => eprintln!("saved {}", self.sav_path),
                Err(e) => {
                    self.error = Some(format!("{}: {e}", self.sav_path));
                    if let Some(error)=self.error.clone(){self.recent_error(&error);}
                    return;
                }
            }
        }
        print_fallback_census();
        if FALLBACK_COLLECT.load(Ordering::Relaxed) {
            if let Err(e) = record_labels(
                self.machine.bus.rom.len(),
                &rom_sha256(&self.machine.bus.rom),
            ) {
                self.error = Some(e);
                if let Some(error)=self.error.clone(){self.recent_error(&error);}
            }
        }
    }
}

/// Interactive play: resolve the image/BIOS/translation, set up audio, input,
/// and screen simulation, then run the winit event loop until the window
/// closes. Teardown (final save, fallback census, label recording) happens
/// in the final event-loop callback, before its display connection is dropped.
pub fn cmd_play(args: &[String]) -> Result<(), String> {
    let session = crate::debug_session::current()?;
    let result = cmd_play_inner(args, session.as_ref());
    if let Some(session) = &session { session.result(&result); }
    result
}

fn cmd_play_inner(args: &[String], session: Option<&crate::debug_session::Session>) -> Result<(), String> {
    let start_recording = session.is_some() && args == ["--record-input"];
    let start_paused = session.is_some() && args == ["--debug-paused"];
    if session.is_some() && !args.is_empty() && !start_recording && !start_paused {
        return Err("The isolated runtime is pinned to its case. Reopen it with debug-session open <folder>, without play overrides.".into());
    }
    if start_recording && session.and_then(|s|s.state()).is_none() { return Err("Recording start needs a snapshot seed".into()); }
    let args = if start_recording || start_paused { &[][..] } else { args };
    let mut rom_path = session.map(|s| s.rom().to_string_lossy().into_owned());
    let mut interp_only = session.is_some_and(|s| s.interp());
    // Opt-in decoded resources until the equivalence/cost gates pass.
    let mut prepare_rom_data = false;
    // Machine-readable lifecycle lines on stdout for a supervising frontend
    // (the launcher): `STATUS building <pct>` / `STATUS playing`.
    let mut status = false;
    // Perf instrumentation is developer tooling: always on in debug builds,
    // opt-in (--stats or GBA_RECOMP_STATS=1) in release.
    let mut show_stats = cfg!(debug_assertions) || std::env::var_os("GBA_RECOMP_STATS").is_some();
    let mut bios_arg: Option<String> = session.and_then(|s| s.bios()).map(|p| p.to_string_lossy().into_owned());
    let mut no_bios = session.is_some_and(|s| s.bios().is_none());
    let mut gamedb_arg: Option<String> = None;
    let mut catalog_arg: Option<std::path::PathBuf> = None;
    let mut replay_path: Option<std::path::PathBuf> = None;
    let mut benchmark = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--benchmark" => {
                if benchmark.is_some() { return Err("--benchmark may only be supplied once".into()); }
                benchmark = Some(benchmark::Benchmark::load(Path::new(it.next().ok_or("--benchmark needs a TOML config path")?))?);
            }
            "--interp" => interp_only = true,
            "--rom-data" => prepare_rom_data = true,
            "--status" => status = true,
            "--stats" => show_stats = true,
            "--record-labels" => FALLBACK_COLLECT.store(true, Ordering::Relaxed),
            "--bios" => bios_arg = Some(it.next().ok_or("--bios needs a value")?.to_string()),
            "--no-bios" => no_bios = true,
            "--debug-replay" => replay_path = Some(it.next().ok_or("--debug-replay needs replay.json")?.into()),
            "--debug-catalog" => catalog_arg = Some(it.next().ok_or("--debug-catalog needs a file")?.into()),
            "--gamedb" => gamedb_arg = Some(it.next().ok_or("--gamedb needs a value")?.to_string()),
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    if benchmark.is_some() && std::env::var_os("RECOMP_RECORD_INPUT").is_some() {
        return Err("--benchmark cannot record input".into());
    }
    let debug_catalog = catalog_arg.as_deref().map(crate::debug_catalog::Catalog::load).transpose()?;
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
    let prepared_resources = if prepare_rom_data {
        Some(crate::rom_data::prepare_startup(&rom, status)?)
    } else {
        None
    };
    // A benchmark has no cartridge destination: avoid save discovery/pickers and
    // directory creation. This inert path only constructs unused slot owners.
    let sav_path = if benchmark.is_some() {
        Path::new(&rom_path).with_extension("sav")
    } else {
        crate::save_path::resolve(Path::new(&rom_path))?
    }.to_string_lossy().into_owned();

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
            Err(e) => {
                eprintln!("DEGRADED: native translation unavailable ({e}); interpreter only");
                None
            }
        }
    };

    let mut machine = make_machine(rom, bios.as_deref());
    let prepared_resources_directory = session.and_then(|s| s.dataset())
        .or_else(|| prepared_resources.as_ref().map(|p| p.directory.clone()));
    if let Some(directory) = &prepared_resources_directory {
        crate::rom_data::attach_from_directory(&mut machine, directory)?;
    }
    let save_backups = crate::save_backups::Backups::new(Path::new(&sav_path));
    if benchmark.is_none() {
        if let Some(sav) = save_backups.capture_startup().map_err(|e| format!("Cannot protect cartridge save: {e}"))? {
            machine.bus.load_save_data(&sav);
            eprintln!("loaded {sav_path}; startup backup completed");
        }
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

    let title = session.map_or_else(|| "FFTARecomp".into(), |s| format!("FFTARecomp - DEBUG: {}", s.manifest.label));

    // Audio. Both paths run at once so the in-game menu can crossfade between
    // faithful and enhanced live (the callback slews between them, the producer
    // fills both rings). tap_channels gates only the extra per-channel work;
    // the faithful mix is produced regardless. The MP2K/engine HLE shadow is
    // armed when the stock driver is present so the enhanced path is ready the
    // instant it's selected; it's read-only, so arming it always is free of
    // correctness risk.
    let mut av = input_config::AvConfig::load();
    if benchmark.is_some() { av.video_fps = "off".into(); show_stats = false; }
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
    let zoom_max = zoom_limit(base_extra);
    machine.bus.tap_channels = true;
    let desc = arm_audio_hle(
        &mut machine,
        manifest.as_ref().and_then(|m| m.engine_hle.as_deref()),
    );
    eprintln!("audio engine: {desc}");
    let streams = Arc::new(Mutex::new(AudioStreams::default()));
    {
        let mut st = streams.lock().unwrap();
        st.configure_audio(&av);
        if benchmark.is_none() && std::env::var("FFTA_PROFILE").as_deref() == Ok("1") {
            st.profile = Some(crate::audio_profile::Profile::with_frame_window(
                std::env::var("FFTA_AUDIO_PROFILE_FRAMES").ok().as_deref(),
            )?);
        }
    }
    let stream = if benchmark.is_some() { None } else { start_audio(streams.clone()) };

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
    if let Some(gilrs)=&pad {
        eprintln!("gamepad: Linux native snapshot polling (gilrs/evdev), {} connected",gilrs.gamepads().count());
    }else{
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
    let lut = screen::ColorLut::build(&screen::ColorSettings {
        screen: screen_kind,
        darken: input_config::AvConfig::knob(&av.screen_darken)
            .map(f64::from)
            .unwrap_or(f64::NAN),
        target: lut_primaries,
    });
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
    let game_saves = crate::game_saves::GameSaves::new(Path::new(&sav_path));
    let state_slots = crate::save_state::Slots::new(Path::new(&sav_path), &machine.bus.rom);
    let game_mouse=gba_core::ffta_mouse::LanguageMouse::new(&machine.bus.rom,icfg.mouse_keyboard || std::env::var("FFTA_MOUSE").as_deref()==Ok("1"));
    if icfg.mouse_keyboard {machine.bus.ffta_battle_mouse.set_enabled(true);}
    let boot_state = machine.save_state()?;
    let debug_start_state = session.and_then(|s| s.state());
    if let Some(state) = &debug_start_state { crate::save_state::load(&mut machine, state)?; }
    let benchmark_zoom = benchmark.as_ref().map_or(0, |b| b.zoom);
    if benchmark_zoom > zoom_max { return Err("benchmark zoom exceeds the selected aspect's limit".into()); }
    if let Some(benchmark) = benchmark.as_mut() { benchmark.restore(&mut machine)?; }
    // Restored snapshots retain their exact serialized tap flag. Route only
    // host audio around unavailable per-channel taps without rewriting state.
    streams.lock().unwrap().taps_unavailable = !machine.bus.tap_channels;
    let diagnosis = crate::diagnostics::Request {
        rom: rom_path.into(),
        bios: bios_arg.map(Into::into).or_else(input_config::find_bios),
        no_bios: !machine.bus.real_bios,
        dataset: prepared_resources_directory,
        input: std::env::var_os("RECOMP_RECORD_INPUT").map(Into::into),
        ..Default::default()
    };
    let control_server=session.map(|s|crate::debug_control::Server::new(&s.root)).transpose()?;
    let events=crate::debug_events::Hub::new(control_server.as_ref().map(|s|s.event_session()).unwrap_or_default());
    let mut app = PlayApp {
        diagnosis,
        diagnosis_clipboard: None,
        debug_session: session.is_some(),
        debug_catalog,
        content_picker: Default::default(),
        workbench: Default::default(),
        replay_path,
        replay_job: None,
        replay_terminal: None,
        cheat_refresh: None,
        cheat_snapshot: None,
        cheat_preview: None,
        last_cheat_once: None,
        replay_preparation_seen:0,
        replay_message: String::new(),
        machine,
        sav_path,
        game_saves,
        state_slots,
        save_backups,
        boot_state,
        sav_seen,
        record,
        capture: None,
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
        modal_input: Default::default(),
        active_controller: None,
        last_direct_activity: None,
        pending_pad: None,
        menu_key_capture: input_config::ComboCapture::default(),
        menu_pad_capture: input_config::ComboCapture::default(),
        menu_key_latch: input_config::ComboLatch::default(),
        menu_pad_latch: input_config::ComboLatch::default(),
        menu_shortcut_held: false,
        #[cfg(not(target_os = "linux"))]
        gilrs_gestures: HashMap::new(),
        #[cfg(target_os = "linux")]
        linux_pads: HashMap::new(),
        direct_gestures: HashMap::new(),
        pad_zoom_prev: (false, false),
        pad_ff_held: false,
        pad_ff_toggle_prev: false,
        atlas_down:[false;7],atlas_focus_prev:false,atlas_reference_prev:false,atlas_information_prev:false,
        atlas_shortcuts:Default::default(),atlas_clock:Instant::now(),
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
        ui_pixels: Default::default(),
        panel_pixels: Default::default(),
        padded_frame: Vec::new(),
        title,
        status,
        window: None,
        fullscreen: Default::default(),
        presenter: None,
        sb_context: None,
        sb_surface: None,
        menu_enabled,
        menu: {let mut menu=menu::Menu::new(true,false);menu.set_operation_references(crate::debug_operations::atlas_references());menu},
        paused: false,
        frame_control: Default::default(),
        probe: None,
        probe_result: None,
        inspection: None,
        graphics_report: None,
        ui_trace: Default::default(),
        events,
        event_console: Default::default(),
        event_console_panel: None,
        command_console_panel: None,
        command_pad_handoff: None,
        selections: Default::default(),
        lifecycle: Default::default(),
        audio_diagnostic: false,
        last_audio_report: None,
        code_trace: None,
        gameplay_trace: None,
        last_gameplay_report: None,
        recent: None,
        last_recent_report: None,
        last_code_report: None,
        last_issue_report: None,
        control_server,
        menu_dirty: true,
        // Starts visible (windowed); the loop reconciles against fullscreen.
        cursor_visible: true,
        suppress: 0,
        nav_prev: NavEdges::default(),
        show_stats,
        fps: crate::fps::Meter::new(Instant::now()),
        profile: {
            let mut profile = PlayProfile::new(std::env::var("FFTA_PROFILE").as_deref() == Ok("1"));
            if benchmark.is_none() && profile.enabled() {
                profile.configure_cadence(std::env::var("FFTA_PRESENT_PROFILE_FRAMES").ok().as_deref())?;
            }
            profile
        },
        benchmark,
        emu_ema_ms: 0.0,
        frames_run: 0,
        slow_warned: false,
        native_run: 0,
        fallback_run: 0,
        primed: false,
        clock_pacing: cfg!(any(target_os = "linux", windows))
            && std::env::var("FFTA_FRAME_PACING").as_deref() != Ok("audio"),
        next_frame: Instant::now(),
        audio_watch: Instant::now(),
        audio_consumed_last: 0,
        audio_stalls: 0,
        audio_retry: None,
        ff_active: false,
        ff_latched: false,
        ff_speed: 100,
        ff_clock:Default::default(),
        ff_audio_active: String::new(),
        pending_key: None,
        base_extra,
        zoom_level: benchmark_zoom,
        zoom_max,
        view: [0.0, 0.0, (240 + 2 * base_extra) as f32, 160.0],
        mouse_pos: None,
        game_mouse,
        game_keys:0x3ff,
        mouse_queue: Vec::new(),
        error: None,
    };

    app.menu.set_debug_enabled(app.av.debug);
    app.menu.set_debug_session(session.is_some());
    if debug_start_state.is_some() { app.after_state_restore(); }
    if start_paused {app.debug_action(&crate::debug_control::Action::Pause)?;}
    if start_recording {
        app.start_input_recording();
        if app.capture.is_none() { return Err("Cannot start input recording; see recording error in the session log".into()); }
    }

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
fn battle_menu_ui_layer(state:&gba_core::ffta_dialogue::DialogueState)->Option<usize> {
    if matches!(state.extra.kind,gba_core::ffta_dialogue::PanelKind::Actions|gba_core::ffta_dialogue::PanelKind::Notice) && state.extra.active() {Some(1)}
    else if state.action_layout || state.town_layout || state.shop_layout {Some(0)}else{None}
}
fn pub_ui_layout(physical:(u32,u32),base_width:f32,size:f32)->screen::ui_layer::Layout {
    screen::ui_layer::Layout::new_with_anchor(physical,base_width,true,size,[0.,0.],0.)
}
fn menu_panel_native(index:usize,bounds:[i32;4],length:f32)->(f32,f32) {
    let distance=((bounds[2]-bounds[0])as f32*(length.clamp(1.,2.)-1.)).round();
    let shift=if bounds[0]+bounds[2]>=240 {distance}else{-distance};
    ((index%480)as f32-120.+shift,(index/480)as f32)
}
#[cfg(test)]
mod gamepad_capture_simulation {
    use super::MenuTapFilter;
    use std::time::{Duration, Instant};

    #[test]
    fn shop_mouse_maps_categories_items_and_quantity_through_the_same_ui_canvas() {
        let mut state=gba_core::ffta_dialogue::DialogueState::default();
        state.shop_layout=true;
        assert_eq!(super::battle_menu_ui_layer(&state),Some(0));
        for window in [(960,640),(1920,1080),(2560,720)] {for scale in [0.5,1.] {
            let layout=super::pub_ui_layout(window,568.,scale);
            for (x,y)in[(24usize,8usize),(40,36),(170,96)] {
                let px=layout.rect[0]+(120.+x as f32+0.5)*layout.rect[2]/480.;
                let py=layout.rect[1]+(y as f32+0.5)*layout.rect[3]/160.;
                let index=layout.source(px,py).unwrap();
                assert_eq!(super::menu_panel_native(index,[0,0,238,155],1.),(x as f32,y as f32));
            }
        }}
    }
    #[test]
    fn pub_menu_mouse_keeps_fixed_right_origin_through_animation() {
        use gba_core::ffta_dialogue::{DialogueState,UiPixel,pack_panel};
        let mut state=DialogueState::default();state.town_layout=true;
        assert_eq!(super::battle_menu_ui_layer(&state),Some(0));
        let bounds=[0,0,240,160];
        let mut pixels=vec![UiPixel::default();480*160];
        pixels[132*480+120+100]=UiPixel::opaque(31);
        let mut packed=Vec::new();pack_panel(&pixels,&[],1.,true,&mut packed);
        for size in [(1280,720),(2560,720)] {
            let layout=super::pub_ui_layout(size,568.,1.);
            let [left,top,width,height]=layout.rect;
            let native_right=left+360.*width/480.;
            assert!((native_right-layout.clip[0]-layout.clip[2]).abs()<1.);
            let index=layout.source(left+220.5/480.*width,top+132.5/160.*height).unwrap();
            assert_eq!(super::menu_panel_native(index,bounds,1.),(100.,132.));
            assert_ne!(packed[index],[0;4]);
        }
    }

    #[test]
    fn battle_menu_mouse_uses_foreground_panel_when_action_hud_is_also_present() {
        use gba_core::ffta_dialogue::{DialogueState,PanelKind,UiPixel,pack_panel};
        let mut state=DialogueState::default();
        state.action_layout=true;
        assert_eq!(super::battle_menu_ui_layer(&state),Some(0)); // confirmation path
        state.extra.kind=PanelKind::Actions;
        let bounds=[160,84,240,148];
        state.extra.bounds=Some(bounds);
        assert_eq!(super::battle_menu_ui_layer(&state),Some(1));
        let mut backing=vec![UiPixel::default();480*160];
        for y in 84..148 {for x in 160..240 {backing[y*480+x+120]=UiPixel::opaque(31);}}
        let mut pixels=backing.clone();
        let target=(192usize,140usize);
        pixels[target.1*480+target.0+120]=UiPixel::opaque(0x7fff);
        for window in [(960,640),(1920,1080),(2560,720)] {
            for width in [240.,284.,568.] {for size in [0.75,1.,1.5] {for length in [1.,1.5,2.] {
                let mut packed=Vec::new();pack_panel(&pixels,&backing,length,true,&mut packed);
                let index=packed.iter().position(|p|*p==UiPixel::opaque(0x7fff).packed()).unwrap();
                let layout=screen::ui_layer::Layout::at_corner(window,width,true,true,size,length,bounds);
                let point=(layout.rect[0]+(index%480)as f32*layout.rect[2]/480.+layout.rect[2]/960.,
                    layout.rect[1]+(index/480)as f32*layout.rect[3]/160.+layout.rect[3]/320.);
                let source=layout.source(point.0,point.1).expect("drawn option must be hittable");
                let (x,y)=super::menu_panel_native(source,bounds,length);
                assert_eq!((x as usize,y as usize),target);
                assert_eq!(((y-84.)/16.)as usize,3);
            }}}
        }
        state.extra.kind=PanelKind::None;
        assert_eq!(super::battle_menu_ui_layer(&state),Some(0));
    }

    #[test]
    fn restored_native_preview_ignores_wide_stride_and_preserves_machine() {
        let mut machine=gba_core::Machine::new(vec![]);
        for (index,pixel) in machine.bus.framebuffer.iter_mut().enumerate() {*pixel=(index%0x7fff)as u16;}
        machine.bus.wide_framebuffer.fill(0x7fff); // Untrusted old layout.
        let before=machine.save_state().unwrap();
        let mut padding=vec![0xffff;123];
        for (extra,ey) in [(0,0),(22,0),(165,81),(448,80)] {
            gba_core::mem::set_extra(extra);gba_core::mem::set_extra_y(ey);
            let width=super::wide_w();let height=super::wide_h();
            let frame=super::restored_native_frame(&machine.bus,&mut padding);
            assert_eq!(frame.len(),width*height);
            for y in 0..height {for x in 0..width {
                let expected=if (extra..extra+240).contains(&x) && (ey..ey+160).contains(&y) {
                    machine.bus.framebuffer[(y-ey)*240+x-extra]
                }else{0};
                assert_eq!(frame[y*width+x],expected);
            }}
            assert!(machine.save_state().unwrap()==before,"preview advanced or mutated restored machine");
        }
        gba_core::mem::set_extra(0);gba_core::mem::set_extra_y(0);
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
        use super::{XInputGamepad, XInputState, xinput_new_press, xinput_pressed};

        let mut av = input_config::AvConfig::default();
        let mut menu = menu::Menu::new(true, false);
        menu.atlas_section(menu::clan_atlas::Domain::Game,6); // Input section.
        menu.handle(menu::MenuInput::Up, &mut av); // Focus input tabs.
        menu.handle(menu::MenuInput::Right, &mut av); // Highlight Gamepad tab.
        menu.handle(menu::MenuInput::Confirm, &mut av); // Select device.
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

#[cfg(test)]
#[path="play_atlas_tests.rs"]
mod atlas_tests;
