//! Scanline PPU renderer (hardware reference, "LCD Video Controller").
//!
//! Renders one visible scanline at HBlank into the RGB555 framebuffer:
//! - modes 0-2: text backgrounds (4/8bpp tiles, flips, scrolling) and
//!   affine backgrounds (8bpp, optional wrap, per-line reference stepping);
//! - modes 3-5: bitmap backgrounds (16bpp, 8bpp paged, small paged);
//! - sprites: 1D/2D mapping, 4/8bpp, flips, affine (incl. double-size),
//!   per-pixel priority against backgrounds, semi-transparent mode,
//!   OBJ-window mode;
//! - windows 0/1, OBJ window, outside region;
//! - color effects: alpha blend, brightness up/down, semi-transparent
//!   sprites forcing alpha against second targets;
//! - backdrop (palette entry 0).
//!
//! Mosaic is not yet implemented. Mid-scanline raster effects are out of
//! scope by design (scanline granularity; see docs/architecture.md §6).
use crate::mem::{MAX_RENDER_WIDTH, MemMap, VISIBLE_WIDTH, extra, render_width};

const TRANSPARENT: u16 = 0xFFFF;

/// One line of layer output over the *render* width (the 240 visible
/// columns plus EXTRA columns per side): RGB555 color or TRANSPARENT.
/// Index `i` is screen column `i - EXTRA`.
type Line = [u16; MAX_RENDER_WIDTH];

#[derive(Clone, Copy)]
struct ObjPixel {
    owner: u32,
    color: u16,
    priority: u8,
    semi: bool,
}

const OBJ_EMPTY: ObjPixel = ObjPixel {
    owner: 0,
    color: TRANSPARENT,
    priority: 0xFF,
    semi: false,
};

fn io16(mem: &MemMap, off: usize) -> u16 {
    u16::from_le_bytes([mem.io[off], mem.io[off + 1]])
}

fn pal16(mem: &MemMap, index: usize) -> u16 {
    u16::from_le_bytes([mem.palette[index * 2], mem.palette[index * 2 + 1]]) & 0x7FFF
}

// The battle sky is drawn by changing palette entry 0 on each hardware line.
// Extend that measured gradient into host-only rows instead of freezing its
// first/last color. The 240x160 hardware frame still uses its live palette.
fn extend_sky(top: u16, bottom: u16, line: i32) -> u16 {
    let mut result = 0u16;
    for shift in [0, 5, 10] {
        let a = ((top >> shift) & 31) as i32;
        let b = ((bottom >> shift) & 31) as i32;
        let v = (a + (b - a) * line / 159).clamp(0, 31) as u16;
        result |= v << shift;
    }
    result
}

#[cfg(test)]
mod sky_extension_tests {
    use super::extend_sky;
    #[test]
    fn continues_the_raster_colors_beyond_both_hardware_edges() {
        let top = 0x7d21;
        let bottom = 0x7fcc;
        assert_eq!(extend_sky(top, bottom, 0), top);
        assert_eq!(extend_sky(top, bottom, 159), bottom);
        assert_ne!(extend_sky(top, bottom, -80), top);
        assert_ne!(extend_sky(top, bottom, 239), bottom);
    }
}

/// Screen column (0..VISIBLE_WIDTH) of render-buffer index `i`, or None
/// when `i` falls in the widescreen margins.
fn visible_col(i: usize) -> Option<usize> {
    i.checked_sub(extra()).filter(|&x| x < VISIBLE_WIDTH)
}

/// Window tests use hardware screen coordinates; margin columns inherit
/// the nearest visible edge column.
#[cfg(test)]
fn win_x(i: usize) -> u8 {
    (i as isize - extra() as isize).clamp(0, VISIBLE_WIDTH as isize - 1) as u8
}

/// Store one composed pixel: always into the wide buffer, and into the
/// legacy 240-wide framebuffer when the column is on the visible screen.
#[cfg(test)]
fn put_pixel(mem: &mut MemMap, line: usize, i: usize, color: u16) {
    let rw = render_width();
    let row = if mem.ffta_movement.zoom_output {
        (line as isize + crate::mem::extra_y() as isize) as usize
    } else {
        line
    };
    mem.wide_framebuffer[row * rw + i] = color;
    if !mem.ffta_movement.zoom_output {
        if let Some(x) = visible_col(i) {
            mem.framebuffer[line * VISIBLE_WIDTH + x] = color;
        }
    }
}

/// Complete the host view after the 160 hardware scanlines. No guest writes,
/// no extra video events, and no changes to the existing sprite/terrain hooks.
pub fn ffta_zoom_rows_ready(mem: &MemMap) -> bool {
    crate::mem::extra_y() > 0
        && (mem.ffta_movement.enabled || mem.ffta_stage.enabled)
        && (0..4).any(|bg| ffta_terrain(mem, bg) || crate::ffta_stage::layer(mem, bg).is_some())
}

pub fn render_ffta_zoom_rows(mem: &mut MemMap) {
    let ey = crate::mem::extra_y();
    if !ffta_zoom_rows_ready(mem) {
        return;
    }
    let _profile = crate::ppu_profile::zoom_timer();
    let rw = render_width();
    mem.wide_framebuffer.copy_within(0..160 * rw, ey * rw);
    if mem.ffta_dialogue.extra.kind == crate::ffta_dialogue::PanelKind::Stats {
        mem.ffta_dialogue.extra.background.copy_within(0..160*rw,ey*rw);
    }
    mem.ffta_movement.zoom_output = true;
    mem.ffta_dialogue.wide_pass = true;
    let dispcnt = io16(mem, 0);
    if dispcnt & 0x80 != 0 {
        mem.wide_framebuffer[..rw * crate::mem::render_height()].fill(0x7fff);
        mem.ffta_movement.zoom_output = false;
        mem.ffta_dialogue.wide_pass = false;
        return;
    }
    if parallel_zoom_enabled()
        && ey >= 64
        && !crate::ffta_sprites::trace_enabled(mem.frames)
        && !crate::ffta_sprites::character_metrics()
    {
        render_zoom_parallel(mem, dispcnt, ey, rw);
        mem.ffta_movement.zoom_output = false;
        mem.ffta_dialogue.wide_pass = false;
        return;
    }
    for sy in (-(ey as isize)..0).chain(160..160 + ey as isize) {
        let line = sy as usize;
        let mut backgrounds = [[TRANSPARENT; MAX_RENDER_WIDTH]; 4];
        let mut used = [false; 4];
        for bg in 0..4 {
            // UI text planes belong to this scanline too: omitting them
            // leaves extended OBJ text without its panel background.
            if dispcnt & (0x100 << bg) != 0 {
                render_text_bg(mem, bg, line, &mut backgrounds[bg]);
                used[bg] = true;
            }
        }
        let mut objects = [OBJ_EMPTY; MAX_RENDER_WIDTH];
        let mut window = [false; MAX_RENDER_WIDTH];
        let semi = if dispcnt & 0x1000 != 0 {
            render_sprites(mem, dispcnt, line, &mut objects, &mut window)
        } else {
            false
        };
        compose(
            mem,
            dispcnt,
            line,
            &backgrounds,
            &used,
            &objects,
            &window,
            semi,
        );
    }
    mem.ffta_movement.zoom_output = false;
    mem.ffta_dialogue.wide_pass = false;
}

static PARALLEL_ZOOM: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(cfg!(target_os = "android"));
pub fn set_parallel_zoom(enabled: bool) {
    PARALLEL_ZOOM.store(enabled, std::sync::atomic::Ordering::Relaxed);
}
fn parallel_zoom_enabled() -> bool {
    static MULTICORE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    PARALLEL_ZOOM.load(std::sync::atomic::Ordering::Relaxed)
        && *MULTICORE
            .get_or_init(|| std::thread::available_parallelism().is_ok_and(|n| n.get() >= 2))
}
// The game is frozen at this point in VBlank. Both lanes borrow the same
// immutable memory snapshot and own disjoint output slices. No guest state,
// register, palette, sprite list or ROM is copied or modified by the workers.
fn render_zoom_parallel(mem: &mut MemMap, dispcnt: u16, ey: usize, rw: usize) {
    let mut scratch = std::mem::take(&mut mem.zoom_scratch);
    let stats = mem.ffta_dialogue.extra.kind == crate::ffta_dialogue::PanelKind::Stats;
    scratch.resize(2 * ey * rw * if stats {2}else{1}, 0);
    let (world, background) = scratch.split_at_mut(2*ey*rw);
    let (even, odd) = world.split_at_mut(ey * rw);
    let (bg_even,bg_odd) = if stats {let (a,b)=background.split_at_mut(ey*rw);(Some(a),Some(b))}else{(None,None)};
    #[cfg(feature = "parallel-ppu")]
    {
        static POOL: std::sync::OnceLock<Option<rayon::ThreadPool>> = std::sync::OnceLock::new();
        let pool = POOL.get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(2)
                .thread_name(|index| format!("FFTA-zoom-{index}"))
                .start_handler(|index| {
                    #[cfg(target_os = "android")]
                    if let Some(&cpu) = zoom_fast_cpus().get(index) {
                        pin_zoom_worker(cpu);
                    }
                    #[cfg(not(target_os = "android"))]
                    let _ = index;
                })
                .build()
                .ok()
        });
        let snapshot = &*mem;
        if let Some(pool) = pool {
            pool.install(|| {
                rayon::join(
                    || render_zoom_lane(snapshot, dispcnt, ey, rw, 0, even, bg_even),
                    || render_zoom_lane(snapshot, dispcnt, ey, rw, 1, odd, bg_odd),
                )
            });
        } else {
            render_zoom_lane(snapshot, dispcnt, ey, rw, 0, even, bg_even);
            render_zoom_lane(snapshot, dispcnt, ey, rw, 1, odd, bg_odd);
        }
    }
    #[cfg(not(feature = "parallel-ppu"))]
    std::thread::scope(|scope| {
        let snapshot = &*mem;
        #[cfg(target_os = "android")]
        let worker_cpu = zoom_worker_cpu();
        scope.spawn(move || {
            #[cfg(target_os = "android")]
            if let Some(cpu) = worker_cpu {
                pin_zoom_worker(cpu);
            }
            render_zoom_lane(snapshot, dispcnt, ey, rw, 0, even, bg_even)
        });
        render_zoom_lane(snapshot, dispcnt, ey, rw, 1, odd, bg_odd);
    });
    for row in 0..2 * ey {
        let source = ((row & 1) * ey + row / 2) * rw;
        let destination = if row < ey { row } else { row + 160 };
        mem.wide_framebuffer[destination * rw..(destination + 1) * rw]
            .copy_from_slice(&world[source..source + rw]);
        if stats {mem.ffta_dialogue.extra.background[destination*rw..(destination+1)*rw].copy_from_slice(&background[source..source+rw]);}
    }
    mem.zoom_scratch = scratch;
}
// Newly spawned short tasks can otherwise finish on an efficiency core before
// Android's scheduler has time to migrate them. Use one of the two fastest
// CPUs already allowed for this app; never change global governor settings.
#[cfg(target_os = "android")]
fn zoom_fast_cpus() -> &'static [usize] {
    static FAST: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
    FAST.get_or_init(|| {
        let mut allowed = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        if unsafe { libc::sched_getaffinity(0, std::mem::size_of_val(&allowed), &mut allowed) } != 0
        {
            return Vec::new();
        }
        let mut capacities = Vec::new();
        for cpu in 0..64 {
            if !unsafe { libc::CPU_ISSET(cpu, &allowed) } {
                continue;
            }
            if let Ok(text) =
                std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{cpu}/cpu_capacity"))
            {
                if let Ok(value) = text.trim().parse::<u32>() {
                    capacities.push((value, cpu));
                }
            }
        }
        capacities.sort_unstable_by(|a, b| b.cmp(a));
        if capacities.len() < 2 {
            return Vec::new();
        }
        capacities.into_iter().take(2).map(|(_, cpu)| cpu).collect()
    })
}
#[cfg(all(target_os = "android", not(feature = "parallel-ppu")))]
fn zoom_worker_cpu() -> Option<usize> {
    let current = unsafe { libc::sched_getcpu() };
    zoom_fast_cpus()
        .iter()
        .copied()
        .find(|&cpu| cpu as i32 != current)
}
#[cfg(target_os = "android")]
fn pin_zoom_worker(cpu: usize) {
    let mut mask = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_ZERO(&mut mask);
        libc::CPU_SET(cpu, &mut mask);
        // Failure is harmless: the normal scheduler remains in control.
        libc::sched_setaffinity(0, std::mem::size_of_val(&mask), &mask);
    }
}
fn render_zoom_lane(
    mem: &MemMap,
    dispcnt: u16,
    ey: usize,
    rw: usize,
    lane: usize,
    output: &mut [u16],
    mut raw_background: Option<&mut [u16]>,
) {
    let mut counts = std::collections::BTreeMap::new();
    let mut backgrounds = [[TRANSPARENT; MAX_RENDER_WIDTH]; 4];
    let mut objects = [OBJ_EMPTY; MAX_RENDER_WIDTH];
    let mut window = [false; MAX_RENDER_WIDTH];
    for (index, row) in output.chunks_exact_mut(rw).enumerate() {
        let y = index * 2 + lane;
        let sy = if y < ey {
            y as isize - ey as isize
        } else {
            160 + (y - ey) as isize
        };
        let line = sy as usize;
        let mut used = [false; 4];
        for bg in 0..4 {
            if dispcnt & (0x100 << bg) != 0 {
                backgrounds[bg][..rw].fill(TRANSPARENT);
                render_text_bg(mem, bg, line, &mut backgrounds[bg]);
                used[bg] = true;
            }
        }
        objects[..rw].fill(OBJ_EMPTY);
        window[..rw].fill(false);
        let semi = if dispcnt & 0x1000 != 0 {
            render_sprites(mem, dispcnt, line, &mut objects, &mut window)
        } else {
            false
        };
        compose_into(
            mem,
            dispcnt,
            line,
            &backgrounds,
            &used,
            &objects,
            &window,
            semi,
            row,
            &mut counts,
        );
        if let Some(bg) = raw_background.as_deref_mut() {
            compose_background_raw(mem,dispcnt,line,&backgrounds,&used,&window,&mut bg[index*rw..(index+1)*rw]);
        }
    }
}

pub fn render_scanline(mem: &mut MemMap, line: usize) {
    let _profile = crate::ppu_profile::scanline_timer();
    use crate::ppu_profile::{scanline_stage_timer, ScanlineStage};
    if line == 0 {
        let _setup = scanline_stage_timer(ScanlineStage::UiSetup);
        begin_dialogue_frame(mem);
        mem.ffta_movement.sky_edges[0] = pal16(mem, 0);
    }
    if line == 159 {
        mem.ffta_movement.sky_edges[1] = pal16(mem, 0);
    }
    if crate::ffta_log::enabled() && mem.frames == 13100 && matches!(line, 0 | 40 | 80 | 120 | 159)
    {
        eprintln!(
            "FFTA_SKY frame={} line={} palette0={:04x} bldcnt={:04x} bldy={:04x} dispcnt={:04x}",
            mem.frames,
            line,
            pal16(mem, 0),
            io16(mem, 0x50),
            io16(mem, 0x54),
            io16(mem, 0)
        );
    }
    if line == 0 && (crate::ffta_log::enabled() || crate::ffta_sprites::character_metrics()) {
        mem.ffta_sprites.pixel_counts.clear();
    }
    let dispcnt = io16(mem, 0x00);

    if dispcnt & 0x80 != 0 {
        // Forced blank: white.
        let row = &mut mem.framebuffer[line * VISIBLE_WIDTH..(line + 1) * VISIBLE_WIDTH];
        row.fill(0x7FFF);
        let rw = render_width();
        let wide = &mut mem.wide_framebuffer[line * rw..(line + 1) * rw];
        wide.fill(0x7FFF);
        return;
    }

    // Latch / step the affine reference accumulators.
    if line == 0 || mem.aff_dirty[0] {
        mem.aff_ref[0] = sign_extend28(read_io32(mem, 0x28));
        mem.aff_ref[1] = sign_extend28(read_io32(mem, 0x2C));
        mem.aff_dirty[0] = false;
    }
    if line == 0 || mem.aff_dirty[1] {
        mem.aff_ref[2] = sign_extend28(read_io32(mem, 0x38));
        mem.aff_ref[3] = sign_extend28(read_io32(mem, 0x3C));
        mem.aff_dirty[1] = false;
    }

    let mode = dispcnt & 7;
    let mut bg_lines = [[TRANSPARENT; MAX_RENDER_WIDTH]; 4];
    let mut bg_used = [false; 4];

    let bg_timer = scanline_stage_timer(ScanlineStage::Backgrounds);
    match mode {
        0 => {
            for bg in 0..4 {
                if dispcnt & (0x100 << bg) != 0 {
                    render_text_bg(mem, bg, line, &mut bg_lines[bg]);
                    bg_used[bg] = true;
                }
            }
        }
        1 => {
            for bg in 0..2 {
                if dispcnt & (0x100 << bg) != 0 {
                    render_text_bg(mem, bg, line, &mut bg_lines[bg]);
                    bg_used[bg] = true;
                }
            }
            if dispcnt & 0x400 != 0 {
                render_affine_bg(mem, 2, &mut bg_lines[2]);
                bg_used[2] = true;
            }
        }
        2 => {
            for bg in 2..4 {
                if dispcnt & (0x100 << bg) != 0 {
                    render_affine_bg(mem, bg, &mut bg_lines[bg]);
                    bg_used[bg] = true;
                }
            }
        }
        3 | 4 | 5 => {
            if dispcnt & 0x400 != 0 {
                render_bitmap_bg(mem, mode, dispcnt, line, &mut bg_lines[2]);
                bg_used[2] = true;
            }
        }
        _ => {}
    }

    drop(bg_timer);

    // Sprites.
    let mut obj_line = [OBJ_EMPTY; MAX_RENDER_WIDTH];
    let mut obj_window = [false; MAX_RENDER_WIDTH];
    let mut line_has_semi = false;
    let obj_timer = scanline_stage_timer(ScanlineStage::Objects);
    if dispcnt & 0x1000 != 0 {
        line_has_semi = render_sprites(mem, dispcnt, line, &mut obj_line, &mut obj_window);
    }

    drop(obj_timer);

    // Step affine accumulators for the next line (PB/PD).
    mem.aff_ref[0] = mem.aff_ref[0].wrapping_add(io16(mem, 0x22) as i16 as i32);
    mem.aff_ref[1] = mem.aff_ref[1].wrapping_add(io16(mem, 0x26) as i16 as i32);
    mem.aff_ref[2] = mem.aff_ref[2].wrapping_add(io16(mem, 0x32) as i16 as i32);
    mem.aff_ref[3] = mem.aff_ref[3].wrapping_add(io16(mem, 0x36) as i16 as i32);

    let composition_timer = scanline_stage_timer(ScanlineStage::Composition);
    compose(
        mem,
        dispcnt,
        line,
        &bg_lines,
        &bg_used,
        &obj_line,
        &obj_window,
        line_has_semi,
    );
    drop(composition_timer);
    let ui_timer = scanline_stage_timer(ScanlineStage::UiCapture);
    if mem.ffta_dialogue.separate {
        let primary=mem.ffta_dialogue.placement.is_some();
        let extra=mem.ffta_dialogue.extra.active();
        if primary || extra {
            let needs_bg=primary || (extra && mem.ffta_dialogue.extra.kind!=crate::ffta_dialogue::PanelKind::Stats);
            let rows=UiBgRow::new(mem,line,&bg_used,needs_bg);
            if primary {capture_dialogue_row(mem,dispcnt,line,&bg_lines,&bg_used,&obj_line,&obj_window,false,&rows);}
            if extra {capture_dialogue_row(mem,dispcnt,line,&bg_lines,&bg_used,&obj_line,&obj_window,true,&rows);}
        }
    }
    drop(ui_timer);
    if mem.ffta_dialogue.placement.is_some() || mem.ffta_dialogue.extra.active() {
        // Keep the hardware framebuffer untouched. Only the host wide view
        // receives relocated UI; reuse the world BG rows already rendered.
        mem.ffta_dialogue.wide_pass = true;
        let bg_timer = scanline_stage_timer(ScanlineStage::Backgrounds);
        for bg in 2..4 {
            if bg_used[bg] {
                render_text_bg(mem, bg, line, &mut bg_lines[bg]);
            }
        }
        drop(bg_timer);
        let obj_timer = scanline_stage_timer(ScanlineStage::Objects);
        obj_line.fill(OBJ_EMPTY);
        obj_window.fill(false);
        let semi = if dispcnt & 0x1000 != 0 {
            render_sprites(mem, dispcnt, line, &mut obj_line, &mut obj_window)
        } else {false};
        drop(obj_timer);
        let _composition = scanline_stage_timer(ScanlineStage::Composition);
        compose(mem, dispcnt, line, &bg_lines, &bg_used, &obj_line, &obj_window, semi);
        mem.ffta_dialogue.wide_pass = false;
    }
}

