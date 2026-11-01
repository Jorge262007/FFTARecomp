//! In-game pause/settings overlay for the play runtime.
//!
//! This crate owns the overlay's state machine, embedded scalable
//! fonts, and rgba compositing — everything the play loop needs to draw a
//! settings panel over the last composed frame and steer it with the same
//! keyboard/gamepad poll it already runs. It deliberately knows nothing
//! about windows, audio rings, or the emulator: the play loop feeds it
//! edge-triggered input and the live [`AvConfig`], and it returns the
//! actions to take (resume, quit, a setting changed) and draws the panel.
//!
//! The settings vocabulary mirrors the launcher's A/V surface, parsed
//! through the `screen` crate's enums so the two frontends never drift.

mod canvas;
pub mod i18n;
pub mod diagnostics;
mod font;
pub mod fps;
pub mod event_console;
pub mod command_console;
mod controller_icons;
mod menu_icons;
mod theme;
pub mod clan_atlas;
pub mod shortcuts;
mod atlas_theme;
mod save_states;
mod save_forms;
mod game_saves;
mod save_backups;
mod debug_catalog;
mod content_selector;
mod tool_inputs;
mod debug_tool;
mod selection;
mod information;
mod interaction;
mod controls;
mod debug_unlock;
pub use debug_tool::{DebugTool,ToolRequest,ToolField,ToolFieldKind};
pub use tool_inputs::ContentSlot;
pub use content_selector::{ContentEntry,ContentViewPreferences,ContentHistory};
pub use debug_catalog::CatalogEntry;
pub use save_backups::BackupInfo;
pub use save_states::StateSlot;
pub use game_saves::GameSaveSlot;

use input_config::{AvConfig, Button, Device, GamepadLayout, InputConfig, Language};
use screen::{ResponseMode, ScreenKind};
use std::ops::{Deref, DerefMut};

/// One edge-triggered navigation event. The play loop debounces its own
/// keyboard/gamepad poll into these (a held direction yields one event on
/// the press edge, never a stream), so the menu logic stays stateless
/// about input timing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuInput {
    Up,
    Down,
    Left,
    Right,
    /// Activate the selected row (Enter / South).
    Confirm,
    /// Back out / close the menu (Escape / East).
    Cancel,
}

/// What the play loop should do after feeding input to the menu.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    /// Toggle the existing session fast-forward latch without changing the preference.
    ToggleFastForward,
    /// Close the menu and continue the game.
    Resume,
    /// Reboot the selected cartridge without leaving this process.
    RestartRom,
    /// Close the game (the .sav is flushed by the play loop's teardown).
    Quit,
    /// Close only the game process so the launcher can select another ROM.
    SwitchRom,
    /// A persisted setting changed; the loop reapplies live state and
    /// writes `av.cfg`. The variant says which surface moved so the loop
    /// only rebuilds what it must.
    Changed(Changed),
    InputDevice(Device),
    ResetGamepad,
    SaveState { slot: u8, overwrite: bool },
    LoadState(u8),
    UndoState,
    CaptureGraphics,
    CreateIssueReport,
    StartCodeTrace,
    SaveCodeReport,
    StartGameplayTrace,
    SaveGameplayReport,
    StartRecentEvents,
    SaveRecentEvents,
    StartAudioCapture,
    SaveAudioReport,
    StartUiTrace,
    SaveUiReport,
    ToggleEventConsole,
    OpenCommandConsole,
    InspectPC,
    DebugStep(u32),
    DebugResume,
    CompareReplay,
    CaptureReplay,
    AuditReplay,
    ReplayCase,
    ChooseReplayProfile,
    CancelReplay,
    StartInputRecording,
    StopInputRecording,
    CopyDiagnosis,
    StartDebugSession,
    CloseDebugSession,
    ContentOpen,
    ChooseToolInput(ContentSlot),
    OpenDebugTool(DebugTool),
    ToolRequest(ToolRequest),
    ContentRefresh,
    ContentBrowseFile,
    ContentAddFolder,
    ContentSelect { index: usize, generation: u64 },
    ContentPreview { index: usize, generation: u64 },
    ContentCancel,
    OpenDebugCatalog,
    OpenCheatCatalog,
    CheatChoose,
    CheatReload,
    ChooseDebugCatalog,
    CatalogSelection(usize),
    CatalogImport(usize),
    CatalogOpen(usize),
    RestoreBackup(u64),
    SwitchGameSave(u8),
    CopyGameSave { slot: u8, overwrite: bool },
}

/// Which A/V surface a [`MenuAction::Changed`] touched, so the play loop
/// rebuilds only the affected objects.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Changed {
    /// Debug save tools; refresh slot metadata without rebuilding A/V.
    Debug,
    /// `audio_enhanced` flipped.
    Audio,
    /// A video setting that rebuilds the color LUT / temporal / grid in
    /// place (screen model, darken, response, grid).
    Video,
    /// Persist-only / session-only change (fast-forward speed): nothing to
    /// rebuild live. The name is historical.
    Gamut,
    /// `video_vsync` flipped; the loop retunes the presenter in place.
    Vsync,
    /// Borderless fullscreen; shared with F11, without rebuilding game/video state.
    Fullscreen,
    /// Persisted FPS overlay setting; no video or audio rebuild needed.
    Overlay,
}

/// What a pending "press a key" capture will bind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bind {
    Menu,
    GamepadMenu,
    /// A GBA pad button.
    Button(Button),
    Gamepad(Button),
    /// The hold-to-fast-forward key.
    FastForward,
    ZoomIn,
    FastForwardToggle,
    ZoomOut,
    GamepadZoomIn,
    GamepadZoomOut,
    GamepadFastForward,
    GamepadFastForwardToggle,
}

/// Which page of the menu is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Page {
    Information,
    CliReference,
    Saves,
    System,
    Debug,
    DebugCatalog,
    CheatCatalog,
    ContentSelector,
    ToolInputs,
    DebugTools,
    ScriptLibrary,
    DebugTool,
    GameSaves,
    SwitchGameSave,
    ReplaceGameSave,
    SaveStates,
    ReplaceState,
    SaveBackups,
    RestoreBackup,
    Main,
    Exit,
    SwitchRom,
    FastForward,
    Audio,
    Video,
    TextBubbles,
    Inputs,
    Gamepad,
    MouseKeyboard,
}

/// The rows of the menu. Action rows (`Resume`, `Quit`, the submenu
/// entries, `Back`, `Key`) have no value to cycle; setting rows cycle on
/// left/right/confirm. `Key(b)` starts a "press a key" capture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row {
    CliReference,
    FastForwardToggle,
    CreateIssueReport,
    StartCodeTrace,
    SaveCodeReport,
    StartGameplayTrace,
    SaveGameplayReport,
    StartRecentEvents,
    SaveRecentEvents,
    StartAudioCapture,
    SaveAudioReport,
    StartUiTrace,
    SaveUiReport,
    ToggleEventConsole,
    OpenCommandConsole,
    CaptureGraphics,
    InspectPC,
    DebugStepOne,
    DebugStepTen,
    DebugResume,
    DebugMenu,
    ExtendedSaves,
    CompareReplay,
    CaptureReplay,
    AuditReplay,
    ReplayCase,
    ChooseReplayProfile,
    CancelReplay,
    StartInputRecording,
    StopInputRecording,
    CopyDiagnosis,
    StartDebugSession,
    CloseDebugSession,
    ContentPicker,
    ToolInputsMenu,
    DebugToolsMenu,
    ScriptLibraryMenu,
    DebugTool(DebugTool),
    ToolInput(ContentSlot),
    DebugCatalog,
    CheatCatalog,
    CatalogRegion, CatalogStatus, CatalogKind, CatalogCase, CatalogEntry, CatalogImport, CatalogOpen, CatalogChoose,
    GameSavesMenu,
    GameSaveSlot,
    UseGameSave,
    CopyGameSave,
    ConfirmSwitchGameSave,
    ConfirmCopyGameSave,
    SaveStatesMenu,
    SaveBackupsMenu,
    Backup(u8),
    ConfirmRestoreBackup,
    StateSlot,
    SaveStateConfirm,
    SaveState,
    LoadState,
    UndoState,
    Menu,
    MouseMode,
    Resume,
    RestartRom,
    FastForwardMenu,
    FastForward,
    AudioMenu,
    Mute,
    FastForwardAudio,
    VideoMenu,
    TextBubblesMenu,
    BubbleSize,
    BubbleLength,
    InputsMenu,
    SwitchRomMenu,
    ResetGamepad,
    Quit,
    QuitYes,
    QuitNo,
    ReturnToGame,
    ChangeRom,
    Volume,
    Audio,
    Screen,
    Brightness,
    MenuTransparency,
    Language,
    Darken,
    Response,
    Grid,
    Scaling,
    Aspect,
    Vsync,
    Fullscreen,
    Fps,
    Key(Button),
    FfKey,
    FfToggleKey,
    ZoomIn,
    ZoomOut,
    Back,
}

const MAIN_ROWS: [Row; 12] = [
    Row::Resume,
    Row::SaveStatesMenu,
    Row::SaveBackupsMenu,
    Row::FastForwardMenu,
    Row::TextBubblesMenu,
    Row::AudioMenu,
    Row::VideoMenu,
    Row::InputsMenu,
    Row::RestartRom,
    Row::SwitchRomMenu,
    Row::DebugMenu,
    Row::Quit,
];

const AUDIO_ROWS: [Row; 4] = [Row::Volume, Row::Mute, Row::Audio, Row::Back];
const EXIT_ROWS: [Row; 2] = [Row::QuitYes, Row::QuitNo];
const SWITCH_ROM_ROWS: [Row; 2] = [Row::ReturnToGame, Row::ChangeRom];
const FAST_FORWARD_ROWS: [Row; 4] = [Row::FastForwardToggle, Row::FastForward, Row::FastForwardAudio, Row::Back];

/// Scaling modes offered by the menu (`av.scaling`): "off" keeps the
/// original exact area-average scaler.
const SCALING_MODES: [&str; 4] = ["off", "nearest", "sharp", "epx"];

/// Aspect ratios offered by the menu (`av.video_aspect`). Applied on resume.
const ASPECT_MODES: [&str; 7] = ["auto", "3:2", "16:10", "16:9", "20:9", "21:9", "32:9"];

const VIDEO_ROWS: [Row; 12] = [
    Row::Aspect,
    Row::Fullscreen,
    Row::Screen,
    Row::Brightness,
    Row::Darken,
    Row::Response,
    Row::Grid,
    Row::Scaling,
    Row::Vsync,
    Row::Fps,
    Row::MenuTransparency,
    Row::Back,
];

const INPUT_ROWS: [Row; 16] = [
    Row::Menu,
    Row::Key(Button::A),
    Row::Key(Button::B),
    Row::Key(Button::L),
    Row::Key(Button::R),
    Row::Key(Button::Start),
    Row::Key(Button::Select),
    Row::Key(Button::Up),
    Row::Key(Button::Down),
    Row::Key(Button::Left),
    Row::Key(Button::Right),
    Row::FfKey,
    Row::FfToggleKey,
    Row::ZoomIn,
    Row::ZoomOut,
    Row::Back,
];
const MOUSE_ROWS: [Row;17] = [
    Row::MouseMode,Row::Menu,Row::Key(Button::A),Row::Key(Button::B),Row::Key(Button::L),Row::Key(Button::R),
    Row::Key(Button::Start),Row::Key(Button::Select),Row::Key(Button::Up),Row::Key(Button::Down),
    Row::Key(Button::Left),Row::Key(Button::Right),Row::FfKey,Row::FfToggleKey,Row::ZoomIn,Row::ZoomOut,Row::Back,
];
const GAMEPAD_ROWS: [Row; 17] = [
    Row::Menu,
    Row::Key(Button::A),
    Row::Key(Button::B),
    Row::Key(Button::L),
    Row::Key(Button::R),
    Row::Key(Button::Start),
    Row::Key(Button::Select),
    Row::Key(Button::Up),
    Row::Key(Button::Down),
    Row::Key(Button::Left),
    Row::Key(Button::Right),
    Row::FfKey,
    Row::FfToggleKey,
    Row::ZoomIn,
    Row::ZoomOut,
    Row::ResetGamepad,
    Row::Back,
];

fn rows_of(page: Page) -> &'static [Row] {
    match page {
        Page::Information => &[Row::Back],
        Page::CliReference => &[Row::Back],
        Page::Saves => clan_atlas::SAVES_ROWS,
        Page::System => clan_atlas::SYSTEM_ROWS,
        Page::Debug => &[Row::ContentPicker, Row::ToolInputsMenu, Row::DebugToolsMenu, Row::ScriptLibraryMenu, Row::ExtendedSaves, Row::GameSavesMenu, Row::CopyDiagnosis, Row::StartDebugSession, Row::CloseDebugSession, Row::DebugCatalog, Row::CheatCatalog, Row::StartInputRecording, Row::StopInputRecording, Row::ReplayCase, Row::ChooseReplayProfile, Row::CancelReplay, Row::AuditReplay, Row::CaptureReplay, Row::CompareReplay, Row::DebugStepOne, Row::DebugStepTen, Row::DebugResume, Row::InspectPC, Row::CaptureGraphics, Row::StartUiTrace, Row::SaveUiReport, Row::ToggleEventConsole, Row::OpenCommandConsole, Row::StartAudioCapture, Row::SaveAudioReport, Row::StartCodeTrace, Row::SaveCodeReport, Row::CreateIssueReport, Row::StartGameplayTrace, Row::SaveGameplayReport, Row::StartRecentEvents, Row::SaveRecentEvents, Row::Back],
        Page::DebugTools => &[Row::DebugTool(DebugTool::Replay),Row::DebugTool(DebugTool::Audit),Row::DebugTool(DebugTool::Capture),Row::DebugTool(DebugTool::Compare),Row::DebugTool(DebugTool::Step),Row::DebugTool(DebugTool::Inspect),Row::DebugTool(DebugTool::Audio),Row::DebugTool(DebugTool::Code),Row::DebugTool(DebugTool::Gameplay),Row::DebugTool(DebugTool::Recent),Row::DebugTool(DebugTool::Graphics),Row::DebugTool(DebugTool::Issue),Row::Back],
        Page::ScriptLibrary => &[Row::DebugTool(DebugTool::Homepage), Row::DebugTool(DebugTool::CampaignReport), Row::DebugTool(DebugTool::ArtifactPreview), Row::DebugTool(DebugTool::Recipe), Row::DebugTool(DebugTool::GuestScripts), Row::Back],
        Page::DebugTool => &[Row::Back],
        Page::ToolInputs => &[Row::ToolInput(ContentSlot::Replay), Row::ToolInput(ContentSlot::Catalog),
            Row::ToolInput(ContentSlot::Session), Row::ToolInput(ContentSlot::Recording),
            Row::ToolInput(ContentSlot::Snapshot), Row::ToolInput(ContentSlot::Input),
            Row::ToolInput(ContentSlot::Dataset), Row::ToolInput(ContentSlot::RuntimeA),
            Row::ToolInput(ContentSlot::RuntimeB), Row::ToolInput(ContentSlot::Library),
            Row::ToolInput(ContentSlot::Labels), Row::ToolInput(ContentSlot::KnownPlan), Row::ToolInput(ContentSlot::HostTool), Row::ToolInput(ContentSlot::HostRuntime), Row::ToolInput(ContentSlot::Campaign), Row::ToolInput(ContentSlot::Recipe), Row::ToolInput(ContentSlot::GuestScripts), Row::Back],
        Page::ContentSelector => &[Row::Back],
        Page::DebugCatalog => &[Row::CatalogRegion,Row::CatalogStatus,Row::CatalogKind,Row::CatalogCase,Row::CatalogEntry,Row::CatalogImport,Row::CatalogOpen,Row::CatalogChoose,Row::Back],
        Page::CheatCatalog => &[Row::Back],
        Page::GameSaves => &[Row::GameSaveSlot, Row::UseGameSave, Row::CopyGameSave, Row::Back],
        Page::SwitchGameSave => &[Row::ConfirmSwitchGameSave, Row::Back],
        Page::ReplaceGameSave => &[Row::ConfirmCopyGameSave, Row::Back],
        Page::Main => &MAIN_ROWS,
        Page::SaveStates => &[Row::StateSlot, Row::SaveState, Row::LoadState, Row::UndoState, Row::Back],
        Page::ReplaceState => &[Row::SaveStateConfirm, Row::Back],
        Page::SaveBackups => &[Row::Backup(0),Row::Backup(1),Row::Back],
        Page::RestoreBackup => &[Row::ConfirmRestoreBackup,Row::Back],
        Page::Exit => &EXIT_ROWS,
        Page::SwitchRom => &SWITCH_ROM_ROWS,
        Page::FastForward => &FAST_FORWARD_ROWS,
        Page::Audio => &AUDIO_ROWS,
        Page::Video => &VIDEO_ROWS,
        Page::TextBubbles => &[Row::BubbleSize, Row::BubbleLength, Row::Back],
        Page::Inputs => &INPUT_ROWS,
        Page::MouseKeyboard => &MOUSE_ROWS,
        Page::Gamepad => &GAMEPAD_ROWS,
    }
}

