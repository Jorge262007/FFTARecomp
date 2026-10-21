//! Controller-friendly slot picker and embedded screenshot preview.
use super::{canvas::Canvas, rows_of, Layout, Menu, Page, Row};

#[derive(Clone)]
pub struct StateSlot {
    pub occupied: bool,
    /// 240x160 RGBA game screenshot; empty for a legacy state or unavailable slot.
    pub preview: Vec<[u8; 4]>,
    pub status: String,
}
impl Default for StateSlot {
    fn default() -> Self {
        Self {
            occupied: false,
            preview: Vec::new(),
            status: "Empty".into(),
        }
    }
}

pub(super) fn layout(n: usize, w: usize, h: usize) -> Layout {
    let scale = (w as f32 / 560.).min(h as f32 / 480.).max(0.05);
    let pad = (14. * scale).round().max(2.) as usize;
    let line_h = (34. * scale).round().max(3.) as usize;
    let panel_w = ((480. * scale) as usize).min(w.saturating_sub(pad * 2));
    let header = (218. * scale) as usize;
    let panel_h = (pad * 2 + header + n * line_h + (36. * scale) as usize).min(h);
    let panel_x = w.saturating_sub(panel_w) / 2;
    let panel_y = h.saturating_sub(panel_h) / 2;
    Layout {
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        pad,
        line_h,
        rows_y: panel_y + pad + header,
        visible_rows: n,
    }
}

pub(super) fn draw(menu: &Menu, px: &mut [[u8; 4]], w: usize, h: usize) {
    for p in px.iter_mut().take(w * h) {
        for c in &mut p[..3] {
            *c = (*c as u16 * 5 / 16) as u8;
        }
    }
    let rows = rows_of(menu.page);
    let l = layout(rows.len(), w, h);
    let scale = l.line_h as f32 / 34.;
    let mut c = Canvas::new(px, w, h);
    let ink = [35, 24, 16];
    let x = (l.panel_x + l.pad) as f32;
    let width = l.panel_w.saturating_sub(2 * l.pad) as f32;
    c.rect(
        l.panel_x as f32,
        l.panel_y as f32,
        l.panel_w as f32,
        l.panel_h as f32,
        7. * scale,
        [251, 244, 226],
        [235, 220, 185],
        [184, 146, 65],
    );
    c.text(
        x,
        (l.panel_y + l.pad) as f32,
        if menu.page == Page::ReplaceState {
            "REPLACE SAVE STATE?"
        } else {
            "SAVE STATES"
        },
        ink,
        18. * scale,
        true,
        width,
    );
    let slot = &menu.state_slots[menu.state_slot];
    let label = format!("Slot {} / 9 - {}", menu.state_slot + 1, slot.status);
    c.text(
        x,
        (l.panel_y + l.pad) as f32 + 27. * scale,
        &label,
        ink,
        14. * scale,
        false,
        width,
    );
    let ih = (150. * scale).max(1.) as usize;
    let iw = ih * 3 / 2;
    let ix = l.panel_x + l.panel_w.saturating_sub(iw) / 2;
    let iy = l.panel_y + l.pad + (49. * scale) as usize;
    c.rect(
        ix as f32 - 2.,
        iy as f32 - 2.,
        iw as f32 + 4.,
        ih as f32 + 4.,
        0.,
        [40, 33, 24],
        [40, 33, 24],
        [184, 146, 65],
    );
    if slot.preview.len() == 240 * 160 {
        // Nearest-neighbour screenshot scaling, bounded to the UI canvas.
        for y in 0..ih.min(h.saturating_sub(iy)) {
            for x in 0..iw.min(w.saturating_sub(ix)) {
                let mut p = slot.preview[(y * 160 / ih) * 240 + x * 240 / iw];
                p[3] = 255;
                let off = ((iy + y) * w + ix + x) * 4;
                c.pix.data_mut()[off..off + 4].copy_from_slice(&p);
            }
        }
    } else {
        c.text(
            ix as f32 + 8. * scale,
            iy as f32 + 65. * scale,
            if slot.occupied {
                "NO PREVIEW"
            } else {
                "EMPTY SLOT"
            },
            [235, 220, 185],
            16. * scale,
            true,
            iw as f32 - 16. * scale,
        );
    }
    for (i, row) in rows.iter().enumerate() {
        let y = (l.rows_y + i * l.line_h) as f32;
        let selected = i == menu.selected;
        let fg = if selected { [255, 244, 211] } else { ink };
        c.rect(
            x,
            y,
            width,
            l.line_h as f32 - 3. * scale,
            4. * scale,
            if selected {
                [84, 49, 17]
            } else {
                [254, 252, 245]
            },
            if selected {
                [35, 20, 9]
            } else {
                [235, 220, 185]
            },
            [184, 146, 65],
        );
        let label = if *row == Row::StateSlot {
            format!("<   Slot {} / 9   >", menu.state_slot + 1)
        } else if *row == Row::UndoState {
            menu.state_undo
                .clone()
                .unwrap_or_else(|| "Undo last operation".into())
        } else if *row == Row::Back && menu.page == Page::ReplaceState {
            "Cancel".into()
        } else {
            row.label().into()
        };
        c.text(
            x + 10. * scale,
            y + 7. * scale,
            &label,
            fg,
            16. * scale,
            true,
            width - 20. * scale,
        );
    }
    let message = if menu.page == Page::ReplaceState {
        "Replace the state in this slot?"
    } else if !menu.state_message.is_empty() {
        &menu.state_message
    } else {
        "Left/Right: slot   Confirm: select   Back: return"
    };
    c.text(
        x,
        (l.rows_y + rows.len() * l.line_h) as f32 + 5. * scale,
        message,
        ink,
        12. * scale,
        false,
        width,
    );
    c.finish(px);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_and_actions_fit_and_mouse_hits_match_at_supported_sizes() {
        for (w, h) in [(240, 160), (640, 480), (1280, 720), (3840, 2160)] {
            let mut menu = Menu::new(true, true);
            menu.open(Page::SaveStates);
            menu.state_slots[0] = StateSlot {
                occupied: true,
                preview: vec![[200, 20, 80, 255]; 240 * 160],
                status: "Saved".into(),
            };
            let l = layout(5, w, h);
            assert!(l.rows_y + 5 * l.line_h <= h);
            for i in 0..5 {
                assert_eq!(
                    menu.row_at(
                        l.panel_x + l.pad + 1,
                        l.rows_y + i * l.line_h + l.line_h / 2,
                        w,
                        h
                    ),
                    Some(i)
                );
            }
            let mut pixels = vec![[0, 0, 0, 255]; w * h];
            draw(&menu, &mut pixels, w, h);
            assert!(pixels.iter().any(|p| *p == [200, 20, 80, 255]));
        }
    }
}