/// Wide opaque warning window, identified by WIN0 and its four frame corners.
/// Text/locale is deliberately irrelevant; narrow Yes/No panels do not qualify.
fn action_menu_bounds(mem:&MemMap)->Option<[i32;4]> {
    // The menu's native width follows its longest translated option. Match
    // its existing BG2 header/footer, not a fixed X or the character's name.
    let base=((io16(mem,12)>>8)&31)as usize*0x800;
    let tile=|x:usize,y:usize|->Option<u16>{
        let o=base+(y*32+x)*2;
        Some(u16::from_le_bytes([*mem.vram.get(o)?,*mem.vram.get(o+1)?]))
    };
    let mut found=None;
    for y in 0..19 {for x in 8..29 {
        if tile(x,y)!=Some(0xc005) || !(x+1..30).all(|xx|tile(xx,y)==Some(0xc006)){continue;}
        for bottom in y+2..20 {
            if tile(x,bottom)!=Some(0xc009) || !(x+1..30).all(|xx|tile(xx,bottom)==Some(0xc00a)){continue;}
            if found.is_some(){return None;}
            found=Some([(x*8)as i32,(y*8)as i32,240,((bottom+1)*8)as i32]);
        }
    }}
    found
}
fn action_banner_bounds(mem:&MemMap)->Option<[i32;4]> {
    // Native short spell/help strip, independent of its translated text.
    if io16(mem,0)&0x6000!=0x2000 || io16(mem,0x48)&63!=31{return None;}
    let h=io16(mem,0x40);let v=io16(mem,0x44);
    let [l,t,r,b]=[(h>>8)as usize,(v>>8)as usize,(h&255)as usize,(v&255)as usize];
    if l>=r || r>240 || b>160 || b.checked_sub(t)!=Some(16) || r-l<160
        || [l,t,r,b].iter().any(|x|x%8!=0){return None;}
    let base=((io16(mem,14)>>8)&31)as usize*0x800;
    let entry=|x,y|{let o=base+((y/8)*32+x/8)*2;u16::from_le_bytes([mem.vram[o],mem.vram[o+1]])};
    if entry(l,t)!=0xb032 || entry(r-8,t)!=0xb432
        || entry(l,b-8)!=0xb832 || entry(r-8,b-8)!=0xbc32{return None;}
    if !(l+8..r-8).step_by(8).all(|x|entry(x,t)==0xb033 && entry(x,b-8)==0xb833){return None;}
    Some([l as i32,t as i32,r as i32,b as i32])
}
// The laws table shares the action BG registers but is one centered modal,
// not the left HUD/right action list. Match its frame, never translated text.
fn laws_bounds(mem: &MemMap) -> Option<[i32; 4]> {
    if io16(mem, 0) & 0xe000 != 0 { return None; }
    let base = ((io16(mem, 14) >> 8) & 31) as usize * 0x800;
    let tile = |x: usize, y: usize| {
        let off = base + (y * 32 + x) * 2;
        u16::from_le_bytes([mem.vram[off], mem.vram[off + 1]])
    };
    if tile(1, 9) != 0x70e1 || tile(26, 9) != 0x74e1
        || tile(1, 15) != 0x70ee || tile(26, 15) != 0x74ee
        || !(2..26).all(|x| tile(x, 9) == 0x70e2 && tile(x, 15) == 0x70ef)
        || ![11, 13].into_iter().all(|y| {
            tile(1, y) == 0x70f3 && tile(13, y) == 0x70fe && tile(25, y) == 0x80f5
                && tile(1, y + 1) == 0x70f6 && tile(13, y + 1) == 0x70ff && tile(25, y + 1) == 0x80f8
        }) { return None; }
    // Include the arrow's one-tile left overhang. Select opens HELP above the
    // table without enabling WIN0, unlike the ordinary Notice recognizer.
    let help = [(2, 2, 0xb00d), (27, 2, 0xb00e), (2, 7, 0xb00f), (27, 7, 0xb010)]
        .into_iter().all(|(x, y, value)| tile(x, y) == value);
    Some(if help { [0, 16, 224, 128] } else { [0, 72, 216, 128] })
}
fn notice_bounds(mem:&MemMap)->Option<[i32;4]> {
    if io16(mem,0)&0x6000!=0x2000 || io16(mem,0x48)&63!=31 || io16(mem,0x4a)&32==0 {return None;}
    let h=io16(mem,0x40);let v=io16(mem,0x44);
    let [l,t,r,b]=[(h>>8)as usize,(v>>8)as usize,(h&255)as usize,(v&255)as usize];
    if l>=r || t>=b || r>240 || b>160 || r-l<160 || b-t<16 || b-t>128
        || [l,t,r,b].iter().any(|v|v%8!=0) {return None;}
    let base=((io16(mem,0xe)>>8)&31)as usize*0x800;
    for (x,y,tile) in [(l,t,0xb00du16),(r-8,t,0xb00e),(l,b-8,0xb00f),(r-8,b-8,0xb010)] {
        let off=base+((y/8)*32+x/8)*2;
        if u16::from_le_bytes([*mem.vram.get(off)?,*mem.vram.get(off+1)?])!=tile {return None;}
    }
    Some([l as i32,t as i32,r as i32,b as i32])
}
fn begin_dialogue_frame(mem: &mut MemMap) {
    mem.ffta_dialogue.wide_pass = false;
    mem.ffta_dialogue.placement = None;
    mem.ffta_dialogue.anchor_bounds = None;
    mem.ffta_dialogue.action_layout=false;
    mem.ffta_dialogue.action_banner=None;
    mem.ffta_dialogue.town_layout=false;
    mem.ffta_dialogue.shop_layout=false;
    mem.ffta_dialogue.site_layout=false;
    mem.ffta_dialogue.laws_layout=false;
    mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::None;
    mem.ffta_dialogue.extra.bounds=None;
    let site_menu=if mem.ffta_dialogue.separate {
        crate::ffta_battle_mouse::MenuScene::site_menu(mem)
    }else{None};
    let terrain=(0..4).any(|bg|ffta_terrain(mem,bg));
    if (extra()==0 && crate::mem::extra_y()==0) || !crate::ffta_dialogue::enabled()
        || (!mem.ffta_movement.enabled && site_menu.is_none()) || mem.ffta_bg_edge==FftaBgEdge::Wrap
        || io16(mem,0)&0x87!=0 || (!terrain && site_menu.is_none()) {return;}
    if mem.ffta_dialogue.separate {
        let mut bounds=[240,160,0,0];let mut pieces=[0;3];
        for obj in mem.oam.chunks_exact(8) {
            let a=u16::from_le_bytes([obj[0],obj[1]]);let b=u16::from_le_bytes([obj[2],obj[3]]);let c=u16::from_le_bytes([obj[4],obj[5]]);
            if a&0x3f00!=0x0400 || c&0xfc00!=0xb000 {continue;}
            let index=match c&1023 {0x1f0=>0,0x200=>1,0x208=>2,_=>continue};
            let shape=(a>>14) as usize;if shape>=3 {continue;}
            let (w,h)=OBJ_SIZES[shape][(b>>14) as usize];let x=(b&511) as i32;let y=(a&255) as i32;
            if x>=240 || y>=160 {continue;}
            pieces[index]+=1;bounds=[bounds[0].min(x),bounds[1].min(y),bounds[2].max(x+w),bounds[3].max(y+h)];
        }
        if pieces==[1,1,2] {
            mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Stats;
            mem.ffta_dialogue.extra.background.resize(render_width()*crate::mem::render_height(),0);
            mem.ffta_dialogue.extra.bounds=Some(bounds);
            mem.ffta_dialogue.extra.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
            mem.ffta_dialogue.extra.pixels.fill(Default::default());
        }
    }
    let pub_scene=if mem.ffta_dialogue.separate {crate::ffta_battle_mouse::MenuScene::pub_menu(mem)}else{None};
    let shop_root=if mem.ffta_dialogue.separate {crate::ffta_battle_mouse::MenuScene::shop_menu(mem)}else{None};
    let shop=mem.ffta_dialogue.separate && crate::ffta_shop_mouse::active(mem);
    let usa_shop_root=!mem.ffta_movement.europe_profile && shop_root.is_some();
    let site=usa_shop_root || (site_menu.is_some() && pub_scene.is_none() && shop_root.is_none() && !shop);
    let town_scene=pub_scene.or(shop_root).or(if site {site_menu}else{None});
    let town=town_scene.is_some();
    mem.ffta_dialogue.shop_layout=shop;
    mem.ffta_dialogue.town_layout=town;
    mem.ffta_dialogue.site_layout=site;
    if town || shop {
        // Town planes are captured as complete pixels, not action backing.
        // Discard any backing retained from the preceding battle panel.
        mem.ffta_dialogue.backing.clear();
        mem.ffta_dialogue.extra.backing.clear();
    }
    let dialogue=io16(mem,0)&0xc00==0xc00 && io16(mem,0xc)==0xb08 && io16(mem,0xe)==0xa08 && ffta_terrain(mem,0);
    let action=mem.ffta_dialogue.separate && io16(mem,0)&0xc00==0xc00
        && io16(mem,0xc)==0x5c08 && io16(mem,0xe)==0x4a09
        && io16(mem,0x18)&511==0 && io16(mem,0x1a)&511==0 && ffta_terrain(mem,0);
    if !dialogue && !action && !town && !shop {
        return;
    }
    mem.ffta_dialogue.action_layout=action;
    if !mem.ffta_movement.europe_profile {
        if let Some(menu)=pub_scene {
            mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Actions;
            mem.ffta_dialogue.extra.bounds=Some([menu.origin[0]-8,menu.origin[1]-16,menu.right-8,
                (menu.origin[1]+menu.count as i32*16+4).min(160)]);
            mem.ffta_dialogue.extra.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
            mem.ffta_dialogue.extra.pixels.fill(Default::default());
        }
    }
    if action {mem.ffta_dialogue.action_banner=action_banner_bounds(mem);}
    if action {
        let laws = laws_bounds(mem);
        mem.ffta_dialogue.laws_layout = laws.is_some();
        if let Some(bounds)=laws.or_else(||notice_bounds(mem)) {
            mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Notice;
            mem.ffta_dialogue.extra.bounds=Some(bounds);
        }
    }
    // Determine the attachment from the actual backing's opaque edge pixels,
    // not the text contents, language or portrait identity. No guessed side.
    let mut edge = [0usize;2];
    let mut row = [TRANSPARENT;MAX_RENDER_WIDTH];
    for y in 0..160 {
        render_text_bg_hardware(mem,3,y,&mut row,0,240);
        edge[0] += usize::from(row[0] != TRANSPARENT);
        edge[1] += usize::from(row[239] != TRANSPARENT);
    }
    if !town && !shop && mem.ffta_dialogue.extra.kind!=crate::ffta_dialogue::PanelKind::Notice
        && ((edge[0] == edge[1] && !action) || (action && edge[0]+edge[1]==0)) { return; }
    let mut placement = crate::ffta_dialogue::Placement::new(
        town || shop || (!action && edge[1] > edge[0]), render_width(), crate::mem::extra_y(), mem.ffta_dialogue.style,
    );
    let state = &mem.ffta_dialogue;
    if state.view_zoom.is_finite() && state.view_zoom >= 1. {
        placement.scale = state.view_zoom * state.style.scale.clamp(0.5, 2.);
        let inset_x = (render_width() as f32 - state.view_width) * 0.5;
        placement.left = if placement.right {
            inset_x + state.view_width - 240. * placement.scale
        } else { inset_x } + state.style.offset_x * state.view_zoom;
        placement.top = ((160 + crate::mem::extra_y()*2) as f32-state.view_height)*0.5
            + state.style.offset_y * state.view_zoom;
    }
    if action || town || shop {placement.length_pixels=0;}
    if action {
        placement.length_pixels=0;
        // Target forecast is an OBJ panel beside the BG attacker information.
        // Keep it when present instead of replacing it with an empty action list.
        if !matches!(mem.ffta_dialogue.extra.kind,crate::ffta_dialogue::PanelKind::Stats|crate::ffta_dialogue::PanelKind::Notice) {
            mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Actions;
            mem.ffta_dialogue.extra.bounds=Some(action_menu_bounds(mem).unwrap_or([160,0,240,160]));
        }
    }
    if action {
        mem.ffta_dialogue.extra.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
        mem.ffta_dialogue.extra.pixels.fill(Default::default());
    }
    mem.ffta_dialogue.placement = Some(placement);
    if mem.ffta_dialogue.separate {
        mem.ffta_dialogue.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
        mem.ffta_dialogue.pixels.fill(Default::default());
        // Portrait rectangles and backing rows define a stable anchor.
        // The continuation arrow is deliberately excluded from these bounds.
        for obj in mem.oam.chunks_exact(8) {
            let attrs=std::array::from_fn(|i|u16::from_le_bytes([obj[2*i],obj[2*i+1]]));
            let x=(attrs[1]&511) as i32;
            if !crate::ffta_dialogue::portrait(attrs,x) {continue;}
            let y=attrs[0]&255;let shape=(attrs[0]>>14) as usize;
            if shape>=3 || y>=160 {continue;}
            let h=OBJ_SIZES[shape][(attrs[1]>>14) as usize].1 as usize;
            mem.ffta_dialogue.include_anchor_bounds(y as usize,(y as usize+h).min(160));
        }
    }
}

/// Immutable native planes shared only within a scanline, never across guest writes.
struct UiBgRow { text: Line, backing: Line }
impl UiBgRow {
    fn new(mem:&MemMap,line:usize,used:&[bool;4],enabled:bool)->Self {
        let mut rows=Self {text:[TRANSPARENT;MAX_RENDER_WIDTH],backing:[TRANSPARENT;MAX_RENDER_WIDTH]};
        if enabled && used[2] {render_text_bg_hardware(mem,2,line,&mut rows.text,0,240);}
        if enabled && used[3] {render_text_bg_hardware(mem,3,line,&mut rows.backing,0,240);}
        rows
    }
}

/// Extract native UI plus GBA blend coefficients. Guest buffers are never modified.
fn capture_dialogue_row(mem:&mut MemMap,dispcnt:u16,line:usize,
    world_bg:&[Line;4],used:&[bool;4],world_obj:&[ObjPixel;MAX_RENDER_WIDTH],
    world_window:&[bool;MAX_RENDER_WIDTH],extra_panel:bool,rows:&UiBgRow) {
    capture_dialogue_part::<true,true>(mem,dispcnt,line,world_bg,used,world_obj,world_window,extra_panel,false,rows);
    if mem.ffta_dialogue.action_layout || (extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats) {
        capture_dialogue_part::<true,true>(mem,dispcnt,line,world_bg,used,world_obj,world_window,extra_panel,true,rows);
    }
}
/// Window registers and vertical membership do not change during UI extraction.
/// Keep WIN0 before WIN1, then OBJ window, then outside, as in hardware compose.
struct UiWindowRow {
    any: bool,
    obj: bool,
    win0: (bool, u16),
    win1: (bool, u16),
    winin: u16,
    winout: u16,
}
impl UiWindowRow {
    fn new(mem: &MemMap, dispcnt: u16, line: usize) -> Self {
        let window = |bit, h, v| {
            let vertical = io16(mem, v);
            (dispcnt & bit != 0 && in_window_range((vertical >> 8) as u8, vertical as u8, line as u8), io16(mem, h))
        };
        Self {
            any: dispcnt & 0xe000 != 0, obj: dispcnt & 0x8000 != 0,
            win0: window(0x2000, 0x40, 0x44), win1: window(0x4000, 0x42, 0x46),
            winin: io16(mem, 0x48), winout: io16(mem, 0x4a),
        }
    }
    fn control(&self, x: u8, object_window: bool) -> u16 {
        if !self.any { 0x3f }
        else if self.win0.0 && in_window_range((self.win0.1 >> 8) as u8, self.win0.1 as u8, x) { self.winin & 63 }
        else if self.win1.0 && in_window_range((self.win1.1 >> 8) as u8, self.win1.1 as u8, x) { (self.winin >> 8) & 63 }
        else if self.obj && object_window { (self.winout >> 8) & 63 }
        else { self.winout & 63 }
    }
}

// Per-call snapshot only: no invalidation is needed across scanlines or restores.
enum UiPanelArea { All, Split, Bounds(Option<(i32,i32,bool)>) }
struct UiPanelRow { banner:Option<(i32,i32)>, area:UiPanelArea, extra:bool }
impl UiPanelRow {
    fn new(mem:&MemMap,line:usize,extra:bool)->Self {
        use crate::ffta_dialogue::PanelKind;
        let ui=&mem.ffta_dialogue;let y=line as i32;
        let banner=ui.action_banner.filter(|&[_,t,_,b]|y>=t && y<b).map(|[l,_,r,_]|(l,r));
        let area=if (ui.action_layout || ui.town_layout) && matches!(ui.extra.kind,PanelKind::Notice|PanelKind::Actions) {
            UiPanelArea::Bounds(ui.extra.bounds.map(|[l,t,r,b]|(l,r,y>=t && y<b)))
        }else if ui.action_layout {UiPanelArea::Split}else{UiPanelArea::All};
        Self {banner,area,extra}
    }
    #[inline]
    fn contains(&self,x:i32)->bool {
        if self.banner.is_some_and(|(l,r)|x>=l && x<r) {return !self.extra;}
        match self.area {
            UiPanelArea::All=>true,
            UiPanelArea::Split=>if self.extra {x>=160}else{x<160},
            UiPanelArea::Bounds(bounds)=>{
                // Keep the old lazy failure: only an eligible BG outside the
                // overriding banner needs bounds, never merely constructing a row.
                let (l,r,on_row)=bounds.unwrap();
                (on_row && x>=l && x<r)==self.extra
            }
        }
    }
}
// Original policy for the uncached reference specialization in tests.
fn ui_panel_contains_uncached(mem:&MemMap,line:usize,x:i32,extra_panel:bool)->bool {
    let banner=mem.ffta_dialogue.action_banner.is_some_and(|[l,t,r,b]|x>=l && x<r && line as i32>=t && (line as i32)<b);
    if banner {!extra_panel}
    else if (mem.ffta_dialogue.action_layout || mem.ffta_dialogue.town_layout) && matches!(mem.ffta_dialogue.extra.kind,crate::ffta_dialogue::PanelKind::Notice|crate::ffta_dialogue::PanelKind::Actions) {
        let [l,t,r,b]=mem.ffta_dialogue.extra.bounds.unwrap();
        let inside=x>=l && x<r && line as i32>=t && (line as i32)<b;
        inside==extra_panel
    }else{!mem.ffta_dialogue.action_layout || (if extra_panel {x>=160}else{x<160})}
}

fn capture_dialogue_part<const SKIP_EMPTY:bool,const CACHE_PANEL:bool>(mem:&mut MemMap, dispcnt:u16, line:usize,
    world_bg:&[Line;4], used:&[bool;4], world_obj:&[ObjPixel;MAX_RENDER_WIDTH],
    world_window:&[bool;MAX_RENDER_WIDTH], extra_panel:bool,background_only:bool,rows:&UiBgRow) {
    use crate::ffta_dialogue::{UiPixel,LAYER_WIDTH,LAYER_INSET};
    if background_only {
        let dst=if extra_panel {&mut mem.ffta_dialogue.extra.backing}else{&mut mem.ffta_dialogue.backing};
        dst.resize(LAYER_WIDTH*160,Default::default());
    }
    let ui_mask=if extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats {0}else{0xc};
    let text=if ui_mask!=0 {&rows.text}else{&[TRANSPARENT;MAX_RENDER_WIDTH]};
    let backing=if ui_mask!=0 {&rows.backing}else{&[TRANSPARENT;MAX_RENDER_WIDTH]};
    let mut filtered_text;
    let text=if background_only && mem.ffta_dialogue.action_layout {
        filtered_text=*text;
        // Verified action-plane border tiles: header/footer and the info strip.
        // Keep dynamic action labels and portrait pixels out of the backing.
        let base=((io16(mem,0x0c)>>8)&31) as usize*0x800;
        for x in 0..240 {
            let off=base+((line/8)*32+x/8)*2;
            let tile=u16::from_le_bytes([mem.vram[off],mem.vram[off+1]])&1023;
            if !matches!(tile,0x005..=0x00a | 0x0a8..=0x0b9) {filtered_text[x]=TRANSPARENT;}
        }
        &filtered_text
    }else{text};
    if backing[..240].iter().any(|&c|c!=TRANSPARENT) {
        mem.ffta_dialogue.include_anchor_bounds(line,line+1);
    }
    let mut portraits=[OBJ_EMPTY;MAX_RENDER_WIDTH];
    let mut window=[false;MAX_RENDER_WIDTH];
    mem.ffta_dialogue.capture=true;
    mem.ffta_dialogue.capture_extra=extra_panel;
    mem.ffta_dialogue.capture_backing=background_only;
    if dispcnt&0x1000!=0 {
        render_sprites_extent(mem,dispcnt,line,&mut portraits,&mut window,LAYER_INSET as usize,LAYER_WIDTH);
    }
    mem.ffta_dialogue.capture_backing=false;
    mem.ffta_dialogue.capture=false;
    mem.ffta_dialogue.capture_extra=false;
    // World-only candidates never enter the UI texture. Still clear this row:
    // a loaded/previous frame may have left visible pixels in the destination.
    if SKIP_EMPTY && text[..240].iter().all(|&c|c==TRANSPARENT)
        && backing[..240].iter().all(|&c|c==TRANSPARENT)
        && portraits[..LAYER_WIDTH].iter().all(|obj|obj.priority==255) {
        let dst=if background_only {
            if extra_panel {&mut mem.ffta_dialogue.extra.backing}else{&mut mem.ffta_dialogue.backing}
        }else if extra_panel {&mut mem.ffta_dialogue.extra.pixels}else{&mut mem.ffta_dialogue.pixels};
        dst[line*LAYER_WIDTH..(line+1)*LAYER_WIDTH].fill(UiPixel::default());
        return;
    }
    let placement=mem.ffta_dialogue.placement.unwrap_or_else(||crate::ffta_dialogue::Placement::new(true,render_width(),0,Default::default()));
    let bld=io16(mem,0x50);
    let alpha=io16(mem,0x52);
    let eva=(alpha&31).min(16) as u8;
    let evb=((alpha>>8)&31).min(16) as u8;
    let evy=(io16(mem,0x54)&31).min(16) as u32;
    let priorities:[u8;4]=std::array::from_fn(|bg|(io16(mem,8+2*bg)&3) as u8);
    let windows=UiWindowRow::new(mem,dispcnt,line);
    let native_origin=extra();
    let modal=extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice;
    let panel=UiPanelRow::new(mem,line,extra_panel);
    for cx in 0..LAYER_WIDTH {
        let x=cx as i32-LAYER_INSET;
        let wx=x.clamp(0,239) as u8;
        let native=native_origin+wx as usize;
        let control=windows.control(wx,world_window[native]);
        let mut candidates=[(0u16,5usize,255u8,false);5];let mut n=0;
        let mut in_panel=None;
        let mut has_ui=false;
        for bg in 0..4 {
            if !used[bg] || control&(1<<bg)==0 {continue;}
            let color=match bg {
                2 if ui_mask!=0=>if (0..240).contains(&x) {text[x as usize]}else{TRANSPARENT},
                3 if ui_mask!=0=>{
                    let mut sx=x+if placement.right {placement.length_pixels}else{-placement.length_pixels};
                    if !mem.ffta_dialogue.town_layout && !mem.ffta_dialogue.shop_layout
                        && mem.ffta_dialogue.extra.kind!=crate::ffta_dialogue::PanelKind::Notice {
                        if placement.right && sx>=200 && sx<240+placement.length_pixels {sx=200;}
                        if !placement.right && sx<40 && sx>=-placement.length_pixels {sx=39;}
                    }
                    if (0..240).contains(&sx) {backing[sx as usize]}else{TRANSPARENT}
                }
                _=>world_bg[bg][native],
            };
            let in_panel=if CACHE_PANEL {
                *in_panel.get_or_insert_with(||panel.contains(x))
            }else{ui_panel_contains_uncached(mem,line,x,extra_panel)};
            let color=if ui_mask&(1<<bg)!=0 && !in_panel {TRANSPARENT}else{color};
            if color!=TRANSPARENT {
                candidates[n]=(color,bg,priorities[bg],false);n+=1;
                has_ui|=ui_mask&(1<<bg)!=0;
            }
        }
        let obj=portraits[cx];
        if obj.priority!=255 && control&16!=0 {
            candidates[n]=(obj.color,4,obj.priority,obj.semi);n+=1;has_ui=true;
        }
        // World-only candidates cannot enter this texture. Keep the destination
        // write to clear stale pixels, but no sorting or blending is needed.
        let pixel=if SKIP_EMPTY && !has_ui {UiPixel::default()}else {
            // Real world objects may be the second blend target, but never enter the UI texture.
            let wo=world_obj[native];
            // Notice is relocated independently of the portrait below it. Do not
            // bake a hole from an OBJ which overlaps only at the old native position.
            let world_object=!modal && !background_only && (0..240).contains(&x) && wo.priority!=255 && control&16!=0 && obj.priority==255;
            candidates[..n].sort_unstable_by_key(|&(_,id,priority,_)|(priority,if id==4 {0}else{id+1}));
            let mut top=[(0u16,5usize,255u8,false);3];
            let mut nt=n.min(2);top[..nt].copy_from_slice(&candidates[..nt]);
            if world_object {
                top[nt]=(wo.color,6,wo.priority,wo.semi);nt+=1;
                top[..nt].sort_unstable_by_key(|&(_,id,priority,_)|(priority,if id>=4 {0}else{id+1}));
            }
            if let Some(&(fc,fl,_,semi))=top[..nt].first() {
                if (fl<4 && ui_mask&(1<<fl)==0) || fl>=5 {UiPixel::default()} else {
                    let (sc,sl,_,_)=top[..nt].get(1).copied().unwrap_or((0,5,255,false));
                    let sl=if sl==6 {4}else{sl};
                    let mode=(bld>>6)&3;
                    let effect=control&32!=0 && bld&(1<<fl)!=0;
                    let blend=(semi || (effect && mode==1)) && bld&(0x100<<sl)!=0;
                    if blend && ((sl<4 && ui_mask&(1<<sl)==0) || sl==5 || top[..nt].get(1).is_some_and(|v|v.1==6)) {
                        UiPixel {color:fc,eva,evb}
                    } else if blend { UiPixel::opaque(alpha_blend(fc,sc,eva as u32,evb as u32)) }
                    else if effect && mode==2 {UiPixel::opaque(brighten(fc,evy))}
                    else if effect && mode==3 {UiPixel::opaque(darken(fc,evy))}
                    else {UiPixel::opaque(fc)}
                }
            } else {UiPixel::default()}
        };
        if background_only {
            let dst=if extra_panel {&mut mem.ffta_dialogue.extra.backing}else{&mut mem.ffta_dialogue.backing};
            dst[line*LAYER_WIDTH+cx]=pixel;
        } else if extra_panel {mem.ffta_dialogue.extra.pixels[line*LAYER_WIDTH+cx]=pixel;}
        else {mem.ffta_dialogue.pixels[line*LAYER_WIDTH+cx]=pixel;}
    }
}

