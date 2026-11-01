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
fn win_x(i: usize) -> u8 {
    (i as isize - extra() as isize).clamp(0, VISIBLE_WIDTH as isize - 1) as u8
}

/// Store one composed pixel: always into the wide buffer, and into the
/// legacy 240-wide framebuffer when the column is on the visible screen.
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
    let t=mem.ffta_dialogue.profile.enabled.then(std::time::Instant::now);
    render_ffta_zoom_rows_impl(mem);
    if let Some(t)=t{mem.ffta_dialogue.profile.zoom_ns=t.elapsed().as_nanos()as u64;}
}
fn render_ffta_zoom_rows_impl(mem: &mut MemMap) {
    let ey = crate::mem::extra_y();
    if !ffta_zoom_rows_ready(mem) {
        return;
    }
    let rw = render_width();
    mem.wide_framebuffer.copy_within(0..160 * rw, ey * rw);
    if mem.ffta_surface.capture(){mem.ffta_surface.mask.copy_within(0..160*rw,ey*rw);mem.ffta_surface.coordinates.copy_within(0..160*rw,ey*rw);}
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
    let mut uv=std::mem::take(&mut mem.ffta_surface.scratch);
    let capture=mem.ffta_surface.capture();
    uv.resize(if capture{2*ey*rw}else{0},0);
    let (uv_even,uv_odd)=if capture{let(a,b)=uv.split_at_mut(ey*rw);(Some(a),Some(b))}else{(None,None)};
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
                .num_threads(zoom_worker_count())
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
                if pool.current_num_threads()>=4 {
                    let first=ey/2;let at=first*rw;
                    let(e0,e1)=even.split_at_mut(at);let(o0,o1)=odd.split_at_mut(at);
                    let(be0,be1)=split_optional(bg_even,at);let(bo0,bo1)=split_optional(bg_odd,at);
                    let(ue0,ue1)=split_optional(uv_even,at);let(uo0,uo1)=split_optional(uv_odd,at);
                    rayon::join(
                        ||{rayon::join(
                            ||render_zoom_lane_part(snapshot,dispcnt,ey,rw,0,0,e0,be0,ue0),
                            ||render_zoom_lane_part(snapshot,dispcnt,ey,rw,0,first,e1,be1,ue1));},
                        ||{rayon::join(
                            ||render_zoom_lane_part(snapshot,dispcnt,ey,rw,1,0,o0,bo0,uo0),
                            ||render_zoom_lane_part(snapshot,dispcnt,ey,rw,1,first,o1,bo1,uo1));},
                    );
                }else{
                    rayon::join(
                        || render_zoom_lane(snapshot, dispcnt, ey, rw, 0, even, bg_even, uv_even),
                        || render_zoom_lane(snapshot, dispcnt, ey, rw, 1, odd, bg_odd, uv_odd),
                    );
                }
            });
        } else {
            render_zoom_lane(snapshot, dispcnt, ey, rw, 0, even, bg_even, uv_even);
            render_zoom_lane(snapshot, dispcnt, ey, rw, 1, odd, bg_odd, uv_odd);
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
            render_zoom_lane(snapshot, dispcnt, ey, rw, 0, even, bg_even, uv_even)
        });
        render_zoom_lane(snapshot, dispcnt, ey, rw, 1, odd, bg_odd, uv_odd);
    });
    for row in 0..2 * ey {
        let source = ((row & 1) * ey + row / 2) * rw;
        let destination = if row < ey { row } else { row + 160 };
        if mem.ffta_surface.capture(){for x in 0..rw{
            let pixel=world[source+x];mem.wide_framebuffer[destination*rw+x]=pixel&0x7fff;
            mem.ffta_surface.mask[destination*rw+x]=if pixel&0x8000!=0{255}else{0};
            mem.ffta_surface.coordinates[destination*rw+x]=uv[source+x];
        }}else{
        mem.wide_framebuffer[destination * rw..(destination + 1) * rw]
            .copy_from_slice(&world[source..source + rw]);
        }
        if stats {mem.ffta_dialogue.extra.background[destination*rw..(destination+1)*rw].copy_from_slice(&background[source..source+rw]);}
    }
    mem.zoom_scratch = scratch;
    mem.ffta_surface.scratch=uv;
}
// Debug override is configured before the first frame/pool creation. The
// automatic path adapts to CPU capacity rather than a device/GPU model list.
static ZOOM_WORKER_LIMIT:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
pub fn set_zoom_worker_limit(limit:usize){
    ZOOM_WORKER_LIMIT.store(if matches!(limit,2|4){limit}else{0},std::sync::atomic::Ordering::Relaxed);
}
fn zoom_worker_count()->usize {
    let requested=ZOOM_WORKER_LIMIT.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(target_os="android")]
    let automatic=if zoom_fast_cpus().len()>=4{4}else{2};
    #[cfg(not(target_os="android"))]
    let automatic=std::thread::available_parallelism().map_or(2,|n|if n.get()>=5{4}else{2});
    if requested==0{automatic}else{requested.min(automatic)}
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
        let fastest=capacities[0].0;
        capacities.into_iter().filter(|(capacity,_)|capacity.saturating_mul(10)>=fastest.saturating_mul(7))
            .take(4).map(|(_,cpu)|cpu).collect()
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
    raw_background: Option<&mut [u16]>,
    coordinates:Option<&mut[u32]>,
) {
    render_zoom_lane_part(mem,dispcnt,ey,rw,lane,0,output,raw_background,coordinates);
}
fn split_optional<T>(slice:Option<&mut[T]>,at:usize)->(Option<&mut[T]>,Option<&mut[T]>){
    slice.map_or((None,None),|s|{let(a,b)=s.split_at_mut(at);(Some(a),Some(b))})
}
/// Conservative sprite envelope for the immutable VBlank snapshot only.
/// Never reuse it across native scanlines: guest OAM/VRAM may change there.
fn zoom_sprite_y_bounds(mem:&MemMap)->Option<(i32,i32)> {
    if !mem.ffta_dialogue.separate || !mem.ffta_dialogue.wide_pass || mem.ffta_dialogue.capture {
        return Some((i32::MIN,i32::MAX));
    }
    let mut bounds:Option<(i32,i32)>=None;
    let mut add=|a:[u16;3],host_y:Option<i32>| {
        let [a0,_,a2]=a;
        let affine=a0&0x100!=0;
        if !affine && a0&0x200!=0 || a0>>14==3 {return;}
        if host_y.is_none() && mem.ffta_sprites.enabled && a==[0x00a8,0x00f8,0] {return;}
        let h=OBJ_SIZES[(a0>>14)as usize][(a[1]>>14)as usize].1
            *if affine && a0&0x200!=0 {2}else{1};
        let native=(a0&255)as i32;
        let y=host_y.unwrap_or(if native>=160{native-256}else{native});
        let stats=(mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats
            || mem.ffta_dialogue.battle_stats.kind==crate::ffta_dialogue::PanelKind::Stats)
            && a0&0x2000==0 && matches!(a2>>12,11|12) && (0x1c8..0x240).contains(&(a2&1023));
        let portrait=mem.ffta_dialogue.action_layout && a0&0x2000!=0
            && a2&0xc00<=0x400 && (0x200..0x400).contains(&(a2&1023));
        let normalized=y.rem_euclid(256);let normalized=if normalized>=160{normalized-256}else{normalized};
        let (lo,hi)=if stats || portrait {(y.min(normalized),y.max(normalized)+h)}else{(y,y+h)};
        bounds=Some(bounds.map_or((lo,hi),|(l,r)|(l.min(lo),r.max(hi))));
    };
    // Keeping replaced/hidden pieces only widens the envelope; it cannot hide
    // a real actor. Published host coordinates include offscreen characters.
    for o in mem.oam.chunks_exact(8) {
        add(std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]])),None);
    }
    if let Some(host)=mem.ffta_sprites.active.as_deref() {
        for sprite in host {add(sprite.attrs,Some(sprite.y));}
    }
    bounds
}

fn render_zoom_lane_part(
    mem: &MemMap,
    dispcnt: u16,
    ey: usize,
    rw: usize,
    lane: usize,
    start_index:usize,
    output: &mut [u16],
    mut raw_background: Option<&mut [u16]>,
    mut coordinates:Option<&mut[u32]>,
) {
    let sprite_bounds=zoom_sprite_y_bounds(mem);
    let mut counts = std::collections::BTreeMap::new();
    let mut backgrounds = [[TRANSPARENT; MAX_RENDER_WIDTH]; 4];
    let mut objects = [OBJ_EMPTY; MAX_RENDER_WIDTH];
    let mut window = [false; MAX_RENDER_WIDTH];
    for (index, row) in output.chunks_exact_mut(rw).enumerate() {
        let y = (start_index+index) * 2 + lane;
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
        let semi = if dispcnt & 0x1000 != 0 && sprite_bounds.is_some_and(|(lo,hi)|sy>=lo as isize && sy<hi as isize) {
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
            coordinates.as_deref_mut().map(|c|&mut c[index*rw..(index+1)*rw]),
        );
        if let Some(bg) = raw_background.as_deref_mut() {
            compose_background_raw(mem,dispcnt,line,&backgrounds,&used,&window,&mut bg[index*rw..(index+1)*rw]);
        }
    }
}

pub fn render_scanline(mem: &mut MemMap, line: usize) {
    if line==0 {let p=&mut mem.ffta_dialogue.profile;p.capture_ns=0;p.setup_ns=0;p.rows=0;p.raster_ns=0;p.zoom_ns=0;}
    let t=mem.ffta_dialogue.profile.enabled.then(std::time::Instant::now);
    render_scanline_impl(mem,line);
    if let Some(t)=t{mem.ffta_dialogue.profile.raster_ns+=t.elapsed().as_nanos()as u64;}
}
fn render_scanline_impl(mem: &mut MemMap, line: usize) {
    if line == 0 {
        if mem.ffta_sprites.enabled && (extra() > 0 || crate::mem::extra_y() > 0) {
            mem.ffta_sprites.validate_oam(&mem.oam);
        }
        let stage=crate::ffta_surface::Descriptor::read(mem);
        mem.ffta_surface.begin(stage,(render_width(),crate::mem::render_height()),mem.frames);
        let setup=mem.ffta_dialogue.profile.enabled.then(std::time::Instant::now);
        begin_dialogue_frame(mem);
        if let Some(t)=setup{mem.ffta_dialogue.profile.setup_ns=t.elapsed().as_nanos()as u64;}
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

    // Sprites.
    let mut obj_line = [OBJ_EMPTY; MAX_RENDER_WIDTH];
    let mut obj_window = [false; MAX_RENDER_WIDTH];
    let mut line_has_semi = false;
    if dispcnt & 0x1000 != 0 {
        line_has_semi = render_sprites(mem, dispcnt, line, &mut obj_line, &mut obj_window);
    }

    // Step affine accumulators for the next line (PB/PD).
    mem.aff_ref[0] = mem.aff_ref[0].wrapping_add(io16(mem, 0x22) as i16 as i32);
    mem.aff_ref[1] = mem.aff_ref[1].wrapping_add(io16(mem, 0x26) as i16 as i32);
    mem.aff_ref[2] = mem.aff_ref[2].wrapping_add(io16(mem, 0x32) as i16 as i32);
    mem.aff_ref[3] = mem.aff_ref[3].wrapping_add(io16(mem, 0x36) as i16 as i32);

    let mut ui_objects=false;
    mem.ffta_dialogue.reuse_world_row=false;
    if mem.ffta_dialogue.separate && (mem.ffta_dialogue.placement.is_some() || mem.ffta_dialogue.extra.active()) {
        // BG2/BG3 are decoded once per scanline, shared by all foreground and
        // backing captures. Guest VRAM writes between scanlines remain visible.
        let rows=dialogue_native_rows(mem,dispcnt,line,&bg_used,&bg_lines);
        if mem.ffta_dialogue.placement.is_some() {ui_objects|=capture_dialogue_row(mem,dispcnt,line,&bg_lines,&bg_used,&obj_line,&obj_window,&rows,false);}
        if mem.ffta_dialogue.extra.active() {ui_objects|=capture_dialogue_row(mem,dispcnt,line,&bg_lines,&bg_used,&obj_line,&obj_window,&rows,true);}
        if mem.ffta_dialogue.battle_stats.active() {
            std::mem::swap(&mut mem.ffta_dialogue.extra,&mut mem.ffta_dialogue.battle_stats);
            ui_objects|=capture_dialogue_row(mem,dispcnt,line,&bg_lines,&bg_used,&obj_line,&obj_window,&rows,true);
            std::mem::swap(&mut mem.ffta_dialogue.extra,&mut mem.ffta_dialogue.battle_stats);
        }
        if mem.ffta_dialogue.service_speech.active() {
            std::mem::swap(&mut mem.ffta_dialogue.extra,&mut mem.ffta_dialogue.service_speech);
            ui_objects|=capture_dialogue_row(mem,dispcnt,line,&bg_lines,&bg_used,&obj_line,&obj_window,&rows,true);
            std::mem::swap(&mut mem.ffta_dialogue.extra,&mut mem.ffta_dialogue.service_speech);
        }
        capture_funds_row(mem,dispcnt,line);
    }
    let d=&mem.ffta_dialogue;
    let plain=d.separate && !d.reference_dialogue && d.placement.is_some() && !ui_objects
        && !d.naming_layout && !d.action_layout && !d.town_layout && !d.shop_layout && !d.site_layout && !d.service_layout
        && !d.attack_name && d.announcement_bounds.is_none() && d.funds_bounds.is_none()
        && !d.extra.active() && !d.battle_stats.active() && !d.service_speech.active()
        && !d.suspended_portrait && !d.suspended_stats && !mem.ffta_surface.capture()
        && (2..4).all(|bg|!ffta_terrain(mem,bg) && crate::ffta_stage::layer(mem,bg).is_none());
    // Deployment HUD removal is separate from dialogue ownership. A published
    // HUD OBJ must still go through the original two-pass path.
    let no_hud=plain && !mem.oam.chunks_exact(8).any(|o|crate::ffta_battle_hud::owns(mem,
        std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]]))));
    if no_hud && (ui_backgrounds_hidden(mem,dispcnt,line,&bg_used,&obj_window)
        || (2..4).all(|bg|!bg_used[bg] || bg_lines[bg][extra()..extra()+240].iter().all(|&p|p==TRANSPARENT))) {
        mem.ffta_dialogue.reuse_world_row=true;
        for bg in 2..4{bg_lines[bg][..render_width()].fill(TRANSPARENT);}
    }
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
    if !mem.ffta_dialogue.reuse_world_row && (mem.ffta_dialogue.placement.is_some() || mem.ffta_dialogue.extra.active()
        || mem.ffta_dialogue.suspended_portrait || mem.ffta_dialogue.suspended_stats) {
        // Keep the hardware framebuffer untouched. Only the host wide view
        // receives relocated UI; reuse the world BG rows already rendered.
        mem.ffta_dialogue.wide_pass = true;
        for bg in 0..4 {
            if bg_used[bg] && (bg>=2 || mem.ffta_dialogue.funds_bg==Some(bg)) {
                render_text_bg(mem, bg, line, &mut bg_lines[bg]);
            }
        }
        obj_line.fill(OBJ_EMPTY);
        obj_window.fill(false);
        let semi = if dispcnt & 0x1000 != 0 {
            render_sprites(mem, dispcnt, line, &mut obj_line, &mut obj_window)
        } else {false};
        compose(mem, dispcnt, line, &bg_lines, &bg_used, &obj_line, &obj_window, semi);
        mem.ffta_dialogue.wide_pass = false;
    }
    if line==159 {finish_shop_page(mem);}
}

/// Publish inventory UI only after a whole frame has sampled a completed page.
/// Guest VRAM can be rebuilt across scanlines; that intermediate work must not
/// replace the visible list. World rendering and emulated time keep running.
fn finish_shop_page(mem:&mut MemMap){
    let card=crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|matches!(s.id,66|67))
        && mem.ffta_dialogue.pub_window_owners[1].is_some();
    let state=&mut mem.ffta_dialogue;
    if !state.separate || !(state.shop_layout || card) || state.shop_numbers_exiting {
        state.shop_page_complete.clear();return;
    }
    if state.shop_page_dirty {
        if state.shop_page_complete.len()==state.pixels.len(){
            state.pixels.copy_from_slice(&state.shop_page_complete);
        }
    }else{state.shop_page_complete.clone_from(&state.pixels);}
}