/// Fast-forward multipliers offered by the menu.
const FF_SPEEDS: [u32; 36] = input_config::fast_forward::PERCENTS;

impl Row {
    fn localized_label(self, language: Language) -> &'static str {
        let key = self.label();
        if matches!(self, Row::Key(Button::A | Button::B | Button::L | Button::R | Button::Start | Button::Select)) {
            key
        } else {
            i18n::text(language, key)
        }
    }

    fn label(self) -> &'static str {
        match self {
            Row::CliReference => "Riferimenti Menu / CLI",
            Row::FastForwardToggle => "Fast Forward ON/OFF",
            Row::Language => "Language",
            Row::MenuTransparency => "Trasparenza menu",
            Row::DebugMenu => "Debug",
            Row::ToolInputsMenu => "Tool inputs / current selections",
            Row::DebugToolsMenu => "Parameters / prepare profiles",
            Row::ScriptLibraryMenu => "Scripts and tools",
            Row::DebugTool(tool) => tool.label(),
            Row::ToolInput(slot) => slot.label(),
            Row::ExtendedSaves => "Extended saves",
            Row::CaptureGraphics => "Capture graphics report",
            Row::CreateIssueReport => "Create issue report",
            Row::StartCodeTrace => "Start code trace",
            Row::SaveCodeReport => "Save code report",
            Row::StartGameplayTrace => "Start task / script trace",
            Row::SaveGameplayReport => "Save gameplay report",
            Row::StartRecentEvents => "Start recent events",
            Row::SaveRecentEvents => "Freeze / save recent events",
            Row::StartAudioCapture => "Start audio capture",
            Row::SaveAudioReport => "Save audio report",
            Row::StartUiTrace => "Start UI / input trace",
            Row::ToggleEventConsole => "Console eventi",
            Row::OpenCommandConsole => "Console comandi",
            Row::SaveUiReport => "Save UI / input report",
            Row::InspectPC => "Inspect PC / registers",
            Row::DebugStepOne => "Step 1 frame (F7)",
            Row::DebugStepTen => "Step 10 frames (F8)",
            Row::DebugResume => "Resume (F6)",
            Row::CompareReplay => "Compare first difference",
            Row::CaptureReplay => "Capture on trigger",
            Row::AuditReplay => "Audit replay",
            Row::ReplayCase => "Replay case",
            Row::ChooseReplayProfile => "Choose replay profile",
            Row::CancelReplay => "Cancel replay",
            Row::StartInputRecording => "Start input recording",
            Row::StopInputRecording => "Stop input recording",
            Row::CopyDiagnosis => "Copy diagnosis",
            Row::StartDebugSession => "Start debug session",
            Row::CloseDebugSession => "Close debug session",
            Row::ContentPicker => "Content selector",
            Row::DebugCatalog => "Debug catalog",
            Row::CheatCatalog => "Catalogo cheat (sola lettura)",
            Row::CatalogRegion => "Region", Row::CatalogStatus => "Recorded status",
            Row::CatalogKind => "Kind", Row::CatalogCase => "DS case",
            Row::CatalogEntry => "Entry", Row::CatalogImport => "Import to free snapshot slot",
            Row::CatalogOpen => "Open isolated case", Row::CatalogChoose => "Choose / reload catalog",
            Row::GameSavesMenu => "Game saves",
            Row::GameSaveSlot => "Slot",
            Row::UseGameSave | Row::ConfirmSwitchGameSave => "Use and restart",
            Row::CopyGameSave => "Copy current save",
            Row::ConfirmCopyGameSave => "Replace game save",
            Row::Menu => "Open menu",
            Row::MouseMode => "Mouse & Keyboard",
            Row::Resume => "Resume",
            Row::RestartRom => "Restart ROM",
            Row::SaveStatesMenu => "Save states",
            Row::SaveBackupsMenu => "Save backups",
            Row::Backup(0) => "Copy 1 - latest",
            Row::Backup(_) => "Copy 2 - previous",
            Row::ConfirmRestoreBackup => "Restore and restart",
            Row::StateSlot => "Slot",
            Row::SaveStateConfirm => "Replace state",
            Row::SaveState => "Save state",
            Row::LoadState => "Load state",
            Row::UndoState => "Undo last operation",
            Row::FastForwardMenu => "Fast forward",
            Row::FastForward => "Multiplier",
            Row::FastForwardAudio => "Audio mode",
            Row::AudioMenu => "Audio",
            Row::VideoMenu => "Video",
            Row::TextBubblesMenu => "Text Bubbles",
            Row::BubbleSize => "Size",
            Row::BubbleLength => "Length",
            Row::InputsMenu => "Input",
            Row::SwitchRomMenu => "Switch ROM",
            Row::ResetGamepad => "Default layout",
            Row::Quit => "Exit",
            Row::QuitYes => "Yes, exit",
            Row::QuitNo => "No, go back",
            Row::ReturnToGame => "Return to game",
            Row::ChangeRom => "Choose ROM...",
            Row::Volume => "Volume",
            Row::Fullscreen => "Fullscreen (F11)",
            Row::Mute => "Mute",
            Row::Audio => "Enhanced Audio",
            Row::Screen => "Screen",
            Row::Brightness => "Brightness",
            Row::Darken => "Darken",
            Row::Response => "Response",
            Row::Grid => "Grid",
            Row::Scaling => "Scaling",
            Row::Aspect => "Aspect",
            Row::Vsync => "Vsync",
            Row::Fps => "FPS counter",
            Row::Key(Button::A) => "A",
            Row::Key(Button::B) => "B",
            Row::Key(Button::L) => "L",
            Row::Key(Button::R) => "R",
            Row::Key(Button::Start) => "Start",
            Row::Key(Button::Select) => "Select",
            Row::Key(Button::Up) => "Up",
            Row::Key(Button::Down) => "Down",
            Row::Key(Button::Left) => "Left",
            Row::Key(Button::Right) => "Right",
            Row::FfKey => "Fast Fwd",
            Row::FfToggleKey => "Toggle Fast Fwd",
            Row::ZoomIn => "Zoom In",
            Row::ZoomOut => "Zoom Out",
            Row::Back => "Back",
        }
    }

    /// The submenu this row opens, if any.
    fn opens(self) -> Option<Page> {
        match self {
            Row::DebugMenu => Some(Page::Debug),
            Row::ToolInputsMenu => Some(Page::ToolInputs),
            Row::DebugToolsMenu => Some(Page::DebugTools),
            Row::ScriptLibraryMenu => Some(Page::ScriptLibrary),
            Row::SaveStatesMenu => Some(Page::SaveStates),
            Row::SaveBackupsMenu => Some(Page::SaveBackups),
            Row::Quit => Some(Page::Exit),
            Row::SwitchRomMenu => Some(Page::SwitchRom),
            Row::FastForwardMenu => Some(Page::FastForward),
            Row::AudioMenu => Some(Page::Audio),
            Row::VideoMenu => Some(Page::Video),
            Row::TextBubblesMenu => Some(Page::TextBubbles),
            Row::InputsMenu => Some(Page::Inputs),
            _ => None,
        }
    }
}

/// Panel geometry in frame pixels, shared by drawing and mouse hit-testing
/// so the two can never disagree.
#[derive(Clone, Copy)]
struct Layout {
    panel_x: usize,
    panel_y: usize,
    panel_w: usize,
    panel_h: usize,
    pad: usize,
    line_h: usize,
    /// Text-top y of the first row (its highlight bar starts 2 px above).
    rows_y: usize,
    visible_rows: usize,
}

/// Responsive panel geometry in UI pixels, independent of the game resolution.
#[cfg(test)]
fn layout(n: usize, w: usize, h: usize) -> Layout { layout_kind(n,w,h,n==MAIN_ROWS.len()) }
fn layout_kind(n: usize, w: usize, h: usize, main: bool) -> Layout {
    let inputs = n == INPUT_ROWS.len() || n == GAMEPAD_ROWS.len();
    let scale = (h as f32 / 480.).min(w as f32 / 560.).max(0.28);
    let pad = (20. * scale).round() as usize;
    let panel_w = w
        .saturating_sub(pad * 2)
        .min(((if main { 320. } else { 512. }) * scale) as usize);
    let panel_x = w.saturating_sub(panel_w) / 2;
    let avail = h
        .saturating_sub(pad * 2)
        .min(if inputs { (460. * scale) as usize } else { h });
    let line_h = ((if main { 33. } else { 44. }) * scale).round().max(12.) as usize;
    let header = if main {
        0
    } else if inputs {
        (82. * scale) as usize
    } else if n == 2 {
        (90. * scale) as usize
    } else {
        (50. * scale) as usize
    };
    let footer = if inputs { (50. * scale) as usize } else { 0 };
    let visible_rows = ((avail.saturating_sub(pad * 2 + header + footer)) / line_h)
        .max(1)
        .min(if inputs { n - 1 } else { n });
    let panel_h = (pad * 2 + header + visible_rows * line_h + footer).min(avail);
    let panel_y = h.saturating_sub(panel_h) / 2;
    let rows_y = panel_y + pad + header;
    Layout {
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        pad,
        line_h,
        rows_y,
        visible_rows,
    }
}

/// One navigation point saved on the submenu stack.
#[derive(Clone, Copy)]
struct PageFrame {
    page: Page,
    selected: usize,
    scroll: usize,
}

/// Navigation model. Page history and the selected row live together so
/// opening, switching tabs, and returning cannot leave a stale cursor behind.
#[doc(hidden)]
pub struct Navigation {
    page: Page,
    selected: usize,
    parents: Vec<PageFrame>,
}

impl Navigation {
    fn new() -> Self {
        Self {
            page: Page::Main,
            selected: 0,
            parents: Vec::new(),
        }
    }

    fn open(&mut self, page: Page) {
        if self.page != page {
            self.parents.push(PageFrame {
                page: self.page,
                selected: self.selected,
                scroll: 0,
            });
        }
        self.page = page;
        self.selected = if page == Page::Exit { 1 } else { 0 };
    }

    fn switch_input_tab(&mut self, page: Page) {
        debug_assert!(matches!(page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard));
        if matches!(self.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard) {
            self.page = page;
        } else {
            self.open(page);
        }
    }



    fn reset(&mut self) {
        self.page = Page::Main;
        self.selected = 0;
        self.parents.clear();
    }
}

/// The pause-overlay model and its public input/draw boundary.
pub struct Menu {
    language: std::cell::Cell<Language>,
    atlas: clan_atlas::State,
    navigation: Navigation,
    /// Whether the enhanced-audio toggle applies live or on next launch.
    /// The play loop reports this once at construction; the menu only
    /// surfaces it as a footnote on that row.
    audio_live: bool,
    /// `Some(button)` while waiting for the player to press a key for it.
    /// The play loop owns the keyboard: it reads [`Menu::capturing`], and
    /// calls [`Menu::end_capture`] once it has taken (or cancelled) the key.
    capture: Option<Bind>,
    scroll: std::cell::Cell<usize>,
    dropdown: Option<Dropdown>,
    viewport: std::cell::Cell<(usize, usize)>,
    input_tabs_focus: Option<u8>,
    input_gamepad: std::cell::Cell<u8>,
    hint_device: std::cell::Cell<Option<Device>>,
    debug_enabled: std::cell::Cell<bool>,
    unlock_progress: usize,
    mouse_mode: std::cell::Cell<bool>,
    controller_layout: std::cell::Cell<GamepadLayout>,
    menu_binding: std::cell::RefCell<String>,
    state_slot: usize,
    state_slots: Vec<StateSlot>,
    state_times: Vec<Option<u64>>,
    game_save_slot: usize,
    game_save_current: usize,
    game_save_slots: Vec<GameSaveSlot>,
    game_save_message: String,
    debug_message: String,
    debug_session: bool,
    debug_position: Vec<String>,
    catalog: debug_catalog::CatalogState,
    content: content_selector::Selector,
    cheats: content_selector::Selector,
    tool_inputs: Vec<(ContentSlot,String)>,
    tool: debug_tool::ToolForm,
    state_message: String,
    state_undo: Option<String>,
    state_undo_description: Option<(&'static str, u8)>,
    save_backups: [BackupInfo;2],
    backup_selection: Option<(u64,u8)>,
    backup_message: String,
    session_message: String,
}

// Keep field access concise inside the renderer/tests while making
// Navigation the sole owner of page/cursor state.
impl Deref for Menu {
    type Target = Navigation;
    fn deref(&self) -> &Self::Target {
        &self.navigation
    }
}

impl DerefMut for Menu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.navigation
    }
}
struct Dropdown {
    row: Row,
    index: usize,
    first: std::cell::Cell<usize>,
    options: Vec<String>,
}

impl Menu {
    /// Open a fresh menu with the first row selected. `audio_live` tells
    /// the menu whether enhanced audio takes effect immediately. The second
    /// argument is kept for source compatibility (the gamut row now lives
    /// in the launcher) and is ignored.
    pub fn new(audio_live: bool, _gamut_live: bool) -> Menu {
        Menu {
            language: std::cell::Cell::new(Language::default()),
            navigation: Navigation::new(),
            atlas: clan_atlas::State::default(),
            audio_live,
            capture: None,
            scroll: std::cell::Cell::new(0),
            dropdown: None,
            viewport: std::cell::Cell::new((854, 480)),
            input_tabs_focus: None,
            input_gamepad: std::cell::Cell::new(0),
            hint_device: std::cell::Cell::new(None),
            debug_enabled: std::cell::Cell::new(false),
            unlock_progress: 0,
            mouse_mode: std::cell::Cell::new(false),
            controller_layout: std::cell::Cell::new(GamepadLayout::Xbox),
            menu_binding: std::cell::RefCell::new("Escape".into()),
            state_slot: 0,
            state_slots: vec![StateSlot::default(); 9],
            state_times:vec![None;9],
            game_save_slot: 0,
            game_save_current: 0,
            game_save_slots: Vec::new(),
            game_save_message: String::new(),
            debug_message: String::new(),
            debug_session: false,
            debug_position: Vec::new(),
            catalog: debug_catalog::CatalogState::default(),
            content: content_selector::Selector::default(),
            cheats: {let mut selector=content_selector::Selector::default();selector.cheat_mode=true;selector},
            tool_inputs: Vec::new(),
            tool: debug_tool::ToolForm::default(),
            state_message: String::new(),
            state_undo: None,
            state_undo_description: None,
            save_backups: std::array::from_fn(|_| BackupInfo::default()),
            backup_selection: None,
            backup_message: String::new(),
            session_message: String::new(),
        }
    }

    /// Reset to the top of the main page (called on each open so the cursor
    /// doesn't persist a stale position across pauses).
    pub fn reset(&mut self) {
        self.cancel_debug_unlock();
        let root=clan_atlas::root_page(self.atlas.domain,self.atlas.selected_section());
        let selected=if self.page==root {self.selected}else{0};
        self.navigation.page=root;self.navigation.selected=selected.min(self.rows().len()-1);self.navigation.parents.clear();
        self.atlas.sidebar_focus=false;
        self.content.finish_search(true);
        self.cheats.finish_search(true);
        self.state_message.clear();
        self.backup_message.clear();
        self.session_message.clear();
        self.backup_selection = None;
        self.capture = None;
        self.scroll.set(0);
        self.dropdown = None;
        self.input_tabs_focus = None;
    }

    pub fn global_shortcut(&mut self,shortcut:shortcuts::Shortcut,av:&mut AvConfig)->Option<MenuAction> {
        if self.capture.is_some() || self.content_text_focus(){return None;}
        use shortcuts::Shortcut::*;
        match shortcut {VolumeDown=>cycle(Row::Volume,av,false).map(MenuAction::Changed),VolumeUp=>cycle(Row::Volume,av,true).map(MenuAction::Changed),Mute=>cycle(Row::Mute,av,true).map(MenuAction::Changed),SpeedDown=>cycle(Row::FastForward,av,false).map(MenuAction::Changed),SpeedUp=>cycle(Row::FastForward,av,true).map(MenuAction::Changed),FastToggle=>Some(MenuAction::ToggleFastForward)}
    }
    /// Move between the permanent sidebar and content without executing a row.
    pub fn escape_goes_back(&self)->bool {self.page!=Page::Main || self.dropdown.is_some() || self.atlas.sidebar_focus}
    pub fn focus_next(&mut self) {self.cancel_debug_unlock();if self.capture.is_none() && self.dropdown.is_none() && !self.content_text_focus() {self.input_tabs_focus=None;self.atlas.sidebar_focus=!self.atlas.sidebar_focus;}}
    pub fn atlas_section(&mut self, domain:clan_atlas::Domain, section:usize) {
        if section>=8 || self.capture.is_some() || (domain==clan_atlas::Domain::Debug && !self.debug_enabled()) {return;}
        self.input_tabs_focus=None;
        let old=self.atlas.index();let prior=self.atlas.selected_section();
        self.atlas.saved[old][prior]=Some(clan_atlas::Saved{page:self.page,selected:self.selected,scroll:self.scroll.get(),parents:self.navigation.parents.clone()});
        self.atlas.domain=domain;let i=self.atlas.index();self.atlas.section[i]=section;self.atlas.nav_cursor=section+1;
        self.navigation.parents.clear();
        if let Some(saved)=self.atlas.saved[i][section].clone() {self.navigation.parents=saved.parents;self.navigation.page=saved.page;self.navigation.selected=saved.selected;self.scroll.set(saved.scroll);}
        else {self.navigation.page=clan_atlas::root_page(domain,section);self.navigation.selected=0;self.scroll.set(0);}
        self.dropdown=None;self.atlas.planned=false;self.atlas.detail_scroll=0;self.atlas.reference_cursor=0;self.atlas.planned_cursor=0;self.atlas.reference_scroll.set(0);
    }
    /// Frontend-supplied values; absence remains visible rather than fabricated.
    pub fn set_operation_references(&mut self,operations:Vec<clan_atlas::OperationReference>){self.atlas.operations=operations;}
    pub fn set_live_status(&mut self,frame:u64,fast_active:bool,effective_percent:u32) {
        self.atlas.frame=Some(frame);self.atlas.fast_active=fast_active;self.atlas.fast_effective=effective_percent;
    }
    /// Slot metadata is read only while opening the menu or after an operation.
    pub fn set_state_slots(&mut self, slots: Vec<StateSlot>) {
        assert!(matches!(slots.len(), 9 | 99));
        self.state_slot = self.state_slot.min(slots.len() - 1);
        self.state_slots = slots;
        self.debug_message.clear();
    }

