//! In-menu restore selection, with a safe default confirmation.
use super::{canvas::Canvas, rows_of, Layout, Menu, Page, Row};
#[derive(Clone)]
pub struct BackupInfo {
    pub generation: Option<u64>,
    pub status: String,
}
impl Default for BackupInfo {
    fn default() -> Self {
        Self {
            generation: None,
            status: "Empty".into(),
        }
    }
}
pub(super) fn layout(page: Page, n: usize, w: usize, h: usize) -> Layout {
    let scale = (w as f32 / 560.).min(h as f32 / 480.).max(0.05);
    let pad = (16. * scale).round().max(2.) as usize;
    let line_h = (44. * scale).round().max(4.) as usize;
    let header = ((if page == Page::RestoreBackup {
        115.
    } else {
        68.
    }) * scale) as usize;
    let panel_w = ((500. * scale) as usize).min(w.saturating_sub(pad * 2));
    let panel_h = (2 * pad + header + n * line_h + (44. * scale) as usize).min(h);
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
    let l = layout(menu.page, rows.len(), w, h);
    let scale = l.line_h as f32 / 44.;
    let mut c = Canvas::new(px, w, h);
    let x = (l.panel_x + l.pad) as f32;
    let y = (l.panel_y + l.pad) as f32;
    let width = l.panel_w.saturating_sub(2 * l.pad) as f32;
    let ink = [35, 24, 16];
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
    let restoring = menu.page == Page::RestoreBackup;
    c.text(
        x,
        y,
        if restoring {
            "RESTORE SAVE BACKUP?"
        } else {
            "CARTRIDGE SAVE BACKUPS"
        },
        ink,
        18. * scale,
        true,
        width,
    );
    if restoring {
        let label = format!(
            "Copy {}",
            menu.backup_selection.map_or(1, |(_, copy)| copy + 1)
        );
        c.text(x, y + 28. * scale, &label, ink, 15. * scale, true, width);
        c.text(
            x,
            y + 53. * scale,
            "Game restarts from this save.",
            ink,
            15. * scale,
            false,
            width,
        );
        c.text(
            x,
            y + 76. * scale,
            "Current session progress is replaced.",
            ink,
            15. * scale,
            false,
            width,
        );
    } else {
        c.text(
            x,
            y + 29. * scale,
            "Two automatic copies made at game startup.",
            ink,
            14. * scale,
            false,
            width,
        );
    }
    for (i, row) in rows.iter().enumerate() {
        let yy = (l.rows_y + i * l.line_h) as f32;
        let selected = i == menu.selected;
        c.rect(
            x,
            yy,
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
        let label = match row {
            Row::Backup(copy) => format!(
                "{} - {}",
                row.label(),
                menu.save_backups[*copy as usize].status
            ),
            Row::Back if restoring => "Cancel".into(),
            _ => row.label().into(),
        };
        c.text(
            x + 10. * scale,
            yy + 11. * scale,
            &label,
            if selected { [255, 244, 211] } else { ink },
            16. * scale,
            true,
            width - 20. * scale,
        );
    }
    let message = if restoring {
        "Confirm replaces the current cartridge save."
    } else if menu.backup_message.is_empty() {
        "Confirm: select   Back: return"
    } else {
        &menu.backup_message
    };
    c.text(
        x,
        (l.rows_y + rows.len() * l.line_h) as f32 + 8. * scale,
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
    fn backup_panels_fit_and_mouse_geometry_matches() {
        for (w, h) in [(240, 160), (1280, 720), (3840, 2160)] {
            for page in [Page::SaveBackups, Page::RestoreBackup] {
                let mut menu = Menu::new(true, true);
                menu.open(page);
                let n = rows_of(page).len();
                let l = layout(page, n, w, h);
                assert!(l.rows_y + n * l.line_h <= h);
                for i in 0..n {
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
                let mut px = vec![[0, 0, 0, 255]; w * h];
                draw(&menu, &mut px, w, h);
            }
        }
    }
}
