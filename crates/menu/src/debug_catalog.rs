//! Shared catalog metadata, filtered by the controller without filesystem access.
use super::{Layout, Menu, MenuAction, MenuInput, Row, canvas::Canvas};
#[derive(Clone, Default)]
pub struct CatalogEntry {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub case: String,
    pub region: String,
    pub status: String,
    pub slot: Option<u8>,
    pub source: String,
    pub evidence: String,
    pub available: bool,
    pub error: String,
}
#[derive(Default)]
pub(super) struct CatalogState {
    entries: Vec<CatalogEntry>,
    region: usize,
    status: usize,
    kind: usize,
    case: usize,
    index: usize,
    pub preview: Vec<[u8; 4]>,
    pub message: String,
}
const REGIONS: [&str; 3] = ["All", "EUR", "USA"];
const STATES: [&str; 6] = [
    "All",
    "validata",
    "parziale",
    "non_coperta",
    "variante",
    "origine",
];
const KINDS: [&str; 6] = ["All", "case", "variant", "IN", "GEN", "NEW"];
impl CatalogState {
    pub(super) fn choices(&self,row:Row)->Vec<String> {
        match row {
            Row::CatalogRegion=>REGIONS.iter().map(|s|s.to_string()).collect(),
            Row::CatalogStatus=>STATES.iter().map(|s|s.to_string()).collect(),
            Row::CatalogKind=>KINDS.iter().map(|s|s.to_string()).collect(),
            Row::CatalogCase=>self.cases(),
            Row::CatalogEntry=>self.matches().iter().map(|i|self.entries[*i].name.clone()).collect(),
            _=>Vec::new(),
        }
    }
    pub(super) fn choice_index(&self,row:Row)->usize {
        match row {Row::CatalogRegion=>self.region,Row::CatalogStatus=>self.status,Row::CatalogKind=>self.kind,Row::CatalogCase=>self.case,Row::CatalogEntry=>self.index,_=>0}
    }
    pub(super) fn details(&self) -> String {
        self.selected()
            .map(|i| {
                let e = &self.entries[i];
                format!(
                    "{}\n{} · {} · {} · {}\nSorgente: {}\nProva: {}\n{}",
                    e.name, e.kind, e.case, e.region, e.status, e.source, e.evidence, e.error
                )
            })
            .unwrap_or_else(|| "Nessuna voce selezionata".into())
    }

