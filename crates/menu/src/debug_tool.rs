//! Reusable parameter form. Validation and IO remain in the host tool handlers.
use super::{
    canvas::Canvas,
    content_selector::{button, geometry, Selector},
    diagnostics, ContentSlot, MenuAction, MenuInput,
};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebugTool {
    Replay,
    Audit,
    Capture,
    Compare,
    Step,
    Inspect,
    Audio,
    Code,
    Gameplay,
    Recent,
    Graphics,
    Issue,
    Homepage,
    CampaignReport,
    ArtifactPreview,
    Recipe,
    GuestScripts,
}
impl DebugTool {
    pub const ALL: [Self; 17] = [
        Self::Replay,
        Self::Audit,
        Self::Capture,
        Self::Compare,
        Self::Step,
        Self::Inspect,
        Self::Audio,
        Self::Code,
        Self::Gameplay,
        Self::Recent,
        Self::Graphics,
        Self::Issue,
        Self::Homepage,
        Self::CampaignReport,
        Self::ArtifactPreview,
        Self::Recipe,
        Self::GuestScripts,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Replay => "Prepare replay",
            Self::Audit => "Prepare ROM audit",
            Self::Capture => "Prepare trigger capture",
            Self::Compare => "Prepare comparison",
            Self::Step => "Configured frame step",
            Self::Inspect => "Inspect memory / registers",
            Self::Audio => "Audio capture options",
            Self::Code => "Code trace options",
            Self::Gameplay => "Task / script trace options",
            Self::Recent => "Recent events options",
            Self::Graphics => "Graphics capture options",
            Self::Issue => "Issue report options",
            Self::Homepage => "Host: report homepage",
            Self::CampaignReport => "Host: campaign report",
            Self::ArtifactPreview => "Host: artifact preview",
            Self::Recipe => "Debug command recipe",
            Self::GuestScripts => "Guest scripts / readonly catalog",
        }
    }
    pub fn library(self) -> bool {
        matches!(self, Self::Homepage | Self::CampaignReport | Self::ArtifactPreview | Self::Recipe | Self::GuestScripts)
    }
    pub fn creates_profile(self) -> bool {
        matches!(
            self,
            Self::Replay | Self::Audit | Self::Capture | Self::Compare
        )
    }
}
#[derive(Clone, Debug)]
pub enum ToolFieldKind {
    Text,
    Choice(Vec<String>),
    Input(ContentSlot),
}
#[derive(Clone, Debug)]
pub struct ToolField {
    pub key: String,
    pub label: String,
    pub value: String,
    pub help: String,
    pub kind: ToolFieldKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolRequest {
    Validate,
    SavePreset,
    LoadPreset,
    Prepare,
    Run,
    Back,
}
const REQUESTS: [ToolRequest; 6] = [
    ToolRequest::Validate,
    ToolRequest::SavePreset,
    ToolRequest::LoadPreset,
    ToolRequest::Prepare,
    ToolRequest::Run,
    ToolRequest::Back,
];
fn request_label(r: ToolRequest, tool: DebugTool) -> &'static str {
    if tool.library() {
        return match r {
            ToolRequest::Validate => "Details / verify",
            ToolRequest::Run => if tool == DebugTool::GuestScripts { "Inspect catalog" } else { "Start explicitly" },
            ToolRequest::Prepare => if tool == DebugTool::GuestScripts { "Gameplay inspector" } else { "Cancel process" },
            ToolRequest::SavePreset => "Save new preset",
            ToolRequest::LoadPreset => "Load last preset",
            ToolRequest::Back => "Back",
        };
    }
    match r {
        ToolRequest::Validate => "Verify draft",
        ToolRequest::SavePreset => "Save new preset",
        ToolRequest::LoadPreset => "Load last preset",
        ToolRequest::Prepare => {
            if tool.creates_profile() {
                "Prepare NEW profile"
            } else {
                "Apply options"
            }
        }
        ToolRequest::Run => {
            if tool.creates_profile() {
                "Run prepared profile"
            } else {
                "Start / apply"
            }
        }
        ToolRequest::Back => "Back / cancel",
    }
}
pub(super) struct ToolForm {
    pub tool: DebugTool,
    pub fields: Vec<ToolField>,
    pub message: String,
    pub summary: String,
    pub busy: bool,
    pub debug_copy: bool,
    focus: usize,
    first: Cell<usize>,
    detail: Cell<usize>,
    editor: Selector,
    editing: Option<usize>,
    choice_edit: Option<usize>,
    revision: u64,
}
impl Default for ToolForm {
    fn default() -> Self {
        Self {
            tool: DebugTool::Replay,
            fields: Vec::new(),
            message: String::new(),
            summary: String::new(),
            busy: false,
            debug_copy: false,
            focus: 0,
            first: Cell::new(0),
            detail: Cell::new(0),
            editor: Selector::default(),
            editing: None,
            choice_edit: None,
            revision: 0,
        }
    }
}
impl ToolForm {
    pub(super) fn selection_state(&self)->Vec<(String,String)>{
        let mut state=vec![("page".into(),"DebugTool".into()),("tool".into(),format!("{:?}",self.tool)),("busy".into(),self.busy.to_string()),("detail_scroll".into(),self.detail.get().to_string()),("feedback".into(),self.message.clone())];
        state.push(("choice_editing".into(),self.choice_focus().to_string()));
        if self.choice_focus(){state.push(("pending_value".into(),self.shown_value(self.focus).into()));}
        if let Some(index)=self.editing {
            state.extend(self.editor.selection_state().into_iter().map(|(k,v)|(format!("editor.{k}"),v)));
            state.push(("item_id".into(),format!("tool/{:?}/editor/{}",self.tool,self.fields[index].key)));
            state.push(("label".into(),self.fields[index].label.clone()));
        }else{
            let (id,label)=if let Some(field)=self.fields.get(self.focus){(format!("field/{}",field.key),field.label.clone())}else{let request=REQUESTS[self.focus-self.fields.len()];(format!("action/{request:?}"),request_label(request,self.tool).into())};
            state.push(("item_id".into(),format!("tool/{:?}/{id}",self.tool)));state.push(("label".into(),label));
        }
        state.extend(self.fields.iter().map(|f|(format!("field.{}",f.key),f.value.clone())));
        state
    }
    pub fn set(&mut self, tool: DebugTool, fields: Vec<ToolField>, message: String) {
        self.choice_edit=None;
        if self.tool != tool {
            self.focus = 0;
            self.first.set(0);
            self.detail.set(0);
        }
        self.tool = tool;
        self.fields = fields.into_iter().take(32).collect();
        self.message = message;
        self.focus = self.focus.min(self.fields.len() + 5);
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn values(&self) -> Vec<(String, String)> {
        self.fields
            .iter()
            .filter(|f| !matches!(f.kind, ToolFieldKind::Input(_)))
            .map(|f| (f.key.clone(), f.value.clone()))
            .collect()
    }
    pub fn text_focus(&self) -> bool {
        self.editing.is_some()
    }
    pub fn choice_focus(&self)->bool {self.choice_edit.is_some()}
    fn shown_value(&self,index:usize)->&str {
        if index==self.focus {
            if let (Some(choice),ToolFieldKind::Choice(options))=(self.choice_edit,&self.fields[index].kind) {
                if let Some(value)=options.get(choice) {return value;}
            }
        }
        &self.fields[index].value
    }
    pub fn text(&mut self, t: &str) {
        self.editor.text(t)
    }
    pub fn preedit(&mut self, t: &str) {
        self.editor.preedit(t)
    }
    pub fn backspace(&mut self) {
        self.editor.text_backspace()
    }
    pub fn finish(&mut self, cancel: bool) {
        if let Some(index)=self.choice_edit.take() {
            if !cancel {
                if let Some(field)=self.fields.get_mut(self.focus) {
                    if let ToolFieldKind::Choice(options)=&field.kind {
                        if let Some(value)=options.get(index) {
                            if field.value!=*value {field.value=value.clone();self.changed();}
                        }
                    }
                }
            }
            return;
        }
        self.editor.finish_search(cancel);
        self.sync_editor()
    }
    fn sync_editor(&mut self) {
        if !self.editor.editing() {
            if let Some(i) = self.editing.take() {
                let v = self.editor.edited_value();
                if v != self.fields[i].value {
                    self.fields[i].value = v;
                    self.changed();
                }
            }
        }
    }
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.message = "Draft changed; verify before preparing or starting.".into();
    }
    pub fn edit(&mut self) {
        if let Some(f) = self.fields.get(self.focus) {
            if matches!(f.kind, ToolFieldKind::Text) {
                self.editing = Some(self.focus);
                self.editor.debug_copy = self.debug_copy;
                self.editor
                    .start_editor(f.label.clone(), f.value.clone(), 2048);
            }
        }
    }
    pub fn tab(&mut self, back: bool) {
        self.finish(self.choice_focus());
        let n = self.fields.len() + 6;
        self.focus = (self.focus + if back { n - 1 } else { 1 }) % n;
    }
    fn first(&self, visible: usize) -> usize {
        let mut f = self
            .first
            .get()
            .min(self.fields.len().saturating_sub(visible));
        if self.focus < self.fields.len() {
            if self.focus < f {
                f = self.focus;
            } else if self.focus >= f + visible {
                f = self.focus + 1 - visible;
            }
        }
        self.first.set(f);
        f
    }
    pub fn handle(&mut self, input: MenuInput) -> Option<MenuAction> {
        if let Some(index)=self.choice_edit {
            match input {
                MenuInput::Confirm=>self.finish(false),
                MenuInput::Cancel=>self.finish(true),
                _=>if let Some(ToolField{kind:ToolFieldKind::Choice(options),..})=self.fields.get(self.focus) {
                    if !options.is_empty() {self.choice_edit=Some((index+if matches!(input,MenuInput::Left|MenuInput::Up){options.len()-1}else{1})%options.len());}
                },
            }
            return None;
        }
        if self.text_focus() {
            self.editor.handle(input);
            self.sync_editor();
            return None;
        }
        match input {
            MenuInput::Cancel => return Some(MenuAction::ToolRequest(ToolRequest::Back)),
            MenuInput::Up => {
                self.focus = (self.focus + self.fields.len() + 5) % (self.fields.len() + 6)
            }
            MenuInput::Down => self.focus = (self.focus + 1) % (self.fields.len() + 6),
            MenuInput::Left | MenuInput::Right => {
                let count=self.fields.len()+6;
                self.focus=(self.focus+if input==MenuInput::Left{count-1}else{1})%count;
            }
            MenuInput::Confirm => {
                if self.focus >= self.fields.len() {
                    let r = REQUESTS[self.focus - self.fields.len()];
                    if !self.busy || r == ToolRequest::Back || (self.tool.library() && r == ToolRequest::Prepare) {
                        return Some(MenuAction::ToolRequest(r));
                    }
                    self.message = "A host operation is finishing; wait or return.".into();
                } else {
                    match &self.fields[self.focus].kind {
                        ToolFieldKind::Input(slot) => {
                            return Some(MenuAction::ChooseToolInput(*slot))
                        }
                        ToolFieldKind::Text => self.edit(),
                        ToolFieldKind::Choice(options) => {
                            if !options.is_empty() {self.choice_edit=Some(options.iter().position(|v|*v==self.fields[self.focus].value).unwrap_or(0));}
                        },
                    }
                }
            }
        }
        None
    }
    pub fn wheel(&mut self, up: bool) {
        if self.text_focus() {
            return;
        }
        if self.focus >= self.fields.len() {
            self.detail.set(if up {
                self.detail.get().saturating_sub(2)
            } else {
                (self.detail.get() + 2).min(2048)
            });
        } else {
            self.handle(if up { MenuInput::Up } else { MenuInput::Down });
        }
    }
    pub fn hover(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if self.choice_focus(){return;}
        if self.text_focus() {
            self.editor.hover(x, y, w, h);
            return;
        }
        let g = geometry(w, h);
        let inside = |r: [usize; 4]| x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3];
        if inside(g.list) {
            let i = self.first(g.visible) + (y - g.list[1]) / g.row;
            if i < self.fields.len() {
                self.focus = i;
            }
        }
        for i in 0..6 {
            if inside(g.actions[i]) {
                self.focus = self.fields.len() + i;
            }
        }
    }
    pub fn click(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        left: bool,
    ) -> Option<MenuAction> {
        if self.choice_focus(){return self.handle(if left {MenuInput::Confirm}else{MenuInput::Cancel});}
        if self.text_focus() {
            self.editor.click(x, y, w, h, left);
            self.sync_editor();
            return None;
        }
        if !left {
            return self.handle(MenuInput::Cancel);
        }
        let g = geometry(w, h);
        let inside = |r: [usize; 4]| x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3];
        if inside(g.details) {
            self.detail.set((self.detail.get() + 2).min(2048));
            return None;
        }
        if inside(g.list) || g.actions[..6].iter().any(|r| inside(*r)) {
            self.hover(x, y, w, h);
            return self.handle(MenuInput::Confirm);
        }
        None
    }
    pub fn focus(&self, w: usize, h: usize) -> diagnostics::Focus {
        if self.text_focus() {
            return self.editor.focus(w, h);
        }
        let g = geometry(w, h);
        let first = self.first(g.visible);
        let (label, bounds) = if let Some(f) = self.fields.get(self.focus) {
            (
                format!("{}: {}", f.label, self.shown_value(self.focus)),
                [
                    g.list[0],
                    g.list[1] + (self.focus - first) * g.row,
                    g.list[2],
                    g.row,
                ],
            )
        } else {
            (
                request_label(REQUESTS[self.focus - self.fields.len()], self.tool).into(),
                g.actions[self.focus - self.fields.len()],
            )
        };
        diagnostics::Focus {
            page: "DebugTool".into(),
            selected: self.focus,
            label,
            capture: self.choice_focus().then(||"Scelta: OK applica / Annulla scarta".into()),
            panel: g.panel,
            selected_bounds: Some(bounds),
            first_visible: first,
            dropdown: None,
        }
    }
}
pub(super) fn draw(s: &ToolForm, px: &mut [[u8; 4]], w: usize, h: usize, language: input_config::Language) {
    let locale = super::i18n::Locale(language);
    if s.text_focus() {
        super::content_selector::draw(&s.editor, px, w, h, language);
        return;
    }
    for p in px.iter_mut().take(w * h) {
        for v in &mut p[..3] {
            *v /= 4;
        }
    }
    let g = geometry(w, h);
    let [x, y, pw, ph] = g.panel;
    let size = (13. * g.scale).max(4.);
    let mut c = Canvas::new(px, w, h);
    let ink = [233, 239, 236];
    c.rect(
        x as f32,
        y as f32,
        pw as f32,
        ph as f32,
        7.,
        [28, 56, 78],
        [28, 56, 78],
        [89, 121, 139],
    );
    c.text(
        (x + 10) as f32,
        (y + 4) as f32,
        &format!(
            "{}DRAFT / {}",
            if s.debug_copy { locale.tr("DEBUG COPY / ") } else { "" },
            locale.tr(s.tool.label())
        ),
        ink,
        size * 1.2,
        true,
        pw.saturating_sub(20) as f32,
    );
    let mut instruction=g.search;instruction[2]=g.panel[2].saturating_sub((instruction[0]-x)*2);
    button(&mut c,instruction,if s.choice_focus(){locale.tr("Scelta: frecce regolano · OK applica · Annulla scarta")}else{locale.tr("Frecce: naviga · OK modifica · usa i pulsanti per eseguire")},false,size);
    let first = s.first(g.visible);
    for (n, f) in s.fields.iter().skip(first).take(g.visible).enumerate() {
        let r = [
            g.list[0],
            g.list[1] + n * g.row,
            g.list[2],
            g.row.saturating_sub(2),
        ];
        button(
            &mut c,
            r,
            &format!("{}: {}", locale.tr(&f.label), s.shown_value(first+n)),
            s.focus == first + n,
            size,
        );
    }
    let help = s
        .fields
        .get(s.focus)
        .map(|f| format!("{}\n{}\n\n", locale.tr(&f.label), locale.tr(&f.help)))
        .unwrap_or_default();
    let text = format!(
        "{help}{}\n\n{}\n\n{}",
        s.summary,
        locale.tr(&s.message),
        if s.busy {
            locale.tr("Host operation pending; no guest execution implied.")
        } else {
            locale.tr("Draft and selected input references are separate from execution. Existing authenticated profiles are never edited.")
        }
    );
    let lines = Canvas::wrap_text(&text, size, g.details[2].saturating_sub(12) as f32);
    let line = (18. * g.scale).max(6.) as usize;
    let visible = g.details[3] / line;
    let scroll = s.detail.get().min(lines.len().saturating_sub(visible));
    s.detail.set(scroll);
    for (n, t) in lines.iter().skip(scroll).take(visible).enumerate() {
        c.text(
            (g.details[0] + 6) as f32,
            (g.details[1] + n * line) as f32,
            t,
            ink,
            size,
            false,
            g.details[2].saturating_sub(12) as f32,
        );
    }
    for i in 0..6 {
        button(
            &mut c,
            g.actions[i],
            locale.tr(request_label(REQUESTS[i], s.tool)),
            s.focus == s.fields.len() + i,
            size,
        );
    }
    let footer = Canvas::wrap_text(locale.tr(&s.message), size, g.panel[2].saturating_sub(20) as f32);
    for (n, t) in footer.iter().take(2).enumerate() {
        c.text(
            (x + 10) as f32,
            (g.actions[5][1] + g.row + (10. * g.scale) as usize + n * line) as f32,
            t,
            ink,
            size,
            false,
            g.panel[2].saturating_sub(20) as f32,
        );
    }
    c.finish(px);
}

#[cfg(test)]
mod tests;
