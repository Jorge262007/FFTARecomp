//! Directional focus navigation and confirmation-owned value choices.
use super::*;

impl Menu {
    pub(super) fn row_choices(&self, row: Row) -> Vec<String> {
        match row {
            Row::StateSlot => (1..=self.state_slots.len())
                .map(|i| i.to_string())
                .collect(),
            Row::GameSaveSlot => (1..=self.game_save_slots.len())
                .map(|i| i.to_string())
                .collect(),
            Row::MouseMode => vec!["OFF".into(), "ON".into()],
            Row::CatalogRegion
            | Row::CatalogStatus
            | Row::CatalogKind
            | Row::CatalogCase
            | Row::CatalogEntry => self.catalog.choices(row),
            _ => setting_options(row),
        }
    }

    pub(super) fn row_choice_index(&self, row: Row, av: &AvConfig, options: &[String]) -> usize {
        match row {
            Row::StateSlot => self.state_slot,
            Row::GameSaveSlot => self.game_save_slot,
            Row::MouseMode => usize::from(self.mouse_mode.get()),
            Row::FastForwardAudio => ["silent", "fast", "normal"]
                .iter()
                .position(|s| *s == av.ff_audio)
                .unwrap_or(0),
            Row::CatalogRegion
            | Row::CatalogStatus
            | Row::CatalogKind
            | Row::CatalogCase
            | Row::CatalogEntry => self.catalog.choice_index(row),
            _ => options
                .iter()
                .position(|s| *s == setting_value(row, av))
                .unwrap_or(0),
        }
        .min(options.len().saturating_sub(1))
    }

    pub(super) fn commit_choice(&mut self, dd: Dropdown, av: &mut AvConfig) -> Option<MenuAction> {
        let result = self.commit_choice_inner(dd, av);
        self.language.set(av.menu_language);
        result
    }

    fn commit_choice_inner(&mut self, dd: Dropdown, av: &mut AvConfig) -> Option<MenuAction> {
        match dd.row {
            Row::StateSlot => {
                self.state_slot = dd.index;
                self.state_message.clear();
                None
            }
            Row::GameSaveSlot => {
                self.game_save_slot = dd.index;
                self.game_save_message.clear();
                None
            }
            Row::MouseMode => {
                if self.mouse_mode.get() != (dd.index == 1) {
                    self.toggle_mouse_mode()
                } else {
                    None
                }
            }
            Row::CatalogRegion
            | Row::CatalogStatus
            | Row::CatalogKind
            | Row::CatalogCase
            | Row::CatalogEntry => {
                let mut action = None;
                for _ in 0..dd.options.len() {
                    if self.catalog.choice_index(dd.row) == dd.index {
                        break;
                    }
                    action = self
                        .catalog
                        .handle(dd.row, MenuInput::Right, self.debug_session)
                        .flatten();
                }
                action
            }
            _ => choose_setting(dd.row, &dd.options[dd.index], av).map(MenuAction::Changed),
        }
    }

    pub(super) fn navigate_horizontal(&mut self, right: bool) {
        let (w, h) = self.viewport.get();
        let selected = self.selected;
        if let Some(current) = clan_atlas::row_rect(self, selected, w, h) {
            let center_x = current.x + current.w / 2;
            let center_y = current.y + current.h / 2;
            let candidate = (0..self.rows().len())
                .filter(|i| *i != selected)
                .filter_map(|i| {
                    let r = clan_atlas::row_rect(self, i, w, h)?;
                    let x = r.x + r.w / 2;
                    let y = r.y + r.h / 2;
                    if y.abs_diff(center_y) <= (r.h + current.h) / 2
                        && ((right && x > center_x + current.w / 3)
                            || (!right && x + current.w / 3 < center_x))
                    {
                        Some((y.abs_diff(center_y) * 8 + x.abs_diff(center_x), i))
                    } else {
                        None
                    }
                })
                .min();
            if let Some((_, index)) = candidate {
                self.selected = index;
                return;
            }
        }
        if !right {
            self.atlas.sidebar_focus = true;
            self.atlas.nav_cursor = self.atlas.selected_section() + 1;
        }
    }

    pub(super) fn input_tabs_input(&mut self, input: MenuInput) -> Option<MenuAction> {
        let index = self.input_tabs_focus?;
        match input {
            MenuInput::Left => self.input_tabs_focus = Some((index + 2) % 3),
            MenuInput::Right => self.input_tabs_focus = Some((index + 1) % 3),
            MenuInput::Up => {
                self.input_tabs_focus = None;
                self.atlas.sidebar_focus = true;
            }
            MenuInput::Down | MenuInput::Cancel => self.input_tabs_focus = None,
            MenuInput::Confirm => {
                self.input_tabs_focus = None;
                return self.input_tab(index);
            }
        }
        None
    }

    pub(super) fn input_tab_rect(&self, index: u8, w: usize, h: usize) -> clan_atlas::Rect {
        let l = self.page_layout(w, h);
        let scale = l.line_h as f32 / 44.;
        let width = (l.panel_w as f32 - 2. * l.pad as f32 - 12. * scale) / 3.;
        clan_atlas::Rect {
            x: (l.panel_x + l.pad) as usize + (index as f32 * (width + 6. * scale)) as usize,
            y: l.panel_y + l.pad,
            w: width as usize,
            h: (30. * scale) as usize,
        }
    }
}

#[cfg(test)]
mod tests;
