//! PC-gated original interpreter execution; host expansion is independent of guest OAM.
use std::collections::BTreeMap;
pub fn stats_panel_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FFTA_PATCH_STATS").as_deref() != Ok("0"))
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct HostSprite {
    pub owner: u32,
    pub group: u64,
    pub x: i32,
    pub y: i32,
    pub attrs: [u16; 3],
    /// Marks the verified unit-hover backing for host-only gap completion.
    pub stats_background: bool,
    /// EUR stats names sit a few pixels inside the unit panel's inner cap.
    pub stats_inner_padding: bool,
    /// Position in original OAM ordering; unlimited objects may share a position.
    pub slot: usize,
    pub replaces: bool,
}

#[derive(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SpriteState {
    pub enabled: bool,
    rom_valid: bool,
    pub profile: Option<crate::ffta_rom::SpriteProfile>,
    observed_pending: BTreeMap<u32, Vec<HostSprite>>,
    pub oam_owners: Vec<u32>,
    pub pixel_counts: BTreeMap<u32, u32>,
    vis_enabled: bool,
    vis_verified: bool,
    visibility_calls: BTreeMap<(u32, u32), (u32, i32, i32, u32, u32)>,
    pub calls: u64,
    ram_verified: bool,
    pending: BTreeMap<u32, (u64, Vec<HostSprite>)>,
    pub active: Option<Vec<HostSprite>>,
    segments: BTreeMap<usize, (usize, Vec<HostSprite>)>,
    pub publications: u64,
    decoded: u64,
    failed_buffer: Option<u32>,
    no_input: u64,
    mismatches: u64,
    mismatch: Option<(usize, [u16; 3], [u16; 3])>,
    observed_instances: std::collections::BTreeSet<u32>,
    observed_tiles: BTreeMap<u32, Vec<usize>>,
}
impl SpriteState {
    pub fn new(rom: &[u8]) -> Self {
        let profile = crate::ffta_rom::sprite_profile(rom);
        let valid = profile.is_some();
        Self {
            enabled: std::env::var("FFTA_PATCH_SPR").as_deref() != Ok("0") && valid,
            rom_valid: valid,
            profile,
            oam_owners: vec![0; 128],
            vis_enabled: std::env::var("FFTA_PATCH_VIS").as_deref() != Ok("0") && valid,
            ..Self::default()
        }
    }
    pub fn publish(&mut self, src: u32, dst: u32, bytes: usize, wide: bool, oam: &[u8]) {
        if crate::ffta_log::enabled() && (0x07000000..0x07000400).contains(&dst) && dst & 7 == 0 {
            let offset = (dst - 0x07000000) as usize;
            self.oam_owners.resize(128, 0);
            if offset == 0 {
                self.oam_owners.fill(0);
            }
            for slot in offset / 8..((offset + bytes) / 8).min(128) {
                self.oam_owners[slot] = 0;
            }
            if let Some(objects) = self.observed_pending.get(&src) {
                for object in objects
                    .iter()
                    .filter(|s| s.replaces && s.slot * 8 + 6 <= bytes)
                {
                    let slot = offset / 8 + object.slot;
                    if slot >= 128 {
                        continue;
                    }
                    let p = slot * 8;
                    if object
                        .attrs
                        .iter()
                        .enumerate()
                        .all(|(i, a)| oam[p + i * 2..p + i * 2 + 2] == a.to_le_bytes())
                    {
                        self.oam_owners[slot] = object.owner;
                    }
                }
            }
        }
        if !self.enabled || !wide || !(0x07000000..0x07000400).contains(&dst) || dst & 7 != 0 {
            return;
        }
        let offset = (dst - 0x07000000) as usize;
        if offset == 0 {
            self.segments.clear();
        }
        // OAM is submitted in pieces: battle cursors first, characters second.
        self.segments
            .retain(|start, (length, _)| *start + *length <= offset || *start >= offset + bytes);
        let incoming = if bytes >= 8 {
            if let Some((_, sprites)) = self.pending.get(&src) {
                let matches = sprites.iter().filter(|s| s.replaces).all(|s| {
                    s.slot * 8 + 6 <= bytes
                        && oam
                            .get(offset + s.slot * 8..offset + s.slot * 8 + 6)
                            .is_some_and(|actual| {
                                s.attrs
                                    .iter()
                                    .enumerate()
                                    .all(|(i, a)| actual[i * 2..i * 2 + 2] == a.to_le_bytes())
                            })
                });
                if !matches {
                    self.mismatches += 1;
                    for s in sprites.iter().filter(|s| s.replaces) {
                        let p = offset + s.slot * 8;
                        if p + 6 > oam.len() {
                            break;
                        }
                        let actual = [
                            u16::from_le_bytes([oam[p], oam[p + 1]]),
                            u16::from_le_bytes([oam[p + 2], oam[p + 3]]),
                            u16::from_le_bytes([oam[p + 4], oam[p + 5]]),
                        ];
                        if actual != s.attrs {
                            self.mismatch = Some((s.slot, s.attrs, actual));
                            break;
                        }
                    }
                }
                matches.then(|| sprites.clone())
            } else {
                self.no_input += 1;
                None
            }
        } else {
            None
        };
        if let Some(sprites) = incoming {
            self.segments.insert(offset, (bytes, sprites));
        }
        if self.segments.is_empty() {
            self.active = None;
        } else {
            let active = self.active.get_or_insert_with(Vec::new);
            active.clear();
            for (start, (_, sprites)) in &self.segments {
                active.extend(sprites.iter().cloned().map(|mut sprite| {
                    sprite.slot += start / 8;
                    sprite
                }));
            }
            active.sort_by_key(|s| s.slot);
        }
        if self.active.is_some() {
            self.publications += 1;
        }
        // Retain a buffer until rebuilt: unchanged OAM can be submitted repeatedly.
    }
}

fn peek(mem: &crate::mem::MemMap, addr: u32, len: usize) -> Option<&[u8]> {
    let (data, offset) = match addr >> 24 {
        2 => (&mem.ewram, (addr & 0x3ffff) as usize),
        3 => (&mem.iwram, (addr & 0x7fff) as usize),
        8 | 9 => (&mem.rom, (addr & 0x1ffffff) as usize),
        _ => return None,
    };
    data.get(offset..offset.checked_add(len)?)
}
fn dimension_values(mem: &crate::mem::MemMap, addr: u32) -> Option<(u16, u16)> {
    if let Some(values) = mem.obj_dimensions(addr) { return Some(values); }
    let size = peek(mem, addr, 4)?;
    Some((u16::from_le_bytes([size[0], size[1]]), u16::from_le_bytes([size[2], size[3]])))
}

fn word(mem: &crate::mem::MemMap, addr: u32) -> Option<u32> {
    Some(u32::from_le_bytes(peek(mem, addr, 4)?.try_into().ok()?))
}

/// Original descriptor ABI and signed-16 arithmetic, with only X extended.
fn visibility_result(
    mem: &crate::mem::MemMap,
    regs: &[u32; 16],
    extra: usize,
    extra_y: usize,
) -> Option<u32> {
    let count = peek(mem, regs[0], 2).map(|v| i16::from_le_bytes(v.try_into().unwrap()))?;
    if count <= 0 {
        return Some(0);
    }
    let data = peek(mem, regs[0].checked_add(2)?, count as usize * 6)?;
    let flip_x = regs[3] as u8 != 0;
    let flip_y = word(mem, regs[13])? as u8 != 0;
    let left = -(extra as i32);
    let right = 239 + extra as i32;
    for p in data.chunks_exact(6) {
        let a0 = u16::from_le_bytes([p[0], p[1]]);
        let a1 = u16::from_le_bytes([p[2], p[3]]);
        let (w, h) = dimension_values(mem, 0x03006d28 + 4 * ((a0 >> 14) * 4 + (a1 >> 14)) as u32)?;
        let w = w as i16 as i32;
        let h = h as i32;
        let ox = (((a1 & 511) as i32) << 23) >> 23;
        let oy = a0 as u8 as i8 as i32;
        let x = if flip_x {
            (regs[1] as i32).wrapping_sub(ox).wrapping_sub(w)
        } else {
            (regs[1] as i32).wrapping_add(ox)
        } as i16 as i32;
        if x < left && x + w < left || x > right && x + w > right {
            continue;
        }
        let y = if flip_y {
            (regs[2] as i32).wrapping_sub(oy).wrapping_sub(h)
        } else {
            (regs[2] as i32).wrapping_add(oy)
        } as i16 as i32;
        if y < -(extra_y as i32) && y + h < -(extra_y as i32)
            || y > 159 + extra_y as i32 && y + h > 159 + extra_y as i32
        {
            continue;
        }
        return Some(1);
    }
    Some(0)
}

