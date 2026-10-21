//! Native, scalable parchment UI. No HTML engine or game assets.
use super::canvas::Canvas;
use super::*;
const GOLD: [u8; 3] = [255, 197, 51];
const BRASS: [u8; 3] = [184, 146, 65];
const INK: [u8; 3] = [35, 24, 16];
const LIGHT: [u8; 3] = [254, 252, 245];
const PAPER: [u8; 3] = [235, 220, 185];
pub(super) fn segment_geometry(l: Layout) -> (usize, usize) {
    let width = ((l.panel_w.saturating_sub(l.pad * 2)) / 6).max(16);
    (l.panel_x + l.panel_w - l.pad - (width * 3 + 4), width)
}
pub(super) fn segment_hit(row: Row, l: Layout, x: usize) -> Option<usize> {
    if !matches!(row, Row::FastForward | Row::FastForwardAudio) {
        return None;
    }
    let (start, width) = segment_geometry(l);
    (0..3).find(|&i| (start + i * (width + 2)..start + i * (width + 2) + width).contains(&x))
}
fn binding_value(
    menu: &Menu,
    row: Row,
    av: &AvConfig,
    keys: &[String; 10],
    ff: &str,
    toggle: &str,
    zin: &str,
    zout: &str,
) -> Option<String> {
    if row == Row::Menu {
        return Some(if matches!(menu.capture, Some(Bind::Menu | Bind::GamepadMenu)) {
            "HOLD + RELEASE".into()
        } else {
            menu.menu_binding.borrow().split('+').map(|token| {
                if menu.page == Page::Gamepad { gamepad_label(token, menu.controller_layout.get()) }
                else { token.to_uppercase() }
            }).collect::<Vec<_>>().join(" + ")
        });
    }
    if menu.page == Page::Gamepad && matches!(row, Row::FfKey | Row::FfToggleKey) {
        let b = if row == Row::FfKey {
            Bind::GamepadFastForward
        } else {
            Bind::GamepadFastForwardToggle
        };
        return Some(if menu.capture == Some(b) {
            "PRESS BUTTON".into()
        } else {
            gamepad_label(
                if row == Row::FfKey { ff } else { toggle },
                menu.controller_layout.get(),
            )
        });
    }
    if row == Row::FfToggleKey {
        return Some(if menu.capture == Some(Bind::FastForwardToggle) {
            "PRESS KEY".into()
        } else {
            toggle.to_uppercase()
        });
    }
    if matches!(row, Row::ZoomIn | Row::ZoomOut) {
        let pad = if row == Row::ZoomIn {
            Bind::GamepadZoomIn
        } else {
            Bind::GamepadZoomOut
        };
        let key = if row == Row::ZoomIn {
            Bind::ZoomIn
        } else {
            Bind::ZoomOut
        };
        return Some(if menu.capture == Some(pad) {
            "PRESS BUTTON".into()
        } else if menu.capture == Some(key) {
            "PRESS KEY".into()
        } else {
            let value = if row == Row::ZoomIn { zin } else { zout };
            if menu.page == Page::Gamepad {
                gamepad_label(value, menu.controller_layout.get())
            } else {
                value.to_uppercase()
            }
        });
    }
    if menu.page == Page::Gamepad {
        if let Row::Key(b) = row {
            return Some(if menu.capture == Some(Bind::Gamepad(b)) {
                "PRESS BUTTON".into()
            } else {
                gamepad_label(&keys[b.index()], menu.controller_layout.get())
            });
        }
    }
    value_of(row, av, keys, ff, menu.capture)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    menu: &Menu,
    av: &AvConfig,
    keys: &[String; 10],
    ff: &str,
    toggle: &str,
    zin: &str,
    zout: &str,
    px: &mut [[u8; 4]],
    w: usize,
    h: usize,
) {
    if w < 32 || h < 32 || px.len() < w * h {
        return;
    }
    if matches!(menu.page, Page::SaveStates | Page::ReplaceState) {
        super::save_states::draw(menu, px, w, h);
        return;
    }
    if matches!(menu.page, Page::SaveBackups | Page::RestoreBackup) {
        super::save_backups::draw(menu,px,w,h);
        return;
    }
    for p in px.iter_mut().take(w * h) {
        p[0] = (p[0] as u16 * 5 / 16 + 5) as u8;
        p[1] = (p[1] as u16 * 4 / 16 + 3) as u8;
        p[2] = (p[2] as u16 * 3 / 16 + 1) as u8;
    }
    let mut c = Canvas::new(px, w, h);
    let rows = rows_of(menu.page);
    let l = menu.page_layout(w,h);
    let first = menu.first_visible(l);
    let scale = l.line_h as f32 / 44.;
    let fs = 16. * scale;
    let title = 18. * scale;
    let (x, y, pw, ph, pad) = (
        l.panel_x as f32,
        l.panel_y as f32,
        l.panel_w as f32,
        l.panel_h as f32,
        l.pad as f32,
    );
    c.rect(
        x + 4.,
        y + 7.,
        pw,
        ph,
        9. * scale,
        [8, 5, 3],
        [8, 5, 3],
        [8, 5, 3],
    );
    c.rect(x, y, pw, ph, 8. * scale, [251, 244, 226], PAPER, BRASS);
    c.rect(
        x + 5. * scale,
        y + 5. * scale,
        pw - 10. * scale,
        ph - 10. * scale,
        5. * scale,
        [251, 244, 226],
        PAPER,
        [208, 184, 136],
    );
    if menu.page != Page::Main {
        let label = match menu.page {
            Page::Exit => "QUIT GAME?",
            Page::SaveStates => "SAVE STATES",
            Page::FastForward => "FAST FORWARD",
            Page::Audio => "AUDIO SETTINGS",
            Page::Video => "VIDEO & DISPLAY",
            Page::TextBubbles => "TEXT BUBBLES",
            Page::Inputs | Page::Gamepad | Page::MouseKeyboard => "CONTROLS & INPUT",
            _ => "",
        };
        c.text(x + pad, y + pad, label, INK, title, true, pw - pad * 2.);
        c.line(
            x + pad,
            y + pad + 30. * scale,
            x + pw - pad,
            y + pad + 30. * scale,
            BRASS,
            scale,
        );
    }
    if matches!(menu.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard) {
        let width = (pw - pad * 2. - 12. * scale) / 3.;
        for (i, label) in ["KEYBOARD", "GAMEPAD", "MOUSE & KEYBOARD"].iter().enumerate() {
            let disabled=i==0 && menu.mouse_mode.get();
            let active = i == match menu.page {Page::Gamepad=>1,Page::MouseKeyboard=>2,_=>0};
            let xx = x + pad + i as f32 * (width + 6. * scale);
            let yy = y + pad + l.line_h as f32;
            c.rect(
                xx,
                yy,
                width,
                30. * scale,
                4. * scale,
                if active { LIGHT } else { [216, 195, 150] },
                if active {
                    [246, 235, 205]
                } else {
                    [206, 181, 128]
                },
                BRASS,
            );
            c.text(
                xx + 12. * scale,
                yy + 7. * scale,
                label,
                if disabled {[145,133,111]}else{INK},
                if i==2 {fs*0.70}else{fs},
                true,
                width - 20. * scale,
            );
        }
    }
    if menu.page == Page::Exit {
        c.text(
            x + pad,
            y + pad + 46. * scale,
            "Unsaved progress will be lost.",
            [107, 78, 45],
            fs,
            false,
            pw - pad * 2.,
        );
    }
    let count = rows.len() - usize::from(matches!(menu.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard));
    for (visible, &row) in rows[..count]
        .iter()
        .skip(first)
        .take(l.visible_rows)
        .enumerate()
    {
        let sel = first + visible == menu.selected;
        let yy = (l.rows_y + visible * l.line_h) as f32;
        let rh = l.line_h as f32 - 6. * scale;
        let (xx, bw) = if row == Row::Back {
            (x + pw - pad - l.line_h as f32 * 3., l.line_h as f32 * 3.)
        } else {
            (x + pad, pw - pad * 2.)
        };
        c.rect(
            xx,
            yy,
            bw,
            rh,
            5. * scale,
            if sel { [59, 40, 24] } else { LIGHT },
            if sel { [24, 15, 9] } else { [237, 219, 169] },
            if sel { GOLD } else { BRASS },
        );
        let fg = if sel { GOLD } else { INK };
        let ty = yy + (rh - fs) / 2.;
        let tx = xx + 18. * scale;
        if row==Row::MouseMode {
            c.text(tx,yy+3.*scale,"Enable (Experimental)",fg,fs*0.72,true,bw-104.*scale);
            c.text(tx,yy+17.*scale,"Keyboard-only is unavailable",fg,fs*0.50,false,bw-104.*scale);
            c.text(tx,yy+27.*scale,"until this mode is disabled.",fg,fs*0.50,false,bw-104.*scale);
            let on=menu.mouse_mode.get();
            let sx=xx+bw-68.*scale;let sy=yy+9.*scale;
            c.rect(sx,sy,52.*scale,24.*scale,12.*scale,if on {BRASS}else{[145,133,111]},if on {GOLD}else{[181,167,137]},BRASS);
            c.rect(sx+if on {30.*scale}else{3.*scale},sy+3.*scale,18.*scale,18.*scale,9.*scale,LIGHT,LIGHT,BRASS);
            continue;
        }
        if matches!(row, Row::FastForward | Row::FastForwardAudio) {
            let (sx, sw) = segment_geometry(l);
            c.text(
                tx,
                if row == Row::FastForwardAudio { ty - 6. * scale } else { ty },
                row.label(),
                fg,
                fs,
                true,
                (sx as f32 - tx - 8. * scale).max(1.),
            );
            if row == Row::FastForwardAudio {
                c.text(tx, ty + 13. * scale, "1X: UP TO 2S DELAY", fg, fs * 0.62,
                    false, (sx as f32 - tx - 8. * scale).max(1.));
            }
            let labels = if row == Row::FastForward {
                ["2X", "4X", "8X"]
            } else {
                ["1X", "SILENT", "FAST"]
            };
            for i in 0..3 {
                let active = if row == Row::FastForward {
                    av.ff_speed == FF_SPEEDS[i]
                } else {
                    av.ff_audio == ["normal", "silent", "fast"][i]
                };
                let ax = (sx + i * (sw + 2)) as f32;
                c.rect(
                    ax,
                    yy + 2. * scale,
                    sw as f32,
                    rh - 4. * scale,
                    4. * scale,
                    if active { [255, 225, 142] } else { LIGHT },
                    if active { [221, 179, 75] } else { PAPER },
                    if active { GOLD } else { BRASS },
                );
                c.text(
                    ax + 6. * scale,
                    ty,
                    labels[i],
                    INK,
                    fs * 0.85,
                    false,
                    sw as f32 - 8. * scale,
                );
            }
            continue;
        }
        let val = if menu.page == Page::Main {
            None
        } else {
            binding_value(menu, row, av, keys, ff, toggle, zin, zout)
        };
        let controller = menu.page == Page::Gamepad
            && val
                .as_deref()
                .is_some_and(super::controller_icons::supported);
        let vw = if controller {
            60. * scale
        } else if val.is_some() {
            (bw * 0.46).max(40. * scale)
        } else {
            0.
        };
        let vx = xx + bw - vw - 8. * scale;
        c.text(
            tx,
            if row == Row::Fps { yy + 2. * scale } else { ty },
            if row == Row::Audio && !menu.audio_live {
                "ENHANCED (RESTART)"
            } else {
                row.label()
            },
            fg,
            if row == Row::Fps { fs * 0.82 } else { fs },
            true,
            if val.is_some() {
                (vx - tx - 8. * scale).max(1.)
            } else {
                bw - 36. * scale
            },
        );
        if row == Row::Fps {
            let help_width = (vx - tx - 8. * scale).max(1.);
            c.text(tx, yy + 16. * scale, "INT: GAME / EXT: PRESENTS", fg,
                fs * 0.55, false, help_width);
            c.text(tx, yy + 27. * scale, "FAST CAN SHOW DIFFERENT RATES", fg,
                fs * 0.52, false, help_width);
        }
        if let Some(v) = val {
            if controller {
                super::controller_icons::draw(
                    &mut c,
                    &v,
                    xx + bw - 42. * scale,
                    yy + 3. * scale,
                    32. * scale,
                );
            } else {
                c.rect(
                    vx,
                    yy + 3. * scale,
                    vw,
                    rh - 6. * scale,
                    4. * scale,
                    if sel { [96, 65, 26] } else { [247, 227, 178] },
                    if sel { [68, 44, 20] } else { [242, 217, 160] },
                    if sel { GOLD } else { BRASS },
                );
                c.text(
                    vx + 8. * scale,
                    ty,
                    &v,
                    if sel { [255, 244, 211] } else { [106, 76, 44] },
                    fs,
                    false,
                    vw - 16. * scale,
                );
            }
        }
        if menu.page == Page::Main {
            let ax = xx + bw - 27. * scale;
            let ay = yy + rh / 2.;
            super::menu_icons::draw(&mut c, row, ax + 5. * scale, ay, scale, fg);
        }
    }
    if count > l.visible_rows {
        let track = (l.visible_rows * l.line_h) as f32;
        let thumb = (track * l.visible_rows as f32 / count as f32).max(8. * scale);
        let sy = l.rows_y as f32
            + (track - thumb) * first as f32 / count.saturating_sub(l.visible_rows).max(1) as f32;
        c.rect(
            x + pw - 10. * scale,
            sy,
            3. * scale,
            thumb,
            1. * scale,
            BRASS,
            BRASS,
            BRASS,
        );
    }
    if matches!(menu.page, Page::Inputs | Page::Gamepad | Page::MouseKeyboard) {
        let sel = menu.selected + 1 == rows.len();
        let bw = l.line_h as f32 * 3.;
        let xx = x + pw - pad - bw;
        let yy = y + ph - pad - l.line_h as f32 + 6.;
        c.rect(
            xx,
            yy,
            bw,
            l.line_h as f32 - 6.,
            5. * scale,
            if sel { [59, 40, 24] } else { LIGHT },
            if sel { [24, 15, 9] } else { PAPER },
            if sel { GOLD } else { BRASS },
        );
        c.text(
            xx + 20. * scale,
            yy + 10. * scale,
            "BACK",
            if sel { GOLD } else { INK },
            fs,
            true,
            bw - 30. * scale,
        );
    }
    if let Some(dd) = menu.dropdown.as_ref() {
        draw_dropdown(dd, &mut c, w, h);
    }
    c.finish(px);
}
fn dropdown_geometry(
    dd: &Dropdown,
    w: usize,
    h: usize,
) -> (usize, usize, usize, usize, usize, usize) {
    let n = setting_options(dd.row).len();
    let pitch = (h / 15).clamp(14, 40);
    let visible = ((h.saturating_sub(24)) / pitch).max(1).min(n);
    let mut first = dd.first.get().min(n.saturating_sub(visible));
    if dd.index < first {
        first = dd.index;
    }
    if dd.index >= first + visible {
        first = dd.index + 1 - visible;
    }
    dd.first.set(first);
    let width = w.saturating_sub(20).min(320);
    let height = visible * pitch + 12;
    (
        w.saturating_sub(width) / 2,
        h.saturating_sub(height) / 2,
        width,
        height,
        visible,
        first,
    )
}
pub(super) fn dropdown_hit(dd: &Dropdown, x: usize, y: usize, w: usize, h: usize) -> Option<usize> {
    let (xx, yy, rw, _, visible, first) = dropdown_geometry(dd, w, h);
    if !(xx..xx + rw).contains(&x) || y < yy + 6 {
        return None;
    }
    let i = (y - yy - 6) / (h / 15).clamp(14, 40);
    (i < visible).then_some(first + i)
}
fn draw_dropdown(dd: &Dropdown, c: &mut Canvas, w: usize, h: usize) {
    let (x, y, rw, rh, visible, first) = dropdown_geometry(dd, w, h);
    let pitch = (h / 15).clamp(14, 40);
    c.rect(
        x as f32,
        y as f32,
        rw as f32,
        rh as f32,
        6.,
        [59, 40, 24],
        [24, 15, 9],
        GOLD,
    );
    for (i, s) in setting_options(dd.row)
        .iter()
        .skip(first)
        .take(visible)
        .enumerate()
    {
        let yy = (y + 6 + i * pitch) as f32;
        let sel = first + i == dd.index;
        if sel {
            c.rect(
                x as f32 + 4.,
                yy,
                rw as f32 - 8.,
                pitch as f32 - 2.,
                3.,
                [96, 65, 26],
                [68, 44, 20],
                BRASS,
            );
        }
        c.text(
            x as f32 + 14.,
            yy + 4.,
            s,
            if sel { GOLD } else { [232, 216, 176] },
            pitch as f32 * 0.6,
            false,
            rw as f32 - 24.,
        );
    }
}