/// Wide opaque warning window, identified by WIN0 and its four frame corners.
/// Text/locale is deliberately irrelevant; narrow Yes/No panels do not qualify.
fn action_menu_bounds(mem:&MemMap)->Option<[i32;4]> {
    // The live constructor retains its canonical rectangle before the first
    // tile/cursor and after input is disabled. Animation extents are separate.
    if mem.ffta_movement.europe_profile && io16(mem,12)==0x5c08 && io16(mem,14)==0x4a09 {
        let mut found:Option<([i32;4],(bool,bool,bool))>=None;
        for o in (0..mem.ewram.len().saturating_sub(44)).step_by(4) {
            let w=|a|u32::from_le_bytes(mem.ewram[a..a+4].try_into().unwrap());
            let h=|a|u16::from_le_bytes(mem.ewram[a..a+2].try_into().unwrap());
            if w(o)!=0x0600e000 || w(o+4)!=0x06005000 || h(o+20)!=0x0140
                || h(o+22)!=0x0c20 || w(o+24)!=0x00010005 {continue;}
            let [x,y,width,height]:[u8;4]=mem.ewram[o+16..o+20].try_into().unwrap();
            if width==0 || height<2 || x as u16+width as u16!=30 || y as u16+height as u16>20 {continue;}
            let current=[h(o+32),h(o+34)];let target=[h(o+36),h(o+38)];
            if current[0]>width as u16 || current[1]>height as u16 || target[0]>width as u16 || target[1]>height as u16 {continue;}
            if current[0]==0 && target[0]==0 {continue;}
            let rect=[x as i32*8,y as i32*8,240,(y as i32+height as i32)*8];
            let priority=(target[0]>0 && target[1]>0,target[0]>current[0],mem.ffta_dialogue.action_owner_bounds==Some(rect));
            if found.as_ref().is_none_or(|(_,old)|priority>*old){found=Some((rect,priority));}
        }
        if let Some((rect,_))=found{return Some(rect);}
    }
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
    if found.is_some(){return found;}
    let mut left=240;let mut top=160;let mut bottom=0;
    for y in 0..20 {for x in 0..30 {
        match tile(x,y) {
            Some(0xc005)=>{left=left.min(x*8);top=top.min(y*8);},
            Some(0xc009)=>{left=left.min(x*8);bottom=bottom.max((y+1)*8);},
            Some(0xc006)=>{top=top.min(y*8);},
            Some(0xc00a)=>{bottom=bottom.max((y+1)*8);},_=>{}
        }
    }}
    let previous=mem.ffta_dialogue.action_owner_bounds.filter(|r|r[0]==left as i32);
    if left<240 {Some([left as i32,if top<160{top as i32}else{previous.map_or(0,|r|r[1])},240,
        if bottom>top{bottom as i32}else{previous.map_or(160,|r|r[3])}])}else{None}
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
/// Match the complete strip before WIN clipping, including a 256px map seam.
/// The window's animated rectangle is a reveal mask, not the strip's identity.
fn action_banner_resource(mem:&MemMap)->Option<([i32;4],usize)> {
    if let Some(bounds)=action_banner_bounds(mem){return Some((bounds,0));}
    let base=((io16(mem,14)>>8)&31)as usize*0x800;
    let entry=|x:usize,y:usize|{let o=base+(y*32+x%32)*2;
        u16::from_le_bytes([mem.vram[o],mem.vram[o+1]])};
    let mut result=None;
    for y in 0..19 {for x in 0..32 {
        if entry(x,y)!=0xb032 || entry(x,y+1)!=0xb832 {continue;}
        for width in 4..=30 {
            if entry(x+width-1,y)!=0xb432 || entry(x+width-1,y+1)!=0xbc32
                || !(x+1..x+width-1).all(|xx|entry(xx,y)==0xb033 && entry(xx,y+1)==0xb833){continue;}
            if result.is_some(){return None;}
            let rotation=if x+width>30{x*8}else{0};
            let left=if rotation!=0{0}else{x*8};
            result=Some(([left as i32,y as i32*8,(left+width*8)as i32,(y+2)as i32*8],rotation));
        }
    }}
    result
}
fn engagement_bounds(mem:&MemMap)->Option<[i32;4]> {
    // Recorded red engagement frame: palette7, tile E0 with four flips.
    // Identify the complete BG canvas, not its temporary animation offset.
    if !ffta_terrain(mem,0) || io16(mem,0)&0x800==0
        || (io16(mem,12),io16(mem,14))!=(0x0b08,0x0a08){return None;}
    let tile=|x:usize,y:usize| {
        let o=0x5000+(y*32+x)*2;
        u16::from_le_bytes([mem.vram[o],mem.vram[o+1]])
    };
    for y in 0..18 {for x in 0..28 {
        if tile(x,y)!=0x70e0{continue;}
        for r in x+4..30 {if tile(r,y)!=0x74e0{continue;}
            for b in y+2..20 {
                if tile(x,b)==0x78e0 && tile(r,b)==0x7ce0
                    && tile(x+2,y)==0x70e2 && tile(x+2,y+1)==0x70e5 {
                    return Some([x as i32*8,y as i32*8,(r+1)as i32*8,(b+1)as i32*8]);
                }
            }
        }
    }}
    None
}
fn notice_bounds(mem:&MemMap)->Option<[i32;4]> {
    if let Some((rect,0))=text_window_owner(mem,4,16) {
        let [l,t,r,b]=rect.map(|v|v as usize);
        let base=((io16(mem,14)>>8)&31)as usize*0x800;
        let tile=|x:usize,y:usize|{let o=base+(y/8*32+x/8)*2;u16::from_le_bytes([mem.vram[o],mem.vram[o+1]])};
        if tile(l,t)==0xb00d && tile(r-8,t)==0xb00e && tile(l,b-8)==0xb00f && tile(r-8,b-8)==0xb010{return Some(rect);}
    }
    // Battle announcements (including Engage) reuse this rectangular frame
    // at other widths and animated WIN positions. Its complete frame, rather
    // than the temporary hardware clip, owns the centered host modal.
    let base=((io16(mem,14)>>8)&31)as usize*0x800;
    let tile=|x:usize,y:usize|{let o=base+(y*32+x)*2;u16::from_le_bytes([mem.vram[o],mem.vram[o+1]])};
    if io16(mem,0)&0x6000!=0x2000 || io16(mem,0x48)&63!=31 || io16(mem,0x4a)&32==0 {return None;}
    let clip_h=io16(mem,0x40);let clip_v=io16(mem,0x44);
    let clip=[(clip_h>>8)as usize,(clip_v>>8)as usize,(clip_h&255)as usize,(clip_v&255)as usize];
    if clip[0]>=clip[2] || clip[1]>=clip[3] {return None;}
    let mut framed=None;
    for y in 0..18 {for x in 0..21 {
        if tile(x,y)!=0xb00d {continue;}
        for r in x+9..30 {if tile(r,y)!=0xb00e {continue;}
            for b in y+2..20 {if tile(x,b)==0xb00f && tile(r,b)==0xb010 {
                // Old frame tiles survive menu changes. Only the active
                // hardware window can establish ownership of this frame.
                if clip[0]<x*8 || clip[1]<y*8 || clip[2]>(r+1)*8 || clip[3]>(b+1)*8 {continue;}
                if framed.is_some(){return None;}
                framed=Some([x as i32*8,y as i32*8,(r+1)as i32*8,(b+1)as i32*8]);
            }}
        }
    }}
    if framed.is_some(){return framed;}
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
/// Original compact text-window owner, before its animated dimensions/clip.
/// Shares the measured Notice constructor; full dialogue windows use a
/// different type and are deliberately excluded.
fn compact_text_owner(mem:&MemMap)->Option<([i32;4],usize)> {
    text_window_owner(mem,1,3)
}
fn text_window_owner(mem:&MemMap,min_height:u8,max_height:u8)->Option<([i32;4],usize)> {
    styled_text_window_owner(mem,min_height,max_height,0x0800,0x0003000d)
}
/// The executed ability capsule has its own guest constructor/style. Its BG
/// resources are also used by battle menus, so BG register identity alone
/// would split this one window into attacker and action-list canvases.
fn executed_attack_owner(mem:&MemMap)->Option<([i32;4],usize)> {
    styled_text_window_owner(mem,2,2,0x0404,0x00030032)
}
fn executed_attack_exiting(mem:&MemMap)->bool {
    let maps=[12,14].map(|r|0x06000000+(((io16(mem,r)>>8)&31)as u32)*0x800);
    (0..mem.ewram.len().saturating_sub(44)).step_by(4).any(|o|{
        let w=|a|u32::from_le_bytes(mem.ewram[a..a+4].try_into().unwrap());
        let h=|a|u16::from_le_bytes(mem.ewram[a..a+2].try_into().unwrap());
        w(o)==maps[0] && w(o+4)==maps[1] && w(o+24)==0x00030032
            && h(o+20)==0x0404 && h(o+22)==0x0b20 && mem.ewram[o+19]==2
            && h(o+36)==0 && h(o+38)==0
    })
}
fn styled_text_window_owner(mem:&MemMap,min_height:u8,max_height:u8,style:u16,kind:u32)->Option<([i32;4],usize)> {
    let maps=[12,14].map(|r|0x06000000+(((io16(mem,r)>>8)&31)as u32)*0x800);
    let mut result=None;
    for o in (0..mem.ewram.len().saturating_sub(44)).step_by(4){
        let w=|a|u32::from_le_bytes(mem.ewram[a..a+4].try_into().unwrap());
        let h=|a|u16::from_le_bytes(mem.ewram[a..a+2].try_into().unwrap());
        if w(o)!=maps[0] || w(o+4)!=maps[1] || w(o+24)!=kind
            || h(o+20)!=style || h(o+22)!=0x0b20 {continue;}
        let [x,y,width,height]:[u8;4]=mem.ewram[o+16..o+20].try_into().unwrap();
        if !(4..=30).contains(&width) || !(min_height..=max_height).contains(&height) || x>=32
            || y as u16+height as u16>20 || h(o+32)>width as u16 || h(o+36)>width as u16
            || h(o+34)>height as u16 || h(o+38)>height as u16
            || h(o+32)==0 && h(o+36)==0 && kind!=0x00030032 {continue;}
        let rotation=if x as u16+width as u16>30{x as usize*8}else{0};
        let left=if rotation==0{x as i32*8}else{0};
        if result.is_some(){return None;}
        result=Some(([left,y as i32*8,left+width as i32*8,(y as i32+height as i32)*8],rotation));
    }
    result
}
// Battle animations can reuse the exact BG2/BG3 dialogue layout. In the
// recorded Teleport frames there is no text controller, only a spell image.
// Use the verified EUR text interpreter's owner, including all its animation
// phases; resource addresses alone do not establish a dialogue canvas.
fn dialogue_controller_present(mem: &MemMap) -> bool {
    if !mem.ffta_movement.europe_profile
        || mem.rom.get(0x1194c..0x11954) != Some(&[0x30,0xb5,0x03,0x48,0x03,0x68,0x05,0x1c]) {
        return true;
    }
    let owner=u32::from_le_bytes(mem.ewram[0x918c..0x9190].try_into().unwrap());
    owner & 3 == 0 && (0x02000000..=0x0203ffbc).contains(&owner)
}
fn stats_object([a,_,c]:[u16;3])->bool {
    a&0x2000==0 && a&0x300!=0x200 && (a>>10)&3<=1
        && c&0xc00==0 && matches!(c>>12,11|12) && (0x1c8..0x240).contains(&(c&1023))
}
fn stats_backing(a:[u16;3])->bool {
    stats_object(a) && a[2]>>12==11 && matches!(a[2]&1023,0x1f0|0x200|0x208)
}
fn detached_action_portrait(mem:&MemMap,attrs:[u16;3],x:i32,y:i32)->bool {
    crate::ffta_dialogue::action_portrait(attrs,x,y)
        && (ffta_terrain(mem,2) || ffta_terrain(mem,0)
            && (matches!((io16(mem,12),io16(mem,14)),(0x5c08,0x4a09))
                // Attack confirmation switches to the dialogue/ability
                // resources before the retired combat portrait leaves OBJ.
                // Genuine dialogue portraits use priority zero; preserve them.
                || attrs[2]&0xc00==0x400
                    && matches!((io16(mem,12),io16(mem,14)),(0x0b08,0x0a08))))
}
fn begin_dialogue_frame(mem: &mut MemMap) {
    mem.ffta_dialogue.menu_pattern=repeating_menu_background(mem);
    mem.ffta_dialogue.world_paper=ffta_world_map(mem,0)
        && mem.ffta_movement.europe_profile
        && [0x5000,0x6000].into_iter().all(|base|
            (0..44).all(|y| (30..32).all(|x| {
                let o=base+y/32*2048+(y%32*32+x)*2;
                mem.vram[o..o+2]==[0,0]
            })));
    if mem.ffta_dialogue.world_paper {
        use std::hash::Hasher;
        let mut hash=std::collections::hash_map::DefaultHasher::new();
        hash.write(&mem.vram[..0x7000]);hash.write(&mem.palette[..512]);hash.write(&mem.io[0x50..0x54]);
        let key=hash.finish();
        if key!=mem.ffta_dialogue.world_fog_key || mem.ffta_dialogue.world_fog.len()!=352*64 {
            mem.ffta_dialogue.world_ocean=world_ocean_colors(mem);
            mem.ffta_dialogue.world_fog_key=key;
            mem.ffta_dialogue.world_fog.resize_with(352*64,||std::sync::atomic::AtomicU16::new(TRANSPARENT));
            for p in &mem.ffta_dialogue.world_fog {p.store(TRANSPARENT,std::sync::atomic::Ordering::Relaxed);}
        }
    }
    mem.ffta_dialogue.title_paper=mem.ffta_movement.europe_profile && mem.ffta_movement.enabled
        && io16(mem,0)&0x0187==0x0100
        && [io16(mem,8),io16(mem,10),io16(mem,12)]==[0x3d83,0x3e0a,0x3f08];
    mem.ffta_dialogue.suspended_stats=mem.ffta_dialogue.separate
        && (ffta_terrain(mem,2) || ffta_terrain(mem,0)
            && io16(mem,12)==0x5c08 && io16(mem,14)==0x4a09)
        && (mem.ffta_sprites.active.as_deref().unwrap_or(&[]).iter().any(|s|stats_backing(s.attrs))
            || mem.oam.chunks_exact(8).any(|o|
                stats_backing(std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]])))));
    // A Move confirmation can have no detached panel at all, yet publish
    // host-only portrait pieces outside the native LCD. Such frames still
    // need the separate world OBJ pass; native/UI capture stays untouched.
    mem.ffta_dialogue.suspended_portrait=mem.ffta_dialogue.separate
        && (mem.ffta_sprites.active.as_deref().unwrap_or(&[]).iter().any(|s|
                detached_action_portrait(mem,s.attrs,s.x,s.y))
            || mem.oam.chunks_exact(8).any(|o| {
                let attrs=std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]]));
                let x=((attrs[1]&511)as i32+256)%512-256;
                let y=(attrs[0]&255)as i32;
                detached_action_portrait(mem,attrs,x,if y>=160{y-256}else{y})
            }));
    let was_action=mem.ffta_dialogue.action_layout;
    let context=[io16(mem,0)&7,io16(mem,8),io16(mem,10),io16(mem,12),io16(mem,14)];
    let previous_panel=if mem.ffta_dialogue.battle_stats.active(){&mem.ffta_dialogue.battle_stats}else{&mem.ffta_dialogue.extra};
    let previous_stats=if previous_panel.kind==crate::ffta_dialogue::PanelKind::Stats
        && mem.ffta_dialogue.stats_context==Some(context) {
        previous_panel.bounds.zip(mem.ffta_dialogue.stats_target)
    }else{None};
    mem.ffta_dialogue.wide_pass = false;
    mem.ffta_dialogue.placement = None;
    mem.ffta_dialogue.anchor_bounds = None;
    mem.ffta_dialogue.native_bounds=None;
    mem.ffta_dialogue.action_layout=false;
    mem.ffta_dialogue.naming_layout=false;
    mem.ffta_dialogue.action_banner=None;
    mem.ffta_dialogue.announcement_bounds=None;
    mem.ffta_dialogue.confirmation_marker=false;
    mem.ffta_dialogue.action_banner_rotation=0;
    mem.ffta_dialogue.town_layout=false;
    mem.ffta_dialogue.shop_layout=false;
    mem.ffta_dialogue.shop_page_dirty=crate::ffta_shop_mouse::redrawing(mem)
        || mem.ffta_battle_mouse.card_redrawing || mem.ffta_battle_mouse.bank_redrawing;
    mem.ffta_dialogue.site_layout=false;
    mem.ffta_dialogue.service_layout=false;
    mem.ffta_dialogue.funds_bounds=None;
    mem.ffta_dialogue.funds_bg=None;
    mem.ffta_dialogue.funds_pixels.clear();
    mem.ffta_dialogue.service_list=false;
    mem.ffta_dialogue.service_list_bounds=None;
    mem.ffta_dialogue.attack_name=false;
    mem.ffta_dialogue.attack_exiting=false;
    mem.ffta_dialogue.attack_rotation=0;
    mem.ffta_dialogue.stats_target=None;
    mem.ffta_dialogue.split_speech=false;
    mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::None;
    mem.ffta_dialogue.extra.bounds=None;
    mem.ffta_dialogue.battle_stats.kind=crate::ffta_dialogue::PanelKind::None;
    mem.ffta_dialogue.battle_stats.bounds=None;
    if mem.ffta_dialogue.separate && crate::ffta_mouse::name_entry_active(mem) {
        // Name entry shares dialogue tiles but is a whole native interface,
        // not a speech bubble or an executed-attack capsule.
        mem.ffta_dialogue.naming_layout=true;
        let scroll=io16(mem,24)&511;
        mem.ffta_dialogue.naming_phase=if scroll==0{2}
            else if matches!(mem.ffta_dialogue.naming_phase,2|3){3}else{1};
        mem.ffta_dialogue.suspended_portrait=false;mem.ffta_dialogue.suspended_stats=false;
        mem.ffta_dialogue.attachment=None;
        mem.ffta_dialogue.native_bounds=Some([0,0,240,160]);
        mem.ffta_dialogue.placement=Some(crate::ffta_dialogue::Placement::new(false,render_width(),crate::mem::extra_y(),Default::default()));
        mem.ffta_dialogue.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
        mem.ffta_dialogue.pixels.fill(Default::default());
        return;
    }
    mem.ffta_dialogue.naming_phase=0;
    let pub_windows=if mem.ffta_dialogue.separate {
        crate::ffta_ui_window::pub_windows(mem,mem.ffta_dialogue.pub_window_owners)
    }else{[None;3]};
    mem.ffta_dialogue.pub_window_owners=pub_windows.map(|w|w.map(|w|w.owner));
    if crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==65) {
        if let Some(w)=pub_windows[1] {
            mem.ffta_dialogue.shop_numbers_exiting=matches!(w.phase,
                crate::ffta_ui_window::Phase::Exiting|crate::ffta_ui_window::Phase::Hidden);
        }
    }else{mem.ffta_dialogue.shop_numbers_exiting=false;}

    let pub_root=pub_windows[0].filter(|w|w.visible());
    let pub_content=pub_windows[1].filter(|w|w.visible());
    let pub_speech=pub_windows[2].filter(|w|w.visible());
    mem.ffta_dialogue.service_speech.kind=crate::ffta_dialogue::PanelKind::None;
    mem.ffta_dialogue.service_speech.bounds=None;
    mem.ffta_dialogue.service_speech_frame=pub_speech.map(|w|w.frame_bounds);
    if let Some(w)=pub_speech {
        mem.ffta_dialogue.service_speech.kind=crate::ffta_dialogue::PanelKind::Speech;
        mem.ffta_dialogue.service_speech.bounds=Some(w.bounds);
        mem.ffta_dialogue.service_speech.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
        mem.ffta_dialogue.service_speech.pixels.fill(Default::default());
        // The native portrait provides its side; retain it through animation
        // frames where the portrait OBJ is not yet/any longer published.
        if let Some(right)=mem.oam.chunks_exact(8).find_map(|o|{
            let a=std::array::from_fn(|i|u16::from_le_bytes([o[2*i],o[2*i+1]]));
            let x=(a[1]&511)as i32;
            crate::ffta_dialogue::portrait(a,x).then_some(x>=120)
        }){mem.ffta_dialogue.service_speech_right=right;}
        else if w.phase==crate::ffta_ui_window::Phase::Entering {mem.ffta_dialogue.service_speech_right=w.bounds[0]+w.bounds[2]>=240;}
    }
    let shared_menus=if mem.ffta_dialogue.separate && !mem.ffta_dialogue.reference_frame {
        Some(crate::ffta_battle_mouse::MenuScene::layout_menus(mem))
    }else{None};
    let service_candidate=mem.ffta_dialogue.separate
        && crate::ffta_surface::Descriptor::read(mem).is_some()
        && io16(mem,0xc)==0x0b08 && io16(mem,0xe)==0x0a08;
    let visual_root=if service_candidate && shared_menus.is_none_or(|m|m.3) {crate::ffta_battle_mouse::MenuScene::service_menu_visual(mem)}else{None};
    let service=service_candidate && (crate::ffta_surface::Descriptor::read(mem)
        .is_some_and(|s|matches!(s.id,62|63|65|66|67)) || visual_root.is_some());
    let site_menu=if let Some(m)=shared_menus {m.0}else if mem.ffta_dialogue.separate {
        crate::ffta_battle_mouse::MenuScene::site_menu(mem)
    }else{None};
    let terrain=(0..4).any(|bg|ffta_terrain(mem,bg));
    if (extra()==0 && crate::mem::extra_y()==0) || !crate::ffta_dialogue::enabled()
        || (!mem.ffta_movement.enabled && site_menu.is_none()) || mem.ffta_bg_edge==FftaBgEdge::Wrap
        || io16(mem,0)&0x87!=0 || (!terrain && site_menu.is_none()) {mem.ffta_dialogue.attachment=None;return;}
    mem.ffta_dialogue.service_layout=service;
    if !service {
        if let Some(bounds)=engagement_bounds(mem) {
            mem.ffta_dialogue.announcement_bounds=Some(bounds);
            mem.ffta_dialogue.native_bounds=Some(bounds);
            mem.ffta_dialogue.anchor_bounds=Some((bounds[1]as usize,bounds[3]as usize));
            mem.ffta_dialogue.attachment=None;
            mem.ffta_dialogue.placement=Some(crate::ffta_dialogue::Placement::new(false,
                render_width(),crate::mem::extra_y(),mem.ffta_dialogue.style));
            mem.ffta_dialogue.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
            mem.ffta_dialogue.pixels.fill(Default::default());
            return;
        }
    }
    if service {
        if let Some((bg,bounds))=crate::ffta_ui::funds_window(mem) {
            mem.ffta_dialogue.funds_bg=Some(bg);mem.ffta_dialogue.funds_bounds=Some(bounds);
        }else if !mem.ffta_movement.europe_profile || crate::ffta_surface::Descriptor::read(mem).is_none_or(|s|!matches!(s.id,62|66)) {
            mem.ffta_dialogue.funds_bounds=crate::ffta_ui::funds_bounds(mem);
        }
        if mem.ffta_dialogue.funds_bounds.is_some(){mem.ffta_dialogue.funds_pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());}
    }
    if mem.ffta_dialogue.separate {
        let mut bounds=[240,160,0,0];let mut pieces=[0;3];let mut stats_origin=None;
        for obj in mem.oam.chunks_exact(8) {
            let a=u16::from_le_bytes([obj[0],obj[1]]);let b=u16::from_le_bytes([obj[2],obj[3]]);let c=u16::from_le_bytes([obj[4],obj[5]]);
            if !stats_backing([a,b,c]){continue;}
            let index=match c&1023 {0x1f0=>0,0x200=>1,0x208=>2,_=>continue};
            let shape=(a>>14) as usize;if shape>=3 {continue;}
            let (w,h)=OBJ_SIZES[shape][(b>>14) as usize];
            let raw_x=(b&511)as i32;let raw_y=(a&255)as i32;
            let x=if raw_x>=256{raw_x-512}else{raw_x};
            let y=if raw_y>=160{raw_y-256}else{raw_y};
            // All four tagged backing pieces identify the panel even while
            // some are entering/leaving the native viewport.
            if index==0 {mem.ffta_dialogue.stats_right=b&0x1000==0;}
            let origin_x=if b&0x1000!=0{x+w-88}else{x};
            if index==0 {stats_origin=Some((origin_x,y));}
            else if index==1 && stats_origin.is_none(){
                stats_origin=Some((origin_x,y-32));mem.ffta_dialogue.stats_right=b&0x1000==0;
            }
            pieces[index]+=1;bounds=[bounds[0].min(x),bounds[1].min(y),bounds[2].max(x+w),bounds[3].max(y+h)];
        }
        if let Some((x,y))=stats_origin {bounds=[x,y,x+88,y+48];}
        let partial=(previous_stats.is_some() || stats_origin.is_some()) && pieces.iter().any(|n|*n>0);
        if pieces==[1,1,2] || partial {
            mem.ffta_dialogue.stats_context=Some(context);
            if stats_origin.is_none() && partial && pieces!=[1,1,2] {bounds=previous_stats.unwrap().0;}
            mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Stats;
            mem.ffta_dialogue.extra.background.resize(render_width()*crate::mem::render_height(),0);
            mem.ffta_dialogue.extra.bounds=Some(bounds);
            let left=if mem.ffta_dialogue.stats_right {152}else{0};
            let top=104;
            mem.ffta_dialogue.stats_target=Some([left,top,left+88,top+48]);
            mem.ffta_dialogue.extra.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
            mem.ffta_dialogue.extra.pixels.fill(Default::default());
        }
    }
    let pub_scene=if let Some(m)=shared_menus {m.1}else if mem.ffta_dialogue.separate {crate::ffta_battle_mouse::MenuScene::pub_menu(mem)}else{None};
    let shop_root=if let Some(m)=shared_menus {m.2}else if mem.ffta_dialogue.separate {crate::ffta_battle_mouse::MenuScene::shop_menu(mem)}else{None};
    let shop=mem.ffta_dialogue.separate && crate::ffta_shop_mouse::visible(mem);
    let service_menu=if service && !shop {pub_scene.or(shop_root).or(site_menu)
        .or(visual_root)}else{None};
    let usa_shop_root=!mem.ffta_movement.europe_profile && shop_root.is_some();
    let site=usa_shop_root || (site_menu.is_some() && pub_scene.is_none() && shop_root.is_none() && !shop);
    let town_scene=if service && shop {None}else{service_menu.or(pub_scene).or(shop_root).or(if site {site_menu}else{None})};
    let town=town_scene.is_some() || service;
    mem.ffta_dialogue.shop_layout=shop;
    mem.ffta_dialogue.town_layout=town;
    mem.ffta_dialogue.site_layout=site;
    mem.ffta_dialogue.service_list=pub_content.is_some() || (pub_root.is_none() && service && (shop ||
        crate::ffta_battle_mouse::MenuScene::town_menu(mem).is_some_and(|m|
            !((m.count==3 || m.count==4) && m.origin==[8,92])
            && pub_scene.is_none() && shop_root.is_none())));

    if let Some(w)=pub_content {mem.ffta_dialogue.service_list_bounds=Some(w.bounds);}
    else if shop {mem.ffta_dialogue.service_list_bounds=Some([8,0,224,160]);}
    else if mem.ffta_dialogue.service_list {
        if let Some(m)=crate::ffta_battle_mouse::MenuScene::town_menu(mem) {
            mem.ffta_dialogue.service_list_bounds=Some([(m.origin[0]-8).max(0),(m.origin[1]-16).max(0),
                m.right,(m.origin[1]+m.count as i32*m.pitch+4).min(160)]);
        }
    }
    if town || shop {
        // Town planes are captured as complete pixels, not action backing.
        // Discard any backing retained from the preceding battle panel.
        mem.ffta_dialogue.backing.clear();
        mem.ffta_dialogue.extra.backing.clear();
    }
    let dialogue=io16(mem,0)&0xc00!=0 && io16(mem,0xc)==0xb08 && io16(mem,0xe)==0xa08 && ffta_terrain(mem,0);
    let action=mem.ffta_dialogue.separate && io16(mem,0)&0xc00!=0
        && io16(mem,0xc)==0x5c08 && io16(mem,0xe)==0x4a09
        && ffta_terrain(mem,0);
    let notice=mem.ffta_dialogue.separate.then(||notice_bounds(mem)).flatten();
    if !dialogue && !action && !town && !shop && notice.is_none() {
        mem.ffta_dialogue.attachment=None;return;
    }
    if action {mem.ffta_dialogue.attachment=None;}
    if !action || !was_action {mem.ffta_dialogue.action_owner_bounds=None;}
    mem.ffta_dialogue.action_layout=action;
    if !mem.ffta_movement.europe_profile {
        if let Some(menu)=pub_scene {
            mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Actions;
            mem.ffta_dialogue.extra.bounds=Some([menu.origin[0]-8,menu.origin[1]-16,menu.right-8,
                (menu.origin[1]+menu.count as i32*menu.pitch+4).min(160)]);
            mem.ffta_dialogue.extra.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
            mem.ffta_dialogue.extra.pixels.fill(Default::default());
        }
    }
    if action {
        mem.ffta_dialogue.action_owner_bounds=action_menu_bounds(mem);
        if let Some((bounds,rotation))=action_banner_resource(mem){
            mem.ffta_dialogue.action_banner=Some(bounds);
            mem.ffta_dialogue.action_banner_rotation=rotation;
        }
        if mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats {
            if let Some(bounds)=action_menu_bounds(mem) {
                std::mem::swap(&mut mem.ffta_dialogue.extra,&mut mem.ffta_dialogue.battle_stats);
                mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Actions;
                mem.ffta_dialogue.extra.bounds=Some(bounds);
            }
        }
    }
    if let Some(bounds)=notice {
        mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Notice;
        mem.ffta_dialogue.extra.bounds=Some(bounds);
        mem.ffta_dialogue.extra.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
        mem.ffta_dialogue.extra.pixels.fill(Default::default());
    }
    // Classify backing geometry before guest HOFS/VOFS animation, not only
    // pixels currently touching a viewport edge. Never retain old UI pixels.
    let menu_bounds=if let Some(window)=pub_root {Some(window.bounds)}else if pub_content.is_some(){None}else if site || service {town_scene.map(|m|[m.origin[0]-8,(m.origin[1]-16).max(0),m.right,(m.origin[1]+m.count as i32*m.pitch+4).min(160)])}else{None};
    let geometry_exclusion=if action {
        mem.ffta_dialogue.extra.bounds.or_else(||action_menu_bounds(mem))
    }else{menu_bounds};
    let mut edge=[0usize;2];let mut bounds=[240usize,160,0,0];
    let mut row=[TRANSPARENT;MAX_RENDER_WIDTH];
    for y in 0..160 {
        if service {render_text_bg_hardware(mem,3,y,&mut row,0,240);}
        else {render_text_bg_hardware_scroll(mem,3,y,&mut row,0,240,Some((0,0)));}
        for x in 0..240 {
            if row[x]==TRANSPARENT || mem.ffta_dialogue.action_banner.is_some_and(|[l,t,r,b]|
                x as i32>=l && (x as i32)<r && y as i32>=t && (y as i32)<b) || geometry_exclusion.is_some_and(|[l,t,r,b]|x as i32>=l && (x as i32)<r && y as i32>=t && (y as i32)<b){continue;}
            edge[0]+=usize::from(x==0);edge[1]+=usize::from(x==239);
            bounds=[bounds[0].min(x),bounds[1].min(y),bounds[2].max(x+1),bounds[3].max(y+1)];
        }
    }
    if bounds[0]<bounds[2] {
        mem.ffta_dialogue.native_bounds=Some(bounds.map(|v|v as i32));
    }
    let portrait=mem.oam.chunks_exact(8).find_map(|o|{
        let a=u16::from_le_bytes([o[0],o[1]]);let b=u16::from_le_bytes([o[2],o[3]]);let c=u16::from_le_bytes([o[4],o[5]]);
        let x=(b&511)as i32;let y=(a&255)as usize;
        (crate::ffta_dialogue::portrait([a,b,c],x) && y<160).then_some((x>=120,y))
    });
    // Teleport reuses the dialogue atlas for a compact 14x30 silhouette.
    // Reject ONLY that detached-layer candidate. Do not disable general UI
    // ownership or foreground clipping: real portraits and transitional
    // panels can remain visible after the text interpreter releases its owner.
    let width=bounds[2].saturating_sub(bounds[0]);
    let height=bounds[3].saturating_sub(bounds[1]);
    if dialogue && !town && !shop && notice.is_none() && portrait.is_none()
        && !mem.ffta_dialogue.extra.active() && !dialogue_controller_present(mem)
        && ((8..=32).contains(&width) && (25..=48).contains(&height)
            || crate::ffta_effects::shared_dialogue_picture(mem,3)) {
        mem.ffta_dialogue.attachment=None;
        mem.ffta_dialogue.native_bounds=None;
        return;
    }
    let executed=if action && !town && !shop && notice.is_none(){executed_attack_owner(mem)}else{None}
        // Forecast and executed-name capsules share their constructor. The
        // forecast ALSO has a lower BG information panel: retain that complete
        // panel and its portrait on the left, with the help strip above it.
        .filter(|(rect,_)|bounds[0]>=bounds[2] || bounds[1]>=rect[1]as usize && bounds[3]<=rect[3]as usize);
    if executed.is_some() {
        mem.ffta_dialogue.attack_exiting=executed_attack_exiting(mem);
    }
    if executed.is_some() {
        // This is the yellow executed-attack capsule, not the blue forecast.
        mem.ffta_dialogue.action_banner=None;
        mem.ffta_dialogue.action_banner_rotation=0;
    }
    let compact=executed.or_else(||if (dialogue || action) && !town && !shop && notice.is_none()
        && mem.ffta_dialogue.action_banner.is_none(){compact_text_owner(mem)}else{None});
    mem.ffta_dialogue.attack_name=compact.is_some() || dialogue && !town && !shop && notice.is_none() && portrait.is_none()
        && bounds[0]<bounds[2] && bounds[3]-bounds[1]<=24;
    if mem.ffta_dialogue.attack_name {
        // A capsule may straddle the 256px hardware map seam. Unwrap both
        // foreground and backing with ONE rotation, not independent pieces.
        let mut occupied=[false;256];
        for y in 0..160 {
            let mut r=[TRANSPARENT;MAX_RENDER_WIDTH];
            render_text_bg_hardware_scroll(mem,3,y,&mut r,0,256,Some((0,0)));
            for x in 0..256 {occupied[x]|=r[x]!=TRANSPARENT;}
        }
        let mut best=(0,0);
        for start in 0..256 {if !occupied[start] {
            let len=(0..256).take_while(|d|!occupied[(start+d)%256]).count();
            if len>best.1 {best=(start,len);}
        }}
        if best.1>0 {
            mem.ffta_dialogue.attack_rotation=(best.0+best.1)%256;
            mem.ffta_dialogue.native_bounds=Some([0,bounds[1]as i32,(256-best.1)as i32,bounds[3]as i32]);
        }
        if let Some((rect,rotation))=compact {
            mem.ffta_dialogue.native_bounds=Some(rect);
            mem.ffta_dialogue.attack_rotation=rotation;
        }
    }
    let action=action && !mem.ffta_dialogue.attack_name;
    mem.ffta_dialogue.action_layout=action;
    if mem.ffta_dialogue.attack_name && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Actions {
        mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::None;
        mem.ffta_dialogue.extra.bounds=None;
    }
    let speech=dialogue && notice.is_none() && (!town && !shop || site && (bounds[0]<bounds[2] || portrait.is_some()));
    if speech {
        let previous=mem.ffta_dialogue.attachment;
        let side=if edge[0]!=edge[1] {Some(edge[1]>edge[0])}
            else {portrait.map(|p|p.0).or(previous.map(|p|p.0)).or_else(||(bounds[0]<bounds[2]).then_some(bounds[0]+bounds[2]>=240))};
        if let Some(right)=side {
            let anchor=if bounds[0]<bounds[2] {if bounds[1]+bounds[3]>=160 {160.}else{0.}}
                else {previous.map(|p|p.1).unwrap_or_else(||if portrait.is_some_and(|p|p.1>=80){160.}else{0.})};
            mem.ffta_dialogue.attachment=Some((right,anchor));
        }
    }else{mem.ffta_dialogue.attachment=None;}
    if !town && !shop && !mem.ffta_dialogue.attack_name
        && mem.ffta_dialogue.extra.kind!=crate::ffta_dialogue::PanelKind::Notice
        && (if action {bounds[0]>=bounds[2] && !mem.ffta_dialogue.extra.active()}else{mem.ffta_dialogue.attachment.is_none()}) {return;}
    // Separate a service's speech and menu: only the latter keeps its centered
    // native canvas. Both are captured from this frame, including transitions.
    let split=(site && speech) || (service && !mem.ffta_dialogue.service_list && menu_bounds.is_some());
    mem.ffta_dialogue.split_speech=split;
    if split {
        if let Some(b)=menu_bounds {
            mem.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Actions;
            mem.ffta_dialogue.extra.bounds=Some(b);
            mem.ffta_dialogue.extra.pixels.resize(crate::ffta_dialogue::LAYER_WIDTH*160,Default::default());
            mem.ffta_dialogue.extra.pixels.fill(Default::default());
        }
    }
    let right=mem.ffta_dialogue.attachment.map(|p|p.0).unwrap_or(town||shop||(!action&&edge[1]>edge[0]));
    let mut placement=crate::ffta_dialogue::Placement::new(right,render_width(),crate::mem::extra_y(),mem.ffta_dialogue.style);
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
    if action || service || mem.ffta_dialogue.attack_name || (town && !split) || shop || notice.is_some() {placement.length_pixels=0;}
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
    mem.ffta_dialogue.confirmation_marker=crate::ffta_battle_mouse::MenuScene::read(mem)
        .is_some_and(|m|m.is_confirmation());
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