    pub fn set_game_saves(&mut self, slots: Vec<GameSaveSlot>) {
        assert_eq!(slots.len(), 99);
        let current = slots.iter().position(|s| s.current).unwrap_or(0);
        if self.game_save_slots.is_empty() || current != self.game_save_current {
            self.game_save_slot = current;
        }
        self.game_save_current = current;
        self.game_save_slots = slots;
    }

    pub fn game_save_result(&mut self, message: String) { self.game_save_message = message; }

    pub fn diagnosis_result(&mut self, message: String) { self.debug_message = message; }
    pub fn diagnosis_message(&self)->&str{&self.debug_message}

    pub fn set_debug_session(&mut self, active: bool) { self.debug_session = active; self.content.debug_copy=active;self.cheats.debug_copy=active;self.tool.debug_copy=active; }
    pub fn set_debug_position(&mut self, frame: u64, clock: u64, reason: &str, remaining: u32) {
        self.debug_position = vec![format!("FRAME {frame}"), format!("CLOCK {clock}"), reason.to_uppercase(), format!("REMAINING {remaining}")];
    }

    pub fn selected_state_slot(&self) -> u8 { self.state_slot as u8 + 1 }

    pub fn debug_focus(&self,w:usize,h:usize)->diagnostics::Focus {
        if let Some(index)=self.input_tabs_focus {
            let r=self.input_tab_rect(index,w,h);let b=clan_atlas::geometry(w,h).body;
            return diagnostics::Focus{page:format!("{:?}",self.page),selected:index as usize,label:format!("Dispositivo: {}",["Tastiera","Gamepad","Mouse e tastiera"][index as usize]),capture:None,panel:[b.x,b.y,b.w,b.h],first_visible:0,dropdown:None,selected_bounds:Some([r.x,r.y,r.w,r.h])};
        }
        if self.tool_active() || self.content_active() || self.cheat_active(){
            let b=clan_atlas::geometry(w,h).body;
            let mut focus=if self.tool_active(){self.tool.focus(b.w,b.h)}else if self.cheat_active(){self.cheats.focus(b.w,b.h)}else{self.content.focus(b.w,b.h)};
            focus.panel[0]+=b.x;focus.panel[1]+=b.y;
            if let Some(bounds)=focus.selected_bounds.as_mut(){bounds[0]+=b.x;bounds[1]+=b.y;}
            if let Some(drop)=focus.dropdown.as_mut(){drop.panel[0]+=b.x;drop.panel[1]+=b.y;}
            return focus;
        }
        if self.page==Page::Information{let b=clan_atlas::geometry(w,h).body;return diagnostics::Focus{page:"Information".into(),selected:0,label:"Dettagli · sola lettura".into(),capture:None,panel:[b.x,b.y,b.w,b.h],first_visible:self.atlas.detail_scroll,dropdown:None,selected_bounds:None};}
        if self.atlas_reference_active(){
            let g=clan_atlas::geometry(w,h);let(first,visible,pitch)=self.atlas_reference_first(w,h);let cursor=self.atlas_reference_cursor();
            return diagnostics::Focus{page:if self.atlas.planned{"DebugV2"}else{"CliReference"}.into(),selected:cursor,label:self.atlas_reference_items().get(cursor).map(|v|v.0.clone()).unwrap_or_default(),capture:None,panel:[g.body.x,g.body.y,g.body.w,g.body.h],first_visible:first,dropdown:None,selected_bounds:if cursor>=first && cursor<first+visible{Some([g.list.x+5,g.list.y+6+(cursor-first)*pitch,g.list.w.saturating_sub(16),pitch-3])}else{None}};
        }
        let l=self.page_layout(w,h);
        let first=self.first_visible(l);
        diagnostics::Focus {page:format!("{:?}",self.page),selected:self.selected,
            label:self.rows().get(self.selected).map(|r|r.localized_label(self.language())).unwrap_or("unknown").into(),
            capture:self.capture.map(|b|format!("{b:?}")),panel:[l.panel_x,l.panel_y,l.panel_w,l.panel_h],
            first_visible:first,dropdown:self.dropdown.as_ref().map(|dd|{
                let(x,y,w,h,_,first)=theme::dropdown_geometry(dd,w,h);
                diagnostics::DropdownFocus {label:dd.row.localized_label(self.language()).into(),index:dd.index,
                    option:dd.options.get(dd.index).cloned().unwrap_or_default(),
                    panel:[x,y,w,h],first_visible:first}
            }),selected_bounds:clan_atlas::row_rect(self,self.selected,w,h).map(|r|[r.x,r.y,r.w,r.h])}

    }

    pub fn set_catalog(&mut self, entries: Vec<CatalogEntry>, region: &str) {
        self.catalog = debug_catalog::CatalogState::new(entries,region);
        self.open(Page::DebugCatalog);
    }
    pub fn open_content(&mut self,title:String,entries:Vec<ContentEntry>,message:String,generation:u64) {
        if !self.content_active(){self.open(Page::ContentSelector);}
        self.content.set(title,entries,message,generation);
    }
    pub fn open_cheats(&mut self,entries:Vec<ContentEntry>,message:String){
        if !self.cheat_active(){self.open(Page::CheatCatalog);}
        self.cheats.set("Cheat · sola lettura".into(),entries,message,0);
    }
    pub fn cheat_active(&self)->bool{self.page==Page::CheatCatalog}
    pub fn set_tool_inputs(&mut self, inputs:Vec<(ContentSlot,String)>){
        for f in &mut self.tool.fields{if let ToolFieldKind::Input(slot)=f.kind{f.value=inputs.iter().find(|(key,_)|*key==slot).map(|(_,v)|v.clone()).unwrap_or("Not selected".into());}}
        self.tool_inputs=inputs;
    }
    pub fn open_tool(&mut self,tool:DebugTool,fields:Vec<ToolField>,message:String){if !self.tool_active(){self.open(Page::DebugTool);}self.tool.set(tool,fields,message);self.set_tool_inputs(self.tool_inputs.clone());}
    pub fn tool_active(&self)->bool{self.page==Page::DebugTool}
    pub fn host_panel_active(&self)->bool{self.content_active()||self.cheat_active()||self.tool_active()}
    pub fn tool_values(&self)->(DebugTool,Vec<(String,String)>,u64){(self.tool.tool,self.tool.values(),self.tool.revision())}
    pub fn tool_feedback(&mut self,message:String,summary:String,busy:bool){self.tool.message=message;self.tool.summary=summary;self.tool.busy=busy;}
    fn tool_action(&mut self,a:Option<MenuAction>)->Option<MenuAction>{if a==Some(MenuAction::ToolRequest(ToolRequest::Back)){if let Some(parent)=self.navigation.parents.pop(){self.page=parent.page;self.selected=parent.selected;self.scroll.set(parent.scroll);}else{self.navigation.reset();}}a}

    pub fn content_preferences(&self)->(ContentViewPreferences,ContentHistory){(self.content.preferences(),self.content.history())}
    pub fn restore_content_preferences(&mut self,view:ContentViewPreferences,history:ContentHistory)->Result<(),String>{self.content.restore_preferences(view,history)}
    pub fn content_scope(&mut self, kind:Vec<String>, current:Option<String>) {self.content.scope(kind,current);}
    pub fn content_active(&self)->bool{self.page==Page::ContentSelector}
    pub fn content_text_focus(&self)->bool{if self.tool_active(){self.tool.text_focus()||self.tool.choice_focus()}else if self.cheat_active(){self.cheats.editing()||self.cheats.choice_focus()}else{self.content_active() && (self.content.editing()||self.content.choice_focus())}}
    pub fn content_start_search(&mut self){if self.tool_active(){self.tool.edit();}else if self.cheat_active(){self.cheats.start_search();}else if self.content_active(){self.content.start_search();}}
    pub fn content_text(&mut self,text:&str){if self.tool_active(){self.tool.text(text);}else if self.cheat_active(){self.cheats.text(text);}else{self.content.text(text);}}
    pub fn content_preedit(&mut self,text:&str){if self.tool_active(){self.tool.preedit(text);}else if self.cheat_active(){self.cheats.preedit(text);}else{self.content.preedit(text);}}
    pub fn content_backspace(&mut self){if self.tool_active(){self.tool.backspace();}else if self.cheat_active(){self.cheats.text_backspace();}else{self.content.text_backspace();}}
    pub fn content_finish_search(&mut self,cancel:bool){if self.tool_active(){self.tool.finish(cancel);}else if self.cheat_active(){self.cheats.finish_search(cancel);}else{self.content.finish_search(cancel);}}
    pub fn content_tab(&mut self,back:bool){if self.tool_active(){self.tool.tab(back);}else if self.cheat_active(){self.cheats.tab(back);}else if self.content_active(){self.content.tab(back);}}
    pub fn content_busy(&mut self,busy:bool){self.content.busy=busy;}
    pub fn content_message(&mut self,message:String){self.content.message=message;}
    pub fn content_selected(&mut self,id:&str){self.content.mark_selected(id);}
    pub fn content_highlight(&self)->Option<&str>{self.content.highlighted_id()}
    pub fn content_preview(&mut self,preview:Vec<[u8;4]>,message:String){self.content.preview=preview;self.content.preview_message=message;}
    pub fn content_wheel(&mut self,up:bool){if self.tool_active(){self.tool.wheel(up);}else if self.cheat_active(){self.cheats.wheel(up);}else if self.content_active(){self.content.wheel(up);}}
    fn cheat_input(&mut self,input:MenuInput)->Option<MenuAction>{
        let action=self.cheats.handle(input);
        if action==Some(MenuAction::ContentCancel){self.back_to_main();return None;}action
    }
    fn content_input(&mut self,input:MenuInput)->Option<MenuAction>{
        let action=self.content.handle(input);
        if action==Some(MenuAction::ContentCancel){
            if let Some(parent)=self.navigation.parents.pop(){self.page=parent.page;self.selected=parent.selected;self.scroll.set(parent.scroll);}else{self.navigation.reset();}
        }action
    }

    pub fn catalog_selection(&self) -> Option<usize> { self.catalog.selected() }
    pub fn catalog_preview(&mut self, preview: Vec<[u8;4]>) { self.catalog.preview=preview; }
    pub fn catalog_result(&mut self, message: String) { self.catalog.message=message; }
    pub fn reveal_imported_slot(&mut self, slot:u8) {
        self.state_slot=usize::from(slot.saturating_sub(1)).min(self.state_slots.len()-1);
        self.open(Page::SaveStates);self.selected=2;
    }


    pub fn state_result(&mut self, message: String) { self.state_message = message; }
    pub fn set_state_timestamps(&mut self,timestamps:Vec<Option<u64>>) {if timestamps.len()==self.state_slots.len(){self.state_times=timestamps;}}
    pub fn set_standard_save_status(&mut self,message:String){self.atlas.standard_save=message;self.atlas.standard_save_stamp=None;}
    pub fn set_standard_save_timestamp(&mut self,stamp:u64){self.atlas.standard_save_stamp=Some(stamp);}
    fn standard_save_text(&self)->String {
        match self.atlas.standard_save_stamp {
            Some(value)=>clan_atlas::timestamp_utc(value).map(|stamp|self.tr_format("Last .sav modification: {stamp}",&[("stamp",stamp)])).unwrap_or_else(||self.tr("Ultima modifica file .sav: data non disponibile").into()),
            None=>self.tr(&self.atlas.standard_save).into(),
        }
    }
    pub fn set_state_undo(&mut self, label: Option<String>) { self.state_undo = label; self.state_undo_description=None; }
    pub fn set_state_undo_description(&mut self, description: Option<(&'static str,u8)>) {
        self.state_undo = description.map(|(key,slot)|i18n::format(Language::default(),key,&[("slot",slot.to_string())]));
        self.state_undo_description=description;
    }
    fn state_undo_text(&self)->Option<String> {
        self.state_undo_description.map(|(key,slot)|self.tr_format(key,&[("slot",slot.to_string())])).or_else(||self.state_undo.as_ref().map(|s|self.tr(s).into()))
    }
    pub fn set_save_backups(&mut self, copies: [BackupInfo;2]) { self.save_backups = copies; }
    pub fn backup_result(&mut self, message: String) { self.backup_message = message; }
    pub fn session_result(&mut self, message: String) { self.session_message = message; }

    fn rows(&self) -> &'static [Row] {
        if !self.debug_enabled() {
            if debug_unlock::debug_page(self.page){return &[Row::Back];}
            if self.page==Page::Saves{return &[Row::SaveStatesMenu,Row::SaveBackupsMenu,Row::Back];}
        }
        match self.page {
            Page::Main => clan_atlas::OVERVIEW_ROWS,
            Page::Debug => clan_atlas::debug_rows(self.atlas.selected_section()),
            _ => rows_of(self.page),
        }
    }
    fn page_layout(&self, w: usize, h: usize) -> Layout {
        if clan_atlas::specialty(self.page) {
            let b=clan_atlas::geometry(w,h).body;
            let mut l=self.content_layout(b.w,b.h);l.panel_x+=b.x;l.panel_y+=b.y;l.rows_y+=b.y;l
        } else {clan_atlas::list_layout(self,w,h)}
    }
    fn content_layout(&self, w: usize, h: usize) -> Layout {
        if matches!(self.page,Page::ContentSelector|Page::DebugTool) {
            let g=content_selector::geometry(w,h); Layout{panel_x:g.panel[0],panel_y:g.panel[1],panel_w:g.panel[2],panel_h:g.panel[3],pad:0,line_h:g.row,rows_y:g.list[1],visible_rows:1}
        } else if self.page == Page::DebugCatalog {
            debug_catalog::layout(w,h)
        } else if matches!(self.page, Page::GameSaves | Page::SwitchGameSave | Page::ReplaceGameSave) {
            game_saves::layout(self.rows().len(), w, h)
        } else if matches!(self.page, Page::SaveStates | Page::ReplaceState) {
            save_states::layout(self.rows().len(), w, h)
        } else if matches!(self.page,Page::SaveBackups|Page::RestoreBackup) {
            save_backups::layout(self.page,self.rows().len(),w,h)
        } else {
            let footer = if self.page == Page::Main && !self.session_message.is_empty() {
                (32. * (h as f32 / 480.).min(w as f32 / 560.).max(0.28)) as usize
            } else { 0 };
            layout_kind(self.rows().len(), w, h.saturating_sub(footer),self.page==Page::Main)
        }
    }

    /// The button currently waiting for a key press, if any.
    pub fn capturing(&self) -> Option<Bind> {
        self.capture
    }

    /// Stop waiting for a key (bound or cancelled).
    pub fn end_capture(&mut self) {
        self.capture = None;
    }

    fn open(&mut self, page: Page) {
        if debug_unlock::debug_page(page) && !self.debug_enabled(){return;}
        let scroll=self.scroll.get();
        self.navigation.open(page);
        if let Some(parent)=self.navigation.parents.last_mut(){parent.scroll=scroll;}
        self.scroll.set(0);
        self.dropdown = None;
    }