// A/B switch for the vertical extension; original PC horizontal behavior is kept.
fn zoom_sprite_extra_y() -> usize {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *ON.get_or_init(|| std::env::var("FFTA_PATCH_ZOOM_SPR").as_deref() != Ok("0")) {
        crate::mem::extra_y()
    } else {
        0
    }
}
static CHARACTER_METRICS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_character_metrics(enabled: bool) {
    CHARACTER_METRICS.store(enabled, std::sync::atomic::Ordering::Relaxed);
}
pub fn character_metrics() -> bool {
    CHARACTER_METRICS.load(std::sync::atomic::Ordering::Relaxed)
}
/// Same measured tutorial roster as the desktop metric. Body owners group all
/// pieces; auxiliary owners, cursor/effects and roster id 8 are excluded.
pub fn tutorial_metric(bus: &crate::mem::MemMap) -> String {
    let mut valid = true;
    let mut ready = Vec::new();
    let mut drawn = Vec::new();
    let mut duplicates = Vec::new();
    let mut missing = Vec::new();
    for index in 0..9u32 {
        let record = 0x020223a4 + index * 0x90;
        let id = word(bus, record + 0x64).unwrap_or(u32::MAX);
        let body = word(bus, record + 0x5c).unwrap_or(0);
        valid &= word(bus, record) == Some(0x0024616c) && id <= 8 && body >> 24 == 2;
        if id >= 8 {
            continue;
        }
        let groups: std::collections::BTreeSet<_> = bus
            .ffta_sprites
            .active
            .iter()
            .flatten()
            .filter(|o| o.owner == body)
            .map(|o| o.group)
            .collect();
        if !groups.is_empty() {
            ready.push(id);
        } else {
            let x = peek(bus, body + 12, 2)
                .map(|v| i16::from_le_bytes([v[0], v[1]]))
                .unwrap_or(0);
            let y = peek(bus, body + 14, 2)
                .map(|v| i16::from_le_bytes([v[0], v[1]]))
                .unwrap_or(0);
            missing.push(format!("{id}:{x}:{y}"));
        }
        if groups.len() > 1 {
            duplicates.push(id);
        }
        if bus
            .ffta_sprites
            .pixel_counts
            .get(&body)
            .copied()
            .unwrap_or(0)
            > 0
        {
            drawn.push(id);
        }
    }
    let ids = |v: Vec<u32>| {
        v.into_iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("|")
    };
    format!(
        "{},{},{},{},{},{},{},{}",
        bus.frames,
        valid,
        ready.len(),
        drawn.len(),
        ids(ready),
        ids(drawn),
        ids(duplicates),
        missing.join("|")
    )
}