/// Capture the verified gold BG frame or the separate OBJ inventory counter.
/// Never crop a guessed rectangle from an unrelated blue content panel.
fn capture_funds_row(mem:&mut MemMap,dispcnt:u16,line:usize) {
    use crate::ffta_dialogue::{UiPixel,LAYER_WIDTH,LAYER_INSET};
    let Some([_,t,_,b])=mem.ffta_dialogue.funds_bounds else{return;};
    if line<t as usize || line>=b as usize || dispcnt&0x1000==0{return;}
    if let (Some(bg),Some([l,_,r,_]))=(mem.ffta_dialogue.funds_bg,mem.ffta_dialogue.funds_bounds) {
        let mut row=[TRANSPARENT;MAX_RENDER_WIDTH];render_text_bg_hardware(mem,bg,line,&mut row,0,240);
        for x in l.max(0)..r.min(240) {
            let color=row[x as usize];
            if color!=TRANSPARENT {mem.ffta_dialogue.funds_pixels[line*LAYER_WIDTH+(x+LAYER_INSET)as usize]=UiPixel::opaque(color);}
        }
        return;
    }
    let mut objects=[OBJ_EMPTY;MAX_RENDER_WIDTH];let mut windows=[false;MAX_RENDER_WIDTH];
    mem.ffta_dialogue.capture=true;mem.ffta_dialogue.capture_funds=true;
    render_sprites_extent(mem,dispcnt,line,&mut objects,&mut windows,LAYER_INSET as usize,LAYER_WIDTH);
    mem.ffta_dialogue.capture_funds=false;mem.ffta_dialogue.capture=false;
    let alpha=io16(mem,0x52);let bld=io16(mem,0x50);let evy=(io16(mem,0x54)&31)as u32;
    for (x,o) in objects[..LAYER_WIDTH].iter().enumerate() {
        if o.priority==255{continue;}
        let p=if o.semi {UiPixel{color:o.color,eva:(alpha&31).min(16)as u8,evb:((alpha>>8)&31).min(16)as u8}}
            else if bld&16!=0 && bld>>6&3==2 {UiPixel::opaque(brighten(o.color,evy))}
            else if bld&16!=0 && bld>>6&3==3 {UiPixel::opaque(darken(o.color,evy))}
            else {UiPixel::opaque(o.color)};
        mem.ffta_dialogue.funds_pixels[line*LAYER_WIDTH+x]=p;
    }
}

fn dialogue_native_rows(mem:&MemMap,dispcnt:u16,line:usize,used:&[bool;4],world:&[Line;4])->[Line;2] {
    let mut rows=[[TRANSPARENT;MAX_RENDER_WIDTH];2];
    let unscrolled=mem.ffta_dialogue.action_layout || mem.ffta_dialogue.attack_name
        || mem.ffta_dialogue.announcement_bounds.is_some();
    let width=if mem.ffta_dialogue.attack_name || mem.ffta_dialogue.action_banner_rotation!=0{256}else{240};
    for (i,bg) in [2,3].into_iter().enumerate() {if used[bg] {
        if !mem.ffta_dialogue.reference_dialogue && !unscrolled && width==240 && reusable_native_ui_row(mem,dispcnt,bg,line) {
            rows[i][..240].copy_from_slice(&world[bg][extra()..extra()+240]);
            continue;
        }
        if unscrolled {
            // Remove the common slide, retaining the text-to-frame offset.
            let scroll=if mem.ffta_dialogue.attack_name && bg==2 {
                (((io16(mem,0x18).wrapping_sub(io16(mem,0x1c)))&511)as usize,
                 ((io16(mem,0x1a).wrapping_sub(io16(mem,0x1e)))&511)as usize)
            }else{(0,0)};
            render_text_bg_hardware_scroll(mem,bg,line,&mut rows[i],0,width,Some(scroll));
        }
        else {render_text_bg_hardware(mem,bg,line,&mut rows[i],0,width);}
        if mem.ffta_dialogue.attack_name {rows[i][..256].rotate_left(mem.ffta_dialogue.attack_rotation);}
        else if mem.ffta_dialogue.action_banner_rotation!=0
            && mem.ffta_dialogue.action_banner.is_some_and(|b|line as i32>=b[1] && (line as i32)<b[3]) {
            rows[i][..256].rotate_left(mem.ffta_dialogue.action_banner_rotation);
        }
    }}
    rows
}
/// Extract native UI plus GBA blend coefficients. Guest buffers are never modified.
#[inline]
fn insert_ui_target(top:&mut [(u16,usize,u8,bool);2],count:&mut usize,value:(u16,usize,u8,bool)) {
    let key=|v:(u16,usize,u8,bool)|(v.2,if v.1>=4 {0}else{v.1+1});
    if *count==0 || key(value)<key(top[0]) {
        if *count>0 {top[1]=top[0];}
        top[0]=value;
    }else if *count==1 || key(value)<key(top[1]) {top[1]=value;}
    *count=(*count+1).min(2);
}
fn capture_dialogue_row(mem:&mut MemMap,dispcnt:u16,line:usize,
    world_bg:&[Line;4],used:&[bool;4],world_obj:&[ObjPixel;MAX_RENDER_WIDTH],
    world_window:&[bool;MAX_RENDER_WIDTH],rows:&[Line;2],extra_panel:bool)->bool {
    let timer=mem.ffta_dialogue.profile.enabled.then(std::time::Instant::now);
    let mut objects=capture_dialogue_part(mem,dispcnt,line,world_bg,used,world_obj,world_window,rows,extra_panel,false);
    if mem.ffta_dialogue.action_layout || (extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats) {
        objects|=capture_dialogue_part(mem,dispcnt,line,world_bg,used,world_obj,world_window,rows,extra_panel,true);
    }
    if let Some(t)=timer{mem.ffta_dialogue.profile.capture_ns+=t.elapsed().as_nanos()as u64;mem.ffta_dialogue.profile.rows+=1;}
    objects
}
fn capture_dialogue_part(mem:&mut MemMap, dispcnt:u16, line:usize,
    world_bg:&[Line;4], used:&[bool;4], world_obj:&[ObjPixel;MAX_RENDER_WIDTH],
    world_window:&[bool;MAX_RENDER_WIDTH], rows:&[Line;2],extra_panel:bool,background_only:bool)->bool {
    use crate::ffta_dialogue::{UiPixel,LAYER_WIDTH,LAYER_INSET};
    if background_only {
        let dst=if extra_panel {&mut mem.ffta_dialogue.extra.backing}else{&mut mem.ffta_dialogue.backing};
        dst.resize(LAYER_WIDTH*160,Default::default());
    }
    let ui_mask=if extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats {0}else{0xc};
    let mut text=if ui_mask!=0 {rows[0]}else{[TRANSPARENT;MAX_RENDER_WIDTH]};
    let backing=if ui_mask!=0 {rows[1]}else{[TRANSPARENT;MAX_RENDER_WIDTH]};
    if background_only && mem.ffta_dialogue.action_layout {
        // Verified action-plane border tiles: header/footer and the info strip.
        // Keep dynamic action labels and portrait pixels out of the backing.
        let base=((io16(mem,0x0c)>>8)&31) as usize*0x800;
        for x in 0..240 {
            let off=base+((line/8)*32+x/8)*2;
            let tile=u16::from_le_bytes([mem.vram[off],mem.vram[off+1]])&1023;
            if !matches!(tile,0x005..=0x00a | 0x0a8..=0x0b9) {text[x]=TRANSPARENT;}
        }
    }
    if backing[..240].iter().any(|&c|c!=TRANSPARENT) {
        mem.ffta_dialogue.include_anchor_bounds(line,line+1);
    }
    let mut portraits=[OBJ_EMPTY;MAX_RENDER_WIDTH];
    let mut window=[false;MAX_RENDER_WIDTH];
    mem.ffta_dialogue.capture=true;
    mem.ffta_dialogue.capture_extra=extra_panel;
    mem.ffta_dialogue.capture_backing=background_only;
    if dispcnt&0x1000!=0 && (!background_only ||
        extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats) {
        render_sprites_extent(mem,dispcnt,line,&mut portraits,&mut window,LAYER_INSET as usize,LAYER_WIDTH);
    }
    mem.ffta_dialogue.capture_backing=false;
    mem.ffta_dialogue.capture=false;
    mem.ffta_dialogue.capture_extra=false;
    let placement=mem.ffta_dialogue.placement.unwrap_or_else(||crate::ffta_dialogue::Placement::new(true,render_width(),0,Default::default()));
    let bld=io16(mem,0x50);
    let alpha=io16(mem,0x52);
    let eva=(alpha&31).min(16) as u8;
    let evb=((alpha>>8)&31).min(16) as u8;
    let evy=(io16(mem,0x54)&31).min(16) as u32;
    let priorities:[u8;4]=std::array::from_fn(|bg|(io16(mem,8+2*bg)&3) as u8);
    let winin=io16(mem,0x48);let winout=io16(mem,0x4a);
    let win0h=io16(mem,0x40);let win1h=io16(mem,0x42);
    let win0v=io16(mem,0x44);let win1v=io16(mem,0x46);
    let row0=dispcnt&0x2000!=0 && in_window_range((win0v>>8)as u8,win0v as u8,line as u8);
    let row1=dispcnt&0x4000!=0 && in_window_range((win1v>>8)as u8,win1v as u8,line as u8);
    let mut bg_order=[0usize,1,2,3];
    bg_order.sort_unstable_by_key(|&bg|(priorities[bg],bg));
    // The native UI canvas is padded on both sides. Clear its row in bulk,
    // then composite only the union of possible BG and actual OBJ coverage.
    // Empty sky/world-only margins never need per-pixel UI classification.
    let native_ui_width=if mem.ffta_dialogue.attack_name{256}else{240};
    let length=if mem.ffta_dialogue.split_speech && extra_panel {0}else{placement.length_pixels};
    let first=portraits[..LAYER_WIDTH].iter().position(|p|p.priority!=255);
    let last=portraits[..LAYER_WIDTH].iter().rposition(|p|p.priority!=255);
    let (mut start,mut end)=first.zip(last).map_or((LAYER_WIDTH,0),|(a,b)|(a,b+1));
    if ui_mask!=0 && (text[..native_ui_width].iter().any(|&c|c!=TRANSPARENT)
        || backing[..native_ui_width].iter().any(|&c|c!=TRANSPARENT)) {
        start=start.min((LAYER_INSET-length).max(0)as usize);
        end=end.max((LAYER_INSET+native_ui_width as i32+length).clamp(0,LAYER_WIDTH as i32)as usize);
    }
    {
        let dst=if background_only {
            if extra_panel {&mut mem.ffta_dialogue.extra.backing}else{&mut mem.ffta_dialogue.backing}
        }else if extra_panel {&mut mem.ffta_dialogue.extra.pixels}else{&mut mem.ffta_dialogue.pixels};
        dst[line*LAYER_WIDTH..(line+1)*LAYER_WIDTH].fill(UiPixel::default());
    }
    if !mem.ffta_dialogue.reference_dialogue && first.is_none() && ui_mask!=0
        && !mem.ffta_dialogue.action_layout && !mem.ffta_dialogue.attack_name
        && mem.ffta_dialogue.action_banner.is_none()
        && ui_backgrounds_hidden(mem,dispcnt,line,used,world_window) {
        // No UI OBJ and both text planes are window-masked. The destination
        // row was cleared above; anchor detection has still sampled backing.
        return false;
    }
    for cx in start..end {
        let x=cx as i32-LAYER_INSET;
        let obj=portraits[cx];
        let text_color=if (0..native_ui_width as i32).contains(&x) {text[x as usize]}else{TRANSPARENT};
        let mut sx=x+if placement.right {length}else{-length};
        if length>0 && !mem.ffta_dialogue.action_layout && !mem.ffta_dialogue.service_layout && (!mem.ffta_dialogue.town_layout || mem.ffta_dialogue.split_speech) && !mem.ffta_dialogue.shop_layout
            && !mem.ffta_dialogue.attack_name && mem.ffta_dialogue.extra.kind!=crate::ffta_dialogue::PanelKind::Notice {
            if placement.right && sx>=200 && sx<240+length {sx=200;}
            if !placement.right && sx<40 && sx>=-length {sx=39;}
        }
        let backing_color=if (0..native_ui_width as i32).contains(&sx) {backing[sx as usize]}else{TRANSPARENT};
            let banner=mem.ffta_dialogue.action_banner.is_some_and(|[l,t,r,b]|x>=l && x<r && line as i32>=t && (line as i32)<b);
            let contains=|b:Option<[i32;4]>|b.is_some_and(|[l,t,r,b]|x>=l && x<r && line as i32>=t && (line as i32)<b);
            let speech_capture=extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Speech;
            let in_panel=if mem.ffta_dialogue.naming_layout {true}
            else if speech_capture {contains(mem.ffta_dialogue.service_speech_frame)}
            else if contains(mem.ffta_dialogue.service_speech_frame) {false}
            else if !extra_panel && mem.ffta_dialogue.service_list && mem.ffta_dialogue.pub_window_owners[1].is_some() {
                contains(mem.ffta_dialogue.service_list_bounds)
            }
            else if banner {!extra_panel}
            else if mem.ffta_dialogue.attack_name { !extra_panel && contains(mem.ffta_dialogue.native_bounds) }
            else if mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice
                || (mem.ffta_dialogue.action_layout || mem.ffta_dialogue.town_layout)
                    && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Actions {
                let [l,t,r,b]=mem.ffta_dialogue.extra.bounds.unwrap();
                let inside=x>=l && x<r && line as i32>=t && (line as i32)<b;
                inside==extra_panel
            }else{!mem.ffta_dialogue.action_layout || (if extra_panel {x>=160}else{x<160})};
        // Clan Funds has its own fixed canvas throughout service transitions.
        let in_panel=in_panel && !mem.ffta_dialogue.funds_bg.is_some_and(|_| {
            mem.ffta_dialogue.funds_bounds.is_some_and(|[l,t,r,b]|
                x>=l && x<r && line as i32>=t && (line as i32)<b)
        });
        // World-only pixels can never become UI output. Clear the destination
        // without sorting/compositing backgrounds that will be discarded.
        if obj.priority==255 && (ui_mask==0 || !in_panel || (text_color==TRANSPARENT && backing_color==TRANSPARENT)) {
            let dst=if background_only {
                if extra_panel {&mut mem.ffta_dialogue.extra.backing}else{&mut mem.ffta_dialogue.backing}
            }else if extra_panel {&mut mem.ffta_dialogue.extra.pixels}else{&mut mem.ffta_dialogue.pixels};
            dst[line*LAYER_WIDTH+cx]=UiPixel::default();
            continue;
        }
        let wx=x.clamp(0,239) as u8;
        let native=extra()+wx as usize;
        let mut control=0x3f;
        // These canvases use unscrolled BG rows or normalized OBJ positions.
        // A WIN rectangle in the old, animated screen coordinates must not
        // crop them a second time after relocation.
        let canonical_capture=mem.ffta_dialogue.action_layout || mem.ffta_dialogue.attack_name || banner
            || extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats;
        if dispcnt&0xe000!=0 {
            control=winout&63;
            if dispcnt&0x8000!=0 && world_window[native] { control=(winout>>8)&63; }
            if row1 && in_window_range((win1h>>8)as u8,win1h as u8,wx){control=(winin>>8)&63;}
            if row0 && in_window_range((win0h>>8)as u8,win0h as u8,wx){control=winin&63;}
        }
        if canonical_capture {
            // Keep the owning window's effects-enable bit. Removing its
            // geometric crop must not turn on alpha that WININ disabled.
            // The opaque top capsule and translucent lower HUD coexist.
            // Only a capsule borrows its owning WININ mask; the lower HUD
            // keeps the effects bit at its own native position (WINOUT).
            let effects=if banner || mem.ffta_dialogue.attack_name {
                if dispcnt&0x2000!=0 && winin&12==12 {winin&32}
                else if dispcnt&0x4000!=0 && (winin>>8)&12==12 {(winin>>8)&32}
                else{control&32}
            }else{control&32};
            control=31|effects;
        }
        // BG priority is constant for this scanline. Only the two visible
        // blend targets are needed; avoid sorting five candidates per pixel.
        let mut top=[(0u16,5usize,255u8,false);2];let mut nt=0;
        for bg in bg_order {
            if !used[bg] || control&(1<<bg)==0 {continue;}
            let color=match bg {
                2 if ui_mask!=0=>text_color,
                3 if ui_mask!=0=>backing_color,
                _=>world_bg[bg][native],
            };
            if color==TRANSPARENT || ui_mask&(1<<bg)!=0 && !in_panel {continue;}
            top[nt]=(color,bg,priorities[bg],false);nt+=1;
            if nt==2 {break;}
        }
        if obj.priority!=255 && control&16!=0 {
            insert_ui_target(&mut top,&mut nt,(obj.color,4,obj.priority,obj.semi));
        }
        let wo=world_obj[native];
        let modal=extra_panel && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice;
        let world_object=!modal && !canonical_capture && !background_only && (0..240).contains(&x) && wo.priority!=255 && control&16!=0 && obj.priority==255;
        if world_object {insert_ui_target(&mut top,&mut nt,(wo.color,6,wo.priority,wo.semi));}
        let pixel=if let Some(&(fc,fl,_,semi))=top[..nt].first() {
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
        } else {UiPixel::default()};
        let pixel=if !extra_panel && (mem.ffta_dialogue.battle_notice()
            || mem.ffta_dialogue.attack_name && mem.ffta_dialogue.attack_exiting) {
            UiPixel::default()
        }else{pixel};
        if background_only {
            let dst=if extra_panel {&mut mem.ffta_dialogue.extra.backing}else{&mut mem.ffta_dialogue.backing};
            dst[line*LAYER_WIDTH+cx]=pixel;
        } else if extra_panel {mem.ffta_dialogue.extra.pixels[line*LAYER_WIDTH+cx]=pixel;}
        else {mem.ffta_dialogue.pixels[line*LAYER_WIDTH+cx]=pixel;}
    }
    first.is_some()
}