#[cfg(test)]
mod ui_window_row_tests {
    use super::*;

    #[test]
    fn panel_row_rules_match_uncached_banner_bounds_and_split_policy() {
        use crate::ffta_dialogue::PanelKind;
        let mut mem=MemMap::new(vec![0;192]);
        for flags in 0..8 {for kind in [PanelKind::None,PanelKind::Stats,PanelKind::Actions,PanelKind::Notice] {
            mem.ffta_dialogue.action_layout=flags&1!=0;
            mem.ffta_dialogue.town_layout=flags&2!=0;
            mem.ffta_dialogue.shop_layout=flags&4!=0;
            mem.ffta_dialogue.extra.kind=kind;
            for bounds in [[128,16,240,144],[-8,0,32,160],[200,40,160,20]] {
                mem.ffta_dialogue.extra.bounds=Some(bounds);
                for banner in [None,Some([-120,20,360,80]),Some([160,0,200,160]),Some([200,100,40,50])] {
                    mem.ffta_dialogue.action_banner=banner;
                    for line in [0,15,16,37,79,80,143,144,159] {for extra in [false,true] {
                        let row=UiPanelRow::new(&mem,line,extra);
                        for x in -120..360 {
                            assert_eq!(row.contains(x),ui_panel_contains_uncached(&mem,line,x,extra));
                        }
                    }}
                }
            }
        }}
        // Missing bounds remain lazy when a banner overrides them.
        mem.ffta_dialogue.action_layout=true;mem.ffta_dialogue.extra.kind=PanelKind::Notice;
        mem.ffta_dialogue.extra.bounds=None;mem.ffta_dialogue.action_banner=Some([-120,0,360,160]);
        for extra in [false,true] {
            let row=UiPanelRow::new(&mem,37,extra);
            assert_eq!(row.contains(0),!extra);
            assert_eq!(row.contains(0),ui_panel_contains_uncached(&mem,37,0,extra));
        }
    }

    #[test]
    fn panel_row_without_eligible_background_does_not_require_bounds() {
        use crate::ffta_dialogue::{PanelKind,LAYER_WIDTH};
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
        let mut actual=crate::Machine::new(vec![0;192]);
        actual.bus.ffta_dialogue.action_layout=true;
        actual.bus.ffta_dialogue.extra.kind=PanelKind::Notice;
        actual.bus.ffta_dialogue.pixels.resize(LAYER_WIDTH*160,Default::default());
        let mut reference=crate::Machine::new(vec![0;192]);reference.load_state(&actual.save_state().unwrap()).unwrap();
        let backgrounds=[[TRANSPARENT;MAX_RENDER_WIDTH];4];let objects=[OBJ_EMPTY;MAX_RENDER_WIDTH];
        let window=[false;MAX_RENDER_WIDTH];let rows=UiBgRow {text:[TRANSPARENT;MAX_RENDER_WIDTH],backing:[TRANSPARENT;MAX_RENDER_WIDTH]};
        capture_dialogue_part::<false,true>(&mut actual.bus,0,37,&backgrounds,&[false;4],&objects,&window,false,false,&rows);
        capture_dialogue_part::<false,false>(&mut reference.bus,0,37,&backgrounds,&[false;4],&objects,&window,false,false,&rows);
        assert!(actual.save_state().unwrap()==reference.save_state().unwrap());
    }

    #[test]
    fn cached_windows_match_pixel_reference_for_wrap_empty_and_overlap() {
        let mut mem=MemMap::new(vec![0;192]);
        let cases=[(0x0838u16,0x0010u16),(0xf010,0x9808),(0x0505,0x0505),(0x00ff,0x00ff),(0x8001,0x8001)];
        for winin in [0x1021u16,0x3f00,0] {
            mem.io[0x48..0x4a].copy_from_slice(&winin.to_le_bytes());
            mem.io[0x4a..0x4c].copy_from_slice(&0x0a35u16.to_le_bytes());
            for (h0,v0) in cases {for (h1,v1) in cases {
                for (offset,value) in [(0x40,h0),(0x42,h1),(0x44,v0),(0x46,v1)] {
                    mem.io[offset..offset+2].copy_from_slice(&value.to_le_bytes());
                }
                for flags in 0..8u16 {for y in [0u8,5,8,10,128,152,159,255] {
                    let dispcnt=flags<<13;
                    let row=UiWindowRow::new(&mem,dispcnt,y as usize);
                    for x in 0..=255u8 {for object in [false,true] {
                        // Previous per-pixel algorithm, with WIN0 overriding WIN1.
                        let mut expected=0x3f;
                        if dispcnt&0xe000!=0 {
                            expected=0x35;
                            if dispcnt&0x8000!=0 && object {expected=0x0a;}
                            for (bit,h,v,mask) in [(0x4000,h1,v1,(winin>>8)&63),(0x2000,h0,v0,winin&63)] {
                                if dispcnt&bit!=0 && in_window_range((h>>8)as u8,h as u8,x)
                                    && in_window_range((v>>8)as u8,v as u8,y) {expected=mask;}
                            }
                        }
                        assert_eq!(row.control(x,object),expected);
                    }}
                }}
            }}
        }
    }
}