    fn back_to_main(&mut self) {
        self.dropdown=None;self.backup_selection=None;
        if let Some(parent)=self.navigation.parents.pop() {self.navigation.page=parent.page;self.navigation.selected=parent.selected;self.scroll.set(parent.scroll);}
        else {self.atlas_section(clan_atlas::Domain::Game,0);}
    }
    fn first_visible(&self, l: Layout) -> usize {
        if self.page==Page::Main {return 0;}
        let len = self.rows().len()
            - usize::from(matches!(self.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard));
        let mut first = self.scroll.get().min(len.saturating_sub(l.visible_rows));
        if self.selected < len {
            if self.selected < first {
                first = self.selected;
            }
            if self.selected >= first + l.visible_rows {
                first = self.selected + 1 - l.visible_rows;
            }
        }
        self.scroll.set(first);
        first
    }
    fn input_tab(&mut self, tab: u8) -> Option<MenuAction> {
        self.input_tabs_focus=None;
        if tab==0 && self.mouse_mode.get() {return None;}
        self.input_gamepad.set(tab);
        self.navigation
            .switch_input_tab(match tab {1=>Page::Gamepad,2=>Page::MouseKeyboard,_=>Page::Inputs});
        self.scroll.set(0);
        self.dropdown = None;
        if tab==2 && !self.mouse_mode.get() {return None;}
        Some(MenuAction::InputDevice(match tab {1=>Device::Gamepad,2=>Device::MouseKeyboard,_=>Device::Keyboard}))
    }
    fn toggle_mouse_mode(&mut self)->Option<MenuAction> {
        let enabled=!self.mouse_mode.get();
        self.mouse_mode.set(enabled);
        self.input_gamepad.set(if enabled {2}else{0});
        Some(MenuAction::InputDevice(if enabled {Device::MouseKeyboard}else{Device::Keyboard}))
    }
    fn open_dropdown(&mut self, row: Row, av: &AvConfig) {
        let opts = self.row_choices(row);
        if opts.is_empty() {
            return;
        }
        let index = self.row_choice_index(row,av,&opts);
        self.dropdown = Some(Dropdown {
            row,
            index,
            first: std::cell::Cell::new(0),
            options: opts,
        });
    }

    /// Feed one navigation event. Mutates `av` for value changes and
    /// returns the action the play loop must take, if any. `Up`/`Down`
    /// move the cursor. `Left`/`Right` move focus; values require Confirm
    /// to open a choice, then Confirm to commit or Cancel to discard.
    /// `Cancel` goes back from a submenu, or resumes from the main page.
    /// While a key capture is pending every event is ignored — the play
    /// loop decides what ends it.
    pub fn handle(&mut self, input: MenuInput, av: &mut AvConfig) -> Option<MenuAction> {
        self.language.set(av.menu_language);
        if av.debug && !self.debug_enabled(){self.set_debug_enabled(true);}
        if self.observe_debug_unlock(input) {
            self.set_debug_enabled(true);av.debug=true;self.debug_message="Debug sbloccato".into();
            return Some(MenuAction::Changed(Changed::Overlay));
        }
        if self.capture.is_some() {return None;}
        if let Some(mut dd)=self.dropdown.take() {
            match input {
                MenuInput::Cancel=>return None,
                MenuInput::Confirm=>return self.commit_choice(dd,av),
                MenuInput::Up|MenuInput::Left=>dd.index=if matches!(dd.row,Row::Volume|Row::Brightness|Row::MenuTransparency|Row::BubbleSize|Row::BubbleLength){dd.index.saturating_sub(1)}else{(dd.index+dd.options.len()-1)%dd.options.len()},
                MenuInput::Down|MenuInput::Right=>dd.index=if matches!(dd.row,Row::Volume|Row::Brightness|Row::MenuTransparency|Row::BubbleSize|Row::BubbleLength){(dd.index+1).min(dd.options.len()-1)}else{(dd.index+1)%dd.options.len()},
            }
            self.dropdown=Some(dd);return None;
        }
        if self.atlas.sidebar_focus && self.capture.is_none() {
            match input {
                MenuInput::Up=>self.atlas.nav_cursor=(self.atlas.nav_cursor+9)%10,
                MenuInput::Down=>self.atlas.nav_cursor=(self.atlas.nav_cursor+1)%10,
                MenuInput::Left|MenuInput::Right if self.atlas.nav_cursor==0=>{
                    let domain=if self.atlas.domain==clan_atlas::Domain::Game {clan_atlas::Domain::Debug}else{clan_atlas::Domain::Game};
                    self.atlas_section(domain,self.atlas.section[usize::from(domain==clan_atlas::Domain::Debug)]);
                },
                MenuInput::Confirm if self.atlas.nav_cursor==0=>{
                    let domain=if self.atlas.domain==clan_atlas::Domain::Game {clan_atlas::Domain::Debug}else{clan_atlas::Domain::Game};self.atlas_section(domain,self.atlas.section[usize::from(domain==clan_atlas::Domain::Debug)]);
                },
                MenuInput::Confirm if self.atlas.nav_cursor<=8=>{self.atlas_section(self.atlas.domain,self.atlas.nav_cursor-1);self.atlas.sidebar_focus=false;},
                MenuInput::Right if self.atlas.nav_cursor>=1 && self.atlas.nav_cursor<=8=>{self.atlas_section(self.atlas.domain,self.atlas.nav_cursor-1);self.atlas.sidebar_focus=false;},
                MenuInput::Confirm=>{self.atlas.sidebar_focus=false;return self.handle(MenuInput::Cancel,av);},
                MenuInput::Cancel=>{self.atlas.sidebar_focus=false;},
                _=>{},
            }
            return None;
        }

        if self.page==Page::Information{return self.information_input(input);}
        if self.atlas_reference_active(){return self.atlas_reference_input(input);}
        if (self.tool_active()||self.content_active()||self.cheat_active()) && !self.content_text_focus() && input==MenuInput::Left {
            self.atlas.sidebar_focus=true;self.atlas.nav_cursor=self.atlas.selected_section()+1;return None;
        }
        if self.tool_active(){let a=self.tool.handle(input);return self.tool_action(a);}
        if self.content_active(){return self.content_input(input);}
        if self.cheat_active(){return self.cheat_input(input);}
        if self.capture.is_some() {
            return None;
        }
        if self.input_tabs_focus.is_some() {return self.input_tabs_input(input);}
        let rows = self.rows();
        let row = rows[self.selected];
        if input==MenuInput::Confirm && !self.row_choices(row).is_empty() {
            self.open_dropdown(row,av);return None;
        }
        if self.page == Page::DebugCatalog && input==MenuInput::Confirm {
            if let Some(result) = self.catalog.handle(row,input,self.debug_session) { return result; }
        }
        match input {
            MenuInput::Up => {
                if self.selected==0 && matches!(self.page,Page::Inputs|Page::Gamepad|Page::MouseKeyboard) {
                    self.input_tabs_focus=Some(match self.page{Page::Gamepad=>1,Page::MouseKeyboard=>2,_=>0});return None;
                }
                self.selected = (self.selected + rows.len() - 1) % rows.len();
                None
            }
            MenuInput::Down => {
                self.selected = (self.selected + 1) % rows.len();
                None
            }
            MenuInput::Cancel => {
                if self.page == Page::Main {
                    Some(MenuAction::Resume)
                } else {
                    self.back_to_main();
                    None
                }
            }
            MenuInput::Confirm => match row {
                Row::CliReference=>{self.open(Page::CliReference);self.atlas.reference_cursor=0;self.atlas.planned_cursor=0;self.atlas.reference_scroll.set(0);self.atlas.detail_scroll=0;None},
                Row::FastForwardToggle=>Some(MenuAction::ToggleFastForward),
                Row::DebugTool(tool) => Some(MenuAction::OpenDebugTool(tool)),
                Row::ToolInput(slot) => Some(MenuAction::ChooseToolInput(slot)),
                Row::ContentPicker => Some(MenuAction::ContentOpen),
                Row::DebugCatalog => Some(MenuAction::OpenDebugCatalog),
                Row::CheatCatalog => Some(MenuAction::OpenCheatCatalog),
                Row::StartRecentEvents|Row::SaveRecentEvents=>{if self.debug_session{Some(if row==Row::StartRecentEvents{MenuAction::StartRecentEvents}else{MenuAction::SaveRecentEvents})}else{self.debug_message="Open an isolated copy before recent-event capture.".into();None}},
                Row::StartGameplayTrace|Row::SaveGameplayReport=>{if self.debug_session{Some(if row==Row::StartGameplayTrace{MenuAction::StartGameplayTrace}else{MenuAction::SaveGameplayReport})}else{self.debug_message="Open an isolated copy before gameplay tracing.".into();None}},
                Row::CreateIssueReport=>{if self.debug_session{Some(MenuAction::CreateIssueReport)}else{self.debug_message="Open an isolated copy before issue reporting.".into();None}},
                Row::StartCodeTrace|Row::SaveCodeReport=>{if self.debug_session{Some(if row==Row::StartCodeTrace{MenuAction::StartCodeTrace}else{MenuAction::SaveCodeReport})}else{self.debug_message="Open an isolated copy before code tracing.".into();None}},
                Row::StartAudioCapture|Row::SaveAudioReport=>{if self.debug_session{Some(if row==Row::StartAudioCapture{MenuAction::StartAudioCapture}else{MenuAction::SaveAudioReport})}else{self.debug_message="Open an isolated copy before audio capture.".into();None}},
                Row::ToggleEventConsole=>Some(MenuAction::ToggleEventConsole),
                Row::OpenCommandConsole=>Some(MenuAction::OpenCommandConsole),
                Row::StartUiTrace|Row::SaveUiReport=>{if self.debug_session{Some(if row==Row::StartUiTrace{MenuAction::StartUiTrace}else{MenuAction::SaveUiReport})}else{self.debug_message="Open an isolated copy before UI tracing.".into();None}},
                Row::CaptureGraphics=>{if self.debug_session{Some(MenuAction::CaptureGraphics)}else{self.debug_message="Open an isolated copy before graphics capture.".into();None}},
                Row::InspectPC => {
                    if self.debug_session {Some(MenuAction::InspectPC)}else{self.debug_message="Open an isolated debug copy before inspecting.".into();None}
                },
                Row::DebugStepOne | Row::DebugStepTen | Row::DebugResume => {
                    if !self.debug_session { self.debug_message = "Open an isolated debug copy before stepping.".into(); None }
                    else if row == Row::DebugResume { Some(MenuAction::DebugResume) }
                    else { Some(MenuAction::DebugStep(if row == Row::DebugStepOne {1} else {10})) }
                },
                Row::CompareReplay => Some(MenuAction::CompareReplay),
                Row::CaptureReplay => Some(MenuAction::CaptureReplay),
                Row::AuditReplay => Some(MenuAction::AuditReplay),
                Row::ReplayCase => Some(MenuAction::ReplayCase),
                Row::ChooseReplayProfile => Some(MenuAction::ChooseReplayProfile),
                Row::CancelReplay => Some(MenuAction::CancelReplay),
                Row::StartInputRecording => {
                    if self.debug_session { Some(MenuAction::StartInputRecording) }
                    else { self.debug_message = "Open an isolated debug copy before recording.".into(); None }
                },
                Row::StopInputRecording => Some(MenuAction::StopInputRecording),
                Row::CopyDiagnosis => Some(MenuAction::CopyDiagnosis),
                Row::StartDebugSession => {
                    if self.debug_session { self.debug_message = "Close this debug session before creating another.".into(); None }
                    else { Some(MenuAction::StartDebugSession) }
                },
                Row::CloseDebugSession => {
                    if self.debug_session { Some(MenuAction::CloseDebugSession) }
                    else { self.debug_message = "This is the normal game; no debug session is active.".into(); None }
                },
                Row::GameSavesMenu => {
                    if av.debug_extended_saves && self.game_save_slots.len() == 99 {
                        self.open(Page::GameSaves);
                    } else { self.debug_message = "Enable Extended saves first.".into(); }
                    None
                },
                Row::GameSaveSlot => None,
                Row::UseGameSave | Row::CopyGameSave => {
                    let slot = &self.game_save_slots[self.game_save_slot];
                    if !slot.available {
                        self.game_save_message = "This save is not available.".into();
                    } else if row == Row::UseGameSave {
                        if slot.current { self.game_save_message = "This game save is already active.".into(); }
                        else { self.open(Page::SwitchGameSave); self.selected = 1; }
                    } else if slot.occupied {
                        self.open(Page::ReplaceGameSave);
                        self.selected = 1;
                    } else {
                        return Some(MenuAction::CopyGameSave { slot: self.game_save_slot as u8 + 1, overwrite: false });
                    }
                    None
                },
                Row::ConfirmSwitchGameSave => {
                    self.back_to_main();
                    Some(MenuAction::SwitchGameSave(self.game_save_slot as u8 + 1))
                },
                Row::ConfirmCopyGameSave => {
                    self.back_to_main();
                    Some(MenuAction::CopyGameSave { slot: self.game_save_slot as u8 + 1, overwrite: true })
                },
                Row::MouseMode => self.toggle_mouse_mode(),
                Row::Resume => Some(MenuAction::Resume),
                Row::RestartRom => Some(MenuAction::RestartRom),
                Row::Backup(copy) => {
                    if let Some(id) = self.save_backups[copy as usize].generation {
                        self.backup_selection = Some((id,copy));
                        self.open(Page::RestoreBackup);
                        self.selected = 1;
                    } else { self.backup_message = "This backup is not available.".into(); }
                    None
                },
                Row::ConfirmRestoreBackup => {
                    let choice = self.backup_selection.take();
                    self.back_to_main();
                    choice.map(|(id,_)| MenuAction::RestoreBackup(id))
                },
                Row::UndoState => {
                    if self.state_undo.is_some() { Some(MenuAction::UndoState) }
                    else { self.state_message = "Nothing to undo.".into(); None }
                },
                Row::StateSlot => None,
                Row::SaveState => {
                    if self.state_slots[self.state_slot].occupied {
                        self.open(Page::ReplaceState);
                        self.selected = 1; // Default to Back; a repeated confirm must not overwrite.
                        None
                    } else { Some(MenuAction::SaveState { slot: self.state_slot as u8 + 1, overwrite: false }) }
                },
                Row::SaveStateConfirm => {
                    self.back_to_main();
                    Some(MenuAction::SaveState { slot: self.state_slot as u8 + 1, overwrite: true })
                },
                Row::LoadState => {
                    if self.state_slots[self.state_slot].occupied {
                        Some(MenuAction::LoadState(self.state_slot as u8 + 1))
                    } else {
                        self.state_message = self.tr_format("Slot {slot} is empty.", &[("slot", (self.state_slot + 1).to_string())]);
                        None
                    }
                },
                Row::Quit => {
                    self.open(Page::Exit);
                    None
                }
                Row::QuitYes => Some(MenuAction::Quit),
                Row::ReturnToGame => Some(MenuAction::Resume),
                Row::ChangeRom => Some(MenuAction::SwitchRom),
                Row::QuitNo => {
                    self.back_to_main();
                    None
                }
                Row::Back => {
                    self.back_to_main();
                    None
                }
                Row::Key(b) => {
                    self.capture = Some(if self.page == Page::Gamepad {
                        Bind::Gamepad(b)
                    } else {
                        Bind::Button(b)
                    });
                    None
                }
                Row::InputsMenu => self.input_tab(self.input_gamepad.get()),
                Row::ResetGamepad => Some(MenuAction::ResetGamepad),
                Row::Menu => {
                    self.capture = Some(if self.page == Page::Gamepad { Bind::GamepadMenu } else { Bind::Menu });
                    None
                }
                Row::FfKey => {
                    self.capture = Some(if self.page == Page::Gamepad {
                        Bind::GamepadFastForward
                    } else {
                        Bind::FastForward
                    });
                    None
                }
                Row::FfToggleKey => {
                    self.capture = Some(if self.page == Page::Gamepad {
                        Bind::GamepadFastForwardToggle
                    } else {
                        Bind::FastForwardToggle
                    });
                    None
                }
                Row::ZoomIn => {
                    self.capture = Some(if self.page == Page::Gamepad {
                        Bind::GamepadZoomIn
                    } else {
                        Bind::ZoomIn
                    });
                    None
                }
                Row::ZoomOut => {
                    self.capture = Some(if self.page == Page::Gamepad {
                        Bind::GamepadZoomOut
                    } else {
                        Bind::ZoomOut
                    });
                    None
                }
                r if r.opens().is_some() => {
                    self.open(r.opens().unwrap());
                    None
                }
                // Value choices are opened above; inert rows have no choices.
                _ => {
                    self.open_dropdown(row, av);
                    None
                }
            },
            MenuInput::Left => {
                self.navigate_horizontal(false);None
            }
            MenuInput::Right => {
                self.navigate_horizontal(true);None
            }
        }
    }

    /// Row index under frame-pixel `(x, y)`, if any.
    fn row_at(&self, x: usize, y: usize, w: usize, h: usize) -> Option<usize> {
        if self.page==Page::Main {return atlas_theme::overview_rects(w,h).iter().position(|r|r.hit(x,y));}
        if self.page==Page::DebugCatalog{return (0..self.rows().len()).find(|&i|clan_atlas::row_rect(self,i,w,h).is_some_and(|r|r.hit(x,y)));}
        if !clan_atlas::specialty(self.page) {return (0..self.rows().len()).find(|&i|clan_atlas::row_rect(self,i,w,h).is_some_and(|r|r.hit(x,y)));}
        let rows = self.rows();
        let l = self.page_layout(w, h);
        if matches!(self.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard)
            && y >= l.panel_y + l.panel_h - l.pad - l.line_h + 6
            && y < l.panel_y + l.panel_h - l.pad
            && x >= l.panel_x + l.panel_w - l.pad - l.line_h * 3
            && x < l.panel_x + l.panel_w - l.pad
        {
            return Some(rows.len() - 1);
        }
        if x < l.panel_x + 2 || x >= l.panel_x + l.panel_w - 2 {
            return None;
        }
        let top = l.rows_y.checked_sub(2)?;
        if y < top {
            return None;
        }
        let i = (y - top) / l.line_h;
        if i >= l.visible_rows {
            return None;
        }
        let index = i + self.first_visible(l);
        if rows[index] == Row::Back
            && !matches!(self.page, Page::SaveStates | Page::ReplaceState | Page::SaveBackups | Page::RestoreBackup
                | Page::GameSaves | Page::SwitchGameSave | Page::ReplaceGameSave)
            && x < l.panel_x + l.panel_w - l.pad - l.line_h * 3 {
            return None;
        }
        Some(index)
    }