#[cfg(test)]
mod ui_priority_tests {
    use super::*;
    #[test]
    fn two_target_merge_matches_full_priority_sort() {
        for packed in 0..256usize {for mask in 0..16 {for obj in 0..5u8 {
            for world in 0..if obj==4 {5}else{1} {
                let mut original=Vec::new();
                for bg in 0..4 {if mask&(1<<bg)!=0 {original.push((bg as u16+1,bg,((packed>>(bg*2))&3)as u8,false));}}
                let mut top=[(0u16,5usize,255u8,false);2];let mut count=0;
                original.sort_unstable_by_key(|v|(v.2,v.1));
                for &v in &original {insert_ui_target(&mut top,&mut count,v);}
                if obj<4 {let v=(7,4,obj,true);original.push(v);insert_ui_target(&mut top,&mut count,v);}
                if obj==4 && world<4 {let v=(8,6,world as u8,false);original.push(v);insert_ui_target(&mut top,&mut count,v);}
                original.sort_unstable_by_key(|v|(v.2,if v.1>=4 {0}else{v.1+1}));
                assert_eq!(&top[..count],&original[..original.len().min(2)]);
            }
        }}}
    }
}

#[cfg(test)]
mod dialogue_layout_tests {
    use super::*;
    #[test]
    fn roulette_picture_keeps_guest_position_without_becoming_a_dialogue() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;
        m.rom.resize(0x11954,0);
        m.rom[0x1194c..0x11954].copy_from_slice(&[0x30,0xb5,0x03,0x48,0x03,0x68,0x05,0x1c]);
        m.io[..2].copy_from_slice(&0x1b40u16.to_le_bytes());
        m.iwram[0x2804..0x2808].copy_from_slice(&0x08025115u32.to_le_bytes());
        for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        for e in m.vram[0x5000..0x5800].chunks_exact_mut(2){e.copy_from_slice(&0x03ffu16.to_le_bytes());}
        for y in 0..9 {for x in 0..7 {
            let o=0x5000+(y*32+x)*2;
            m.vram[o..o+2].copy_from_slice(&(0x63a2u16+(y*7+x)as u16).to_le_bytes());
        }}
        m.vram[0xf440..0xfc20].fill(0x11);
        m.palette[0xc2..0xc4].copy_from_slice(&0x1234u16.to_le_bytes());
        for(o,v)in[(0x1c,424u16),(0x1e,488)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let oam=m.oam.clone();let vram=m.vram.clone();
        for y in 0..160{render_scanline(&mut m,y);}
        assert!(m.ffta_dialogue.placement.is_none());
        assert!(crate::ffta_ui::Scene::capture(&m.ffta_dialogue).nodes.iter().all(Option::is_none));
        assert!(crate::ffta_effects::layer(&m,3).is_some());
        assert_eq!(m.wide_framebuffer[24*568+164+88],0x1234);
        assert!(m.wide_framebuffer[..568*160].chunks_exact(568).all(|r|
            r[..164].iter().chain(r[404..].iter()).all(|&p|p!=0x1234)));
        assert_eq!(m.oam,oam);assert_eq!(m.vram,vram);
        // An actual dialogue controller still owns its UI, regardless of phase.
        m.ewram[0x918c..0x9190].copy_from_slice(&0x02001000u32.to_le_bytes());
        begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.placement.is_some());
        // Ownerless real text/frame palettes are not the effect's atlas.
        m.ewram[0x918c..0x9190].fill(0);
        let o=0x5000;m.vram[o..o+2].copy_from_slice(&0xb001u16.to_le_bytes());
        assert!(!crate::ffta_effects::shared_dialogue_picture(&m,3));
        begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.placement.is_some());
        crate::mem::set_extra(0);
    }
    #[test]
    fn teleport_silhouette_filter_preserves_ownerless_portraits_and_margin_clipping() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut mem=scene(false);mem.ffta_dialogue.separate=true;
        mem.rom.resize(0x11954,0);
        mem.rom[0x1194c..0x11954].copy_from_slice(&[0x30,0xb5,0x03,0x48,0x03,0x68,0x05,0x1c]);
        assert!(!dialogue_controller_present(&mem));
        // Portrait/panel transitions outlive the text owner. Keep them detached.
        begin_dialogue_frame(&mut mem);
        assert!(mem.ffta_dialogue.placement.is_some());
        assert!(ffta_dialogue_foreground(&mem,2));
        // Compact actor-sized BG3 image, no portrait, no text controller.
        for obj in mem.oam.chunks_exact_mut(8) {obj[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        mem.vram[0x5000..0x5800].fill(0);
        for y in 3..7 {for x in 1..3 {
            let p=0x5000+(y*32+x)*2;
            mem.vram[p..p+2].copy_from_slice(&1u16.to_le_bytes());
        }}
        begin_dialogue_frame(&mut mem);
        assert!(mem.ffta_dialogue.placement.is_none());
        assert!(ffta_dialogue_foreground(&mem,2),"effects must not re-enable UI wrapping in blue margins");
        mem.io[0x18..0x1a].copy_from_slice(&6u16.to_le_bytes());
        for cell in mem.vram[0x5800..0x6000].chunks_exact_mut(2) {cell.copy_from_slice(&1u16.to_le_bytes());}
        let mut row=[31;MAX_RENDER_WIDTH];
        render_text_bg_hardware(&mem,2,32,&mut row,164,568);
        assert!(row[..164].iter().chain(row[404..568].iter()).all(|v|*v==TRANSPARENT));
        assert!(row[164..404].iter().any(|v|*v!=TRANSPARENT));

        // A real text owner preserves every animation phase even for tiny UI.
        mem.ewram[0x918c..0x9190].copy_from_slice(&0x02001000u32.to_le_bytes());
        for phase in [0,1,6,7,11] {
            mem.ewram[0x1008]=phase;
            begin_dialogue_frame(&mut mem);
            assert!(mem.ffta_dialogue.placement.is_some(),"speech phase {phase}");
        }
        crate::mem::set_extra(0);
    }
    #[test]
    fn detached_ui_planes_never_flash_when_guest_owner_is_between_phases(){
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;m.ffta_dialogue.wide_pass=true;
        m.ffta_dialogue.placement=None;
        m.io[12..14].copy_from_slice(&0x5c08u16.to_le_bytes());m.io[14..16].copy_from_slice(&0x4a09u16.to_le_bytes());
        let mut row=[31;MAX_RENDER_WIDTH];
        render_text_bg_extent(&m,3,16,&mut row,164,568);
        assert!(row[..568].iter().all(|p|*p==TRANSPARENT));
        let mut original=[TRANSPARENT;MAX_RENDER_WIDTH];
        render_text_bg_hardware(&m,3,16,&mut original,0,240);
        assert!(original[..240].iter().any(|p|*p!=TRANSPARENT),"native scene remains unchanged");
        crate::mem::set_extra(0);
    }
    #[test]
    fn unscrolled_spell_is_not_reextended_after_its_margin_was_cleared(){
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=false;
        m.io[12..14].copy_from_slice(&0x1c04u16.to_le_bytes());
        m.vram[0x4020..0x4040].fill(0x11);
        for tile in m.vram[0xe000..0xe800].chunks_exact_mut(2){tile.copy_from_slice(&1u16.to_le_bytes());}
        let mut native=[TRANSPARENT;MAX_RENDER_WIDTH];let mut wide=native;
        render_text_bg_hardware(&m,2,16,&mut native,0,240);
        render_text_bg_hardware(&m,2,16,&mut wide,164,568);
        assert_eq!(&wide[164..404],&native[..240]);
        assert!(wide[..164].iter().chain(&wide[404..568]).all(|p|*p==TRANSPARENT));
        // An animation callback may replace the battle handler, but not its stage.
        m.iwram[0x2804..0x2808].copy_from_slice(&0x08012345u32.to_le_bytes());
        assert!(crate::ffta_effects::layer(&m,2).is_some());
        crate::mem::set_extra(0);
    }
    #[test]
    fn wrapped_confirmation_strip_keeps_both_halves_during_window_animation(){
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;
        m.io[12..14].copy_from_slice(&0x5c08u16.to_le_bytes());m.io[14..16].copy_from_slice(&0x4a09u16.to_le_bytes());
        m.vram[0x5000..0x6000].fill(0);
        for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        for tile in [0x32usize,0x33]{m.vram[0x8000+tile*32..0x8000+(tile+1)*32].fill(0x11);}
        m.palette[0x162..0x164].copy_from_slice(&0x03e0u16.to_le_bytes());
        for y in 6..8{for dx in 0..24{let tile=if dx==0{0xb032}else if dx==23{0xb432}else{0xb033};
            let tile:u16=tile|if y==7{0x800}else{0};let o=0x5000+(y*32+(28+dx)%32)*2;m.vram[o..o+2].copy_from_slice(&tile.to_le_bytes());}}
        let mut expected=None;
        for clip in [0x0102u16,0x10e0,0x80c0,0xf001]{
            for(o,v)in[(0,0x3f40u16),(0x40,clip),(0x44,0x3038),(0x48,31),(0x4a,63),(0x50,0x2048),(0x52,0x0808)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
            for y in 0..160{render_scanline(&mut m,y);}
            assert_eq!(m.ffta_dialogue.action_banner,Some([0,48,192,64]));
            assert_eq!(m.ffta_dialogue.action_banner_rotation,224);
            for y in 48..64{assert_eq!(m.ffta_dialogue.pixels[y*480+120..y*480+312].iter().filter(|p|p.visible()).count(),192);}
            assert!(m.ffta_dialogue.pixels.iter().filter(|p|p.visible()).all(|p|p.eva==16 && p.evb==0),"WININ disables effects for the opaque strip even though the world uses alpha");
            if let Some(ref pixels)=expected{assert_eq!(&m.ffta_dialogue.pixels,pixels);}else{expected=Some(m.ffta_dialogue.pixels.clone());}
        }
        crate::mem::set_extra(0);
    }
    #[test]
    fn affine_hover_stats_remain_on_the_host_canvas_through_guest_window_clipping(){
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=stats_scene();
        for obj in m.oam.chunks_exact_mut(8){let a=u16::from_le_bytes([obj[0],obj[1]]);if a&0x300!=0x200{obj[..2].copy_from_slice(&(a|0x100).to_le_bytes());}}
        for(o,v)in[(6,256u16),(14,0),(22,0),(30,256)]{m.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        for(o,v)in[(0,0x3f40u16),(0x40,0x0008),(0x44,0x0008),(0x48,31),(0x4a,32)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let original=m.oam.clone();for y in 0..160{render_scanline(&mut m,y);}
        assert_eq!(m.ffta_dialogue.stats_target,Some([152,104,240,152]));
        assert!(m.ffta_dialogue.extra.pixels.iter().filter(|p|p.visible()).count()>100);
        assert_eq!(m.oam,original);
        crate::mem::set_extra(0);
    }
    #[test]
    fn compact_attack_owner_keeps_top_layout_through_entry_exit_and_stale_portrait(){
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;m.ffta_dialogue.style.length=2.;
        m.vram[0x5000..0x5800].fill(0);
        for y in 2..4{for x in 2..16{let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());}}
        let owner=0x1000;
        for(a,v)in[(0,0x06005800u32),(4,0x06005000),(24,0x0003000d)]{m.ewram[owner+a..owner+a+4].copy_from_slice(&v.to_le_bytes());}
        m.ewram[owner+16..owner+20].copy_from_slice(&[2,2,14,2]);
        for(a,v)in[(20,0x0800u16),(22,0x0b20),(34,2),(38,2)]{m.ewram[owner+a..owner+a+2].copy_from_slice(&v.to_le_bytes());}
        // The native portrait from the preceding state is still published.
        assert!(crate::ffta_dialogue::portrait([0xa000,0xc000,0x240],0));
        let original=m.oam.clone();let mut expected=None;
        for(current,target)in[(1u16,14u16),(4,14),(14,14),(14,0),(1,0)]{
            m.ewram[owner+32..owner+34].copy_from_slice(&current.to_le_bytes());m.ewram[owner+36..owner+38].copy_from_slice(&target.to_le_bytes());
            for(o,v)in[(0,0x3f40u16),(0x40,0x0102),(0x44,0x0102),(0x48,31),(0x4a,32)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
            for y in 0..160{render_scanline(&mut m,y);}
            let node=crate::ffta_ui::Scene::capture(&m.ffta_dialogue).nodes[0].unwrap();
            assert_eq!(node.role,crate::ffta_ui::Role::AttackName);assert_eq!(node.bounds,[16,16,128,32]);
            assert_eq!(node.anchor,crate::ffta_ui::Anchor::TopCenter);assert!(!node.length);
            if let Some(ref pixels)=expected{assert_eq!(&m.ffta_dialogue.pixels,pixels);}else{expected=Some(m.ffta_dialogue.pixels.clone());}
            assert_eq!(m.oam,original);
        }
        crate::mem::set_extra(0);
    }
    #[test]
    fn shop_page_keeps_complete_pixels_until_rebuild_has_finished(){
        use crate::ffta_dialogue::UiPixel;
        let mut m=MemMap::new(vec![]);m.ffta_dialogue.separate=true;m.ffta_dialogue.shop_layout=true;
        let original=vec![UiPixel::opaque(31);480*160];
        m.ffta_dialogue.pixels=original.clone();finish_shop_page(&mut m);
        // A clear or a half-painted page cannot leak into presentation.
        for partial in [0,100,480*80,480*160] {
            m.ffta_dialogue.shop_page_dirty=true;
            m.ffta_dialogue.pixels.fill(UiPixel::default());
            m.ffta_dialogue.pixels[..partial].fill(UiPixel::opaque(992));
            finish_shop_page(&mut m);
            assert!(m.ffta_dialogue.pixels.iter().all(|p|p.packed()==original[0].packed()));
        }
        m.ffta_dialogue.shop_page_dirty=false;
        m.ffta_dialogue.pixels.fill(UiPixel::opaque(992));finish_shop_page(&mut m);
        assert!(m.ffta_dialogue.shop_page_complete.iter().all(|p|p.color==992));
        m.ffta_dialogue.shop_numbers_exiting=true;finish_shop_page(&mut m);
        assert!(m.ffta_dialogue.shop_page_complete.is_empty(),"closing cannot resurrect old digits");
    }
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
    #[test]
    fn selected_unit_pointer_stays_in_world_and_zero_length_banner_has_no_left_copy() {
        assert!(!crate::ffta_dialogue::list_marker([0x802c,0x0072,0x0134]));
        assert!(crate::ffta_dialogue::list_marker([0x4000,0,0x0120]));
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;m.ffta_dialogue.action_layout=true;
        m.ffta_dialogue.placement=Some(crate::ffta_dialogue::Placement::new(false,568,0,Default::default()));
        m.ffta_dialogue.placement.as_mut().unwrap().length_pixels=0;
        m.ffta_dialogue.pixels.resize(480*160,Default::default());
        let world=[[TRANSPARENT;MAX_RENDER_WIDTH];4];let used=[false,false,false,true];
        let objects=[OBJ_EMPTY;MAX_RENDER_WIDTH];let windows=[false;MAX_RENDER_WIDTH];
        let mut rows=[[TRANSPARENT;MAX_RENDER_WIDTH];2];rows[1][16..224].fill(31);
        capture_dialogue_row(&mut m,0x1f40,8,&world,&used,&objects,&windows,&rows,false);
        assert!(!m.ffta_dialogue.pixels[8*480+120].visible());
        assert!(m.ffta_dialogue.pixels[8*480+120+16].visible());
        crate::mem::set_extra(0);
    }
    #[test]
    fn target_stats_and_partial_spell_menu_retain_independent_owners() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=stats_scene();
        for(bg,cnt)in[0x4c03u16,0x4e02,0x5c08,0x4a09].into_iter().enumerate(){m.io[8+bg*2..10+bg*2].copy_from_slice(&cnt.to_le_bytes());}
        m.vram[0xe000..0xe800].fill(0);
        for (x,y,v) in [(18,7,0xc005u16),(19,7,0xc006),(18,13,0xc009),(19,13,0xc00a)] {
            let o=0xe000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
        begin_dialogue_frame(&mut m);
        assert_eq!(m.ffta_dialogue.extra.kind,crate::ffta_dialogue::PanelKind::Actions);
        assert_eq!(m.ffta_dialogue.extra.bounds,Some([144,56,240,112]));
        assert_eq!(m.ffta_dialogue.battle_stats.kind,crate::ffta_dialogue::PanelKind::Stats);
        assert_eq!(m.ffta_dialogue.stats_target,Some([152,104,240,152]));
        // Entry/exit may remove the footer before the header.
        m.vram[0xe000+(13*32+18)*2..0xe000+(13*32+20)*2].fill(0);
        begin_dialogue_frame(&mut m);
        assert_eq!(m.ffta_dialogue.extra.kind,crate::ffta_dialogue::PanelKind::Actions);
        assert_eq!(m.ffta_dialogue.extra.bounds,Some([144,56,240,112]));
        assert!(m.ffta_dialogue.battle_stats.active());
        crate::mem::set_extra(0);
    }
    #[test]
    fn inventory_numbers_and_funds_keep_separate_pixels_without_cutting_backing() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);
        for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        for (slot,a) in [[144u16,176,0x21a0],[144,112,0x31c0]].into_iter().enumerate(){
            for (i,v) in a.iter().enumerate(){m.oam[slot*8+i*2..slot*8+i*2+2].copy_from_slice(&v.to_le_bytes());}
        }
        for tile in [0x1a0usize,0x1c0]{m.vram[0x10000+tile*32..0x10020+tile*32].fill(0x11);}
        m.palette[0x242..0x244].copy_from_slice(&0x7c00u16.to_le_bytes());
        m.palette[0x262..0x264].copy_from_slice(&0x03e0u16.to_le_bytes());
        m.ffta_dialogue.separate=true;m.ffta_dialogue.service_layout=true;m.ffta_dialogue.shop_layout=true;
        m.ffta_dialogue.placement=Some(crate::ffta_dialogue::Placement::new(true,568,0,Default::default()));
        m.ffta_dialogue.funds_bounds=crate::ffta_ui::funds_bounds(&m);
        m.ffta_dialogue.pixels.resize(480*160,Default::default());
        m.ffta_dialogue.funds_pixels.resize(480*160,Default::default());
        let world=[[TRANSPARENT;MAX_RENDER_WIDTH];4];let used=[false,false,false,true];
        let objects=[OBJ_EMPTY;MAX_RENDER_WIDTH];let windows=[false;MAX_RENDER_WIDTH];
        let mut rows=[[TRANSPARENT;MAX_RENDER_WIDTH];2];rows[1][..240].fill(31);
        let oam=m.oam.clone();
        capture_dialogue_row(&mut m,0x1f40,144,&world,&used,&objects,&windows,&rows,false);
        capture_funds_row(&mut m,0x1f40,144);
        assert_eq!(m.ffta_dialogue.pixels[144*480+120+112].color,0x03e0);
        assert_eq!(m.ffta_dialogue.pixels[144*480+120+176].color,31,"blue backing stays beneath the counter");
        assert_eq!(m.ffta_dialogue.funds_pixels[144*480+120+176].color,0x7c00);
        assert!(!m.ffta_dialogue.funds_pixels[144*480+120+112].visible());
        assert_eq!(m.oam,oam);crate::mem::set_extra(0);
    }
    #[test]
    fn executed_attack_owner_keeps_complete_yellow_capsule_on_one_canvas() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for (x,width) in [(6usize,18usize),(2,26),(30,18)] {
            let mut m=scene(false);m.ffta_dialogue.separate=true;
            m.io[12..14].copy_from_slice(&0x5c08u16.to_le_bytes());
            m.io[14..16].copy_from_slice(&0x4a09u16.to_le_bytes());
            for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
            m.vram[0x5000..0x6000].fill(0);
            for y in 1..3 {for dx in 0..width {
                let o=0x5000+(y*32+(x+dx)%32)*2;
                m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
            }}
            let owner=0x1000;
            for(a,v)in[(0,0x0600e000u32),(4,0x06005000),(24,0x00030032)]{
                m.ewram[owner+a..owner+a+4].copy_from_slice(&v.to_le_bytes());
            }
            m.ewram[owner+16..owner+20].copy_from_slice(&[x as u8,1,width as u8,2]);
            for(a,v)in[(20,0x0404u16),(22,0x0b20),(34,2),(38,2)]{
                m.ewram[owner+a..owner+a+2].copy_from_slice(&v.to_le_bytes());
            }
            let left=if x+width>30{0}else{x*8};
            let bounds=[left as i32,8,(left+width*8)as i32,24];
            for(current,target)in[(1,width as u16),(width as u16,width as u16),(1,0)] {
                m.ewram[owner+32..owner+34].copy_from_slice(&current.to_le_bytes());
                m.ewram[owner+36..owner+38].copy_from_slice(&target.to_le_bytes());
                for y in 0..160 {render_scanline(&mut m,y);}
                let ui=crate::ffta_ui::Scene::capture(&m.ffta_dialogue);
                let node=ui.nodes[0].unwrap();
                assert_eq!(node.role,crate::ffta_ui::Role::AttackName);
                assert_eq!(node.anchor,crate::ffta_ui::Anchor::TopCenter);
                assert_eq!(node.bounds,bounds);assert!(!node.length);
                assert!(ui.nodes[1].is_none());
                assert_eq!(crate::ffta_dialogue::pixel_bounds(&m.ffta_dialogue.pixels),Some(bounds));
            }
            // On exit the owner closes both dimensions. Hide immediately,
            // including when its tiles outlive the final zero-sized frame.
            for current in [width as u16,1,0] {
                m.ewram[owner+32..owner+34].copy_from_slice(&current.to_le_bytes());
                m.ewram[owner+36..owner+40].fill(0);
                for y in 0..160 {render_scanline(&mut m,y);}
                assert!(m.ffta_dialogue.attack_name && m.ffta_dialogue.attack_exiting);
                assert!(crate::ffta_dialogue::pixel_bounds(&m.ffta_dialogue.pixels).is_none());
            }
            // The same constructor can instead be a forecast/help capsule.
            // A lower BG information panel must keep the complete left HUD.
            m.ewram[owner+32..owner+34].copy_from_slice(&(width as u16).to_le_bytes());
            m.ewram[owner+36..owner+38].copy_from_slice(&(width as u16).to_le_bytes());
            m.ewram[owner+38..owner+40].copy_from_slice(&2u16.to_le_bytes());
            for y in 13..19 {for x in 0..17 {
                let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
            }}
            for y in 0..160 {render_scanline(&mut m,y);}
            assert!(m.ffta_dialogue.action_layout && !m.ffta_dialogue.attack_name);
            let ui=crate::ffta_ui::Scene::capture(&m.ffta_dialogue);
            assert_eq!(ui.nodes[0].unwrap().role,crate::ffta_ui::Role::ActionInfo);
            assert!(m.ffta_dialogue.pixels[104*480+120].visible());
        }
        crate::mem::set_extra(0);
    }
    #[test]
    fn wrapped_attack_capsule_is_one_top_panel_and_ignores_length() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut expected=None;
        for length in [1.,1.5,2.] {
            let mut m=scene(false);m.ffta_dialogue.separate=true;m.ffta_dialogue.style.length=length;
            for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
            m.vram[0x5000..0x6000].fill(0);
            for x in [30usize,31,0,1,2,3,4,5,6] {
                let o=0x5000+(2*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
            }
            m.vram[0x5800+2*32*2..0x5800+2*32*2+2].copy_from_slice(&2u16.to_le_bytes());
            m.vram[0x8040..0x8060].fill(0x22);m.palette[4..6].copy_from_slice(&0x03e0u16.to_le_bytes());
            let oam=m.oam.clone();
            for y in 0..160 {render_scanline(&mut m,y);}
            assert!(m.ffta_dialogue.attack_name);
            assert_eq!(m.ffta_dialogue.attack_rotation,240);
            assert_eq!(crate::ffta_dialogue::pixel_bounds(&m.ffta_dialogue.pixels),Some([0,16,72,24]));
            let scene=crate::ffta_ui::Scene::capture(&m.ffta_dialogue);
            assert_eq!(scene.nodes[0].unwrap().role,crate::ffta_ui::Role::AttackName);
            assert_eq!(scene.nodes[0].unwrap().anchor,crate::ffta_ui::Anchor::TopCenter);
            if let Some(ref pixels)=expected {assert_eq!(&m.ffta_dialogue.pixels,pixels);}
            else{expected=Some(m.ffta_dialogue.pixels.clone());}
            assert_eq!(m.oam,oam);
        }
        crate::mem::set_extra(0);
    }
    #[test]
    fn affine_spell_wrap_is_preserved_only_inside_the_native_viewport() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);
        m.io[..2].copy_from_slice(&0x1f41u16.to_le_bytes());
        m.io[12..14].copy_from_slice(&0x3c84u16.to_le_bytes());
        m.io[0x20..0x22].copy_from_slice(&256u16.to_le_bytes());
        m.vram[0xe000..0xe100].fill(1);m.vram[0x4040..0x4080].fill(1);
        let mut repeated=[TRANSPARENT;MAX_RENDER_WIDTH];let mut finite=repeated;
        render_affine_bg(&m,2,&mut repeated);
        m.iwram[0x2804..0x2808].copy_from_slice(&0x08025115u32.to_le_bytes());
        render_affine_bg(&m,2,&mut finite);
        assert_eq!(&finite[164..404],&repeated[164..404]);
        assert_ne!(repeated[0],TRANSPARENT);assert_eq!(finite[0],TRANSPARENT);
        crate::mem::set_extra(0);
    }
    #[test]
    fn battle_effect_atlas_does_not_repeat_in_wide_margins_and_preserves_native_pixels() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=false;
        m.io[12..14].copy_from_slice(&0x1c04u16.to_le_bytes());
        m.io[0x18..0x1a].copy_from_slice(&8u16.to_le_bytes());
        m.vram[0x4020..0x4040].fill(0x11);
        for i in 0..1024 {m.vram[0xe000+i*2..0xe002+i*2].copy_from_slice(&1u16.to_le_bytes());}
        let mut original=[TRANSPARENT;MAX_RENDER_WIDTH];let mut finite=original;
        m.ffta_movement.europe_profile=false;
        render_text_bg_hardware(&m,2,16,&mut original,164,568);
        m.ffta_movement.europe_profile=true;
        m.iwram[0x2804..0x2808].copy_from_slice(&0x08025115u32.to_le_bytes());
        render_text_bg_hardware(&m,2,16,&mut finite,164,568);
        assert_eq!(&finite[164..404],&original[164..404]);
        assert_ne!(original[164+344],TRANSPARENT);assert_eq!(finite[164+344],TRANSPARENT);
        // The source classification is tied to the actual BG register resource.
        let effect=crate::ffta_effects::layer(&m,2).unwrap();
        assert_eq!((effect.map,effect.characters),(0x0600e000,0x06004000));
        crate::mem::set_extra(0);
    }
    #[test]
    fn service_ui_matches_native_scroll_and_leaves_other_stages_unchanged() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(true);m.ffta_dialogue.separate=true;
        for stage in [62u16,63,65,66] {
            m.ewram[0x7f10..0x7f12].copy_from_slice(&stage.to_le_bytes());
            for scroll in [0u16,8,64,248,0] {
                for reg in [0x18,0x1a,0x1c,0x1e] {m.io[reg..reg+2].copy_from_slice(&scroll.to_le_bytes());}
                for y in 0..160 {render_scanline(&mut m,y);}
                assert!(m.ffta_dialogue.service_layout);
                for y in 0..160 {for x in 0..240 {
                    let pixel=m.ffta_dialogue.pixels[y*480+x+120];
                    assert_eq!(pixel.resolve(m.wide_framebuffer[y*568+x+164]),m.framebuffer[y*240+x],
                        "stage={stage} scroll={scroll} x={x} y={y}");
                }}
            }
        }
        m.ewram[0x7f10..0x7f12].copy_from_slice(&0u16.to_le_bytes());
        begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.service_layout);
        crate::mem::set_extra(0);
        m.ewram[0x7f10..0x7f12].copy_from_slice(&62u16.to_le_bytes());
        begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.service_layout);
    }
    #[test]
    fn engagement_uses_complete_centered_canvas_through_native_scroll() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;
        for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        m.vram[0x5000..0x6000].fill(0);
        for y in 6..14 {for x in 5..25 {
            let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&0x70e5u16.to_le_bytes());
        }}
        for (x,y,tile) in [(5,6,0x70e0u16),(24,6,0x74e0),(5,13,0x78e0),
            (24,13,0x7ce0),(7,6,0x70e2)] {
            let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&tile.to_le_bytes());
        }
        m.vram[0x9c00..0x9cc0].fill(0x11);
        m.palette[0xe2..0xe4].copy_from_slice(&0x1234u16.to_le_bytes());
        let oam=m.oam.clone();let mut expected=None;
        for (x,y) in [(0u16,0u16),(48,0),(511,12)] {
            for (o,v)in[(0x18,x),(0x1a,y),(0x1c,x),(0x1e,y)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
            for line in 0..160{render_scanline(&mut m,line);}
            let n=crate::ffta_ui::Scene::capture(&m.ffta_dialogue).nodes[0].unwrap();
            assert_eq!(n.role,crate::ffta_ui::Role::Notice);
            assert_eq!(n.anchor,crate::ffta_ui::Anchor::Modal);
            assert_eq!(n.bounds,[40,48,200,112]);assert!(!n.length);
            let pixels:Vec<_>=m.ffta_dialogue.pixels.iter().map(|p|p.packed()).collect();
            assert!(m.ffta_dialogue.pixels.iter().any(|p|p.visible()));
            if let Some(ref old)=expected{assert_eq!(&pixels,old);}else{expected=Some(pixels);}
            assert!(m.wide_framebuffer[..568*160].iter().all(|&p|p!=0x1234));
            assert_eq!(m.oam,oam);
        }
        m.io[0..2].copy_from_slice(&0x1740u16.to_le_bytes());
        assert!(engagement_bounds(&m).is_none(),"disabled BG3 cannot retain a stale frame");
        crate::mem::set_extra(0);
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
    fn target_stats_host_only_entry_and_exit_never_leak_native_animation() {
        use crate::ffta_sprites::HostSprite;
        use crate::ffta_dialogue::PanelKind;
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for right in [false,true] {for color in [31u16,0x7c00] {for dx in [-80,0,80] {
            let mut m=stats_scene();
            m.palette[0x362..0x364].copy_from_slice(&color.to_le_bytes());
            let mut sprites=Vec::new();
            for (slot,o) in m.oam.chunks_exact_mut(8).enumerate() {
                let mut attrs=std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]]));
                if attrs[0]&0x300==0x200{continue;}
                if !right{attrs[1]^=0x1000;}
                let x=(attrs[1]&511)as i32+dx;
                attrs[1]=(attrs[1]&!511)|(x&511)as u16;
                sprites.push(HostSprite{x,y:(attrs[0]&255)as i32,attrs,slot,replaces:false,
                    stats_background:stats_backing(attrs),stats_inner_padding:true,..Default::default()});
                o[..2].copy_from_slice(&0x200u16.to_le_bytes());
            }
            m.ffta_sprites.active=Some(sprites);
            let oam=m.oam.clone();
            for y in 0..160{render_scanline(&mut m,y);}
            assert_eq!(m.ffta_dialogue.extra.kind,PanelKind::None);
            assert!(m.ffta_dialogue.suspended_stats);
            assert!(m.wide_framebuffer[..568*160].iter().all(|&p|p==0),"stock stats leaked right={right} dx={dx}");
            assert_eq!(m.oam,oam);
            // Once hardware publication is complete, use the original UI
            // capture, not host coordinates or their legacy padding flags.
            for s in m.ffta_sprites.active.as_ref().unwrap() {
                for(i,a)in s.attrs.iter().enumerate(){
                    m.oam[s.slot*8+i*2..s.slot*8+i*2+2].copy_from_slice(&a.to_le_bytes());
                }
            }
            for y in 0..160{render_scanline(&mut m,y);}
            assert_eq!(m.ffta_dialogue.extra.kind,PanelKind::Stats,"right={right} dx={dx}");
            assert_eq!(m.ffta_dialogue.stats_target,Some(if right{[152,104,240,152]}else{[0,104,88,152]}));
            assert!(m.ffta_dialogue.extra.pixels.iter().any(|p|p.visible()));
            assert!(m.wide_framebuffer[..568*160].iter().all(|&p|p==0));
        }}}
        crate::mem::set_extra(0);
    }

    #[test]
    fn hover_stats_use_one_fixed_target_through_offset_and_blend_transitions() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for mode in [0u16,0x400] {for (dx,dy) in [(-180,0),(70,0),(0,-115),(0,45),(0,0)] {
            let mut m=stats_scene();
            for o in m.oam.chunks_exact_mut(8) {
                let a=u16::from_le_bytes([o[0],o[1]]);if a&0x300==0x200{continue;}
                let b=u16::from_le_bytes([o[2],o[3]]);
                let c=u16::from_le_bytes([o[4],o[5]]);
                let a=if c>>12==11 {(a&!0xc00)|mode}else{a};
                o[..2].copy_from_slice(&((a&!255)|(((a&255)as i32+dy)&255)as u16).to_le_bytes());
                o[2..4].copy_from_slice(&((b&!511)|(((b&511)as i32+dx)&511)as u16).to_le_bytes());
            }
            let oam=m.oam.clone();
            for y in 0..160 {render_scanline(&mut m,y);}
            assert_eq!(m.ffta_dialogue.stats_target,Some([152,104,240,152]));
            let node=crate::ffta_ui::Scene::capture(&m.ffta_dialogue).nodes[1].unwrap();
            assert_eq!(node.role,crate::ffta_ui::Role::Stats);
            assert_eq!(node.anchor,crate::ffta_ui::Anchor::Corner{right:true,bottom:true});
            assert!(m.ffta_dialogue.extra.pixels.iter().any(|p|p.visible()));
            assert_eq!(m.oam,oam);
        }}
        crate::mem::set_extra(0);
    }
    #[test]
    fn stats_and_forecast_keep_custom_layer_through_partial_entry_and_exit(){
        crate::mem::set_extra(164);crate::mem::set_extra_y(80);
        for forecast in [false,true]{for (dx,dy) in [(-180,0),(70,0),(0,-115),(0,45),(0,0)]{
            let mut m=stats_scene();
            if forecast{for(bg,cnt)in[0x4c03u16,0x4e02,0x5c08,0x4a09].into_iter().enumerate(){m.io[8+bg*2..10+bg*2].copy_from_slice(&cnt.to_le_bytes());}}
            for obj in m.oam.chunks_exact_mut(8){
                let a=u16::from_le_bytes([obj[0],obj[1]]);if a&0x300==0x200{continue;}
                let b=u16::from_le_bytes([obj[2],obj[3]]);
                let x=((b&511)as i32+dx)&511;let y=((a&255)as i32+dy)&255;
                obj[..2].copy_from_slice(&((a&!255)|y as u16).to_le_bytes());obj[2..4].copy_from_slice(&((b&!511)|x as u16).to_le_bytes());
            }
            let original=m.oam.clone();begin_dialogue_frame(&mut m);
            assert_eq!(m.ffta_dialogue.extra.kind,crate::ffta_dialogue::PanelKind::Stats,"forecast={forecast} dx={dx} dy={dy}");
            assert!(m.ffta_dialogue.extra.active());
            if forecast{assert!(m.ffta_dialogue.placement.is_some());}
            assert_eq!(m.oam,original);
        }}
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn attacker_action_layer_is_recognized_during_guest_scroll_animation(){
        crate::mem::set_extra(164);crate::mem::set_extra_y(80);
        for (x,y) in [(0u16,0u16),(1,0),(511,0),(0,24),(0,488)]{
            let mut m=scene(false);m.ffta_dialogue.separate=true;
            m.io[12..14].copy_from_slice(&0x5c08u16.to_le_bytes());m.io[14..16].copy_from_slice(&0x4a09u16.to_le_bytes());
            m.io[0x18..0x1a].copy_from_slice(&x.to_le_bytes());m.io[0x1a..0x1c].copy_from_slice(&y.to_le_bytes());
            begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.action_layout,"scroll={x},{y}");assert!(m.ffta_dialogue.placement.is_some());
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
        begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.extra.active(),"partial exit keeps the same owner");
        for obj in m.oam.chunks_exact_mut(8){obj[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
        begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.extra.active(),"absence never replays old UI pixels");
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
    fn forecast_hud_keeps_winout_alpha_below_opaque_top_capsule() {
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=scene(false);m.ffta_dialogue.separate=true;
        for(o,v)in[(0,0x3f40u16),(12,0x5c08),(14,0x4a09),
            (0x40,0x10e0),(0x44,0x0818),(0x48,31),(0x4a,63),
            (0x50,0x3348),(0x52,0x090e)] {
            m.io[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
        m.vram[0x5000..0x5800].fill(0);
        for y in 13..19 {for x in 0..17 {
            let o=0x5000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
        }}
        for y in 0..160 {render_scanline(&mut m,y);}
        let p=m.ffta_dialogue.pixels[110*480+120+80];
        assert!(p.visible());assert_eq!((p.eva,p.evb),(14,9));
        crate::mem::set_extra(0);
    }
    #[test]
    fn notice_window_is_a_complete_modal_and_hides_suspended_action_hud() {
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
        assert!(!m.ffta_dialogue.pixels[idx(16,112)].visible());
        assert_eq!(m.ffta_dialogue.extra.pixels[idx(27,44)].resolve(0),0x03e0);
        assert!(!m.ffta_dialogue.pixels[idx(27,44)].visible());
        assert!(!m.ffta_dialogue.extra.pixels[idx(16,112)].visible());
        // The modal remains pixel-identical; the suspended HUD outside it
        // is intentionally absent in the host presentation.
        for y in 16..64 {for x in 16..224 {
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
    fn open_action_child_wins_over_previous_closing_parent(){
        let mut m=scene(false);
        m.io[12..14].copy_from_slice(&0x5c08u16.to_le_bytes());m.io[14..16].copy_from_slice(&0x4a09u16.to_le_bytes());
        m.ffta_dialogue.action_owner_bounds=Some([176,64,240,152]);
        for(o,rect,current,target)in[(0x1000,[22u8,8,8,11],4u16,0u16),(0x1100,[19,10,11,9],11,11)]{
            for(a,v)in[(0,0x0600e000u32),(4,0x06005000),(24,0x00010005)]{m.ewram[o+a..o+a+4].copy_from_slice(&v.to_le_bytes());}
            m.ewram[o+16..o+20].copy_from_slice(&rect);
            for(a,v)in[(20,0x0140u16),(22,0x0c20),(32,current),(34,rect[3]as u16),(36,target),(38,rect[3]as u16)]{m.ewram[o+a..o+a+2].copy_from_slice(&v.to_le_bytes());}
        }
        assert_eq!(action_menu_bounds(&m),Some([152,80,240,152]));
    }
    fn action_ui_fixture()->MemMap {
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
        m
    }
    #[test]
    #[ignore = "release UI capture microbenchmark; not gameplay FPS"]
    fn bench_action_ui_frame_cost() {
        for (ex,ey) in [(164,0),(448,80)] {
            crate::mem::set_extra(ex);crate::mem::set_extra_y(ey);
            let mut m=action_ui_fixture();
            for y in 0..160 {render_scanline(&mut m,y);}
            let start=std::time::Instant::now();
            for _ in 0..300 {for y in 0..160 {render_scanline(&mut m,y);}std::hint::black_box(&m);}
            let hash=m.ffta_dialogue.pixels.iter().chain(&m.ffta_dialogue.extra.pixels).fold(
                0xcbf29ce484222325u64,|h,p|(h^p.color as u64^((p.eva as u64)<<16)^((p.evb as u64)<<24)).wrapping_mul(0x100000001b3));
            println!("UI_ACTION_BENCH ex={ex} ey={ey} mean_ms={:.3} ui_fnv={hash:016x}",start.elapsed().as_secs_f64()*1000./300.);
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
    #[test]
    fn actions_have_two_disjoint_layers_with_portrait_and_cursor() {
        use crate::ffta_dialogue::{LAYER_WIDTH,LAYER_INSET,PanelKind};
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        let mut m=action_ui_fixture();
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
    fn opening_and_closing_scroll_keep_custom_dialogue_attachment_without_old_pixels() {
        for right in [false,true] {
            crate::mem::set_extra(164);crate::mem::set_extra_y(80);
            let mut m=scene(right);m.ffta_dialogue.separate=true;
            m.ffta_dialogue.style.scale=1.4;m.ffta_dialogue.style.length=1.3;
            let oam=m.oam.clone();
            for (hofs,vofs,layers) in [(64u16,16u16,0x800u16),(32,8,0xc00),(0,0,0xc00),(496,248,0x400),(480,240,0x800)] {
                m.io[..2].copy_from_slice(&(0x1340|layers).to_le_bytes());
                m.io[0x1c..0x1e].copy_from_slice(&hofs.to_le_bytes());m.io[0x1e..0x20].copy_from_slice(&vofs.to_le_bytes());
                for y in 0..160 {render_scanline(&mut m,y);}
                let p=m.ffta_dialogue.placement.expect("animation stays on custom canvas");
                assert_eq!(p.right,right);assert!((p.scale-2.8).abs()<0.0001);assert_eq!(p.length_pixels,36);
                assert_eq!(m.ffta_dialogue.vertical_anchor(),0.);assert_eq!(m.oam,oam);
            }
            m.vram[0x5000..0x6000].fill(0);for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
            for y in 0..160 {render_scanline(&mut m,y);}
            assert!(m.ffta_dialogue.pixels.iter().all(|p|!p.visible()),"no retained portrait or bubble pixels");
            m.io[12]=9;render_scanline(&mut m,0);assert!(m.ffta_dialogue.layer().is_none());assert!(m.ffta_dialogue.attachment.is_none());
        }
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
    #[ignore = "manual release CPU renderer benchmark, not gameplay FPS"]
    fn bench_dialogue_frame_cost() {
        for (ex,ey,separate) in [(0,0,false),(164,0,false),(164,0,true),(448,80,true)] {
            crate::mem::set_extra(ex);crate::mem::set_extra_y(ey);
            let mut m=scene(true);m.ffta_dialogue.separate=separate;
            for y in 0..160 {render_scanline(&mut m,y);}
            let start=std::time::Instant::now();
            for _ in 0..100 {for y in 0..160 {render_scanline(&mut m,y);} std::hint::black_box(&m);}
            let hash=m.wide_framebuffer.iter().chain(&m.framebuffer).chain(m.oam.chunks_exact(2).map(|_|&0u16))
                .fold(0xcbf29ce484222325u64,|h,&v|(h^v as u64).wrapping_mul(0x100000001b3));
            let ui=m.ffta_dialogue.pixels.iter().fold(0xcbf29ce484222325u64,|h,p| {
                (h^p.color as u64^((p.eva as u64)<<16)^((p.evb as u64)<<24)).wrapping_mul(0x100000001b3)
            });
            println!("UI_BENCH ex={ex} ey={ey} separate={separate} mean_ms={:.3} frame_fnv={hash:016x} ui_fnv={ui:016x}",start.elapsed().as_secs_f64()*10.);
        }
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
    let mode=io16(mem,0)&7;
    if mode!=0 && !(mode==1 && bg<2) {
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

fn world_paper_pixel(mem:&MemMap,bg:usize,x:usize,y:usize)->u16 {
    let o=0x5000+bg*4096+y/256*2048+(y%256/8*32+x/8)*2;
    let e=u16::from_le_bytes([mem.vram[o],mem.vram[o+1]]);
    let u=if e&1024!=0{7-x%8}else{x%8};
    let v=if e&2048!=0{7-y%8}else{y%8};
    let b=mem.vram[(e as usize&1023)*32+v*4+u/2];
    let index=(b>>(u%2*4))as usize&15;
    if index==0{0x7fff}else{pal16(mem,(e as usize>>12)*16+index)}
}
// One continuous paper silhouette: rounded top/bottom corners, no isolated
// scallops where a coastline changes color along the native image edge.
fn world_paper_cap(y:usize)->f32 {
    let edge=(y as f32+0.5).min(351.5-y as f32).max(0.);
    let dy=(24.-edge).max(0.);
    12.*(1.-dy*dy/(24.*24.)).max(0.).sqrt()
}
fn world_edge_is_land(c:u16)->bool {(c&31)>(c>>10&31)+4}
fn world_edge_is_sea(c:u16)->bool {
    let r=c&31;let g=c>>5&31;let b=c>>10&31;
    g>r+3 && b>r+3
}
fn world_ocean_colors(mem:&MemMap)->[u16;3] {
    let mut sums=[[0u32;3];3];let mut count=0;
    let alpha=io16(mem,0x52);let eva=(alpha&31).min(16)as u32;let evb=(alpha>>8&31).min(16)as u32;
    for y in (0..352).step_by(8) {for x in (0..240).step_by(8) {
        let a=world_paper_pixel(mem,1,x,y);
        if !world_edge_is_sea(a){continue;}
        let b=world_paper_pixel(mem,0,x,y);
        for (channel,shift)in [0,5,10].into_iter().enumerate(){
            let av=((a>>shift)&31)as u32;let bv=((b>>shift)&31)as u32;
            sums[0][channel]+=bv;sums[1][channel]+=av;
            sums[2][channel]+=((av*eva+bv*evb)/16).min(31);
        }
        count+=1;
    }}
    if count==0 {let c=world_paper_pixel(mem,0,0,0);return [c;3];}
    std::array::from_fn(|layer| (0..3).fold(0u16,|c,ch|c|(((sums[layer][ch]+count/2)/count)as u16)<<(ch*5)))
}
fn world_land_cap(mem:&MemMap,right:bool,y:usize)->Option<(f32,u16)> {
    let x=if right{239}else{0};
    if !world_edge_is_land(world_paper_pixel(mem,1,x,y)){return None;}
    let mut top=y;let mut bottom=y+1;
    while top>0 && world_edge_is_land(world_paper_pixel(mem,1,x,top-1)){top-=1;}
    while bottom<352 && world_edge_is_land(world_paper_pixel(mem,1,x,bottom)){bottom+=1;}
    let radius=((bottom-top)as f32*0.5).min(8.);
    let center=(y as f32+0.5).clamp(top as f32+radius,bottom as f32-radius);
    let dy=y as f32+0.5-center;
    let cap=(radius*radius-dy*dy).max(0.).sqrt();
    let sea=if top>0 && y-top<=bottom-y {world_paper_pixel(mem,1,x,top-1)}
        else if bottom<352 {world_paper_pixel(mem,1,x,bottom)}else{0x7fff};
    Some((cap,sea))
}
fn world_blurred_fog(mem:&MemMap,right:bool,y:usize,distance:i32,cap:f32)->u16 {
    use std::sync::atomic::Ordering::Relaxed;
    let cached=mem.ffta_dialogue.world_fog.get((y*2+right as usize)*32+(distance+8)as usize);
    if let Some(value)=cached {let color=value.load(Relaxed);if color!=TRANSPARENT{return color;}}
    let t=((distance as f32+4.)/(cap+8.)).clamp(0.,1.);
    let radius=1+(t*3.).floor()as usize;
    let weights:&[u32]=match radius {
        1=>&[1,2,1],2=>&[1,4,6,4,1],3=>&[1,6,15,20,15,6,1],
        _=>&[1,8,28,56,70,56,28,8,1],
    };
    let weight=weights.iter().sum::<u32>().pow(2);
    let inset=distance.unsigned_abs().min(18)as i32;
    let x=if right{239-inset}else{inset};
    let mut sum=[0u32;3];
    let alpha=io16(mem,0x52);let eva=(alpha&31).min(16)as u32;let evb=(alpha>>8&31).min(16)as u32;
    let blend=io16(mem,0x50)&0x01c2==0x0142;
    for (j,&wy) in weights.iter().enumerate() {for (i,&wx) in weights.iter().enumerate() {
        let sx=(x+i as i32-radius as i32).clamp(0,239)as usize;
        let sy=(y as i32+j as i32-radius as i32).clamp(0,351)as usize;
        let b=world_paper_pixel(mem,0,sx,sy);
        let raw=world_paper_pixel(mem,1,sx,sy);
        // Blur the ocean/paper texture only, never ghost an inland marker or
        // road into the external fog. Yellow coastline keeps its own cap.
        let a=if world_edge_is_sea(raw){raw}else{mem.ffta_dialogue.world_ocean[1]};
        for (ch,shift)in [0,5,10].into_iter().enumerate() {
            let av=((a>>shift)&31)as u32;let bv=((b>>shift)&31)as u32;
            let color=if blend{((av*eva+bv*evb)/16).min(31)}else{av};
            sum[ch]+=color*wx*wy;
        }
    }}
    let fade=(t*t*(3.-2.*t)*256.).round()as u32;
    let blurred=(0..3).fold(0u16,|color,ch| {
        let value=(sum[ch]*256+weight/2)/weight;
        let mixed=(value*(256-fade)+31*256*fade+32768)/65536;
        color|(mixed.min(31)as u16)<<(ch*5)
    });
    // Ease into the filtered image too: replacing the first border column
    // outright leaves a visible seam even when the outgoing fog is smooth.
    let join=((distance as f32+4.)/6.).clamp(0.,1.);
    let join=(join*join*(3.-2.*join)*256.).round()as u32;
    let a=world_paper_pixel(mem,1,x as usize,y);
    let b=world_paper_pixel(mem,0,x as usize,y);
    let color=[0,5,10].into_iter().fold(0u16,|color,shift| {
        let av=((a>>shift)&31)as u32;let bv=((b>>shift)&31)as u32;
        let original=if blend{((av*eva+bv*evb)/16).min(31)}else{av};
        let filtered=((blurred>>shift)&31)as u32;
        color|(((original*(256-join)+filtered*join+128)/256)as u16)<<shift
    });
    if let Some(value)=cached{value.store(color,Relaxed);}
    color
}
fn extend_world_paper(mem:&MemMap,bg:usize,line:usize,out:&mut Line,ex:usize,rw:usize) {
    if ex==0 || !mem.ffta_dialogue.world_paper || bg>=2
        || mem.ffta_bg_edge==FftaBgEdge::Wrap{return;}
    let hofs=(io16(mem,16+bg*4)&511)as i32;
    let hofs=if hofs>=256{hofs-512}else{hofs};
    let y=(line as i32+(io16(mem,18+bg*4)&511)as i32).rem_euclid(512)as usize;
    let caps=if y<352{[world_paper_cap(y);2]}else{[0.;2]};
    let land=if y<352{[world_land_cap(mem,false,y),world_land_cap(mem,true,y)]}else{[None;2]};
    let sea=mem.ffta_dialogue.world_ocean;
    for x in (0..ex+4).chain(ex+236..rw) {
        let world_x=x as i32-ex as i32+hofs;
        if (4..236).contains(&world_x) && y<352 {continue;}
        if (0..240).contains(&world_x) && y<352
            && world_edge_is_land(world_paper_pixel(mem,1,world_x as usize,y)){continue;}
        let right=world_x>=120;
        let distance=if right{world_x-239}else{-world_x};
        let cap=caps[right as usize];
        if y>=352 || distance as f32>=cap+4. {out[x]=if bg==1{TRANSPARENT}else{0x7fff};continue;}
        let keep_coast=distance>=0 && land[right as usize].is_some_and(|(coast,_)|distance as f32<=coast);
        if !keep_coast {
            out[x]=if bg==1{TRANSPARENT}else{world_blurred_fog(mem,right,y,distance,cap)};
            continue;
        }
        // Continue a small interior texture patch, not a single edge column.
        // Land caps stop before the first interior route/location markers.
        let inset=distance.clamp(0,18)as usize;
        let source_x=if right{239-inset}else{inset};
        let mut c=world_paper_pixel(mem,bg,source_x,y);
        if bg==1 {
            if let Some((coast,sea))=land[right as usize] {
                if distance as f32>coast {c=sea;}
            }else if world_edge_is_land(c) {
                // Do not mirror an inland route or location into the ocean.
                c=world_paper_pixel(mem,1,if right{239}else{0},y);
            }
        }
        // No white outline around the coast: ocean continues beyond it, and
        // only that exterior ocean dissolves into the white screen.
        if bg==1 && !world_edge_is_land(c) && !world_edge_is_sea(c) {
            c=sea[1];
        }
        let t=((distance as f32-cap).max(0.)/64.).clamp(0.,1.);
        let fade=(t*t*(3.-2.*t)*256.).round()as u32;
        let mut color=0u16;
        for shift in [0,5,10] {
            let value=((c>>shift)&31)as u32;
            color|=((value+(31-value)*fade/256)as u16)<<shift;
        }
        out[x]=color;
    }
}

#[cfg(test)]
mod world_map_edge_tests {
    use super::*;
    #[test]
    fn world_paper_keeps_color_until_visible_rounded_outline() {
        let mut m=MemMap::new(vec![]);
        m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
        m.ffta_bg_edge=FftaBgEdge::Transparent;
        for(o,v)in[(0,0x1f40u16),(8,0xaa03),(10,0xac02)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.vram[32..64].fill(0x11);m.palette[2..4].copy_from_slice(&0x4210u16.to_le_bytes());
        for base in [0x5000,0x6000]{for y in 0..44{for x in 0..30{
            let o=base+y/32*2048+(y%32*32+x)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
        }}}
        let original=m.vram.clone();let oam=m.oam.clone();
        begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.world_paper);
        for bg in [0,1] {
            let mut native=[TRANSPARENT;MAX_RENDER_WIDTH];let mut wide=native;
            render_text_bg_hardware(&m,bg,40,&mut native,0,240);
            render_text_bg_hardware(&m,bg,40,&mut wide,164,568);
            assert_eq!(&wide[168..400],&native[4..236]);
            if bg==0 {assert!(wide[163]>=0x4210 && wide[163]<0x7fff);assert!(wide[404]>=0x4210 && wide[404]<0x7fff);}
            else {assert_eq!(wide[163],TRANSPARENT);assert_eq!(wide[404],TRANSPARENT);}
            assert_eq!(wide[0],if bg==0{0x7fff}else{TRANSPARENT});
            assert_eq!(wide[567],if bg==0{0x7fff}else{TRANSPARENT});
            assert_eq!(wide[172],0x4210,"map interior stays sharp");
            if bg==0 {assert!(wide[151]>0x4210 && wide[151]<0x7fff,"mist is pale but has no solid white seam");}
            else {assert_eq!(wide[151],TRANSPARENT,"no double blend outside the rounded silhouette");}
            if bg==0 {
                assert!(wide[156]>0x4210 && wide[156]<0x7fff,"short fog band retains intermediate colors");
                assert_eq!(wide[136],0x7fff,"fog reaches pure white within sixteen pixels");
            }
        }
        assert_eq!(m.vram,original);assert_eq!(m.oam,oam);
        m.io[10..12].fill(0);begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.world_paper);
    }
    #[test]
    fn map_fog_really_blurs_texture_instead_of_only_changing_tint() {
        let mut m=MemMap::new(vec![]);
        m.palette[2..4].copy_from_slice(&0u16.to_le_bytes());
        m.palette[4..6].copy_from_slice(&0x7fffu16.to_le_bytes());
        m.palette[6..8].copy_from_slice(&0x7fe0u16.to_le_bytes());
        m.ffta_dialogue.world_ocean=[0,0x7fe0,0];
        m.io[0x50..0x52].copy_from_slice(&0x0142u16.to_le_bytes());
        m.io[0x52..0x54].copy_from_slice(&0x0808u16.to_le_bytes());
        for y in 0..8 {m.vram[32+y*4..36+y*4].fill(if y%2==0{0x11}else{0x22});}
        m.vram[96..128].fill(0x33);
        for y in 0..44 {for x in 0..30 {for (base,tile)in[(0x5000,1u16),(0x6000,3)] {
            let o=base+y/32*2048+(y%32*32+x)*2;
            m.vram[o..o+2].copy_from_slice(&tile.to_le_bytes());
        }}}
        assert_ne!(world_paper_pixel(&m,0,0,40),world_paper_pixel(&m,0,0,41));
        let a=world_blurred_fog(&m,false,40,2,12.);
        let b=world_blurred_fog(&m,false,41,2,12.);
        assert_eq!(a,b,"blur removes alternating high-frequency rows");
        assert_ne!(a,0x7fff,"texture is filtered before it vanishes in white");
    }
    #[test]
    fn world_paper_outline_is_continuous_and_rounds_both_corners() {
        assert!(world_paper_cap(0)<3.);
        assert_eq!(world_paper_cap(176),12.);
        for y in 0..352 {
            assert!((world_paper_cap(y)-world_paper_cap(351-y)).abs()<0.001);
            if y>0 {assert!((world_paper_cap(y)-world_paper_cap(y-1)).abs()<2.);}
        }
    }
    #[test]
    fn long_coast_caps_use_water_beyond_the_run_not_an_inland_sample() {
        let mut m=MemMap::new(vec![]);
        m.vram[32..64].fill(0x11);m.vram[64..96].fill(0x22);
        m.palette[2..4].copy_from_slice(&0x7fffu16.to_le_bytes());
        m.palette[4..6].copy_from_slice(&0x03ffu16.to_le_bytes());
        for y in 0..44 {for x in [0,29] {
            let o=0x6000+y/32*2048+(y%32*32+x)*2;
            let tile=if (8..24).contains(&y){2u16}else{1};
            m.vram[o..o+2].copy_from_slice(&tile.to_le_bytes());
        }}
        for right in [false,true] {
            let (radius,sea)=world_land_cap(&m,right,128).unwrap();
            assert_eq!(radius,8.);assert_eq!(sea,0x7fff);
            assert!(world_land_cap(&m,right,64).unwrap().0<radius-3.);
            assert!(world_land_cap(&m,right,192).is_none());
        }
    }
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
pub(crate) fn ffta_movement_grid(mem: &MemMap, bg: usize) -> bool {
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
    fn title_paper_extends_clean_patches_without_repeating_logo_or_native_content() {
        let mut m=MemMap::new(vec![]);
        m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;m.ffta_dialogue.separate=true;
        for(o,v)in[(0,0x0140u16),(8,0x3d83),(10,0x3e0a),(12,0x3f08)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.vram[..64].fill(1);m.vram[64..128].fill(2);
        m.palette[2..4].copy_from_slice(&0x3e0u16.to_le_bytes());
        m.palette[4..6].copy_from_slice(&31u16.to_le_bytes());
        let o=0xe800+(2*32+2)*2;m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
        let original=m.vram.clone();let oam=m.oam.clone();
        for ex in [22,164,448] {
            crate::mem::set_extra(ex);begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.title_paper);
            let rw=240+ex*2;
            for y in [0,16,64,127,159] {
                let mut native=[TRANSPARENT;MAX_RENDER_WIDTH];let mut wide=native;
                render_text_bg_hardware(&m,0,y,&mut native,0,240);
                render_text_bg_hardware(&m,0,y,&mut wide,ex,rw);
                assert_eq!(&wide[ex..ex+240],&native[..240]);
                assert!(wide[..ex].iter().chain(wide[ex+240..rw].iter()).all(|&p|p==0x3e0));
                if y==16{assert_eq!(wide[ex+16],31);}
            }
        }
        assert_eq!(m.vram,original);assert_eq!(m.oam,oam);
        m.io[8..10].fill(0);begin_dialogue_frame(&mut m);
        assert!(!m.ffta_dialogue.title_paper);
        crate::mem::set_extra(0);
    }
    #[test]
    fn clan_backdrop_reflects_margins_without_changing_native_image() {
        let mut m=MemMap::new(vec![]);
        m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
        for(o,v)in[(0,0x3d40u16),(8,0x8a01),(10,0x8c0a)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        for y in 0..20 {for x in 0..30 {
            let tile=y*30+x;let o=0x5000+(y*32+x)*2;
            m.vram[o..o+2].copy_from_slice(&(tile as u16).to_le_bytes());
            m.vram[tile*32..tile*32+32].fill(if x%2==0{0x11}else{0x22});
        }}
        m.palette[2..4].copy_from_slice(&31u16.to_le_bytes());
        m.palette[4..6].copy_from_slice(&0x3e0u16.to_le_bytes());
        let mut native=[TRANSPARENT;MAX_RENDER_WIDTH];
        render_text_bg_hardware(&m,0,80,&mut native,0,240);
        begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.menu_pattern);
        let mut wide=native;render_text_bg_hardware(&m,0,80,&mut wide,164,568);
        assert_eq!(&wide[164..404],&native[..240]);
        for d in 0..164 {assert_eq!(wide[163-d],native[d]);assert_eq!(wide[404+d],native[239-d]);}
        m.vram[0x5000..0x5002].copy_from_slice(&1u16.to_le_bytes());
        begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.menu_pattern);
    }
    #[test]
    fn party_motif_uses_its_own_map_and_rejects_foreground_content() {
        let mut m=MemMap::new(vec![]);
        m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
        for(o,v)in[(0,0x1f40u16),(8,0xae0b),(10,0xac0a)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let motif=[[0xa03cu16,0xa03d,0xa43d,0xa43c],[0xa83c,0xa83d,0xac3d,0xac3c]];
        for y in 0..64 {for x in 0..32 {
            let o=0x7000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&motif[y%2][x%4].to_le_bytes());
        }}
        begin_dialogue_frame(&mut m);assert!(m.ffta_dialogue.menu_pattern);
        m.vram[0x7ffe..0x8000].fill(0);
        begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.menu_pattern);
    }
    #[test]
    fn verified_menu_motif_tiles_both_axes_without_changing_native_pixels() {
        let mut m=MemMap::new(vec![]);
        m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
        for(o,v)in[(0,0x1f40u16),(8,0xad0b),(10,0xac09)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let motif=[[0xa03cu16,0xa03d,0xa43d,0xa43c],[0xa83c,0xa83d,0xac3d,0xac3c]];
        for y in 0..64 {for x in 0..32 {
            let o=0x6800+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&motif[y%2][x%4].to_le_bytes());
        }}
        m.vram[0x8780..0x87a0].fill(0x11);m.vram[0x87a0..0x87c0].fill(0x22);
        m.palette[0x142..0x144].copy_from_slice(&31u16.to_le_bytes());
        m.palette[0x144..0x146].copy_from_slice(&0x3e0u16.to_le_bytes());
        let vram=m.vram.clone();let oam=m.oam.clone();
        for ex in [0,22,164] {
            crate::mem::set_extra(ex);begin_dialogue_frame(&mut m);
            assert!(m.ffta_dialogue.menu_pattern);
            let mut native=[TRANSPARENT;MAX_RENDER_WIDTH];
            render_text_bg_hardware(&m,0,8,&mut native,0,240);
            for y in [-520,-80,-8,8,168,520] {
                let mut wide=vec![0;240+ex*2];
                render_menu_background_row(&m,y,&mut wide);
                for (x,&p)in wide.iter().enumerate(){
                    assert_eq!(p,native[(x as i32-ex as i32).rem_euclid(32)as usize]);
                }
            }
        }
        assert_eq!(m.vram,vram);assert_eq!(m.oam,oam);
        m.vram[0x77fe..0x7800].fill(0);
        begin_dialogue_frame(&mut m);
        assert!(!m.ffta_dialogue.menu_pattern,"a foreground/nonperiodic map must not tile");
        crate::mem::set_extra(0);
    }
    #[test]
    fn affine_spell_keeps_text_terrain_reconstruction_without_wrapping_margins() {
        let mut m=MemMap::new(vec![]);
        m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
        m.ffta_dialogue.separate=true;m.ffta_stage.enabled=true;
        m.ffta_stage.bases=[Some(0x02001000),Some(0x02003000)];
        m.ffta_stage.origins=[Some((0,0)),Some((0,0))];m.ffta_stage.valid=[true,true];
        for e in m.ewram[0x1000..0x5000].chunks_exact_mut(2){e.copy_from_slice(&1u16.to_le_bytes());}
        for e in m.vram[0x6000..0x8000].chunks_exact_mut(2){e.copy_from_slice(&1u16.to_le_bytes());}
        m.vram[32..64].fill(0x11);m.palette[2..4].copy_from_slice(&31u16.to_le_bytes());
        for(o,v)in[(8,0x4c03u16),(10,0x4e02),(12,0x4b8c)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let mut previous=None;
        for display in [0x1740u16,0x1741] {
            m.io[..2].copy_from_slice(&display.to_le_bytes());
            for bg in 0..2 {
                assert!(ffta_terrain(&m,bg));assert_eq!(crate::ffta_stage::layer(&m,bg),Some(bg));
                let mut row=[TRANSPARENT;MAX_RENDER_WIDTH];
                render_text_bg_hardware(&m,bg,8,&mut row,164,568);
                assert!(row[..164].iter().all(|&p|p==TRANSPARENT));
                assert_eq!(row[164],31);assert_eq!(row[567],31);
                if let Some(ref old)=previous{assert_eq!(&row[..568],old);}else{previous=Some(row[..568].to_vec());}
            }
        }
        assert!(crate::ffta_effects::viewport_local(&m,2));
        assert!(!ffta_terrain(&m,2));assert_eq!(crate::ffta_stage::layer(&m,2),None);
        m.io[0]=0x42;
        assert!(!ffta_terrain(&m,0));assert_eq!(crate::ffta_stage::layer(&m,0),None);
    }
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
            let asset_row = crate::asset_cache::enabled().then(||crate::asset_cache::row(
                &mem.vram,&mem.palette,tile-fy*4,false,palette,fy));
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
                if let Some(row)=asset_row {out[sx+i]=row[fx];continue;}
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

fn repeating_menu_background(mem:&MemMap)->bool {
    if !mem.ffta_movement.europe_profile || !mem.ffta_movement.enabled
        || io16(mem,0)&0x0187!=0x0100{return false;}
    let base=match (io16(mem,8),io16(mem,10)) {
        (0x8a01,0x8c0a)=>return (0..20).all(|y|(0..30).all(|x| {
            let o=0x5000+(y*32+x)*2;
            u16::from_le_bytes([mem.vram[o],mem.vram[o+1]])&1023==(y*30+x)as u16
        })), // Clan: a complete illustrated backdrop, separate from BG2/BG3 UI.
        (0xad0b,0xac09)=>0x6800, // Save / Resume Battle
        (0xae0b,0xac0a)=>0x7000, // Party gallery
        _=>return false,
    };
    let motif=[[0xa03cu16,0xa03d,0xa43d,0xa43c],[0xa83c,0xa83d,0xac3d,0xac3c]];
    // Validate the whole 256x512 tilemap once per frame. Extending another
    // menu plane could repeat labels, portraits or stale transition pixels.
    (0..64).all(|y|(0..32).all(|x| {
        let o=base+(y*32+x)*2;
        u16::from_le_bytes([mem.vram[o],mem.vram[o+1]])==motif[y%2][x%4]
    }))
}
/// Only the verified backdrop, for host rows outside a menu's native height.
/// Foreground windows/OBJ are never tiled into these extra rows.
pub fn render_menu_background_row(mem:&MemMap,line:i32,out:&mut[u16]) {
    let save_screen=mem.ffta_movement.europe_profile &&
        matches!(u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap()),0x0813dd75|0x0813d049);
    if !mem.ffta_dialogue.menu_pattern && !save_screen {out.fill(0);return;}
    let mut row=[TRANSPARENT;MAX_RENDER_WIDTH];
    render_text_bg_hardware(mem,0,line as usize,&mut row,extra(),out.len());
    for (p,&c)in out.iter_mut().zip(row.iter()){*p=if c==TRANSPARENT{pal16(mem,0)}else{c};}
}
/// Background only for expanded frontend rows. Never repeat titles, flags,
/// selectors or OBJ into the padding outside the original 240x160 canvas.
pub fn render_frontend_background_row(mem:&MemMap,line:i32,out:&mut[u16])->bool {
    let save_screen=mem.ffta_movement.europe_profile &&
        matches!(u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap()),0x0813dd75|0x0813d049);
    if mem.ffta_dialogue.menu_pattern || save_screen {
        render_menu_background_row(mem,line,out);return true;
    }
    if mem.ffta_dialogue.title_paper {
        let p=(line-64).rem_euclid(128)as usize;
        let y=64+if p<64{p}else{127-p};
        let ex=extra()as i32;
        for (x,c)in out.iter_mut().enumerate(){
            let nx=x as i32-ex;let right=nx>=120;
            let u=nx.rem_euclid(112)as usize;let u=if u<56{u}else{111-u};
            *c=title_paper_pixel(mem,if right{184+u}else{u},y);
        }
        return true;
    }
    // Verified language selector: flags and labels occupy rows 28..132.
    // Tile only the clean margin rows, never the language entries themselves.
    if io16(mem,0)==0x1140 && io16(mem,8)==0x3000 {
        let q=line.rem_euclid(32)as usize;let q=if q<16{q}else{31-q};
        let y=if line<0{q}else{144+q};
        let mut row=[TRANSPARENT;MAX_RENDER_WIDTH];
        render_text_bg_hardware(mem,0,y,&mut row,extra(),out.len());
        for (p,&c)in out.iter_mut().zip(&row){*p=if c==TRANSPARENT{pal16(mem,0)}else{c};}
        return true;
    }
    false
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
fn title_paper_pixel(mem:&MemMap,x:usize,y:usize)->u16 {
    let o=0xe800+((y/8)*32+x/8)*2;
    let entry=u16::from_le_bytes([mem.vram[o],mem.vram[o+1]]);
    let u=if entry&0x400!=0{7-x%8}else{x%8};
    let v=if entry&0x800!=0{7-y%8}else{y%8};
    let address=(entry as usize&1023)*64+v*8+u;
    pal16(mem,mem.vram[address]as usize)
}
fn extend_title_paper(mem:&MemMap,line:usize,out:&mut Line,ex:usize,rw:usize) {
    if ex==0 || !mem.ffta_dialogue.title_paper || line>=160{return;}
    let mirror=|p:i32|{let p=p.rem_euclid(128)as usize;if p<64{p}else{127-p}};
    // The title/credits are baked into BG0. Use only the clean 56x64 paper
    // patches beside the city, below the logo and above the copyright text.
    let sy=64+mirror(line as i32-64);
    for right in [false,true] {
        // Exclude the shaded columns immediately beside the city.
        let samples: [u16;56]=std::array::from_fn(|x|title_paper_pixel(mem,if right{184+x}else{x},sy));
        let edge=title_paper_pixel(mem,if right{239}else{0},line);
        let range=if right{ex+240..rw}else{0..ex};
        for x in range {
            let distance=if right{x-ex-240}else{ex-1-x};
            let p=distance%112;let u=if p<56{p}else{111-p};
            let color=samples[if right{55-u}else{u}];
            let weight=distance.min(16)as u32;
            let mut result=0;
            for shift in [0,5,10] {
                let a=((edge>>shift)&31)as u32;let b=((color>>shift)&31)as u32;
                result|=(((a*(16-weight)+b*weight+8)/16)as u16)<<shift;
            }
            out[x]=result;
        }
    }
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
        // These verified planes belong entirely to detached UI. During a
        // guest animation the owner may be between publication phases; never
        // expose that original screen-space image underneath the host slide.
        if mem.ffta_dialogue.separate && mem.ffta_movement.enabled && ffta_terrain(mem,0)
            && matches!((io16(mem,12),io16(mem,14)),(0x5c08,0x4a09)|(0x0b08,0x0a08)) {
            out[..rw].fill(TRANSPARENT);return;
        }
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
    render_text_bg_hardware_scroll(mem,bg,line,out,ex,rw,None);
}
#[allow(clippy::too_many_arguments)]
fn render_text_bg_hardware_scroll(mem:&MemMap,bg:usize,line:usize,out:&mut Line,ex:usize,rw:usize,scroll:Option<(usize,usize)>) {
    if bg==0 && mem.ffta_dialogue.menu_pattern && io16(mem,8)==0x8a01 {
        // Reflect the finite illustration at its edges, never the foreground.
        // Keep native coordinates unchanged; negative host rows reflect too.
        let reflect=|p:i32,n:i32| {let p=p.rem_euclid(n*2);if p<n{p}else{n*2-1-p}};
        let y=reflect(line as i32,160)as usize;
        for (screen,pixel) in out[..rw].iter_mut().enumerate() {
            let x=reflect(screen as i32-ex as i32,240)as usize;
            let o=0x5000+(y/8*32+x/8)*2;
            let e=u16::from_le_bytes([mem.vram[o],mem.vram[o+1]]);
            let u=if e&0x400!=0{7-x%8}else{x%8};
            let v=if e&0x800!=0{7-y%8}else{y%8};
            let b=mem.vram[(e as usize&1023)*32+v*4+u/2];
            let index=(b>>(4*(u%2)))&15;
            *pixel=if index==0{TRANSPARENT}else{pal16(mem,(e as usize>>12)*16+index as usize)};
        }
        return;
    }
    let cnt = io16(mem, 0x08 + bg * 2);
    let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
    let eight_bpp = cnt & 0x80 != 0;
    let screen_base = ((cnt >> 8) & 0x1F) as usize * 0x800;
    let size = (cnt >> 14) & 3;

    let hofs = scroll.map_or((io16(mem,0x10+bg*4)&0x1ff)as usize,|p|p.0);
    let finite_world = ex>0 && mem.ffta_bg_edge!=FftaBgEdge::Wrap && ffta_world_map(mem,bg);
    let vofs = scroll.map_or((io16(mem,0x12+bg*4)&0x1ff)as usize,|p|p.1);
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
        && !(bg==0 && mem.ffta_dialogue.menu_pattern)
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
    let effect=crate::ffta_effects::viewport_local(mem,bg);
    if effect && !(0..160).contains(&(line as i32)){out[..rw].fill(TRANSPARENT);return;}

    // Walk the line in tile spans: all pixels within one span share the
    // same map entry, so fetch and decode it once. The walk covers the
    // wide buffer; margin columns read the map at negative / past-240
    // screen positions (wrapping at the map width, like the hardware).
    let mut sx = 0usize;
    while sx < rw {
        let screen_x = sx as isize - ex as isize;
        if mem.ffta_dialogue.wide_pass && mem.ffta_dialogue.funds_bg==Some(bg)
            && mem.ffta_dialogue.funds_bounds.is_some_and(|[l,t,r,b]|line as i32>=t && (line as i32)<b
                && screen_x>=l as isize && screen_x<r as isize) {
            out[sx]=TRANSPARENT;sx+=1;continue;
        }
        let in_margin = ex>0 && (screen_x < 0 || screen_x >= VISIBLE_WIDTH as isize);
        if effect && in_margin {
            let end=if screen_x<0 {ex.min(rw)}else{rw};
            out[sx..end].fill(TRANSPARENT);sx=end;continue;
        }
        let (sample_x,horizontal_policy)=if finite_world && in_margin {
            let signed_hofs=if hofs>=256 {hofs as isize-512}else{hofs as isize};
            (screen_x+signed_hofs,FftaBgEdge::Transparent)
        }else{(screen_x+hofs as isize,policy)};
        let Some(x) = edge_coordinate(sample_x, map_w, horizontal_policy) else {
            // A finite plane's off-map interval contains no tiles. Skip the
            // whole transparent run, not a map lookup per empty pixel.
            let span=if sample_x<0 {(-sample_x)as usize}else{rw-sx};
            // Native viewport and margins can use different edge policies.
            let boundary=if screen_x<0 {ex}else if screen_x<240 {ex+240}else{rw};
            let end=(sx+span.min(rw-sx)).min(boundary);
            out[sx..end].fill(TRANSPARENT);sx=end;continue;
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
            && !(bg==0 && mem.ffta_dialogue.menu_pattern)
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
        let mut entry = u16::from_le_bytes([mem.vram[entry_off], mem.vram[entry_off + 1]]);
        if bg==2 && entry>>12==11 && crate::ffta_battle_mouse::pub_scroll_unselected(mem)
            && (!crate::ffta_roster_mouse::unselected(mem)
                || crate::ffta_roster_mouse::neutral_tile(mem,entry_off)) {
            entry=(entry&0x0fff)|(12<<12);
        }

        let tile = (entry & 0x3FF) as usize;
        let hflip = entry & 0x400 != 0;
        let vflip = entry & 0x800 != 0;
        let fy = if vflip { 7 - py } else { py };

        let bytes=if eight_bpp {64}else{32};
        let asset_base=char_base+tile*bytes;
        if crate::asset_cache::enabled() && asset_base+bytes<=0x10000 {
            let bank=if eight_bpp {0}else{((entry>>12)as usize&15)*16};
            let row=crate::asset_cache::row(&mem.vram,&mem.palette,asset_base,eight_bpp,bank,fy);
            for i in 0..span {out[sx+i]=row[if hflip {7-px-i}else{px+i}];}
            sx+=span;continue;
        }

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
        && !(bg==0 && mem.ffta_dialogue.menu_pattern)
        && !(bg>=2 && ffta_world_map(mem,0))
        && !effect
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
    if bg==0 {extend_title_paper(mem,line,out,ex,rw);}
    extend_world_paper(mem,bg,line,out,ex,rw);
}

// ---- affine backgrounds ----

fn render_affine_bg(mem: &MemMap, bg: usize, out: &mut Line) {
    let cnt = io16(mem, 0x08 + bg * 2);
    let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
    let screen_base = ((cnt >> 8) & 0x1F) as usize * 0x800;
    let wrap = cnt & 0x2000 != 0;
    let effect=crate::ffta_effects::viewport_local(mem,bg);
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

    let mut asset_row:Option<(usize,usize,[u16;8])>=None;
    for (column,out_px) in out[..render_width()].iter_mut().enumerate() {
        let mut tx = fx >> 8;
        let mut ty = fy >> 8;
        fx = fx.wrapping_add(pa);
        fy = fy.wrapping_add(pc);

        let outside_native=column<extra() || column>=extra()+240;
        if effect && outside_native {
            *out_px=TRANSPARENT;continue;
        }
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
        if crate::asset_cache::enabled() {
            let base=char_base+tile*64;let y=ty%8;
            if asset_row.as_ref().is_none_or(|r|r.0!=base || r.1!=y) {
                asset_row=Some((base,y,crate::asset_cache::row(&mem.vram,&mem.palette,base,true,0,y)));
            }
            *out_px=asset_row.as_ref().unwrap().2[tx%8];continue;
        }
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
    fn move_layer_swap_hides_parked_portrait_without_hiding_world_objects() {
        use crate::bus::Bus;
        use crate::ffta_sprites::HostSprite;
        let mut mem=MemMap::new(vec![]);
        mem.write16(0x04000000,0x1f40);
        for (offset,value) in [(8,0x0a0b),(10,0x0b0a),(12,0x4c03),(14,0x4e02)] {
            mem.io[offset..offset+2].copy_from_slice(&(value as u16).to_le_bytes());
        }
        for o in mem.oam.chunks_exact_mut(8) {o[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
        mem.vram[0x10000+0x240*32..0x10000+0x280*32].fill(1);
        mem.vram[0x10020..0x10040].fill(0x11);
        mem.write16(0x05000202,31);
        mem.ffta_sprites.active=Some(vec![
            HostSprite {x:-136,y:88,attrs:[0xa058,0xd178,0x640],slot:3,replaces:false,..Default::default()},
            HostSprite {x:80,y:88,attrs:[88,80,1],slot:4,replaces:false,..Default::default()},
        ]);
        let original=mem.oam.clone();
        let mut out=[OBJ_EMPTY;MAX_RENDER_WIDTH];let mut window=[false;MAX_RENDER_WIDTH];
        // Negative control: widened rendering exposes the parked portrait.
        render_sprites_extent(&mem,0x1f40,88,&mut out,&mut window,164,568);
        assert_eq!(out[28].color,31);
        mem.ffta_dialogue.separate=true;mem.ffta_dialogue.wide_pass=true;
        out.fill(OBJ_EMPTY);
        render_sprites_extent(&mem,0x1f40,88,&mut out,&mut window,164,568);
        assert_eq!(out[28].color,TRANSPARENT);
        assert_eq!(out[244].color,31);
        // Native/capture rendering still has access to the original portrait.
        mem.ffta_dialogue.wide_pass=false;out.fill(OBJ_EMPTY);
        render_sprites_extent(&mem,0x1f40,88,&mut out,&mut window,164,568);
        assert_eq!(out[28].color,31);
        assert_eq!(mem.oam,original);
    }

    #[test]
    fn move_confirmation_without_any_ui_still_runs_world_portrait_suppression() {
        use crate::bus::Bus;
        use crate::ffta_sprites::HostSprite;
        crate::mem::set_extra(164);crate::mem::set_extra_y(0);
        for controls in [[0x0a0b,0x0b0a,0x4c03,0x4e02],
            [0x4c03,0x4e02,0x5c08,0x4a09],[0x4c03,0x4e02,0x0b08,0x0a08]] {
        let mut mem=MemMap::new(vec![]);
        mem.write16(0x04000000,0x1f40);
        for (offset,value) in [8,10,12,14].into_iter().zip(controls) {
            mem.io[offset..offset+2].copy_from_slice(&(value as u16).to_le_bytes());
        }
        for o in mem.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
        mem.vram[0x14800..0x15000].fill(1);
        mem.vram[0x10020..0x10040].fill(0x11);
        mem.write16(0x05000202,31);
        mem.ffta_sprites.active=Some(vec![
            HostSprite{x:-136,y:88,attrs:[0xa058,0xd178,0x640],slot:3,replaces:false,..Default::default()},
            HostSprite{x:80,y:88,attrs:[88,80,1],slot:4,replaces:false,..Default::default()},
        ]);
        render_scanline(&mut mem,0);render_scanline(&mut mem,88);
        assert_eq!(mem.wide_framebuffer[88*568+28],31);
        let native=mem.framebuffer.clone();let oam=mem.oam.clone();
        mem.ffta_dialogue.separate=true;
        render_scanline(&mut mem,0);render_scanline(&mut mem,88);
        assert!(mem.ffta_dialogue.placement.is_none());
        assert!(!mem.ffta_dialogue.extra.active());
        assert!(mem.ffta_dialogue.suspended_portrait);
        assert_eq!(mem.wide_framebuffer[88*568+28],0);
        assert_eq!(mem.wide_framebuffer[88*568+244],31);
        assert_eq!(mem.framebuffer,native);
        assert_eq!(mem.oam,oam);
        if controls[2]==0x0b08 {
            assert!(!detached_action_portrait(&mem,[0xa058,0xd178,0x240],-136,88),
                "real priority-zero dialogue portraits must survive");
        }
        // No residual portrait: no additional world pass is requested.
        mem.ffta_sprites.active.as_mut().unwrap().remove(0);
        render_scanline(&mut mem,0);
        assert!(!mem.ffta_dialogue.suspended_portrait);
        }
        crate::mem::set_extra(0);
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
    render_sprites_extent_impl::<true>(mem,dispcnt,line,out,window,ex,rw)
}

fn render_sprites_extent_impl<const EARLY_WORLD_CULL:bool>(
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
    // Party parks gallery actors beyond the native LCD during child-menu
    // slides. Widescreen must not reveal those transition-only OBJ pieces.
    // Gate on the owning task, not its current substate or mouse setting.
    let party_canvas = mem.ffta_movement.europe_profile
        && u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap()) == 0x08072929;

    let host = if !mem.ffta_dialogue.capture && (ex > 0 || crate::mem::extra_y() > 0) {
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
        if x >= (if mem.ffta_dialogue.capture {256}else{VISIBLE_WIDTH+ex}) as i32 {
            x -= 512;
        }
        if let Some((host_x, host_y)) = coordinates {
            x = host_x;
            y = host_y;
        }
        // Host positions use full integers; OAM stores wrapped 9/8-bit values.
        // During a slide both represent the same UI piece. Classification and
        // extraction must use the same native source coordinates, or the
        // unrecognized copy leaks back into the world at a different edge.
        if mem.ffta_dialogue.separate {
            let stats=(mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats
                || mem.ffta_dialogue.battle_stats.kind==crate::ffta_dialogue::PanelKind::Stats)
                && a0&0x2000==0 && matches!(a2>>12,11|12) && (0x1c8..0x240).contains(&(a2&1023));
            let portrait=mem.ffta_dialogue.action_layout && a0&0x2000!=0
                && a2&0xc00<=0x400 && (0x200..0x400).contains(&(a2&1023));
            if stats || portrait {
                x=(x+256).rem_euclid(512)-256;
                y=y.rem_euclid(256);if y>=160{y-=256;}
            }
        }
        // Detached UI is removed from the world, not relocated inside it.
        // Its surviving world sprites therefore sample this exact scanline,
        // including zoom margins. Reject absent rows before classifying UI.
        // The legacy integrated-UI path still needs its relocated source row.
        // Captured stats use their canonical panel Y, as before.
        if mem.ffta_dialogue.capture || !mem.ffta_dialogue.wide_pass || EARLY_WORLD_CULL && mem.ffta_dialogue.separate {
            let sample_y=if mem.ffta_dialogue.capture
                && mem.ffta_dialogue.extra.contains_stats([a0,a1,a2],x,y,bw,bh) {
                mem.ffta_dialogue.extra.bounds.zip(mem.ffta_dialogue.stats_target)
                    .map_or(y,|(src,dst)|y+dst[1]-src[1])
            }else{y};
            if line<sample_y || line>=sample_y+bh {continue;}
        }
        if crate::ffta_battle_hud::owns(mem,[a0,a1,a2]) {continue;}
        let bounded=(-64..240).contains(&x) && (-64..160).contains(&y);
        let marker=crate::ffta_dialogue::list_marker([a0,a1,a2]);
        if !mem.ffta_dialogue.naming_layout && crate::ffta_battle_mouse::hide_mouse_indicator(mem,[a0,a1,a2]) {continue;}
        if mem.ffta_dialogue.shop_numbers_exiting
            && (mem.ffta_dialogue.capture || mem.ffta_dialogue.wide_pass)
            && crate::ffta_dialogue::shop_attribute_digit([a0,a1,a2]) {continue;}

        if marker && (a2&1023)<=0x127 && !crate::ffta_dialogue::continuation([a0,a1,a2])
            && crate::ffta_battle_mouse::pub_scroll_unselected(mem) {
            continue;
        }
        let marker_extra=marker && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Actions
            && mem.ffta_dialogue.extra.bounds.is_some_and(|[l,t,r,b]|x>=l-16 && x<r+16 && y>=t-16 && y<b+16);
        let counter=mem.ffta_dialogue.service_layout && crate::ffta_dialogue::service_counter([a0,a1,a2],x,y);
        let funds_object=counter && mem.ffta_dialogue.funds_bounds.is_some_and(|[l,t,r,b]|
            x>=l && x<r && y>=t && y<b);
        let secondary_stats=mem.ffta_dialogue.battle_stats.contains_stats([a0,a1,a2],x,y,bw,bh);
        let stats_piece=mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats
            && mem.ffta_dialogue.extra.contains_stats([a0,a1,a2],x,y,bw,bh);
        if stats_piece && mem.ffta_dialogue.capture {
            if let (Some(src),Some(dst))=(mem.ffta_dialogue.extra.bounds,mem.ffta_dialogue.stats_target) {
                x+=dst[0]-src[0];y+=dst[1]-src[1];
            }
        }
        let bounded=bounded || stats_piece;
        let notice_object=mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice
            && (crate::ffta_dialogue::notice_cursor([a0,a1,a2],x,y,mem.ffta_dialogue.extra.bounds)
                || crate::ffta_dialogue::continuation([a0,a1,a2]));
        let detached_cursor=mem.ffta_dialogue.town_layout && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Actions
            && crate::ffta_dialogue::notice_cursor([a0,a1,a2],x,y,mem.ffta_dialogue.extra.bounds);
        let split_cursor=mem.ffta_dialogue.split_speech && a1&0xfe00==0 && ((a0&0xff00==0x4000 && (0x120..=0x123).contains(&(a2&1023))) || (a0&0xff00==0 && (0x124..=0x127).contains(&(a2&1023))));
        let speech_object=mem.ffta_dialogue.service_speech.active()
            && (crate::ffta_dialogue::portrait([a0,a1,a2],x) || crate::ffta_dialogue::continuation([a0,a1,a2]));
        let bank_preview=mem.ffta_dialogue.service_list
            && crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==66)
            && crate::ffta_dialogue::bank_preview([a0,a1,a2]);
        let main_object=if mem.ffta_dialogue.naming_layout {a2&0xc00<=0x400 && (0x120..0x400).contains(&(a2&1023))}else{mem.ffta_dialogue.announcement_bounds.is_none() && (bank_preview || bounded && !speech_object && !funds_object && !notice_object && !detached_cursor && !split_cursor && !marker_extra && if mem.ffta_dialogue.action_layout {
            crate::ffta_dialogue::action_portrait([a0,a1,a2],x,y) || marker
        } else if mem.ffta_dialogue.town_layout || mem.ffta_dialogue.shop_layout {
            counter || (mem.ffta_dialogue.shop_layout && crate::ffta_dialogue::service_object([a0,a1,a2]))
                || marker || crate::ffta_dialogue::portrait([a0,a1,a2],x)
                || crate::ffta_dialogue::continuation([a0,a1,a2])
                || (mem.ffta_dialogue.shop_layout && a2&0x0c00==0 && (0x120..=0x12f).contains(&(a2&1023)))
                || (a1&0xfe00==0 && ((a0&0xff00==0x4000 && (0x120..=0x123).contains(&(a2&1023)))
                || (a0&0xff00==0 && (0x124..=0x127).contains(&(a2&1023)))))
        } else {!mem.ffta_dialogue.attack_name && (crate::ffta_dialogue::portrait([a0,a1,a2],x) || crate::ffta_dialogue::continuation([a0,a1,a2]))})};
        let extra_object=bounded && match mem.ffta_dialogue.extra.kind {
            crate::ffta_dialogue::PanelKind::Speech=>crate::ffta_dialogue::portrait([a0,a1,a2],x)
                || crate::ffta_dialogue::continuation([a0,a1,a2]),
            crate::ffta_dialogue::PanelKind::Notice=>notice_object,
            crate::ffta_dialogue::PanelKind::Stats=>stats_piece,
            crate::ffta_dialogue::PanelKind::Actions=>marker_extra || if mem.ffta_dialogue.town_layout {
                a1&0xfe00==0 && ((a0&0xff00==0x4000 && (0x120..=0x123).contains(&(a2&1023)))
                    || (a0&0xff00==0 && (0x124..=0x127).contains(&(a2&1023))))
            }else{crate::ffta_dialogue::notice_cursor([a0,a1,a2],x,y,mem.ffta_dialogue.extra.bounds.map(|[l,t,r,b]|[l-8,t,r,b]))},
            _=>false,
        };
        if mem.ffta_dialogue.capture && !(if mem.ffta_dialogue.capture_funds {funds_object}else if mem.ffta_dialogue.capture_extra {extra_object}else{main_object}) {continue;}
        if mem.ffta_dialogue.capture && mem.ffta_dialogue.capture_backing
            && !(mem.ffta_dialogue.capture_extra && mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Stats && a2>>12==11 && matches!(a2&1023,0x1f0|0x200|0x208)) {continue;}
        let suspended_portrait=mem.ffta_dialogue.separate && mem.ffta_dialogue.wide_pass
            && (detached_action_portrait(mem,[a0,a1,a2],x,y)
                || io16(mem,12)==0x5c08 && io16(mem,14)==0x4a09 && ffta_terrain(mem,0)
                    && crate::ffta_dialogue::list_marker([a0,a1,a2]));
        if suspended_portrait {continue;}
        // These pieces may be published beyond OAM while the native panel
        // enters/exits. Remove them before the legacy full-width gap fill,
        // independently of a temporarily missing/partial stats rectangle.
        if mem.ffta_dialogue.wide_pass && mem.ffta_dialogue.suspended_stats
            && stats_object([a0,a1,a2]) {continue;}
        if mem.ffta_dialogue.separate && mem.ffta_dialogue.wide_pass && ((main_object || funds_object) && mem.ffta_dialogue.placement.is_some() || extra_object || secondary_stats || speech_object) {continue;}
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
        let mut cached_asset_row:Option<(usize,usize,[u16;8])>=None;
        for sample in start..end {
            let (bx,sx) = if let Some(p) = dialogue {
                (p.source_x(sample)-x,sample-ex as i32)
            } else {(sample,x+sample)};
            // Buffer index = screen column + EXTRA (margins included).
            let idx = sx + ex as i32;
            if !(0..rw as i32).contains(&idx) {
                continue;
            }
            if party_canvas && !(0..VISIBLE_WIDTH as i32).contains(&sx) {
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
            let color=if crate::asset_cache::enabled() {
                let base=0x10000+if eight_bpp {(tile/2)*64}else{tile*32};let y=pv as usize;
                if cached_asset_row.as_ref().is_none_or(|r|r.0!=base || r.1!=y) {
                    let bank=256+if eight_bpp {0}else{palette*16};
                    cached_asset_row=Some((base,y,crate::asset_cache::row(&mem.vram,&mem.palette,base,eight_bpp,bank,y)));
                }
                let color=cached_asset_row.as_ref().unwrap().2[pu as usize];
                if color==TRANSPARENT {continue;} color
            } else {
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
            pal16(mem, 256 + color_idx)
            };

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
    let mut coordinates=[0u32;MAX_RENDER_WIDTH];
    let capture=mem.ffta_surface.capture();
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
        capture.then_some(&mut coordinates[..rw]),
    );
    let zoom = mem.ffta_movement.zoom_output;
    let row = if zoom {
        (line as isize + crate::mem::extra_y() as isize) as usize
    } else {
        line
    };
    if capture {mem.ffta_surface.coordinates[row*rw..(row+1)*rw].copy_from_slice(&coordinates[..rw]);}
    if mem.ffta_surface.capture(){for (x,pixel) in output[..rw].iter_mut().enumerate(){
        mem.ffta_surface.mask[row*rw+x]=if *pixel&0x8000!=0{255}else{0};*pixel&=0x7fff;
    }}
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
// A spare scratch bit carries original PPU layer ownership to the host mask.
// It is stripped before either guest framebuffer is published.
fn stage_surface_tag(mem:&MemMap,layer:usize)->u16{
    if !mem.ffta_surface.capture(){return 0;}
    let receives=layer==4 || (layer<4 && matches!(io16(mem,8+layer*2),0x4c03|0x4e02));
    if receives {0x8000}else{0}
}
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
    mut coordinates:Option<&mut [u32]>,
) {
    if let Some(c)=coordinates.as_deref_mut(){c.fill(0);}
    let rw = render_width();
    let ex = extra();
    let native_only=!mem.ffta_movement.zoom_output && !mem.ffta_dialogue.wide_pass
        && !mem.ffta_dialogue.reuse_world_row
        && mem.ffta_dialogue.separate
        && (mem.ffta_dialogue.placement.is_some() || mem.ffta_dialogue.extra.active());
    let visible=if native_only {ex..ex+VISIBLE_WIDTH}else{0..rw};
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
    // The Bank list's hardware window clips its native preview/UI, not the
    // detached world canvas. Keep native composition and UI capture intact.
    let bank_world=mem.ffta_dialogue.separate && mem.ffta_dialogue.wide_pass
        && mem.ffta_dialogue.service_layout
        && crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==66);
    let any_window = (win0_on || win1_on || objwin_on) && !bank_world;

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

    // Brightness/darkness only transform the winning color. A lower layer
    // is needed exclusively for alpha blending (including semi-transparent
    // OBJ). Keep window effects and per-layer targets in the top-only path.
    let alpha_possible = blend_mode == 1 && bldcnt & 0x3f != 0 && bldcnt & 0x3f00 != 0;
    if !alpha_possible && !line_has_semi
        && (blend_mode==0 || !obj_line[..rw].iter().any(|o|o.priority!=0xff && o.semi)) {
        for x in visible.clone() {
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
            let mut top_layer=5usize;
            for &(bp, bg) in bg_prio {
                if control & (1 << bg) != 0 {
                    let c = bg_lines[bg][x];
                    if c != TRANSPARENT {
                        color = c;
                        top_prio = bp;
                        top_layer=bg;
                        break;
                    }
                }
            }
            let obj = &obj_line[x];
            if obj.priority != 0xFF && control & 0x10 != 0 && obj.priority <= top_prio {
                color = obj.color;
                top_layer=4;
                if obj.owner != 0 && trace {
                    *counts.entry(obj.owner).or_default() += 1;
                }
            }
            if control & 0x20 != 0 && bldcnt & (1 << top_layer) != 0 {
                color = match blend_mode {
                    2 => brighten(color, evy),
                    3 => darken(color, evy),
                    _ => color,
                };
            }
            output[x] = color | stage_surface_tag(mem,top_layer);
            if let Some(c)=coordinates.as_deref_mut(){c[x]=crate::ffta_surface::coordinate(mem,top_layer,x,line);}
        }
        return;
    }

    for x in visible.clone() {
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

        output[x] = color | stage_surface_tag(mem,fl);
        if let Some(c)=coordinates.as_deref_mut(){c[x]=crate::ffta_surface::coordinate(mem,fl,x,line);}
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
    // The Bank list's hardware window clips its native preview/UI, not the
    // detached world canvas. Keep native composition and UI capture intact.
    let bank_world=mem.ffta_dialogue.separate && mem.ffta_dialogue.wide_pass
        && mem.ffta_dialogue.service_layout
        && crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==66);
    let any_window = (win0_on || win1_on || objwin_on) && !bank_world;

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
    fn party_transition_sprites_stay_inside_native_canvas() {
        let mut mem = MemMap::new(vec![]);
        mem.ffta_movement.europe_profile = true;
        mem.iwram[0x2804..0x2808].copy_from_slice(&0x08072929u32.to_le_bytes());
        mem.write16(0x05000202, 0x03e0);
        mem.vram[0x10020..0x10040].fill(0x11);
        for slot in 0..128 {mem.write16(0x07000000 + slot * 8, 0x200);}
        for (slot,x) in [(0,244u16),(1,508),(2,24)] {
            mem.write16(0x07000000 + slot * 8, 20);
            mem.write16(0x07000002 + slot * 8, x);
            mem.write16(0x07000004 + slot * 8, 1);
        }
        let ex=32;
        let rw=240+ex*2;
        for phase in [3,9,12] {
            mem.iwram[0x1000]=phase;
            let mut out=[OBJ_EMPTY;MAX_RENDER_WIDTH];
            let mut windows=[false;MAX_RENDER_WIDTH];
            render_sprites_extent(&mem,0x1000,22,&mut out,&mut windows,ex,rw);
            assert!(out[..ex].iter().all(|p|p.color==TRANSPARENT));
            assert!(out[ex+240..rw].iter().all(|p|p.color==TRANSPARENT));
            assert_eq!(out[ex].color,0x03e0); // crossing left edge remains visible inside
            assert_eq!(out[ex+24].color,0x03e0);
        }
        // Ordinary worlds retain their extended sprite area.
        mem.iwram[0x2804..0x2808].fill(0);
        let mut out=[OBJ_EMPTY;MAX_RENDER_WIDTH];
        let mut windows=[false;MAX_RENDER_WIDTH];
        render_sprites_extent(&mem,0x1000,22,&mut out,&mut windows,ex,rw);
        assert_eq!(out[ex+244].color,0x03e0);
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
        mem.ffta_surface.enabled=true;
        mem.ffta_surface.begin(Some(crate::ffta_surface::Descriptor{id:0,terrain_bg:2,scroll:[0,0],source_origin:None}),(render_width(),crate::mem::render_height()),0);
        let original = mem.wide_framebuffer.clone();
        for blend in [0u16, 0x3f41, 0x3fff, 0x3f9f] {
            mem.io[0x50..0x52].copy_from_slice(&blend.to_le_bytes());
            mem.io[0x52..0x54].copy_from_slice(&0x0808u16.to_le_bytes());
            mem.io[0x54..0x56].copy_from_slice(&7u16.to_le_bytes());
            mem.wide_framebuffer.copy_from_slice(&original);
            mem.ffta_surface.mask.fill(0);mem.ffta_surface.coordinates.fill(0);
            set_parallel_zoom(false);
            render_ffta_zoom_rows(&mut mem);
            let expected = mem.wide_framebuffer.clone();
            let expected_mask=mem.ffta_surface.mask.clone();
            let expected_coordinates=mem.ffta_surface.coordinates.clone();
            mem.ffta_surface.mask.fill(0);mem.ffta_surface.coordinates.fill(0);
            let hardware = mem.framebuffer.clone();
            let oam = mem.oam.clone();
            mem.wide_framebuffer.copy_from_slice(&original);
            set_parallel_zoom(true);
            render_ffta_zoom_rows(&mut mem);
            assert_eq!(mem.wide_framebuffer, expected, "blend {blend:04x}");
            assert_eq!(mem.ffta_surface.mask,expected_mask,"coverage blend {blend:04x}");
            assert!(mem.ffta_surface.coordinates==expected_coordinates,"source plane coordinates differ for blend {blend:04x}");
            assert!(mem.wide_framebuffer[..render_width()*crate::mem::render_height()].iter().all(|c|c&0x8000==0));
            assert_eq!(mem.framebuffer, hardware);
            assert_eq!(mem.oam, oam);
        }
        set_parallel_zoom(cfg!(target_os = "android"));
        crate::mem::set_extra(0);
        crate::mem::set_extra_y(0);
    }
}


#[cfg(test)]
mod stage_surface_tests {
    use super::*;
    #[test]
    fn ownership_sidecar_preserves_600_frame_rgb_and_oam() {
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
        let mut m=MemMap::new(vec![]);
        m.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        m.io[10..12].copy_from_slice(&0x0b08u16.to_le_bytes());
        let stage=crate::ffta_surface::Descriptor{id:0,terrain_bg:0,scroll:[0,0],source_origin:None};
        let mut bg=[[TRANSPARENT;MAX_RENDER_WIDTH];4];
        bg[0][0]=0x1234;bg[1][1]=0x4321;
        let mut obj=[OBJ_EMPTY;MAX_RENDER_WIDTH];obj[2]=ObjPixel{owner:1,color:0x1f,priority:0,semi:false};
        let before_oam=m.oam.clone();
        let mut hashes=[0xcbf29ce484222325u64;2];
        for frame in 0..600 {
            // Alternate normal composition and brightness path.
            m.io[0x50..0x52].copy_from_slice(&(if frame%2==0{0u16}else{0x00bf}).to_le_bytes());
            for on in 0..2 {
                m.ffta_surface.enabled=on==1;
                m.ffta_surface.begin(Some(stage),(render_width(),160),frame);
                compose(&mut m,0x1300,0,&bg,&[true,true,false,false],&obj,&[false;MAX_RENDER_WIDTH],false);
                for pixel in &m.framebuffer[..240]{hashes[on]=(hashes[on]^*pixel as u64).wrapping_mul(0x100000001b3);}
                assert!(m.framebuffer[..240].iter().all(|v|v&0x8000==0));
                if on==1 {assert_eq!(&m.ffta_surface.mask[..4],&[255,0,255,0]);}
            }
        }
        assert_eq!(hashes[0],hashes[1]);assert_eq!(m.oam,before_oam);
        eprintln!("STAGE_SURFACE frames=600 framebuffer_off={:016x} framebuffer_on={:016x} oam_identical=true",hashes[0],hashes[1]);
    }
}



fn reusable_native_ui_row(mem:&MemMap,dispcnt:u16,bg:usize,line:usize)->bool {
    if dispcnt&7!=0 || mem.ffta_dialogue.wide_pass || mem.ffta_movement.zoom_output{return false;}
    if extra()==0{return true;}
    if crate::ffta_stage::layer(mem,bg).is_some() || ffta_movement_grid(mem,bg)
        || ffta_terrain(mem,bg) || ffta_world_map(mem,bg){return false;}
    let cnt=io16(mem,8+bg*2);
    let width=if cnt&0x4000!=0{512}else{256};
    let height=if cnt&0x8000!=0{512}else{256};
    (io16(mem,0x10+bg*4)&511) as usize+240<=width
        && (io16(mem,0x12+bg*4)&511) as usize+line<height
}

fn ui_backgrounds_hidden(mem:&MemMap,dispcnt:u16,line:usize,used:&[bool;4],window:&[bool;MAX_RENDER_WIDTH])->bool {
    let mask=(if used[2]{4}else{0})|(if used[3]{8}else{0});
    if mask==0{return true;}
    if dispcnt&0xe000==0{return false;}
    let win0v=io16(mem,0x44);let win1v=io16(mem,0x46);
    let row0=dispcnt&0x2000!=0 && in_window_range((win0v>>8)as u8,win0v as u8,line as u8);
    let row1=dispcnt&0x4000!=0 && in_window_range((win1v>>8)as u8,win1v as u8,line as u8);
    let outside=io16(mem,0x4a);
    if !row0 && !row1 && dispcnt&0x8000==0{return outside&mask==0;}
    let h0=io16(mem,0x40);let h1=io16(mem,0x42);let inside=io16(mem,0x48);
    (0..240).all(|x|{
        let control=if row0&&in_window_range((h0>>8)as u8,h0 as u8,x as u8){inside&63}
            else if row1&&in_window_range((h1>>8)as u8,h1 as u8,x as u8){(inside>>8)&63}
            else if dispcnt&0x8000!=0&&window[extra()+x]{(outside>>8)&63}else{outside&63};
        control&mask==0
    })
}

#[cfg(test)]mod mobile_row_reuse_tests {
    use super::*;
    #[test]
    fn reusable_native_rows_match_hardware_for_scroll_sizes_depths_and_zoom(){
        let mut m=MemMap::new(vec![0;192]);let mut seed=19u32;
        for byte in m.vram.iter_mut().chain(m.palette.iter_mut()) {
            seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);*byte=(seed>>16)as u8;
        }
        for ex in [0,42,62,136] {crate::mem::set_extra(ex);
            for bg in [2,3] {for size in 0..4u16 {for bpp in [0,0x80] {
                m.io[8+2*bg..10+2*bg].copy_from_slice(&(0x1004|bpp|(size<<14)).to_le_bytes());
                for x in [0,5,16,255,511u16] {for y in [0,13,255,511u16] {
                    m.io[0x10+4*bg..0x12+4*bg].copy_from_slice(&x.to_le_bytes());
                    m.io[0x12+4*bg..0x14+4*bg].copy_from_slice(&y.to_le_bytes());
                    for line in [0,7,159] {
                        if !reusable_native_ui_row(&m,0x0c00,bg,line){continue;}
                        let mut native=[TRANSPARENT;MAX_RENDER_WIDTH];let mut world=native;
                        render_text_bg_hardware(&m,bg,line,&mut native,0,240);
                        render_text_bg_extent(&m,bg,line,&mut world,ex,240+2*ex);
                        assert_eq!(&native[..240],&world[ex..ex+240],"bg={bg} ex={ex} size={size} bpp={bpp} x={x} y={y} line={line}");
                    }
                }}
            }}}
        }
        assert!(!reusable_native_ui_row(&m,1,2,0));
        m.ffta_movement.zoom_output=true;assert!(!reusable_native_ui_row(&m,0,2,0));
        m.ffta_movement.zoom_output=false;m.ffta_dialogue.wide_pass=true;
        assert!(!reusable_native_ui_row(&m,0,2,0));
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
}

#[cfg(test)]mod name_entry_layout_tests {
    use super::*;
    #[test]
    fn naming_keys_stay_on_one_centered_canvas_instead_of_an_attack_capsule(){
        crate::mem::set_extra(136);crate::mem::set_extra_y(176);
        let mut m=MemMap::new(vec![0;0x400000]);
        m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;m.ffta_dialogue.separate=true;
        m.rom[0x12e81a..0x12e81e].copy_from_slice(&[1,0x30,0x20,0x70]);
        m.rom[0x12e6a0..0x12e6a4].copy_from_slice(&[0xfe,0xd2,0x3a,8]);
        m.iwram[0x27d1]=2;m.ewram[0x166db]=3;
        for(o,v)in[(0,0x1f40u16),(8,0x4c03),(10,0x4e02),(12,0x0b08),(14,0x0a08)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        m.oam[..2].copy_from_slice(&0u16.to_le_bytes());m.oam[4..6].copy_from_slice(&0x120u16.to_le_bytes());
        m.palette[2..4].copy_from_slice(&31u16.to_le_bytes());m.vram[0x8020..0x8040].fill(0x11);
        // The selected tab and name-field frame are priority-one OBJ;
        // field actors use priority two/three and must stay in the world.
        for(o,v)in[(8,0x0080u16),(10,0x8050),(12,0x56d0)]{m.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.vram[0x10000+0x2d0*32..0x10020+0x2d0*32].fill(0x22);
        let palette=(256+5*16+2)*2;m.palette[palette..palette+2].copy_from_slice(&0x7c00u16.to_le_bytes());
        let cell=0x5800+(6*32+8)*2;m.vram[cell..cell+2].copy_from_slice(&1u16.to_le_bytes());
        for row in 0..160{render_scanline(&mut m,row);}
        assert!(m.ffta_dialogue.naming_layout);assert!(!m.ffta_dialogue.attack_name);
        let node=crate::ffta_ui::Scene::capture(&m.ffta_dialogue).nodes[0].unwrap();
        assert_eq!(node.anchor,crate::ffta_ui::Anchor::Center);assert_eq!(node.bounds,[0,0,240,160]);
        assert_eq!(m.ffta_dialogue.pixels[48*480+120+64].color,31);
        assert_eq!(m.ffta_dialogue.pixels[128*480+120+80].color,0x7c00);
        // When the verified name task ends, the special owner must disappear.
        m.iwram[0x27d1]=0;begin_dialogue_frame(&mut m);assert!(!m.ffta_dialogue.naming_layout);
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
}

#[cfg(test)]mod frontend_padding_tests {
    use super::*;
    #[test]fn language_padding_uses_only_clean_rows_and_never_tiles_choices(){
        crate::mem::set_extra(22);crate::mem::set_extra_y(80);
        let mut m=MemMap::new(vec![]);
        m.io[..2].copy_from_slice(&0x1140u16.to_le_bytes());
        m.io[8..10].copy_from_slice(&0x3000u16.to_le_bytes());
        m.vram[..32].fill(0x11);m.vram[32..64].fill(0x22);
        m.palette[2..4].copy_from_slice(&992u16.to_le_bytes());
        m.palette[4..6].copy_from_slice(&31u16.to_le_bytes());
        // Language text region uses a red tile; clean margins are green.
        for y in 3..17 {for x in 0..32 {
            let o=0x8000+(y*32+x)*2;
            m.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
        }}
        let mut out=vec![0;284];
        for y in [-320,-81,-1,160,239,479] {
            assert!(render_frontend_background_row(&m,y,&mut out));
            assert!(out.iter().all(|&p|p==992));
        }
        m.io[..2].fill(0);assert!(!render_frontend_background_row(&m,-1,&mut out));
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
}

#[cfg(test)]
mod detached_sprite_cull_tests {
    use super::*;
    #[test]
    fn detached_cull_matches_previous_scan_for_zoom_and_ui_modes() {
        crate::mem::set_extra(136);crate::mem::set_extra_y(176);
        let mut mem=MemMap::new(vec![]);
        let mut seed=9713u32;
        let mut next=||{seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;seed};
        for region in [&mut mem.vram[..],&mut mem.palette[..],&mut mem.oam[..]] {
            for byte in region {*byte=next()as u8;}
        }
        mem.ffta_dialogue.separate=true;mem.ffta_dialogue.wide_pass=true;
        mem.ffta_dialogue.placement=Some(crate::ffta_dialogue::Placement::new(true,512,136,Default::default()));
        mem.ffta_dialogue.extra.bounds=Some([0,96,120,160]);
        mem.ffta_dialogue.stats_target=Some([0,96,120,160]);
        mem.ffta_sprites.active=Some((0..24).map(|i|crate::ffta_sprites::HostSprite{
            attrs:[(i*13)as u16,((i*17)&511)as u16,0x200+(i*3)as u16],
            slot:i*3,x:i as i32*19-120,y:i as i32*23-180,replaces:i%2==0,
            ..Default::default()
        }).collect());
        use crate::ffta_dialogue::PanelKind;
        for mode in 0..8 {
            mem.ffta_dialogue.action_layout=mode&1!=0;
            mem.ffta_dialogue.naming_layout=mode&2!=0;
            mem.ffta_dialogue.extra.kind=if mode&4!=0 {PanelKind::Stats}else{PanelKind::Actions};
            for line in (-256i32..512).step_by(7) {
                let mut fast=[OBJ_EMPTY;MAX_RENDER_WIDTH];let mut old=fast;
                let mut fw=[false;MAX_RENDER_WIDTH];let mut ow=fw;
                let fs=render_sprites_extent_impl::<true>(&mem,0x1f40,line as usize,&mut fast,&mut fw,136,512);
                let os=render_sprites_extent_impl::<false>(&mem,0x1f40,line as usize,&mut old,&mut ow,136,512);
                if !zoom_sprite_y_bounds(&mem).is_some_and(|(lo,hi)|line>=lo && line<hi) {
                    assert!(!os && ow.iter().all(|&p|!p) && old.iter().all(|p|p.priority==0xff),"envelope mode={mode} row={line}");
                }
                assert_eq!(fs,os,"semi mode={mode} row={line}");
                assert_eq!(fw,ow,"window mode={mode} row={line}");
                for (x,(a,b)) in fast.iter().zip(&old).enumerate() {
                    assert_eq!((a.color,a.priority,a.semi,a.owner),(b.color,b.priority,b.semi,b.owner),"mode={mode} row={line} x={x}");
                }
            }
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
}

#[cfg(test)]mod fullscreen_menu_background_tests {
    use super::*;
    #[test]fn clan_top_and_bottom_extensions_reflect_the_native_border_without_phase_jump(){
        crate::mem::set_extra(8);crate::mem::set_extra_y(48);
        let mut mem=MemMap::new(vec![]);
        mem.ffta_dialogue.menu_pattern=true;
        mem.io[8..10].copy_from_slice(&0x8a01u16.to_le_bytes());
        for tile in 0..600usize {
            let bank=(tile/30)%15+1;
            mem.vram[tile*32..tile*32+32].fill((bank as u8)*17);
            let y=tile/30;let x=tile%30;let at=0x5000+(y*32+x)*2;
            mem.vram[at..at+2].copy_from_slice(&(tile as u16).to_le_bytes());
        }
        for i in 1..16 {mem.palette[i*2..i*2+2].copy_from_slice(&(i as u16*97).to_le_bytes());}
        for y in [-48i32,-1,160,207] {
            let p=y.rem_euclid(320);let native_y=if p<160{p}else{319-p};
            let mut expected=vec![0;256];let mut actual=expected.clone();
            render_menu_background_row(&mem,native_y,&mut expected);
            assert!(render_frontend_background_row(&mem,y,&mut actual));
            assert_eq!(actual,expected,"border row {y}");
        }
        crate::mem::set_extra(0);crate::mem::set_extra_y(0);
    }
}
