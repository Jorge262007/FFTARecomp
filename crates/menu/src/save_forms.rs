//! Save forms keep readable controls beside the preview inside the fixed Atlas body.
use super::{
    atlas_theme::{panel, text},
    canvas::Canvas,
    clan_atlas::Rect,
    *,
};
const INK: [u8; 3] = [233, 239, 236];
const MUTED: [u8; 3] = [168, 189, 199];
pub(super) fn layout(page: Page, n: usize, w: usize, h: usize) -> Layout {
    let state = matches!(page, Page::SaveStates | Page::ReplaceState);
    let wide = w >= 470;
    let pad = 8.min(w / 10).max(1);
    let header = if state && !wide {
        h.saturating_sub(n * 30 + 28).min(170).max(30)
    } else {
        30
    };
    let line_h = (h.saturating_sub(header + pad + 24) / n.max(1)).clamp(12, 38);
    Layout {
        panel_x: 0,
        panel_y: 0,
        panel_w: if wide { w * 55 / 100 } else { w },
        panel_h: h,
        pad,
        line_h,
        rows_y: header,
        visible_rows: n,
    }
}
pub(super) fn draw(menu: &Menu, px: &mut [[u8; 4]], w: usize, h: usize) {
    let rows = menu.rows();
    let l = layout(menu.page, rows.len(), w, h);
    let wide = w >= 470;
    let state = matches!(menu.page, Page::SaveStates | Page::ReplaceState);
    let game = matches!(
        menu.page,
        Page::GameSaves | Page::SwitchGameSave | Page::ReplaceGameSave
    );
    let mut c = Canvas::new(px, w, h);
    panel(&mut c, Rect { x: 0, y: 0, w, h }, false);
    let fs = (l.line_h as f32 * 0.43).clamp(10., 16.);
    let caption = if state {
        format!(
            "Slot {} / {} · {}",
            menu.state_slot + 1,
            menu.state_slots.len(),
            menu.tr(&menu.state_slots[menu.state_slot].status)
        )
    } else if game {
        menu.game_save_slots
            .get(menu.game_save_slot)
            .map(|s| {
                format!(
                    "Slot {} / 99 · {}{}",
                    menu.game_save_slot + 1,
                    menu.tr(&s.status),
                    if s.current { menu.tr(" · attivo") } else { "" }
                )
            })
            .unwrap_or_else(|| menu.tr("Salvataggi non disponibili").into())
    } else {
        menu.tr("Backup cartuccia · due copie automatiche").into()
    };
    c.text(
        8.,
        7.,
        &caption,
        INK,
        fs,
        false,
        w.saturating_sub(16) as f32,
    );
    let detail = if wide {
        Rect {
            x: l.panel_w + 8,
            y: 32,
            w: w.saturating_sub(l.panel_w + 16),
            h: h.saturating_sub(40),
        }
    } else {
        Rect {
            x: 8,
            y: 30,
            w: w.saturating_sub(16),
            h: l.rows_y.saturating_sub(38),
        }
    };
    if state && detail.h >= 20 && detail.w >= 30 {
        let slot = &menu.state_slots[menu.state_slot];
        let iw = detail.w.min(detail.h.saturating_sub(22) * 3 / 2).min(360);
        let ih = iw * 2 / 3;
        let ix = detail.x + (detail.w - iw) / 2;
        let iy = detail.y;
        c.rect(
            ix as f32,
            iy as f32,
            iw as f32,
            ih as f32,
            1.,
            [17, 43, 64],
            [17, 43, 64],
            [89, 121, 139],
        );
        if slot.preview.len() == 240 * 160 && iw > 0 && ih > 0 {
            for y in 0..ih {
                for x in 0..iw {
                    let mut p = slot.preview[(y * 160 / ih) * 240 + x * 240 / iw];
                    p[3] = 255;
                    let off = ((iy + y) * w + ix + x) * 4;
                    c.pix.data_mut()[off..off + 4].copy_from_slice(&p);
                }
            }
        } else {
            c.text(
                (ix + 4) as f32,
                (iy + ih / 2) as f32,
                if slot.occupied {
                    menu.tr("Anteprima non disponibile")
                } else {
                    menu.tr("Slot vuoto")
                },
                MUTED,
                fs * 0.8,
                false,
                iw.saturating_sub(8) as f32,
            );
        }
        let stamp = menu
            .state_times
            .get(menu.state_slot)
            .copied()
            .flatten()
            .and_then(clan_atlas::timestamp_utc)
            .unwrap_or_else(|| menu.tr("Data non disponibile").into());
        c.text(
            detail.x as f32,
            (iy + ih + 5) as f32,
            &stamp,
            MUTED,
            (fs * 0.75).max(9.),
            false,
            detail.w as f32,
        );
    } else if wide {
        let description = if game {
            let name = menu
                .game_save_slots
                .get(menu.game_save_slot)
                .map(|s| s.name.as_str())
                .unwrap_or("");
            menu.tr_format("{name}\nLe copie contengono il progresso salvato nel gioco. Usare un file riavvia il gioco. Sostituire richiede conferma; Annulla è preselezionato.", &[("name", (name).to_string())])
        } else if menu.page == Page::RestoreBackup {
            menu.tr("Il gioco riparte da questa copia. Confermare sostituisce il salvataggio cartuccia attivo; il progresso della sessione viene sostituito.").into()
        } else {
            menu.tr("Copie automatiche create all'avvio. Selezionare una copia richiede conferma prima del ripristino.").into()
        };
        for (i, line) in Canvas::wrap_text(&description, fs, detail.w as f32)
            .iter()
            .take(detail.h / (fs as usize + 5))
            .enumerate()
        {
            c.text(
                detail.x as f32,
                (detail.y + i * (fs as usize + 5)) as f32,
                line,
                MUTED,
                fs,
                false,
                detail.w as f32,
            );
        }
    }
    for (i, row) in rows.iter().enumerate() {
        let r = Rect {
            x: l.pad,
            y: l.rows_y + i * l.line_h,
            w: l.panel_w.saturating_sub(l.pad * 2),
            h: l.line_h.saturating_sub(3),
        };
        panel(&mut c, r, i == menu.selected && !menu.atlas.sidebar_focus);
        let label = match row {
            Row::StateSlot => format!(
                "‹ Slot {} / {} ›",
                menu.state_slot + 1,
                menu.state_slots.len()
            ),
            Row::GameSaveSlot => format!("‹ Slot {} / 99 ›", menu.game_save_slot + 1),
            Row::UndoState => menu
                .state_undo_text()
                .unwrap_or_else(|| menu.tr("Annulla ultima operazione").into()),
            Row::Backup(copy) => format!(
                "{} · {}",
                row.localized_label(menu.language()),
                menu.tr(&menu.save_backups[*copy as usize].status)
            ),
            Row::Back
                if matches!(
                    menu.page,
                    Page::ReplaceState
                        | Page::ReplaceGameSave
                        | Page::SwitchGameSave
                        | Page::RestoreBackup
                ) =>
            {
                menu.tr("Annulla").into()
            }
            _ => row.localized_label(menu.language()).into(),
        };
        text(
            &mut c,
            r,
            &label,
            fs,
            true,
            if *row == Row::LoadState && !menu.state_slots[menu.state_slot].occupied {
                MUTED
            } else {
                INK
            },
        );
    }
    let message = if state {
        &menu.state_message
    } else if game {
        &menu.game_save_message
    } else {
        &menu.backup_message
    };
    c.text(
        8.,
        (h.saturating_sub(18)) as f32,
        if message.is_empty() {
            menu.tr("Frecce: naviga · OK: scegli/conferma · Annulla: indietro")
        } else {
            menu.tr(message)
        },
        MUTED,
        10.,
        false,
        w.saturating_sub(16) as f32,
    );
    c.finish(px);
}