    /// Pointer moved: highlight the row under it (ignored while a key
    /// capture is pending). `(x, y)` are frame pixels, `w`/`h` the frame size.
    pub fn hover(&mut self, x: usize, y: usize, w: usize, h: usize) {
        let g=clan_atlas::geometry(w,h);
        self.atlas.information_hover=self.capture.is_none()&&g.information.hit(x,y);
        if self.dropdown.is_some(){if let Some(dd)=self.dropdown.as_mut(){if let Some(i)=theme::dropdown_hit(dd,x,y,w,h){dd.index=i;}}return;}
        if self.capture.is_some(){return;}
        if !self.content_text_focus() {
            if let Some(i)=g.nav.iter().position(|r|r.hit(x,y)) {self.input_tabs_focus=None;self.atlas.nav_cursor=i+1;self.atlas.sidebar_focus=true;return;}
        }
        if self.atlas_reference_active(){self.atlas_reference_hover(x,y,w,h);return;}
        if g.body.hit(x,y){self.atlas.sidebar_focus=false;}
        if clan_atlas::specialty(self.page) {
            if !g.body.hit(x,y){return;}
            let (x,y,w,h)=(x-g.body.x,y-g.body.y,g.body.w,g.body.h);
            if self.tool_active(){self.tool.hover(x,y,w,h);return;}
            if self.content_active(){self.content.hover(x,y,w,h);return;}
            if self.cheat_active(){self.cheats.hover(x,y,w,h);return;}
        }
        if self.capture.is_some() {
            return;
        }
        if let Some(dd) = self.dropdown.as_mut() {
            if let Some(i) = theme::dropdown_hit(dd, x, y, w, h) {
                dd.index = i;
            }
            return;
        }
        if let Some(i) = self.row_at(x, y, w, h) {
            self.input_tabs_focus=None;
            self.selected = i;
        }
    }

    /// Mouse click at frame-pixel `(x, y)`. Left on an action/submenu row =
    /// Confirm; left on a value row opens its choices; right = Cancel
    /// (back). A left click outside any row does nothing.
    pub fn click(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        left: bool,
        av: &mut AvConfig,
    ) -> Option<MenuAction> {
        self.cancel_debug_unlock();
        let g=clan_atlas::geometry(w,h);
        if let Some(dd) = self.dropdown.as_ref() {
            let hit = theme::dropdown_hit(dd, x, y, w, h);
            if left {
                if let Some(i) = hit {
                    self.dropdown.as_mut().unwrap().index = i;
                    return self.handle(MenuInput::Confirm, av);
                }
            }
            self.dropdown = None;
            return None;
        }
        if self.capture.is_none() && !self.content_text_focus() && left {
            if matches!(self.page,Page::Debug|Page::CliReference) && g.debug_tabs.hit(x,y){self.atlas.planned=x>=g.debug_tabs.x+g.debug_tabs.w/2;self.atlas.detail_scroll=0;return None;}
            if self.atlas_reference_active() && g.body.hit(x,y){self.atlas_reference_hover(x,y,w,h);return None;}
            if g.domain.hit(x,y) {
                let domain=if !self.debug_enabled() || x<g.domain.x+g.domain.w/2 {clan_atlas::Domain::Game}else{clan_atlas::Domain::Debug};
                self.atlas_section(domain,self.atlas.section[usize::from(domain==clan_atlas::Domain::Debug)]);self.atlas.sidebar_focus=false;return None;
            }
            if let Some(i)=g.nav.iter().position(|r|r.hit(x,y)){self.atlas_section(self.atlas.domain,i);self.atlas.sidebar_focus=false;return None;}
            if g.back.hit(x,y){self.atlas.sidebar_focus=false;return self.handle(MenuInput::Cancel,av);}
        }
        if g.body.hit(x,y){self.atlas.sidebar_focus=false;}
        if clan_atlas::specialty(self.page) && g.body.hit(x,y) {
            let (x,y,w,h)=(x-g.body.x,y-g.body.y,g.body.w,g.body.h);
            if self.tool_active(){let a=self.tool.click(x,y,w,h,left);return self.tool_action(a);}
            if self.content_active(){let action=self.content.click(x,y,w,h,left);if action==Some(MenuAction::ContentCancel){return self.content_input(MenuInput::Cancel);}return action;}
            if self.cheat_active(){let action=self.cheats.click(x,y,w,h,left);if action==Some(MenuAction::ContentCancel){self.back_to_main();return None;}return action;}
        }
        if self.tool_active() || self.content_active() || self.cheat_active() {return None;}
        if self.capture.is_some() {
            return None;
        }
        if matches!(self.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard) {
            if let Some(tab)=(0..3).find(|&i| self.input_tab_rect(i,w,h).hit(x,y)) {
                return if left { self.input_tab(tab) } else { None };
            }
        }
        match self.row_at(x, y, w, h) {
            Some(i) => {
                self.input_tabs_focus=None;
                self.selected = i;
                self.handle(if left {MenuInput::Confirm}else{MenuInput::Cancel},av)
            }
            None if left => None,
            None => self.handle(MenuInput::Cancel, av),
        }
    }

    /// Composite the overlay onto a composed `rgba` frame (`w*h`,
    /// row-major, RGBA bytes — the alpha channel is left at 255 and
    /// ignored by both present paths). `keys` are the current keyboard
    /// bindings (indexed by `Button::index`), shown on the Inputs page.
    /// The play loop supplies a high-resolution pause canvas independent
    /// of the GBA frame. Both GPU and CPU presenters display that canvas.
    /// Its native blue/brass backgrounds share one opacity setting; text remains opaque.
    pub fn draw(
        &self,
        av: &AvConfig,
        keys: &[String; 10],
        ff_key: &str,
        ff_toggle_key: &str,
        zoom_in: &str,
        zoom_out: &str,
        rgba: &mut [[u8; 4]],
        w: usize,
        h: usize,
    ) {
        self.language.set(av.menu_language);
        self.viewport.set((w,h));
        let transparency = if av.menu_transparency.is_finite() { av.menu_transparency.clamp(0.,1.) } else { 0. };
        canvas::with_panel_alpha(((1.-transparency)*255.).round() as u8, || atlas_theme::draw(
            self,
            av,
            keys,
            ff_key,
            ff_toggle_key,
            zoom_in,
            zoom_out,
            rgba,
            w,
            h,
        ));
    }

    fn atlas_binding(&self,cfg:&InputConfig) {
        if self.hint_device.get().is_none(){self.note_input(cfg.device);}
        *self.menu_binding.borrow_mut()=if self.hints_gamepad() {cfg.menu_pad.clone()}else{cfg.menu_key.clone()};
    }
    pub fn draw_inputs(
        &self,
        av: &AvConfig,
        cfg: &InputConfig,
        rgba: &mut [[u8; 4]],
        w: usize,
        h: usize,
    ) {
        if av.debug {self.debug_enabled.set(true);}
        self.atlas_binding(cfg);
        self.mouse_mode.set(cfg.mouse_keyboard);
        self.controller_layout.set(cfg.gamepad_layout);
        if !matches!(self.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard) {
            self.input_gamepad.set(if cfg.device==Device::Gamepad {1}else if cfg.mouse_keyboard {2}else{0});
        }
        self.draw(
            av,
            if self.page == Page::Gamepad {
                &cfg.pads
            } else {
                if self.page==Page::MouseKeyboard {&cfg.mouse_keys}else{&cfg.keys}
            },
            if self.page == Page::Gamepad {
                &cfg.ff_pad
            } else {
                &cfg.ff_key
            },
            if self.page == Page::Gamepad {
                &cfg.ff_toggle_pad
            } else {
                &cfg.ff_toggle_key
            },
            if self.page == Page::Gamepad {
                &cfg.zoom_in_pad
            } else {
                &cfg.zoom_in_key
            },
            if self.page == Page::Gamepad {
                &cfg.zoom_out_pad
            } else {
                &cfg.zoom_out_key
            },
            rgba,
            w,
            h,
        );
    }
}

/// A 0..1-ish value in whole tenths (so stepping never accumulates float
/// error), and back.
fn tenths(v: f32) -> i32 {
    (v * 10.0).round() as i32
}

fn percent(v: f32) -> String {
    format!("{}%", tenths(v) * 10)
}

/// The display value for a row (`None` for rows without one).
fn value_of(
    row: Row,
    av: &AvConfig,
    keys: &[String; 10],
    ff_key: &str,
    capture: Option<Bind>,
) -> Option<String> {
    Some(match row {
        Row::Language => av.menu_language.name().into(),
        Row::CliReference => "Sola lettura".into(),
        Row::DebugMenu => ">".into(),
        Row::GameSavesMenu => if av.debug_extended_saves { ">" } else { "DISABLED" }.into(),
        Row::ExtendedSaves => if av.debug_extended_saves { "ON" } else { "OFF" }.into(),
        Row::ScriptLibraryMenu | Row::DebugToolsMenu | Row::DebugTool(_) | Row::ToolInputsMenu | Row::ToolInput(_) | Row::ContentPicker | Row::DebugCatalog | Row::CheatCatalog | Row::CatalogRegion | Row::CatalogStatus | Row::CatalogKind | Row::CatalogCase | Row::CatalogEntry | Row::CatalogImport | Row::CatalogOpen | Row::CatalogChoose
            | Row::StartRecentEvents | Row::SaveRecentEvents | Row::StartGameplayTrace | Row::SaveGameplayReport | Row::CreateIssueReport | Row::StartCodeTrace | Row::SaveCodeReport | Row::StartAudioCapture | Row::SaveAudioReport | Row::StartUiTrace | Row::SaveUiReport | Row::ToggleEventConsole | Row::OpenCommandConsole | Row::CaptureGraphics | Row::InspectPC | Row::DebugStepOne | Row::DebugStepTen | Row::DebugResume | Row::CompareReplay | Row::CaptureReplay | Row::AuditReplay | Row::ReplayCase | Row::ChooseReplayProfile | Row::CancelReplay | Row::StartInputRecording | Row::StopInputRecording | Row::StartDebugSession | Row::CloseDebugSession | Row::CopyDiagnosis | Row::GameSaveSlot | Row::UseGameSave | Row::CopyGameSave | Row::ConfirmSwitchGameSave | Row::ConfirmCopyGameSave => return None,
        Row::MouseMode => return None,
        Row::Menu => "HOLD + RELEASE".to_string(),
        Row::FastForwardToggle => if av.ff_speed>0 { "Session".into() } else { "Unavailable".into() },
        Row::FastForwardMenu | Row::SaveStatesMenu | Row::SaveBackupsMenu => ">".to_string(),
        Row::Backup(_) | Row::ConfirmRestoreBackup | Row::StateSlot | Row::SaveStateConfirm | Row::SaveState | Row::LoadState | Row::UndoState | Row::Resume
        | Row::RestartRom
        | Row::Quit
        | Row::QuitYes
        | Row::QuitNo
        | Row::ReturnToGame
        | Row::ChangeRom
        | Row::Back
        | Row::ResetGamepad => return None,
        Row::AudioMenu | Row::TextBubblesMenu
                | Row::VideoMenu | Row::InputsMenu | Row::SwitchRomMenu => ">".to_string(),
        Row::FastForward => format!("{}%",input_config::fast_forward::selected_percent(av.ff_speed)),
        Row::Volume => percent(av.audio_volume),
        Row::Mute => if av.audio_mute { "ON" } else { "OFF" }.to_string(),
        Row::FastForwardAudio => match av.ff_audio.as_str() {
            "fast" => "FAST AUDIO",
            "normal" => "1X DELAYED",
            _ => "SILENT",
        }
        .to_string(),
        Row::Audio => if av.audio_enhanced { "ON" } else { "OFF" }.to_string(),
        Row::Screen => screen_kind(av).name().to_uppercase(),
        Row::BubbleSize => percent(av.bubble_size),
        Row::BubbleLength => percent(av.bubble_length),
        Row::Brightness => percent(av.video_brightness),
        Row::MenuTransparency => percent(av.menu_transparency),
        Row::Darken => av.screen_darken.to_uppercase(),
        Row::Response => response_mode(av).name().to_uppercase(),
        Row::Grid => av.grid.to_uppercase(),
        Row::Scaling => scaling_name(av).to_uppercase(),
        Row::Aspect => aspect_name(av).to_uppercase(),
        Row::Vsync => if av.video_vsync { "ON" } else { "OFF" }.to_string(),
        Row::Fullscreen => if av.video_fullscreen { "ON" } else { "OFF" }.to_string(),
        Row::Fps => av.video_fps.to_uppercase(),
        Row::Key(b) => {
            if capture == Some(Bind::Gamepad(b)) {
                "PRESS BUTTON".to_string()
            } else if capture == Some(Bind::Button(b)) {
                "PRESS KEY".to_string()
            } else {
                keys[b.index()].to_uppercase()
            }
        }
        Row::FfKey => {
            if capture == Some(Bind::FastForward) {
                "PRESS KEY".to_string()
            } else {
                ff_key.to_uppercase()
            }
        }
        Row::ZoomIn | Row::ZoomOut => "PRESS KEY".to_string(),
        Row::FfToggleKey => "PRESS KEY".to_string(),
    })
}

fn gamepad_label(token: &str, layout: GamepadLayout) -> String {
    token
        .strip_prefix("DInputButton")
        .map(|n| format!("BUTTON {n}"))
        .unwrap_or_else(|| layout.label(token))
}

/// Step a 0..1 style slider by one tenth within `[lo, hi]` tenths (clamped,
/// no wrap). Returns whether the value moved.
fn step_slider(value: &mut f32, forward: bool, lo: i32, hi: i32) -> bool {
    let cur = tenths(*value).clamp(lo, hi);
    let next = (cur + if forward { 1 } else { -1 }).clamp(lo, hi);
    let changed = next != tenths(*value);
    *value = next as f32 / 10.0;
    changed
}

