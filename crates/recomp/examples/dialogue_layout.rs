//! Private snapshot renderer for native UI/world separation; contains no game data.
use gba_core::{
    mem::{self, MemMap},
    ppu,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let opt = |key: &str| args.windows(2).find(|v| v[0] == key).map(|v| v[1].as_str());
    let prefix = args.get(1).ok_or("snapshot prefix required")?;
    let rom = std::fs::read(opt("--rom").ok_or("--rom required")?)?;
    if gba_core::ffta_rom::version(&rom).is_none() {
        return Err("unsupported ROM".into());
    }
    let z: f32 = opt("--zoom").unwrap_or("1").parse()?;
    let base_width:f32=opt("--width").unwrap_or("568").parse()?;
    let ex = ((base_width * z - 240.) / 2.).ceil() as usize;
    let ey = ((160. * z - 160.) / 2.).ceil() as usize;
    mem::set_extra(ex);
    mem::set_extra_y(ey);
    let mut m = MemMap::new(rom);
    for (name, dst) in [
        ("ewram", &mut m.ewram),
        ("iwram", &mut m.iwram),
        ("vram", &mut m.vram),
        ("io", &mut m.io),
        ("oam", &mut m.oam),
        ("palette", &mut m.palette),
    ] {
        dst.copy_from_slice(&std::fs::read(format!("{prefix}-{name}.bin"))?);
    }
    m.ffta_movement.enabled = true;
    m.ffta_dialogue.separate = !args.iter().any(|s| s == "--legacy");
    m.ffta_dialogue.view_zoom = z;
    m.ffta_dialogue.view_width = base_width * z;
    m.ffta_dialogue.view_height = 160. * z;
    m.ffta_dialogue.style.scale = opt("--size").unwrap_or("1").parse()?;
    m.ffta_dialogue.style.length = opt("--length").unwrap_or("1").parse()?;
    for y in 0..160 {
        ppu::render_scanline(&mut m, y);
    }
    ppu::render_ffta_zoom_rows(&mut m);
    if args.iter().any(|s|s=="--verify-notice-mouse") {
        m.ffta_battle_mouse.enabled=true;
        let scene=gba_core::ffta_battle_mouse::MenuScene::read(&m).ok_or("no verified Notice choices")?;
        eprintln!("NOTICE_MOUSE selected={} count={} origin={:?} hits={:?}",scene.selected,scene.count,scene.origin,
            (0..240).filter_map(|x|scene.hit(x as f32,scene.origin[1]as f32+8.).map(|i|(x,i))).collect::<Vec<_>>());
    }
    let (rw, rh) = (mem::render_width(), mem::render_height());
    let view = [
        (rw as f32 - base_width * z) * 0.5,
        (rh as f32 - 160. * z) * 0.5,
        base_width * z,
        160. * z,
    ];
    if args.iter().any(|a| a == "--verify-native") {
        let mut changed = 0;
        for y in 0..160 {
            for x in 0..240 {
                let mut color = m.wide_framebuffer[(y + ey) * rw + x + ex];
                let i = y * gba_core::ffta_dialogue::LAYER_WIDTH
                    + x
                    + gba_core::ffta_dialogue::LAYER_INSET as usize;
                if let Some((pixels, _)) = m.ffta_dialogue.layer() {
                    color = pixels[i].resolve(color);
                }
                if m.ffta_dialogue.extra.active() {
                    let p=m.ffta_dialogue.extra.pixels[i];
                    if p.visible() { let bg=if m.ffta_dialogue.extra.kind==gba_core::ffta_dialogue::PanelKind::Stats {m.ffta_dialogue.extra.background[(y+ey)*rw+x+ex]}else{color}; color=p.resolve(bg); }
                }
                changed += usize::from(color != m.framebuffer[y * 240 + x]);
            }
        }
        println!("NATIVE_UI_RECONSTRUCTION changed_pixels={changed}");
        if changed != 0 {
            return Err("native UI reconstruction mismatch".into());
        }
    }
    let lut = screen::ColorLut::build(&screen::ColorSettings {
        screen: screen::ScreenKind::Raw,
        ..Default::default()
    });
    let (w, h) = ((base_width*3.).round()as usize, 480usize);
    let mut canvas = vec![[0u8; 4]; w * h];
    for y in 0..h {
        for x in 0..w {
            let wx = (view[0] + (x as f32 + 0.5) / w as f32 * view[2]).floor() as usize;
            let wy = (view[1] + (y as f32 + 0.5) / h as f32 * view[3]).floor() as usize;
            canvas[y * w + x] = lut.table[(m.wide_framebuffer[wy * rw + wx] & 0x7fff) as usize];
        }
    }
    for index in 0..2 {
        let state = &m.ffta_dialogue;
        let (pixels, right) = if index == 0 {
            let Some(layer) = state.layer() else {
                continue;
            };
            layer
        } else {
            if !state.separate || !state.extra.active() {
                continue;
            }
            (state.extra.pixels.as_slice(), true)
        };
        let mut packed: Vec<_> = pixels.iter().map(|p| p.packed()).collect();
        if index==1 || state.action_layout {
            let backing=if index==1 {&state.extra.backing}else{&state.backing};
            let b=gba_core::ffta_dialogue::panel_bounds(pixels,backing).unwrap_or([0;4]);
            gba_core::ffta_dialogue::pack_panel(pixels,backing,if index==1 && state.extra.kind==gba_core::ffta_dialogue::PanelKind::Notice {1.}else{state.style.length},b[0]+b[2]>=240,&mut packed);
        }
        if let Some(prefix)=opt("--panel-debug") {
            let backing=if index==1 {&state.extra.backing}else{&state.backing};
            for (name,data) in [("native",pixels),("backing",backing.as_slice())] {
                if data.len()!=480*160 {continue;}
                let mut img=image::RgbaImage::new(480,160);
                for (dst,p) in img.pixels_mut().zip(data) {
                    let c=p.resolve(0x4210);
                    *dst=image::Rgba([((c&31)*255/31)as u8,(((c>>5)&31)*255/31)as u8,(((c>>10)&31)*255/31)as u8,if p.visible(){255}else{0}]);
                }
                img.save(format!("{prefix}-{index}-{name}.png"))?;
            }
        }
        let layout = if index==1 && state.extra.kind==gba_core::ffta_dialogue::PanelKind::Notice {
            screen::ui_layer::Layout::centered_panel((w as u32,h as u32),base_width,state.style.scale,state.extra.bounds.unwrap())
        } else if index == 1 || state.action_layout {
            let backing=if index==1 {&state.extra.backing}else{&state.backing};
            let b=if index==1 && state.extra.kind==gba_core::ffta_dialogue::PanelKind::Stats {
                gba_core::ffta_dialogue::pixel_bounds(pixels)
            }else{gba_core::ffta_dialogue::panel_bounds(pixels,backing)};
            let Some(b)=b else {continue;};
            screen::ui_layer::Layout::at_corner(
                (w as u32, h as u32),
                base_width,
                state.town_layout || b[0] + b[2] >= 240,
                b[1] + b[3] >= 160,
                state.style.scale,
                state.style.length,
                b,
            )
        } else if state.shop_layout || state.site_layout {
            screen::ui_layer::Layout::centered((w as u32,h as u32),base_width,state.style.scale)
        } else {
            screen::ui_layer::Layout::new_with_anchor(
                (w as u32, h as u32),
                base_width,
                right,
                state.style.scale,
                [0., 0.],
                if state.town_layout || args.iter().any(|a| a == "--old-anchor") {
                    0.
                } else {
                    state.vertical_anchor()
                },
            )
        };
        let frame = screen::ui_layer::Frame {
            pixels: &packed,
            pixels_revision: None,
            raw_world: if index==1 && state.extra.kind==gba_core::ffta_dialogue::PanelKind::Stats {&state.extra.background}else{&m.wide_framebuffer},
            world_dims: (rw as u32, rh as u32),
            lut: lut.table.as_slice(),
            layout,
            world_view: view,
        };
        for y in 0..h {
            for x in 0..w {
                if let Some(c) = frame.sample(x as f32 + 0.5, y as f32 + 0.5) {
                    canvas[y * w + x] = c;
                }
            }
        }
        if let Some(path) = opt("--ui-out") {
            let path = if index == 0 {
                path.to_owned()
            } else {
                format!("{path}-extra")
            };
            std::fs::write(path, packed.iter().flatten().copied().collect::<Vec<_>>())?;
        }
        println!(
            "UI_LAYER index={index} pixels={} rect={:?}",
            pixels.iter().filter(|p| p.visible()).count(),
            layout.rect
        );
    }
    let mut png = image::RgbaImage::new(w as u32, h as u32);
    for (out, pixel) in png.pixels_mut().zip(canvas) {
        *out = image::Rgba(pixel);
    }
    let path = opt("--out").ok_or("--out required")?;
    png.save(path)?;
    println!("CAPTURE {path}");
    Ok(())
}