pub fn trace_enabled(frame: u64) -> bool {
    static FROM: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    crate::ffta_log::enabled()
        && frame
            >= *FROM.get_or_init(|| {
                std::env::var("FFTA_OAM_LOG_FROM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0)
            })
}

/// Per-frame observations; no guest state or emulated time is changed.
pub fn trace_frame(bus: &crate::mem::MemMap) {
    if !trace_enabled(bus.frames) {
        return;
    }
    if matches!(bus.frames, 11340 | 12042 | 12540) {
        if let Ok(prefix) = std::env::var("FFTA_OAM_RAM_DUMP_PREFIX") {
            for (name, data) in [("ewram", &bus.ewram), ("iwram", &bus.iwram)] {
                let path = format!("{prefix}-{}-{name}.bin", bus.frames);
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                {
                    Ok(mut file) => {
                        if let Err(error) = std::io::Write::write_all(&mut file, data) {
                            eprintln!("FFTA_RAM_DUMP_ERROR {path} {error}");
                        }
                    }
                    Err(error) => eprintln!("FFTA_RAM_DUMP_ERROR {path} {error}"),
                }
            }
        }
    }
    if bus.frames == 10500 {
        println!(
            "FFTA_RAM_ID frame={} ram=030062ec rom=08a38ea0 length=296 equal={} sha256={}",
            bus.frames,
            bus.rom
                .get(0xa38ea0..0xa38fc8)
                .is_some_and(|original| bus.iwram[0x62ec..0x6414] == *original),
            crate::ffta_rom::digest(&bus.iwram[0x62ec..0x6414])
        );
    }
    let reg = |off| u16::from_le_bytes([bus.io[off], bus.io[off + 1]]);
    println!(
        "FFTA_FRAME frame={} camera={:?} bgcnt={:04x},{:04x},{:04x},{:04x} host={} publications={} mismatches={}",
        bus.frames,
        (0..4)
            .map(|i| (reg(0x10 + i * 4) & 511, reg(0x12 + i * 4) & 511))
            .collect::<Vec<_>>(),
        reg(8),
        reg(10),
        reg(12),
        reg(14),
        bus.ffta_sprites.active.as_ref().map_or(0, Vec::len),
        bus.ffta_sprites.publications,
        bus.ffta_sprites.mismatches
    );
    if let Some(objects) = bus.ffta_sprites.active.as_ref() {
        for (index, s) in objects.iter().enumerate() {
            println!(
                "FFTA_FRAME_HOST frame={} index={index} slot={} x={} y={} attrs={:04x?} replaces={} owner={:08x} group={}",
                bus.frames, s.slot, s.x, s.y, s.attrs, s.replaces, s.owner, s.group
            );
        }
    }
    for &address in &bus.ffta_sprites.observed_instances {
        if let Some(data) = peek(bus, address, 0x174) {
            let half = |off| i16::from_le_bytes([data[off], data[off + 1]]);
            let words = (0x140..0x174)
                .step_by(4)
                .map(|off| u32::from_le_bytes(data[off..off + 4].try_into().unwrap()))
                .collect::<Vec<_>>();
            let mut tiles = Vec::new();
            let mut nonzero = 0;
            let mut missing = 0;
            if let Some(offsets) = bus.ffta_sprites.observed_tiles.get(&address) {
                for &off in offsets {
                    if let Some(chunk) = bus.vram.get(off..off + 32) {
                        nonzero += chunk.iter().filter(|&&v| v != 0).count();
                        tiles.extend_from_slice(chunk);
                    } else {
                        missing += 1;
                    }
                }
            }
            let gfx = u32::from_le_bytes(data[0x150..0x154].try_into().unwrap());
            println!(
                "FFTA_INSTANCE frame={} address={address:08x} flags={:04x} x={} y={} tile={} priority_bytes={:02x?} bounds_and_links={:08x?} tile_bytes={} tile_nonzero={nonzero} tile_out_of_range={missing} tile_sha256={} gfx={gfx:08x} gfx_word0={:08x?}",
                bus.frames,
                half(0) as u16,
                half(12),
                half(14),
                half(28),
                &data[0x1e..0x30],
                words,
                tiles.len(),
                crate::ffta_rom::digest(&tiles),
                word(bus, gfx)
            );
        }
    }
    for (slot, entry) in bus.oam.chunks_exact(8).enumerate() {
        let attrs = [
            u16::from_le_bytes([entry[0], entry[1]]),
            u16::from_le_bytes([entry[2], entry[3]]),
            u16::from_le_bytes([entry[4], entry[5]]),
        ];
        if attrs[0] & 0x300 == 0x200 {
            continue;
        }
        println!(
            "FFTA_FRAME_OAM frame={} slot={slot} x9={} y8={} attrs={:04x?}",
            bus.frames,
            attrs[1] & 511,
            attrs[0] & 255,
            attrs
        );
    }
    if bus.frames >= 11340 {
        let mut bodies = BTreeMap::new();
        let mut auxiliaries = std::collections::BTreeSet::new();
        let mut valid = true;
        for index in 0..9u32 {
            let record = 0x020223a4 + index * 0x90;
            let id = word(bus, record + 0x64).unwrap_or(u32::MAX);
            let body = word(bus, record + 0x5c).unwrap_or(0);
            let aux = word(bus, record + 0x60).unwrap_or(0);
            valid &= word(bus, record) == Some(0x0024616c) && id <= 8 && body >> 24 == 2;
            if id < 8 {
                bodies.insert(body, id);
                auxiliaries.insert(aux);
            }
        }
        let mut ready = std::collections::BTreeSet::new();
        let mut pixels = std::collections::BTreeSet::new();
        let mut oam = std::collections::BTreeSet::new();
        let mut groups: BTreeMap<u32, std::collections::BTreeSet<u64>> = BTreeMap::new();
        for &owner in &bus.ffta_sprites.oam_owners {
            if let Some(&id) = bodies.get(&owner) {
                oam.insert(id);
            }
        }
        if crate::mem::extra() > 0 && bus.ffta_sprites.enabled {
            if let Some(objects) = bus.ffta_sprites.active.as_ref() {
                for object in objects {
                    if let Some(&id) = bodies.get(&object.owner) {
                        ready.insert(id);
                        groups.entry(id).or_default().insert(object.group);
                    }
                }
            }
        } else {
            ready = oam.clone();
        }
        for (&owner, &count) in &bus.ffta_sprites.pixel_counts {
            if count > 0 {
                if let Some(&id) = bodies.get(&owner) {
                    pixels.insert(id);
                }
            }
        }
        let duplicates = groups
            .iter()
            .filter_map(|(&id, group)| (group.len() > 1).then_some((id, group.len())))
            .collect::<Vec<_>>();
        let npc = word(bus, 0x02022824 + 0x5c).unwrap_or(0);
        println!(
            "FFTA_CHARACTERS frame={} roster_valid={valid} present={} ids={:?} visible={} visible_ids={:?} oam={} oam_ids={:?} duplicate_groups={duplicates:?} npc_present={} npc_pixels={} auxiliary_pixels={}",
            bus.frames,
            ready.len(),
            ready,
            pixels.len(),
            pixels,
            oam.len(),
            oam,
            bus.ffta_sprites
                .active
                .as_ref()
                .is_some_and(|objects| objects.iter().any(|s| s.owner == npc)),
            bus.ffta_sprites
                .pixel_counts
                .get(&npc)
                .copied()
                .unwrap_or(0),
            auxiliaries
                .iter()
                .map(|a| bus.ffta_sprites.pixel_counts.get(a).copied().unwrap_or(0))
                .sum::<u32>()
        );
    }
    let requested = std::env::var("FFTA_OAM_CAPTURE_FRAMES")
        .ok()
        .is_some_and(|values| {
            values
                .split(',')
                .any(|v| v.parse::<u64>().ok() == Some(bus.frames))
        });
    if bus.frames % 30 == 0 || requested {
        println!(
            "FFTA_FRAME_HASH frame={} framebuffer={} wide={} oam={} obj_vram={}",
            bus.frames,
            crate::ffta_rom::digest(
                &bus.framebuffer
                    .iter()
                    .flat_map(|p| p.to_le_bytes())
                    .collect::<Vec<_>>()
            ),
            crate::ffta_rom::digest(
                &bus.wide_framebuffer[..crate::mem::render_width() * 160]
                    .iter()
                    .flat_map(|p| p.to_le_bytes())
                    .collect::<Vec<_>>()
            ),
            crate::ffta_rom::digest(&bus.oam),
            crate::ffta_rom::digest(&bus.vram[0x10000..0x18000])
        );
        tile_diagnostics(bus);
        if let Ok(prefix) = std::env::var("FFTA_OAM_CAPTURE_PREFIX") {
            if bus.frames % 120 == 0 || requested {
                let width = crate::mem::render_width();
                let mut ppm = format!("P6\n{width} 160\n255\n").into_bytes();
                for &px in &bus.wide_framebuffer[..width * 160] {
                    for value in [px & 31, (px >> 5) & 31, (px >> 10) & 31] {
                        let v = value as u8;
                        ppm.push(v << 3 | v >> 2);
                    }
                }
                let path = format!("{prefix}-{}.ppm", bus.frames);
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                {
                    Ok(mut file) => {
                        if let Err(error) = std::io::Write::write_all(&mut file, &ppm) {
                            eprintln!("FFTA_CAPTURE_ERROR {path} {error}");
                        }
                    }
                    Err(error) => eprintln!("FFTA_CAPTURE_ERROR {path} {error}"),
                }
            }
        }
    }
}

/// Own decoder of the measured six-byte descriptor ABI. Never writes guest memory.
fn decode(mem: &crate::mem::MemMap, regs: &[u32; 16]) -> Option<(u32, Vec<HostSprite>)> {
    let profile = mem.ffta_sprites.profile?;
    let first = regs[15] == profile.draw_a;
    let arg = |offset| word(mem, regs[13].wrapping_add(offset));
    let (tile_add, prio_add, pal_add, buffer, limit, initial, flags) = if first {
        (
            arg(0)?,
            arg(4)?,
            arg(8)?,
            arg(12)?,
            arg(16)?,
            arg(20)?,
            regs[3],
        )
    } else {
        (
            regs[3],
            arg(0)? & 255,
            arg(4)?,
            arg(8)?,
            arg(12)?,
            arg(16)?,
            0,
        )
    };
    // The battle also submits 48-entry buffers at 03002C58/03002DD8.
    // Accept the function's actual RAM buffer, not only the title/UI banks.
    // Publication still checks every replaced attribute against transferred OAM.
    if buffer & 3 != 0
        || !matches!(buffer >> 24, 2 | 3)
        || peek(mem, buffer, (limit.min(128) * 8) as usize).is_none()
        || initial > 128
        || limit > 128
    {
        return None;
    }
    let count = u16::from_le_bytes(peek(mem, regs[0], 2)?.try_into().ok()?) as usize;
    if !first && count >= 32768 {
        return Some((buffer, Vec::new()));
    }
    let descriptors = peek(mem, regs[0].checked_add(2)?, count.checked_mul(6)?)?;
    let mut next = initial as usize;
    let mut sprites = Vec::with_capacity(count);
    for bytes in descriptors.chunks_exact(6) {
        let mut a0 = u16::from_le_bytes([bytes[0], bytes[1]]);
        let mut a1 = u16::from_le_bytes([bytes[2], bytes[3]]);
        let mut a2 = u16::from_le_bytes([bytes[4], bytes[5]]);
        let dim = if profile.draw_a == 0x08001500 {
            0x0814ced8
        } else {
            0x08149510
        } + 4 * (((a0 >> 14) * 4 + (a1 >> 14)) as u32);
        let (w, h) = dimension_values(mem, dim)?;
        let w = w as i32;
        let h = h as i32;
        let ox = (((a1 & 511) as i32) << 23) >> 23;
        let oy = a0 as u8 as i8 as i32;
        let origin_x = regs[1] as i32;
        let origin_y = regs[2] as i32;
        let x = if flags & 0x10000000 != 0 {
            origin_x.wrapping_sub(ox).wrapping_sub(w)
        } else {
            origin_x.wrapping_add(ox)
        };
        let y = if flags & 0x20000000 != 0 {
            origin_y.wrapping_sub(oy).wrapping_sub(h)
        } else {
            origin_y.wrapping_add(oy)
        };
        // Model original rejection only to merge original slots; do not reject host objects.
        let gx = x as i16 as i32;
        let gy = y as i16 as i32;
        let visible = !(gx < 0 && gx + w < 0
            || gx > 239 && gx + w > 239
            || gy < 0 && gy + h < 0
            || gy > 159 && gy + h > 159);
        let replaces = visible && next < limit as usize;
        let slot = next;
        if replaces {
            next += 1;
        }
        a0 = (a0 & 0xff00) | (y as u16 & 255);
        a1 = (a1 & !511) | (x as u16 & 511);
        let packed =
            (((a0 as u32) | ((a1 as u32) << 16)) | (flags & 0xcfffffff)) ^ (flags & 0x30000000);
        a0 = packed as u16;
        a1 = (packed >> 16) as u16;
        let tile = (a2 as u32 & 1023).wrapping_add(tile_add) & 1023;
        let prio = ((a2 as u32 >> 10) & 3).wrapping_add(prio_add) & 3;
        let pal = (a2 as u32 >> 12).wrapping_add(pal_add) & 15;
        a2 = (tile | (prio << 10) | (pal << 12)) as u16;
        let stats_background = regs[0] == profile.stats_list
            && count == 4
            && a0 & 0x0c00 == 0x0400
            && a2 >> 12 == 11
            && matches!(a2 & 1023, 0x1f0 | 0x200 | 0x208)
            && stats_panel_enabled();
        let stats_inner_padding = stats_background && profile.draw_a == 0x08001500;
        sprites.push(HostSprite {
            x,
            y,
            attrs: [a0, a1, a2],
            stats_background,
            stats_inner_padding,
            slot,
            replaces,
            ..HostSprite::default()
        });
    }
    Some((buffer, sprites))
}

/// Battle routine copied verbatim from ROM A3915C to IWRAM 65A8.
fn decode_battle(mem: &crate::mem::MemMap, regs: &[u32; 16]) -> Option<(u32, Vec<HostSprite>)> {
    let arg = |offset| word(mem, regs[13].wrapping_add(offset));
    let bank = *peek(mem, 0x03000028, 1)?.first()? as u32;
    if bank > 1 {
        return None;
    }
    let buffer = 0x03000030 + bank * 1024;
    let initial = word(mem, 0x03000020 + bank * 4)? as i32;
    if !(0..=128).contains(&initial) {
        return None;
    }
    let budget = arg(28)? as i32;
    let limit = if budget == 0 {
        128
    } else {
        initial.wrapping_add(budget).min(128)
    };
    let count = u16::from_le_bytes(peek(mem, regs[0], 2)?.try_into().ok()?) as i16;
    if count <= 0 {
        return Some((buffer, Vec::new()));
    }
    let count = count as usize;
    let data = peek(mem, regs[0].checked_add(2)?, count.checked_mul(6)?)?;
    let priorities = peek(mem, arg(20)?, count)?;
    let mosaic = arg(0)? & 1;
    let bpp8 = arg(4)? & 1;
    let flip_x = arg(8)? as u8;
    let flip_y = arg(12)? as u8;
    let tile_add = arg(16)?;
    let pal_add = arg(24)?;
    let mut next = initial;
    let mut objects = Vec::with_capacity(count);
    for (i, p) in data.chunks_exact(6).enumerate() {
        let mut a0 = u16::from_le_bytes([p[0], p[1]]);
        let mut a1 = u16::from_le_bytes([p[2], p[3]]);
        let a2 = u16::from_le_bytes([p[4], p[5]]);
        let dim = 0x03006d28 + 4 * (((a0 >> 14) * 4 + (a1 >> 14)) as u32);
        let (w, h) = dimension_values(mem, dim)?;
        let w = w as i32;
        let h = h as i32;
        let ox = (((a1 & 511) as i32) << 23) >> 23;
        let oy = a0 as u8 as i8 as i32;
        let x = if flip_x != 0 {
            (regs[1] as i32).wrapping_sub(ox).wrapping_sub(w)
        } else {
            (regs[1] as i32).wrapping_add(ox)
        };
        let y = if flip_y != 0 {
            (regs[2] as i32).wrapping_sub(oy).wrapping_sub(h)
        } else {
            (regs[2] as i32).wrapping_add(oy)
        };
        let gx = x as i16 as i32;
        let gy = y as i16 as i32;
        let end_x = gx.wrapping_add(w) as i16 as i32;
        // Literal original instruction 03006724 uses ip (X), not Y, here.
        let end_y = gx.wrapping_add(h) as i16 as i32;
        let visible = !(gx < 0 && end_x < 0
            || gx > 239 && end_x > 239
            || gy < 0 && end_y < 0
            || gy > 159 && end_y > 159);
        let replaces = visible && next < limit;
        let slot = next as usize;
        if replaces {
            next += 1;
        }
        a0 = (a0 & !(255 | 0x0c00 | 0x1000 | 0x2000))
            | (y as u16 & 255)
            | ((regs[3] as u16 & 3) << 10)
            | (mosaic as u16) << 12
            | (bpp8 as u16) << 13;
        a1 = ((a1 & !511) ^ (((flip_x as u16) & 1) << 12) ^ (((flip_y as u16) & 1) << 13))
            | (x as u16 & 511);
        let tile = ((a2 as u32 & 1023).wrapping_add(tile_add)) & 1023;
        let priority = (((a2 as u32 >> 10) & 3) + priorities[i] as u32) & 3;
        let palette = ((a2 as u32 >> 12).wrapping_add(pal_add)) & 15;
        objects.push(HostSprite {
            x,
            y,
            attrs: [a0, a1, (tile | priority << 10 | palette << 12) as u16],
            slot,
            replaces,
            owner: arg(20)?.checked_sub(0x1e)?,
            group: 0,
            stats_background: false,
            stats_inner_padding: false,
        });
    }
    Some((buffer, objects))
}
impl crate::machine::Machine {
    pub fn ffta_observe_native_producer(&mut self, pc: u32) {
        let Some(profile) = self.bus.ffta_sprites.profile else {
            return;
        };
        if self.bus.ffta_sprites.enabled
            || !self.bus.ffta_sprites.rom_valid
            || !trace_enabled(self.bus.frames)
            || (pc != profile.draw_a && pc != profile.draw_b && pc != profile.ram_start)
        {
            return;
        }
        let mut regs = self.cpu.regs;
        regs[15] = pc;
        let decoded = if pc == profile.ram_start {
            decode_battle(&self.bus, &regs)
        } else {
            decode(&self.bus, &regs)
        };
        if let Some((buffer, objects)) = decoded {
            let pending = self
                .bus
                .ffta_sprites
                .observed_pending
                .entry(buffer)
                .or_default();
            if objects.first().is_some_and(|s| s.slot == 0) {
                pending.clear();
            }
            pending.extend(objects);
        }
    }
    pub fn ffta_visibility_entry(&mut self) {
        if !self.bus.ffta_sprites.enabled
            || !self.bus.ffta_sprites.vis_enabled
            || (crate::mem::extra() == 0 && zoom_sprite_extra_y() == 0)
            || !self.cpu.thumb()
        {
            return;
        }
        let Some(profile) = self.bus.ffta_sprites.profile else {
            return;
        };
        let key = (self.cpu.regs[13], self.cpu.cpsr & 31);
        if self.cpu.regs[15] == profile.vis_start {
            self.bus.ffta_sprites.vis_verified = self.bus.iwram.get(0x62ec..0x6414)
                == self.bus.rom.get(profile.vis_source..profile.vis_source_end)
                && self.bus.iwram.get(0x6d28..0x6d58)
                    == self.bus.obj_source_table(profile.size_table);
            if self.bus.ffta_sprites.vis_verified {
                if let Some(result) = visibility_result(
                    &self.bus,
                    &self.cpu.regs,
                    crate::mem::extra(),
                    zoom_sprite_extra_y(),
                ) {
                    self.bus.ffta_sprites.visibility_calls.insert(
                        key,
                        (
                            result,
                            self.cpu.regs[1] as i32,
                            self.cpu.regs[2] as i32,
                            self.cpu.regs[0],
                            self.cpu.regs[14],
                        ),
                    );
                }
            }
        } else if self.cpu.regs[15] == profile.vis_return {
            if let Some((result, x, y, list, caller)) =
                self.bus.ffta_sprites.visibility_calls.remove(&key)
            {
                if trace_enabled(self.bus.frames) {
                    println!(
                        "FFTA_VIS_RETURN frame={} original={} new={result} lr={caller:08x} list={list:08x} x={x} y={y}",
                        self.bus.frames, self.cpu.regs[0]
                    );
                }
                self.cpu.regs[0] = result;
            }
        }
    }
    /// Native blocks use the same interpreter, memory and instruction timing as step().
    pub fn ffta_sprite_interpret(&mut self, pc: u32) -> bool {
        if !self.bus.ffta_sprites.enabled || !self.cpu.thumb() {
            return false;
        }
        let Some(profile) = self.bus.ffta_sprites.profile else {
            return false;
        };
        if (profile.vis_start..profile.vis_end).contains(&pc) {
            if !self.bus.ffta_sprites.vis_enabled
                || (crate::mem::extra() == 0 && zoom_sprite_extra_y() == 0)
            {
                return false;
            }
            if pc == profile.vis_start {
                self.bus.ffta_sprites.vis_verified = self.bus.iwram.get(0x62ec..0x6414)
                    == self.bus.rom.get(profile.vis_source..profile.vis_source_end)
                    && self.bus.iwram.get(0x6d28..0x6d58)
                        == self.bus.obj_source_table(profile.size_table);
            }
            return self.bus.ffta_sprites.vis_verified;
        }
        if pc == profile.ram_start {
            self.bus.ffta_sprites.ram_verified = self.bus.iwram.get(0x65a8..0x68a4)
                == self.bus.rom.get(profile.ram_source..profile.ram_source_end)
                && self.bus.iwram.get(0x6d28..0x6d58)
                    == self.bus.obj_source_table(profile.size_table);
        }
        if (profile.ram_start..profile.ram_end).contains(&pc) {
            return self.bus.ffta_sprites.ram_verified;
        }
        crate::ffta_rom::sprite_pc(pc)
    }
    pub fn ffta_sprite_entry(&mut self) {
        if !self.bus.ffta_sprites.enabled {
            self.ffta_observe_native_producer(self.cpu.regs[15]);
            return;
        }
        let Some(profile) = self.bus.ffta_sprites.profile else {
            return;
        };
        if self.ffta_sprite_interpret(self.cpu.regs[15])
            && (self.cpu.regs[15] == profile.draw_a
                || self.cpu.regs[15] == profile.draw_b
                || self.cpu.regs[15] == profile.ram_start)
        {
            self.bus.ffta_sprites.calls += 1;
            if trace_enabled(self.bus.frames) {
                if self.cpu.regs[15] == profile.ram_start {
                    if let Some(address) =
                        word(&self.bus, self.cpu.regs[13] + 20).and_then(|v| v.checked_sub(0x1e))
                    {
                        if address >> 24 == 2 || address >> 24 == 3 {
                            self.bus.ffta_sprites.observed_instances.insert(address);
                        }
                    }
                }
                let stack = (0..16)
                    .map(|i| word(&self.bus, self.cpu.regs[13] + i * 4))
                    .collect::<Vec<_>>();
                println!(
                    "FFTA_PRODUCER frame={} pc={:08x} lr={:08x} list={:08x} x={} y={} regs={:08x?} stack={:08x?}",
                    self.bus.frames,
                    self.cpu.regs[15],
                    self.cpu.regs[14],
                    self.cpu.regs[0],
                    self.cpu.regs[1] as i32,
                    self.cpu.regs[2] as i32,
                    self.cpu.regs,
                    stack
                );
            }
            if crate::mem::extra() > 0 || trace_enabled(self.bus.frames) {
                let decoded = if self.cpu.regs[15] == profile.ram_start {
                    decode_battle(&self.bus, &self.cpu.regs)
                } else {
                    decode(&self.bus, &self.cpu.regs)
                };
                if let Some((buffer, mut sprites)) = decoded {
                    for sprite in &mut sprites {
                        sprite.group = (self.bus.frames << 32) | self.bus.ffta_sprites.calls;
                    }
                    if trace_enabled(self.bus.frames) {
                        let observed = self
                            .bus
                            .ffta_sprites
                            .observed_pending
                            .entry(buffer)
                            .or_default();
                        if sprites.first().is_some_and(|s| s.slot == 0) {
                            observed.clear();
                        }
                        observed.extend(sprites.iter().cloned());
                    }
                    if trace_enabled(self.bus.frames) {
                        if self.cpu.regs[15] == profile.ram_start {
                            if let Some(address) = word(&self.bus, self.cpu.regs[13] + 20)
                                .and_then(|v| v.checked_sub(0x1e))
                            {
                                let mut offsets = std::collections::BTreeSet::new();
                                let sizes = [
                                    [(8, 8), (16, 16), (32, 32), (64, 64)],
                                    [(16, 8), (32, 8), (32, 16), (64, 32)],
                                    [(8, 16), (8, 32), (16, 32), (32, 64)],
                                ];
                                for sprite in &sprites {
                                    let [a0, a1, a2] = sprite.attrs;
                                    let Some(shape) = sizes.get((a0 >> 14) as usize) else {
                                        continue;
                                    };
                                    let (w, h) = shape[(a1 >> 14) as usize];
                                    let step = if a0 & 0x2000 != 0 { 2 } else { 1 };
                                    let stride = if self.bus.io[0] & 0x40 != 0 {
                                        w / 8 * step
                                    } else {
                                        32
                                    };
                                    for row in 0..h / 8 {
                                        for col in 0..w / 8 {
                                            let tile =
                                                (a2 & 1023) as usize + row * stride + col * step;
                                            let tile = if step == 2 { tile & !1 } else { tile };
                                            offsets.insert(0x10000 + tile * 32);
                                            if step == 2 {
                                                offsets.insert(0x10000 + (tile + 1) * 32);
                                            }
                                        }
                                    }
                                }
                                self.bus
                                    .ffta_sprites
                                    .observed_tiles
                                    .insert(address, offsets.into_iter().collect());
                            }
                        }
                        println!(
                            "FFTA_PRODUCER_LIST frame={} pc={:08x} lr={:08x} buffer={buffer:08x} received={}",
                            self.bus.frames,
                            self.cpu.regs[15],
                            self.cpu.regs[14],
                            sprites.len()
                        );
                        for (index, s) in sprites.iter().enumerate() {
                            println!(
                                "FFTA_PRODUCER_OBJ frame={} pc={:08x} list={:08x} index={index} slot={} x={} y={} attrs={:04x?} replaces={}",
                                self.bus.frames,
                                self.cpu.regs[15],
                                self.cpu.regs[0],
                                s.slot,
                                s.x,
                                s.y,
                                s.attrs,
                                s.replaces
                            );
                        }
                    }
                    self.bus.ffta_sprites.decoded += 1;
                    let pending = self
                        .bus
                        .ffta_sprites
                        .pending
                        .entry(buffer)
                        .or_insert_with(|| (self.bus.frames, Vec::new()));
                    if pending.0 != self.bus.frames {
                        pending.0 = self.bus.frames;
                        pending.1.clear();
                    }
                    pending.1.extend(sprites);
                } else {
                    self.bus.ffta_sprites.failed_buffer = word(
                        &self.bus,
                        self.cpu.regs[13].wrapping_add(
                            if self
                                .bus
                                .ffta_sprites
                                .profile
                                .is_some_and(|p| self.cpu.regs[15] == p.draw_a)
                            {
                                12
                            } else {
                                8
                            },
                        ),
                    );
                }
            }
        }
    }
}

pub fn verification(bus: &crate::mem::MemMap) {
    if std::env::var("FFTA_PATCH_VERIFY").as_deref() == Ok("1") {
        println!(
            "FFTA_PATCH_VERIFY frames={} extra={} sprite_enabled={} calls={} framebuffer={} oam={}",
            bus.frames,
            crate::mem::extra(),
            bus.ffta_sprites.enabled,
            bus.ffta_sprites.calls,
            crate::ffta_rom::digest(
                &bus.framebuffer
                    .iter()
                    .flat_map(|p| p.to_le_bytes())
                    .collect::<Vec<_>>()
            ),
            crate::ffta_rom::digest(&bus.oam)
        );
        println!(
            "FFTA_HOST publications={} objects={}",
            bus.ffta_sprites.publications,
            bus.ffta_sprites.active.as_ref().map_or(0, Vec::len)
        );
        println!(
            "FFTA_PIPELINE decoded={} last_failed_buffer={:?} no_input={} mismatches={} last_mismatch={:?} active={} pending={:?}",
            bus.ffta_sprites.decoded,
            bus.ffta_sprites.failed_buffer,
            bus.ffta_sprites.no_input,
            bus.ffta_sprites.mismatches,
            bus.ffta_sprites.mismatch,
            bus.ffta_sprites.active.is_some(),
            bus.ffta_sprites
                .pending
                .iter()
                .map(|(buffer, (frame, objects))| (format!("{buffer:08x}"), frame, objects.len()))
                .collect::<Vec<_>>()
        );
        let width = crate::mem::render_width();
        println!(
            "FFTA_VIEW width={} wide_sha256={} bgcnt={:02x}{:02x},{:02x}{:02x},{:02x}{:02x},{:02x}{:02x}",
            width,
            crate::ffta_rom::digest(
                &bus.wide_framebuffer[..width * 160]
                    .iter()
                    .flat_map(|p| p.to_le_bytes())
                    .collect::<Vec<_>>()
            ),
            bus.io[9],
            bus.io[8],
            bus.io[11],
            bus.io[10],
            bus.io[13],
            bus.io[12],
            bus.io[15],
            bus.io[14]
        );
        tile_diagnostics(bus);
    }
}

/// Zero data is measured, not proof that a tile is missing (transparent art is valid).
pub fn tile_diagnostics(bus: &crate::mem::MemMap) {
    let Some(sprites) = bus.ffta_sprites.active.as_ref() else {
        return;
    };
    let mut rejected = 0;
    let mut zero = 0;
    let mut out_of_range = 0;
    let sizes = [
        [(8, 8), (16, 16), (32, 32), (64, 64)],
        [(16, 8), (32, 8), (32, 16), (64, 32)],
        [(8, 16), (8, 32), (16, 32), (32, 64)],
    ];
    for s in sprites.iter().filter(|s| !s.replaces) {
        rejected += 1;
        let [a0, a1, a2] = s.attrs;
        let Some(shape) = sizes.get((a0 >> 14) as usize) else {
            continue;
        };
        let (w, h) = shape[(a1 >> 14) as usize];
        let bpp8 = a0 & 0x2000 != 0;
        let step = if bpp8 { 2 } else { 1 };
        let stride = if bus.io[0] & 0x40 != 0 {
            w / 8 * step
        } else {
            32
        };
        let mut nonzero = false;
        let mut invalid = false;
        for row in 0..h / 8 {
            for col in 0..w / 8 {
                let tile = (a2 & 1023) as usize + row * stride + col * step;
                let offset = 0x10000 + if bpp8 { tile / 2 * 64 } else { tile * 32 };
                match bus.vram.get(offset..offset + if bpp8 { 64 } else { 32 }) {
                    Some(data) => nonzero |= data.iter().any(|&b| b != 0),
                    None => invalid = true,
                }
            }
        }
        zero += (!nonzero && !invalid) as usize;
        out_of_range += invalid as usize;
    }
    println!(
        "FFTA_HOST_TILES frame={} objects={} original_rejected_or_capped={} all_tiles_zero={} out_of_obj_vram={} obj_vram_nonzero_bytes={} status=CONTENTS_ONLY",
        bus.frames,
        sprites.len(),
        rejected,
        zero,
        out_of_range,
        bus.vram[0x10000..0x18000]
            .iter()
            .filter(|&&b| b != 0)
            .count()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, Machine};
    #[test]
    #[ignore = "requires the user's exact EUR/USA ROMs; original instructions are read only from those local images"]
    fn decoded_metadata_matches_original_regional_draw_instructions_and_host_lists() {
        use crate::rom_data::{ObjDimensions, RuntimeResources};
        let mut cases = 0;
        for env in ["FFTA_TEST_ROM_USA", "FFTA_TEST_ROM_EUR"] {
            let rom = std::fs::read(std::env::var(env).unwrap()).unwrap();
            let profile = crate::ffta_rom::sprite_profile(&rom).unwrap();
            let version = crate::ffta_rom::version(&rom).unwrap();
            let source = crate::rom_data::Extraction::from_rom(&rom).unwrap();
            let resources = RuntimeResources::synthetic(&rom, version, source.dimensions);
            assert!(
                ObjDimensions::decode(&rom[profile.size_table..profile.size_table + 48]).is_ok()
            );
            for first in [true, false] {
                for flags in [0, 0x10000000, 0x20000000, 0x30000400] {
                    for x in [-20i32, 100, 250] {
                        let mut legacy = Machine::new(rom.clone());
                        let mut decoded = Machine::new(rom.clone());
                        decoded
                            .bus
                            .set_rom_resources(Some(resources.clone()))
                            .unwrap();
                        for m in [&mut legacy, &mut decoded] {
                            m.bus.ffta_sprites.enabled = false;
                            m.cpu.cpsr |= crate::cpu::FLAG_T;
                            m.cpu.regs[0] = 0x02000000;
                            m.cpu.regs[1] = x as u32;
                            m.cpu.regs[2] = 70;
                            m.cpu.regs[3] = if first { flags } else { 7 };
                            m.cpu.regs[13] = 0x03007000;
                            m.cpu.regs[14] = 0x08002001;
                            m.cpu.regs[15] = if first {
                                profile.draw_a
                            } else {
                                profile.draw_b
                            };
                            let args = if first {
                                vec![7, 2, 3, 0x03000030, 48, 0]
                            } else {
                                vec![2, 3, 0x03000030, 48, 0]
                            };
                            for (i, value) in args.into_iter().enumerate() {
                                m.bus.write32(0x03007000 + i as u32 * 4, value);
                            }
                            m.bus.write16(0x02000000, 12);
                            for i in 0..12u32 {
                                let p = 0x02000002 + i * 6;
                                m.bus.write16(
                                    p,
                                    ((i as i32 % 3 - 1) * 12) as u16 & 255 | ((i / 4) as u16) << 14,
                                );
                                m.bus.write16(
                                    p + 2,
                                    ((i as i32 - 6) * 12) as u16 & 511 | ((i % 4) as u16) << 14,
                                );
                                m.bus.write16(p + 4, i as u16 | 0x7400);
                            }
                        }
                        let (_, old_host) = decode(&legacy.bus, &legacy.cpu.regs).unwrap();
                        let (_, new_host) = decode(&decoded.bus, &decoded.cpu.regs).unwrap();
                        let summarize = |list: &[HostSprite]| {
                            list.iter()
                                .map(|s| (s.x, s.y, s.attrs, s.slot, s.replaces))
                                .collect::<Vec<_>>()
                        };
                        assert_eq!(summarize(&old_host), summarize(&new_host));
                        for step in 0..10000 {
                            assert_eq!(legacy.cpu.regs, decoded.cpu.regs);
                            assert_eq!(legacy.cpu.cpsr, decoded.cpu.cpsr);
                            assert_eq!(legacy.bus.clock, decoded.bus.clock);
                            if legacy.cpu.regs[15] == 0x08002000 {
                                break;
                            }
                            assert!(step < 9999, "original draw routine did not return");
                            legacy.step();
                            decoded.step();
                        }
                        assert_eq!(legacy.bus.iwram, decoded.bus.iwram);
                        for sprite in &new_host {
                            if sprite.replaces {
                                let off = 0x30 + sprite.slot * 8;
                                let actual: Vec<_> = decoded.bus.iwram[off..off + 6]
                                    .chunks_exact(2)
                                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                                    .collect();
                                assert_eq!(actual, sprite.attrs);
                            }
                        }
                        cases += 1;
                    }
                }
            }
            assert!(
                resources.stats().bus_reads > 0,
                "original instructions must consume decoded data"
            );
            assert!(resources.stats().host_rom_reads > 0);
            eprintln!("ROM_DATA_DRAW region={version:?} original_guest_reads={} typed_host_reads={} source_mask={}",
                resources.stats().bus_reads,resources.stats().host_rom_reads,resources.stats().bus_source_mask);
        }
        assert_eq!(cases, 48);
    }
    #[test]
    fn decoded_dimensions_replace_rom_reads_and_preserve_changed_ram() {
        use crate::rom_data::{ObjDimensions, ObjSize, RuntimeResources};
        let dimensions = ObjDimensions {
            entries: std::array::from_fn(|i| ObjSize {
                width: 8 << (i % 4),
                height: 8 << ((i + 1) % 4),
            }),
        };
        let bytes = dimensions.source_bytes();
        let mut rom = vec![0; 0xa39910];
        rom[0x149510..0x149540].copy_from_slice(&bytes);
        rom[0xa398dc..0xa3990c].copy_from_slice(&bytes);
        let resources =
            RuntimeResources::synthetic(&rom, crate::ffta_rom::Version::Usa, dimensions);
        let mut mem = crate::MemMap::new(rom);
        mem.set_rom_resources(Some(resources.clone())).unwrap();
        mem.rom[0x149510..0x149540].fill(0xcd);
        mem.rom[0xa398dc..0xa3990c].fill(0xcd);
        mem.iwram[0x6d28..0x6d58].copy_from_slice(&bytes);
        for i in 0..12 {
            let offset = i * 4;
            let expected = (
                u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()),
                u16::from_le_bytes(bytes[offset + 2..offset + 4].try_into().unwrap()),
            );
            assert_eq!(
                dimension_values(&mem, 0x08149510 + offset as u32),
                Some(expected)
            );
            assert_eq!(
                dimension_values(&mem, 0x03006d28 + offset as u32),
                Some(expected)
            );
        }
        mem.write16(0x03006d28, 123);
        assert_eq!(dimension_values(&mem, 0x03006d28), Some((123, 16)));
        mem.write32(0x03006d58, 0x34561234);
        assert_eq!(dimension_values(&mem, 0x03006d58), Some((0x1234, 0x3456)));
        assert_eq!(resources.stats().host_rom_reads, 12);
        assert_eq!(resources.stats().host_ram_reads, 12);
        assert_eq!(resources.stats().dynamic_ram_reads, 1);
    }
    #[test]
    fn visibility_extends_only_x_and_keeps_y_and_empty_lists() {
        let mut mem = crate::mem::MemMap::new(vec![0; 192]);
        mem.write16(0x02000000, 1);
        mem.write16(0x03006d28, 8);
        mem.write16(0x03006d2a, 8);
        let mut regs = [0u32; 16];
        regs[0] = 0x02000000;
        regs[1] = (-10i32) as u32;
        regs[2] = 50;
        regs[13] = 0x03007000;
        assert_eq!(visibility_result(&mem, &regs, 0, 0), Some(0));
        assert_eq!(visibility_result(&mem, &regs, 164, 0), Some(1));
        regs[1] = 300;
        assert_eq!(visibility_result(&mem, &regs, 0, 0), Some(0));
        assert_eq!(visibility_result(&mem, &regs, 164, 0), Some(1));
        regs[2] = 200;
        assert_eq!(visibility_result(&mem, &regs, 164, 0), Some(0));
        mem.write16(0x02000000, 0);
        assert_eq!(visibility_result(&mem, &regs, 648, 0), Some(0));
    }
    #[test]
    #[ignore = "requires user-owned ROM via FFTA_TEST_ROM"]
    fn visibility_original_result_matches_user_rom() {
        let rom = std::fs::read(std::env::var("FFTA_TEST_ROM").unwrap()).unwrap();
        assert!(crate::ffta_rom::eligible(&rom));
        assert!(crate::ffta_rom::verify_visibility(&rom));
        let mut cases = 0;
        for x in [-200, -10, -7, 0, 239, 240, 700] {
            for y in [-40, 0, 50, 159, 180] {
                for flip in [0, 1] {
                    let mut m = Machine::new(rom.clone());
                    m.bus.ffta_sprites.enabled = false;
                    m.bus.iwram[0x62ec..0x6414].copy_from_slice(
                        &rom[crate::ffta_rom::VIS_SOURCE_START..crate::ffta_rom::VIS_SOURCE_END],
                    );
                    m.bus.iwram[0x6d28..0x6d58].copy_from_slice(&rom[0xa398dc..0xa3990c]);
                    m.cpu.cpsr |= crate::cpu::FLAG_T;
                    m.cpu.regs[0] = 0x02000000;
                    m.cpu.regs[1] = x as u32;
                    m.cpu.regs[2] = y as u32;
                    m.cpu.regs[3] = flip;
                    m.cpu.regs[13] = 0x03007000;
                    m.cpu.regs[14] = 0x08002001;
                    m.cpu.regs[15] = crate::ffta_rom::VIS_START;
                    m.bus.write32(0x03007000, flip);
                    m.bus.write16(0x02000000, 1);
                    m.bus.write16(0x02000002, 0x8000);
                    m.bus.write16(0x02000004, 0);
                    let expected = visibility_result(&m.bus, &m.cpu.regs, 0, 0).unwrap();
                    let mut steps = 0;
                    while m.cpu.regs[15] != 0x08002000 {
                        m.step();
                        steps += 1;
                        assert!(steps < 10000);
                    }
                    assert_eq!(m.cpu.regs[0], expected, "x={x} y={y} flip={flip}");
                    cases += 1;
                }
            }
        }
        println!("FFTA_VISIBILITY original_results=IDENTICAL cases={cases}");
    }
    #[test]
    #[ignore = "requires user-owned ROM via FFTA_TEST_ROM; embeds no ROM bytes"]
    fn ffta_decoder_matches_original_and_keeps_rejected_objects() {
        let path = std::env::var("FFTA_TEST_ROM").expect("FFTA_TEST_ROM");
        let rom = std::fs::read(path).unwrap();
        assert!(crate::ffta_rom::eligible(&rom));
        for pc in [0x080014c8, 0x080016d4] {
            for buffer in [0x03000030, 0x03002c58, 0x03002dd8] {
                for flags in [0, 0x10000000, 0x20000000, 0x30000400] {
                    let mut m = Machine::new(rom.clone());
                    m.bus.ffta_sprites.enabled = false;
                    m.cpu.cpsr |= crate::cpu::FLAG_T;
                    m.cpu.regs[0] = 0x02000000;
                    m.cpu.regs[1] = 100;
                    m.cpu.regs[2] = 70;
                    m.cpu.regs[3] = if pc == 0x080014c8 { flags } else { 7 };
                    m.cpu.regs[13] = 0x03007000;
                    m.cpu.regs[14] = 0x08002001;
                    m.cpu.regs[15] = pc;
                    let args = if pc == 0x080014c8 {
                        vec![7, 2, 3, buffer, 48, 0]
                    } else {
                        vec![2, 3, buffer, 48, 0]
                    };
                    for (i, v) in args.into_iter().enumerate() {
                        m.bus.write32(m.cpu.regs[13] + i as u32 * 4, v);
                    }
                    m.bus.write16(0x02000000, 200);
                    for i in 0..200 {
                        let p = 0x02000002 + i * 6;
                        let y = (i as i32 % 11 - 5) as u16 & 255;
                        let x = (i as i32 % 17 - 8) as u16 & 511;
                        m.bus.write16(p, y | ((i % 3) as u16 * 0x4000));
                        m.bus.write16(p + 2, x | ((i % 4) as u16 * 0x4000));
                        m.bus.write16(p + 4, (i as u16) | 0x7400);
                    }
                    let (_, objects) = decode(&m.bus, &m.cpu.regs).unwrap();
                    assert_eq!(objects.len(), 200);
                    assert!(objects.iter().filter(|o| !o.replaces).count() >= 152);
                    let mut steps = 0;
                    while m.cpu.regs[15] != 0x08002000 {
                        m.step();
                        steps += 1;
                        assert!(steps < 100000);
                    }
                    for object in objects.iter().filter(|o| o.replaces) {
                        let p = buffer + object.slot as u32 * 8;
                        assert_eq!(
                            object.attrs,
                            [m.bus.read16(p), m.bus.read16(p + 2), m.bus.read16(p + 4)],
                            "pc={pc:x} flags={flags:x} slot={}",
                            object.slot
                        );
                    }
                }
            }
        }
        println!(
            "FFTA_DECODER original_attributes=IDENTICAL cases=24 host_objects_per_case=200 original_limit=48"
        );
    }
    #[test]
    #[ignore = "requires user-owned ROM via FFTA_TEST_ROM; embeds no ROM bytes"]
    fn ffta_ram_battle_decoder_matches_original() {
        let rom = std::fs::read(std::env::var("FFTA_TEST_ROM").expect("FFTA_TEST_ROM")).unwrap();
        assert!(crate::ffta_rom::eligible(&rom));
        let mut cases = 0;
        for bank in [0u32, 1] {
            for (x, y) in [
                (100, 70),
                (-40, 70),
                (300, 70),
                (700, 70),
                (100, 180),
                (100, -40),
            ] {
                for flip in [0, 1] {
                    let mut m = Machine::new(rom.clone());
                    m.bus.ffta_sprites.enabled = false;
                    m.bus.iwram[0x65a8..0x68a4].copy_from_slice(
                        &rom[crate::ffta_rom::RAM_SOURCE_START..crate::ffta_rom::RAM_SOURCE_END],
                    );
                    m.bus.iwram[0x6d28..0x6d58].copy_from_slice(&rom[0xa398dc..0xa3990c]);
                    m.bus.write8(0x03000028, bank as u8);
                    m.bus.write32(0x03000020 + bank * 4, 0);
                    m.cpu.cpsr |= crate::cpu::FLAG_T;
                    m.cpu.regs[0] = 0x02000000;
                    m.cpu.regs[1] = x as u32;
                    m.cpu.regs[2] = y as u32;
                    m.cpu.regs[3] = 1;
                    m.cpu.regs[13] = 0x03007000;
                    m.cpu.regs[14] = 0x08002001;
                    m.cpu.regs[15] = crate::ffta_rom::RAM_SPRITE_START;
                    for (i, arg) in [1, 0, flip, flip, 7, 0x02002000, 3, 48]
                        .into_iter()
                        .enumerate()
                    {
                        m.bus.write32(0x03007000 + i as u32 * 4, arg);
                    }
                    m.bus.write16(0x02000000, 200);
                    for i in 0..200u32 {
                        let p = 0x02000002 + i * 6;
                        m.bus.write16(
                            p,
                            ((i as i32 % 11 - 5) as u16 & 255) | ((i % 3) as u16 * 0x4000),
                        );
                        m.bus.write16(
                            p + 2,
                            ((i as i32 % 17 - 8) as u16 & 511) | ((i % 4) as u16 * 0x4000),
                        );
                        m.bus.write16(p + 4, i as u16 | 0x7400);
                        m.bus.write8(0x02002000 + i, (i % 4) as u8);
                    }
                    let (buffer, objects) = decode_battle(&m.bus, &m.cpu.regs).unwrap();
                    assert_eq!(objects.len(), 200);
                    let mut steps = 0;
                    while m.cpu.regs[15] != 0x08002000 {
                        m.step();
                        steps += 1;
                        assert!(steps < 100000);
                    }
                    for object in objects.iter().filter(|s| s.replaces) {
                        let p = buffer + object.slot as u32 * 8;
                        assert_eq!(
                            object.attrs,
                            [m.bus.read16(p), m.bus.read16(p + 2), m.bus.read16(p + 4)],
                            "bank={bank} x={x} y={y} flip={flip} slot={}",
                            object.slot
                        );
                    }
                    assert_eq!(
                        objects.iter().filter(|s| s.replaces).count() as u32,
                        m.bus.read32(0x03000020 + bank * 4)
                    );
                    cases += 1;
                }
            }
        }
        println!(
            "FFTA_RAM_DECODER original_attributes=IDENTICAL cases={cases} host_objects_per_case=200 original_limit=48"
        );
    }
    #[test]
    fn ffta_publish_rebuilds_and_clears_unrecognised_transfers() {
        let mut state = SpriteState {
            enabled: true,
            ..SpriteState::default()
        };
        let object = HostSprite {
            x: 700,
            y: 10,
            attrs: [10, 188, 1],
            slot: 0,
            replaces: true,
            ..HostSprite::default()
        };
        let mut oam = vec![0; 1024];
        for (i, a) in object.attrs.iter().enumerate() {
            oam[i * 2..i * 2 + 2].copy_from_slice(&a.to_le_bytes());
        }
        state.pending.insert(0x03000030, (1, vec![object]));
        for _ in 0..3 {
            state.publish(0x03000030, 0x07000000, 8, true, &oam);
            assert_eq!(state.active.as_ref().unwrap()[0].x, 700);
        }
        oam[0] = 11;
        state.publish(0x03000030, 0x07000000, 1024, true, &oam);
        assert!(state.active.is_none());
        state.publish(0x02000000, 0x07000000, 1024, true, &oam);
        assert!(state.active.is_none());
    }
    #[test]
    fn ffta_partial_oam_submissions_merge_shift_and_fail_closed() {
        let mut state = SpriteState {
            enabled: true,
            ..SpriteState::default()
        };
        let mut oam = vec![0; 1024];
        let cursor = HostSprite {
            x: 20,
            y: 10,
            attrs: [10, 20, 1],
            slot: 0,
            replaces: true,
            ..HostSprite::default()
        };
        let character = HostSprite {
            x: 50,
            y: 15,
            attrs: [15, 50, 2],
            slot: 0,
            replaces: true,
            ..HostSprite::default()
        };
        let hidden = HostSprite {
            x: 900,
            y: 15,
            attrs: [15, 388, 2],
            slot: 1,
            replaces: false,
            ..HostSprite::default()
        };
        for (offset, object) in [(0, &cursor), (32, &character)] {
            for (i, attr) in object.attrs.iter().enumerate() {
                oam[offset + i * 2..offset + i * 2 + 2].copy_from_slice(&attr.to_le_bytes());
            }
        }
        state.pending.insert(0x03002c58, (1, vec![cursor]));
        state
            .pending
            .insert(0x03000030, (1, vec![character, hidden]));
        state.publish(0x03002c58, 0x07000000, 32, true, &oam);
        state.publish(0x03000030, 0x07000020, 8, true, &oam);
        let objects = state.active.as_ref().unwrap();
        assert_eq!(
            objects.iter().map(|s| s.slot).collect::<Vec<_>>(),
            [0, 4, 5]
        );
        assert_eq!(objects[2].x, 900);
        // A mismatching partial update removes its own segment, not the cursor.
        oam[32] = 16;
        state.publish(0x03000030, 0x07000020, 8, true, &oam);
        assert_eq!(state.active.as_ref().unwrap().len(), 1);
        // Unknown data overlapping an existing segment must invalidate it.
        state.publish(0x02000000, 0x07000008, 8, true, &oam);
        assert!(state.active.is_none());
        assert_eq!(state.mismatches, 1);
    }
}
