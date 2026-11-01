//! Clan ledger navigation and one shared geometry for painting and pointer routing.
use super::{Layout, Menu, Page, Row};
mod references;
pub use references::OperationReference;

pub use references::reference_group;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Domain {
    #[default]
    Game,
    Debug,
}
pub const GAME_SECTIONS: [&str; 8] = [
    "Panoramica",
    "Salvataggi",
    "Fast Forward",
    "Finestre di testo",
    "Audio",
    "Video",
    "Controlli",
    "Sistema",
];
pub const DEBUG_SECTIONS: [&str; 8] = [
    "Panoramica",
    "Sessioni e casi",
    "Replay e confronti",
    "Esecuzione e memoria",
    "Tracce e ispettori",
    "Console e script",
    "Cheat",
    "Report e archivio",
];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
impl Rect {
    pub fn hit(self, x: usize, y: usize) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub header: Rect,
    pub sidebar: Rect,
    pub domain: Rect,
    pub nav: [Rect; 8],
    pub binding: Rect,
    pub back: Rect,
    pub breadcrumb: Rect,
    pub status: Rect,
    pub title: Rect,
    pub body: Rect,
    pub shortcuts: Rect,
    pub feedback: Rect,
    pub information:Rect,
    pub list: Rect,
    pub detail: Rect,
    pub debug_tabs: Rect,
    pub scale_milli: usize,
}
pub fn geometry(w: usize, h: usize) -> Geometry {
    let scale = (w as f32 / 854.).min(h as f32 / 480.).clamp(0.45, 6.);
    let px = |v: f32| (v * scale).round().max(1.) as usize;
    let margin = px(10.).min(w / 20).min(h / 20);
    let header_h = px(34.);
    let footer_h = px(55.);
    let status_h = px(22.);
    let title_h = px(44.);
    let crumb_h = px(25.);
    let sidebar_w = ((w as f32 * 0.23) as usize)
        .clamp(px(128.), px(200.))
        .min(w.saturating_sub(margin * 3 + px(120.)));
    let side = Rect {
        x: margin,
        y: margin + header_h + margin,
        w: sidebar_w,
        h: h.saturating_sub(header_h + margin * 3),
    };
    let x = side.x + side.w + margin;
    let width = w.saturating_sub(x + margin);
    let top = side.y;
    let breadcrumb = Rect {
        x,
        y: top,
        w: width,
        h: crumb_h,
    };
    let status = Rect {
        x,
        y: top + crumb_h,
        w: width,
        h: status_h,
    };
    let title = Rect {
        x,
        y: status.y + status.h,
        w: width,
        h: title_h,
    };
    let feedback = Rect {
        x,
        y: h.saturating_sub(margin + status_h),
        w: width,
        h: status_h,
    };
    let shortcuts = Rect {
        x,
        y: feedback.y.saturating_sub(footer_h + margin),
        w: width,
        h: footer_h,
    };
    let body = Rect {
        x,
        y: title.y + title.h + margin,
        w: width,
        h: shortcuts.y.saturating_sub(title.y + title.h + margin * 2),
    };
    let brand = px(66.);
    let domain = Rect {
        x: side.x + margin,
        y: side.y + brand,
        w: side.w.saturating_sub(margin * 2),
        h: px(27.),
    };
    let back = Rect {
        x: side.x + margin,
        y: side.y + side.h - px(27.) - margin,
        w: domain.w,
        h: px(27.),
    };
    let binding = Rect {
        x: domain.x,
        y: back.y.saturating_sub(px(67.) + margin),
        w: domain.w,
        h: px(67.),
    };
    let nav_y = domain.y + domain.h + margin;
    let nav_h = ((binding.y.saturating_sub(nav_y + margin)) / 8)
        .min(px(36.))
        .max(1);
    let nav = std::array::from_fn(|i| Rect {
        x: domain.x,
        y: nav_y + i * nav_h,
        w: domain.w,
        h: nav_h.saturating_sub(px(3.)),
    });
    let debug_tabs = Rect {
        x: body.x,
        y: body.y,
        w: body.w,
        h: px(30.),
    };
    let gap = margin;
    let work = Rect {
        x: body.x,
        y: body.y + debug_tabs.h + gap,
        w: body.w,
        h: body.h.saturating_sub(debug_tabs.h + gap),
    };
    let (list, detail) = if body.w >= 470 {
        let lw = work.w * 53 / 100;
        (
            Rect { w: lw, ..work },
            Rect {
                x: work.x + lw + gap,
                w: work.w.saturating_sub(lw + gap),
                ..work
            },
        )
    } else {
        let lh = work.h * 55 / 100;
        (
            Rect { h: lh, ..work },
            Rect {
                y: work.y + lh + gap,
                h: work.h.saturating_sub(lh + gap),
                ..work
            },
        )
    };
    Geometry {
        header: Rect {
            x: margin,
            y: margin,
            w: w.saturating_sub(margin * 2),
            h: header_h,
        },
        sidebar: side,
        domain,
        nav,
        binding,
        back,
        breadcrumb,
        status,
        title,
        body,
        shortcuts,
        feedback,
        information:Rect{x:feedback.x+feedback.w.saturating_sub(px(105.)),y:feedback.y,w:px(105.).min(feedback.w),h:feedback.h},
        list,
        detail,
        debug_tabs,
        scale_milli: (scale * 1000.) as usize,
    }
}
#[derive(Clone)]
pub(super) struct Saved {
    pub page: Page,
    pub selected: usize,
    pub scroll: usize,
    pub parents: Vec<super::PageFrame>,
}
pub(super) struct State {
    pub domain: Domain,
    pub section: [usize; 2],
    pub sidebar_focus: bool,
    pub nav_cursor: usize,
    pub planned: bool,
    pub planned_cursor: usize,
    pub reference_cursor: usize,
    pub reference_scroll: std::cell::Cell<usize>,
    pub detail_scroll: usize,
    pub detail_limit:std::cell::Cell<usize>,
    pub operations: Vec<OperationReference>,
    pub saved: [[Option<Saved>; 8]; 2],
    pub frame: Option<u64>,
    pub fast_active: bool,
    pub fast_effective: u32,
    pub standard_save: String,
    pub standard_save_stamp: Option<u64>,
    pub information:String,
    pub information_hover:bool,
    pub information_sidebar_focus:bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            domain: Domain::Game,
            section: [0, 0],
            sidebar_focus: false,
            nav_cursor: 1,
            planned: false,
            planned_cursor: 0,
            reference_cursor: 0,
            reference_scroll: std::cell::Cell::new(0),
            detail_scroll: 0,
            detail_limit:std::cell::Cell::new(0),
            operations: Vec::new(),
            saved: std::array::from_fn(|_|std::array::from_fn(|_|None)),
            frame: None,
            fast_active: false,
            fast_effective: 100,
            standard_save: "Nessun salvataggio standard trovato".into(),
            standard_save_stamp: None,
            information:String::new(),
            information_hover:false,
            information_sidebar_focus:false,
        }
    }
}
impl State {
    pub fn index(&self) -> usize {
        usize::from(self.domain == Domain::Debug)
    }
    pub fn selected_section(&self) -> usize {
        self.section[self.index()]
    }
    pub fn title(&self) -> &'static str {
        if self.domain == Domain::Game {
            GAME_SECTIONS[self.selected_section()]
        } else {
            DEBUG_SECTIONS[self.selected_section()]
        }
    }
}
pub(super) const OVERVIEW_ROWS: &[Row] = &[
    Row::Resume,
    Row::StateSlot,
    Row::SaveState,
    Row::LoadState,
    Row::UndoState,
    Row::Volume,
    Row::Mute,
    Row::FastForward,
    Row::FastForwardToggle,
];
pub(super) const SAVES_ROWS: &[Row] = &[
    Row::SaveStatesMenu,
    Row::ExtendedSaves,
    Row::GameSavesMenu,
    Row::SaveBackupsMenu,
    Row::Back,
];
pub(super) const SYSTEM_ROWS: &[Row] = &[Row::Language, Row::RestartRom, Row::SwitchRomMenu, Row::Quit, Row::Back];
pub(super) fn root_page(domain: Domain, section: usize) -> Page {
    if domain == Domain::Debug {
        return Page::Debug;
    }
    match section {
        0 => Page::Main,
        1 => Page::Saves,
        2 => Page::FastForward,
        3 => Page::TextBubbles,
        4 => Page::Audio,
        5 => Page::Video,
        6 => Page::Inputs,
        _ => Page::System,
    }
}
pub(super) fn debug_rows(section: usize) -> &'static [Row] {
    match section {
        0 => &[
            Row::StartDebugSession,
            Row::CloseDebugSession,
            Row::ContentPicker,
            Row::ToolInputsMenu,
            Row::OpenCommandConsole,
            Row::DebugToolsMenu,
            Row::ScriptLibraryMenu,
            Row::CopyDiagnosis,
            Row::CliReference,
            Row::Back,
        ],
        1 => &[
            Row::ExtendedSaves,
            Row::GameSavesMenu,
            Row::StartDebugSession,
            Row::CloseDebugSession,
            Row::DebugCatalog,
            Row::ContentPicker,
            Row::ToolInputsMenu,
            Row::StartInputRecording,
            Row::StopInputRecording,
            Row::CopyDiagnosis,
            Row::CliReference,
            Row::Back,
        ],
        2 => &[
            Row::ReplayCase,
            Row::ChooseReplayProfile,
            Row::CancelReplay,
            Row::AuditReplay,
            Row::CaptureReplay,
            Row::CompareReplay,
            Row::DebugTool(super::DebugTool::Replay),
            Row::DebugTool(super::DebugTool::Audit),
            Row::DebugTool(super::DebugTool::Capture),
            Row::DebugTool(super::DebugTool::Compare),
            Row::CliReference,
            Row::Back,
        ],
        3 => &[
            Row::DebugStepOne,
            Row::DebugStepTen,
            Row::DebugResume,
            Row::InspectPC,
            Row::DebugTool(super::DebugTool::Step),
            Row::CliReference,
            Row::Back,
        ],
        4 => &[
            Row::CaptureGraphics,
            Row::StartUiTrace,
            Row::SaveUiReport,
            Row::ToggleEventConsole,
            Row::StartAudioCapture,
            Row::SaveAudioReport,
            Row::StartCodeTrace,
            Row::SaveCodeReport,
            Row::StartGameplayTrace,
            Row::SaveGameplayReport,
            Row::StartRecentEvents,
            Row::SaveRecentEvents,
            Row::CliReference,
            Row::Back,
        ],
        5 => &[
            Row::ScriptLibraryMenu,
            Row::DebugToolsMenu,
            Row::ToolInputsMenu,
            Row::CliReference,
            Row::Back,
        ],
        6 => &[Row::CheatCatalog, Row::CliReference, Row::Back],
        _ => &[
            Row::CopyDiagnosis,
            Row::CreateIssueReport,
            Row::SaveUiReport,
            Row::SaveAudioReport,
            Row::SaveCodeReport,
            Row::SaveGameplayReport,
            Row::SaveRecentEvents,
            Row::DebugTool(super::DebugTool::Homepage),
            Row::DebugTool(super::DebugTool::CampaignReport),
            Row::DebugTool(super::DebugTool::ArtifactPreview),
            Row::CliReference,
            Row::Back,
        ],
    }
}
pub(super) fn specialty(page: Page) -> bool {
    matches!(
        page,
        Page::DebugCatalog
            | Page::CheatCatalog
            | Page::ContentSelector
            | Page::DebugTool
            | Page::GameSaves
            | Page::SwitchGameSave
            | Page::ReplaceGameSave
            | Page::SaveStates
            | Page::ReplaceState
            | Page::SaveBackups
            | Page::RestoreBackup
    )
}
pub(super) fn list_layout(menu: &Menu, w: usize, h: usize) -> Layout {
    let g = geometry(w, h);
    let r = if matches!(menu.page, Page::Debug | Page::CliReference) {
        g.list
    } else {
        g.body
    };
    let pad = (g.scale_milli * 12 / 1000).max(3);
    let line_h = (g.scale_milli * 38 / 1000).max(24);
    let header = if matches!(
        menu.page,
        Page::Inputs | Page::Gamepad | Page::MouseKeyboard
    ) {
        line_h
    } else {
        pad
    };
    let footer = if matches!(
        menu.page,
        Page::Inputs | Page::Gamepad | Page::MouseKeyboard
    ) {
        line_h + pad
    } else {
        pad
    };
    Layout {
        panel_x: r.x,
        panel_y: r.y,
        panel_w: r.w,
        panel_h: r.h,
        pad,
        line_h,
        rows_y: r.y + pad + header,
        visible_rows: ((r.h.saturating_sub(header + footer + pad * 2)) / line_h).max(1)
            * columns(menu, w, h),
    }
}

