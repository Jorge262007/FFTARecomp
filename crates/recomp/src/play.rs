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
struct PlayApp {
    // Core machine + persistence.
    machine: Machine,
    sav_path: String,
    state_slots: crate::save_state::Slots,
    save_backups: crate::save_backups::Backups,
    boot_state: Vec<u8>,
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
    mouse_queue: Vec<MouseEv>,

    /// A fatal error raised inside a callback; surfaced after `run_app`.
    error: Option<String>,
}

impl PlayApp {
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
        screen::ui_layer::Frame {pixels_revision:if index==0 {self.ui_pixels.revision()}else{self.panel_pixels.revision()},pixels:if index==0 {&self.ui_pixels}else{&self.panel_pixels},
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
            let ui=screen::ui_layer::Frame {pixels_revision:None,pixels:if i==0 {&self.ui_pixels}else{&self.panel_pixels},raw_world:if i==1 && self.machine.bus.ffta_dialogue.extra.kind==gba_core::ffta_dialogue::PanelKind::Stats {&self.machine.bus.ffta_dialogue.extra.background}else{&self.machine.bus.wide_framebuffer},
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
        self.pad_zoom_prev = (pad_zoom_in, pad_zoom_out);
        if !self.paused && pad_ff_toggle && !self.pad_ff_toggle_prev {
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

    fn refresh_state_slots(&mut self) {
        let slots = std::array::from_fn(|i| {
            let info = self.state_slots.info(i as u8 + 1);
            menu::StateSlot {
                occupied: info.occupied,
                preview: info.preview,
                status: info.status,
            }
        });
        self.menu.set_state_slots(slots);
        self.menu.set_state_undo(self.state_slots.undo_label());
        let expected = self.machine.bus.save_data().map_or(0, |data| data.len());
        let choices = self.save_backups.choices(expected);
        self.menu
            .set_save_backups(choices.map(|choice| menu::BackupInfo {
                generation: choice.generation,
                status: choice.status,
            }));
    }

    /// Capture the displayed game view plus native UI, excluding the pause menu and FPS HUD.
    fn state_preview(&mut self) -> Vec<[u8; 4]> {
        let (w, h) = (crate::save_state::PREVIEW_W, crate::save_state::PREVIEW_H);
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
        for layer in 0..2 {
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

    fn restore_save_backup(&mut self, generation: u64) {
        if self.record.is_some() {
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
                Err(error) => format!("Cannot restore backup: {error}"),
            };
        self.refresh_state_slots();
        self.menu.backup_result(message);
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn after_state_restore(&mut self) {
        self.profile.presentation_restored(self.machine.bus.frames);
        // Discard consumer resamplers and queues from the old timeline.
        drop(self.stream.take());
        self.ff_latched = false;
        {
            let mut st = self.streams.lock().unwrap();
            st.dead = false;
            if let Some(profile) = st.profile.as_mut() {
                profile.restored_frame(self.machine.bus.frames);
            }
        }
        self.set_fast_forward(false, 1);
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
        if self.record.is_some() && self.state_slots.undo_restores_machine() {
            self.menu
                .state_result("Stop input recording before undoing a load.".into());
            self.menu_dirty = true;
            return;
        }
        let message = match self.state_slots.undo(&mut self.machine) {
            Ok(crate::save_state::UndoEffect::Save(slot)) => format!("Undid save in slot {slot}."),
            Ok(crate::save_state::UndoEffect::Load(slot)) => {
                self.after_state_restore();
                format!("Undid load from slot {slot}.")
            }
            Err(error) => format!("Cannot undo: {error}"),
        };
        self.refresh_state_slots();
        self.menu.state_result(message);
        self.menu_dirty = true;
        self.next_frame = Instant::now();
    }

    fn state_operation(&mut self, saving: bool, slot: u8, overwrite: bool) {
        if !saving && self.record.is_some() {
            self.menu
                .state_result("Stop input recording before loading a state.".into());
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
                if !saving {
                    self.after_state_restore();
                }
                format!("Slot {slot} {}.", if saving { "saved" } else { "loaded" })
            }
            Err(error) => format!(
                "Cannot {} slot {slot}: {error}",
                if saving { "save" } else { "load" }
            ),
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
        self.ff_audio_active = self.av.ff_audio.clone();
        {
            let mut st = self.streams.lock().unwrap();
            st.discard_pending();
            st.muted = on && self.av.ff_audio == "silent";
            st.playback_speed = if on && self.av.ff_audio == "fast" {
                speed as f64
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
        // Apply a pending zoom change before the frame is emulated, so the
        // core renders (and we read back) the size the zoom needs.
        self.sync_view();
        if self.benchmark.is_none() {
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
        }
        let emu_t0 = Instant::now();
        self.fps.configure(&self.av.video_fps, emu_t0);
        let frames_before = self.machine.bus.frames;
        let core_start = self.profile.start();
        let ppu_capture = gba_core::ppu_profile::Capture::start_detailed(
            self.profile.enabled(), self.profile.detailed_ppu(),
        );
        match &self.native {
            Some((_lib, table)) => {
                let mptr = &mut self.machine as *mut Machine as *mut std::ffi::c_void;
                let (n, f) = run_frame_native(&mut self.machine, table, mptr, 5_000_000);
                self.native_run += n;
                self.fallback_run += f;
            }
            None => self.machine.run_frame(5_000_000),
        }
        let ppu_timings = ppu_capture.finish();
        self.profile.finish(Phase::Core, core_start);
        // A guard hit is an incomplete frame, not a new temporal-history entry.
        // Stop cleanly instead of presenting/counting partially advanced state.
        if self.machine.bus.frames == frames_before {
            self.error = Some("emulation step guard exceeded before a complete frame".into());
            return false;
        }
        self.profile.record_ppu(ppu_timings);
        self.fps.emulated(self.machine.bus.frames.saturating_sub(frames_before));
        gba_core::ffta_sprites::trace_frame(&self.machine.bus);
        let ui_start = self.profile.start();
        self.refresh_ui_layers();
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
                    self.title,
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
        let bus = &mut self.machine.bus;
        if self.stream.is_none() || (self.ff_active && self.av.ff_audio == "silent") {
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if matches!(&event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }) {
            self.profile.presentation_boundary();
        }
        if self.benchmark.is_some() {
            if matches!(event, WindowEvent::CloseRequested) { event_loop.exit(); }
            return; // Physical input must not alter a deterministic benchmark.
        }
        self.menu_dirty = true;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            // Lost focus: a key held during alt-tab might never deliver its
            // release, so drop the held set defensively (no stuck inputs).
            WindowEvent::Focused(false) => {self.held.clear();self.game_keys=0x3ff;self.game_mouse.cancel();self.machine.bus.ffta_battle_mouse.cancel();},
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
            } => match state {
                ElementState::Pressed => {
                    if !self.paused && self.game_mouse.is_name_input(&self.machine.bus) {
                        let consumed=match code {
                            KeyCode::Backspace=>self.game_mouse.name_command(&self.machine.bus,false),
                            KeyCode::Enter|KeyCode::NumpadEnter=>self.game_mouse.name_command(&self.machine.bus,true),
                            _=>text.as_ref().is_some_and(|t|self.game_mouse.type_text(&self.machine.bus,t.as_str())),
                        };
                        if consumed {self.held.remove(&code);return;}
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
            },
            WindowEvent::CursorMoved { position, .. } => {
                let p = (position.x, position.y);
                self.mouse_pos = Some(p);
                if self.paused {
                    self.mouse_queue.push(MouseEv::Move(p));
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
                        MouseButton::Left => self.mouse_queue.push(MouseEv::Click(p, true)),
                        MouseButton::Right => self.mouse_queue.push(MouseEv::Click(p, false)),
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
                        self.mouse_queue.push(MouseEv::Wheel(true));
                    } else if dy < 0.0 {
                        self.mouse_queue.push(MouseEv::Wheel(false));
                    }
                } else {
                    let dy=match delta {MouseScrollDelta::LineDelta(_,y)=>y,MouseScrollDelta::PixelDelta(p)=>(p.y/30.)as f32};
                    if dy!=0. {gba_core::ffta_shop_mouse::scroll(&mut self.machine.bus,dy>0.);}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.benchmark.is_some() && self.window.is_some() {
            self.benchmark_tick(event_loop); return;
        }
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
        let capturing_now = self.paused && self.menu.capturing().is_some();
        let toggle_edge = !capturing_now && nav.open && !self.nav_prev.open;
        if toggle_edge {
            if !self.menu_enabled {
                event_loop.exit();
                return;
            }
            self.paused = !self.paused;
            self.fps.reset(Instant::now());
            self.profile.presentation_boundary();
            if self.paused {
                // End the current audio timeline immediately. Keep the user's
                // Fast latch; the effective speed is resolved again on resume.
                self.set_fast_forward(false, 1);
                self.game_mouse.cancel();self.machine.bus.ffta_battle_mouse.cancel();
                self.menu.reset();
                self.refresh_state_slots();
                self.menu_dirty = true;
                // Drop stale pointer events from before the menu opened.
                self.mouse_queue.clear();
            }
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
                        let ev = if up {
                            menu::MenuInput::Up
                        } else {
                            menu::MenuInput::Down
                        };
                        if let Some(a) = self.menu.handle(ev, &mut self.av) {
                            acts.push(a);
                        }
                    }
                }
            }
            for action in acts {
                match action {
                    menu::MenuAction::Resume => self.paused = false,
                    menu::MenuAction::SaveState { slot, overwrite } => self.state_operation(true, slot, overwrite),
                    menu::MenuAction::LoadState(slot) => self.state_operation(false, slot, false),
                    menu::MenuAction::UndoState => self.undo_state_operation(),
                    menu::MenuAction::RestoreBackup(id) => self.restore_save_backup(id),
                    menu::MenuAction::Quit => quit = true,
                    menu::MenuAction::SwitchRom => {
                        if self.choose_rom() {quit=true;break;}
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
                        if !matches!(what, menu::Changed::Overlay | menu::Changed::Audio | menu::Changed::Fullscreen) {
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
                self.reset_audio_counters();
                self.fps.reset(Instant::now());
                self.profile.presentation_boundary();
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
            for i in 0..2 {if let Some(layout)=self.ui_layout((mw as u32,mh as u32),i) {
                let mut canvas=std::mem::take(&mut self.pause_rgba);
                let ui=self.ui_frame(layout,i);
                for y in 0..mh {for x in 0..mw {
                    if let Some(c)=ui.sample(x as f32+0.5,y as f32+0.5) {canvas[y*mw+x]=c;}
                }}
                self.pause_rgba=canvas;
            }}
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
        let ff_speed = input_config::fast_forward_speed(self.av.ff_speed, self.ff_latched, ff_held);
        let ff_now = ff_speed > 1;
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
            self.ff_speed
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
                    (st.mixed.len() / 2).min(st.psg.len() / 2)
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
            if (st.mixed.len() / 2).min(st.psg.len() / 2) >= RING_TARGET {
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

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
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
            }
        }
    }
}

/// Interactive play: resolve the image/BIOS/translation, set up audio, input,
/// and screen simulation, then run the winit event loop until the window
/// closes. Teardown (final save, fallback census, label recording) happens
/// in the final event-loop callback, before its display connection is dropped.
pub fn cmd_play(args: &[String]) -> Result<(), String> {
    let mut rom_path = None;
    let mut interp_only = false;
    // Opt-in decoded resources until the equivalence/cost gates pass.
    let mut prepare_rom_data = false;
    // Machine-readable lifecycle lines on stdout for a supervising frontend
    // (the launcher): `STATUS building <pct>` / `STATUS playing`.
    let mut status = false;
    // Perf instrumentation is developer tooling: always on in debug builds,
    // opt-in (--stats or GBA_RECOMP_STATS=1) in release.
    let mut show_stats = cfg!(debug_assertions) || std::env::var_os("GBA_RECOMP_STATS").is_some();
    let mut bios_arg: Option<String> = None;
    let mut no_bios = false;
    let mut gamedb_arg: Option<String> = None;
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
            "--gamedb" => gamedb_arg = Some(it.next().ok_or("--gamedb needs a value")?.to_string()),
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    if benchmark.is_some() && std::env::var_os("RECOMP_RECORD_INPUT").is_some() {
        return Err("--benchmark cannot record input".into());
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
    if let Some(prepared) = prepared_resources {
        crate::rom_data::attach_from_directory(&mut machine, &prepared.directory)?;
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

    let title = String::from("FFTARecomp");

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
    let state_slots = crate::save_state::Slots::new(Path::new(&sav_path), &machine.bus.rom);
    let game_mouse=gba_core::ffta_mouse::LanguageMouse::new(&machine.bus.rom,icfg.mouse_keyboard || std::env::var("FFTA_MOUSE").as_deref()==Ok("1"));
    if icfg.mouse_keyboard {machine.bus.ffta_battle_mouse.set_enabled(true);}
    let boot_state = machine.save_state()?;
    let benchmark_zoom = benchmark.as_ref().map_or(0, |b| b.zoom);
    if benchmark_zoom > zoom_max { return Err("benchmark zoom exceeds the selected aspect's limit".into()); }
    if let Some(benchmark) = benchmark.as_mut() { benchmark.restore(&mut machine)?; }
    let mut app = PlayApp {
        machine,
        sav_path,
        state_slots,
        save_backups,
        boot_state,
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
        menu: menu::Menu::new(true, false),
        paused: false,
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
        ff_speed: 4,
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
        for _ in 0..7 {
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