/// Cycle the selected setting forward (`forward`) or backward and report
/// which surface moved. Returns `None` for action rows (and for sliders
/// already at their end).
fn setting_options(row: Row) -> Vec<String> {
    match row {
        Row::Language => Language::ALL.iter().map(|l| l.name().into()).collect(),
        Row::Volume | Row::MenuTransparency => (0..=10).map(|i|format!("{}%",i*10)).collect(),
        Row::Brightness => (5..=15).map(|i|format!("{}%",i*10)).collect(),
        Row::BubbleSize => (5..=20).map(|i|format!("{}%",i*10)).collect(),
        Row::BubbleLength => (10..=20).map(|i|format!("{}%",i*10)).collect(),
        Row::Audio | Row::Mute | Row::Vsync | Row::Fullscreen => vec!["OFF".into(),"ON".into()],
        Row::FastForwardAudio => vec!["SILENT".into(),"FAST".into(),"NORMAL".into()],
        Row::FastForward => return FF_SPEEDS.iter().map(|p|format!("{p}%")).collect(),
        Row::ExtendedSaves => ["OFF", "ON"].iter().map(|s| s.to_string()).collect(),
        Row::Screen => ScreenKind::ALL
            .iter()
            .map(|s| s.name().to_uppercase())
            .collect(),
        Row::Response => ResponseMode::ALL
            .iter()
            .map(|s| s.name().to_uppercase())
            .collect(),
        Row::Scaling => SCALING_MODES.iter().map(|s| s.to_uppercase()).collect(),
        Row::Aspect => ASPECT_MODES.iter().map(|s| s.to_uppercase()).collect(),
        Row::Fps => ["OFF", "INTERNAL", "EXTERNAL", "BOTH"].iter().map(|s| s.to_string()).collect(),
        Row::Grid | Row::Darken => std::iter::once("AUTO".into())
            .chain((0..=10).map(|i| format!("{:.1}", i as f32 / 10.0)))
            .collect(),
        _ => Vec::new(),
    }
}
fn setting_value(row: Row, av: &AvConfig) -> String {
    value_of(row, av, &std::array::from_fn(|_| String::new()), "", None).unwrap_or_default()
}
fn choose_setting(row: Row, value: &str, av: &mut AvConfig) -> Option<Changed> {
    if setting_value(row,av)==value {return None;}
    if row==Row::Language {
        av.menu_language = Language::ALL.into_iter().find(|l| l.name()==value)?;
        return Some(Changed::Overlay);
    }
    if row==Row::FastForwardAudio {
        if !["SILENT","FAST","NORMAL"].contains(&value) {return None;}
        av.ff_audio=value.to_ascii_lowercase();return Some(Changed::Gamut);
    }
    if matches!(row,Row::Volume|Row::MenuTransparency|Row::Brightness|Row::BubbleSize|Row::BubbleLength) {
        if !setting_options(row).iter().any(|v|v==value) {return None;}
        let number=value.trim_end_matches('%').parse::<f32>().ok()?/100.;
        return Some(match row {
            Row::Volume=>{av.audio_volume=number;Changed::Audio},
            Row::MenuTransparency=>{av.menu_transparency=number;Changed::Overlay},
            Row::Brightness=>{av.video_brightness=number;Changed::Video},
            Row::BubbleSize=>{av.bubble_size=number;Changed::Gamut},
            _=>{av.bubble_length=number;Changed::Gamut},
        });
    }
    if row==Row::FastForward {
        let percent=value.trim_end_matches('%').parse::<u32>().ok()?;
        if FF_SPEEDS.contains(&percent){av.ff_speed=percent;return Some(Changed::Gamut);}return None;
    }
    let mut change = None;
    for _ in 0..=setting_options(row).len() {
        if setting_value(row, av) == value {
            break;
        }
        change = cycle(row, av, true);
    }
    change
}
fn cycle(row: Row, av: &mut AvConfig, forward: bool) -> Option<Changed> {
    match row {
        Row::Language => {
            av.menu_language = step_enum(&Language::ALL, av.menu_language, forward);
            Some(Changed::Overlay)
        },
        Row::MenuTransparency=>step_slider(&mut av.menu_transparency,forward,0,10).then_some(Changed::Overlay),
        Row::ExtendedSaves => {
            av.debug_extended_saves = !av.debug_extended_saves;
            Some(Changed::Debug)
        },
        Row::ScriptLibraryMenu | Row::DebugToolsMenu | Row::DebugTool(_) | Row::ToolInputsMenu | Row::ToolInput(_) | Row::ContentPicker | Row::DebugCatalog | Row::CheatCatalog | Row::CatalogRegion | Row::CatalogStatus | Row::CatalogKind | Row::CatalogCase | Row::CatalogEntry | Row::CatalogImport | Row::CatalogOpen | Row::CatalogChoose
            | Row::StartRecentEvents | Row::SaveRecentEvents | Row::StartGameplayTrace | Row::SaveGameplayReport | Row::CreateIssueReport | Row::StartCodeTrace | Row::SaveCodeReport | Row::StartAudioCapture | Row::SaveAudioReport | Row::StartUiTrace | Row::SaveUiReport | Row::ToggleEventConsole | Row::OpenCommandConsole | Row::CaptureGraphics | Row::InspectPC | Row::DebugStepOne | Row::DebugStepTen | Row::DebugResume | Row::CompareReplay | Row::CaptureReplay | Row::AuditReplay | Row::ReplayCase | Row::ChooseReplayProfile | Row::CancelReplay | Row::StartInputRecording | Row::StopInputRecording | Row::StartDebugSession | Row::CloseDebugSession | Row::CopyDiagnosis | Row::DebugMenu | Row::GameSavesMenu | Row::GameSaveSlot | Row::UseGameSave | Row::CopyGameSave
            | Row::ConfirmSwitchGameSave | Row::ConfirmCopyGameSave => None,
        Row::FastForwardToggle | Row::CliReference => None,
        Row::Menu => None,
        Row::MouseMode => None,
        Row::Backup(_) | Row::ConfirmRestoreBackup | Row::StateSlot | Row::SaveStateConfirm | Row::SaveState | Row::LoadState | Row::UndoState | Row::Resume
        | Row::RestartRom
        | Row::Quit
        | Row::QuitYes
        | Row::QuitNo
        | Row::ReturnToGame
        | Row::ChangeRom
        | Row::SaveStatesMenu
        | Row::SaveBackupsMenu
        | Row::FastForwardMenu
        | Row::AudioMenu
        | Row::TextBubblesMenu
                | Row::VideoMenu
        | Row::InputsMenu
        | Row::SwitchRomMenu
        | Row::ResetGamepad
        | Row::Key(_)
        | Row::FfKey
        | Row::FfToggleKey
        | Row::ZoomIn
        | Row::ZoomOut
        | Row::Back => None,
        Row::FastForward => {
            let percent=input_config::fast_forward::selected_percent(av.ff_speed);
            av.ff_speed=if forward {(percent+20).min(800)}else{percent.saturating_sub(20).max(100)};
            Some(Changed::Gamut)
        }
        Row::Volume => step_slider(&mut av.audio_volume, forward, 0, 10).then_some(Changed::Audio),
        Row::Fullscreen => {
            av.video_fullscreen = !av.video_fullscreen;
            Some(Changed::Fullscreen)
        }
        Row::Mute => {
            av.audio_mute = !av.audio_mute;
            Some(Changed::Audio)
        }
        Row::FastForwardAudio => {
            let modes = ["silent", "fast", "normal"];
            av.ff_audio = step_enum(
                &modes,
                modes
                    .iter()
                    .copied()
                    .find(|m| *m == av.ff_audio)
                    .unwrap_or("silent"),
                forward,
            )
            .into();
            Some(Changed::Gamut)
        }
        Row::Audio => {
            av.audio_enhanced = !av.audio_enhanced;
            Some(Changed::Audio)
        }
        Row::Screen => {
            let cur = screen_kind(av);
            let next = step_enum(&ScreenKind::ALL, cur, forward);
            av.screen = next.name().to_string();
            Some(Changed::Video)
        }
        Row::BubbleSize => step_slider(&mut av.bubble_size, forward, 5, 20).then_some(Changed::Video),
        Row::BubbleLength => step_slider(&mut av.bubble_length, forward, 10, 20).then_some(Changed::Video),
        Row::Brightness => {
            step_slider(&mut av.video_brightness, forward, 5, 15).then_some(Changed::Video)
        }
        Row::Darken => {
            av.screen_darken = step_knob(&av.screen_darken, forward);
            Some(Changed::Video)
        }
        Row::Response => {
            let cur = response_mode(av);
            let next = step_enum(&ResponseMode::ALL, cur, forward);
            av.response = next.name().to_string();
            Some(Changed::Video)
        }
        Row::Grid => {
            av.grid = step_knob(&av.grid, forward);
            Some(Changed::Video)
        }
        Row::Scaling => {
            let next = step_enum(&SCALING_MODES, scaling_name(av), forward);
            av.scaling = next.to_string();
            Some(Changed::Video)
        }
        Row::Aspect => {
            let next = step_enum(&ASPECT_MODES, aspect_name(av), forward);
            av.video_aspect = next.to_string();
            Some(Changed::Video)
        }
        Row::Vsync => {
            av.video_vsync = !av.video_vsync;
            Some(Changed::Vsync)
        }
        Row::Fps => {
            av.video_fps = step_enum(&["off", "internal", "external", "both"], av.video_fps.as_str(), forward).to_string();
            Some(Changed::Overlay)
        }
    }
}

/// The configured scaling mode, normalized (unknown values read as "off").
fn scaling_name(av: &AvConfig) -> &'static str {
    SCALING_MODES
        .iter()
        .copied()
        .find(|m| *m == av.scaling.trim().to_lowercase())
        .unwrap_or("off")
}

/// The configured aspect, normalized (unknown values read as "16:9").
fn aspect_name(av: &AvConfig) -> &'static str {
    ASPECT_MODES
        .iter()
        .copied()
        .find(|m| *m == av.video_aspect.trim().to_lowercase())
        .unwrap_or("16:9")
}

/// A 0..1 knob string cycled through `auto` and discrete tenths. The
/// launcher exposes these as continuous sliders; the overlay steps them
/// so a d-pad can reach every useful value.
fn step_knob(cur: &str, forward: bool) -> String {
    // Ordered ring: auto, 0.0, 0.1, ... 1.0.
    let mut steps: Vec<String> = vec!["auto".to_string()];
    for i in 0..=10 {
        steps.push(format!("{:.1}", i as f32 / 10.0));
    }
    // Find the closest current entry (parse, else "auto").
    let idx = match AvConfig::knob(cur) {
        None => 0,
        Some(v) => {
            let q = (v.clamp(0.0, 1.0) * 10.0).round() as usize;
            (q + 1).min(steps.len() - 1)
        }
    };
    let next = if forward {
        (idx + 1) % steps.len()
    } else {
        (idx + steps.len() - 1) % steps.len()
    };
    steps[next].clone()
}

/// Step through a fixed enum table, wrapping.
fn step_enum<T: Copy + PartialEq>(all: &[T], cur: T, forward: bool) -> T {
    let i = all.iter().position(|&x| x == cur).unwrap_or(0);
    let n = all.len();
    if forward {
        all[(i + 1) % n]
    } else {
        all[(i + n - 1) % n]
    }
}

fn screen_kind(av: &AvConfig) -> ScreenKind {
    ScreenKind::by_name(&av.screen).unwrap_or(ScreenKind::Frontlit)
}

fn response_mode(av: &AvConfig) -> ResponseMode {
    ResponseMode::by_name(&av.response).unwrap_or(ResponseMode::Smart)
}

// ── rgba drawing primitives ─────────────────────────────────────────

#[cfg(test)]
mod tests {