/// Wide settings use two columns inside the fixed content panel.
pub(super) fn columns(menu: &Menu, w: usize, h: usize) -> usize {
    let g = geometry(w, h);
    if g.body.w >= 750
        && matches!(
            menu.page,
            Page::Video
                | Page::Inputs
                | Page::Gamepad
                | Page::MouseKeyboard
                | Page::ToolInputs
                | Page::DebugTools
                | Page::ScriptLibrary
        )
    {
        2
    } else {
        1
    }
}
pub(super) fn row_rect(menu: &Menu, index: usize, w: usize, h: usize) -> Option<Rect> {
    if menu.page==Page::DebugCatalog{let b=geometry(w,h).body;let mut r=super::debug_catalog::row_rect(index,b.w,b.h);r.x+=b.x;r.y+=b.y;return Some(r);}
    if menu.page == Page::Main {
        return crate::atlas_theme::overview_rects(w, h).get(index).copied();
    }
    let l = menu.page_layout(w, h);
    let input = matches!(
        menu.page,
        Page::Inputs | Page::Gamepad | Page::MouseKeyboard
    );
    if input && index + 1 == menu.rows().len() {
        return Some(Rect {
            x: l.panel_x + l.panel_w - l.pad - l.line_h * 3,
            y: l.panel_y + l.panel_h - l.pad - l.line_h + 6,
            w: l.line_h * 3,
            h: l.line_h - 6,
        });
    }
    let first = menu.first_visible(l);
    if index < first || index >= first + l.visible_rows {
        return None;
    }
    let cols = columns(menu, w, h);
    let capacity = l.visible_rows / cols;
    let visible = index - first;
    let gap = l.pad;
    let width = (l.panel_w - l.pad * 2 - gap * (cols - 1)) / cols;
    Some(Rect {
        x: l.panel_x + l.pad + (visible / capacity) * (width + gap),
        y: l.rows_y + (visible % capacity) * l.line_h,
        w: width,
        h: l.line_h.saturating_sub(4),
    })
}

/// Explicit UTC file metadata; unavailable timestamps never acquire invented dates.
pub fn timestamp_utc(seconds: u64) -> Option<String> {
    let mut days = seconds / 86400;
    if days > 2_932_896 {
        return None;
    }
    let mut year = 1970 + (days / 146097) * 400;
    days %= 146097;
    let leap = |y: u64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    loop {
        let n = if leap(year) { 366 } else { 365 };
        if days < n {
            break;
        }
        days -= n;
        year += 1;
    }
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for n in lengths {
        if days < n {
            break;
        }
        days -= n;
        month += 1;
    }
    Some(format!(
        "{year:04}-{month:02}-{:02} {:02}:{:02} UTC",
        days + 1,
        (seconds / 3600) % 24,
        (seconds / 60) % 60
    ))
}

#[cfg(test)]
mod tests;