    pub(super) fn selection_state(&self) -> Vec<(String, String)> {
        let mut state: Vec<_> = [
            Row::CatalogRegion,
            Row::CatalogStatus,
            Row::CatalogKind,
            Row::CatalogCase,
            Row::CatalogEntry,
        ]
        .into_iter()
        .map(|r| (format!("catalog.{r:?}"), self.value(r)))
        .collect();
        state.push(("catalog.feedback".into(), self.message.clone()));
        if let Some(index) = self.selected() {
            let entry = &self.entries[index];
            state.push(("catalog.item_id".into(), entry.key.clone()));
            state.push(("catalog.label".into(), entry.name.clone()));
        }
        state
    }
    pub fn new(entries: Vec<CatalogEntry>, region: &str) -> Self {
        Self {
            entries,
            region: REGIONS.iter().position(|r| *r == region).unwrap_or(0),
            ..Default::default()
        }
    }
    fn cases(&self) -> Vec<String> {
        let mut v: Vec<_> = self
            .entries
            .iter()
            .filter(|e| !e.case.is_empty())
            .map(|e| e.case.clone())
            .collect();
        v.sort();
        v.dedup();
        v.insert(0, "All".into());
        v
    }
    fn matches(&self) -> Vec<usize> {
        let cases = self.cases();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                (self.region == 0 || e.region.is_empty() || e.region == REGIONS[self.region])
                    && (self.status == 0 || e.status == STATES[self.status])
                    && (self.kind == 0 || e.kind == KINDS[self.kind])
                    && (self.case == 0 || e.case == cases[self.case])
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn selected(&self) -> Option<usize> {
        self.matches().get(self.index).copied()
    }
    pub fn handle(
        &mut self,
        row: Row,
        input: MenuInput,
        isolated: bool,
    ) -> Option<Option<MenuAction>> {
        let advance = |value: &mut usize, count: usize| {
            *value = (*value
                + if input == MenuInput::Left {
                    count - 1
                } else {
                    1
                })
                % count
        };
        match row {
            Row::CatalogRegion => advance(&mut self.region, REGIONS.len()),
            Row::CatalogStatus => advance(&mut self.status, STATES.len()),
            Row::CatalogKind => advance(&mut self.kind, KINDS.len()),
            Row::CatalogCase => {
                let n = self.cases().len();
                advance(&mut self.case, n);
            }
            Row::CatalogEntry => {
                let n = self.matches().len();
                if n > 0 {
                    advance(&mut self.index, n);
                }
            }
            Row::CatalogImport | Row::CatalogOpen if input == MenuInput::Confirm => {
                if row == Row::CatalogImport && !isolated {
                    self.message = "Import requires a debug copy. Use Open isolated case.".into();
                    return Some(None);
                }
                let Some(index) = self.selected() else {
                    return Some(None);
                };
                if !self.entries[index].available {
                    self.message =
                        "No verified file for this entry; its recorded status is unchanged.".into();
                    return Some(None);
                }
                return Some(Some(if row == Row::CatalogImport {
                    MenuAction::CatalogImport(index)
                } else {
                    MenuAction::CatalogOpen(index)
                }));
            }
            Row::CatalogChoose if input == MenuInput::Confirm => {
                return Some(Some(MenuAction::ChooseDebugCatalog));
            }
            _ => return None,
        }
        if row != Row::CatalogEntry {
            self.index = 0;
        }
        self.preview.clear();
        self.message.clear();
        Some(self.selected().map(MenuAction::CatalogSelection))
    }
    fn value(&self, row: Row) -> String {
        match row {
            Row::CatalogRegion => REGIONS[self.region].into(),
            Row::CatalogStatus => STATES[self.status].into(),
            Row::CatalogKind => KINDS[self.kind].into(),
            Row::CatalogCase => self.cases()[self.case].clone(),
            Row::CatalogEntry => self.selected().map_or_else(
                || "No matches".into(),
                |i| {
                    format!(
                        "{} / {}  {}",
                        self.index + 1,
                        self.matches().len(),
                        self.entries[i].key
                    )
                },
            ),
            _ => String::new(),
        }
    }
}
pub(super) fn layout(w: usize, h: usize) -> Layout {
    let pad = 8.min(w / 12).max(2);
    let wide = w >= 470;
    let preview = if wide {
        0
    } else {
        h.saturating_sub(9 * 22 + 48).min(100)
    };
    let header = 30 + preview;
    let line_h = (h.saturating_sub(header + 12) / if wide { 5 } else { 9 }).clamp(12, 34);
    Layout {
        panel_x: 0,
        panel_y: 0,
        panel_w: w,
        panel_h: h,
        pad,
        line_h,
        rows_y: header,
        visible_rows: 9,
    }
}
pub(super) fn row_rect(index: usize, w: usize, h: usize) -> super::clan_atlas::Rect {
    let l = layout(w, h);
    let wide = w >= 470;
    let left = if wide { w * 55 / 100 } else { w };
    if wide && index >= 5 {
        let pitch = (h.saturating_sub(100) / 4).clamp(18, 28);
        super::clan_atlas::Rect {
            x: left + 8,
            y: h.saturating_sub(4 * pitch + 8) + (index - 5) * pitch,
            w: w.saturating_sub(left + 16),
            h: pitch.saturating_sub(3),
        }
    } else {
        super::clan_atlas::Rect {
            x: l.pad,
            y: l.rows_y + index * l.line_h,
            w: left.saturating_sub(2 * l.pad),
            h: l.line_h.saturating_sub(3),
        }
    }
}
pub(super) fn draw(menu: &Menu, px: &mut [[u8; 4]], w: usize, h: usize) {
    let l = layout(w, h);
    let wide = w >= 470;
    let mut c = Canvas::new(px, w, h);
    super::atlas_theme::panel(&mut c, super::clan_atlas::Rect { x: 0, y: 0, w, h }, false);
    let fs = (l.line_h as f32 * 0.46).clamp(10., 16.);
    let ink = [233, 239, 236];
    c.text(
        8.,
        7.,
        "DEBUG SAVE CATALOG",
        ink,
        fs,
        true,
        w.saturating_sub(16) as f32,
    );
    if let Some(i) = menu.catalog.selected() {
        let e = &menu.catalog.entries[i];
        let dx = if wide { w * 55 / 100 + 8 } else { 8 };
        let dw = if wide {
            w.saturating_sub(dx + 8)
        } else {
            w.saturating_sub(16)
        };
        let top = if wide { 8 } else { 30 };
        let bottom = if wide { row_rect(5, w, h).y } else { l.rows_y };
        c.text(dx as f32, top as f32, &e.name, ink, fs, false, dw as f32);
        c.text(
            dx as f32,
            (top + 18) as f32,
            &format!("{} / {} / {}", e.region, e.status, e.kind),
            ink,
            (fs * 0.8).max(9.),
            false,
            dw as f32,
        );
        let ih = bottom.saturating_sub(top + 44).min(120);
        let iw = (ih * 3 / 2).min(dw);
        let iy = top + 38;
        if ih > 0 && iw > 0 {
            c.rect(
                dx as f32,
                iy as f32,
                iw as f32,
                ih as f32,
                1.,
                [17, 43, 64],
                [17, 43, 64],
                [89, 121, 139],
            );
            if menu.catalog.preview.len() == 240 * 160 {
                for y in 0..ih {
                    for x in 0..iw {
                        let at = ((iy + y) * w + dx + x) * 4;
                        if at + 4 <= c.pix.data().len() {
                            c.pix.data_mut()[at..at + 4].copy_from_slice(
                                &menu.catalog.preview[(y * 160 / ih) * 240 + x * 240 / iw],
                            );
                        }
                    }
                }
            } else {
                c.text(
                    (dx + 3) as f32,
                    (iy + ih / 2) as f32,
                    menu.tr("NO PREVIEW"),
                    [168, 189, 199],
                    10.,
                    false,
                    iw.saturating_sub(6) as f32,
                );
            }
        }
    }
    for (i, row) in menu.rows().iter().enumerate() {
        let r = row_rect(i, w, h);
        let active = i == menu.selected && !menu.atlas.sidebar_focus;
        super::atlas_theme::panel(&mut c, r, active);
        let value = menu.catalog.value(*row);
        let size = (r.h as f32 * 0.46).clamp(10., 15.);
        if value.is_empty() {
            super::atlas_theme::text(&mut c, r, row.localized_label(menu.language()), size, false, ink);
        } else {
            let label_width = r.w * 43 / 100;
            c.text(
                (r.x + 6) as f32,
                (r.y + 5) as f32,
                row.localized_label(menu.language()),
                ink,
                size,
                true,
                label_width.saturating_sub(8) as f32,
            );
            c.text(
                (r.x + label_width) as f32,
                (r.y + 5) as f32,
                &value,
                ink,
                size,
                false,
                r.w.saturating_sub(label_width + 6) as f32,
            );
        }
    }
    c.finish(px);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_catalog_selection_preserves_real_key_and_label_without_dispatch() {
        let mut state = CatalogState::new(
            vec![
                CatalogEntry {
                    key: "first-id".into(),
                    name: "same label".into(),
                    ..Default::default()
                },
                CatalogEntry {
                    key: "second-id".into(),
                    name: "same label".into(),
                    ..Default::default()
                },
            ],
            "All",
        );
        let a: std::collections::BTreeMap<_, _> = state.selection_state().into_iter().collect();
        state.index = 1;
        let b: std::collections::BTreeMap<_, _> = state.selection_state().into_iter().collect();
        assert_eq!(a["catalog.item_id"], "first-id");
        assert_eq!(b["catalog.item_id"], "second-id");
        assert_eq!(a["catalog.label"], b["catalog.label"]);
        assert_eq!(state.index, 1);
        assert!(state.preview.is_empty());
        assert!(state.message.is_empty());
    }
    fn entries() -> Vec<CatalogEntry> {
        vec![
            CatalogEntry {
                key: "DS-20/EUR/20".into(),
                name: "Failed dispatch".into(),
                kind: "case".into(),
                case: "DS-20".into(),
                region: "EUR".into(),
                status: "validata".into(),
                slot: Some(20),
                source: "USER-DS-20-EUR".into(),
                available: true,
                ..Default::default()
            },
            CatalogEntry {
                key: "DS-03//none".into(),
                kind: "case".into(),
                case: "DS-03".into(),
                status: "non_coperta".into(),
                ..Default::default()
            },
            CatalogEntry {
                key: "IN-20/EUR".into(),
                kind: "IN".into(),
                region: "EUR".into(),
                status: "origine".into(),
                available: true,
                ..Default::default()
            },
        ]
    }
    #[test]
    fn filters_keep_uncovered_cases_and_in_identifiers_distinct() {
        let mut s = CatalogState::new(entries(), "EUR");
        assert_eq!(s.matches(), vec![0, 1, 2]);
        s.handle(Row::CatalogKind, MenuInput::Right, true);
        assert_eq!(s.matches(), vec![0, 1]);
        s.handle(Row::CatalogCase, MenuInput::Left, true);
        assert_eq!(s.matches(), vec![0]);
        assert_eq!(
            s.handle(Row::CatalogImport, MenuInput::Confirm, false),
            Some(None)
        );
        assert!(s.message.contains("debug copy"));
        assert_eq!(
            s.handle(Row::CatalogImport, MenuInput::Confirm, true),
            Some(Some(MenuAction::CatalogImport(0)))
        );
        s.handle(Row::CatalogCase, MenuInput::Left, true);
        assert_eq!(s.matches(), vec![1]);
        assert_eq!(
            s.handle(Row::CatalogOpen, MenuInput::Confirm, true),
            Some(None)
        );
    }
    #[test]
    fn catalog_details_preview_and_actions_render_with_shared_hit_geometry() {
        let mut menu = Menu::unlocked_for_test(true, true);
        menu.set_catalog(entries(), "EUR");
        menu.catalog_preview(vec![[70, 90, 120, 255]; 240 * 160]);
        let av = input_config::AvConfig::default();
        for (w, h) in [(852, 480), (1280, 720), (620, 480)] {
            let mut px = vec![[0, 0, 0, 255]; w * h];
            menu.draw(
                &av,
                &std::array::from_fn(|_| String::new()),
                "",
                "",
                "",
                "",
                &mut px,
                w,
                h,
            );
            let l = layout(w, h);
            assert!(l.rows_y + 9 * l.line_h <= h);
            assert!(px.iter().any(|p| p[0] > 30));
        }
    }
}