    #[test]
    fn event_console_is_an_explicit_debug_menu_action() {
        let mut av=input_config::AvConfig::default();let mut menu=super::Menu::unlocked_for_test(true,true);
        menu.atlas_section(super::clan_atlas::Domain::Debug,4);
        at(&mut menu,super::Row::ToggleEventConsole);
        assert_eq!(menu.handle(super::MenuInput::Confirm,&mut av),Some(super::MenuAction::ToggleEventConsole));
    }
    #[test]
    fn command_console_is_an_explicit_debug_menu_action() {
        let mut av=input_config::AvConfig::default();let mut menu=super::Menu::unlocked_for_test(true,true);
        menu.atlas_section(super::clan_atlas::Domain::Debug,0);
        at(&mut menu,super::Row::OpenCommandConsole);
        assert_eq!(menu.handle(super::MenuInput::Confirm,&mut av),Some(super::MenuAction::OpenCommandConsole));
    }
    use super::*;
    #[test]
    fn debug_focus_reports_actual_dropdown_option_and_existing_geometry() {
        let mut menu=Menu::unlocked_for_test(true,true);let mut av=AvConfig::default();menu.open(Page::Debug);
        at(&mut menu,Row::ExtendedSaves);
        assert!(menu.debug_focus(852,480).dropdown.is_none());
        menu.handle(MenuInput::Confirm,&mut av);
        let focus=menu.debug_focus(852,480);let dropdown=focus.dropdown.unwrap();
        assert_eq!(dropdown.label,"Extended saves");assert_eq!(dropdown.index,0);
        let dd=menu.dropdown.as_ref().unwrap();let(x,y,w,h,_,_)=theme::dropdown_geometry(dd,852,480);assert_eq!(dropdown.panel,[x,y,w,h]);
        assert_eq!(dropdown.option,setting_options(Row::ExtendedSaves)[0]);
        menu.handle(MenuInput::Down,&mut av);
        let selected=menu.debug_focus(852,480).dropdown.unwrap();
        assert_eq!(selected.index,1);assert_eq!(selected.option,setting_options(Row::ExtendedSaves)[1]);
        assert_eq!(menu.rows()[menu.selected],Row::ExtendedSaves);
    }
    #[test]
    fn physical_pixel_layout_scales_proportionally_from_1080p_to_4k() {
        for n in [MAIN_ROWS.len(), INPUT_ROWS.len(), GAMEPAD_ROWS.len()] {
            let hd = layout(n, 1920, 1080);
            let uhd = layout(n, 3840, 2160);
            assert!(uhd.panel_w.abs_diff(2 * hd.panel_w) <= 2);
            assert!(uhd.line_h.abs_diff(2 * hd.line_h) <= 1);
            assert!(uhd.panel_x + uhd.panel_w <= 3840);
            assert!(uhd.panel_y + uhd.panel_h <= 2160);
            assert_eq!(hd.visible_rows, uhd.visible_rows);
        }
    }
    #[test]
    fn dropdown_cancel_and_commit_use_real_video_settings() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Video);
        at(&mut menu, Row::Screen);
        let original = av.screen.clone();
        menu.handle(MenuInput::Confirm, &mut av);
        assert!(menu.dropdown.is_some());
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(av.screen, original);
        menu.handle(MenuInput::Cancel, &mut av);
        assert_eq!(menu.page, Page::Video);
        assert!(menu.dropdown.is_none());
        menu.handle(MenuInput::Confirm, &mut av);
        menu.handle(MenuInput::Down, &mut av);
        let index = menu.dropdown.as_ref().unwrap().index;
        let expected = setting_options(Row::Screen)[index].clone();
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::Changed(Changed::Video))
        );
        assert_eq!(av.screen.to_uppercase(), expected);
    }

    #[test]
    fn fullscreen_menu_and_mouse_toggle_without_rebuilding_video_or_losing_scroll() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Video);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.rows()[menu.selected], Row::Fullscreen);
        assert_eq!(adjust(&mut menu, &mut av, MenuInput::Right), Some(MenuAction::Changed(Changed::Fullscreen)));
        assert_eq!(value_of(Row::Fullscreen, &av, &keys(), "Tab", None).unwrap(), "ON");
        assert!(AvConfig::parse(&av.serialize()).video_fullscreen);
        let (w,h) = (640,480);
        let l = menu.page_layout(w,h);
        let r=clan_atlas::row_rect(&menu,menu.selected,w,h).unwrap();
        assert_eq!(menu.click(r.x+10,r.y+5,w,h,true,&mut av),None);
        assert!(av.video_fullscreen);
        menu.handle(MenuInput::Down,&mut av);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::Changed(Changed::Fullscreen)));
        assert!(!av.video_fullscreen);
        at(&mut menu, Row::Back);
        assert!(menu.first_visible(l)>0);
        assert!(menu.first_visible(l)+l.visible_rows>=VIDEO_ROWS.len());
    }

    #[test]
    fn fps_dropdown_commits_and_cycles_without_rebuilding_video() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Video);
        at(&mut menu, Row::Fps);
        menu.handle(MenuInput::Confirm, &mut av);
        assert!(menu.dropdown.is_some());
        for _ in 0..3 { menu.handle(MenuInput::Down, &mut av); }
        assert_eq!(av.video_fps, "off");
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::Changed(Changed::Overlay)));
        assert_eq!(av.video_fps, "both");
        assert_eq!(AvConfig::parse(&av.serialize()).video_fps, "both");
        if let Ok(path) = std::env::var("FFTA_FPS_MENU_PREVIEW") {
            let mut rgba = vec![[70, 90, 110, 255]; 640 * 480];
            menu.draw(&av, &keys(), "Tab", "T", "Equal", "Minus", &mut rgba, 640, 480);
            let mut ppm = b"P6\n640 480\n255\n".to_vec();
            for pixel in rgba { ppm.extend_from_slice(&pixel[..3]); }
            std::fs::write(path, ppm).unwrap();
        }
        assert_eq!(adjust(&mut menu, &mut av, MenuInput::Left), Some(MenuAction::Changed(Changed::Overlay)));
        assert_eq!(av.video_fps, "external");
    }

    #[test]
    fn menu_shortcut_capture_is_available_for_keyboard_mouse_and_gamepad() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        for (page, expected) in [(Page::Inputs, Bind::Menu), (Page::MouseKeyboard, Bind::Menu), (Page::Gamepad, Bind::GamepadMenu)] {
            menu.end_capture();
            menu.open(page);
            at(&mut menu, Row::Menu);
            assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
            assert_eq!(menu.capturing(), Some(expected));
            assert_eq!(menu.handle(MenuInput::Down, &mut av), None);
            assert_eq!(menu.rows()[menu.selected], Row::Menu);
        }
    }
    #[test]
    fn scrolled_input_hit_test_and_tabs_keep_binding_targets() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Gamepad);
        at(&mut menu, Row::FfToggleKey);
        let l = menu.page_layout(640,480);
        let first = menu.first_visible(l);
        assert!(first > 0);
        let y = l.rows_y + (menu.selected - first) * l.line_h + 2;
        assert_eq!(menu.click(l.panel_x + 30, y, 640, 480, true, &mut av), None);
        assert_eq!(menu.capturing(), Some(Bind::GamepadFastForwardToggle));
        menu.end_capture();
        assert_eq!(
            select_tab(&mut menu, &mut av, MenuInput::Left),
            Some(MenuAction::InputDevice(Device::Keyboard))
        );
        assert_eq!(menu.page, Page::Inputs);
        assert_eq!(
            select_tab(&mut menu, &mut av, MenuInput::Right),
            Some(MenuAction::InputDevice(Device::Gamepad))
        );
        menu.handle(MenuInput::Cancel, &mut av);
        menu.atlas_section(clan_atlas::Domain::Game,6);
        assert_eq!(menu.page,Page::Inputs);
        assert_eq!(select_tab(&mut menu,&mut av,MenuInput::Right),Some(MenuAction::InputDevice(Device::Gamepad)));
    }
    #[test]
    fn third_input_tab_selects_independent_mouse_keyboard_profile() {
        let mut menu=Menu::unlocked_for_test(true,true);
        let mut av=AvConfig::default();
        menu.open(Page::Gamepad);
        assert_eq!(select_tab(&mut menu,&mut av,MenuInput::Right),None);
        assert_eq!(menu.page,Page::MouseKeyboard);
        assert_eq!(adjust(&mut menu,&mut av,MenuInput::Right),Some(MenuAction::InputDevice(Device::MouseKeyboard)));
        assert!(menu.mouse_mode.get());
        assert_eq!(menu.input_tab(0),None);
        assert_eq!(menu.page,Page::MouseKeyboard);
        at(&mut menu,Row::Key(Button::Up));
        menu.handle(MenuInput::Confirm,&mut av);
        assert_eq!(menu.capturing(),Some(Bind::Button(Button::Up)));
        menu.end_capture();
        assert_eq!(select_tab(&mut menu,&mut av,MenuInput::Left),Some(MenuAction::InputDevice(Device::Gamepad)));
        menu.input_tab(2);
        at(&mut menu,Row::MouseMode);
        assert_eq!(adjust(&mut menu,&mut av,MenuInput::Left),Some(MenuAction::InputDevice(Device::Keyboard)));
        assert!(!menu.mouse_mode.get());
        assert_eq!(menu.input_tab(0),Some(MenuAction::InputDevice(Device::Keyboard)));
    }
    #[test]
    fn segmented_controls_set_exact_values_by_mouse() {
        let mut menu=Menu::unlocked_for_test(true,true);let mut av=AvConfig::default();menu.open(Page::FastForward);
        at(&mut menu,Row::FastForward);
        let r=clan_atlas::row_rect(&menu,menu.selected,854,480).unwrap();
        menu.click(r.x+10,r.y+5,854,480,true,&mut av);assert!(menu.dropdown.is_some());
        menu.dropdown.as_mut().unwrap().index=35;
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::Changed(Changed::Gamut)));
        assert_eq!(av.ff_speed,800);
        at(&mut menu,Row::FastForwardAudio);let l=menu.page_layout(854,480);let(x,_)=theme::segment_geometry(l);
        let r=clan_atlas::row_rect(&menu,menu.selected,854,480).unwrap();
        menu.click(x+3,r.y+3,854,480,true,&mut av);assert_eq!(av.ff_audio,"silent");
        assert!(menu.dropdown.is_some());menu.dropdown.as_mut().unwrap().index=2;
        menu.handle(MenuInput::Confirm,&mut av);assert_eq!(av.ff_audio,"normal");
    }

    #[test]
    fn input_submenus_keep_keyboard_and_gamepad_captures_separate() {
        let mut menu = Menu::unlocked_for_test(true, false);
        let mut av = AvConfig::default();
        menu.atlas_section(clan_atlas::Domain::Game,6);
        assert_eq!(
            select_tab(&mut menu, &mut av, MenuInput::Right),
            Some(MenuAction::InputDevice(Device::Gamepad))
        );
        at(&mut menu, Row::Key(Button::A));
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.capturing(), Some(Bind::Gamepad(Button::A)));
        assert_eq!(
            value_of(Row::Key(Button::A), &av, &keys(), "Tab", menu.capturing()),
            Some("PRESS BUTTON".into())
        );
        menu.end_capture();
        for (row, binding) in [
            (Row::FfKey, Bind::GamepadFastForward),
            (Row::FfToggleKey, Bind::GamepadFastForwardToggle),
        ] {
            at(&mut menu, row);
            menu.handle(MenuInput::Confirm, &mut av);
            assert_eq!(menu.capturing(), Some(binding));
            menu.end_capture();
        }
        menu.handle(MenuInput::Cancel, &mut av);
        assert_eq!(menu.page, Page::Main);
        menu.open(Page::Gamepad);
        assert_eq!(
            select_tab(&mut menu, &mut av, MenuInput::Left),
            Some(MenuAction::InputDevice(Device::Keyboard))
        );
        at(&mut menu, Row::Key(Button::A));
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.capturing(), Some(Bind::Button(Button::A)));
    }

    fn adjust(menu: &mut Menu, av: &mut AvConfig, direction: MenuInput) -> Option<MenuAction> {
        let before=av.serialize();
        assert_eq!(menu.handle(MenuInput::Confirm,av),None);
        assert!(menu.dropdown.is_some());
        assert_eq!(menu.handle(direction,av),None);
        assert_eq!(av.serialize(),before,"pending choices must not change configuration");
        menu.handle(MenuInput::Confirm,av)
    }
    fn select_tab(menu: &mut Menu, av: &mut AvConfig, direction: MenuInput) -> Option<MenuAction> {
        menu.selected=0;
        assert_eq!(menu.handle(MenuInput::Up,av),None);
        assert_eq!(menu.handle(direction,av),None);
        menu.handle(MenuInput::Confirm,av)
    }

    fn keys() -> [String; 10] {
        input_config::InputConfig::default().keys
    }

    fn at(menu: &mut Menu, row: Row) {
        if menu.page==Page::Debug && !menu.rows().contains(&row) {
            let group=(0..8).find(|&i|clan_atlas::debug_rows(i).contains(&row)).expect("debug row still routed");
            menu.atlas_section(clan_atlas::Domain::Debug,group);
        }
        menu.selected=menu.rows().iter().position(|r|*r==row).expect("row on this page");
    }

    #[test]
    fn enhanced_audio_lives_in_audio_submenu() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.atlas_section(clan_atlas::Domain::Game,4);
        assert_eq!(menu.page, Page::Audio);
        at(&mut menu, Row::Audio);
        let act = adjust(&mut menu, &mut av, MenuInput::Right);
        assert_eq!(act, Some(MenuAction::Changed(Changed::Audio)));
        assert!(av.audio_enhanced);
    }

    #[test]
    fn mute_navigation_keeps_volume_and_reports_live_audio_change() {
        let mut av = AvConfig { audio_volume: 0.6, ..Default::default() };
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::Audio);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.rows()[menu.selected], Row::Mute);
        assert_eq!(adjust(&mut menu, &mut av, MenuInput::Right), Some(MenuAction::Changed(Changed::Audio)));
        assert_eq!(value_of(Row::Mute, &av, &keys(), "Tab", None).unwrap(), "ON");
        assert!(av.audio_mute);
        assert_eq!(av.audio_volume, 0.6);
        at(&mut menu, Row::Volume);
        adjust(&mut menu, &mut av, MenuInput::Left);
        assert!(av.audio_mute);
        assert_eq!(av.audio_volume, 0.5);
        at(&mut menu, Row::Mute);
        adjust(&mut menu, &mut av, MenuInput::Left);
        assert!(!av.audio_mute);
        assert_eq!(av.audio_volume, 0.5);
    }

    #[test]
    fn volume_steps_clamps_and_reports_audio() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::Audio);
        at(&mut menu, Row::Volume);
        assert_eq!(adjust(&mut menu,&mut av,MenuInput::Right), None);
        assert_eq!(
            adjust(&mut menu,&mut av,MenuInput::Left),
            Some(MenuAction::Changed(Changed::Audio))
        );
        assert!((av.audio_volume - 0.9).abs() < 1e-6);
        for _ in 0..20 {
            adjust(&mut menu,&mut av,MenuInput::Left);
        }
        assert_eq!(av.audio_volume, 0.0);
        assert_eq!(adjust(&mut menu,&mut av,MenuInput::Left), None);
    }

    #[test]
    fn fast_forward_multiplier_cycles_without_enabling() {
        let mut av = AvConfig::default();
        assert_eq!(av.ff_speed, 4);
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::FastForward);
        at(&mut menu, Row::FastForward);
        let seq: Vec<u32> = (0..5)
            .map(|_| {
                adjust(&mut menu, &mut av, MenuInput::Right);
                av.ff_speed
            })
            .collect();
        assert_eq!(seq, vec![420,440,460,480,500]);
        adjust(&mut menu, &mut av, MenuInput::Left);
        assert_eq!(av.ff_speed,480);
        assert_eq!(
            value_of(Row::FastForward, &av, &keys(), "Tab", None).unwrap(),
            "480%"
        );
    }

    #[test]
    fn bubbles_are_main_menu_sibling_and_all_main_rows_fit() {
        assert_eq!(MAIN_ROWS[4],Row::TextBubblesMenu);
        assert!(!VIDEO_ROWS.contains(&Row::TextBubblesMenu));
        assert_eq!(layout(MAIN_ROWS.len(),1280,720).visible_rows,MAIN_ROWS.len());
    }
    #[test]
    fn submenus_open_and_cancel_goes_back() {
        let mut av=AvConfig::default();let mut menu=Menu::unlocked_for_test(true,true);
        for(section,page)in [(2,Page::FastForward),(3,Page::TextBubbles),(4,Page::Audio),(5,Page::Video),(6,Page::Inputs)] {
            menu.atlas_section(clan_atlas::Domain::Game,section);assert_eq!(menu.page,page);
            assert_eq!(menu.handle(MenuInput::Cancel,&mut av),None);assert_eq!(menu.page,Page::Main);
        }
        assert_eq!(menu.handle(MenuInput::Cancel,&mut av),Some(MenuAction::Resume));
    }

    #[test]
    fn state_slots_cycle_and_emit_correct_controller_actions() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.atlas_section(clan_atlas::Domain::Game,1);
        at(&mut menu, Row::SaveStatesMenu);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.page, Page::SaveStates);
        adjust(&mut menu, &mut av, MenuInput::Left);
        assert_eq!(menu.state_slot, 8);
        adjust(&mut menu, &mut av, MenuInput::Right);
        assert_eq!(menu.state_slot, 0);
        for _ in 0..8 { adjust(&mut menu, &mut av, MenuInput::Right); }
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::SaveState {slot:9,overwrite:false}));
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert!(menu.state_message.contains("empty"));
        menu.state_slots[8].occupied = true;
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::LoadState(9)));
        menu.handle(MenuInput::Cancel, &mut av);
        assert_eq!(menu.rows()[menu.selected], Row::SaveStatesMenu);
    }

    #[test]
    fn isolated_session_actions_preserve_the_normal_game_and_require_explicit_close() {
        let mut menu = Menu::unlocked_for_test(true,true);
        let mut av = AvConfig::default();
        menu.open(Page::Debug);
        at(&mut menu, Row::CloseDebugSession);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        at(&mut menu, Row::StartDebugSession);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::StartDebugSession));
        assert_eq!(menu.page,Page::Debug);
        menu.set_debug_session(true);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        at(&mut menu,Row::CloseDebugSession);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::CloseDebugSession));
        assert_eq!(menu.page,Page::Debug);
        let mut normal = vec![[20,20,20,255];640*480];
        menu.set_debug_session(false);
        menu.draw(&av, &std::array::from_fn(|_| String::new()), "", "", "", "", &mut normal, 640, 480);
        let mut copy = vec![[20,20,20,255];640*480];
        menu.set_debug_session(true);
        menu.reset();
        menu.draw(&av, &std::array::from_fn(|_| String::new()), "", "", "", "", &mut copy, 640, 480);
        assert_ne!(copy, normal, "Debug session identity must remain visible in the fixed status area");
    }

    #[test]
    fn recording_actions_require_an_isolated_copy_and_keep_the_menu_paused() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Debug);
        at(&mut menu, Row::StartInputRecording);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert!(menu.debug_message.contains("isolated"));
        menu.set_debug_session(true);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::StartInputRecording));
        at(&mut menu, Row::StopInputRecording);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::StopInputRecording));
        assert_eq!(menu.page, Page::Debug);
        assert!(!av.debug_extended_saves);
        for (w, h) in [(640,480),(852,480),(1280,720)] {
            let l = menu.page_layout(w,h);
            assert!(l.rows_y+l.visible_rows*l.line_h<=h);
        }
    }

    #[test]
    fn code_trace_actions_require_isolated_copy() {
        let mut menu=Menu::unlocked_for_test(true,true);let mut av=AvConfig::default();menu.open(Page::Debug);
        at(&mut menu,Row::StartRecentEvents);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        at(&mut menu,Row::SaveRecentEvents);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        at(&mut menu,Row::StartGameplayTrace);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        at(&mut menu,Row::SaveGameplayReport);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        at(&mut menu,Row::CreateIssueReport);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        at(&mut menu,Row::StartCodeTrace);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        menu.set_debug_session(true);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::StartCodeTrace));
        at(&mut menu,Row::StartGameplayTrace);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::StartGameplayTrace));
        at(&mut menu,Row::SaveGameplayReport);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::SaveGameplayReport));
        at(&mut menu,Row::StartRecentEvents);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::StartRecentEvents));
        at(&mut menu,Row::SaveRecentEvents);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::SaveRecentEvents));
        at(&mut menu,Row::SaveCodeReport);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::SaveCodeReport));
        at(&mut menu,Row::CreateIssueReport);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::CreateIssueReport));
        assert_eq!(menu.page,Page::Debug);
    }

    #[test]
    fn frame_steps_are_isolated_actions_and_leave_menu_open() {
        let mut menu=Menu::unlocked_for_test(true,true);let mut av=AvConfig::default();menu.open(Page::Debug);
        at(&mut menu,Row::DebugStepOne);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert!(menu.debug_message.contains("isolated"));
        at(&mut menu,Row::StartAudioCapture);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        menu.set_debug_session(true);
        at(&mut menu,Row::StartAudioCapture);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::StartAudioCapture));
        at(&mut menu,Row::SaveAudioReport);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::SaveAudioReport));
        at(&mut menu,Row::InspectPC);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::InspectPC));
        at(&mut menu,Row::CaptureGraphics);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::CaptureGraphics));
        for (row, action) in [(Row::DebugStepOne,MenuAction::DebugStep(1)),(Row::DebugStepTen,MenuAction::DebugStep(10)),(Row::DebugResume,MenuAction::DebugResume)] {
            at(&mut menu,row);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(action));assert_eq!(menu.page,Page::Debug);
        }
    }

    #[test]
    fn replay_actions_are_available_in_the_normal_menu_without_extended_saves() {
        let mut menu=Menu::unlocked_for_test(true,true);let mut av=AvConfig::default();menu.open(Page::Debug);
        for (row,action) in [(Row::CompareReplay,MenuAction::CompareReplay),(Row::CaptureReplay,MenuAction::CaptureReplay),(Row::AuditReplay,MenuAction::AuditReplay),(Row::ReplayCase,MenuAction::ReplayCase),(Row::ChooseReplayProfile,MenuAction::ChooseReplayProfile),(Row::CancelReplay,MenuAction::CancelReplay)] {
            at(&mut menu,row);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(action));assert_eq!(menu.page,Page::Debug);
        }
        at(&mut menu,Row::Back);
        for(w,h)in[(640,480),(852,480),(1280,720)]{let l=menu.page_layout(w,h);assert!(menu.first_visible(l)>0);assert!(l.rows_y+l.visible_rows*l.line_h<=h);}
        assert!(!av.debug_extended_saves);
    }

    #[test]
    fn diagnosis_is_available_without_extended_saves_and_keeps_menu_paused() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Debug);
        at(&mut menu, Row::CopyDiagnosis);
        assert_eq!(menu.handle(MenuInput::Left, &mut av), None);
        assert!(menu.atlas.sidebar_focus);
        menu.handle(MenuInput::Right,&mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::CopyDiagnosis));
        assert_eq!(menu.page, Page::Debug);
        assert!(!av.debug_extended_saves);
        menu.diagnosis_result("Diagnosis copied; report saved.".into());
        assert!(menu.debug_message.contains("report saved"));
        let mut pixels = vec![[0; 4]; 640 * 480];
        menu.draw(&av, &std::array::from_fn(|_| String::new()), "", "", "", "", &mut pixels, 640, 480);
        assert!(pixels.iter().any(|p| *p != [0; 4]));
    }

    #[test]
    fn debug_toggle_gates_game_saves_and_snapshot_picker_reaches_99_then_clamps() {
        let mut menu = Menu::unlocked_for_test(true,true);
        let mut av = AvConfig::default();
        menu.open(Page::Debug);
        at(&mut menu,Row::GameSavesMenu);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.page,Page::Debug);
        at(&mut menu,Row::ExtendedSaves);
        assert_eq!(adjust(&mut menu,&mut av,MenuInput::Right),Some(MenuAction::Changed(Changed::Debug)));
        assert!(av.debug_extended_saves);
        menu.set_state_slots(vec![StateSlot::default();99]);
        menu.open(Page::SaveStates);
        adjust(&mut menu,&mut av,MenuInput::Left);
        assert_eq!(menu.state_slot,98);
        at(&mut menu,Row::SaveState);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::SaveState{slot:99,overwrite:false}));
        menu.state_slots[98].occupied=true;
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.page,Page::ReplaceState);
        menu.handle(MenuInput::Up,&mut av);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::SaveState{slot:99,overwrite:true}));
        menu.set_state_slots(vec![StateSlot::default();9]);
        assert_eq!(menu.state_slot,8);
        at(&mut menu,Row::StateSlot);
        adjust(&mut menu,&mut av,MenuInput::Right);
        assert_eq!(menu.state_slot,0);
    }

    #[test]
    fn numbered_game_save_actions_require_confirmation_and_keep_slot_identity() {
        let mut menu=Menu::unlocked_for_test(true,true);
        let mut av=AvConfig{debug_extended_saves:true,..Default::default()};
        menu.set_game_saves((1..=99).map(|slot| GameSaveSlot {
            name:format!("ffta-slot-{slot:02}.sav"),status:"Saved".into(),
            occupied:true,available:true,current:slot==1,
        }).collect());
        menu.open(Page::Debug);
        at(&mut menu,Row::GameSavesMenu);
        menu.handle(MenuInput::Confirm,&mut av);
        assert_eq!(menu.page,Page::GameSaves);
        adjust(&mut menu,&mut av,MenuInput::Left);
        assert_eq!(menu.game_save_slot,98);
        at(&mut menu,Row::UseGameSave);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.page,Page::SwitchGameSave);
        assert_eq!(menu.rows()[menu.selected],Row::Back);
        menu.handle(MenuInput::Confirm,&mut av); // Repeated confirm safely cancels.
        assert_eq!(menu.page,Page::GameSaves);
        menu.handle(MenuInput::Confirm,&mut av);
        menu.handle(MenuInput::Up,&mut av);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::SwitchGameSave(99)));
        at(&mut menu,Row::CopyGameSave);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.page,Page::ReplaceGameSave);
        menu.handle(MenuInput::Up,&mut av);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::CopyGameSave{slot:99,overwrite:true}));
        menu.game_save_slots[98].occupied=false;
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::CopyGameSave{slot:99,overwrite:false}));
        menu.game_save_slots[98].available=false;
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
    }

    #[test]
    fn undo_action_tracks_last_operation_independently_of_selected_slot() {
        let mut menu = Menu::unlocked_for_test(true,true);
        let mut av = AvConfig::default();
        menu.open(Page::SaveStates);
        at(&mut menu,Row::UndoState);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.state_message,"Nothing to undo.");
        menu.set_state_undo(Some("Undo load (slot 9)".into()));
        at(&mut menu,Row::StateSlot);
        adjust(&mut menu,&mut av,MenuInput::Right);
        at(&mut menu,Row::UndoState);
        assert_eq!(menu.state_slot,1);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::UndoState));
        menu.reset(); // Closing/reopening the menu does not discard the session undo.
        menu.open(Page::SaveStates);
        at(&mut menu,Row::UndoState);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::UndoState));
        menu.set_state_undo(None);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
    }

    #[test]
    fn restoring_backup_requires_confirmation_and_pins_the_chosen_generation() {
        let mut menu=Menu::unlocked_for_test(true,true);let mut av=AvConfig::default();
        menu.set_save_backups([BackupInfo{generation:Some(7),status:"Ready".into()},BackupInfo::default()]);
        menu.open(Page::SaveBackups);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.page,Page::RestoreBackup);
        assert_eq!(menu.rows()[menu.selected],Row::Back);
        menu.handle(MenuInput::Confirm,&mut av); // Default Cancel does not restore.
        assert_eq!(menu.page,Page::SaveBackups);
        menu.handle(MenuInput::Confirm,&mut av);
        menu.set_save_backups([BackupInfo{generation:Some(8),status:"Ready".into()},BackupInfo::default()]);
        menu.handle(MenuInput::Up,&mut av);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::RestoreBackup(7)));
        assert_eq!(menu.page,Page::SaveBackups);
        menu.handle(MenuInput::Down,&mut av); // Empty second copy.
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert!(menu.backup_message.contains("not available"));
    }

    #[test]
    fn main_and_video_have_distinct_correct_layouts() {
        let mut menu=Menu::unlocked_for_test(true,true);let fixed=clan_atlas::geometry(1280,720);
        assert_eq!(crate::atlas_theme::overview_rects(1280,720).len(),9);
        menu.open(Page::Video);let video=menu.page_layout(1280,720);
        assert_eq!(video.panel_x,fixed.body.x);assert_eq!(video.panel_w,fixed.body.w);
        assert_eq!(clan_atlas::columns(&menu,1280,720),2);
        assert!(video.rows_y>video.panel_y+video.pad);
    }

    #[test]
    fn replacing_a_slot_requires_explicit_controller_confirmation() {
        let mut menu = Menu::unlocked_for_test(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::SaveStates);
        menu.state_slots[0].occupied = true;
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::ReplaceState);
        assert_eq!(menu.rows()[menu.selected], Row::Back);
        menu.handle(MenuInput::Confirm, &mut av); // Repeated A safely cancels.
        assert_eq!(menu.page, Page::SaveStates);
        menu.handle(MenuInput::Confirm, &mut av);
        menu.handle(MenuInput::Up, &mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::SaveState {slot:1,overwrite:true}));
        assert_eq!(menu.page, Page::SaveStates);
    }

    #[test]
    fn screen_cycles_through_known_models() {
        let mut av = AvConfig::default();
        av.screen = "frontlit".into();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::Video);
        at(&mut menu, Row::Screen);
        adjust(&mut menu, &mut av, MenuInput::Right);
        assert!(ScreenKind::by_name(&av.screen).is_some());
        for _ in 0..ScreenKind::ALL.len() {
            adjust(&mut menu, &mut av, MenuInput::Right);
        }
        assert!(ScreenKind::by_name(&av.screen).is_some());
    }

    #[test]
    fn brightness_steps_within_range_and_rebuilds_video() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::Video);
        at(&mut menu, Row::Brightness);
        assert_eq!(
            adjust(&mut menu,&mut av,MenuInput::Right),
            Some(MenuAction::Changed(Changed::Video))
        );
        assert!((av.video_brightness - 1.1).abs() < 1e-6);
        for _ in 0..30 {
            adjust(&mut menu,&mut av,MenuInput::Right);
        }
        assert!((av.video_brightness - 1.5).abs() < 1e-6);
    }

    #[test]
    fn key_row_starts_capture_and_freezes_menu() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::Inputs);
        at(&mut menu, Row::Key(Button::Start));
        assert_eq!(menu.capturing(), None);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.capturing(), Some(Bind::Button(Button::Start)));
        // Everything is ignored while waiting for a key.
        let before = menu.selected;
        assert_eq!(menu.handle(MenuInput::Down, &mut av), None);
        assert_eq!(menu.handle(MenuInput::Cancel, &mut av), None);
        assert_eq!(menu.selected, before);
        assert_eq!(menu.page, Page::Inputs);
        let v = value_of(
            Row::Key(Button::Start),
            &av,
            &keys(),
            "Tab",
            menu.capturing(),
        );
        assert_eq!(v.unwrap(), "PRESS KEY");
        menu.end_capture();
        assert_eq!(menu.capturing(), None);
        let v = value_of(Row::Key(Button::Start), &av, &keys(), "Tab", None);
        assert_eq!(v.unwrap(), "ENTER");
    }

    #[test]
    fn confirm_opens_pending_choice_and_second_confirm_applies_it() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::Audio);
        at(&mut menu, Row::Audio);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert!(!av.audio_enhanced);
        assert_eq!(menu.handle(MenuInput::Right,&mut av),None);
        assert!(!av.audio_enhanced);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::Changed(Changed::Audio)));
        assert!(av.audio_enhanced);
        // Same for the fast-forward submenu multiplier.
        menu.reset();
        menu.open(Page::FastForward);
        at(&mut menu, Row::FastForward);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(av.ff_speed, 4);
    }

    #[test]
    fn ff_key_row_captures_the_fast_forward_binding() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::Inputs);
        at(&mut menu, Row::FfKey);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.capturing(), Some(Bind::FastForward));
        let v = value_of(Row::FfKey, &av, &keys(), "Tab", menu.capturing());
        assert_eq!(v.unwrap(), "PRESS KEY");
        menu.end_capture();
        let v = value_of(Row::FfKey, &av, &keys(), "Tab", None);
        assert_eq!(v.unwrap(), "TAB");
    }

    #[test]
    fn toggle_key_and_fast_forward_audio_are_configurable() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.open(Page::FastForward);
        at(&mut menu, Row::FastForwardAudio);
        adjust(&mut menu, &mut av, MenuInput::Right);
        assert_eq!(av.ff_audio, "fast");
        adjust(&mut menu, &mut av, MenuInput::Right);
        assert_eq!(av.ff_audio, "normal");
        assert_eq!(value_of(Row::FastForwardAudio, &av, &keys(), "Tab", None).unwrap(), "1X DELAYED");
        adjust(&mut menu, &mut av, MenuInput::Right);
        assert_eq!(av.ff_audio, "silent");
        menu.open(Page::Inputs);
        at(&mut menu, Row::FfToggleKey);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.capturing(), Some(Bind::FastForwardToggle));
    }

    #[test]
    fn resume_and_quit_actions() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        at(&mut menu, Row::Resume);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::Resume)
        );
        assert_eq!(
            menu.handle(MenuInput::Cancel, &mut av),
            Some(MenuAction::Resume)
        );
        menu.atlas_section(clan_atlas::Domain::Game,7);
        at(&mut menu, Row::Quit);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::Exit);
        assert_eq!(menu.rows()[menu.selected], Row::QuitNo);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::System);
        menu.atlas_section(clan_atlas::Domain::Game,7);
        at(&mut menu, Row::Quit);
        menu.handle(MenuInput::Confirm, &mut av);
        menu.handle(MenuInput::Up, &mut av);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::Quit)
        );
    }

    #[test]
    fn restart_is_a_main_menu_action_and_errors_keep_navigation_available() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.atlas_section(clan_atlas::Domain::Game,7);
        at(&mut menu, Row::RestartRom);
        assert_eq!(menu.handle(MenuInput::Left, &mut av), None);
        assert!(menu.atlas.sidebar_focus);
        menu.handle(MenuInput::Right,&mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::RestartRom));
        menu.session_result("Cannot restart ROM: save is unavailable".into());
        for (w,h) in [(640,480), (1280,720), (3840,2160)] {
            let l = menu.page_layout(w,h);
            let first = menu.first_visible(l);
            let y = l.rows_y + (menu.selected - first) * l.line_h + l.line_h / 2;
            assert_eq!(menu.click(l.panel_x + l.pad + 1, y, w, h, true, &mut av), Some(MenuAction::RestartRom));
            let mut px = vec![[0,0,0,255];w*h];
            let keys = std::array::from_fn(|_| String::new());
            menu.draw(&av,&keys,"Tab","t","+","-",&mut px,w,h);
        }
        menu.reset();
        assert!(menu.session_message.is_empty());
        assert_eq!(menu.handle(MenuInput::Cancel,&mut av),None);
        assert_eq!(menu.handle(MenuInput::Cancel,&mut av),Some(MenuAction::Resume));
    }

    #[test]
    fn switch_rom_submenu_can_resume_or_return_to_launcher() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.atlas_section(clan_atlas::Domain::Game,7);
        at(&mut menu, Row::SwitchRomMenu);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::SwitchRom);
        assert_eq!(menu.rows()[menu.selected], Row::ReturnToGame);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::Resume)
        );

        menu.reset();
        menu.atlas_section(clan_atlas::Domain::Game,7);
        at(&mut menu, Row::SwitchRomMenu);
        menu.handle(MenuInput::Confirm, &mut av);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::SwitchRom)
        );
        menu.reset();
        menu.atlas_section(clan_atlas::Domain::Game,7);
        at(&mut menu, Row::SwitchRomMenu);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.handle(MenuInput::Cancel, &mut av), None);
        assert_eq!(menu.page, Page::System);
        assert_eq!(menu.rows()[menu.selected], Row::SwitchRomMenu);
    }

    #[test]
    fn knob_steps_wrap_through_auto() {
        assert_eq!(step_knob("auto", true), "0.0");
        assert_eq!(step_knob("0.0", false), "auto");
        assert_eq!(step_knob("1.0", true), "auto");
        assert_eq!(step_knob("0.5", true), "0.6");
    }

    #[test]
    fn navigation_wraps() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.selected = 0;
        menu.handle(MenuInput::Up, &mut av);
        assert_eq!(menu.selected, clan_atlas::OVERVIEW_ROWS.len() - 1);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn tool_input_slots_keep_parent_cursor_and_only_request_internal_selection(){
        let mut menu=Menu::unlocked_for_test(true,true);let mut av=AvConfig::default();menu.open(Page::Debug);
        at(&mut menu,Row::ToolInputsMenu);assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.page,Page::ToolInputs);
        for slot in ContentSlot::ALL {
            at(&mut menu,Row::ToolInput(slot));assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::ChooseToolInput(slot)));
            menu.open_content(slot.label().into(),Vec::new(),"No matching inputs; Refresh explicitly".into(),0);
            menu.content_scope(vec!["snapshot".into()],None);
            assert_eq!(menu.handle(MenuInput::Cancel,&mut av),Some(MenuAction::ContentCancel));
            assert_eq!(menu.page,Page::ToolInputs);assert_eq!(menu.rows()[menu.selected],Row::ToolInput(slot));
        }
        menu.handle(MenuInput::Cancel,&mut av);assert_eq!(menu.page,Page::Debug);assert_eq!(menu.rows()[menu.selected],Row::ToolInputsMenu);
    }

    #[test]
    fn tool_identity_and_editor_footer_use_their_actual_custom_geometry(){
        let mut menu=Menu::unlocked_for_test(true,true);menu.set_debug_session(true);
        menu.open_tool(DebugTool::Step,vec![ToolField{key:"frames".into(),label:"frames".into(),value:"1".into(),help:String::new(),kind:ToolFieldKind::Text}],String::new());
        let mut px=vec![[0,0,0,255];940*620];
        for editing in [false,true]{
            if editing{menu.content_start_search();}
            let cap=diagnostics::capture(||menu.draw(&AvConfig::default(),&keys(),"","","","",&mut px,940,620)).unwrap();
            assert!(cap.runs.iter().any(|r|r.text.starts_with("DEBUG COPY / ")));
            assert!(!cap.runs.iter().any(|r|r.text=="DEBUG COPY"||r.text.starts_with("0 items")));
            assert_eq!(menu.page_layout(940,620).panel_x,menu.debug_focus(940,620).panel[0]);
        }
    }

    #[test]
    fn draw_does_not_panic_on_any_page() {
        let av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(false, false);
        for page in [
            Page::Debug,
            Page::ToolInputs,
            Page::DebugTools,
            Page::ScriptLibrary,
            Page::DebugTool,
            Page::GameSaves,
            Page::SwitchGameSave,
            Page::ReplaceGameSave,
            Page::Main,
            Page::SaveStates,
            Page::ReplaceState,
            Page::SaveBackups,
            Page::RestoreBackup,
            Page::Exit,
            Page::SwitchRom,
            Page::FastForward,
            Page::Audio,
            Page::Video,
            Page::Inputs,
            Page::Gamepad,
        ] {
            menu.page = page;
            let mut rgba = vec![[10u8, 10, 10, 255]; 240 * 160];
            menu.draw(
                &av,
                &keys(),
                "Tab",
                "T",
                "Equal",
                "Minus",
                &mut rgba,
                240,
                160,
            );
            assert!(rgba.iter().any(|p| p[0] != 10 || p[1] != 10 || p[2] != 10));
        }
    }

    #[test]
    fn mouse_hover_and_click_hit_rows() {
        let mut av = AvConfig::default();
        let mut menu = Menu::unlocked_for_test(true, true);
        let (w, h) = (854usize, 480usize);

        // Main page: hover selects, left click on "Audio >" opens it.
        let r=clan_atlas::geometry(w,h).nav[4];
        menu.hover(r.x+10,r.y+5,w,h);assert_eq!(menu.atlas.nav_cursor,5);
        assert_eq!(menu.click(r.x+10,r.y+5,w,h,true,&mut av),None);
        assert_eq!(menu.page, Page::Audio);

        // Clicking opens choices; right click discards, selecting a choice commits.
        for row in [Row::Mute,Row::Audio] {
            at(&mut menu,row);
            let r=clan_atlas::row_rect(&menu,menu.selected,w,h).unwrap();
            assert_eq!(menu.click(r.x+5,r.y+5,w,h,true,&mut av),None);
            menu.handle(MenuInput::Down,&mut av);
            let original=av.serialize();
            assert_eq!(menu.click(0,0,w,h,false,&mut av),None);
            assert_eq!(av.serialize(),original);
            menu.click(r.x+5,r.y+5,w,h,true,&mut av);
            menu.handle(MenuInput::Down,&mut av);
            let dd=menu.dropdown.as_ref().unwrap();
            let (x,y,_,_,_,first)=theme::dropdown_geometry(dd,w,h);
            let pitch=(clan_atlas::geometry(w,h).body.h/15).clamp(14,40);
            assert_eq!(menu.click(x+10,y+6+pitch*(dd.index-first)+pitch/2,w,h,true,&mut av),Some(MenuAction::Changed(Changed::Audio)));
        }
        assert!(av.audio_mute && av.audio_enhanced);assert_eq!(av.audio_volume,1.);
        assert_eq!(menu.click(0,0,w,h,true,&mut av),None);
        assert_eq!(menu.page,Page::Audio);
        assert_eq!(menu.click(0,0,w,h,false,&mut av),None);
        assert_eq!(menu.page,Page::Main);
    }

}