#[cfg(test)]
mod dialogue_layout_tests {
    use super::*;
    fn scene(right:bool)->MemMap {
        let mut m=MemMap::new(vec![0;192]);
        m.ffta_movement.enabled=true;
        m.ffta_movement.europe_profile=true;
        m.ffta_bg_edge=FftaBgEdge::Transparent;
        for (off,v) in [(0,0x1f40u16),(8,0x4c03),(10,0x4e02),(12,0x0b08),(14,0x0a08)] {
            m.io[off..off+2].copy_from_slice(&v.to_le_bytes());
        }
        m.vram[0x8020..0x8040].fill(0x11);
        for col in if right {12..30}else{0..18} {
            let off=0x5000+(2*32+col)*2;
            m.vram[off..off+2].copy_from_slice(&1u16.to_le_bytes());
        }
        for (index,color) in [(1,0x001fu16),(259,0x7c00)] {
            m.palette[index*2..index*2+2].copy_from_slice(&color.to_le_bytes());
        }
        for entry in m.oam.chunks_exact_mut(8) {entry[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
        for (off,v) in [(0,0xa000u16),(2,0xc0d0),(4,0x0240)] {
            m.oam[off..off+2].copy_from_slice(&v.to_le_bytes());
        }
        m.vram[0x14800..0x15000].fill(3);
        m
    }
    fn stats_scene()->MemMap {
        let mut m=scene(true);m.ffta_dialogue.separate=true;
        for (bg,cnt) in [0x0a0bu16,0x0b0a,0x4c03,0x4e02].into_iter().enumerate() {
            m.io[8+bg*2..10+bg*2].copy_from_slice(&cnt.to_le_bytes());
        }
        m.vram[0x5000..0x6000].fill(0);
        for o in m.oam.chunks_exact_mut(8) {o[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
        for (slot,attrs) in [[0x4068u16,0x8098,0xc228],[0x0468,0x8098,0xb1f0],
            [0x4488,0x8098,0xb200],[0x4468,0xc0b0,0xb208],[0x4478,0xc0b0,0xb208]].into_iter().enumerate() {
            for (i,a) in attrs.iter().enumerate() {m.oam[slot*8+i*2..slot*8+i*2+2].copy_from_slice(&a.to_le_bytes());}
        }
        m.vram[0x13e00..0x14800].fill(0x11);
        m.palette[0x362..0x364].copy_from_slice(&0x7c00u16.to_le_bytes());
        m.palette[0x382..0x384].copy_from_slice(&31u16.to_le_bytes());
        m.io[0x50..0x52].copy_from_slice(&0x0c00u16.to_le_bytes());
        m.io[0x52..0x54].copy_from_slice(&0x0808u16.to_le_bytes());
        m
    }
    #[test]
    fn mixed_ui_rows_clear_world_only_pixels_and_preserve_portraits_and_blending() {
        use crate::ffta_dialogue::{LAYER_WIDTH,PanelKind,UiPixel};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let backgrounds=[[0x7c00u16;MAX_RENDER_WIDTH];4];
        let window=std::array::from_fn(|x|x%11==0);
        let mut rows=UiBgRow {text:[TRANSPARENT;MAX_RENDER_WIDTH],backing:[TRANSPARENT;MAX_RENDER_WIDTH]};
        for x in 0..240 {
            if x%17==0 {rows.text[x]=31;}
            if x%29==0 {rows.backing[x]=0x03e0;}
        }
        let mut mixed=false;let mut world_blend=false;let mut portrait_only=false;
        for kind in [PanelKind::Actions,PanelKind::Stats,PanelKind::Notice] {
            for extra in [false,true] {for backing in [false,true] {for mode in 0..4u16 {for priority in [255,2] {
                let mut actual=crate::Machine::new(vec![0;192]);actual.bus=stats_scene();
                actual.bus.ffta_dialogue.extra.kind=kind;
                actual.bus.ffta_dialogue.extra.bounds=Some([128,16,240,144]);
                for pixels in [&mut actual.bus.ffta_dialogue.pixels,&mut actual.bus.ffta_dialogue.backing,
                    &mut actual.bus.ffta_dialogue.extra.pixels,&mut actual.bus.ffta_dialogue.extra.backing] {
                    pixels.resize(LAYER_WIDTH*160,UiPixel::opaque(0x1234));
                }
                for (offset,value) in [(8,3u16),(10,3),(12,0),(14,0),(0x48,0x3f3f),(0x4a,0x3f3f),
                    (0x50,0x3f3f|(mode<<6)),(0x52,0x0808),(0x54,8)] {
                    actual.bus.io[offset..offset+2].copy_from_slice(&value.to_le_bytes());
                }
                let objects=[ObjPixel {color:0x03e0,priority,semi:true,..OBJ_EMPTY};MAX_RENDER_WIDTH];
                let mut reference=crate::Machine::new(vec![0;192]);reference.load_state(&actual.save_state().unwrap()).unwrap();
                capture_dialogue_part::<true,true>(&mut actual.bus,0xff40,104,&backgrounds,&[true;4],&objects,&window,extra,backing,&rows);
                capture_dialogue_part::<false,false>(&mut reference.bus,0xff40,104,&backgrounds,&[true;4],&objects,&window,extra,backing,&rows);
                let target=if backing {if extra {&actual.bus.ffta_dialogue.extra.backing}else{&actual.bus.ffta_dialogue.backing}}
                    else if extra {&actual.bus.ffta_dialogue.extra.pixels}else{&actual.bus.ffta_dialogue.pixels};
                let row=&target[104*LAYER_WIDTH..105*LAYER_WIDTH];
                mixed|=row.iter().any(|&p|p==UiPixel::default()) && row.iter().any(|&p|p!=UiPixel::default());
                world_blend|=row.iter().any(|p|p.eva==8 && p.evb==8);
                portrait_only|=kind==PanelKind::Stats && extra && !backing && row.iter().any(|&p|p!=UiPixel::default());
                assert_eq!(target[0],UiPixel::opaque(0x1234));
                assert!(actual.save_state().unwrap()==reference.save_state().unwrap(),"pixel shortcut changed full panel/backing state");
            }}}}
        }
        assert!(mixed && world_blend && portrait_only,"fixture must exercise holes, world alpha and Stats portrait UI");
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }

    #[test]
    fn empty_ui_rows_clear_stale_pixels_and_match_full_loop_with_world_blending() {
        use crate::ffta_dialogue::{LAYER_WIDTH,PanelKind,UiPixel};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let backgrounds=[[0x03e0u16;MAX_RENDER_WIDTH];4];
        let objects=[ObjPixel {color:31,priority:0,semi:true,..OBJ_EMPTY};MAX_RENDER_WIDTH];
        let window=std::array::from_fn(|x|x%5==0);
        let rows=UiBgRow {text:[TRANSPARENT;MAX_RENDER_WIDTH],backing:[TRANSPARENT;MAX_RENDER_WIDTH]};
        for kind in [PanelKind::Stats,PanelKind::Actions,PanelKind::Notice] {
            for extra in [false,true] {for backing in [false,true] {for mode in 0..4u16 {
                let mut actual=crate::Machine::new(vec![0;192]);actual.bus=scene(false);
                for obj in actual.bus.oam.chunks_exact_mut(8) {obj[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
                actual.bus.ffta_dialogue.action_layout=true;
                actual.bus.ffta_dialogue.extra.kind=kind;
                actual.bus.ffta_dialogue.extra.bounds=Some([128,16,240,144]);
                for pixels in [&mut actual.bus.ffta_dialogue.pixels,&mut actual.bus.ffta_dialogue.backing,
                    &mut actual.bus.ffta_dialogue.extra.pixels,&mut actual.bus.ffta_dialogue.extra.backing] {
                    pixels.resize(LAYER_WIDTH*160,UiPixel::opaque(31));
                }
                for (offset,value) in [(0x40,0x0838u16),(0x42,0xf010),(0x44,0x0010),(0x46,0x9808),
                    (0x48,0x152a),(0x4a,0x3f3f),(0x50,0x3f3f|(mode<<6)),(0x52,0x0808),(0x54,8)] {
                    actual.bus.io[offset..offset+2].copy_from_slice(&value.to_le_bytes());
                }
                let mut reference=crate::Machine::new(vec![0;192]);reference.load_state(&actual.save_state().unwrap()).unwrap();
                capture_dialogue_part::<true,true>(&mut actual.bus,0xff40,37,&backgrounds,&[true;4],&objects,&window,extra,backing,&rows);
                capture_dialogue_part::<false,false>(&mut reference.bus,0xff40,37,&backgrounds,&[true;4],&objects,&window,extra,backing,&rows);
                let target=if backing {if extra {&actual.bus.ffta_dialogue.extra.backing}else{&actual.bus.ffta_dialogue.backing}}
                    else if extra {&actual.bus.ffta_dialogue.extra.pixels}else{&actual.bus.ffta_dialogue.pixels};
                assert!(target[37*LAYER_WIDTH..38*LAYER_WIDTH].iter().all(|&p|p==UiPixel::default()));
                assert_eq!(target[0],UiPixel::opaque(31));
                assert!(!actual.bus.ffta_dialogue.capture && !actual.bus.ffta_dialogue.capture_extra && !actual.bus.ffta_dialogue.capture_backing);
                assert!(actual.save_state().unwrap()==reference.save_state().unwrap(),"empty-row shortcut changed complete state");
            }}}
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }

    #[test]
    fn shared_native_bg_rows_match_fresh_preparation_for_panels_and_backing() {
        use crate::ffta_dialogue::{LAYER_WIDTH,PanelKind,Placement};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let backgrounds=std::array::from_fn(|bg|std::array::from_fn(|x|((x+bg*7)%0x7fff)as u16));
        let objects=[OBJ_EMPTY;MAX_RENDER_WIDTH];
        let window=std::array::from_fn(|x|x%17==0);
        for kind in [PanelKind::Stats,PanelKind::Actions,PanelKind::Notice] {
            for primary in [false,true] {for action in [false,true] {for mask in 0..4 {
                let used=[true,true,mask&1!=0,mask&2!=0];
                let mut actual=crate::Machine::new(vec![0;192]);actual.bus=stats_scene();
                actual.bus.ffta_dialogue.action_layout=action;
                for (offset,value) in [(0x0c,0x5c08u16),(0x0e,0x4a09)] {
                    actual.bus.io[offset..offset+2].copy_from_slice(&value.to_le_bytes());
                }
                actual.bus.vram[0x8020..0x8040].fill(0x11);
                actual.bus.vram[0x80a0..0x80c0].fill(0x22);
                actual.bus.palette[0x162..0x164].copy_from_slice(&31u16.to_le_bytes());
                actual.bus.palette[0x164..0x166].copy_from_slice(&0x03e0u16.to_le_bytes());
                for y in 0..20 {for x in 0..30 {
                    let tile=[0xb005u16,0xb001,0][x%3];
                    let offset=(y*32+x)*2;
                    actual.bus.vram[0xe000+offset..0xe002+offset].copy_from_slice(&tile.to_le_bytes());
                    actual.bus.vram[0x5000+offset..0x5002+offset].copy_from_slice(&0xb001u16.to_le_bytes());
                }}
                actual.bus.ffta_dialogue.placement=primary.then(|| {
                    let mut p=Placement::new(false,568,0,Default::default());p.length_pixels=32;p
                });
                actual.bus.ffta_dialogue.extra.kind=kind;
                actual.bus.ffta_dialogue.extra.bounds=Some([128,16,240,144]);
                actual.bus.ffta_dialogue.pixels.resize(LAYER_WIDTH*160,crate::ffta_dialogue::UiPixel::opaque(31));
                actual.bus.ffta_dialogue.extra.pixels.resize(LAYER_WIDTH*160,crate::ffta_dialogue::UiPixel::opaque(31));
                actual.bus.ffta_dialogue.backing.resize(LAYER_WIDTH*160,crate::ffta_dialogue::UiPixel::opaque(31));
                actual.bus.ffta_dialogue.extra.backing.resize(LAYER_WIDTH*160,crate::ffta_dialogue::UiPixel::opaque(31));
                actual.bus.io[0x48..0x4c].fill(0x3f);
                let mut reference=crate::Machine::new(vec![0;192]);
                reference.load_state(&actual.save_state().unwrap()).unwrap();
                let dispcnt=io16(&actual.bus,0);
                for y in 0..160 {
                    let rows=UiBgRow::new(&actual.bus,y,&used,primary || kind!=PanelKind::Stats);
                    if (primary || kind!=PanelKind::Stats) && used[2] {
                        assert!(rows.text[..240].iter().any(|&pixel|pixel!=TRANSPARENT));
                    }
                    if (primary || kind!=PanelKind::Stats) && used[3] {
                        assert!(rows.backing[..240].iter().any(|&pixel|pixel!=TRANSPARENT));
                    }
                    for extra in [false,true] {
                        if !extra && !primary {continue;}
                        capture_dialogue_row(&mut actual.bus,dispcnt,y,&backgrounds,&used,&objects,&window,extra,&rows);
                        // Before sharing, both normal/backing calls prepared native BGs afresh.
                        for backing in [false,true] {
                            if backing && !(action || (extra && kind==PanelKind::Stats)) {continue;}
                            let fresh=UiBgRow::new(&reference.bus,y,&used,!extra || kind!=PanelKind::Stats);
                            capture_dialogue_part::<false,false>(&mut reference.bus,dispcnt,y,&backgrounds,&used,&objects,&window,extra,backing,&fresh);
                        }
                    }
                }
                assert!(actual.save_state().unwrap()==reference.save_state().unwrap(),"shared rows changed panel/backing or machine state");
            }}}
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }

    #[test]
    fn attack_forecast_keeps_obj_target_beside_bg_attacker_for_both_regions_and_zooms() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET,PanelKind};
        for europe in [false,true] {for (ex,ey) in [(164,0),(448,80)] {
            crate::mem::set_extra(ex);crate::mem::set_extra_y(ey);
            let mut m=stats_scene();m.ffta_movement.europe_profile=europe;
            for (bg,cnt) in [0x4c03u16,0x4e02,0x5c08,0x4a09].into_iter().enumerate() {
                m.io[8+bg*2..10+bg*2].copy_from_slice(&cnt.to_le_bytes());
            }
            for y in 13..19 {for x in 0..17 {let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());}}
            // Forecast glyphs can share bank 11 with its four-piece backing.
            m.oam[4..6].copy_from_slice(&0xb228u16.to_le_bytes());
            m.vram[0x14500..0x14600].fill(0x22);
            m.palette[0x364..0x366].copy_from_slice(&0x03e0u16.to_le_bytes());
            let original=m.oam.clone();
            for y in 0..160 {render_scanline(&mut m,y);}
            assert!(m.ffta_dialogue.action_layout);
            assert_eq!(m.ffta_dialogue.extra.kind,PanelKind::Stats);
            let i=104*LAYER_WIDTH+152+LAYER_INSET as usize;
            assert_eq!(m.ffta_dialogue.extra.pixels[i].color,0x03e0);
            assert_ne!(m.ffta_dialogue.extra.backing[i].color,0x03e0);
            render_ffta_zoom_rows(&mut m);
            let rw=crate::mem::render_width();
            for y in 0..160 {for x in 0..240 {
                let i=y*LAYER_WIDTH+x+LAYER_INSET as usize;
                let world=(y+ey)*rw+x+ex;
                let mut c=m.ffta_dialogue.pixels[i].resolve(m.wide_framebuffer[world]);
                let p=m.ffta_dialogue.extra.pixels[i];
                if p.visible() {c=p.resolve(m.ffta_dialogue.extra.background[world]);}
                assert_eq!(c,m.framebuffer[y*240+x],"europe={europe} ex={ex} x={x} y={y}");
            }}
            assert_eq!(m.oam,original);
        }}
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn stats_layer_reconstructs_original_alpha_and_requires_complete_verified_body() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET,PanelKind};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=stats_scene();let original=m.oam.clone();
        for y in 0..160 {render_scanline(&mut m,y);}
        assert_eq!(m.ffta_dialogue.extra.kind,PanelKind::Stats);
        assert!(m.ffta_dialogue.placement.is_none());
        assert_eq!(m.ffta_dialogue.extra.bounds,Some([152,104,240,152]));
        for y in 0..160 {for x in 0..240 {
            let ui=m.ffta_dialogue.extra.pixels[y*LAYER_WIDTH+x+LAYER_INSET as usize];
            assert_eq!(ui.resolve(m.wide_framebuffer[y*568+x+164]),m.framebuffer[y*240+x],"x={x} y={y}");
        }}
        assert_eq!(m.oam,original);
        m.oam[4*8..4*8+2].copy_from_slice(&0x0200u16.to_le_bytes());
        begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.extra.active());
        m=stats_scene();m.ffta_movement.enabled=false;
        begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.extra.active());
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn stats_blend_uses_raw_bg_even_when_world_obj_is_underneath() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET};
        crate::mem::set_extra(164);crate::mem::set_extra_y(80);
        let mut m=stats_scene();
        for (i,a) in [0x4068u16,0x8098,0xd628].into_iter().enumerate() {
            m.oam[20*8+i*2..20*8+i*2+2].copy_from_slice(&a.to_le_bytes());
        }
        m.vram[0x14500..0x14900].fill(0x11);
        m.palette[0x3a2..0x3a4].copy_from_slice(&0x03e0u16.to_le_bytes());
        m.io[0x50..0x52].copy_from_slice(&0x3f43u16.to_le_bytes());
        m.io[0x52..0x54].copy_from_slice(&0x090eu16.to_le_bytes());
        for y in 0..160 {render_scanline(&mut m,y);}
        set_parallel_zoom(false);render_ffta_zoom_rows(&mut m);
        let serial=m.ffta_dialogue.extra.background.clone();
        let world=m.wide_framebuffer.clone();
        for y in 0..160 {for x in 0..240 {
            let p=m.ffta_dialogue.extra.pixels[y*LAYER_WIDTH+x+LAYER_INSET as usize];
            let index=(y+80)*568+x+164;
            let c=if p.visible(){p.resolve(serial[index])}else{world[index]};
            assert_eq!(c,m.framebuffer[y*240+x],"x={x} y={y}");
        }}
        for y in 0..160 {render_scanline(&mut m,y);}
        set_parallel_zoom(true);render_ffta_zoom_rows(&mut m);
        assert_eq!(m.ffta_dialogue.extra.background,serial);
        assert_eq!(m.wide_framebuffer,world);
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn legacy_stats_padding_never_changes_separate_cap_on_either_side() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for mirror in [false,true] {
            let mut plain=stats_scene();let mut tagged=stats_scene();
            if mirror {
                for m in [&mut plain,&mut tagged] {for obj in m.oam.chunks_exact_mut(8) {
                    let a=u16::from_le_bytes([obj[0],obj[1]]);if a&0x300==0x200 {continue;}
                    let b=u16::from_le_bytes([obj[2],obj[3]]);
                    let w=OBJ_SIZES[(a>>14)as usize][(b>>14)as usize].0;
                    let x=240-(b&511)as i32-w;
                    obj[2..4].copy_from_slice(&(((b^0x1000)&!511)|(x as u16&511)).to_le_bytes());
                }}
            }
            tagged.ffta_sprites.active=Some(tagged.oam.chunks_exact(8).enumerate().filter_map(|(slot,obj)|{
                let attrs:[u16;3]=std::array::from_fn(|i|u16::from_le_bytes([obj[i*2],obj[i*2+1]]));
                if attrs[2]>>12!=11 {return None;}
                Some(crate::ffta_sprites::HostSprite{attrs,x:(attrs[1]&511)as i32,y:(attrs[0]&255)as i32,
                    slot,replaces:true,stats_background:true,stats_inner_padding:true,..Default::default()})
            }).collect());
            for y in 0..160 {render_scanline(&mut plain,y);render_scanline(&mut tagged,y);}
            assert_eq!(plain.ffta_dialogue.extra.backing,tagged.ffta_dialogue.extra.backing);
            assert_eq!(plain.ffta_dialogue.extra.pixels,tagged.ffta_dialogue.extra.pixels);
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn action_frames_are_backing_and_world_portraits_cannot_punch_holes_in_it() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;
        m.io[0xc..0xe].copy_from_slice(&0x5c08u16.to_le_bytes());
        m.io[0xe..0x10].copy_from_slice(&0x4a09u16.to_le_bytes());
        m.vram[0xe000..0xe800].fill(0);
        for y in 13..19 {for x in 0..17 {let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());}}
        for (y,tile) in [(8,6u16),(9,8),(18,10)] {
            m.vram[0x8000+tile as usize*32..0x8000+tile as usize*32+32].fill(0x11);
            for x in 22..30 {let o=0xe000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&tile.to_le_bytes());}
        }
        m.vram[0x8400..0x8420].fill(0x11);
        let o=0xe000+(10*32+23)*2;m.vram[o..o+2].copy_from_slice(&32u16.to_le_bytes());
        for (i,a) in [0xa068u16,0xc000,0x0640].into_iter().enumerate() {m.oam[i*2..i*2+2].copy_from_slice(&a.to_le_bytes());}
        for y in 0..160 {render_scanline(&mut m,y);}
        let index=|x,y|y*LAYER_WIDTH+x+LAYER_INSET as usize;
        assert!(m.ffta_dialogue.backing[index(16,110)].visible());
        for y in [64,72,144] {assert!(m.ffta_dialogue.extra.backing[index(232,y)].visible());}
        assert!(!m.ffta_dialogue.extra.backing[index(184,80)].visible());
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn notice_window_is_a_complete_modal_and_keeps_action_hud_separate() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET,PanelKind};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);
        for(o,v)in[(0,0x3f40u16),(12,0x5c08),(14,0x4a09),(0x40,0x10e0),(0x44,0x1040),(0x48,31),(0x4a,63)] {m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.oam.fill(0);for obj in m.oam.chunks_exact_mut(8){obj[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        m.vram[0x5000..0x5800].fill(0);
        for tile in [3usize,13,14,15,16] {m.vram[0x8000+tile*32..0x8000+tile*32+32].fill(0x11);}
        m.palette[0x162..0x164].copy_from_slice(&0x3defu16.to_le_bytes());
        for y in 2..8 {for x in 2..28 {let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&0xb003u16.to_le_bytes());}}
        for(x,y,tile)in[(2,2,0xb00du16),(27,2,0xb00e),(2,7,0xb00f),(27,7,0xb010)] {let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&tile.to_le_bytes());}
        for(x,y)in[(26,4),(2,14)] {let o=0xe000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());}
        // Choice cursor belongs to the same modal as its BG text/frame.
        for(o,v)in[(0,44u16),(2,27),(4,0x125)]{m.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.vram[0x10000+0x125*32..0x10000+0x126*32].fill(0x11);
        m.palette[0x202..0x204].copy_from_slice(&0x03e0u16.to_le_bytes());
        for y in 0..160 {render_scanline(&mut m,y);}
        let hardware=m.framebuffer.clone();let oam=m.oam.clone();
        m.ffta_dialogue.separate=true;
        for y in 0..160 {render_scanline(&mut m,y);}
        assert_eq!(m.ffta_dialogue.extra.kind,PanelKind::Notice);
        assert_eq!(m.ffta_dialogue.extra.bounds,Some([16,16,224,64]));
        let idx=|x,y|y*LAYER_WIDTH+x+LAYER_INSET as usize;
        assert!(!m.ffta_dialogue.pixels[idx(208,32)].visible());
        assert_eq!(m.ffta_dialogue.extra.pixels[idx(208,32)].resolve(0),31);
        assert_eq!(m.ffta_dialogue.pixels[idx(16,112)].resolve(0),31);
        assert_eq!(m.ffta_dialogue.extra.pixels[idx(27,44)].resolve(0),0x03e0);
        assert!(!m.ffta_dialogue.pixels[idx(27,44)].visible());
        assert!(!m.ffta_dialogue.extra.pixels[idx(16,112)].visible());
        for y in 0..160 {for x in 0..240 {
            let world=m.wide_framebuffer[y*568+x+164];
            let restored=m.ffta_dialogue.extra.pixels[idx(x,y)].resolve(m.ffta_dialogue.pixels[idx(x,y)].resolve(world));
            assert_eq!(restored,hardware[y*240+x],"x={x} y={y}");
        }}
        assert_eq!(m.framebuffer,hardware);assert_eq!(m.oam,oam);
        // A portrait overlapping the old native modal location must not leave
        // a hole after the portrait and modal move to independent destinations.
        for(o,v)in[(8,0x2030u16),(10,16),(12,0x200)]{m.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.vram[0x14000..0x14040].fill(1);
        m.ffta_dialogue.separate=false;
        for y in 0..160 {render_scanline(&mut m,y);}
        let hardware=m.framebuffer.clone();let oam=m.oam.clone();
        m.ffta_dialogue.separate=true;
        for y in 0..160 {render_scanline(&mut m,y);}
        assert_eq!(m.ffta_dialogue.extra.pixels[idx(16,48)].resolve(0),0x3def);
        assert_eq!(m.framebuffer,hardware);assert_eq!(m.oam,oam);
        m.io[0x40..0x42].copy_from_slice(&0xa0f0u16.to_le_bytes());
        assert!(notice_bounds(&m).is_none()); // narrow action confirmation remains unchanged
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    fn laws_scene(help: bool, hud: bool) -> MemMap {
        let mut m = scene(false);
        m.ffta_dialogue.separate = true;
        for (off, value) in [(12, 0x5c08u16), (14, 0x4a09)] {
            m.io[off..off+2].copy_from_slice(&value.to_le_bytes());
        }
        m.vram[0x5000..0x5800].fill(0);
        let mut tile = |x: usize, y: usize, value: u16| {
            let off = 0x5000 + (y * 32 + x) * 2;
            m.vram[off..off+2].copy_from_slice(&value.to_le_bytes());
        };
        for x in 2..26 { tile(x, 9, 0x70e2); tile(x, 15, 0x70ef); }
        for (x, y, value) in [(1,9,0x70e1), (26,9,0x74e1), (1,15,0x70ee), (26,15,0x74ee)] { tile(x,y,value); }
        for y in [11, 13] {
            for x in 2..25 { tile(x,y,0x70f4); tile(x,y+1,0x70f7); }
            for (x,dy,value) in [(1,0,0x70f3), (13,0,0x70fe), (25,0,0x80f5),
                (1,1,0x70f6), (13,1,0x70ff), (25,1,0x80f8)] { tile(x,y+dy,value); }
        }
        if help {
            for y in 2..8 { for x in 2..28 { tile(x,y,0xb003); } }
            for (x,y,value) in [(2,2,0xb00d),(27,2,0xb00e),(2,7,0xb00f),(27,7,0xb010)] { tile(x,y,value); }
        }
        if hud { for x in 0..8 { tile(x,17,1); } }
        for index in [1usize,3,13,14,15,16,0xe1,0xe2,0xee,0xef,0xf3,0xf4,0xf5,0xf6,0xf7,0xf8,0xfe,0xff] {
            m.vram[0x8000+index*32..0x8000+(index+1)*32].fill(0x11);
        }
        for bank in [7usize,8,11] { m.palette[(bank*16+1)*2..(bank*16+1)*2+2].copy_from_slice(&0x3defu16.to_le_bytes()); }
        // Distinct source text on both sides of the former action split.
        for (x,y) in [(2,11),(23,11),(3,4),(24,5)] {
            if help || y == 11 { let off=0xe000+(y*32+x)*2; m.vram[off..off+2].copy_from_slice(&1u16.to_le_bytes()); }
        }
        m.oam.fill(0);
        for obj in m.oam.chunks_exact_mut(8) { obj[..2].copy_from_slice(&0x200u16.to_le_bytes()); }
        for (off,value) in [(0,0x8058u16),(2,2),(4,0x2dc)] { m.oam[off..off+2].copy_from_slice(&value.to_le_bytes()); }
        m.vram[0x10000+0x2dc*32..0x10000+0x2de*32].fill(0x11);
        m.palette[0x202..0x204].copy_from_slice(&0x03e0u16.to_le_bytes());
        if help {
            for (off,value) in [(8,0x8035u16),(10,116),(12,0x2122)] { m.oam[off..off+2].copy_from_slice(&value.to_le_bytes()); }
            m.vram[0x10000+0x122*32..0x10000+0x124*32].fill(0x11);
            m.palette[0x242..0x244].copy_from_slice(&0x7c00u16.to_le_bytes());
        }
        m
    }
    #[test]
    fn laws_table_and_help_remain_one_modal_with_or_without_hud() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET,PanelKind};
        for ex in [22,164,448] { for help in [false,true] { for hud in [false,true] {
            crate::mem::set_extra(ex); crate::mem::set_extra_y(0);
            let mut m=laws_scene(help,hud);
            m.ffta_dialogue.separate=false;
            for y in 0..160 { render_scanline(&mut m,y); }
            let hardware=m.framebuffer.clone(); let oam=m.oam.clone(); let vram=m.vram.clone();
            m.ffta_dialogue.separate=true;
            for y in 0..160 { render_scanline(&mut m,y); }
            assert!(m.ffta_dialogue.laws_layout);
            assert_eq!(m.ffta_dialogue.extra.kind,PanelKind::Notice);
            assert_eq!(m.ffta_dialogue.extra.bounds,Some(if help {[0,16,224,128]}else{[0,72,216,128]}));
            let idx=|x,y| y*LAYER_WIDTH+x+LAYER_INSET as usize;
            for (x,y) in [(16,88),(184,88),(2,88)] {
                assert!(m.ffta_dialogue.extra.pixels[idx(x,y)].visible());
                assert!(!m.ffta_dialogue.pixels[idx(x,y)].visible());
            }
            if help { assert!(m.ffta_dialogue.extra.pixels[idx(116,53)].visible()); }
            if hud { assert!(m.ffta_dialogue.pixels[idx(0,136)].visible()); }
            for y in 0..160 { for x in 0..240 {
                let world=m.wide_framebuffer[y*(240+2*ex)+x+ex];
                let restored=m.ffta_dialogue.extra.pixels[idx(x,y)].resolve(m.ffta_dialogue.pixels[idx(x,y)].resolve(world));
                assert_eq!(restored,hardware[y*240+x],"help={help} hud={hud} ex={ex} x={x} y={y}");
            }}
            assert_eq!(m.framebuffer,hardware); assert_eq!(m.oam,oam); assert_eq!(m.vram,vram);
            // Reject incomplete frames and clear the subtype on the next frame.
            m.vram[0x5000+(9*32+1)*2]=0;
            assert!(laws_bounds(&m).is_none());
            for y in 0..160 { render_scanline(&mut m,y); }
            assert!(!m.ffta_dialogue.laws_layout);
        }}}
        crate::mem::set_extra(0); crate::mem::set_extra_y(0);
    }
    #[test]
    fn long_ear_pieces_join_action_portrait_without_changing_hardware_pixels() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);
        m.io[0xc..0xe].copy_from_slice(&0x5c08u16.to_le_bytes());
        m.io[0xe..0x10].copy_from_slice(&0x4a09u16.to_le_bytes());
        // Measured Clara pieces: 32x16 at (0,56), 8x8 at (16,48).
        for (slot,attrs) in [[0x6038u16,0x9000,0x06aa],[0x2030,0x1010,0x06ba]].into_iter().enumerate() {
            for (i,a) in attrs.iter().enumerate() {m.oam[slot*8+i*2..slot*8+i*2+2].copy_from_slice(&a.to_le_bytes());}
        }
        m.vram[0x15540..0x15800].fill(3);
        for y in 0..160 {render_scanline(&mut m,y);}
        let hardware=m.framebuffer.clone();let oam=m.oam.clone();
        m.ffta_dialogue.separate=true;
        for y in 0..160 {render_scanline(&mut m,y);}
        assert!(m.ffta_dialogue.action_layout);
        for (x,y) in [(0,56),(16,48)] {
            assert_eq!(m.ffta_dialogue.pixels[y*LAYER_WIDTH+x+LAYER_INSET as usize].resolve(0),0x7c00);
            assert_ne!(m.wide_framebuffer[y*568+x+164],0x7c00,"ear piece must leave the world layer");
        }
        assert_eq!(m.framebuffer,hardware);assert_eq!(m.oam,oam);
        assert!(!crate::ffta_dialogue::action_portrait([0x4038,0x9000,0x06aa],0,56));
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn action_frame_width_follows_native_header_and_keeps_cursor_with_it() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for left in [18usize,19,20,22] {
            let mut m=scene(false);
            for(o,v)in[(12,0x5c08u16),(14,0x4a09)] {m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
            m.vram[0xe000..0xe800].fill(0);
            for tile in [5usize,6,9,10] {m.vram[0x8000+tile*32..0x8000+tile*32+32].fill(0x11);}
            m.palette[0x182..0x184].copy_from_slice(&0x03e0u16.to_le_bytes());
            for y in [10,18] {for x in left..30 {
                let tile=if y==10 {if x==left{5}else{6}}else if x==left{9}else{10};
                let o=0xe000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&(0xc000u16+tile).to_le_bytes());
            }}
            m.oam.fill(0);for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
            for(o,v)in[(0,117u16),(2,(left*8-1)as u16),(4,0x125)]{m.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
            m.vram[0x124a0..0x124c0].fill(0x11);
            m.palette[0x202..0x204].copy_from_slice(&31u16.to_le_bytes());
            for y in 0..160 {render_scanline(&mut m,y);}
            let hardware=m.framebuffer.clone();let oam=m.oam.clone();
            m.ffta_dialogue.separate=true;
            for y in 0..160 {render_scanline(&mut m,y);}
            assert_eq!(m.ffta_dialogue.extra.bounds,Some([(left*8)as i32,80,240,152]));
            let at=|x,y|y*LAYER_WIDTH+x+LAYER_INSET as usize;
            assert_eq!(m.ffta_dialogue.extra.pixels[at(left*8,80)].resolve(0),0x03e0);
            assert_eq!(m.ffta_dialogue.extra.pixels[at(left*8-1,117)].resolve(0),31);
            assert!(!m.ffta_dialogue.pixels[at(left*8,80)].visible());
            assert_eq!(m.framebuffer,hardware);assert_eq!(m.oam,oam);
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn spell_help_strip_keeps_its_right_text_with_the_left_panel() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);
        for obj in m.oam.chunks_exact_mut(8){obj[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        for(o,v)in[(0,0x3f40u16),(12,0x5c08),(14,0x4a09),(0x40,0x10e0),(0x44,0x0818),(0x48,31),(0x4a,63)] {m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        for tile in [0x32usize,0x33] {m.vram[0x8000+tile*32..0x8000+tile*32+32].fill(0x11);}
        m.palette[0x162..0x164].copy_from_slice(&0x03e0u16.to_le_bytes());
        for y in 1..3 {for x in 2..28 {
            let a=0xb000u16+if x==2||x==27{0x32}else{0x33}
                +if x==27{0x400}else{0}+if y==2{0x800}else{0};
            let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&a.to_le_bytes());
        }}
        let o=0xe000+(1*32+26)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
        for y in 0..160 {render_scanline(&mut m,y);}
        let hardware=m.framebuffer.clone();let oam=m.oam.clone();
        m.ffta_dialogue.separate=true;
        for y in 0..160 {render_scanline(&mut m,y);}
        assert_eq!(m.ffta_dialogue.action_banner,Some([16,8,224,24]));
        let i=8*LAYER_WIDTH+208+LAYER_INSET as usize;
        assert_eq!(m.ffta_dialogue.pixels[i].resolve(0),31);
        assert!(!m.ffta_dialogue.extra.pixels[i].visible());
        assert_eq!(m.framebuffer,hardware);assert_eq!(m.oam,oam);
        m.io[0x44..0x46].copy_from_slice(&0x0820u16.to_le_bytes());
        assert!(action_banner_bounds(&m).is_none());
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn actions_have_two_disjoint_layers_with_portrait_and_cursor() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET,PanelKind};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;
        m.io[0xc..0xe].copy_from_slice(&0x5c08u16.to_le_bytes());
        m.io[0xe..0x10].copy_from_slice(&0x4a09u16.to_le_bytes());
        m.vram[0x5000..0x5800].fill(0);
        for (cols,rows) in [(0..17,13..19),(22..30,10..18)] {
            for y in rows {for x in cols.clone() {
                let off=0x5000+(y*32+x)*2;m.vram[off..off+2].copy_from_slice(&1u16.to_le_bytes());
            }}
        }
        for (slot,attrs) in [[0xa058u16,0xc000,0x0640],[0x0055,0x00af,0x0125]].into_iter().enumerate() {
            for (i,a) in attrs.iter().enumerate() {m.oam[slot*8+i*2..slot*8+i*2+2].copy_from_slice(&a.to_le_bytes());}
        }
        m.vram[0x124a0..0x124c0].fill(0x11);
        m.palette[0x202..0x204].copy_from_slice(&0x03ffu16.to_le_bytes());
        let original=m.oam.clone();
        for y in 0..160 {render_scanline(&mut m,y);}
        assert!(m.ffta_dialogue.action_layout);
        assert_eq!(m.ffta_dialogue.extra.kind,PanelKind::Actions);
        for y in 0..160 {for x in 0..240 {
            let n=y*LAYER_WIDTH+x+LAYER_INSET as usize;
            let main=m.ffta_dialogue.pixels[n];let extra=m.ffta_dialogue.extra.pixels[n];
            assert!(!(main.visible() && extra.visible()));
            let world=m.wide_framebuffer[y*568+x+164];
            assert_eq!(extra.resolve(main.resolve(world)),m.framebuffer[y*240+x],"x={x} y={y}");
        }}
        assert_eq!(m.ffta_dialogue.pixels[88*LAYER_WIDTH+120].resolve(0),0x7c00);
        assert_eq!(m.ffta_dialogue.extra.pixels[85*LAYER_WIDTH+120+175].resolve(0),0x03ff);
        assert_eq!(m.oam,original);
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn actual_backing_edges_select_side_and_zoom_does_not_stretch_panel() {
        for (ex,ey) in [(164,0),(448,80)] {
            crate::mem::set_extra(ex);crate::mem::set_extra_y(ey);
            for right in [false,true] {
                let mut m=scene(right);
                begin_dialogue_frame(&mut m);
                let p=m.ffta_dialogue.placement.expect("verified backing edge");
                m.ffta_dialogue.wide_pass=true;
                let sy=(20.*p.scale).floor() as i32-ey as i32;
                let mut row=[TRANSPARENT;MAX_RENDER_WIDTH];
                render_text_bg(&m,3,sy as usize,&mut row);
                let painted=row[..render_width()].iter().filter(|&&c|c!=TRANSPARENT).count();
                assert_eq!(painted,(144.*p.scale) as usize);
                assert_eq!(row[if right {render_width()-1}else{0}],0x001f);
                assert_eq!(row[if right {0}else{render_width()-1}],TRANSPARENT);
            }
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn portrait_moves_and_scales_with_panel_without_changing_hardware_frame() {
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
        let mut hardware=scene(true);
        for y in 0..160 {render_scanline(&mut hardware,y);}
        assert!(hardware.ffta_dialogue.placement.is_none());
        crate::mem::set_extra(164);
        let mut wide=scene(true);
        let oam=wide.oam.clone();
        for y in 0..160 {render_scanline(&mut wide,y);}
        assert_eq!(wide.framebuffer,hardware.framebuffer);
        assert_eq!(wide.oam,oam);
        assert_eq!(wide.wide_framebuffer[568*4+540],0x7c00);
        assert_ne!(wide.wide_framebuffer[568*4+376],0x7c00);
        crate::mem::set_extra(448);crate::mem::set_extra_y(80);
        let mut zoom=scene(true);
        for y in 0..160 {render_scanline(&mut zoom,y);}
        render_ffta_zoom_rows(&mut zoom);
        assert_eq!(zoom.wide_framebuffer[1136*8+1080],0x7c00);
        assert_eq!(zoom.framebuffer,hardware.framebuffer);
        assert_eq!(zoom.oam,oam);
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn continuation_marker_follows_dialogue_and_keeps_guest_bounce_and_visibility() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for host in [false,true] {for y in [57u16,58,59,145,146,147] {
            let mut m=scene(true);m.ffta_dialogue.separate=true;
            if y>=140 {
                m.oam[..2].copy_from_slice(&0xa058u16.to_le_bytes());
                m.vram[0x5000..0x5800].fill(0);
                for col in 12..30 {
                    let off=0x5000+(14*32+col)*2;
                    m.vram[off..off+2].copy_from_slice(&1u16.to_le_bytes());
                }
            }
            m.vram[0x12440..0x12480].fill(0x11);
            m.palette[0x242..0x244].copy_from_slice(&0x03ffu16.to_le_bytes());
            let attrs=[0x8000|y,136,0x2122];
            for (i,a) in attrs.iter().enumerate() {m.oam[8+i*2..10+i*2].copy_from_slice(&a.to_le_bytes());}
            if host {
                m.ffta_sprites.enabled=true;
                m.ffta_sprites.active=Some(vec![crate::ffta_sprites::HostSprite {
                    attrs,x:136,y:y as i32,slot:1,replaces:true,..Default::default()
                }]);
            }
            let oam=m.oam.clone();
            for line in 0..160 {render_scanline(&mut m,line);}
            assert_eq!(m.ffta_dialogue.vertical_anchor(),if y>=140 {160.}else{0.});
            let ui=m.ffta_dialogue.pixels[y as usize*LAYER_WIDTH+LAYER_INSET as usize+136];
            assert_eq!(ui.resolve(0),0x03ff);
            assert_ne!(m.wide_framebuffer[y as usize*568+164+136],0x03ff);
            assert_eq!(m.oam,oam);
            m.ffta_sprites.active=None;
            m.oam[8..10].copy_from_slice(&0x0200u16.to_le_bytes());
            for line in 0..160 {render_scanline(&mut m,line);}
            assert!(!m.ffta_dialogue.pixels[y as usize*LAYER_WIDTH+LAYER_INSET as usize+136].visible());
        }}
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn separate_layer_reconstructs_native_dialogue_with_exact_alpha() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for blend in [false,true] {
            let mut m=scene(true);
            m.ffta_dialogue.separate=true;
            m.vram[64..96].fill(0x22);
            for tile in m.vram[0x6000..0x7000].chunks_exact_mut(2) {tile.copy_from_slice(&2u16.to_le_bytes());}
            m.palette[4..6].copy_from_slice(&0x03e0u16.to_le_bytes());
            if blend {
                m.io[0x50..0x52].copy_from_slice(&0x0148u16.to_le_bytes());
                m.io[0x52..0x54].copy_from_slice(&0x0808u16.to_le_bytes());
            }
            for y in 0..160 {render_scanline(&mut m,y);}
            assert!(m.ffta_dialogue.layer().is_some());
            for y in 0..160 {for x in 0..240 {
                let pixel=m.ffta_dialogue.pixels[y*LAYER_WIDTH+x+LAYER_INSET as usize];
                let world=m.wide_framebuffer[y*568+x+164];
                assert_eq!(pixel.resolve(world),m.framebuffer[y*240+x],"blend={blend} x={x} y={y}");
            }}
            assert_eq!(m.wide_framebuffer[4*568+208+164],0x03e0,"portrait must leave world layer");
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn published_host_portrait_matches_oam_at_both_edges_and_all_sizes() {
        for right in [false,true] {
            for size in [0.5,1.,1.5] {
                for level in 0..=6 {
                    let z=1.122462f32.powi(level);
                    let ex=((568.*z-240.)/2.).ceil() as usize;
                    let ey=((160.*z-160.)/2.).ceil() as usize;
                    crate::mem::set_extra(ex);crate::mem::set_extra_y(ey);
                    let x=if right {208}else{0};
                    let mut frames=Vec::new();
                    for published in [false,true] {
                        let mut m=scene(right);
                        let a1=0xc000u16|x as u16;
                        m.oam[2..4].copy_from_slice(&a1.to_le_bytes());
                        m.ffta_dialogue.style.scale=size;
                        m.ffta_dialogue.view_zoom=z;
                        m.ffta_dialogue.view_width=568.*z;
                        m.ffta_dialogue.view_height=160.*z;
                        if published {
                            m.ffta_sprites.enabled=true;
                            m.ffta_sprites.active=Some(vec![crate::ffta_sprites::HostSprite {
                                attrs:[0xa000,a1,0x240], x, y:0, slot:0, replaces:true,
                                ..Default::default()
                            }]);
                        }
                        for y in 0..160 { render_scanline(&mut m,y); }
                        render_ffta_zoom_rows(&mut m);
                        let p=m.ffta_dialogue.placement.unwrap();
                        let rw=render_width();
                        let blue:Vec<_>=m.wide_framebuffer[..rw*(160+2*ey)].iter().enumerate()
                            .filter_map(|(i,&c)|(c==0x7c00).then_some(i%rw)).collect();
                        assert_eq!(blue.iter().min().copied(),Some(p.destination_x(x).max(0) as usize));
                        assert_eq!(blue.iter().max().copied(),Some((p.destination_x(x+32).min(rw as i32)-1) as usize));
                        frames.push(m.wide_framebuffer);
                    }
                    assert_eq!(frames[0],frames[1],"host/OAM differ side={right} size={size} zoom={level}");
                }
            }
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn exact_crop_zoom_preserves_uniform_scale_and_length_leaves_text_unchanged() {
        for level in 0..=12 {
            let z=1.122462f32.powi(level);
            let ey=((160.*z-160.)/2.).ceil() as usize;
            let ex=((568.*z-240.)/2.).ceil() as usize;
            crate::mem::set_extra(ex);crate::mem::set_extra_y(ey);
            let mut m=scene(true);
            m.ffta_dialogue.view_zoom=z;
            m.ffta_dialogue.view_width=568.*z;
            m.ffta_dialogue.view_height=160.*z;
            begin_dialogue_frame(&mut m);
            let p=m.ffta_dialogue.placement.unwrap();
            assert!((p.scale/z-1.).abs()<0.00001);
            assert!((p.top-(((160+2*crate::mem::extra_y()) as f32-160.*z)*0.5)).abs()<0.00001);
            m.ffta_dialogue.style.length=1.5;
            begin_dialogue_frame(&mut m);
            let longer=m.ffta_dialogue.placement.unwrap();
            assert_eq!(p.scale,longer.scale);
            assert_eq!(p.source_x(100),longer.source_x(100));
            assert_eq!(longer.length_pixels,60);
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn unrelated_layers_and_unverified_profiles_keep_existing_layout() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(80);
        let mut m=scene(true);
        m.ffta_movement.enabled=false;
        begin_dialogue_frame(&mut m);
        assert!(m.ffta_dialogue.placement.is_none());
        m.ffta_movement.enabled=true;m.io[12]=0x09;
        begin_dialogue_frame(&mut m);
        assert!(m.ffta_dialogue.placement.is_none());
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
}

/// Debug aid for widescreen margins. Set FFTA_WIDE_DEBUG before launching:
///   nospr -> sprites are hidden in the margins
///   nobg  -> backgrounds are hidden in the margins
///   none  -> both hidden (margins show only the backdrop)
fn margin_debug() -> (bool, bool) {
    use std::sync::OnceLock;
    static V: OnceLock<(bool, bool)> = OnceLock::new();
    *V.get_or_init(|| match std::env::var("FFTA_WIDE_DEBUG").as_deref() {
        Ok("nospr") => (true, false),
        Ok("nobg") => (false, true),
        Ok("none") => (true, true),
        _ => (false, false),
    })
}

fn read_io32(mem: &MemMap, off: usize) -> u32 {
    u32::from_le_bytes([
        mem.io[off],
        mem.io[off + 1],
        mem.io[off + 2],
        mem.io[off + 3],
    ])
}

fn sign_extend28(v: u32) -> i32 {
    ((v << 4) as i32) >> 4
}

// ---- text backgrounds ----

/// Read-only layer isolation for the requested movement-grid measurements.
pub fn dump_ffta_text_layers(mem: &MemMap) -> Result<(), String> {
    if !crate::ffta_log::enabled() {
        return Ok(());
    }
    let Ok(prefix) = std::env::var("FFTA_BG_DUMP_PREFIX") else {
        return Ok(());
    };
    if io16(mem, 0) & 7 != 0 {
        return Err("FFTA_BG_DUMP requires text mode 0".into());
    }
    let width = render_width();
    for bg in 0..4 {
        if io16(mem, 0) & (0x100 << bg) == 0 {
            continue;
        }
        let mut ppm = format!("P6\n{width} 160\n255\n").into_bytes();
        for y in 0..160 {
            let mut pixels = [TRANSPARENT; MAX_RENDER_WIDTH];
            render_text_bg(mem, bg, y, &mut pixels);
            for &pixel in &pixels[..width] {
                let pixel = if pixel == TRANSPARENT { 0 } else { pixel };
                let r = (pixel & 31) as u8;
                let g = ((pixel >> 5) & 31) as u8;
                let b = ((pixel >> 10) & 31) as u8;
                ppm.extend_from_slice(&[r << 3 | r >> 2, g << 3 | g >> 2, b << 3 | b >> 2]);
            }
        }
        let path = format!("{prefix}-bg{bg}.ppm");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("{path}: {e}"))?;
        std::io::Write::write_all(&mut file, &ppm).map_err(|e| e.to_string())?;
        let cnt = io16(mem, 8 + bg * 2);
        let (w, h) = match cnt >> 14 {
            0 => (256, 256),
            1 => (512, 256),
            2 => (256, 512),
            _ => (512, 512),
        };
        println!(
            "FFTA_BG_LAYER frame={} bg={bg} cnt=0x{cnt:04X} map=0x{:08X} char=0x{:08X} size={w}x{h} priority={} hofs={} vofs={} path={path}",
            mem.frames,
            0x06000000 + ((cnt as u32 >> 8) & 31) * 0x800,
            0x06000000 + ((cnt as u32 >> 2) & 3) * 0x4000,
            cnt & 3,
            io16(mem, 0x10 + bg * 4) & 511,
            io16(mem, 0x12 + bg * 4) & 511
        );
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) enum FftaBgEdge {
    Wrap,
    Transparent,
    Clamp,
}

impl FftaBgEdge {
    pub(crate) fn for_rom(rom: &[u8]) -> Self {
        if std::env::var("FFTA_PATCH_BG").as_deref() == Ok("0") {
            return Self::Wrap;
        }
        if crate::ffta_rom::version(rom).is_none() {
            return Self::Wrap;
        }
        match std::env::var("FFTA_BG_EDGE").as_deref() {
            Ok("wrap") => Self::Wrap,
            Ok("clamp") => Self::Clamp,
            _ => Self::Transparent,
        }
    }
}

// Require the paired terrain layers, not a single register signature.
// Semantic identification remains a hypothesis until battle visual validation.
pub fn ffta_terrain(mem: &MemMap, bg: usize) -> bool {
    if io16(mem, 0) & 7 != 0 {
        return false;
    }
    for first in [0usize, 2] {
        if bg != first && bg != first + 1 {
            continue;
        }
        if io16(mem, 0) & (3 << (8 + first)) != 3 << (8 + first) {
            continue;
        }
        if io16(mem, 8 + first * 2) != 0x4c03 || io16(mem, 10 + first * 2) != 0x4e02 {
            continue;
        }
        if io16(mem, 0x10 + first * 4) == io16(mem, 0x14 + first * 4)
            && io16(mem, 0x12 + first * 4) == io16(mem, 0x16 + first * 4)
        {
            return true;
        }
    }
    false
}

// Exact-ROM-gated map profile, paired planes rather than an arbitrary BG.
// This is separate from battle terrain: the world map is a finite 256px plane.
fn ffta_world_map(mem:&MemMap,bg:usize)->bool {
    bg<2 && mem.ffta_movement.enabled && io16(mem,0)&0x0307==0x0300
        && io16(mem,8)==0xaa03 && io16(mem,10)==0xac02
}

#[cfg(test)]
mod world_map_edge_tests {
    use super::*;
    #[test]
    fn finite_world_map_margins_preserve_native_pixels_and_other_scenes() {
        for europe in [false,true] {for ey in [0,80] {for hofs in [0u16,504] {
            crate::mem::set_extra_y(ey);
            let mut mem=MemMap::new(vec![]);
            mem.ffta_movement.enabled=true;mem.ffta_movement.europe_profile=europe;
            mem.ffta_bg_edge=FftaBgEdge::Transparent;
            for(o,v)in[(0,0x1f40u16),(8,0xaa03),(10,0xac02),(16,hofs),(20,hofs)] {
                mem.io[o..o+2].copy_from_slice(&v.to_le_bytes());
            }
            mem.vram[32..64].fill(0x11);
            for o in (0x5000..0x7000).step_by(2) {mem.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());}
            mem.palette[2..4].copy_from_slice(&31u16.to_le_bytes());
            for bg in [0,1] {
                let mut native=[TRANSPARENT;MAX_RENDER_WIDTH];
                render_text_bg_hardware(&mem,bg,40,&mut native,0,240);
                let mut wide=[TRANSPARENT;MAX_RENDER_WIDTH];
                render_text_bg_hardware(&mem,bg,40,&mut wide,164,568);
                assert_eq!(&wide[164..404],&native[..240]);
                assert_eq!(wide[0],TRANSPARENT,"left europe={europe} ey={ey} hofs={hofs} bg={bg} finite={}",ffta_world_map(&mem,bg));
                assert_eq!(wide[567],TRANSPARENT,"right europe={europe} ey={ey} hofs={hofs} bg={bg}");
                assert_eq!(wide[404],31); // remaining real map pixels, not a stretched edge
                mem.ffta_bg_edge=FftaBgEdge::Wrap;
                render_text_bg_hardware(&mem,bg,40,&mut wide,164,568);
                if hofs!=0 {assert_eq!(wide[0],31);}
                mem.ffta_bg_edge=FftaBgEdge::Transparent;
            }
            mem.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
            assert!(!ffta_world_map(&mem,0));
        }}}
        crate::mem::set_extra_y(0);
    }
}

// Measured movement overlay: BG0 contains the blue cells and BG1 their
// foreground fragments. Require their paired battle layout and local scroll;
// static UI (HOFS=0), title/cloud layouts and other background assignments fail.
fn ffta_movement_grid(mem: &MemMap, bg: usize) -> bool {
    if bg > 1 || io16(mem, 0) & 0x0f07 != 0x0f00 || !ffta_terrain(mem, 2) {
        return false;
    }
    if io16(mem, 8) != 0x0a0b || io16(mem, 10) != 0x0b0a {
        return false;
    }
    let h = io16(mem, 0x10) & 511;
    let v = io16(mem, 0x12) & 511;
    (8..=15).contains(&h) && h == io16(mem, 0x14) & 511 && v == io16(mem, 0x16) & 511
}

#[cfg(test)]
mod ffta_bg_tests {
    use super::*;
    #[test]
    fn stats_body_extends_toward_edge_and_keeps_cap_and_alpha() {
        let mut m = MemMap::new(vec![0; 192]);
        for o in m.oam.chunks_exact_mut(8) {
            o[..2].copy_from_slice(&0x200u16.to_le_bytes());
        }
        m.vram[0x10000 + 0x208 * 32..0x10000 + 0x228 * 32].fill(0x66);
        m.palette[0x36c..0x36e].copy_from_slice(&0x7c00u16.to_le_bytes());
        let original = m.oam.clone();
        let mut out = [OBJ_EMPTY; MAX_RENDER_WIDTH];
        let mut window = [false; MAX_RENDER_WIDTH];
        for flip in [0, 0x1000] {
            let cap_x = if flip == 0 { 90 } else { 170 };
            m.ffta_sprites.active = Some(vec![
                crate::ffta_sprites::HostSprite {
                    x: 100,
                    y: 10,
                    attrs: [0x440a, 0xc064 | flip, 0xb208],
                    slot: 0,
                    replaces: true,
                    stats_background: true,
                    ..Default::default()
                },
                crate::ffta_sprites::HostSprite {
                    x: cap_x,
                    y: 10,
                    attrs: [0x040a, cap_x as u16, 0xb1f0],
                    slot: 1,
                    replaces: true,
                    stats_background: true,
                    ..Default::default()
                },
            ]);
            out.fill(OBJ_EMPTY);
            render_sprites_extent(&m, 0x1040, 10, &mut out, &mut window, 164, 568);
            let edge = if flip == 0 { 567 } else { 0 };
            assert_eq!(out[edge].color, 0x7c00);
            assert!(out[edge].semi);
            assert_eq!(out[cap_x as usize + 164].priority, 0xff);
            m.ffta_sprites.active.as_mut().unwrap()[0].stats_background = false;
            out.fill(OBJ_EMPTY);
            render_sprites_extent(&m, 0x1040, 10, &mut out, &mut window, 164, 568);
            assert_eq!(out[edge].priority, 0xff);
        }
        out.fill(OBJ_EMPTY);
        render_sprites_extent(&m, 0x1040, 10, &mut out, &mut window, 0, 240);
        assert_eq!(out[239].priority, 0xff);
        assert_eq!(m.oam, original);
    }
    #[test]
    fn stats_backing_keeps_original_transparency_and_preserves_oam() {
        let mut m = MemMap::new(vec![0; 192]);
        for o in m.oam.chunks_exact_mut(8) {
            o[..2].copy_from_slice(&0x200u16.to_le_bytes());
        }
        let attrs = [0x040au16, 10, 0xb001];
        for (i, a) in attrs.iter().enumerate() {
            m.oam[2 * i..2 * i + 2].copy_from_slice(&a.to_le_bytes());
        }
        m.vram[0x10020..0x10040].fill(0x11);
        m.palette[0x362..0x364].copy_from_slice(&0x7c00u16.to_le_bytes());
        m.ffta_sprites.active = Some(vec![crate::ffta_sprites::HostSprite {
            x: 10,
            y: 10,
            attrs,
            slot: 0,
            replaces: true,
            stats_background: true,
            ..Default::default()
        }]);
        let original = m.oam.clone();
        let mut out = [OBJ_EMPTY; MAX_RENDER_WIDTH];
        let mut window = [false; MAX_RENDER_WIDTH];
        render_sprites_extent(&m, 0x1040, 10, &mut out, &mut window, 164, 568);
        assert_eq!(out[174].color, 0x7c00);
        assert!(out[174].semi);
        out.fill(OBJ_EMPTY);
        render_sprites_extent(&m, 0x1040, 10, &mut out, &mut window, 0, 240);
        assert_eq!(out[10].color, 0x7c00);
        assert!(out[10].semi);
        assert_eq!(m.oam, original);
    }
    #[test]
    fn zoom_ui_rows_keep_panel_and_do_not_repeat_outside_map() {
        let mut m = MemMap::new(vec![0; 192]);
        m.io.fill(0);
        m.io[8..10].copy_from_slice(&0x0800u16.to_le_bytes());
        m.vram[..32].fill(0x11);
        m.palette[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        let mut row = [TRANSPARENT; MAX_RENDER_WIDTH];
        m.ffta_movement.zoom_output = true;
        render_text_bg_extent(&m, 0, 180, &mut row, 164, 568);
        assert_eq!(row[164], 0x1234);
        for line in [(-1isize) as usize, 256] {
            render_text_bg_extent(&m, 0, line, &mut row, 164, 568);
            assert!(row[..568].iter().all(|p| *p == TRANSPARENT));
        }
        m.ffta_movement.zoom_output = false;
        render_text_bg_extent(&m, 0, 256, &mut row, 0, 240);
        assert_eq!(row[0], 0x1234);
    }
    #[test]
    fn verified_stage_survives_hidden_partner_and_bg_reassignment() {
        let mut m = MemMap::new(vec![0; 192]);
        m.ffta_stage.enabled = true;
        m.ffta_stage.bases = [Some(0x020091a0), Some(0x0200b1a0)];
        m.ffta_stage.origins = [Some((0, 0)), Some((0, 0))];
        m.ffta_stage.valid = [true, true];
        m.vram[0..32].fill(0x11);
        m.palette[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        // Tile 0 is colored; tile 1 is empty. The host plane extends below
        // the hardware's 32 tile rows, independently of the partner layer.
        m.vram[0x6000..0x8000]
            .chunks_exact_mut(2)
            .for_each(|b| b.copy_from_slice(&1u16.to_le_bytes()));
        for bg in [0, 2] {
            m.io.fill(0);
            m.io[0..2].copy_from_slice(&(0x100u16 << bg).to_le_bytes());
            m.io[8 + bg * 2..10 + bg * 2].copy_from_slice(&0x4c03u16.to_le_bytes());
            assert!(!ffta_terrain(&m, bg));
            let mut wide = [TRANSPARENT; MAX_RENDER_WIDTH];
            render_text_bg_extent(&m, bg, 300, &mut wide, 164, 568);
            assert_eq!(wide[164], 0x1234);
            m.ffta_stage.enabled = false;
            render_text_bg_extent(&m, bg, 300, &mut wide, 164, 568);
            assert_eq!(wide[164], TRANSPARENT);
            // 600 synthetic frames: the original-width path is identical.
            for frame in 0..600 {
                let mut off = [TRANSPARENT; MAX_RENDER_WIDTH];
                let mut on = off;
                render_text_bg_extent(&m, bg, frame % 160, &mut off, 0, 240);
                m.ffta_stage.enabled = true;
                render_text_bg_extent(&m, bg, frame % 160, &mut on, 0, 240);
                assert_eq!(off, on);
                m.ffta_stage.enabled = false;
            }
            m.ffta_stage.enabled = true;
        }
        m.io[8..10].copy_from_slice(&0x0a0bu16.to_le_bytes());
        assert_eq!(crate::ffta_stage::layer(&m, 0), None);
        m.ffta_stage.valid = [false, false];
        assert_eq!(crate::ffta_stage::layer(&m, 2), None);
    }
    #[test]
    fn ffta_movement_edges_preserve_hardware_and_exclude_ui() {
        let mut m = MemMap::new(vec![0; 192]);
        for (off, value) in [
            (0, 0x0f00u16),
            (8, 0x0a0b),
            (10, 0x0b0a),
            (12, 0x4c03),
            (14, 0x4e02),
            (0x10, 8),
            (0x14, 8),
            (0x12, 88),
            (0x16, 88),
            (0x18, 200),
            (0x1c, 200),
            (0x1a, 56),
            (0x1e, 56),
        ] {
            m.io[off..off + 2].copy_from_slice(&value.to_le_bytes());
        }
        m.vram[0x8000..0x8020].fill(0x11);
        m.palette[2..4].copy_from_slice(&0x7c00u16.to_le_bytes());
        assert!(ffta_movement_grid(&m, 0));
        assert!(ffta_movement_grid(&m, 1));
        for bg in 0..2 {
            let mut baseline = [TRANSPARENT; MAX_RENDER_WIDTH];
            m.ffta_bg_edge = FftaBgEdge::Wrap;
            render_text_bg_extent(&m, bg, 0, &mut baseline, 0, 240);
            for edge in [FftaBgEdge::Wrap, FftaBgEdge::Transparent, FftaBgEdge::Clamp] {
                m.ffta_bg_edge = edge;
                let mut hardware = [TRANSPARENT; MAX_RENDER_WIDTH];
                render_text_bg_extent(&m, bg, 0, &mut hardware, 0, 240);
                assert_eq!(hardware, baseline);
                let mut wide = [TRANSPARENT; MAX_RENDER_WIDTH];
                render_text_bg_extent(&m, bg, 0, &mut wide, 164, 568);
                let expected = if edge == FftaBgEdge::Transparent {
                    TRANSPARENT
                } else {
                    0x7c00
                };
                assert_eq!(wide[0], expected);
                assert_eq!(wide[567], expected);
                assert_eq!(wide[164], 0x7c00);
            }
        }
        // In-battle static menu scroll must keep its previous rendering policy.
        m.io[0x10] = 0;
        m.io[0x14] = 0;
        assert!(!ffta_movement_grid(&m, 0));
        // A repeated cloud/title map with the same overlay registers is insufficient.
        m.io[0x10] = 8;
        m.io[0x14] = 8;
        m.io[12] = 0;
        assert!(!ffta_movement_grid(&m, 0));
        m.io[12] = 3;
        m.io[14] = 0;
        assert!(!ffta_movement_grid(&m, 1));
    }
    #[test]
    fn ffta_host_coordinates_unlimited_order_affine_and_semi() {
        use crate::ffta_sprites::HostSprite;
        let mut m = MemMap::new(vec![0; 192]);
        for entry in m.oam.chunks_exact_mut(8) {
            entry[..2].copy_from_slice(&0x200u16.to_le_bytes());
        }
        m.vram[0x10020..0x10040].fill(0x11);
        m.vram[0x10040..0x10060].fill(0x22);
        m.palette[0x202..0x204].copy_from_slice(&0x03e0u16.to_le_bytes());
        m.palette[0x204..0x206].copy_from_slice(&0x001fu16.to_le_bytes());
        // Original affine matrix #0: identity. Host objects use the same matrix words.
        m.oam[6..8].copy_from_slice(&0x100u16.to_le_bytes());
        m.oam[30..32].copy_from_slice(&0x100u16.to_le_bytes());
        let mut objects = vec![
            HostSprite {
                x: -10000,
                y: 20,
                attrs: [20, 0, 1],
                slot: 0,
                replaces: false,
                ..HostSprite::default()
            };
            200
        ];
        objects.push(HostSprite {
            x: 700,
            y: 20,
            attrs: [20 | 0x500, 0, 1],
            slot: 0,
            replaces: false,
            ..HostSprite::default()
        });
        // Same priority, later object: first must win and keep semitransparency.
        objects.push(HostSprite {
            x: 700,
            y: 20,
            attrs: [20, 0, 2],
            slot: 0,
            replaces: false,
            ..HostSprite::default()
        });
        objects.push(HostSprite {
            x: -600,
            y: 20,
            attrs: [20, 0, 1],
            slot: 0,
            replaces: false,
            ..HostSprite::default()
        });
        // Higher priority wins, regardless of insertion order.
        objects.push(HostSprite {
            x: 600,
            y: 20,
            attrs: [20, 0, 0x401],
            slot: 0,
            replaces: false,
            ..HostSprite::default()
        });
        objects.push(HostSprite {
            x: 600,
            y: 20,
            attrs: [20, 0, 2],
            slot: 0,
            replaces: false,
            ..HostSprite::default()
        });
        // Partial submissions can place unlimited host entries past slot 128.
        objects.push(HostSprite {
            x: 800,
            y: 20,
            attrs: [20, 0, 1],
            slot: 200,
            replaces: false,
            ..HostSprite::default()
        });
        m.ffta_sprites.active = Some(objects);
        let mut out = [OBJ_EMPTY; MAX_RENDER_WIDTH];
        let mut window = [false; MAX_RENDER_WIDTH];
        assert!(render_sprites_extent(
            &m,
            0x1040,
            22,
            &mut out,
            &mut window,
            648,
            1536
        ));
        assert_eq!(out[1348].color, 0x03e0);
        assert!(out[1348].semi);
        assert_eq!(out[48].color, 0x03e0);
        assert_eq!(out[1248].color, 0x001f);
        assert_eq!(out[1448].color, 0x03e0);
        let mut hardware = [OBJ_EMPTY; MAX_RENDER_WIDTH];
        assert!(!render_sprites_extent(
            &m,
            0x1040,
            22,
            &mut hardware,
            &mut window,
            0,
            240
        ));
        assert!(hardware[..240].iter().all(|p| p.priority == 0xff));
    }

    fn scene() -> MemMap {
        let mut m = MemMap::new(vec![0; 192]);
        for (off, value) in [
            (0, 0x0300u16),
            (8, 0x4c03),
            (10, 0x4e02),
            (0x10, 17),
            (0x14, 17),
        ] {
            m.io[off..off + 2].copy_from_slice(&value.to_le_bytes());
        }
        m.vram[..32].fill(0x11);
        m.palette[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        m
    }
    #[test]
    fn ffta_bg_wrap_and_zero_extra_preserve_pixels() {
        let mut m = scene();
        let mut baseline = [TRANSPARENT; MAX_RENDER_WIDTH];
        render_text_bg_extent(&m, 0, 0, &mut baseline, 0, 240);
        for policy in [FftaBgEdge::Wrap, FftaBgEdge::Transparent, FftaBgEdge::Clamp] {
            m.ffta_bg_edge = policy;
            let mut actual = [TRANSPARENT; MAX_RENDER_WIDTH];
            render_text_bg_extent(&m, 0, 0, &mut actual, 0, 240);
            assert_eq!(actual, baseline);
        }
    }
    #[test]
    fn ffta_bg_edges_and_unrelated_layers() {
        let mut m = scene();
        assert!(ffta_terrain(&m, 0));
        assert!(!ffta_terrain(&m, 2));
        for (policy, expected) in [
            (FftaBgEdge::Wrap, 0x1234),
            (FftaBgEdge::Clamp, 0x1234),
            (FftaBgEdge::Transparent, TRANSPARENT),
        ] {
            m.ffta_bg_edge = policy;
            let mut pixels = [TRANSPARENT; MAX_RENDER_WIDTH];
            render_text_bg_extent(&m, 0, 0, &mut pixels, 648, 1536);
            assert_eq!(pixels[0], expected);
            assert_eq!(pixels[1535], expected);
            assert_eq!(pixels[648], 0x1234);
        }
        m.io[10] = 0;
        assert!(!ffta_terrain(&m, 0));
        m.ffta_bg_edge = FftaBgEdge::Transparent;
        let mut pixels = [TRANSPARENT; MAX_RENDER_WIDTH];
        render_text_bg_extent(&m, 0, 0, &mut pixels, 164, 568);
        assert_eq!(pixels[0], 0x1234);
        assert_eq!(FftaBgEdge::for_rom(&[0; 192]), FftaBgEdge::Wrap);
    }
}

fn edge_coordinate(value: isize, size: isize, policy: FftaBgEdge) -> Option<usize> {
    match policy {
        FftaBgEdge::Wrap => Some(value.rem_euclid(size) as usize),
        FftaBgEdge::Transparent => (0..size).contains(&value).then_some(value as usize),
        FftaBgEdge::Clamp => Some(value.clamp(0, size - 1) as usize),
    }
}

fn render_text_bg(mem: &MemMap, bg: usize, line: usize, out: &mut Line) {
    render_text_bg_extent(mem, bg, line, out, extra(), render_width());
}

// Immutable row inputs are resolved once; one RAM map entry serves up to 8 pixels.
#[allow(clippy::too_many_arguments)]
fn render_host_tile_row(
    mem: &MemMap,
    line: usize,
    out: &mut Line,
    ex: usize,
    rw: usize,
    hofs: usize,
    vofs: usize,
    char_base: usize,
    base: usize,
    ox: i32,
    oy: i32,
    outside: Option<u16>,
) {
    let y = (line as isize + vofs as isize) as i32;
    let ty = y.div_euclid(8) + oy;
    let py = y.rem_euclid(8) as usize;
    let mut sx = 0;
    while sx < rw {
        let x = sx as i32 - ex as i32 + hofs as i32;
        let px = x.rem_euclid(8) as usize;
        let tx = x.div_euclid(8) + ox;
        let span = (8 - px).min(rw - sx);
        let entry = if (0..64).contains(&tx) && (0..64).contains(&ty) {
            let off = base + 2 * (ty as usize * 64 + tx as usize);
            Some(u16::from_le_bytes([mem.ewram[off], mem.ewram[off + 1]]))
        } else {
            outside
        };
        if let Some(entry) = entry {
            let fy = if entry & 0x0800 != 0 { 7 - py } else { py };
            let tile = char_base + (entry as usize & 1023) * 32 + fy * 4;
            let palette = (entry as usize >> 12) * 16;
            if mem.vram[tile..tile + 4].iter().all(|&byte| byte == 0) {
                out[sx..sx + span].fill(TRANSPARENT);
                sx += span;
                continue;
            }
            for i in 0..span {
                let fx = if entry & 0x0400 != 0 {
                    7 - px - i
                } else {
                    px + i
                };
                let byte = mem.vram[tile + fx / 2];
                let index = (byte >> (4 * (fx % 2))) & 15;
                out[sx + i] = if index == 0 {
                    TRANSPARENT
                } else {
                    pal16(mem, palette + index as usize)
                };
            }
        } else {
            out[sx..sx + span].fill(TRANSPARENT);
        }
        sx += span;
    }
}

fn ffta_action_foreground(mem: &MemMap, bg: usize) -> bool {
    // Measured action HUD: BG2 carries portrait/level lettering and meter
    // outlines; BG3 carries the panel backing. Extending BG2's edge rows
    // replicates the black meter borders across the widescreen margin.
    mem.ffta_movement.enabled
        && mem.ffta_bg_edge != FftaBgEdge::Wrap
        && bg == 2
        && io16(mem, 0x0c) == 0x5c08
        && io16(mem, 0x0e) == 0x4a09
        && io16(mem, 0x18) & 511 == 0
        && io16(mem, 0x1a) & 511 == 0
        && ffta_terrain(mem, 0)
}

fn ffta_dialogue_foreground(mem: &MemMap, bg: usize) -> bool {
    // Measured EUR dialogue: HOFS=6 offsets the text plane; BG3 is its backing.
    mem.ffta_movement.enabled
        && mem.ffta_movement.europe_profile
        && mem.ffta_bg_edge != FftaBgEdge::Wrap
        && bg == 2
        && io16(mem, 0x0c) == 0x0b08
        && io16(mem, 0x0e) == 0x0a08
        && ffta_terrain(mem, 0)
}

fn render_text_bg_extent(
    mem: &MemMap,
    bg: usize,
    line: usize,
    out: &mut Line,
    ex: usize,
    rw: usize,
) {
    if mem.ffta_dialogue.wide_pass && bg >= 2 {
        if let Some(placement) = mem.ffta_dialogue.placement {
            if mem.ffta_dialogue.separate { out[..rw].fill(TRANSPARENT); return; }
            let y = placement.source_y(line as i32);
            out[..rw].fill(TRANSPARENT);
            if (0..160).contains(&y) {
                let mut native = [TRANSPARENT;MAX_RENDER_WIDTH];
                render_text_bg_hardware(mem,bg,y as usize,&mut native,0,240);
                for (x,color) in out[..rw].iter_mut().enumerate() {
                    let mut source = placement.source_x(x as i32);
                    // Length repeats only the flat backing interior. Text and
                    // portrait keep the same uniform scale and curved end cap.
                    if bg == 3 && placement.length_pixels > 0 {
                        source += if placement.right { placement.length_pixels } else { -placement.length_pixels };
                        if placement.right && source >= 200 && source < 240+placement.length_pixels { source=200; }
                        if !placement.right && source < 40 && source >= -placement.length_pixels { source=39; }
                    }
                    if (0..240).contains(&source) { *color = native[source as usize]; }
                }
            }
            return;
        }
    }
    render_text_bg_hardware(mem,bg,line,out,ex,rw);
}

fn render_text_bg_hardware(
    mem: &MemMap, bg: usize, line: usize, out: &mut Line, ex: usize, rw: usize,
) {
    let cnt = io16(mem, 0x08 + bg * 2);
    let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
    let eight_bpp = cnt & 0x80 != 0;
    let screen_base = ((cnt >> 8) & 0x1F) as usize * 0x800;
    let size = (cnt >> 14) & 3;

    let hofs = (io16(mem, 0x10 + bg * 4) & 0x1FF) as usize;
    let finite_world = ex>0 && mem.ffta_bg_edge!=FftaBgEdge::Wrap && ffta_world_map(mem,bg);
    let vofs = (io16(mem, 0x12 + bg * 4) & 0x1FF) as usize;
    let dialogue_foreground = ffta_dialogue_foreground(mem, bg);
    if dialogue_foreground && !(0..crate::mem::VISIBLE_HEIGHT).contains(&line) {
        out[..rw].fill(TRANSPARENT);
        return;
    }

    if ex > 0 {
        if let Some(index) = crate::ffta_stage::layer(mem, bg) {
            let (ox, oy) = mem.ffta_stage.origins[index].unwrap();
            let base = (mem.ffta_stage.bases[index].unwrap() - 0x02000000) as usize;
            render_host_tile_row(
                mem, line, out, ex, rw, hofs, vofs, char_base, base, ox, oy, None,
            );
            return;
        }
        if ffta_movement_grid(mem, bg) && mem.ffta_movement.enabled && mem.ffta_movement.valid[bg] {
            if let Some((ox, oy)) = mem.ffta_movement.origins[bg] {
                let base = if bg == 0 { 0x5cb0 } else { 0x3cb0 };
                render_host_tile_row(
                    mem,
                    line,
                    out,
                    ex,
                    rw,
                    hofs,
                    vofs,
                    char_base,
                    base,
                    ox,
                    oy,
                    Some(0),
                );
                return;
            }
            // Preserve entry(...).unwrap_or(0) if an invalid origin is observed.
            render_host_tile_row(
                mem,
                line,
                out,
                ex,
                rw,
                hofs,
                vofs,
                char_base,
                0,
                0,
                -10000,
                Some(0),
            );
            return;
        }
    }

    let (w_tiles, h_tiles) = match size {
        0 => (32, 32),
        1 => (64, 32),
        2 => (32, 64),
        _ => (64, 64),
    };

    // European battle and cutscene planes repeat the 240px viewport in their
    // tile map. Treat zoom margins as finite map edges; verified terrain and
    // movement planes are reconstructed from source RAM above.
    let eur_zoom_margin =
        mem.ffta_movement.europe_profile && crate::mem::extra_y() > 0 && hofs != 0
            && !ffta_world_map(mem,bg);
    let policy = if !dialogue_foreground
        && ex > 0
        && (ffta_terrain(mem, bg) || ffta_movement_grid(mem, bg) || eur_zoom_margin)
    {
        mem.ffta_bg_edge
    } else {
        FftaBgEdge::Wrap
    };
    // Extend menu panels only within their real tile plane. Do not repeat
    // a static UI into the rows above/below it when zooming out.
    let vertical_policy = if mem.ffta_movement.zoom_output
        && !ffta_terrain(mem, bg)
        && !ffta_movement_grid(mem, bg)
    {
        FftaBgEdge::Transparent
    } else {
        policy
    };
    let Some(y) = edge_coordinate(
        line as isize + vofs as isize,
        (h_tiles * 8) as isize,
        vertical_policy,
    ) else {
        out[..rw].fill(TRANSPARENT);
        return;
    };
    let ty = y / 8;
    let py = y % 8;
    let map_w = (w_tiles * 8) as isize;

    // Walk the line in tile spans: all pixels within one span share the
    // same map entry, so fetch and decode it once. The walk covers the
    // wide buffer; margin columns read the map at negative / past-240
    // screen positions (wrapping at the map width, like the hardware).
    let mut sx = 0usize;
    while sx < rw {
        let screen_x = sx as isize - ex as isize;
        let in_margin = screen_x < 0 || screen_x >= VISIBLE_WIDTH as isize;
        let (sample_x,horizontal_policy)=if finite_world && in_margin {
            let signed_hofs=if hofs>=256 {hofs as isize-512}else{hofs as isize};
            (screen_x+signed_hofs,FftaBgEdge::Transparent)
        }else{(screen_x+hofs as isize,policy)};
        let Some(x) = edge_coordinate(sample_x, map_w, horizontal_policy) else {
            out[sx] = TRANSPARENT;
            sx += 1;
            continue;
        };
        let tx = x / 8;
        let px = x % 8;
        let span =
            if horizontal_policy != FftaBgEdge::Clamp || (0..map_w).contains(&sample_x) {
                (8 - px).min(rw - sx)
            } else {
                1
            };

        // Offset text tiles can straddle the viewport edge. Split them there
        // so clearing a margin never clears pixels in the native viewport.
        let span = if screen_x < 0 {
            span.min((-screen_x) as usize)
        } else if screen_x < VISIBLE_WIDTH as isize {
            span.min(VISIBLE_WIDTH - screen_x as usize)
        } else {
            span
        };

        // Static (unscrolled) layers such as dialog boxes and HUD were built
        // for exactly 240 px. Their map holds leftovers past column 240 and
        // wrapping would paste the right end of the map into the left
        // margin, so leave the margins empty for them. Scrolling layers
        // (hofs != 0) keep real map content and still read through.
        if in_margin
            && (dialogue_foreground
                || margin_debug().1
                || (hofs == 0 && policy == FftaBgEdge::Wrap && !finite_world))
        {
            for i in 0..span {
                out[sx + i] = TRANSPARENT;
            }
            sx += span;
            continue;
        }

        // Screenblock layout: 32x32-tile blocks, left-to-right then down.
        let block = (ty / 32) * (w_tiles / 32) + tx / 32;
        let entry_idx = block * 1024 + (ty % 32) * 32 + (tx % 32);
        let entry_off = screen_base + entry_idx * 2;
        let entry = u16::from_le_bytes([mem.vram[entry_off], mem.vram[entry_off + 1]]);

        let tile = (entry & 0x3FF) as usize;
        let hflip = entry & 0x400 != 0;
        let vflip = entry & 0x800 != 0;
        let fy = if vflip { 7 - py } else { py };

        if eight_bpp {
            let base = char_base + tile * 64 + fy * 8;
            for i in 0..span {
                let fx = if hflip { 7 - (px + i) } else { px + i };
                let addr = base + fx;
                let color_idx = if addr >= 0x1_0000 {
                    0
                } else {
                    mem.vram[addr] as usize
                };
                out[sx + i] = if color_idx == 0 {
                    TRANSPARENT
                } else {
                    pal16(mem, color_idx)
                };
            }
        } else {
            let base = char_base + tile * 32 + fy * 4;
            let pal_base = ((entry >> 12) as usize & 0xF) * 16;
            for i in 0..span {
                let fx = if hflip { 7 - (px + i) } else { px + i };
                let addr = base + fx / 2;
                let color_idx = if addr >= 0x1_0000 {
                    0
                } else {
                    let b = mem.vram[addr];
                    let nib = if fx & 1 == 0 { b & 0xF } else { b >> 4 } as usize;
                    if nib == 0 { 0 } else { pal_base + nib }
                };
                out[sx + i] = if color_idx == 0 {
                    TRANSPARENT
                } else {
                    pal16(mem, color_idx)
                };
            }
        }
        sx += span;
    }

    // Static UI layers: stretch the edge columns into the margins so bands,
    // panels and menu boxes reach the edges of the wide frame instead of
    // ending 22 px short. Content stays where the game put it; only the
    // empty margin is filled, and only where the edge pixel is opaque.
    if hofs == 0
        && ex > 0
        && !finite_world
        && !margin_debug().1
        && policy == FftaBgEdge::Wrap
        && !ffta_action_foreground(mem, bg)
        && !dialogue_foreground
    {
        let l = ex;
        let r = ex + VISIBLE_WIDTH - 1;
        let (cl, cr) = (out[l], out[r]);
        if cl != TRANSPARENT {
            for i in 0..l {
                out[i] = cl;
            }
        }
        if cr != TRANSPARENT {
            for i in (r + 1)..rw {
                out[i] = cr;
            }
        }
    }
}

// ---- affine backgrounds ----

fn render_affine_bg(mem: &MemMap, bg: usize, out: &mut Line) {
    let cnt = io16(mem, 0x08 + bg * 2);
    let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
    let screen_base = ((cnt >> 8) & 0x1F) as usize * 0x800;
    let wrap = cnt & 0x2000 != 0;
    let size_tiles = 16usize << ((cnt >> 14) & 3); // 16,32,64,128
    let size_px = (size_tiles * 8) as i32;

    let (pa, pc, ref_base) = if bg == 2 {
        (
            io16(mem, 0x20) as i16 as i32,
            io16(mem, 0x24) as i16 as i32,
            0,
        )
    } else {
        (
            io16(mem, 0x30) as i16 as i32,
            io16(mem, 0x34) as i16 as i32,
            2,
        )
    };
    // The buffer starts EXTRA columns left of screen column 0, so back the
    // reference point up by EXTRA steps of (PA, PC).
    let mut fx = mem.aff_ref[ref_base].wrapping_sub(pa.wrapping_mul(extra() as i32));
    let mut fy = mem.aff_ref[ref_base + 1].wrapping_sub(pc.wrapping_mul(extra() as i32));

    for out_px in out[..render_width()].iter_mut() {
        let mut tx = fx >> 8;
        let mut ty = fy >> 8;
        fx = fx.wrapping_add(pa);
        fy = fy.wrapping_add(pc);

        if wrap {
            tx = tx.rem_euclid(size_px);
            ty = ty.rem_euclid(size_px);
        } else if tx < 0 || ty < 0 || tx >= size_px || ty >= size_px {
            *out_px = TRANSPARENT;
            continue;
        }
        let (tx, ty) = (tx as usize, ty as usize);

        let entry_off = screen_base + (ty / 8) * size_tiles + tx / 8;
        let tile = mem.vram[entry_off] as usize;
        let addr = char_base + tile * 64 + (ty % 8) * 8 + tx % 8;
        let color_idx = if addr < 0x1_0000 {
            mem.vram[addr] as usize
        } else {
            0
        };
        *out_px = if color_idx == 0 {
            TRANSPARENT
        } else {
            pal16(mem, color_idx)
        };
    }
}

// ---- bitmap backgrounds ----

fn render_bitmap_bg(mem: &MemMap, mode: u16, dispcnt: u16, line: usize, out: &mut Line) {
    let page = if dispcnt & 0x10 != 0 { 0xA000usize } else { 0 };
    // Bitmap modes only exist on the 240-px screen; margins stay transparent.
    match mode {
        3 => {
            for (i, out_px) in out[..render_width()].iter_mut().enumerate() {
                let Some(x) = visible_col(i) else {
                    *out_px = TRANSPARENT;
                    continue;
                };
                let off = (line * VISIBLE_WIDTH + x) * 2;
                let c = u16::from_le_bytes([mem.vram[off], mem.vram[off + 1]]) & 0x7FFF;
                *out_px = c;
            }
        }
        4 => {
            for (i, out_px) in out[..render_width()].iter_mut().enumerate() {
                let Some(x) = visible_col(i) else {
                    *out_px = TRANSPARENT;
                    continue;
                };
                let idx = mem.vram[page + line * VISIBLE_WIDTH + x] as usize;
                *out_px = if idx == 0 {
                    TRANSPARENT
                } else {
                    pal16(mem, idx)
                };
            }
        }
        _ => {
            // Mode 5: 160x128 16bpp, paged.
            for (i, out_px) in out[..render_width()].iter_mut().enumerate() {
                match visible_col(i) {
                    Some(x) if x < 160 && line < 128 => {
                        let off = page + (line * 160 + x) * 2;
                        let c = u16::from_le_bytes([mem.vram[off], mem.vram[off + 1]]) & 0x7FFF;
                        *out_px = c;
                    }
                    _ => *out_px = TRANSPARENT,
                }
            }
        }
    }
}

// ---- sprites ----

#[cfg(test)]
mod android_zoom_regressions {
    use super::*;

    #[test]
    fn offset_dialogue_text_never_repeats_in_margins_at_x1_or_zoom_out() {
        let mut mem = MemMap::new(vec![]);
        mem.ffta_movement.enabled = true;
        mem.ffta_movement.europe_profile = true;
        mem.ffta_bg_edge = FftaBgEdge::Transparent;
        mem.io[..2].copy_from_slice(&0x0f00u16.to_le_bytes());
        for (bg, cnt) in [0x4c03u16, 0x4e02, 0x0b08, 0x0a08].into_iter().enumerate() {
            mem.io[8 + bg * 2..10 + bg * 2].copy_from_slice(&cnt.to_le_bytes());
        }
        mem.vram[0x8000..0x8020].fill(0x21);
        mem.palette[2..4].copy_from_slice(&31u16.to_le_bytes());
        mem.palette[4..6].copy_from_slice(&0x7c00u16.to_le_bytes());
        for ey in [0, 80] {
            crate::mem::set_extra_y(ey);
            for hofs in [6u16, 506] {
                mem.io[0x18..0x1a].copy_from_slice(&hofs.to_le_bytes());
                let mut hardware = [TRANSPARENT; MAX_RENDER_WIDTH];
                let mut wide = [TRANSPARENT; MAX_RENDER_WIDTH];
                render_text_bg_extent(&mem, 2, 20, &mut hardware, 0, 240);
                render_text_bg_extent(&mem, 2, 20, &mut wide, 164, 568);
                assert_eq!(&wide[164..404], &hardware[..240], "ey={ey} hofs={hofs}");
                assert!(
                    wide[..164]
                        .iter()
                        .chain(&wide[404..568])
                        .all(|&p| p == TRANSPARENT)
                );
            }
        }
        crate::mem::set_extra_y(0);
        mem.ffta_movement.enabled = false;
        let mut original = [TRANSPARENT; MAX_RENDER_WIDTH];
        render_text_bg_extent(&mem, 2, 20, &mut original, 164, 568);
        assert!(original[..164].iter().any(|&p| p != TRANSPARENT));
    }

    #[test]
    fn action_hud_keeps_meter_inside_view_and_extends_only_backing() {
        let mut mem = MemMap::new(vec![]);
        mem.ffta_movement.enabled = true;
        mem.ffta_bg_edge = FftaBgEdge::Transparent;
        mem.io[..2].copy_from_slice(&0x0f00u16.to_le_bytes());
        for (bg, cnt) in [0x4c03u16, 0x4e02, 0x5c08, 0x4a09].into_iter().enumerate() {
            mem.io[8 + bg * 2..10 + bg * 2].copy_from_slice(&cnt.to_le_bytes());
        }
        mem.vram[0x8000..0x8020].fill(0x11);
        for entry in mem.vram[0x5000..0x6000].chunks_exact_mut(2) {
            entry.copy_from_slice(&0x1000u16.to_le_bytes());
        }
        mem.palette[34..36].copy_from_slice(&0x7c00u16.to_le_bytes());
        let mut foreground = [TRANSPARENT; MAX_RENDER_WIDTH];
        let mut backing = [TRANSPARENT; MAX_RENDER_WIDTH];
        render_text_bg_extent(&mem, 2, 152, &mut foreground, 22, 284);
        render_text_bg_extent(&mem, 3, 152, &mut backing, 22, 284);
        assert!(foreground[..22].iter().all(|&p| p == TRANSPARENT));
        assert!(foreground[22..262].iter().all(|&p| p == 0));
        assert!(backing[..284].iter().all(|&p| p == 0x7c00));
        let mut hardware = [TRANSPARENT; MAX_RENDER_WIDTH];
        render_text_bg_extent(&mem, 2, 152, &mut hardware, 0, 240);
        assert_eq!(&foreground[22..262], &hardware[..240]);
        mem.ffta_movement.enabled = false;
        render_text_bg_extent(&mem, 2, 152, &mut foreground, 22, 284);
        assert_eq!(foreground[0], 0);
    }

    #[test]
    fn parked_guest_oam_is_hidden_but_verified_host_piece_survives() {
        crate::mem::set_extra(136);
        crate::mem::set_extra_y(176);
        let mut mem = MemMap::new(vec![]);
        for o in mem.oam.chunks_exact_mut(8) {
            o[..2].copy_from_slice(&0x0200u16.to_le_bytes());
        }
        for (i, attr) in [0x00a8u16, 0x00f8, 0].iter().enumerate() {
            mem.oam[127 * 8 + i * 2..127 * 8 + i * 2 + 2].copy_from_slice(&attr.to_le_bytes());
        }
        mem.vram[0x10000..0x10020].fill(0x11);
        mem.palette[0x202..0x204].copy_from_slice(&31u16.to_le_bytes());
        mem.ffta_sprites.enabled = true;
        let mut objects = [OBJ_EMPTY; MAX_RENDER_WIDTH];
        let mut window = [false; MAX_RENDER_WIDTH];
        render_sprites(&mem, 0x1040, (-88i32) as usize, &mut objects, &mut window);
        assert!(objects[..512].iter().all(|o| o.color == TRANSPARENT));
        mem.ffta_sprites.active = Some(vec![crate::ffta_sprites::HostSprite {
            attrs: [0x00a8, 0x00f8, 0],
            x: 248,
            y: -88,
            slot: 127,
            replaces: true,
            ..Default::default()
        }]);
        render_sprites(&mem, 0x1040, (-88i32) as usize, &mut objects, &mut window);
        assert_eq!(objects[136 + 248].color, 31);
        crate::mem::set_extra(0);
        crate::mem::set_extra_y(0);
    }
}

const OBJ_SIZES: [[(i32, i32); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)], // square
    [(16, 8), (32, 8), (32, 16), (64, 32)], // horizontal
    [(8, 16), (8, 32), (16, 32), (32, 64)], // vertical
];

/// Returns true if any semi-transparent sprite pixel was written
/// (conservative: a later overwrite by an opaque sprite still counts).
fn render_sprites(
    mem: &MemMap,
    dispcnt: u16,
    line: usize,
    out: &mut [ObjPixel; MAX_RENDER_WIDTH],
    window: &mut [bool; MAX_RENDER_WIDTH],
) -> bool {
    render_sprites_extent(mem, dispcnt, line, out, window, extra(), render_width())
}

fn render_sprites_extent(
    mem: &MemMap,
    dispcnt: u16,
    line: usize,
    out: &mut [ObjPixel; MAX_RENDER_WIDTH],
    window: &mut [bool; MAX_RENDER_WIDTH],
    ex: usize,
    rw: usize,
) -> bool {
    let mut any_semi = false;
    let one_dim = dispcnt & 0x40 != 0;
    let bitmap_mode = (dispcnt & 7) >= 3;
    let line = line as i32;

    let host = if ex > 0 || crate::mem::extra_y() > 0 {
        mem.ffta_sprites.active.as_deref().unwrap_or(&[])
    } else {
        &[]
    };
    // Published host entries are stably sorted. Merge in linear time without
    // scanline allocations; guest OAM and affine words remain live reads.
    let (mut slot, mut cursor, mut replaced) = (0usize, 0usize, false);
    let expanded = std::iter::from_fn(|| {
        loop {
            if cursor < host.len() && (host[cursor].slot <= slot || slot >= 128) {
                let sprite = &host[cursor];
                cursor += 1;
                replaced |= sprite.replaces;
                return Some((
                    sprite.attrs,
                    Some((sprite.x, sprite.y)),
                    sprite.owner,
                    sprite.stats_background,
                    sprite.stats_inner_padding,
                ));
            }
            if slot >= 128 {
                return None;
            }
            let i = slot;
            slot += 1;
            let emit = !replaced;
            replaced = false;
            if emit {
                let p = &mem.oam[i * 8..i * 8 + 6];
                return Some((
                    [
                        u16::from_le_bytes([p[0], p[1]]),
                        u16::from_le_bytes([p[2], p[3]]),
                        u16::from_le_bytes([p[4], p[5]]),
                    ],
                    None,
                    mem.ffta_sprites.oam_owners.get(i).copied().unwrap_or(0),
                    false,
                    false,
                ));
            }
        }
    });
    for ([a0, a1, a2], coordinates, owner, stats_background, stats_inner_padding) in expanded {
        // Unused FFTA OAM slots park offscreen at (248,168). Vertical zoom
        // would unwrap that Y coordinate and expose tile zero in the sky.
        // Verified host objects carry full coordinates and are not filtered.
        if coordinates.is_none()
            && mem.ffta_sprites.enabled
            && (ex > 0 || crate::mem::extra_y() > 0)
            && [a0, a1, a2] == [0x00a8, 0x00f8, 0]
        {
            continue;
        }

        let affine = a0 & 0x100 != 0;
        if !affine && a0 & 0x200 != 0 {
            continue; // disabled
        }
        let shape = ((a0 >> 14) & 3) as usize;
        if shape == 3 {
            continue; // prohibited
        }
        let size_idx = ((a1 >> 14) & 3) as usize;
        let (w, h) = OBJ_SIZES[shape][size_idx];
        let double = affine && a0 & 0x200 != 0;
        let (bw, bh) = if double { (w * 2, h * 2) } else { (w, h) };

        let mut y = (a0 & 0xFF) as i32;
        if y >= 160 {
            y -= 256;
        }
        let mut x = (a1 & 0x1FF) as i32;
        // 9-bit X wraps. Values past the right margin read as negative
        // (hardware: >= 240). With EXTRA > 0, a sprite parked in the
        // 240..240+EXTRA range will show in the right margin.
        if x >= (VISIBLE_WIDTH + ex) as i32 {
            x -= 512;
        }
        if let Some((host_x, host_y)) = coordinates {
            x = host_x;
            y = host_y;
        }
        let bounded=(-64..240).contains(&x) && (-64..160).contains(&y);
        let notice_object=mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice
            && (crate::ffta_dialogue::notice_cursor([a0,a1,a2],x,y,mem.ffta_dialogue.extra.bounds)
                || (mem.ffta_dialogue.laws_layout && crate::ffta_dialogue::laws_cursor([a0,a1,a2],x,y,mem.ffta_dialogue.extra.bounds)));
        let detached_cursor=mem.ffta_dialogue.town_layout && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Actions
            && crate::ffta_dialogue::notice_cursor([a0,a1,a2],x,y,mem.ffta_dialogue.extra.bounds);
        let main_object=bounded && !notice_object && !detached_cursor && if mem.ffta_dialogue.action_layout {
            crate::ffta_dialogue::action_portrait([a0,a1,a2],x,y)
        } else if mem.ffta_dialogue.town_layout || mem.ffta_dialogue.shop_layout {
            crate::ffta_dialogue::portrait([a0,a1,a2],x)
                || crate::ffta_dialogue::continuation([a0,a1,a2])
                || (mem.ffta_dialogue.shop_layout && a2&0x0c00==0 && (0x120..=0x12f).contains(&(a2&1023)))
                || (a1&0xfe00==0 && ((a0&0xff00==0x4000 && (0x120..=0x123).contains(&(a2&1023)))
                || (a0&0xff00==0 && (0x124..=0x127).contains(&(a2&1023)))))
        } else {crate::ffta_dialogue::portrait([a0,a1,a2],x) || crate::ffta_dialogue::continuation([a0,a1,a2])};
        let extra_object=bounded && match mem.ffta_dialogue.extra.kind {
            crate::ffta_dialogue::PanelKind::Notice=>notice_object,
            crate::ffta_dialogue::PanelKind::Stats=>mem.ffta_dialogue.extra.contains_stats([a0,a1,a2],x,y,bw,bh),
            crate::ffta_dialogue::PanelKind::Actions=>if mem.ffta_dialogue.town_layout {
                a1&0xfe00==0 && ((a0&0xff00==0x4000 && (0x120..=0x123).contains(&(a2&1023)))
                    || (a0&0xff00==0 && (0x124..=0x127).contains(&(a2&1023))))
            }else{crate::ffta_dialogue::notice_cursor([a0,a1,a2],x,y,mem.ffta_dialogue.extra.bounds.map(|[l,t,r,b]|[l-8,t,r,b]))},
            _=>false,
        };
        if mem.ffta_dialogue.capture && !(if mem.ffta_dialogue.capture_extra {extra_object}else{main_object}) {continue;}
        if mem.ffta_dialogue.capture && mem.ffta_dialogue.capture_backing
            && !(mem.ffta_dialogue.capture_extra && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats && a2>>12==11 && matches!(a2&1023,0x1f0|0x200|0x208)) {continue;}
        if mem.ffta_dialogue.separate && mem.ffta_dialogue.wide_pass && (main_object && mem.ffta_dialogue.placement.is_some() || extra_object) {continue;}
        let dialogue=if mem.ffta_dialogue.wide_pass && main_object {mem.ffta_dialogue.placement}else{None};
        let sample_line = dialogue.map_or(line,|p|p.source_y(line));
        if sample_line < y || sample_line >= y + bh {
            continue;
        }

        let mode = (a0 >> 10) & 3;
        let eight_bpp = a0 & 0x2000 != 0;
        let tile_base = (a2 & 0x3FF) as usize;
        let priority = ((a2 >> 10) & 3) as u8;
        let palette = ((a2 >> 12) & 0xF) as usize;
        // The game uploads temporary snowball pieces before the matching OBJ
        // palette bank. A host sprite may reach the widened area at that point.
        // Check this piece's own bank; a nonempty palette elsewhere is no proof.
        if ex > 0 && !eight_bpp {
            let colors = &mem.palette[0x200 + palette * 32 + 2..0x200 + palette * 32 + 32];
            if colors.iter().all(|&byte| byte == 0) {
                continue;
            }
        }

        // Affine parameters.
        let (pa, pb, pc, pd) = if affine {
            let g = ((a1 >> 9) & 0x1F) as usize * 32;
            let p = |off: usize| {
                u16::from_le_bytes([mem.oam[g + off], mem.oam[g + off + 1]]) as i16 as i32
            };
            (p(6), p(14), p(22), p(30))
        } else {
            (0x100, 0, 0, 0x100)
        };

        let row_tiles = if one_dim {
            (w / 8) * if eight_bpp { 2 } else { 1 }
        } else {
            32
        };

        let ly = sample_line - y;
        // Stretch only the rectangular half toward its screen edge. The
        // curved cap and the text keep their original geometry and alpha.
        let stats_body = stats_background && tile_base == 0x208 && !affine && !eight_bpp;
        let inner_padding = if stats_inner_padding { 6 } else { 0 };
        let (start, end) = if mem.ffta_dialogue.capture {
            // Extract the original curved cap; legacy host gap padding is applied
            // only by the old world-UI path, never inside the native layer.
            (0,bw)
        } else if let Some(p) = dialogue {
            (p.destination_x(x).max(0),p.destination_x(x+bw).min(rw as i32))
        } else if stats_body && a1 & 0x1000 != 0 {
            (-((ex as i32) + x).max(0), bw + inner_padding)
        } else if stats_body {
            (-inner_padding, (rw as i32 - ex as i32 - x).max(bw))
        } else {
            (0, bw)
        };
        for sample in start..end {
            let (bx,sx) = if let Some(p) = dialogue {
                (p.source_x(sample)-x,sample-ex as i32)
            } else {(sample,x+sample)};
            // Buffer index = screen column + EXTRA (margins included).
            let idx = sx + ex as i32;
            if !(0..rw as i32).contains(&idx) {
                continue;
            }
            if margin_debug().0 && (sx < 0 || sx >= VISIBLE_WIDTH as i32) {
                continue;
            }
            let sx_usize = idx as usize;

            // Texture coordinates.
            let (u, v) = if affine {
                let dx = bx - bw / 2;
                let dy = ly - bh / 2;
                let u = ((pa * dx + pb * dy) >> 8) + w / 2;
                let v = ((pc * dx + pd * dy) >> 8) + h / 2;
                if u < 0 || v < 0 || u >= w || v >= h {
                    continue;
                }
                (u, v)
            } else {
                let mut u = bx.clamp(0, w - 1);
                let mut v = ly;
                // Extend the flat panel interior instead of repeating its
                // dark outline into widescreen margins.
                if stats_body && ex > 0 && (bx < 0 || bx >= bw) {
                    u = w / 2;
                }
                // OBJ attr1 flips: bit 12 horizontal, bit 13 vertical
                // (unlike BG map entries, where flips are bits 10/11).
                if a1 & 0x1000 != 0 {
                    u = w - 1 - u;
                }
                if a1 & 0x2000 != 0 {
                    v = h - 1 - v;
                }
                (u, v)
            };

            let (tu, pu) = (u / 8, u % 8);
            let (tv, pv) = (v / 8, v % 8);
            let tile_step = if eight_bpp { 2 } else { 1 };
            let tile = tile_base + tv as usize * row_tiles as usize + tu as usize * tile_step;
            // In bitmap modes the first 512 tiles overlap the bitmap.
            if bitmap_mode && tile < 512 {
                continue;
            }
            let addr = 0x1_0000
                + if eight_bpp {
                    (tile / 2) * 64 + pv as usize * 8 + pu as usize
                } else {
                    tile * 32 + pv as usize * 4 + pu as usize / 2
                };
            if addr >= 0x1_8000 {
                continue;
            }
            let color_idx = if eight_bpp {
                mem.vram[addr] as usize
            } else {
                let b = mem.vram[addr];
                let nib = if pu & 1 == 0 { b & 0xF } else { b >> 4 } as usize;
                if nib == 0 { 0 } else { palette * 16 + nib }
            };
            if color_idx == 0 {
                continue;
            }
            let color = pal16(mem, 256 + color_idx);

            if mode == 2 {
                window[sx_usize] = true;
            } else if out[sx_usize].priority == 0xFF || priority < out[sx_usize].priority {
                out[sx_usize] = ObjPixel {
                    owner,
                    color,
                    priority,
                    semi: mode == 1,
                };
                any_semi |= mode == 1;
            }
        }
    }
    any_semi
}

// ---- composition ----

/// Layer ids for blend targeting: 0-3 BGs, 4 OBJ, 5 backdrop.
#[allow(clippy::too_many_arguments)]
fn compose(
    mem: &mut MemMap,
    dispcnt: u16,
    line: usize,
    bg_lines: &[Line; 4],
    bg_used: &[bool; 4],
    obj_line: &[ObjPixel; MAX_RENDER_WIDTH],
    obj_window: &[bool; MAX_RENDER_WIDTH],
    line_has_semi: bool,
) {
    let rw = render_width();
    let ex = extra();
    let mut output = [0u16; MAX_RENDER_WIDTH];
    let mut counts = std::collections::BTreeMap::new();
    compose_into(
        mem,
        dispcnt,
        line,
        bg_lines,
        bg_used,
        obj_line,
        obj_window,
        line_has_semi,
        &mut output[..rw],
        &mut counts,
    );
    let zoom = mem.ffta_movement.zoom_output;
    let row = if zoom {
        (line as isize + crate::mem::extra_y() as isize) as usize
    } else {
        line
    };
    mem.wide_framebuffer[row * rw..(row + 1) * rw].copy_from_slice(&output[..rw]);
    if mem.ffta_dialogue.wide_pass && mem.ffta_dialogue.extra.kind == crate::ffta_dialogue::PanelKind::Stats {
        let mut raw=[0u16;MAX_RENDER_WIDTH];
        compose_background_raw(mem,dispcnt,line,bg_lines,bg_used,obj_window,&mut raw[..rw]);
        mem.ffta_dialogue.extra.background[row*rw..(row+1)*rw].copy_from_slice(&raw[..rw]);
    }
    if !zoom && !mem.ffta_dialogue.wide_pass {
        mem.framebuffer[line * VISIBLE_WIDTH..(line + 1) * VISIBLE_WIDTH]
            .copy_from_slice(&output[ex..ex + VISIBLE_WIDTH]);
    }
    if (mem.ffta_dialogue.placement.is_none() && !mem.ffta_dialogue.extra.active()) || mem.ffta_dialogue.wide_pass {
        for (owner, count) in counts {
            *mem.ffta_sprites.pixel_counts.entry(owner).or_default() += count;
        }
    }
}

// A semitransparent OBJ blends against the first raw BG, never another OBJ
// or a BG which has already received its own color effect.
fn compose_background_raw(mem:&MemMap,dispcnt:u16,line:usize,bg_lines:&[Line;4],used:&[bool;4],obj_window:&[bool;MAX_RENDER_WIDTH],out:&mut[u16]) {
    let backdrop=if mem.ffta_movement.zoom_output {let [a,b]=mem.ffta_movement.sky_edges;extend_sky(a,b,line as i32)}else{pal16(mem,0)};
    let mut order=[0,1,2,3];order.sort_unstable_by_key(|&bg|(io16(mem,8+bg*2)&3,bg));
    let wy=if mem.ffta_movement.zoom_output {(line as isize).clamp(0,159) as u8}else{line as u8};
    for (x,pixel) in out.iter_mut().enumerate() {
        let wx=(x as isize-extra() as isize).clamp(0,239) as u8;
        let inside=|h:usize,v:usize| {let a=io16(mem,h);let b=io16(mem,v);in_window_range((a>>8)as u8,a as u8,wx)&&in_window_range((b>>8)as u8,b as u8,wy)};
        let control=if dispcnt&0xe000==0 {0x3f}else if dispcnt&0x2000!=0&&inside(0x40,0x44){io16(mem,0x48)&63}else if dispcnt&0x4000!=0&&inside(0x42,0x46){(io16(mem,0x48)>>8)&63}else if dispcnt&0x8000!=0&&obj_window[x]{(io16(mem,0x4a)>>8)&63}else{io16(mem,0x4a)&63};
        *pixel=backdrop;
        for bg in order {if used[bg]&&control&(1<<bg)!=0&&bg_lines[bg][x]!=TRANSPARENT {*pixel=bg_lines[bg][x];break;}}
    }
}

#[allow(clippy::too_many_arguments)]
fn compose_into(
    mem: &MemMap,
    dispcnt: u16,
    line: usize,
    bg_lines: &[Line; 4],
    bg_used: &[bool; 4],
    obj_line: &[ObjPixel; MAX_RENDER_WIDTH],
    obj_window: &[bool; MAX_RENDER_WIDTH],
    line_has_semi: bool,
    output: &mut [u16],
    counts: &mut std::collections::BTreeMap<u32, u32>,
) {
    let rw = render_width();
    let ex = extra();
    let backdrop = if mem.ffta_movement.zoom_output {
        let [top, bottom] = mem.ffta_movement.sky_edges;
        extend_sky(top, bottom, line as i32)
    } else {
        pal16(mem, 0)
    };

    // Background priorities (compose ties: lower BG number wins; sort
    // keys are unique, so unstable sort matches the old stable sort).
    let mut bg_prio_buf = [(0u8, 0usize); 4];
    let mut nbg = 0;
    for bg in 0..4 {
        if bg_used[bg] && bg_lines[bg][..rw].iter().any(|&color| color != TRANSPARENT) {
            bg_prio_buf[nbg] = ((io16(mem, 0x08 + bg * 2) & 3) as u8, bg);
            nbg += 1;
        }
    }
    bg_prio_buf[..nbg].sort_unstable();
    let bg_prio = &bg_prio_buf[..nbg];

    let win0_on = dispcnt & 0x2000 != 0;
    let win1_on = dispcnt & 0x4000 != 0;
    let objwin_on = dispcnt & 0x8000 != 0;
    let any_window = win0_on || win1_on || objwin_on;

    let winin = io16(mem, 0x48);
    let winout = io16(mem, 0x4A);

    let win0h = io16(mem, 0x40);
    let win1h = io16(mem, 0x42);
    let win0v = io16(mem, 0x44);
    let win1v = io16(mem, 0x46);

    let window_y = if mem.ffta_movement.zoom_output {
        (line as isize).clamp(0, 159) as u8
    } else {
        line as u8
    };
    let in_win0_y = win0_on && in_window_range((win0v >> 8) as u8, win0v as u8, window_y);
    let in_win1_y = win1_on && in_window_range((win1v >> 8) as u8, win1v as u8, window_y);

    let bldcnt = io16(mem, 0x50);
    let blend_mode = (bldcnt >> 6) & 3;
    let bldalpha = io16(mem, 0x52);
    let eva = (bldalpha & 0x1F).min(16) as u32;
    let evb = ((bldalpha >> 8) & 0x1F).min(16) as u32;
    let evy = (io16(mem, 0x54) & 0x1F).min(16) as u32;

    // Entire sky rows are common at mobile zoom-out. No object/layer
    // exists here, so preserve the backdrop effect once for the whole row.
    if nbg == 0
        && (!any_window || blend_mode == 0 || bldcnt & 0x20 == 0)
        && obj_line[..rw].iter().all(|obj| obj.priority == 0xff)
    {
        let color = if !any_window && bldcnt & 0x20 != 0 {
            match blend_mode {
                1 if bldcnt & 0x2000 != 0 => alpha_blend(backdrop, backdrop, eva, evb),
                2 => brighten(backdrop, evy),
                3 => darken(backdrop, evy),
                _ => backdrop,
            }
        } else {
            backdrop
        };
        output.fill(color);
        return;
    }
    let trace =
        crate::ffta_sprites::trace_enabled(mem.frames) || crate::ffta_sprites::character_metrics();

    // Fast path: no color effects and no semi-transparent sprite pixels
    // on this line, so the second-layer search is dead code - the top
    // layer alone decides each pixel: the first opaque BG in priority
    // order whose window control bit allows it, beaten by the sprite
    // when its priority is <= that BG's (sprites rank above BGs of
    // equal priority).
    if blend_mode == 0 && !line_has_semi {
        for x in 0..rw {
            let wx = (x as isize - ex as isize).clamp(0, 239) as u8;
            let control: u16 = if !any_window {
                0x3F
            } else if in_win0_y && in_window_range((win0h >> 8) as u8, win0h as u8, wx) {
                winin & 0x3F
            } else if in_win1_y && in_window_range((win1h >> 8) as u8, win1h as u8, wx) {
                (winin >> 8) & 0x3F
            } else if objwin_on && obj_window[x] {
                (winout >> 8) & 0x3F
            } else {
                winout & 0x3F
            };

            let mut color = backdrop;
            let mut top_prio = 0xFFu8;
            for &(bp, bg) in bg_prio {
                if control & (1 << bg) != 0 {
                    let c = bg_lines[bg][x];
                    if c != TRANSPARENT {
                        color = c;
                        top_prio = bp;
                        break;
                    }
                }
            }
            let obj = &obj_line[x];
            if obj.priority != 0xFF && control & 0x10 != 0 && obj.priority <= top_prio {
                color = obj.color;
                if obj.owner != 0 && trace {
                    *counts.entry(obj.owner).or_default() += 1;
                }
            }
            output[x] = color;
        }
        return;
    }

    for x in 0..rw {
        let wx = (x as isize - ex as isize).clamp(0, 239) as u8;
        // Window control for this pixel: bits 0-3 BGs, 4 OBJ, 5 effects.
        let control: u16 = if !any_window {
            0x3F
        } else if in_win0_y && in_window_range((win0h >> 8) as u8, win0h as u8, wx) {
            winin & 0x3F
        } else if in_win1_y && in_window_range((win1h >> 8) as u8, win1h as u8, wx) {
            (winin >> 8) & 0x3F
        } else if objwin_on && obj_window[x] {
            (winout >> 8) & 0x3F
        } else {
            winout & 0x3F
        };

        // Find the top two visible layers (OBJ ranks above BGs of equal
        // priority; BGs tie-break by index via the pre-sorted list).
        let obj = &obj_line[x];
        let obj_visible = obj.priority != 0xFF && control & 0x10 != 0;

        let mut first: Option<(u16, usize)> = None;
        let mut second: Option<(u16, usize)> = None;
        // BGs already have a total priority order. Merge the one OBJ
        // into that order rather than scanning all BGs at each priority.
        let mut pending_obj = obj_visible;
        for &(bp, bg) in bg_prio {
            if pending_obj && obj.priority <= bp {
                pending_obj = false;
                if first.is_none() {
                    first = Some((obj.color, 4));
                } else {
                    second = Some((obj.color, 4));
                    break;
                }
            }
            if control & (1 << bg) != 0 {
                let c = bg_lines[bg][x];
                if c != TRANSPARENT {
                    if first.is_none() {
                        first = Some((c, bg));
                    } else {
                        second = Some((c, bg));
                        break;
                    }
                }
            }
        }
        if pending_obj && second.is_none() {
            if first.is_none() {
                first = Some((obj.color, 4));
            } else {
                second = Some((obj.color, 4));
            }
        }
        let (fc, fl) = first.unwrap_or((backdrop, 5));
        if fl == 4 && obj.owner != 0 && trace {
            *counts.entry(obj.owner).or_default() += 1;
        }
        let (sc, sl) = second.unwrap_or((backdrop, 5));

        let effects_allowed = control & 0x20 != 0;
        let semi = obj_visible && fl == 4 && obj.semi;

        let color = if semi && bldcnt & (0x100 << sl) != 0 {
            // Semi-transparent sprite forces alpha against 2nd targets.
            alpha_blend(fc, sc, eva, evb)
        } else if effects_allowed && blend_mode != 0 && bldcnt & (1 << fl) != 0 {
            match blend_mode {
                1 if bldcnt & (0x100 << sl) != 0 => alpha_blend(fc, sc, eva, evb),
                2 => brighten(fc, evy),
                3 => darken(fc, evy),
                _ => fc,
            }
        } else {
            fc
        };

        output[x] = color;
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn compose_reference(
    mem: &mut MemMap,
    dispcnt: u16,
    line: usize,
    bg_lines: &[Line; 4],
    bg_used: &[bool; 4],
    obj_line: &[ObjPixel; MAX_RENDER_WIDTH],
    obj_window: &[bool; MAX_RENDER_WIDTH],
    line_has_semi: bool,
) {
    let backdrop = if mem.ffta_movement.zoom_output {
        let [top, bottom] = mem.ffta_movement.sky_edges;
        extend_sky(top, bottom, line as i32)
    } else {
        pal16(mem, 0)
    };

    // Background priorities (compose ties: lower BG number wins; sort
    // keys are unique, so unstable sort matches the old stable sort).
    let mut bg_prio_buf = [(0u8, 0usize); 4];
    let mut nbg = 0;
    for bg in 0..4 {
        if bg_used[bg] {
            bg_prio_buf[nbg] = ((io16(mem, 0x08 + bg * 2) & 3) as u8, bg);
            nbg += 1;
        }
    }
    bg_prio_buf[..nbg].sort_unstable();
    let bg_prio = &bg_prio_buf[..nbg];

    let win0_on = dispcnt & 0x2000 != 0;
    let win1_on = dispcnt & 0x4000 != 0;
    let objwin_on = dispcnt & 0x8000 != 0;
    let any_window = win0_on || win1_on || objwin_on;

    let winin = io16(mem, 0x48);
    let winout = io16(mem, 0x4A);

    let win0h = io16(mem, 0x40);
    let win1h = io16(mem, 0x42);
    let win0v = io16(mem, 0x44);
    let win1v = io16(mem, 0x46);

    let window_y = if mem.ffta_movement.zoom_output {
        (line as isize).clamp(0, 159) as u8
    } else {
        line as u8
    };
    let in_win0_y = win0_on && in_window_range((win0v >> 8) as u8, win0v as u8, window_y);
    let in_win1_y = win1_on && in_window_range((win1v >> 8) as u8, win1v as u8, window_y);

    let bldcnt = io16(mem, 0x50);
    let blend_mode = (bldcnt >> 6) & 3;
    let bldalpha = io16(mem, 0x52);
    let eva = (bldalpha & 0x1F).min(16) as u32;
    let evb = ((bldalpha >> 8) & 0x1F).min(16) as u32;
    let evy = (io16(mem, 0x54) & 0x1F).min(16) as u32;

    // Fast path: no color effects and no semi-transparent sprite pixels
    // on this line, so the second-layer search is dead code - the top
    // layer alone decides each pixel: the first opaque BG in priority
    // order whose window control bit allows it, beaten by the sprite
    // when its priority is <= that BG's (sprites rank above BGs of
    // equal priority).
    if blend_mode == 0 && !line_has_semi {
        for x in 0..render_width() {
            let wx = win_x(x);
            let control: u16 = if !any_window {
                0x3F
            } else if in_win0_y && in_window_range((win0h >> 8) as u8, win0h as u8, wx) {
                winin & 0x3F
            } else if in_win1_y && in_window_range((win1h >> 8) as u8, win1h as u8, wx) {
                (winin >> 8) & 0x3F
            } else if objwin_on && obj_window[x] {
                (winout >> 8) & 0x3F
            } else {
                winout & 0x3F
            };

            let mut color = backdrop;
            let mut top_prio = 0xFFu8;
            for &(bp, bg) in bg_prio {
                if control & (1 << bg) != 0 {
                    let c = bg_lines[bg][x];
                    if c != TRANSPARENT {
                        color = c;
                        top_prio = bp;
                        break;
                    }
                }
            }
            let obj = &obj_line[x];
            if obj.priority != 0xFF && control & 0x10 != 0 && obj.priority <= top_prio {
                color = obj.color;
                if obj.owner != 0 && crate::ffta_sprites::trace_enabled(mem.frames) {
                    *mem.ffta_sprites.pixel_counts.entry(obj.owner).or_default() += 1;
                }
            }
            put_pixel(mem, line, x, color);
        }
        return;
    }

    for x in 0..render_width() {
        let wx = win_x(x);
        // Window control for this pixel: bits 0-3 BGs, 4 OBJ, 5 effects.
        let control: u16 = if !any_window {
            0x3F
        } else if in_win0_y && in_window_range((win0h >> 8) as u8, win0h as u8, wx) {
            winin & 0x3F
        } else if in_win1_y && in_window_range((win1h >> 8) as u8, win1h as u8, wx) {
            (winin >> 8) & 0x3F
        } else if objwin_on && obj_window[x] {
            (winout >> 8) & 0x3F
        } else {
            winout & 0x3F
        };

        // Find the top two visible layers (OBJ ranks above BGs of equal
        // priority; BGs tie-break by index via the pre-sorted list).
        let obj = &obj_line[x];
        let obj_visible = obj.priority != 0xFF && control & 0x10 != 0;

        let mut first: Option<(u16, usize)> = None;
        let mut second: Option<(u16, usize)> = None;
        'outer: for p in 0..4u8 {
            if obj_visible && obj.priority == p {
                if first.is_none() {
                    first = Some((obj.color, 4));
                } else {
                    second = Some((obj.color, 4));
                    break 'outer;
                }
            }
            for &(bp, bg) in bg_prio {
                if bp == p && control & (1 << bg) != 0 {
                    let c = bg_lines[bg][x];
                    if c != TRANSPARENT {
                        if first.is_none() {
                            first = Some((c, bg));
                        } else {
                            second = Some((c, bg));
                            break 'outer;
                        }
                    }
                }
            }
        }
        let (fc, fl) = first.unwrap_or((backdrop, 5));
        if fl == 4 && obj.owner != 0 && crate::ffta_sprites::trace_enabled(mem.frames) {
            *mem.ffta_sprites.pixel_counts.entry(obj.owner).or_default() += 1;
        }
        let (sc, sl) = second.unwrap_or((backdrop, 5));

        let effects_allowed = control & 0x20 != 0;
        let semi = obj_visible && fl == 4 && obj.semi;

        let color = if semi && bldcnt & (0x100 << sl) != 0 {
            // Semi-transparent sprite forces alpha against 2nd targets.
            alpha_blend(fc, sc, eva, evb)
        } else if effects_allowed && blend_mode != 0 && bldcnt & (1 << fl) != 0 {
            match blend_mode {
                1 if bldcnt & (0x100 << sl) != 0 => alpha_blend(fc, sc, eva, evb),
                2 => brighten(fc, evy),
                3 => darken(fc, evy),
                _ => fc,
            }
        } else {
            fc
        };

        put_pixel(mem, line, x, color);
    }
}

/// Window coordinate test: x1 = left/top (inclusive), x2 = right/bottom
/// (exclusive); x1 > x2 wraps.
fn in_window_range(x1: u8, x2: u8, v: u8) -> bool {
    if x1 <= x2 {
        (x1..x2).contains(&v)
    } else {
        v >= x1 || v < x2
    }
}

// Color-effect arithmetic models the hardware blend unit, not GBATEK's
// fractional notation: per-channel multiply-accumulate rounds half-up
// (+8 >> 4; the darken subtrahend rounds half-down, +7, so the result
// still rounds half-up), and green passes through a 6-bit lane (the
// real hardware LSB of that lane is PRAM bit 15, which we mask at
// palette fetch — see pal16).

fn alpha_blend(a: u16, b: u16, eva: u32, evb: u32) -> u16 {
    let (ar, ag, ab) = split6(a);
    let (br, bg, bb) = split6(b);
    let r = ((ar * eva + br * evb + 8) >> 4).min(31);
    let g = ((ag * eva + bg * evb + 8) >> 4).min(63);
    let b = ((ab * eva + bb * evb + 8) >> 4).min(31);
    join6(r, g, b)
}

fn brighten(c: u16, evy: u32) -> u16 {
    let (r, g, b) = split6(c);
    join6(
        r + (((31 - r) * evy + 8) >> 4),
        g + (((63 - g) * evy + 8) >> 4),
        b + (((31 - b) * evy + 8) >> 4),
    )
}

fn darken(c: u16, evy: u32) -> u16 {
    let (r, g, b) = split6(c);
    join6(
        r - ((r * evy + 7) >> 4),
        g - ((g * evy + 7) >> 4),
        b - ((b * evy + 7) >> 4),
    )
}

/// Channels as the blend unit sees them: 5-bit red/blue, 6-bit green
/// lane (5-bit value doubled; hardware ORs in PRAM bit 15 as the LSB,
/// which we strip at fetch).
fn split6(c: u16) -> (u32, u32, u32) {
    (
        (c & 31) as u32,
        (((c >> 5) & 31) as u32) << 1,
        ((c >> 10) & 31) as u32,
    )
}

fn join6(r: u32, g: u32, b: u32) -> u16 {
    (r | ((g >> 1) << 5) | (b << 10)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;

    #[test]
    fn mode3_pixel() {
        let mut mem = MemMap::new(vec![]);
        mem.write16(0x0400_0000, 0x0403); // mode 3, BG2 on
        mem.write16(0x0600_0000 + 2 * (5 + 7 * 240), 0x7C1F); // magenta at (5,7)
        render_scanline(&mut mem, 7);
        assert_eq!(mem.framebuffer[7 * 240 + 5], 0x7C1F);
        assert_eq!(mem.framebuffer[7 * 240 + 6], 0x0000);
    }

    #[test]
    fn mode0_text_tile() {
        let mut mem = MemMap::new(vec![]);
        mem.write16(0x0400_0000, 0x0100); // mode 0, BG0 on
        // BG0: char base 0, screen base block 31 (0xF800), 4bpp, 32x32.
        mem.write16(0x0400_0008, 31 << 8);
        // Palette 1, color 3 = red-ish.
        mem.write16(0x0500_0000 + (16 + 3) * 2, 0x001F);
        // Tile 1: all pixels = color 3.
        for row in 0..8 {
            mem.write32(0x0600_0000 + 32 + row * 4, 0x3333_3333);
        }
        // Map entry (0,0): tile 1, palette 1.
        mem.write16(0x0600_F800, (1 << 12) | 1);
        render_scanline(&mut mem, 0);
        assert_eq!(mem.framebuffer[0], 0x001F);
        // (8,0) is tile (1,0): map entry 0 -> tile 0 -> transparent -> backdrop.
        assert_eq!(mem.framebuffer[8], pal16(&mem, 0));
    }

    #[test]
    fn sprite_over_backdrop() {
        let mut mem = MemMap::new(vec![]);
        mem.write16(0x0400_0000, 0x1000); // mode 0, OBJ on
        // OBJ palette color 1 = green.
        mem.write16(0x0500_0200 + 2, 0x03E0);
        // Tile 1 in OBJ charblock: all color 1 (4bpp).
        for row in 0..8 {
            mem.write32(0x0601_0000 + 32 + row * 4, 0x1111_1111);
        }
        // Sprite 0: 8x8 at (10, 20), tile 1.
        mem.write16(0x0700_0000, 20); // attr0: y=20
        mem.write16(0x0700_0002, 10); // attr1: x=10
        mem.write16(0x0700_0004, 1); // attr2: tile 1
        render_scanline(&mut mem, 22);
        assert_eq!(mem.framebuffer[22 * 240 + 10], 0x03E0);
        assert_eq!(mem.framebuffer[22 * 240 + 18], 0x0000); // outside sprite
    }

    #[test]
    fn sprite_flips_use_obj_attr1_bits() {
        // Tile with one green pixel at texel (0, 0); the rest transparent.
        // Flips relocate the lit screen pixel: hflip = attr1 bit 12,
        // vflip = bit 13 (NOT the BG-entry bits 10/11).
        let mut mem = MemMap::new(vec![]);
        mem.write16(0x0400_0000, 0x1000); // mode 0, OBJ on
        mem.write16(0x0500_0200 + 2, 0x03E0);
        mem.write32(0x0601_0000 + 32, 0x0000_0001); // tile 1 row 0: texel 0 = color 1
        mem.write16(0x0700_0000, 20); // attr0: y=20
        mem.write16(0x0700_0002, 10); // attr1: x=10
        mem.write16(0x0700_0004, 1); // attr2: tile 1
        let lit = |mem: &mut MemMap, line: usize, x: usize| {
            render_scanline(mem, line);
            mem.framebuffer[line * 240 + x] == 0x03E0
        };
        assert!(lit(&mut mem, 20, 10)); // unflipped: top-left
        mem.write16(0x0700_0002, 10 | 0x1000); // hflip
        assert!(lit(&mut mem, 20, 17));
        mem.write16(0x0700_0002, 10 | 0x2000); // vflip
        assert!(lit(&mut mem, 27, 10));
        mem.write16(0x0700_0002, 10 | 0x3000); // both
        assert!(lit(&mut mem, 27, 17));
        // BG-entry flip bits must do nothing on sprites.
        mem.write16(0x0700_0002, 10 | 0x0C00);
        assert!(lit(&mut mem, 20, 10));
    }

    #[test]
    fn brightness_math() {
        assert_eq!(brighten(0x0000, 16), 0x7FFF);
        assert_eq!(darken(0x7FFF, 16), 0x0000);
        assert_eq!(brighten(0x0000, 0), 0x0000);
    }

    // Hardware blend-unit identities:
    // round-half-up MAC, 6-bit green lane.

    #[test]
    fn blend_identity_and_extremes() {
        // eva=16/evb=0 passes every color through exactly, including
        // the green lane round-trip.
        for c in [0x0000u16, 0x7FFF, 0x7C1F, 0x03E0, 0x1234, 0x5A5A] {
            assert_eq!(alpha_blend(c, 0x7FFF ^ c, 16, 0), c);
            assert_eq!(alpha_blend(0x7FFF ^ c, c, 0, 16), c);
        }
        // Saturation: white + white at full coefficients clamps.
        assert_eq!(alpha_blend(0x7FFF, 0x7FFF, 16, 16), 0x7FFF);
        // evy=0 is a no-op for both brightness directions.
        for c in [0x7FFF, 0x1234, 0x5A5A] {
            assert_eq!(brighten(c, 0), c);
            assert_eq!(darken(c, 0), c);
        }
    }

    #[test]
    fn blend_rounds_half_up() {
        // 50/50 of red 15 and 16: true value 15.5, hardware rounds up.
        // (15*8 + 16*8 + 8) >> 4 = 16.
        assert_eq!(alpha_blend(15, 16, 8, 8) & 31, 16);
        // Red 0 with 1 at evb=8: 0.5 rounds up to 1 (truncation gave 0).
        assert_eq!(alpha_blend(0, 1, 8, 8) & 31, 1);
        // Green's 6-bit lane rounds up only at fraction >= 12/16:
        // greens 15/16 at 8/8 -> S=124 in g6 (30*8+32*8=496; +8 >>4 =
        // 31; >>1 = 15): the half-step stays DOWN on green.
        assert_eq!((alpha_blend(15 << 5, 16 << 5, 8, 8) >> 5) & 31, 15);
        // ...but green 15/eva=4, 16/evb=12 (true 15.75) rounds up.
        assert_eq!((alpha_blend(15 << 5, 16 << 5, 4, 12) >> 5) & 31, 16);
    }

    #[test]
    fn brightness_rounds() {
        // Brighten red 0 by 1/16: (31*1 + 8) >> 4 = 2 (truncation: 1).
        assert_eq!(brighten(0, 1) & 31, 2);
        // Darken red 31 by 1/16: 31 - ((31 + 7) >> 4) = 29.
        assert_eq!(darken(31, 1) & 31, 29);
        // Green lane: brighten green 0 by 1/16 -> g6 = (63+8)>>4 = 4,
        // back to 5-bit = 2.
        assert_eq!((brighten(0, 1) >> 5) & 31, 2);
    }
}

#[cfg(test)]
mod optimized_composition_tests {
    use super::*;
    #[test]
    fn sorted_merge_matches_original_for_windows_blends_and_priorities() {
        // Run this test with one test thread: render dimensions are global.
        crate::mem::set_extra(136);
        crate::mem::set_extra_y(176);
        let mut fast = MemMap::new(vec![]);
        let mut reference = MemMap::new(vec![]);
        let mut seed = 0x12345678u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for case in 0..512 {
            for off in (0..0x56).step_by(2) {
                let value = next() as u16;
                fast.io[off..off + 2].copy_from_slice(&value.to_le_bytes());
            }
            reference.io.copy_from_slice(&fast.io);
            fast.palette[0..2].copy_from_slice(&(next() as u16).to_le_bytes());
            reference.palette.copy_from_slice(&fast.palette);
            let mut layers = [[TRANSPARENT; MAX_RENDER_WIDTH]; 4];
            let used = std::array::from_fn(|_| next() & 1 != 0);
            for layer in &mut layers {
                for c in &mut layer[..512] {
                    let n = next();
                    *c = if n & 3 == 0 {
                        TRANSPARENT
                    } else {
                        n as u16 & 0x7fff
                    };
                }
            }
            let mut objects = [OBJ_EMPTY; MAX_RENDER_WIDTH];
            let mut windows = [false; MAX_RENDER_WIDTH];
            for x in 0..512 {
                let n = next();
                objects[x] = ObjPixel {
                    color: n as u16 & 0x7fff,
                    priority: if n & 7 == 0 { 255 } else { (n & 3) as u8 },
                    semi: n & 8 != 0,
                    owner: 0,
                };
                windows[x] = next() & 1 != 0;
            }
            if case % 8 == 0 {
                layers.fill([TRANSPARENT; MAX_RENDER_WIDTH]);
                objects.fill(OBJ_EMPTY);
            }
            fast.ffta_movement.zoom_output = case & 1 != 0;
            reference.ffta_movement.zoom_output = fast.ffta_movement.zoom_output;
            let line = if case & 1 != 0 {
                (-80isize) as usize
            } else {
                70
            };
            let dispcnt = next() as u16;
            compose(
                &mut fast,
                dispcnt,
                line,
                &layers,
                &used,
                &objects,
                &windows,
                case & 2 != 0,
            );
            compose_reference(
                &mut reference,
                dispcnt,
                line,
                &layers,
                &used,
                &objects,
                &windows,
                case & 2 != 0,
            );
            assert_eq!(
                fast.wide_framebuffer, reference.wide_framebuffer,
                "wide case {case}"
            );
            assert_eq!(
                fast.framebuffer, reference.framebuffer,
                "hardware case {case}"
            );
        }
        crate::mem::set_extra(0);
        crate::mem::set_extra_y(0);
    }
}

#[cfg(test)]
mod parallel_zoom_tests {
    use super::*;
    #[test]
    fn parallel_rows_match_serial_with_terrain_tiles_sprites_and_effects() {
        crate::mem::set_extra(136);
        crate::mem::set_extra_y(176);
        let mut mem = MemMap::new(vec![0; 192]);
        mem.ffta_stage.enabled = true;
        mem.ffta_stage.bases = [Some(0x020091a0), Some(0x0200b1a0)];
        mem.ffta_stage.origins = [Some((11, 8)), Some((11, 8))];
        mem.ffta_stage.valid = [true, true];
        mem.io[0..2].copy_from_slice(&0x1f00u16.to_le_bytes());
        for (bg, cnt) in [0x0a0bu16, 0x0b0a, 0x4c03, 0x4e02].into_iter().enumerate() {
            mem.io[8 + bg * 2..10 + bg * 2].copy_from_slice(&cnt.to_le_bytes());
        }
        let mut seed = 7123u32;
        for region in [&mut mem.vram[..], &mut mem.ewram[..], &mut mem.palette[..]] {
            for byte in region {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                *byte = (seed >> 24) as u8;
            }
        }
        let original = mem.wide_framebuffer.clone();
        for blend in [0u16, 0x3f41, 0x3fff, 0x3f9f] {
            mem.io[0x50..0x52].copy_from_slice(&blend.to_le_bytes());
            mem.io[0x52..0x54].copy_from_slice(&0x0808u16.to_le_bytes());
            mem.io[0x54..0x56].copy_from_slice(&7u16.to_le_bytes());
            mem.wide_framebuffer.copy_from_slice(&original);
            set_parallel_zoom(false);
            render_ffta_zoom_rows(&mut mem);
            let expected = mem.wide_framebuffer.clone();
            let hardware = mem.framebuffer.clone();
            let oam = mem.oam.clone();
            mem.wide_framebuffer.copy_from_slice(&original);
            set_parallel_zoom(true);
            render_ffta_zoom_rows(&mut mem);
            assert_eq!(mem.wide_framebuffer, expected, "blend {blend:04x}");
            assert_eq!(mem.framebuffer, hardware);
            assert_eq!(mem.oam, oam);
        }
        set_parallel_zoom(cfg!(target_os = "android"));
        crate::mem::set_extra(0);
        crate::mem::set_extra_y(0);
    }
}

