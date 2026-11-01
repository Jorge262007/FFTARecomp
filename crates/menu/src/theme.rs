//! Native, scalable parchment UI. No HTML engine or game assets.
use super::canvas::Canvas;
use super::*;
const GOLD: [u8; 3] = [240, 215, 164];
const BRASS: [u8; 3] = [89, 121, 139];
const INK: [u8; 3] = [233, 239, 236];
const LIGHT: [u8; 3] = [28, 56, 78];
const PAPER: [u8; 3] = [28, 56, 78];
pub(super) fn segment_geometry(l: Layout) -> (usize, usize) {
    let width = ((l.panel_w.saturating_sub(l.pad * 2)) / 6).max(16);
    (l.panel_x + l.panel_w - l.pad - (width * 3 + 4), width)
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
    if row == Row::FastForwardToggle {
        return Some(if menu.atlas.fast_active { menu.tr("ON") } else { menu.tr("OFF") }.into());
    }
    if row == Row::Menu {
        return Some(
            if matches!(menu.capture, Some(Bind::Menu | Bind::GamepadMenu)) {
                menu.tr("HOLD + RELEASE").into()
            } else {
                menu.menu_binding
                    .borrow()
                    .split('+')
                    .map(|token| {
                        if menu.page == Page::Gamepad {
                            gamepad_label(token, menu.controller_layout.get())
                        } else {
                            token.to_uppercase()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" + ")
            },
        );
    }
    if menu.page == Page::Gamepad && matches!(row, Row::FfKey | Row::FfToggleKey) {
        let b = if row == Row::FfKey {
            Bind::GamepadFastForward
        } else {
            Bind::GamepadFastForwardToggle
        };
        return Some(if menu.capture == Some(b) {
            menu.tr("PRESS BUTTON").into()
        } else {
            gamepad_label(
                if row == Row::FfKey { ff } else { toggle },
                menu.controller_layout.get(),
            )
        });
    }
    if row == Row::FfToggleKey {
        return Some(if menu.capture == Some(Bind::FastForwardToggle) {
            menu.tr("PRESS KEY").into()
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
            menu.tr("PRESS BUTTON").into()
        } else if menu.capture == Some(key) {
            menu.tr("PRESS KEY").into()
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
                menu.tr("PRESS BUTTON").into()
            } else {
                gamepad_label(&keys[b.index()], menu.controller_layout.get())
            });
        }
    }
    value_of(row, av, keys, ff, menu.capture)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_rows(
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
    let mut c = Canvas::new(px, w, h);
    let rows = menu.rows();
    let l = menu.page_layout(w, h);
    let first = menu.first_visible(l);
    let scale = l.line_h as f32 / 44.;
    let fs = 16. * scale;
    let (x, y, pw, ph, pad) = (
        l.panel_x as f32,
        l.panel_y as f32,
        l.panel_w as f32,
        l.panel_h as f32,
        l.pad as f32,
    );
    if matches!(
        menu.page,
        Page::Inputs | Page::Gamepad | Page::MouseKeyboard
    ) {
        for (i, label) in [menu.tr("KEYBOARD"), menu.tr("GAMEPAD"), menu.tr("MOUSE & KEYBOARD")]
            .iter()
            .enumerate()
        {
            let disabled = i == 0 && menu.mouse_mode.get();
            let active = i
                == match menu.page {
                    Page::Gamepad => 1,
                    Page::MouseKeyboard => 2,
                    _ => 0,
                };
            let r=menu.input_tab_rect(i as u8,w,h);
            let (xx,yy,width)=(r.x as f32,r.y as f32,r.w as f32);
            c.rect(
                xx,
                yy,
                width,
                30. * scale,
                4. * scale,
                if active || menu.input_tabs_focus==Some(i as u8) { [45, 78, 100] } else { LIGHT },
                if active { [45, 78, 100] } else { [28, 56, 78] },
                if menu.input_tabs_focus==Some(i as u8) {GOLD}else{BRASS},
            );
            c.text(
                xx + 12. * scale,
                yy + 7. * scale,
                label,
                if disabled { [145, 133, 111] } else { INK },
                if i == 2 { fs * 0.70 } else { fs },
                true,
                width - 20. * scale,
            );
        }
    }
    let count = rows.len()
        - usize::from(matches!(
            menu.page,
            Page::Inputs | Page::Gamepad | Page::MouseKeyboard
        ));
    for (visible, &row) in rows[..count]
        .iter()
        .skip(first)
        .take(l.visible_rows)
        .enumerate()
    {
        let sel = first + visible == menu.selected && !menu.atlas.sidebar_focus && menu.input_tabs_focus.is_none();
        let rect =
            clan_atlas::row_rect(menu, first + visible, w, h).expect("visible row rectangle");
        let yy = rect.y as f32;
        let rh = l.line_h as f32 - 6. * scale;
        let (xx, bw) = (rect.x as f32, rect.w as f32);
        c.rect(
            xx,
            yy,
            bw,
            rh,
            5. * scale,
            if sel { [45, 78, 100] } else { LIGHT },
            if sel { [35, 62, 82] } else { [28, 56, 78] },
            if sel { GOLD } else { BRASS },
        );
        let fg = if sel { GOLD } else { INK };
        let ty = yy + (rh - fs) / 2.;
        let tx = xx + 18. * scale;
        if let Row::ToolInput(slot) = row {
            c.text(
                tx,
                yy + 3. * scale,
                menu.tr(slot.label()),
                fg,
                fs * 0.78,
                true,
                bw - 36. * scale,
            );
            let current = menu
                .tool_inputs
                .iter()
                .find(|(key, _)| *key == slot)
                .map(|(_, v)| v.as_str())
                .unwrap_or(menu.tr("Not selected - choose an internal reference"));
            c.text(
                tx,
                yy + 22. * scale,
                current,
                fg,
                fs * 0.60,
                false,
                bw - 36. * scale,
            );
            continue;
        }
        if row == Row::MouseMode {
            c.text(
                tx,
                yy + 3. * scale,
                menu.tr("Enable (Experimental)"),
                fg,
                fs * 0.72,
                true,
                bw - 104. * scale,
            );
            c.text(
                tx,
                yy + 17. * scale,
                menu.tr("Keyboard-only is unavailable"),
                fg,
                fs * 0.50,
                false,
                bw - 104. * scale,
            );
            c.text(
                tx,
                yy + 27. * scale,
                menu.tr("until this mode is disabled."),
                fg,
                fs * 0.50,
                false,
                bw - 104. * scale,
            );
            let on = menu.mouse_mode.get();
            let sx = xx + bw - 68. * scale;
            let sy = yy + 9. * scale;
            c.rect(
                sx,
                sy,
                52. * scale,
                24. * scale,
                12. * scale,
                if on { BRASS } else { [145, 133, 111] },
                if on { GOLD } else { [181, 167, 137] },
                BRASS,
            );
            c.rect(
                sx + if on { 30. * scale } else { 3. * scale },
                sy + 3. * scale,
                18. * scale,
                18. * scale,
                9. * scale,
                LIGHT,
                LIGHT,
                BRASS,
            );
            continue;
        }
        if matches!(row, Row::FastForwardAudio) {
            let (sx, sw) = segment_geometry(l);
            c.text(
                tx,
                if row == Row::FastForwardAudio {
                    ty - 6. * scale
                } else {
                    ty
                },
                row.localized_label(menu.language()),
                fg,
                fs,
                true,
                (sx as f32 - tx - 8. * scale).max(1.),
            );
            if row == Row::FastForwardAudio {
                c.text(
                    tx,
                    ty + 13. * scale,
                    menu.tr("1X: UP TO 2S DELAY"),
                    fg,
                    fs * 0.62,
                    false,
                    (sx as f32 - tx - 8. * scale).max(1.),
                );
            }
            let labels = if row == Row::FastForward {
                ["2X", "4X", "8X"]
            } else {
                ["1X", menu.tr("SILENT"), menu.tr("FAST")]
            };
            for i in 0..3 {
                let active = if row == Row::FastForward {
                    input_config::fast_forward::selected_percent(av.ff_speed) == FF_SPEEDS[i]
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
                    if active { [45, 78, 100] } else { LIGHT },
                    if active { [45, 78, 100] } else { PAPER },
                    if active { GOLD } else { BRASS },
                );
                c.text(
                    ax + 6. * scale,
                    ty,
                    menu.tr(labels[i]),
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
                menu.tr("ENHANCED (RESTART)")
            } else {
                row.localized_label(menu.language())
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
            c.text(
                tx,
                yy + 16. * scale,
                menu.tr("INT: GAME / EXT: PRESENTS"),
                fg,
                fs * 0.55,
                false,
                help_width,
            );
            c.text(
                tx,
                yy + 27. * scale,
                menu.tr("FAST CAN SHOW DIFFERENT RATES"),
                fg,
                fs * 0.52,
                false,
                help_width,
            );
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
                    if sel { [17, 43, 64] } else { [17, 43, 64] },
                    if sel { [17, 43, 64] } else { [17, 43, 64] },
                    if sel { GOLD } else { BRASS },
                );
                c.text(
                    vx + 8. * scale,
                    ty,
                    if matches!(row, Row::Key(_) | Row::Menu | Row::FfKey | Row::FfToggleKey | Row::ZoomIn | Row::ZoomOut) { &v } else { menu.tr(&v) },
                    if sel {
                        [233, 239, 236]
                    } else {
                        [233, 239, 236]
                    },
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
        let track = (l.visible_rows / clan_atlas::columns(menu, w, h) * l.line_h) as f32;
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
    if matches!(
        menu.page,
        Page::Inputs | Page::Gamepad | Page::MouseKeyboard
    ) {
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
            if sel { [45, 78, 100] } else { LIGHT },
            if sel { [35, 62, 82] } else { PAPER },
            if sel { GOLD } else { BRASS },
        );
        c.text(
            xx + 20. * scale,
            yy + 10. * scale,
            menu.tr("BACK"),
            if sel { GOLD } else { INK },
            fs,
            true,
            bw - 30. * scale,
        );
    }
    if menu.page == Page::Main && !menu.session_message.is_empty() {
        c.text(
            x + pad,
            y + ph + 3. * scale,
            menu.tr(&menu.session_message),
            [233, 239, 236],
            12. * scale,
            false,
            pw - pad * 2.,
        );
    }
    c.finish(px);
}

pub(super) fn dropdown_geometry(
    dd: &Dropdown,
    w: usize,
    h: usize,
) -> (usize, usize, usize, usize, usize, usize) {
    let body = clan_atlas::geometry(w, h).body;
    let (w, h) = (body.w, body.h);
    let n = dd.options.len();
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
        body.x + w.saturating_sub(width) / 2,
        body.y + h.saturating_sub(height) / 2,
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
    let i = (y - yy - 6) / (clan_atlas::geometry(w, h).body.h / 15).clamp(14, 40);
    (i < visible).then_some(first + i)
}
pub(super) fn draw_dropdown(menu: &Menu, dd: &Dropdown, c: &mut Canvas, w: usize, h: usize) {
    let (x, y, rw, rh, visible, first) = dropdown_geometry(dd, w, h);
    let pitch = (clan_atlas::geometry(w, h).body.h / 15).clamp(14, 40);
    c.rect(
        x as f32,
        y as f32,
        rw as f32,
        rh as f32,
        6.,
        [28, 56, 78],
        [17, 43, 64],
        GOLD,
    );
    for (i, s) in dd.options
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
            if matches!(dd.row, Row::CatalogRegion | Row::CatalogStatus | Row::CatalogKind | Row::CatalogCase | Row::CatalogEntry) { s } else { menu.tr(s) },
            if sel { GOLD } else { [232, 216, 176] },
            pitch as f32 * 0.6,
            false,
            rw as f32 - 24.,
        );
    }
}
