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
mod font;
pub mod fps;
mod controller_icons;
mod menu_icons;
mod theme;
mod save_states;
mod save_backups;
pub use save_backups::BackupInfo;
pub use save_states::StateSlot;

use input_config::{AvConfig, Button, Device, GamepadLayout, InputConfig};
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
    /// Close the menu and continue the game.
    Resume,
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
    RestoreBackup(u64),
}

/// Which A/V surface a [`MenuAction::Changed`] touched, so the play loop
/// rebuilds only the affected objects.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Changed {
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

const MAIN_ROWS: [Row; 10] = [
    Row::Resume,
    Row::SaveStatesMenu,
    Row::SaveBackupsMenu,
    Row::FastForwardMenu,
    Row::TextBubblesMenu,
    Row::AudioMenu,
    Row::VideoMenu,
    Row::InputsMenu,
    Row::SwitchRomMenu,
    Row::Quit,
];

const AUDIO_ROWS: [Row; 4] = [Row::Volume, Row::Mute, Row::Audio, Row::Back];
const EXIT_ROWS: [Row; 2] = [Row::QuitYes, Row::QuitNo];
const SWITCH_ROM_ROWS: [Row; 2] = [Row::ReturnToGame, Row::ChangeRom];
const FAST_FORWARD_ROWS: [Row; 3] = [Row::FastForward, Row::FastForwardAudio, Row::Back];

/// Scaling modes offered by the menu (`av.scaling`): "off" keeps the
/// original exact area-average scaler.
const SCALING_MODES: [&str; 4] = ["off", "nearest", "sharp", "epx"];

/// Aspect ratios offered by the menu (`av.video_aspect`). Applied on resume.
const ASPECT_MODES: [&str; 7] = ["auto", "3:2", "16:10", "16:9", "20:9", "21:9", "32:9"];

