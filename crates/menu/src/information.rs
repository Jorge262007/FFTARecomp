//! Read-only full text for values, bindings and feedback clipped in compact rows.
use super::*;
impl Menu {
    pub fn information_available(&self) -> bool {
        self.capture.is_none()
            && !self.content_text_focus()
            && self.dropdown.is_none()
            && self.input_tabs_focus.is_none()
            && !self.atlas_reference_active()
            && !(self.tool_active() && self.tool.busy)
            && !(self.content_active() && self.content.busy)
            && !(self.cheat_active() && self.cheats.busy)
    }
    /// Capture the selected context without executing it or leaving an editor/capture.
    pub fn information(&mut self, av: &AvConfig, cfg: &InputConfig) {
        self.language.set(av.menu_language);
        self.cancel_debug_unlock();
        if !self.information_available() {
            return;
        }
        if self.page == Page::Information {
            self.close_information();
            return;
        }
        if self.atlas_reference_active() {
            return;
        }
        let properties = self.selection_state(av, cfg);
        let label = properties
            .iter()
            .find(|(k, _)| k == "label")
            .map(|(_, v)| v.as_str())
            .unwrap_or(self.atlas.title());
        let mut parts = vec![
            format!("{} / {}", self.tr(self.atlas.title()), label),
            self.tr("Dettagli della selezione all’apertura").into(),
        ];
        if let Some((_, value)) = properties.iter().find(|(k, _)| k == "value") {
            parts.push(self.tr_format("Valore: {value}", &[("value", (value).to_string())]));
        }
        if self.tool_active() {
            parts.push(self.tool.message.clone());
            parts.push(self.tool.summary.clone());
            for field in &self.tool.fields {
                parts.push(format!("{}: {}\n{}", field.label, field.value, field.help));
            }
        }
        if self.content_active() {
            parts.push(self.content.description());
            parts.push(self.content.message.clone());
            parts.push(self.content.preview_message.clone());
            if let Some(entry) = self
                .content
                .highlighted_id()
                .and_then(|id| self.content.entries.iter().find(|e| e.id == id))
            {
                parts.push(self.tr_format(
                    "{}\n{}\nPosizione: {}\nIdentità: {}\nStato: {}\nCompatibilità: {}\n{}",
                    &[
                        ("0", (entry.name).to_string()),
                        ("1", (entry.description).to_string()),
                        ("2", (entry.location).to_string()),
                        ("3", (entry.identity).to_string()),
                        ("4", (entry.status).to_string()),
                        ("5", (entry.compatibility).to_string()),
                        ("6", (entry.reason).to_string()),
                    ],
                ));
            }
        }
        if self.cheat_active() {
            parts.push(self.cheats.description());
            parts.push(self.cheats.message.clone());
            if let Some(entry) = self
                .cheats
                .highlighted_id()
                .and_then(|id| self.cheats.entries.iter().find(|e| e.id == id))
            {
                parts.push(format!(
                    "{}\n{}\n{}\n{}",
                    entry.name, entry.description, entry.identity, entry.reason
                ));
            }
        }
        if self.page == Page::DebugCatalog {
            parts.push(self.catalog.details());
            parts.push(self.catalog.message.clone());
        }
        parts.push(self.tr_format("Menu tastiera: {}", &[("0", (cfg.menu_key).to_string())]));
        parts.push(self.tr_format("Menu gamepad: {}", &[("0", (cfg.menu_pad).to_string())]));
        if matches!(
            self.page,
            Page::Main | Page::SaveStates | Page::ReplaceState
        ) {
            if let Some(slot) = self.state_slots.get(self.state_slot) {
                parts.push(self.tr_format(
                    "Stato {} / {}: {}",
                    &[
                        ("0", (self.state_slot + 1).to_string()),
                        ("1", (self.state_slots.len()).to_string()),
                        ("2", self.tr(&slot.status).to_string()),
                    ],
                ));
                parts.push(
                    self.tr_format(
                        "Data stato: {}",
                        &[(
                            "0",
                            (self
                                .state_times
                                .get(self.state_slot)
                                .copied()
                                .flatten()
                                .and_then(clan_atlas::timestamp_utc)
                                .unwrap_or_else(|| self.tr("non disponibile").into()))
                            .to_string(),
                        )],
                    ),
                );
            }
            parts.push(self.standard_save_text());
        }
        for (label, message) in [
            (self.tr("Stato"), &self.state_message),
            (self.tr("Salvataggi cartuccia"), &self.game_save_message),
            ("Backup", &self.backup_message),
            ("Debug", &self.debug_message),
            (self.tr("Sessione"), &self.session_message),
        ] {
            if !message.is_empty() {
                parts.push(format!("{label}: {message}"));
            }
        }
        if let Some(slot) = self.game_save_slots.get(self.game_save_slot) {
            if matches!(
                self.page,
                Page::GameSaves | Page::SwitchGameSave | Page::ReplaceGameSave
            ) {
                parts.push(self.tr_format(
                    "File: {}\nStato: {}",
                    &[
                        ("0", (slot.name).to_string()),
                        ("1", self.tr(&slot.status).to_string()),
                    ],
                ));
            }
        }
        self.atlas.information = parts.join("\n\n");
        self.atlas.information_sidebar_focus = self.atlas.sidebar_focus;
        self.atlas.sidebar_focus = false;
        self.open(Page::Information);
        self.atlas.detail_scroll = 0;
    }
    fn close_information(&mut self) {
        let focus = self.atlas.information_sidebar_focus;
        self.back_to_main();
        self.atlas.sidebar_focus = focus;
    }
    pub fn information_hit(&self, x: usize, y: usize, w: usize, h: usize) -> bool {
        clan_atlas::geometry(w, h).information.hit(x, y)
    }
    pub(super) fn information_input(&mut self, input: MenuInput) -> Option<MenuAction> {
        match input {
            MenuInput::Up | MenuInput::Left => {
                self.atlas.detail_scroll = self.atlas.detail_scroll.saturating_sub(3)
            }
            MenuInput::Down | MenuInput::Right => {
                self.atlas.detail_scroll = self
                    .atlas
                    .detail_scroll
                    .saturating_add(3)
                    .min(self.atlas.detail_limit.get())
            }
            MenuInput::Confirm | MenuInput::Cancel => self.close_information(),
        }
        None
    }
}
pub(super) fn draw(menu: &Menu, px: &mut [[u8; 4]], w: usize, h: usize) {
    let g = clan_atlas::geometry(w, h);
    let r = g.body;
    let mut c = canvas::Canvas::new(px, w, h);
    atlas_theme::panel(&mut c, r, false);
    let fs = (g.scale_milli as f32 * 14. / 1000.).max(11.);
    let pitch = fs as usize + 6;
    let lines: Vec<_> = menu
        .atlas
        .information
        .split('\n')
        .flat_map(|paragraph| {
            let chars: Vec<_> = paragraph.chars().collect();
            if chars.is_empty() {
                return vec![String::new()];
            }
            chars
                .chunks(16384)
                .flat_map(|chunk| {
                    canvas::Canvas::wrap_text(
                        &chunk.iter().collect::<String>(),
                        fs,
                        r.w.saturating_sub(28) as f32,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let capacity = r.h.saturating_sub(46) / pitch;
    let limit = lines.len().saturating_sub(capacity);
    menu.atlas.detail_limit.set(limit);
    let first = menu.atlas.detail_scroll.min(limit);
    for (i, line) in lines.iter().skip(first).take(capacity).enumerate() {
        c.text(
            (r.x + 12) as f32,
            (r.y + 10 + i * pitch) as f32,
            line,
            [233, 239, 236],
            fs,
            false,
            r.w.saturating_sub(24) as f32,
        );
    }
    c.text(
        (r.x + 12) as f32,
        (r.y + r.h.saturating_sub(26)) as f32,
        menu.tr("↑↓ / rotella: scorri · F1 / R3 / Esc: chiudi"),
        [168, 189, 199],
        (fs * 0.8).max(9.),
        false,
        r.w.saturating_sub(24) as f32,
    );
    c.finish(px);
}