const VIDEO_ROWS: [Row; 11] = [
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
const FF_SPEEDS: [u32; 3] = [2, 4, 8];

impl Row {
    fn label(self) -> &'static str {
        match self {
            Row::Menu => "Open menu",
            Row::MouseMode => "Mouse & Keyboard",
            Row::Resume => "Resume",
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

    /// Is this an action row (no value to cycle) rather than a setting row?
    fn is_action(self) -> bool {
        matches!(
            self,
            Row::Backup(_) | Row::ConfirmRestoreBackup
                | Row::SaveStateConfirm
                | Row::SaveState
                | Row::LoadState
                | Row::UndoState
                | Row::Resume
                | Row::Menu
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
                | Row::Back
        )
    }

    /// The submenu this row opens, if any.
    fn opens(self) -> Option<Page> {
        match self {
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
    let line_h = ((if main { 40. } else { 44. }) * scale).round().max(12.) as usize;
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

    fn back_to_main(&mut self) {
        while let Some(parent) = self.parents.pop() {
            self.page = parent.page;
            self.selected = parent.selected;
            if self.page == Page::Main {
                return;
            }
        }
        self.page = Page::Main;
        self.selected = 0;
    }

    fn reset(&mut self) {
        self.page = Page::Main;
        self.selected = 0;
        self.parents.clear();
    }
}

/// The pause-overlay model and its public input/draw boundary.
pub struct Menu {
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
    input_gamepad: std::cell::Cell<u8>,
    mouse_mode: std::cell::Cell<bool>,
    controller_layout: std::cell::Cell<GamepadLayout>,
    menu_binding: std::cell::RefCell<String>,
    state_slot: usize,
    state_slots: [StateSlot; 9],
    state_message: String,
    state_undo: Option<String>,
    save_backups: [BackupInfo;2],
    backup_selection: Option<(u64,u8)>,
    backup_message: String,
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
}

impl Menu {
    /// Open a fresh menu with the first row selected. `audio_live` tells
    /// the menu whether enhanced audio takes effect immediately. The second
    /// argument is kept for source compatibility (the gamut row now lives
    /// in the launcher) and is ignored.
    pub fn new(audio_live: bool, _gamut_live: bool) -> Menu {
        Menu {
            navigation: Navigation::new(),
            audio_live,
            capture: None,
            scroll: std::cell::Cell::new(0),
            dropdown: None,
            input_gamepad: std::cell::Cell::new(0),
            mouse_mode: std::cell::Cell::new(false),
            controller_layout: std::cell::Cell::new(GamepadLayout::Xbox),
            menu_binding: std::cell::RefCell::new("Escape".into()),
            state_slot: 0,
            state_slots: std::array::from_fn(|_| StateSlot::default()),
            state_message: String::new(),
            state_undo: None,
            save_backups: std::array::from_fn(|_| BackupInfo::default()),
            backup_selection: None,
            backup_message: String::new(),
        }
    }

    /// Reset to the top of the main page (called on each open so the cursor
    /// doesn't persist a stale position across pauses).
    pub fn reset(&mut self) {
        self.navigation.reset();
        self.state_message.clear();
        self.backup_message.clear();
        self.backup_selection = None;
        self.capture = None;
        self.scroll.set(0);
        self.dropdown = None;
    }

    /// Slot metadata is read only while opening the menu or after an operation.
    pub fn set_state_slots(&mut self, slots: [StateSlot; 9]) { self.state_slots = slots; }

    pub fn state_result(&mut self, message: String) { self.state_message = message; }
    pub fn set_state_undo(&mut self, label: Option<String>) { self.state_undo = label; }
    pub fn set_save_backups(&mut self, copies: [BackupInfo;2]) { self.save_backups = copies; }
    pub fn backup_result(&mut self, message: String) { self.backup_message = message; }

    fn page_layout(&self, w: usize, h: usize) -> Layout {
        if matches!(self.page, Page::SaveStates | Page::ReplaceState) {
            save_states::layout(rows_of(self.page).len(), w, h)
        } else if matches!(self.page,Page::SaveBackups|Page::RestoreBackup) {
            save_backups::layout(self.page,rows_of(self.page).len(),w,h)
        } else { layout_kind(rows_of(self.page).len(), w, h,self.page==Page::Main) }
    }

    fn change_state_slot(&mut self, forward: bool) {
        self.state_slot = (self.state_slot + if forward { 1 } else { 8 }) % 9;
        self.state_message.clear();
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
        self.navigation.open(page);
        self.scroll.set(0);
        self.dropdown = None;
    }

    fn back_to_main(&mut self) {
        self.scroll.set(0);
        self.dropdown = None;
        if matches!(self.page,Page::ReplaceState|Page::RestoreBackup) {
            self.backup_selection = None;
            if let Some(parent) = self.navigation.parents.pop() {
                self.navigation.page = parent.page;
                self.navigation.selected = parent.selected;
            } else { self.navigation.reset(); }
        } else { self.navigation.back_to_main(); }
    }
    fn first_visible(&self, l: Layout) -> usize {
        let len = rows_of(self.page).len()
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
        let opts = setting_options(row);
        if opts.is_empty() {
            return;
        }
        let index = opts
            .iter()
            .position(|s| *s == setting_value(row, av))
            .unwrap_or(0);
        self.dropdown = Some(Dropdown {
            row,
            index,
            first: std::cell::Cell::new(0),
        });
    }

    /// Feed one navigation event. Mutates `av` for value changes and
    /// returns the action the play loop must take, if any. `Up`/`Down`
    /// move the cursor (no action). `Left`/`Right` cycle the selected
    /// setting. `Confirm` activates an action row (and does nothing on a
    /// value setting).
    /// `Cancel` goes back from a submenu, or resumes from the main page.
    /// While a key capture is pending every event is ignored — the play
    /// loop decides what ends it.
    pub fn handle(&mut self, input: MenuInput, av: &mut AvConfig) -> Option<MenuAction> {
        if self.capture.is_some() {
            return None;
        }
        if self.page == Page::SaveStates && matches!(input, MenuInput::Left | MenuInput::Right) {
            self.change_state_slot(input == MenuInput::Right);
            return None;
        }
        if let Some(mut dd) = self.dropdown.take() {
            let opts = setting_options(dd.row);
            match input {
                MenuInput::Cancel => return None,
                MenuInput::Confirm => {
                    return choose_setting(dd.row, &opts[dd.index], av).map(MenuAction::Changed);
                }
                MenuInput::Up | MenuInput::Left => {
                    dd.index = (dd.index + opts.len() - 1) % opts.len()
                }
                MenuInput::Down | MenuInput::Right => dd.index = (dd.index + 1) % opts.len(),
            }
            self.dropdown = Some(dd);
            return None;
        }
        if matches!(self.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard)
            && matches!(input, MenuInput::Left | MenuInput::Right)
        {
            let tab=match self.page {Page::Gamepad=>1,Page::MouseKeyboard=>2,_=>0};
            if self.page==Page::MouseKeyboard && rows_of(self.page)[self.selected]==Row::MouseMode {
                return self.toggle_mouse_mode();
            }
            let mut next=if input==MenuInput::Right {(tab+1)%3}else{(tab+2)%3};
            if next==0 && self.mouse_mode.get() {next=if input==MenuInput::Right {1}else{2};}
            return self.input_tab(next);
        }
        let rows = rows_of(self.page);
        let row = rows[self.selected];
        match input {
            MenuInput::Up => {
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
                Row::MouseMode => self.toggle_mouse_mode(),
                Row::Resume => Some(MenuAction::Resume),
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
                Row::StateSlot => { self.change_state_slot(true); None },
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
                        self.state_message = format!("Slot {} is empty.", self.state_slot + 1);
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
                // Confirm does nothing on value settings: they change with
                // Left/Right only (so Enter can't flip an ON/OFF by accident).
                _ => {
                    self.open_dropdown(row, av);
                    None
                }
            },
            MenuInput::Left => {
                if row.is_action() {
                    None
                } else {
                    cycle(row, av, false).map(MenuAction::Changed)
                }
            }
            MenuInput::Right => {
                if row == Row::InputsMenu {
                    self.handle(MenuInput::Confirm, av)
                } else if let Some(p) = row.opens() {
                    self.open(p);
                    None
                } else if row.is_action() {
                    None
                } else {
                    cycle(row, av, true).map(MenuAction::Changed)
                }
            }
        }
    }

    /// Row index under frame-pixel `(x, y)`, if any.
    fn row_at(&self, x: usize, y: usize, w: usize, h: usize) -> Option<usize> {
        let rows = rows_of(self.page);
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
            && !matches!(self.page, Page::SaveStates | Page::ReplaceState | Page::SaveBackups | Page::RestoreBackup)
            && x < l.panel_x + l.panel_w - l.pad - l.line_h * 3 {
            return None;
        }
        Some(index)
    }

    /// Pointer moved: highlight the row under it (ignored while a key
    /// capture is pending). `(x, y)` are frame pixels, `w`/`h` the frame size.
    pub fn hover(&mut self, x: usize, y: usize, w: usize, h: usize) {
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
            self.selected = i;
        }
    }

    /// Mouse click at frame-pixel `(x, y)`. Left on an action/submenu row =
    /// Confirm; left on a value row = next value; right on a value row =
    /// previous value; right on an action row or outside any row = Cancel
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
        if self.capture.is_some() {
            return None;
        }
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
        if matches!(self.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard) {
            let l = self.page_layout(w,h);
            if y >= l.panel_y + l.pad + l.line_h
                && y < l.rows_y - 2
                && x >= l.panel_x + l.pad
                && x < l.panel_x + l.panel_w - l.pad
            {
                return self.input_tab((((x-l.panel_x-l.pad)*3)/(l.panel_w-2*l.pad)).min(2) as u8);
            }
        }
        match self.row_at(x, y, w, h) {
            Some(i) => {
                self.selected = i;
                let row = rows_of(self.page)[i];
                if left {
                    if let Some(choice) =
                        theme::segment_hit(row, self.page_layout(w,h), x)
                    {
                        match row {
                            Row::FastForward => av.ff_speed = FF_SPEEDS[choice],
                            Row::FastForwardAudio => {
                                av.ff_audio = ["normal", "silent", "fast"][choice].into()
                            }
                            _ => unreachable!(),
                        }
                        return Some(MenuAction::Changed(Changed::Gamut));
                    }
                    if !setting_options(row).is_empty() {
                        self.open_dropdown(row, av);
                        return None;
                    }
                }
                let input = if row.is_action() {
                    if left {
                        MenuInput::Confirm
                    } else {
                        MenuInput::Cancel
                    }
                } else if left {
                    MenuInput::Right
                } else {
                    MenuInput::Left
                };
                self.handle(input, av)
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
    /// Its parchment and brass palette follows the supplied HTML reference.
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
        theme::draw(
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
        );
    }

    pub fn draw_inputs(
        &self,
        av: &AvConfig,
        cfg: &InputConfig,
        rgba: &mut [[u8; 4]],
        w: usize,
        h: usize,
    ) {
        self.mouse_mode.set(cfg.mouse_keyboard);
        self.controller_layout.set(cfg.gamepad_layout);
        *self.menu_binding.borrow_mut() = if self.page == Page::Gamepad { cfg.menu_pad.clone() } else { cfg.menu_key.clone() };
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
        Row::MouseMode => return None,
        Row::Menu => "HOLD + RELEASE".to_string(),
        Row::FastForwardMenu | Row::SaveStatesMenu | Row::SaveBackupsMenu => ">".to_string(),
        Row::Backup(_) | Row::ConfirmRestoreBackup | Row::StateSlot | Row::SaveStateConfirm | Row::SaveState | Row::LoadState | Row::UndoState | Row::Resume
        | Row::Quit
        | Row::QuitYes
        | Row::QuitNo
        | Row::ReturnToGame
        | Row::ChangeRom
        | Row::Back
        | Row::ResetGamepad => return None,
        Row::AudioMenu | Row::TextBubblesMenu
                | Row::VideoMenu | Row::InputsMenu | Row::SwitchRomMenu => ">".to_string(),
        Row::FastForward => {
            if av.ff_speed <= 1 {
                "OFF".to_string()
            } else {
                format!("{}X", av.ff_speed)
            }
        }
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
        Row::Menu => None,
        Row::MouseMode => None,
        Row::Backup(_) | Row::ConfirmRestoreBackup | Row::StateSlot | Row::SaveStateConfirm | Row::SaveState | Row::LoadState | Row::UndoState | Row::Resume
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
            let cur = FF_SPEEDS
                .iter()
                .copied()
                .find(|s| *s == av.ff_speed)
                .unwrap_or(4);
            av.ff_speed = step_enum(&FF_SPEEDS, cur, forward);
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
    use super::*;
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
        let mut menu = Menu::new(true, true);
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
        let mut menu = Menu::new(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Video);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(rows_of(menu.page)[menu.selected], Row::Fullscreen);
        assert_eq!(menu.handle(MenuInput::Right, &mut av), Some(MenuAction::Changed(Changed::Fullscreen)));
        assert_eq!(value_of(Row::Fullscreen, &av, &keys(), "Tab", None).unwrap(), "ON");
        assert!(AvConfig::parse(&av.serialize()).video_fullscreen);
        let (w,h) = (640,480);
        let l = menu.page_layout(w,h);
        assert_eq!(menu.click(l.panel_x+l.pad+20, l.rows_y+l.line_h+2,w,h,true,&mut av),
            Some(MenuAction::Changed(Changed::Fullscreen)));
        assert!(!av.video_fullscreen);
        at(&mut menu, Row::Back);
        assert!(menu.first_visible(l)>0);
        assert!(menu.first_visible(l)+l.visible_rows>=VIDEO_ROWS.len());
    }

    #[test]
    fn fps_dropdown_commits_and_cycles_without_rebuilding_video() {
        let mut menu = Menu::new(true, true);
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
        assert_eq!(menu.handle(MenuInput::Left, &mut av), Some(MenuAction::Changed(Changed::Overlay)));
        assert_eq!(av.video_fps, "external");
    }

    #[test]
    fn menu_shortcut_capture_is_available_for_keyboard_mouse_and_gamepad() {
        let mut menu = Menu::new(true, true);
        let mut av = AvConfig::default();
        for (page, expected) in [(Page::Inputs, Bind::Menu), (Page::MouseKeyboard, Bind::Menu), (Page::Gamepad, Bind::GamepadMenu)] {
            menu.end_capture();
            menu.open(page);
            at(&mut menu, Row::Menu);
            assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
            assert_eq!(menu.capturing(), Some(expected));
            assert_eq!(menu.handle(MenuInput::Down, &mut av), None);
            assert_eq!(rows_of(menu.page)[menu.selected], Row::Menu);
        }
    }
    #[test]
    fn scrolled_input_hit_test_and_tabs_keep_binding_targets() {
        let mut menu = Menu::new(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::Gamepad);
        at(&mut menu, Row::FfToggleKey);
        let l = layout(GAMEPAD_ROWS.len(), 240, 160);
        let first = menu.first_visible(l);
        assert!(first > 0);
        let y = l.rows_y + (menu.selected - first) * l.line_h + 2;
        assert_eq!(menu.click(l.panel_x + 30, y, 240, 160, true, &mut av), None);
        assert_eq!(menu.capturing(), Some(Bind::GamepadFastForwardToggle));
        menu.end_capture();
        assert_eq!(
            menu.handle(MenuInput::Left, &mut av),
            Some(MenuAction::InputDevice(Device::Keyboard))
        );
        assert_eq!(menu.page, Page::Inputs);
        assert_eq!(
            menu.handle(MenuInput::Right, &mut av),
            Some(MenuAction::InputDevice(Device::Gamepad))
        );
        menu.handle(MenuInput::Cancel, &mut av);
        at(&mut menu, Row::InputsMenu);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::InputDevice(Device::Gamepad))
        );
    }
    #[test]
    fn third_input_tab_selects_independent_mouse_keyboard_profile() {
        let mut menu=Menu::new(true,true);
        let mut av=AvConfig::default();
        menu.open(Page::Gamepad);
        assert_eq!(menu.handle(MenuInput::Right,&mut av),None);
        assert_eq!(menu.page,Page::MouseKeyboard);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::InputDevice(Device::MouseKeyboard)));
        assert!(menu.mouse_mode.get());
        assert_eq!(menu.input_tab(0),None);
        assert_eq!(menu.page,Page::MouseKeyboard);
        at(&mut menu,Row::Key(Button::Up));
        menu.handle(MenuInput::Confirm,&mut av);
        assert_eq!(menu.capturing(),Some(Bind::Button(Button::Up)));
        menu.end_capture();
        assert_eq!(menu.handle(MenuInput::Right,&mut av),Some(MenuAction::InputDevice(Device::Gamepad)));
        menu.input_tab(2);
        at(&mut menu,Row::MouseMode);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),Some(MenuAction::InputDevice(Device::Keyboard)));
        assert!(!menu.mouse_mode.get());
        assert_eq!(menu.input_tab(0),Some(MenuAction::InputDevice(Device::Keyboard)));
    }
    #[test]
    fn segmented_controls_set_exact_values_by_mouse() {
        let mut menu = Menu::new(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::FastForward);
        let l = layout(FAST_FORWARD_ROWS.len(), 568, 160);
        let (x, bw) = theme::segment_geometry(l);
        assert_eq!(
            menu.click(x + 2 * (bw + 2) + 3, l.rows_y + 3, 568, 160, true, &mut av),
            Some(MenuAction::Changed(Changed::Gamut))
        );
        assert_eq!(av.ff_speed, 8);
        menu.click(x + 3, l.rows_y + l.line_h + 3, 568, 160, true, &mut av);
        assert_eq!(av.ff_audio, "normal");
    }

    #[test]
    fn input_submenus_keep_keyboard_and_gamepad_captures_separate() {
        let mut menu = Menu::new(true, false);
        let mut av = AvConfig::default();
        at(&mut menu, Row::InputsMenu);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(
            menu.handle(MenuInput::Right, &mut av),
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
            menu.handle(MenuInput::Left, &mut av),
            Some(MenuAction::InputDevice(Device::Keyboard))
        );
        at(&mut menu, Row::Key(Button::A));
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.capturing(), Some(Bind::Button(Button::A)));
    }

    fn keys() -> [String; 10] {
        input_config::InputConfig::default().keys
    }

    fn at(menu: &mut Menu, row: Row) {
        menu.selected = rows_of(menu.page)
            .iter()
            .position(|r| *r == row)
            .expect("row on this page");
    }

    #[test]
    fn enhanced_audio_lives_in_audio_submenu() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
        at(&mut menu, Row::AudioMenu);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::Audio);
        at(&mut menu, Row::Audio);
        let act = menu.handle(MenuInput::Right, &mut av);
        assert_eq!(act, Some(MenuAction::Changed(Changed::Audio)));
        assert!(av.audio_enhanced);
    }

    #[test]
    fn mute_navigation_keeps_volume_and_reports_live_audio_change() {
        let mut av = AvConfig { audio_volume: 0.6, ..Default::default() };
        let mut menu = Menu::new(true, true);
        menu.open(Page::Audio);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(rows_of(menu.page)[menu.selected], Row::Mute);
        assert_eq!(menu.handle(MenuInput::Right, &mut av), Some(MenuAction::Changed(Changed::Audio)));
        assert_eq!(value_of(Row::Mute, &av, &keys(), "Tab", None).unwrap(), "ON");
        assert!(av.audio_mute);
        assert_eq!(av.audio_volume, 0.6);
        at(&mut menu, Row::Volume);
        menu.handle(MenuInput::Left, &mut av);
        assert!(av.audio_mute);
        assert_eq!(av.audio_volume, 0.5);
        at(&mut menu, Row::Mute);
        menu.handle(MenuInput::Left, &mut av);
        assert!(!av.audio_mute);
        assert_eq!(av.audio_volume, 0.5);
    }

    #[test]
    fn volume_steps_clamps_and_reports_audio() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
        menu.open(Page::Audio);
        at(&mut menu, Row::Volume);
        assert_eq!(menu.handle(MenuInput::Right, &mut av), None);
        assert_eq!(
            menu.handle(MenuInput::Left, &mut av),
            Some(MenuAction::Changed(Changed::Audio))
        );
        assert!((av.audio_volume - 0.9).abs() < 1e-6);
        for _ in 0..20 {
            menu.handle(MenuInput::Left, &mut av);
        }
        assert_eq!(av.audio_volume, 0.0);
        assert_eq!(menu.handle(MenuInput::Left, &mut av), None);
    }

    #[test]
    fn fast_forward_multiplier_cycles_without_enabling() {
        let mut av = AvConfig::default();
        assert_eq!(av.ff_speed, 4);
        let mut menu = Menu::new(true, true);
        menu.open(Page::FastForward);
        at(&mut menu, Row::FastForward);
        let seq: Vec<u32> = (0..5)
            .map(|_| {
                menu.handle(MenuInput::Right, &mut av);
                av.ff_speed
            })
            .collect();
        assert_eq!(seq, vec![8, 2, 4, 8, 2]);
        menu.handle(MenuInput::Left, &mut av);
        assert_eq!(av.ff_speed, 8);
        assert_eq!(
            value_of(Row::FastForward, &av, &keys(), "Tab", None).unwrap(),
            "8X"
        );
    }

    #[test]
    fn bubbles_are_main_menu_sibling_and_all_ten_rows_fit() {
        assert_eq!(MAIN_ROWS[4],Row::TextBubblesMenu);
        assert!(!VIDEO_ROWS.contains(&Row::TextBubblesMenu));
        assert_eq!(layout(MAIN_ROWS.len(),1280,720).visible_rows,MAIN_ROWS.len());
    }
    #[test]
    fn submenus_open_and_cancel_goes_back() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
        for (row, page) in [
            (Row::FastForwardMenu, Page::FastForward),
            (Row::TextBubblesMenu, Page::TextBubbles),
            (Row::AudioMenu, Page::Audio),
            (Row::VideoMenu, Page::Video),
            (Row::InputsMenu, Page::Inputs),
        ] {
            menu.reset();
            at(&mut menu, row);
            let expected = if row == Row::InputsMenu {
                Some(MenuAction::InputDevice(Device::Keyboard))
            } else {
                None
            };
            assert_eq!(menu.handle(MenuInput::Confirm, &mut av), expected);
            assert_eq!(menu.page, page);
            // Cancel leaves the submenu (cursor returns to its entry row).
            assert_eq!(menu.handle(MenuInput::Cancel, &mut av), None);
            assert_eq!(menu.page, Page::Main);
            assert_eq!(rows_of(Page::Main)[menu.selected], row);
        }
        assert_eq!(
            menu.handle(MenuInput::Cancel, &mut av),
            Some(MenuAction::Resume)
        );
    }

    #[test]
    fn state_slots_cycle_and_emit_correct_controller_actions() {
        let mut menu = Menu::new(true, true);
        let mut av = AvConfig::default();
        at(&mut menu, Row::SaveStatesMenu);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.page, Page::SaveStates);
        menu.handle(MenuInput::Left, &mut av);
        assert_eq!(menu.state_slot, 8);
        menu.handle(MenuInput::Right, &mut av);
        assert_eq!(menu.state_slot, 0);
        for _ in 0..8 { menu.handle(MenuInput::Right, &mut av); }
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::SaveState {slot:9,overwrite:false}));
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert!(menu.state_message.contains("empty"));
        menu.state_slots[8].occupied = true;
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), Some(MenuAction::LoadState(9)));
        menu.handle(MenuInput::Cancel, &mut av);
        assert_eq!(rows_of(menu.page)[menu.selected], Row::SaveStatesMenu);
    }

    #[test]
    fn undo_action_tracks_last_operation_independently_of_selected_slot() {
        let mut menu = Menu::new(true,true);
        let mut av = AvConfig::default();
        menu.open(Page::SaveStates);
        at(&mut menu,Row::UndoState);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.state_message,"Nothing to undo.");
        menu.set_state_undo(Some("Undo load (slot 9)".into()));
        menu.handle(MenuInput::Right,&mut av);
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
        let mut menu=Menu::new(true,true);let mut av=AvConfig::default();
        menu.set_save_backups([BackupInfo{generation:Some(7),status:"Ready".into()},BackupInfo::default()]);
        menu.open(Page::SaveBackups);
        assert_eq!(menu.handle(MenuInput::Confirm,&mut av),None);
        assert_eq!(menu.page,Page::RestoreBackup);
        assert_eq!(rows_of(menu.page)[menu.selected],Row::Back);
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
    fn main_and_video_with_ten_rows_have_distinct_correct_layouts() {
        let mut menu=Menu::new(true,true);
        let main=menu.page_layout(1280,720);
        assert_eq!(main.visible_rows,10);
        menu.open(Page::Video);
        let video=menu.page_layout(1280,720);
        assert!(video.panel_w>main.panel_w);
        assert!(video.rows_y>video.panel_y+video.pad);
    }

    #[test]
    fn replacing_a_slot_requires_explicit_controller_confirmation() {
        let mut menu = Menu::new(true, true);
        let mut av = AvConfig::default();
        menu.open(Page::SaveStates);
        menu.state_slots[0].occupied = true;
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::ReplaceState);
        assert_eq!(rows_of(menu.page)[menu.selected], Row::Back);
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
        let mut menu = Menu::new(true, true);
        menu.open(Page::Video);
        at(&mut menu, Row::Screen);
        menu.handle(MenuInput::Right, &mut av);
        assert!(ScreenKind::by_name(&av.screen).is_some());
        for _ in 0..ScreenKind::ALL.len() {
            menu.handle(MenuInput::Right, &mut av);
        }
        assert!(ScreenKind::by_name(&av.screen).is_some());
    }

    #[test]
    fn brightness_steps_within_range_and_rebuilds_video() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
        menu.open(Page::Video);
        at(&mut menu, Row::Brightness);
        assert_eq!(
            menu.handle(MenuInput::Right, &mut av),
            Some(MenuAction::Changed(Changed::Video))
        );
        assert!((av.video_brightness - 1.1).abs() < 1e-6);
        for _ in 0..30 {
            menu.handle(MenuInput::Right, &mut av);
        }
        assert!((av.video_brightness - 1.5).abs() < 1e-6);
    }

    #[test]
    fn key_row_starts_capture_and_freezes_menu() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
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
    fn enter_does_nothing_on_value_settings_but_arrows_change_them() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
        menu.open(Page::Audio);
        at(&mut menu, Row::Audio);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert!(!av.audio_enhanced);
        assert_eq!(
            menu.handle(MenuInput::Right, &mut av),
            Some(MenuAction::Changed(Changed::Audio))
        );
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
        let mut menu = Menu::new(true, true);
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
        let mut menu = Menu::new(true, true);
        menu.open(Page::FastForward);
        at(&mut menu, Row::FastForwardAudio);
        menu.handle(MenuInput::Right, &mut av);
        assert_eq!(av.ff_audio, "fast");
        menu.handle(MenuInput::Right, &mut av);
        assert_eq!(av.ff_audio, "normal");
        assert_eq!(value_of(Row::FastForwardAudio, &av, &keys(), "Tab", None).unwrap(), "1X DELAYED");
        menu.handle(MenuInput::Right, &mut av);
        assert_eq!(av.ff_audio, "silent");
        menu.open(Page::Inputs);
        at(&mut menu, Row::FfToggleKey);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.capturing(), Some(Bind::FastForwardToggle));
    }

    #[test]
    fn resume_and_quit_actions() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
        at(&mut menu, Row::Resume);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::Resume)
        );
        assert_eq!(
            menu.handle(MenuInput::Cancel, &mut av),
            Some(MenuAction::Resume)
        );
        at(&mut menu, Row::Quit);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::Exit);
        assert_eq!(rows_of(menu.page)[menu.selected], Row::QuitNo);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::Main);
        at(&mut menu, Row::Quit);
        menu.handle(MenuInput::Confirm, &mut av);
        menu.handle(MenuInput::Up, &mut av);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::Quit)
        );
    }

    #[test]
    fn switch_rom_submenu_can_resume_or_return_to_launcher() {
        let mut av = AvConfig::default();
        let mut menu = Menu::new(true, true);
        at(&mut menu, Row::SwitchRomMenu);
        assert_eq!(menu.handle(MenuInput::Confirm, &mut av), None);
        assert_eq!(menu.page, Page::SwitchRom);
        assert_eq!(rows_of(menu.page)[menu.selected], Row::ReturnToGame);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::Resume)
        );

        menu.reset();
        at(&mut menu, Row::SwitchRomMenu);
        menu.handle(MenuInput::Confirm, &mut av);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(
            menu.handle(MenuInput::Confirm, &mut av),
            Some(MenuAction::SwitchRom)
        );
        menu.reset();
        at(&mut menu, Row::SwitchRomMenu);
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(menu.handle(MenuInput::Cancel, &mut av), None);
        assert_eq!(menu.page, Page::Main);
        assert_eq!(rows_of(menu.page)[menu.selected], Row::SwitchRomMenu);
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
        let mut menu = Menu::new(true, true);
        menu.selected = 0;
        menu.handle(MenuInput::Up, &mut av);
        assert_eq!(menu.selected, MAIN_ROWS.len() - 1);
        menu.handle(MenuInput::Down, &mut av);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn draw_does_not_panic_on_any_page() {
        let av = AvConfig::default();
        let mut menu = Menu::new(false, false);
        for page in [
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
        let mut menu = Menu::new(true, true);
        let (w, h) = (372usize, 160usize);

        // Main page: hover selects, left click on "Audio >" opens it.
        let l = layout(MAIN_ROWS.len(), w, h);
        let x = l.panel_x + l.panel_w / 2;
        let y_main = |i: usize| l.rows_y + i * l.line_h + 1;
        let audio_row = MAIN_ROWS.iter().position(|r| *r == Row::AudioMenu).unwrap();
        menu.hover(x, y_main(audio_row), w, h);
        assert_eq!(menu.selected, audio_row);
        assert_eq!(menu.click(x, y_main(audio_row), w, h, true, &mut av), None);
        assert_eq!(menu.page, Page::Audio);

        // Both boolean controls support mouse toggles without changing volume.
        let l = layout(AUDIO_ROWS.len(), w, h);
        let y_audio = |i: usize| l.rows_y + i * l.line_h + 1;
        let mute_row = AUDIO_ROWS.iter().position(|r| *r == Row::Mute).unwrap();
        assert_eq!(menu.click(x, y_audio(mute_row), w, h, true, &mut av),
            Some(MenuAction::Changed(Changed::Audio)));
        assert!(av.audio_mute);
        assert_eq!(av.audio_volume, 1.);
        assert_eq!(menu.click(x, y_audio(mute_row), w, h, false, &mut av),
            Some(MenuAction::Changed(Changed::Audio)));
        assert!(!av.audio_mute);
        let audio_row = AUDIO_ROWS.iter().position(|r| *r == Row::Audio).unwrap();
        assert_eq!(
            menu.click(x, y_audio(audio_row), w, h, true, &mut av),
            Some(MenuAction::Changed(Changed::Audio))
        );
        assert!(av.audio_enhanced);
        assert_eq!(
            menu.click(x, y_audio(audio_row), w, h, false, &mut av),
            Some(MenuAction::Changed(Changed::Audio))
        );
        assert!(!av.audio_enhanced);

        // Outside the panel: left does nothing, right goes back.
        assert_eq!(menu.click(0, 0, w, h, true, &mut av), None);
        assert_eq!(menu.page, Page::Audio);
        assert_eq!(menu.click(0, 0, w, h, false, &mut av), None);
        assert_eq!(menu.page, Page::Main);
    }
}
