//! Original Party/Clan/Area selectors. Mouse requests are committed by the
//! owning guest updater, so dependent portraits and panels follow the same path.
use crate::{Machine, MemMap};
fn h(b: &[u8], p: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(p..p + 2)?.try_into().ok()?))
}
fn w(b: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(p..p + 4)?.try_into().ok()?))
}
fn ram(p: u32, n: usize) -> Option<usize> {
    let p = p.checked_sub(0x02000000)? as usize;
    (p.checked_add(n)? <= 0x40000).then_some(p)
}
#[derive(Default)]
pub(crate) struct State {
    request: Option<Request>,
    widget: Option<(usize, u64)>,
    neutral: bool,
    neutral_map: Option<[usize; 4]>,
    closed: Option<usize>,
    scroll: Option<(usize, u16)>,
    resume: Option<Resume>,
    steps: u16,
    category_keys: Option<[u8; 6]>,
    list_cursor: std::sync::atomic::AtomicUsize,
}
#[derive(Clone, Copy)]
struct Resume {
    regs: [u32; 16],
    cpsr: u32,
    keys: [u8; 6],
    entry: u32,
}
#[derive(Clone, Copy)]
struct Request {
    base: usize,
    target: u8,
    kind: u8,
    confirm: bool,
    deadline: u64,
    committed: Option<u64>,
}
impl State {
    pub(crate) fn cancel(&mut self) {
        self.request = None;
        self.scroll = None;
        self.neutral = false;
    }
    pub(crate) fn pending(&self) -> bool {
        self.request.is_some() || self.scroll.is_some() || self.resume.is_some()
    }
    pub(crate) fn interprets(&self, pc: u32) -> bool {
        // Observe the common widget's live owner, not dormant copies in RAM.
        (0x0808bdec..0x0808bdf4).contains(&pc)
            || (self.pending() && (0x080091b4..0x080091bc).contains(&pc))
            || ((self.scroll.is_some() || self.resume.is_some())
                && ((0x08031600..0x08031610).contains(&pc)
                    || (0x08042478..0x08042548).contains(&pc)
                    || (0x08051160..0x08052c80).contains(&pc)
                    || (0x08086fd8..0x08086fe8).contains(&pc)
                    || (0x080853f8..0x08085408).contains(&pc)
                    || (0x08084b50..0x08084cb4).contains(&pc)
                    || (0x08086ed8..0x08086fcc).contains(&pc)
                    || matches!(pc, 0x080873b2 | 0x0807d6a0 | 0x080905c4 | 0x0809070c)
                    || pc == 0x08144f04))
            || (self.request.is_some()
                && ((0x080853f8..0x08085400).contains(&pc)
                    || (0x08075194..0x08075780).contains(&pc)
                    || matches!(pc, 0x0807d6a0 | 0x080905c4 | 0x0809070c)
                    || (0x08086fd8..0x080873c8).contains(&pc)
                    || (0x080876d0..0x08087c90).contains(&pc)
                    || (0x08084b50..0x08084cb4).contains(&pc)
                    || (0x08086ed8..0x08086fcc).contains(&pc)
                    || (0x08088328..0x08089308).contains(&pc)
                    || (0x0805176c..0x080518c4).contains(&pc)
                    || (0x08018cac..0x08018ce0).contains(&pc)
                    || (0x080395b4..0x080396b8).contains(&pc)
                    || (0x0808bdec..0x0808bfbc).contains(&pc)))
    }
}
pub fn active(mem: &MemMap) -> bool {
    mem.ffta_battle_mouse.enabled
        && mem.ffta_movement.europe_profile
        && matches!(
            w(&mem.iwram, 0x2804),
            Some(0x08072929 | 0x0804e849 | 0x08031601)
        )
}
#[derive(Clone, Debug)]
pub struct Scene {
    pub bounds: Vec<[i32; 4]>,
    pub selected: u8,
    base: usize,
    kind: u8,
}
impl Scene {
    pub fn read(mem: &MemMap) -> Option<Self> {
        if !active(mem) {
            return None;
        }
        if let Some(list)=options_list(mem) {
            let mut scene=list.scene(mem);scene.kind=7;return Some(scene);
        }
        if let Some((base, frame)) = mem
            .ffta_battle_mouse
            .roster
            .widget
            .filter(|(_, f)| mem.frames <= f + 4)
        {
            if let Some(scene) = widget(mem, base) {
                return Some(scene);
            }
            let _ = frame;
        }
        if let Some(scene) = jobs(mem).or_else(|| report(mem)) {
            return Some(scene);
        }
        if let Some(list) = list(mem) {
            return Some(list.scene(mem));
        }
        if w(&mem.iwram, 0x2804) == Some(0x08072929) {
            let manager = ram(w(&mem.iwram, 0x2818)?, 0x1d20)?;
            // Child state 9 owns the unit menu; state 3 is the gallery itself.
            let states = w(&mem.iwram, 0x2730 + 12)?.checked_sub(0x03000000)? as usize;
            if *mem.iwram.get(states)? != 3 {
                return None;
            }
            let count = mem.ewram[manager + 0x95c] as usize;
            let selected = mem.ewram[manager + 0x986];
            if count == 0 || count > 24 || selected as usize >= count {
                return None;
            }
            let mut bounds = Vec::with_capacity(count);
            for index in 0..count {
                let p = manager + 0x9fa + index * 20;
                let x = h(&mem.ewram, p)? as i16 as i32;
                let y = h(&mem.ewram, p + 2)? as i16 as i32
                    - h(&mem.ewram, manager + 0x998)? as i16 as i32;
                bounds.push([x + 2, y + 2, x + 32, (y + 40).min(152)]);
            }
            return Some(Self {
                bounds,
                selected,
                base: manager,
                kind: 0,
            });
        }
        None
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<u8> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        self.bounds
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                x >= b[0] as f32 && x < b[2] as f32 && y >= b[1] as f32 && y < b[3] as f32
            })
            .min_by(|(_, a), (_, b)| {
                let distance = |r: &[i32; 4]| {
                    (x - (r[0] + r[2]) as f32 * 0.5).powi(2)
                        + (y - (r[1] + r[3]) as f32 * 0.5).powi(2)
                };
                distance(a).total_cmp(&distance(b))
            })
            .map(|(i, _)| i as u8)
    }
}
fn widget(mem: &MemMap, base: usize) -> Option<Scene> {
    let count = *mem.ewram.get(base + 0x104)?;
    let selected = *mem.ewram.get(base + 0x105)?;
    if count == 0 || count > 32 || selected >= count || mem.ewram[base] & 1 == 0 {
        return None;
    }
    let mut bounds = Vec::new();
    for i in 0..count as usize {
        let p = base + 4 + i * 8;
        let offset = if w(&mem.iwram, 0x2804) == Some(0x08072929) && mem.ewram[base + 5] == 0 {
            [136, 8]
        } else {
            [0, 0]
        };
        let x = mem.ewram[p + 1] as i32 + offset[0];
        let y = mem.ewram[p + 2] as i32 + offset[1];
        if x > 232 || y >= 160 {
            return None;
        }
        let right = (i + 1..count as usize)
            .map(|j| base + 4 + j * 8)
            .filter(|p| mem.ewram[*p + 2] as i32 == y && mem.ewram[*p + 1] as i32 > x)
            .map(|p| mem.ewram[p + 1] as i32 - 2)
            .min()
            .unwrap_or(if x < 120 { 128 } else { 232 });
        bounds.push([(x - 7).max(0), (y - 5).max(0), right, (y + 11).min(160)]);
    }
    Some(Scene {
        bounds,
        selected,
        base,
        kind: 1,
    })
}
pub fn party_gallery(mem: &MemMap) -> bool {
    active(mem)
        && w(&mem.iwram, 0x2804) == Some(0x08072929)
        && w(&mem.iwram, 0x273c)
            .and_then(|p| p.checked_sub(0x03000000))
            .and_then(|p| mem.iwram.get(p as usize))
            .is_some_and(|state| *state == 3)
}
pub fn aim(mem: &mut MemMap, x: f32, y: f32, confirm: bool) -> bool {
    let Some(scene) = Scene::read(mem) else {
        return false;
    };
    // Gallery tools are explicit click targets. Hover never sorts the party.
    if scene.kind == 0 && confirm && (0. ..16.).contains(&y) {
        let key = if x < 72. {
            0x1ff
        } else if x >= 176. {
            0x2ff
        } else {
            0x3f7
        };
        return crate::ffta_battle_mouse::press(mem, key);
    }
    if scene.kind == 5
        && party_publisher(mem) == 0x08084b50
        && (if full_inventory(mem) {
            0. ..16.
        } else {
            48. ..64.
        })
        .contains(&y)
    {
        if let Some(manager) = w(&mem.iwram, 0x2818).and_then(|p| ram(p, 0x3000)) {
            let ids: Vec<_> = (0..if full_inventory(mem) { 8 } else { 5 })
                .filter(|i| h(&mem.ewram, manager + 0xc60 + i * 2).is_some_and(|v| v != 0))
                .collect();
            let left = if full_inventory(mem) { 24. } else { 72. };
            if x >= left && x < left + ids.len() as f32 * 16. {
                if confirm {
                    let target = ids[((x - left) / 16.) as usize] as u8;
                    if mem.ewram[manager + 0xc21] != target {
                        mem.ffta_battle_mouse.roster.request = Some(Request {
                            base: scene.base,
                            target,
                            kind: 6,
                            confirm: false,
                            deadline: mem.frames + 120,
                            committed: None,
                        });
                    }
                }
                return true;
            }
        }
    }
    // Hover updates Party's native spotlight/portrait, but never confirms.
    // Once clicked, incidental motion must not retarget the queued opening.
    if scene.kind == 0
        && !confirm
        && mem
            .ffta_battle_mouse
            .roster
            .request
            .is_some_and(|r| r.kind == 0 && r.confirm)
    {
        return true;
    }
    let Some(target) = scene.hit(x, y) else {
        mem.ffta_battle_mouse.roster.cancel();
        return false;
    };
    if scene.kind == 2 {
        let Some(list) = list(mem) else {
            return false;
        };
        publish_list(mem, list, list.first, target, false);
        if confirm {
            return crate::ffta_battle_mouse::press(mem, 0x3fe);
        }
        return true;
    }
    let s = &mut mem.ffta_battle_mouse.roster;
    if let Some(r) = s
        .request
        .as_mut()
        .filter(|r| r.base == scene.base && r.target == target && r.kind == scene.kind)
    {
        r.confirm |= confirm;
        r.deadline = mem.frames + 120;
        return true;
    }
    s.neutral = false;
    s.request = Some(Request {
        base: scene.base,
        target,
        kind: scene.kind,
        confirm,
        deadline: mem.frames + 120,
        // A matching selector byte does not prove that a newly opened widget
        // has published its input/highlight state. Run its publisher even for
        // the default row; otherwise the first click can precede initialization.
        committed: (!matches!(scene.kind,1|7) && scene.selected == target).then_some(mem.frames),
    });
    true
}
/// Run Party's original gallery publisher independently of native block layout.
/// This routine is leaf code: no event scripts, frame execution or save writes.
pub fn publish_party_click(machine: &mut Machine) -> bool {
    let Some(request) = machine
        .bus
        .ffta_battle_mouse
        .roster
        .request
        .filter(|r| r.kind == 0 && r.committed.is_none())
    else {
        return false;
    };
    let Some(scene) = Scene::read(&machine.bus).filter(|s| {
        s.kind == 0 && s.base == request.base && (request.target as usize) < s.bounds.len()
    }) else {
        return false;
    };
    let _ = scene;
    if machine.bus.rom.get(0x75194..0x7519c)
        != Some(&[0xf0, 0xb5, 0x57, 0x46, 0x4e, 0x46, 0x45, 0x46])
    {
        return false;
    }
    let base = request.base;
    machine.bus.ewram[base + 0x971] = request.target % 4;
    machine.bus.ewram[base + 0x972] = request.target / 4;
    machine.bus.ewram[base + 0x970] |= 2;
    machine.bus.ewram[base + 0x25] = 0;
    // A private CPU/stack invokes the unmodified publisher. Its r0=FFFF
    // path refreshes troop identity, portrait pointer and yellow cursor.
    let stack = machine.bus.iwram[0x7000..0x7200].to_vec();
    let mut cpu = crate::Cpu::new();
    cpu.cpsr |= crate::cpu::FLAG_T | crate::cpu::FLAG_I;
    cpu.regs[0] = 0xffff;
    cpu.regs[1] = 1; // preserve the viewport while publishing the selector
    cpu.regs[13] = 0x03007200;
    cpu.regs[14] = 0x03007001;
    cpu.regs[15] = 0x08075194;
    let mut returned = false;
    for _ in 0..2048 {
        if cpu.regs[15] == 0x03007000 {
            returned = true;
            break;
        }
        crate::exec::step_hle(&mut cpu, &mut machine.bus);
    }
    machine.bus.iwram[0x7000..0x7200].copy_from_slice(&stack);
    if !returned {
        machine.bus.ffta_battle_mouse.roster.cancel();
        return false;
    }
    // Keep the original yellow spotlight directly under its new troop.
    for (current, target) in [(0x978, 0x980), (0x97a, 0x982)] {
        let value = h(&machine.bus.ewram, base + target).unwrap_or(0);
        machine.bus.ewram[base + current..base + current + 2].copy_from_slice(&value.to_le_bytes());
    }
    machine.bus.ewram[base + 0x984..base + 0x986].fill(0);
    machine
        .bus
        .ffta_battle_mouse
        .roster
        .request
        .as_mut()
        .unwrap()
        .committed = Some(machine.bus.frames);
    eprintln!("FFTA_PARTY_EVENT frame={} target={} unit={} portrait={:08x} yellow=({}, {}) native_publisher=true",
        machine.bus.frames, request.target, machine.bus.ewram[base + 0x987],
        w(&machine.bus.ewram,base + 0x1d10).unwrap_or(0),
        h(&machine.bus.ewram,base + 0x978).unwrap_or(0),h(&machine.bus.ewram,base + 0x97a).unwrap_or(0));
    true
}
pub(crate) fn keys(mem: &mut MemMap) -> Option<u16> {
    if let Some(keys) = mem.ffta_battle_mouse.roster.category_keys.take() {
        mem.iwram[..6].copy_from_slice(&keys);
    }
    if mem.ffta_battle_mouse.roster.scroll.is_some()
        || mem.ffta_battle_mouse.roster.resume.is_some()
    {
        return Some(0x3ff);
    }
    let Some(r) = mem.ffta_battle_mouse.roster.request else {
        return (Scene::read(mem).is_some() || active(mem) && h(&mem.io, 0) == Some(0x3d40))
            .then_some(0x3ff);
    };
    let scene = Scene::read(mem);
    if mem.frames > r.deadline
        || scene
            .as_ref()
            .is_none_or(|s| s.base != r.base || s.kind != r.kind && !(r.kind == 6 && s.kind == 5))
    {
        mem.ffta_battle_mouse.roster.cancel();
        return None;
    }
    let ready = r.kind != 0 || mem.ewram.get(r.base + 0x970).is_some_and(|v| v & 3 == 1);
    if ready && r.committed.is_some_and(|f| mem.frames >= f + 2) {
        mem.ffta_battle_mouse.roster.request = None;
        if r.confirm {
            mem.ffta_battle_mouse.queue_shop_key(0x3fe);
        }
    }
    Some(0x3ff)
}
pub(crate) fn entry(m: &mut Machine) {
    if !active(&m.bus) && m.bus.ffta_battle_mouse.roster.resume.is_none() {
        return;
    }
    let pc = m.cpu.regs[15];
    if pc==0x080091b4 && options_entry(m) {return;}
    if scroll_entry(m) {
        return;
    }
    if pc == 0x0808bdec
        && m.bus.rom.get(0x8bdec..0x8bdf4) == Some(&[0x70, 0xb5, 4, 0x1c, 9, 4, 10, 12])
    {
        if let Some(base) = ram(m.cpu.regs[0], 0x114) {
            if widget(&m.bus, base).is_some() {
                m.bus.ffta_battle_mouse.roster.widget = Some((base, m.bus.frames));
            }
        }
    }
    let Some(r) = m
        .bus
        .ffta_battle_mouse
        .roster
        .request
        .filter(|r| r.committed.is_none() && m.bus.frames <= r.deadline)
    else {
        return;
    };
    if r.kind == 6 && pc == 0x080853f8 {
        let Some(manager) = w(&m.bus.iwram, 0x2818).and_then(|p| ram(p, 0x3000)) else {
            return;
        };
        let ids: Vec<_> = (0..if full_inventory(&m.bus) { 8 } else { 5 })
            .filter(|i| h(&m.bus.ewram, manager + 0xc60 + i * 2).is_some_and(|v| v != 0))
            .collect();
        let Some(index) = ids.iter().position(|i| *i == r.target as usize) else {
            return;
        };
        m.bus.ewram[manager + 0xc21] = ids[(index + ids.len() - 1) % ids.len()] as u8;
        // Let the original Right-category path rebuild the destination once.
        // Choosing its predecessor skips intermediate categories entirely.
        m.bus.ffta_battle_mouse.roster.category_keys = Some(m.bus.iwram[..6].try_into().unwrap());
        for offset in [0, 2, 4] {
            m.bus.iwram[offset..offset + 2].copy_from_slice(&0x10u16.to_le_bytes());
        }
        m.bus
            .ffta_battle_mouse
            .roster
            .request
            .as_mut()
            .unwrap()
            .committed = Some(m.bus.frames);
        return;
    }
    if r.kind == 5
        && matches!(pc, 0x080873b2 | 0x0807d6a0 | 0x080905c4 | 0x0809070c)
        && m.bus.ffta_battle_mouse.roster.resume.is_none()
    {
        let Some(manager) = w(&m.bus.iwram, 0x2818).and_then(|p| ram(p, 0x2e00)) else {
            return;
        };
        let Some(list) = list(&m.bus).filter(|l| l.cursor == r.base) else {
            return;
        };
        let saved = Resume {
            regs: m.cpu.regs,
            cpsr: m.cpu.cpsr,
            keys: m.bus.iwram[..6].try_into().unwrap(),
            entry: pc,
        };
        m.bus.ffta_battle_mouse.roster.resume = Some(saved);
        let slot = (r.base - manager - 0x2d44) / 0xcc;
        m.bus.ewram[manager + slot * 0xcc + 0x2cea..manager + slot * 0xcc + 0x2cec]
            .copy_from_slice(&0x80u16.to_le_bytes());
        if let Some(descriptor) =
            w(&m.bus.ewram, manager + slot * 0xcc + 0x2d74).and_then(|p| ram(p, 28))
        {
            if let Some(records) = w(&m.bus.ewram, descriptor + 24)
                .and_then(|p| ram(p, (list.first as usize + r.target as usize + 1) * 20))
            {
                let row = records + (list.first as usize + r.target as usize) * 20;
                for (to, from) in [(0x2cec, 14), (0x2ce4, 12)] {
                    let value = h(&m.bus.ewram, row + from).unwrap_or(0);
                    m.bus.ewram[manager + slot * 0xcc + to..manager + slot * 0xcc + to + 2]
                        .copy_from_slice(&value.to_le_bytes());
                }
            }
        }
        m.cpu.regs[0] = slot as u32;
        m.cpu.regs[1] = 0x80;
        m.cpu.regs[2] = r.target.min(list.visible as u8 - 1) as u32;
        m.cpu.regs[3] = 0;
        m.cpu.regs[14] = pc | 1;
        m.cpu.regs[15] = party_publisher(&m.bus);
        m.bus
            .ffta_battle_mouse
            .roster
            .request
            .as_mut()
            .unwrap()
            .committed = Some(m.bus.frames);
        return;
    }
    if r.kind == 3
        && pc == 0x08088ab8
        && m.cpu.regs[2] == 0x02000000 + r.base as u32
        && m.bus.rom.get(0x88efe..0x88f08)
            == Some(&[0x26, 0x4b, 0xc9, 0x18, 8, 0x78, 1, 0x30, 8, 0x70])
    {
        let count = w(&m.bus.ewram, r.base + 0x1274).unwrap_or(0) as u8;
        if count == 0 || r.target >= count {
            return;
        }
        m.bus.ewram[r.base + 0x1279] = r.target;
        m.bus.ewram[r.base + 0x127a] = 0;
        m.cpu.regs[7] = 0x03002818;
        m.cpu.regs[6] = 0;
        m.cpu.regs[3] = 0x1279;
        m.cpu.regs[15] = 0x08088f1c;
        m.bus
            .ffta_battle_mouse
            .roster
            .request
            .as_mut()
            .unwrap()
            .committed = Some(m.bus.frames);
        return;
    }
    if r.kind == 4
        && pc == 0x08051782
        && m.cpu.regs[5] == 0x02000000 + r.base as u32
        && m.bus.rom.get(0x518a4..0x518ac) == Some(&[0, 0x2e, 9, 0xd0, 0x29, 0x1c, 0x98, 0x31])
    {
        let Some(manager) = w(&m.bus.ewram, r.base).and_then(|p| ram(p, 0x568)) else {
            return;
        };
        m.bus.ewram[manager + 0x562] = r.target % 6;
        m.bus.ewram[manager + 0x563] = r.target / 6;
        m.cpu.regs[6] = 1;
        m.cpu.regs[15] = 0x080518a4;
        m.bus
            .ffta_battle_mouse
            .roster
            .request
            .as_mut()
            .unwrap()
            .committed = Some(m.bus.frames);
        return;
    }
    if r.kind == 0
        && pc == 0x08075466
        && m.bus.rom.get(0x75466..0x7546e)
            == Some(&[0x21, 0x4c, 0x20, 0x68, 0x21, 0x4b, 0xc1, 0x18])
    {
        if Scene::read(&m.bus)
            .is_none_or(|s| s.base != r.base || r.target as usize >= s.bounds.len())
        {
            return;
        }
        m.bus.ewram[r.base + 0x971] = r.target % 4;
        m.bus.ewram[r.base + 0x972] = r.target / 4;
        m.bus.ewram[r.base + 0x970] |= 2;
        m.bus.ewram[r.base + 0x25] = 0;
        m.bus
            .ffta_battle_mouse
            .roster
            .request
            .as_mut()
            .unwrap()
            .committed = Some(m.bus.frames);
    }
    if r.kind == 1 && pc == 0x0808bdec && m.cpu.regs[0] == 0x02000000 + r.base as u32 {
        // Wake the original directional publisher. Its neighbour is replaced before
        // validation, so even a last->first jump invokes the complete refresh once.
        m.cpu.regs[1] = 0x80;
    }
    if r.kind == 1
        && pc == 0x0808be4e
        && m.cpu.regs[4] == 0x02000000 + r.base as u32
        && m.cpu.regs[6] == 0x02000105 + r.base as u32
        && m.bus.rom.get(0x8be4e..0x8be56) == Some(&[0x28, 0x1c, 8, 0x40, 0, 0x28, 4, 0xd0])
    {
        m.cpu.regs[5] = r.target as u32;
        m.bus
            .ffta_battle_mouse
            .roster
            .request
            .as_mut()
            .unwrap()
            .committed = Some(m.bus.frames);
    }
}

/// Shared paged list model. Bounds are derived from the owning window, never
/// translated labels. Parent lists with disabled cursors cannot steal input.
#[derive(Clone, Copy, Debug)]
pub struct List {
    pub cursor: usize,
    pub total: u16,
    pub visible: u16,
    pub first: u16,
    pub bounds: [i32; 4],
    origin: [i32; 2],
    pitch: i32,
}
impl List {
    fn scene(self, mem: &MemMap) -> Scene {
        let count = (self.total - self.first).min(self.visible);
        Scene {
            bounds: (0..count)
                .map(|i| {
                    [
                        self.bounds[0],
                        self.origin[1] + i as i32 * self.pitch,
                        self.bounds[2] - 8,
                        self.origin[1] + (i as i32 + 1) * self.pitch,
                    ]
                })
                .collect(),
            selected: mem.ewram[self.cursor + 17],
            base: self.cursor,
            kind: if w(&mem.ewram, self.cursor) == Some(0x08083961) {
                5
            } else {
                2
            },
        }
    }
}
// The Options task keeps dormant map selectors alive. Its own verified owner
// takes priority, and provides all ten rows regardless of translated labels.
fn options_list(mem:&MemMap)->Option<List> {
    if !active(mem) || w(&mem.iwram,0x2804)!=Some(0x08031601)
        || mem.rom.get(0x91b4..0x91be)!=Some(&[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46,0xe0,0xb4]) {return None;}
    let manager=ram(w(&mem.ewram,0x9188)?,0x1bd0)?;
    if w(&mem.ewram,manager+4)!=Some(4) || mem.ewram[manager]&1==0{return None;}
    let base=manager+0x2f0;let cursor=base+32;
    if w(&mem.ewram,cursor)!=Some(0x08018fc5)
        || w(&mem.ewram,cursor+4)!=Some(0x02000000+base as u32)
        || mem.ewram[cursor+16]&3!=3{return None;}
    let total=h(&mem.ewram,base+24)?;let first=h(&mem.ewram,base+26)?;
    let visible=mem.ewram[base+22]as u16;
    if total==0 || total>32 || visible==0 || visible>8 || first>=total{return None;}
    let x=mem.ewram[base+19]as i32*8;
    let y=mem.ewram[base+20]as i32*8+4;
    let right=(x+mem.ewram[base+21]as i32*8).min(216);
    Some(List{cursor,total,visible,first,bounds:[x-8,y-4,right,y+visible as i32*16],origin:[x,y],pitch:16})
}
fn options_call(mem:&mut MemMap,pc:u32,args:[u32;3])->bool {
    let stack=mem.iwram[0x7000..0x7800].to_vec();
    let mut cpu=crate::Cpu::new();cpu.cpsr|=crate::cpu::FLAG_T|crate::cpu::FLAG_I;
    cpu.regs[..3].copy_from_slice(&args);cpu.regs[13]=0x03007800;
    cpu.regs[14]=0x03007001;cpu.regs[15]=pc;
    let mut returned=false;
    for _ in 0..200000 {
        if cpu.regs[15]==0x03007000{returned=true;break;}
        crate::exec::step_hle(&mut cpu,mem);
    }
    mem.iwram[0x7000..0x7800].copy_from_slice(&stack);returned
}
fn options_entry(m:&mut Machine)->bool {
    let Some(mut list)=options_list(&m.bus)else{return false;};
    if let Some((cursor,first))=m.bus.ffta_battle_mouse.roster.scroll.filter(|p|p.0==list.cursor) {
        let base=cursor-32;let first=first.min(list.total.saturating_sub(list.visible));
        m.bus.ewram[base+26..base+28].copy_from_slice(&first.to_le_bytes());
        for row in 0..list.visible {if !options_call(&mut m.bus,0x08008ff8,[row as u32,0,0]){return false;}}
        list.first=first;publish_list(&mut m.bus,list,first,0,true);
        m.bus.ewram[cursor+8..cursor+10].copy_from_slice(&8u16.to_le_bytes());
        m.bus.ffta_battle_mouse.roster.scroll=None;
        return true;
    }
    if let Some(r)=m.bus.ffta_battle_mouse.roster.request.filter(|r|r.kind==7 && r.base==list.cursor && r.committed.is_none()) {
        if !options_call(&mut m.bus,0x08009408,[0x80,r.target as u32,0]){return false;}
        // The native directional path normally clears the departed row before
        // calling this publisher. Direct jumps bypass that step: normalize the
        // complete ring buffer so its later DMA cannot restore old highlights.
        publish_list(&mut m.bus,list,list.first,r.target,false);
        m.bus.ffta_battle_mouse.roster.request.as_mut().unwrap().committed=Some(m.bus.frames);
        m.bus.ffta_battle_mouse.roster.neutral=false;
        return true;
    }
    false
}
pub fn list(mem: &MemMap) -> Option<List> {
    if !active(mem) {
        return None;
    }
    if let Some(list)=options_list(mem){return Some(list);}
    if mem
        .ffta_battle_mouse
        .roster
        .widget
        .is_some_and(|(p, f)| mem.frames <= f + 4 && widget(mem, p).is_some())
    {
        return None;
    }
    let mut result = None;
    let (start, end, step) = if w(&mem.iwram, 0x2804) == Some(0x08072929) {
        let manager = ram(w(&mem.iwram, 0x2818)?, 0x3000)?;
        (manager + 0x2d44, manager + 0x2d44 + 0xcc * 3, 0xcc)
    } else {
        let cached = mem
            .ffta_battle_mouse
            .roster
            .list_cursor
            .load(std::sync::atomic::Ordering::Relaxed);
        if cached > 0
            && w(&mem.ewram, cached - 1) == Some(0x08018fc5)
            && mem.ewram.get(cached - 1 + 16).is_some_and(|f| f & 3 == 3)
        {
            (cached - 1, cached, 1)
        } else {
            (0x20, 0x3ffc0, 4)
        }
    };
    for cursor in (start..end).step_by(step) {
        let callback = w(&mem.ewram, cursor)?;
        if !matches!(callback, 0x08018fc5 | 0x080397ed | 0x08083961)
            || w(&mem.ewram, cursor + 4) != Some(0x02000000 + cursor as u32 - 0x20)
            || mem.ewram[cursor + 16] & 3 != 3
        {
            continue;
        }
        let base = cursor - 0x20;
        let total = h(&mem.ewram, base + 0x18)?;
        let visible = mem.ewram[base + 0x16] as u16;
        let first = h(&mem.ewram, base + 0x1a)?;
        let selected = mem.ewram[cursor + 17];
        let width = mem.ewram[base + 0x15] as i32;
        if total == 0
            || total > 256
            || visible == 0
            || visible > 12
            || first >= total
            || selected as u16 >= (total - first).min(visible)
            || width == 0
            || width > 29
        {
            continue;
        }
        let (mut x, mut y) = (
            mem.ewram[base + 0x13] as i32 * 8,
            h(&mem.ewram, cursor + 10)? as i16 as i32,
        );
        if callback == 0x08083961 {
            let Some(manager) = w(&mem.iwram, 0x2818).and_then(|p| ram(p, 0x2e00)) else {
                continue;
            };
            if cursor < manager + 0x2d44 || (cursor - manager - 0x2d44) % 0xcc != 0 {
                continue;
            }
            let owner = manager + (cursor - manager - 0x2d44) / 0xcc * 0xcc;
            if mem.ewram[owner + 0x2cde] != 6 {
                continue;
            }
            let slot = h(&mem.ewram, owner + 0x2cdc)? as usize;
            let win = 0x9e0 + slot * 0x4c;
            if win + 0x4c > mem.iwram.len() {
                continue;
            }
            x = (mem.iwram[win + 0x34] as i8 as i32 + mem.iwram[win + 0x48] as i8 as i32 - 1) * 8;
            y += (mem.iwram[win + 0x35] as i8 as i32 + mem.iwram[win + 0x49] as i8 as i32 - 1) * 8;
        }
        if callback == 0x080397ed {
            if mem.ewram[cursor + 16] & 0x80 == 0 || w(&mem.iwram, 0x2804) != Some(0x08031601) {
                continue;
            }
            let Some(manager) = ram(w(&mem.iwram, 0x2810)?, 0xc00) else {
                continue;
            };
            if base < manager + 0x9cc || (base - manager - 0x9cc) % 0xcc != 0 {
                continue;
            }
            let owner = manager + (base - manager - 0x9cc) / 0xcc * 0xcc;
            let slot = h(&mem.ewram, owner + 0x984)? as usize;
            let win = 0x9e0 + slot * 0x4c;
            if win + 0x4c > mem.iwram.len() {
                continue;
            }
            x = (mem.iwram[win + 0x34] as i8 as i32 + mem.iwram[win + 0x48] as i8 as i32 - 1) * 8
                + h(&mem.ewram, cursor + 8)? as i16 as i32;
            y += (mem.iwram[win + 0x35] as i8 as i32 + mem.iwram[win + 0x49] as i8 as i32 - 1) * 8;
        }
        let right = (x + (width + 2) * 8).min(240);
        if x < 0 || x >= right || y < 0 || y + visible as i32 * 16 > 160 {
            continue;
        }
        if result.is_some() {
            return None;
        }
        mem.ffta_battle_mouse
            .roster
            .list_cursor
            .store(cursor + 1, std::sync::atomic::Ordering::Relaxed);
        result = Some(List {
            cursor,
            total,
            visible,
            first,
            bounds: [
                (x - 8).max(0),
                (y - 4).max(0),
                right,
                (y + visible as i32 * 16).min(160),
            ],
            origin: [x, y],
            pitch: 16,
        });
    }
    result
}
fn publish_list(mem: &mut MemMap, list: List, first: u16, selected: u8, neutral: bool) {
    remember_neutral_map(mem, list);
    let base = list.cursor - 0x20;
    let width = mem.ewram[base + 0x15] as usize;
    let stride = mem.ewram[base + 0x1e] as usize;
    let rows = 2usize;
    if stride < width || stride > 64 {
        return;
    }
    let slots = mem.ewram[base + 0x17] as usize;
    if slots == 0 || slots > 32 {
        return;
    }
    let Some(source) = w(&mem.ewram, base + 12).and_then(|p| ram(p, slots * rows * width * 2))
    else {
        return;
    };
    let Some(dest) = w(&mem.ewram, base + 8) else {
        return;
    };
    let first = first.min(list.total.saturating_sub(list.visible));
    let count = (list.total - first).min(list.visible) as usize;
    let selected = (selected as usize).min(count - 1);
    // The original item/card row has a 16x16 artwork prefix, then paired
    // font tiles. Identify its atlas layout, not its current palette (which
    // may already have been recolored by a previous selection/scroll pass).
    let icon_prefix = width > 2
        && (0..slots).any(|row| {
            let p = source + row * rows * width * 2;
            let tile = |column: usize, line: usize| {
                h(&mem.ewram, p + (line * width + column) * 2).unwrap_or(0) & 1023
            };
            let start = tile(0, 0);
            [tile(1, 0), tile(0, 1), tile(1, 1), tile(2, 0), tile(2, 1)]
                == [start + 1, start + 2, start + 3, start + 4, start + 5]
        });
    // Normalize the entire packed source, so the next original publication
    // cannot restore a previously hovered row's palette.
    for row in 0..slots {
        for i in 0..rows * width {
            let p = source + (row * rows * width + i) * 2;
            let tile = h(&mem.ewram, p).unwrap();
            let artwork = icon_prefix && i % width < 2;
            if !artwork && matches!(tile >> 12, 11 | 12) {
                let tile = (tile & 0xfff)
                    | if !neutral && row == (first as usize + selected) % slots {
                        0xb000
                    } else {
                        0xc000
                    };
                mem.ewram[p..p + 2].copy_from_slice(&tile.to_le_bytes());
            }
        }
    }
    let packed: Vec<u8> = (0..count)
        .flat_map(|row| {
            let at = source + ((first as usize + row) % slots) * rows * width * 2;
            mem.ewram[at..at + rows * width * 2].to_vec()
        })
        .collect();
    let (data, start) = match dest >> 24 {
        2 => (&mut mem.ewram, (dest & 0x3ffff) as usize),
        6 => (&mut mem.vram, (dest & 0x1ffff) as usize),
        _ => return,
    };
    if start + (count * rows - 1) * stride * 2 + width * 2 > data.len() {
        return;
    }
    // Original item/card artwork is published separately with per-icon
    // D/E/F palettes. The packed text buffer does NOT contain those final
    // palettes: copying its prefix would recolor every icon visited.
    let skip = if icon_prefix { 4 } else { 0 };
    for row in 0..count * rows {
        data[start + row * stride * 2 + skip..start + row * stride * 2 + width * 2]
            .copy_from_slice(&packed[row * width * 2 + skip..(row + 1) * width * 2]);
    }
    mem.ewram[base + 0x1a..base + 0x1c].copy_from_slice(&first.to_le_bytes());
    mem.ewram[list.cursor + 17] = selected as u8;
    mem.ewram[list.cursor + 14..list.cursor + 16]
        .copy_from_slice(&(selected as u16 * list.pitch as u16).to_le_bytes());
    mem.ffta_battle_mouse.roster.neutral = neutral;
}
pub fn scroll_to(mem: &mut MemMap, first: u16) -> bool {
    if let Some(list) = gallery_list(mem) {
        let base = list.cursor - 0x998;
        let first = first.min(list.total - list.visible);
        for p in [0x994, 0x998, 0x99c] {
            mem.ewram[base + p..base + p + 2].copy_from_slice(&first.to_le_bytes());
        }
        let count = mem.ewram[base + 0x95c] as usize;
        let row = ((first as usize + 48) / 40).min((count - 1) / 4);
        let target = (row * 4 + mem.ewram[base + 0x971] as usize).min(count - 1) as u8;
        mem.ewram[base + 0x990..base + 0x994].fill(0);
        mem.ewram[base + 0x996..base + 0x998].fill(0);
        mem.ffta_battle_mouse.roster.request = Some(Request {
            base,
            target,
            kind: 0,
            confirm: false,
            deadline: mem.frames + 120,
            committed: None,
        });
        mem.ffta_battle_mouse.roster.neutral = true;
        return true;
    }
    let Some(list) = list(mem)
        .or_else(|| report_list(mem))
        .filter(|l| l.total > l.visible)
    else {
        return false;
    };
    mem.ffta_battle_mouse.roster.request = None;
    mem.ffta_battle_mouse.roster.steps = 0;
    mem.ffta_battle_mouse.roster.scroll = Some((list.cursor, first.min(list.total - list.visible)));
    remember_neutral_map(mem, list);
    mem.ffta_battle_mouse.roster.neutral = true;
    true
}
pub fn scroll_by(mem: &mut MemMap, up: bool) -> bool {
    let Some(list) = scrollbar_list(mem) else {
        return false;
    };
    scroll_to(
        mem,
        if up {
            list.first.saturating_sub(1)
        } else {
            list.first.saturating_add(1)
        },
    )
}
fn remember_neutral_map(mem: &mut MemMap, list: List) {
    let b = list.cursor.saturating_sub(32);
    let destination = w(&mem.ewram, b + 8).unwrap_or(0);
    let width = mem.ewram.get(b + 21).copied().unwrap_or(0) as usize;
    let stride = mem.ewram.get(b + 30).copied().unwrap_or(0) as usize;
    mem.ffta_battle_mouse.roster.neutral_map =
        (destination >> 24 == 6 && width > 0 && stride >= width).then_some([
            (destination & 0x1ffff) as usize,
            width * 2,
            stride * 2,
            list.visible as usize * 2,
        ]);
}
/// BG2 also contains the Clan heading. Neutral scroll colors apply only to
/// the current list's published tilemap cells, never the entire BG layer.
pub(crate) fn neutral_tile(mem: &MemMap, offset: usize) -> bool {
    if !unselected(mem) {
        return false;
    }
    let Some([start, width, stride, rows]) = mem.ffta_battle_mouse.roster.neutral_map else {
        return false;
    };
    let Some(relative) = offset.checked_sub(start) else {
        return false;
    };
    relative / stride < rows && relative % stride < width
}
pub(crate) fn unselected(mem: &MemMap) -> bool {
    active(mem) && mem.ffta_battle_mouse.roster.neutral
}

pub fn scrollbar_list(mem: &MemMap) -> Option<List> {
    list(mem)
        .or_else(|| gallery_list(mem))
        .or_else(|| report_list(mem))
        .map(|mut list| {
            // Area List's cursor Y includes the smooth row-scroll offset.
            // Its scrollbar belongs to the window, not that moving row.
            if w(&mem.ewram,list.cursor)==Some(0x080397ed) {
                if let Some(manager)=w(&mem.iwram,0x2810).and_then(|p|ram(p,0xc00)) {
                    if let Some(relative)=list.cursor.checked_sub(manager+0x9ec) {
                        let owner=manager+relative/0xcc*0xcc;
                        if let Some(slot)=h(&mem.ewram,owner+0x984) {
                            let win=0x9e0+slot as usize*0x4c;
                            if let Some(window)=mem.iwram.get(win..win+0x4c) {
                                let top=window[0x35]as i8 as i32*8;
                                let bottom=top+window[0x39]as i32*8;
                                if top>=0 && bottom<=160 && bottom-top>=32 {
                                    // Cursor X is animated too. Place the track
                                    // in its own gutter beyond the frame, away
                                    // from the original bouncing page arrow.
                                    let right=(window[0x34]as i8 as i32+window[0x38]as i32)*8;
                                    list.bounds[0]=right;
                                    list.bounds[2]=(right+8).min(240);
                                    list.bounds[1]=top+16;
                                    list.bounds[3]=bottom-12;
                                }
                            }
                        }
                    }
                }
            }
            list
        })
        .filter(|l| mem.ffta_battle_mouse.roster.closed != Some(l.cursor))
}
pub(crate) fn dismiss(mem: &mut MemMap) {
    mem.ffta_battle_mouse.roster.closed = scrollbar_list(mem).map(|l| l.cursor);
}
pub(crate) fn tick(mem: &mut MemMap) {
    if mem.ffta_battle_mouse.roster.closed.is_some_and(|p| {
        list(mem)
            .or_else(|| gallery_list(mem))
            .or_else(|| report_list(mem))
            .is_none_or(|l| l.cursor != p)
    }) {
        mem.ffta_battle_mouse.roster.closed = None;
    }
}

// Reuse the original row loader repeatedly inside one guest update. Its
// circular tile buffer contains visible rows only; copying from an assumed
// full backing array would produce blank pages. Registers and keys are restored.
fn scroll_entry(m: &mut Machine) -> bool {
    let pc = m.cpu.regs[15];
    if !matches!(
        pc,
        0x08031600
            | 0x08051160
            | 0x08086fd8
            | 0x080853f8
            | 0x08051782
            | 0x080873b2
            | 0x0807d6a0
            | 0x080905c4
            | 0x0809070c
            | 0x08144f04
    ) {
        return false;
    }

    let resume = m.bus.ffta_battle_mouse.roster.resume;
    if pc == 0x08144f04 && resume.is_some() && m.cpu.regs[0] == 0x6d {
        m.cpu.regs[15] = m.cpu.regs[14] & !1;
        return true;
    }
    if let Some(saved) = resume.filter(|s| s.entry == pc) {
        m.cpu.regs = saved.regs;
        m.cpu.cpsr = saved.cpsr;
        m.bus.iwram[..6].copy_from_slice(&saved.keys);
        m.bus.ffta_battle_mouse.roster.resume = None;
    }
    let Some((cursor, target)) = m.bus.ffta_battle_mouse.roster.scroll else {
        return false;
    };
    if let Some(scene) = report(&m.bus).filter(|s| s.base == cursor) {
        if pc == 0x08051782 && m.cpu.regs[5] == 0x02000000 + scene.base as u32 {
            if let Some(manager) = w(&m.bus.ewram, cursor).and_then(|p| ram(p, 0x568)) {
                m.bus.ewram[manager + 0x560] = target.min(16) as u8;
                m.cpu.regs[0] = 0xfffd;
                m.cpu.regs[15] = 0x08051844;
                m.bus.ffta_battle_mouse.roster.scroll = None;
                return true;
            }
        }
        return false;
    }
    let Some(list) = list(&m.bus).filter(|l| l.cursor == cursor) else {
        m.bus.ffta_battle_mouse.roster.scroll = None;
        return false;
    };
    if options_list(&m.bus).is_some_and(|l|l.cursor==cursor){return false;}
    if !matches!(pc, 0x08031600 | 0x08051160 | 0x08086fd8 | 0x080853f8) {
        return false;
    }
    if m.bus.ffta_battle_mouse.roster.steps >= 256 {
        m.bus.ffta_battle_mouse.roster.scroll = None;
        return false;
    }
    m.bus.ffta_battle_mouse.roster.steps += 1;
    if list.first == target {
        publish_list(&mut m.bus, list, list.first, 0, true);
        m.bus.ffta_battle_mouse.roster.scroll = None;
        return false;
    }
    let down = target > list.first;
    if matches!(pc, 0x08086fd8 | 0x080853f8) && w(&m.bus.ewram, cursor) == Some(0x08083961) {
        let Some(manager) = w(&m.bus.iwram, 0x2818).and_then(|p| ram(p, 0x2e00)) else {
            return false;
        };
        let slot = (cursor - manager - 0x2d44) / 0xcc;
        if m.cpu.regs[0] != slot as u32 {
            return false;
        }
        m.bus.ffta_battle_mouse.roster.resume = Some(Resume {
            regs: m.cpu.regs,
            cpsr: m.cpu.cpsr,
            keys: m.bus.iwram[..6].try_into().unwrap(),
            entry: pc,
        });
        m.cpu.regs[0] = slot as u32;
        m.cpu.regs[1] = if down { 0x80 } else { 0x40 };
        m.cpu.regs[2] = if down { list.visible as u32 } else { 255 };
        m.cpu.regs[3] = 1;
        m.cpu.regs[14] = pc | 1;
        m.cpu.regs[15] = party_publisher(&m.bus);
        return true;
    }
    if pc == 0x08051160 {
        if m.cpu.regs[0] != 0x02000000 + cursor as u32 - 0x68 {
            return false;
        }
        let saved = Resume {
            regs: m.cpu.regs,
            cpsr: m.cpu.cpsr,
            keys: m.bus.iwram[..6].try_into().unwrap(),
            entry: pc,
        };
        m.bus.ffta_battle_mouse.roster.resume = Some(saved);
        m.cpu.regs[0] = 0x02000000 + cursor as u32 - 0x68;
        m.cpu.regs[1] = if down { 0x80 } else { 0x40 };
        m.cpu.regs[2] = if down { list.visible as u32 } else { 255 };
        m.cpu.regs[3] = 1;
        m.cpu.regs[14] = pc | 1;
        m.cpu.regs[15] = 0x08052ac4;
        return true;
    }
    if pc == 0x08031600 && w(&m.bus.iwram, 0x2804) == Some(0x08031601) {
        let saved = Resume {
            regs: m.cpu.regs,
            cpsr: m.cpu.cpsr,
            keys: m.bus.iwram[..6].try_into().unwrap(),
            entry: pc,
        };
        m.bus.ffta_battle_mouse.roster.resume = Some(saved);
        let Some(manager) = w(&m.bus.iwram, 0x2810).and_then(|p| ram(p, 0xc00)) else {
            return false;
        };
        m.cpu.regs[0] = ((cursor - 0x20 - manager - 0x9cc) / 0xcc) as u32;
        m.cpu.regs[1] = if down { 0x80 } else { 0x40 };
        m.cpu.regs[2] = if down { list.visible as u32 } else { 255 };
        m.cpu.regs[3] = 1;
        m.cpu.regs[14] = pc | 1;
        m.cpu.regs[15] = 0x08042478;
        return true;
    }
    false
}

fn jobs(mem: &MemMap) -> Option<Scene> {
    if w(&mem.iwram, 0x2804) != Some(0x08072929) {
        return None;
    }
    let states = w(&mem.iwram, 0x273c)?.checked_sub(0x03000000)? as usize;
    let table = w(&mem.iwram, 0x2738)?.checked_sub(0x03000000)? as usize;
    if w(&mem.iwram, table + 8) != Some(0x08088329) || *mem.iwram.get(states + 2)? != 4 {
        return None;
    }
    let manager = ram(w(&mem.iwram, 0x2818)?, 0x1400)?;
    let count = w(&mem.ewram, manager + 0x1274)? as usize;
    let selected = mem.ewram[manager + 0x1279];
    if count == 0 || count > 24 || selected as usize >= count {
        return None;
    }
    let objects = ram(w(&mem.iwram, 0xe34)?, count * 28)?;
    let mut bounds = vec![[0; 4]; count];
    for slot in 0..count {
        let p = objects + slot * 28;
        let tile = h(&mem.ewram, p + 22)? as usize;
        let index = if tile >= 664 && (tile - 664) % 20 == 0 {
            8 + (tile - 664) / 20
        } else if tile >= 128 && (tile - 128) % 20 == 0 {
            (tile - 128) / 20
        } else {
            continue;
        };
        if index >= count {
            continue;
        }
        let index = (count - index) % count;
        let x = mem.ewram[p + 16] as i32;
        let y = mem.ewram[p + 17] as i32;
        bounds[index] = [
            (x - 11).max(112),
            (y - 30).max(52),
            (x + 11).min(240),
            (y + 3).min(148),
        ];
    }
    Some(Scene {
        bounds,
        selected,
        base: manager,
        kind: 3,
    })
}
fn report(mem: &MemMap) -> Option<Scene> {
    if w(&mem.iwram, 0x2804) != Some(0x0804e849) {
        return None;
    }
    // The verified report builder uses the global Clan model. Its embedded
    // child, rather than a search through all RAM, identifies this selector.
    let manager = ram(w(&mem.ewram, 0x1f548)?, 0x1500)?;
    let node = manager + 0x1444;
    let base = node + 24;
    if w(&mem.ewram, node) != Some(0x0805086d)
        || w(&mem.ewram, base) != Some(0x02000000 + manager as u32)
        || mem.ewram[base + 8] != 8
        || mem.ewram[base + 11] == 0
        || mem.ewram[manager + 0x561] != 17
    {
        return None;
    }
    let page = mem.ewram[manager + 0x560];
    let x = mem.ewram[manager + 0x562];
    let y = mem.ewram[manager + 0x563];
    if page >= 17 || x >= 6 || y >= 3 {
        return None;
    }
    let count = if page == 16 { 12 } else { 18 };
    Some(Scene {
        bounds: (0..count)
            .map(|i| {
                [
                    16 + i % 6 * 32,
                    60 + i / 6 * 32,
                    48 + i % 6 * 32,
                    92 + i / 6 * 32,
                ]
            })
            .collect(),
        selected: x + y * 6,
        base,
        kind: 4,
    })
}

fn gallery_list(mem: &MemMap) -> Option<List> {
    let scene = Scene::read(mem)?;
    if scene.kind != 0 {
        return None;
    }
    let rows = (scene.bounds.len() + 3) / 4;
    let maximum = (rows as i32 * 40 - 128).max(0) as u16;
    (maximum > 0).then_some(List {
        cursor: scene.base + 0x998,
        total: 112 + maximum,
        visible: 112,
        first: h(&mem.ewram, scene.base + 0x998)?.min(maximum),
        bounds: [16, 16, 240, 128],
        origin: [16, 16],
        pitch: 1,
    })
}
fn report_list(mem: &MemMap) -> Option<List> {
    let scene = report(mem)?;
    let manager = ram(w(&mem.ewram, scene.base)?, 0x568)?;
    Some(List {
        cursor: scene.base,
        total: 17,
        visible: 1,
        first: mem.ewram[manager + 0x560] as u16,
        bounds: [8, 56, 232, 160],
        origin: [16, 60],
        pitch: 32,
    })
}

fn party_publisher(mem: &MemMap) -> u32 {
    let table = w(&mem.iwram, 0x2738).unwrap_or(0).wrapping_sub(0x03000000) as usize;
    if w(&mem.iwram, table + 12) == Some(0x0807d005) || w(&mem.iwram, table + 8) == Some(0x08090395)
    {
        0x08084b50
    } else {
        0x08086ed8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> Machine {
        let mut m = Machine::new(vec![0; 0x100]);
        m.bus.ffta_battle_mouse.enabled = true;
        m.bus.ffta_movement.europe_profile = true;
        m.bus.io[9] = 8;
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0804e849u32.to_le_bytes());
        m
    }
    #[test]
    fn default_widget_row_is_published_before_first_click_confirms() {
        let mut m=model();let base=0x1000;
        m.bus.ewram[base]=1;
        m.bus.ewram[base+0x104]=2;
        m.bus.ewram[base+5]=16;m.bus.ewram[base+6]=32;
        m.bus.ewram[base+13]=16;m.bus.ewram[base+14]=48;
        m.bus.ffta_battle_mouse.roster.widget=Some((base,0));
        assert!(aim(&mut m.bus,20.,32.,false));
        assert!(aim(&mut m.bus,20.,32.,true));
        let r=m.bus.ffta_battle_mouse.roster.request.unwrap();
        assert_eq!(r.target,0);assert!(r.confirm);assert!(r.committed.is_none());
        m.bus.frames=2;keys(&mut m.bus);
        assert!(m.bus.ffta_battle_mouse.roster.request.is_some());
        m.bus.ffta_battle_mouse.roster.request.as_mut().unwrap().committed=Some(2);
        m.bus.ffta_battle_mouse.roster.widget=Some((base,4));
        m.bus.frames=4;keys(&mut m.bus);
        assert_eq!(crate::ffta_battle_mouse::keys(&mut m.bus,0x3ff),0x3fe);
    }
    fn make_list(m: &mut Machine, cursor: usize, flags: u8) {
        let b = cursor - 32;
        for (p, v) in [
            (cursor, 0x08018fc5u32),
            (cursor + 4, 0x02000000 + b as u32),
            (b + 8, 0x06004000),
            (b + 12, 0x02003000),
        ] {
            m.bus.ewram[p..p + 4].copy_from_slice(&v.to_le_bytes());
        }
        for (p, v) in [
            (b + 0x13, 2),
            (b + 0x15, 8),
            (b + 0x16, 5),
            (b + 0x17, 6),
            (b + 0x18, 20),
            (b + 0x1e, 10),
            (cursor + 10, 40),
            (cursor + 16, flags),
        ] {
            m.bus.ewram[p] = v;
        }
        for row in 0..6 {
            for i in 0..32 {
                let p = 0x3000 + row * 32 + i;
                m.bus.ewram[p] = 0;
            }
        }
        for row in 0..6 {
            for tile in 0..16 {
                let p = 0x3000 + (row * 16 + tile) * 2;
                m.bus.ewram[p..p + 2]
                    .copy_from_slice(&(0xc000u16 + 100 + row as u16).to_le_bytes());
            }
        }
    }
    #[test]
    fn options_owner_beats_dormant_map_lists_and_keeps_bounded_targets() {
        let mut m=model();let manager=0x10000usize;let cursor=manager+0x310;
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08031601u32.to_le_bytes());
        m.bus.rom.resize(0x9200,0);
        m.bus.rom[0x91b4..0x91be].copy_from_slice(&[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46,0xe0,0xb4]);
        m.bus.ewram[0x9188..0x918c].copy_from_slice(&(0x02000000+manager as u32).to_le_bytes());
        m.bus.ewram[manager]=1;m.bus.ewram[manager+4]=4;
        make_list(&mut m,0x2000,3);make_list(&mut m,cursor,7);
        let base=cursor-32;
        for (o,v)in[(19,4),(20,2),(21,22),(22,6),(24,10)]{m.bus.ewram[base+o]=v;}
        let scene=Scene::read(&m.bus).unwrap();
        assert_eq!(scene.kind,7);assert_eq!(scene.base,cursor);
        assert_eq!(scene.hit(24.,20.),Some(0));assert_eq!(scene.hit(192.,72.),Some(3));
        assert_eq!(scene.hit(16.,72.),None);assert_eq!(scene.hit(212.,72.),None);
        assert_eq!(scene.hit(40.,120.),None);
        assert!(aim(&mut m.bus,192.,72.,false));
        let r=m.bus.ffta_battle_mouse.roster.request.unwrap();
        assert_eq!(r.target,3);assert!(!r.confirm);assert!(r.committed.is_none());
        assert!(scroll_to(&mut m.bus,20));
        assert_eq!(m.bus.ffta_battle_mouse.roster.scroll,Some((cursor,4)));
        let bar=scrollbar_list(&m.bus).unwrap();assert_eq!(bar.bounds,[24,16,208,116]);
        m.bus.ewram[manager+4]=6;assert!(options_list(&m.bus).is_none());
    }
    #[test]
    fn area_scrollbar_stays_in_window_while_rows_animate() {
        let mut m=model();let manager=0x1000usize;let cursor=manager+0x9ec;
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08031601u32.to_le_bytes());
        m.bus.iwram[0x2810..0x2814].copy_from_slice(&0x02001000u32.to_le_bytes());
        make_list(&mut m,cursor,0x83);
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&0x080397edu32.to_le_bytes());
        m.bus.iwram[0x9e0+0x34]=11;m.bus.iwram[0x9e0+0x35]=3;
        m.bus.iwram[0x9e0+0x38]=9;m.bus.iwram[0x9e0+0x39]=13;
        m.bus.iwram[0x9e0+0x48]=1;m.bus.iwram[0x9e0+0x49]=1;
        for y in [16u16,20,24,28] {
            m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&y.to_le_bytes());
            m.bus.ewram[cursor+8..cursor+10].copy_from_slice(&(y-16).to_le_bytes());
            let moving=list(&m.bus).unwrap();assert_eq!(moving.origin[1],24+y as i32);
            let stable=scrollbar_list(&m.bus).unwrap();
            assert_eq!(stable.bounds,[160,40,168,116]);
        }
    }
    #[test]
    fn disabled_parent_does_not_steal_child_selection() {
        let mut m = model();
        make_list(&mut m, 0x2000, 13);
        make_list(&mut m, 0x2100, 3);
        let scene = Scene::read(&m.bus).unwrap();
        assert_eq!(scene.base, 0x2100);
        assert_eq!(scene.hit(20., scene.bounds[3][1] as f32 + 4.), Some(3));
        assert_eq!(scene.hit(-1., 70.), None);
        assert_eq!(scene.hit(239., 70.), None);
    }
    #[test]
    fn circular_backing_is_repainted_without_old_highlights() {
        let mut m = model();
        make_list(&mut m, 0x2000, 3);
        let mut l = list(&m.bus).unwrap();
        l.first = 7;
        publish_list(&mut m.bus, l, 7, 3, false);
        for row in 0..5 {
            let tile = h(&m.bus.vram, 0x4000 + row * 2 * 10 * 2).unwrap();
            assert_eq!(tile & 0xfff, (100 + (7 + row) % 6) as u16);
            assert_eq!(tile >> 12, if row == 3 { 11 } else { 12 });
        }
        publish_list(&mut m.bus, l, 7, 0, true);
        for row in 0..5 {
            assert_eq!(h(&m.bus.vram, 0x4000 + row * 2 * 10 * 2).unwrap() >> 12, 12);
        }
    }
    #[test]
    fn back_hides_bar_immediately_and_mode_switch_disables_targets() {
        let mut m = model();
        make_list(&mut m, 0x2000, 3);
        assert!(scrollbar_list(&m.bus).is_some());
        dismiss(&mut m.bus);
        assert!(scrollbar_list(&m.bus).is_none());
        m.bus.ffta_battle_mouse.enabled = false;
        let before = m.bus.ewram.clone();
        assert!(!aim(&mut m.bus, 24., 50., true));
        assert_eq!(m.bus.ewram, before);
    }
    #[test]
    fn text_hover_and_scroll_preserve_separately_published_artwork() {
        let mut m = model();
        make_list(&mut m, 0x2000, 3);
        for row in 0..6 {
            let start = 0x100u16 + row as u16 * 32;
            for (line, values) in [
                (0, [start, start + 1, start + 4]),
                (1, [start + 2, start + 3, start + 5]),
            ] {
                for (column, tile) in values.into_iter().enumerate() {
                    let p = 0x3000 + (row * 16 + line * 8 + column) * 2;
                    m.bus.ewram[p..p + 2].copy_from_slice(&(0xc000 | tile).to_le_bytes());
                }
            }
        }
        for row in 0..5 {
            for line in 0..2 {
                for column in 0..2 {
                    let at = 0x4000 + (row * 2 + line) * 20 + column * 2;
                    let artwork = (((13 + row % 3) as u16) << 12)
                        | ((row * 4 + line * 2 + column + 1) as u16);
                    m.bus.vram[at..at + 2].copy_from_slice(&artwork.to_le_bytes());
                }
            }
        }
        let native = m.bus.vram.clone();
        let l = list(&m.bus).unwrap();
        for (selected, neutral) in [(3, false), (1, false), (0, true), (4, false), (0, false)] {
            publish_list(&mut m.bus, l, 0, selected, neutral);
            for row in 0..5 {
                for line in 0..2 {
                    for column in 0..2 {
                        let at = 0x4000 + (row * 2 + line) * 20 + column * 2;
                        assert_eq!(&m.bus.vram[at..at + 2], &native[at..at + 2]);
                    }
                }
                let at = 0x4004 + row * 40;
                assert_eq!(
                    h(&m.bus.vram, at).unwrap() >> 12,
                    if !neutral && row == selected as usize {
                        11
                    } else {
                        12
                    }
                );
            }
        }
    }
    fn gallery(count: u8) -> Machine {
        let mut m = model();
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08072929u32.to_le_bytes());
        m.bus.iwram[0x2818..0x281c].copy_from_slice(&0x02010000u32.to_le_bytes());
        m.bus.iwram[0x273c..0x2740].copy_from_slice(&0x03001000u32.to_le_bytes());
        m.bus.iwram[0x1000] = 3;
        m.bus.ewram[0x1095c] = count;
        m.bus.ewram[0x10970] = 1;
        for index in 0..count as usize {
            let p = 0x109fa + index * 20;
            m.bus.ewram[p..p + 2].copy_from_slice(&((index % 4 * 40) as u16).to_le_bytes());
            m.bus.ewram[p + 2..p + 4]
                .copy_from_slice(&((index / 4 * 20 + 16) as u16).to_le_bytes());
        }
        m
    }
    #[test]
    fn gallery_targets_follow_live_roster_instead_of_character_ids() {
        for count in [1, 7, 20, 24] {
            let m = gallery(count);
            let scene = Scene::read(&m.bus).unwrap();
            assert_eq!(scene.bounds.len(), count as usize);
            let b = scene.bounds[count as usize - 1];
            assert_eq!(
                scene.hit((b[0] + b[2]) as f32 / 2., (b[1] + b[3]) as f32 / 2.),
                Some(count - 1)
            );
        }
    }
    #[test]
    fn gallery_hover_marks_without_confirming_and_click_opens_same_troop() {
        let mut m = gallery(20);
        assert!(aim(&mut m.bus, 52., 24., false));
        let hover = m.bus.ffta_battle_mouse.roster.request.unwrap();
        assert_eq!(hover.target, 1);
        assert!(!hover.confirm);
        // Model completion of the original gallery publisher.
        m.bus.ewram[0x10986] = 1;
        m.bus
            .ffta_battle_mouse
            .roster
            .request
            .as_mut()
            .unwrap()
            .committed = Some(0);
        m.bus.frames = 3;
        keys(&mut m.bus);
        assert_eq!(crate::ffta_battle_mouse::keys(&mut m.bus, 0x3ff), 0x3ff);
        assert!(aim(&mut m.bus, 52., 24., true));
        assert!(m.bus.ffta_battle_mouse.roster.request.unwrap().confirm);
        // Motion over another troop cannot redirect an accepted click.
        assert!(aim(&mut m.bus, 92., 24., false));
        assert_eq!(m.bus.ffta_battle_mouse.roster.request.unwrap().target, 1);
        m.bus.frames = 6;
        keys(&mut m.bus);
        assert_eq!(crate::ffta_battle_mouse::keys(&mut m.bus, 0x3ff), 0x3fe);
    }
    #[test]
    fn gallery_click_waits_for_original_selector_before_confirming() {
        let mut m = gallery(7);
        assert!(aim(&mut m.bus, 12., 24., true));
        m.bus.frames = 3;
        m.bus.ewram[0x10970] = 3;
        keys(&mut m.bus);
        assert!(m.bus.ffta_battle_mouse.roster.request.is_some());
        m.bus.ewram[0x10970] = 1;
        keys(&mut m.bus);
        assert!(m.bus.ffta_battle_mouse.roster.request.is_none());
        assert_eq!(crate::ffta_battle_mouse::keys(&mut m.bus, 0x3ff), 0x3fe);
    }
    #[test]
    fn scroll_neutralization_excludes_clan_heading_and_other_menu_cells() {
        let mut m = model();
        make_list(&mut m, 0x2000, 3);
        assert!(scroll_to(&mut m.bus, 4));
        assert!(neutral_tile(&m.bus, 0x4000));
        assert!(neutral_tile(&m.bus, 0x400e));
        assert!(neutral_tile(&m.bus, 0x4000 + 9 * 20));
        assert!(!neutral_tile(&m.bus, 0x3000)); // heading in the same BG layer
        assert!(!neutral_tile(&m.bus, 0x4010)); // neighboring menu column
        assert!(!neutral_tile(&m.bus, 0x4000 + 10 * 20)); // below the visible list
        let l = list(&m.bus).unwrap();
        publish_list(&mut m.bus, l, 0, 2, false);
        assert!(!neutral_tile(&m.bus, 0x4000));
    }
    #[test]
    fn scrollbar_tracks_requested_page_without_confirming() {
        let mut m = model();
        make_list(&mut m, 0x2000, 3);
        assert!(scroll_to(&mut m.bus, 99));
        assert_eq!(m.bus.ffta_battle_mouse.roster.scroll, Some((0x2000, 15)));
        assert!(m.bus.ffta_battle_mouse.roster.request.is_none());
        assert!(unselected(&m.bus));
    }
}

fn full_inventory(mem: &MemMap) -> bool {
    let table = w(&mem.iwram, 0x2738).unwrap_or(0).wrapping_sub(0x03000000) as usize;
    w(&mem.iwram, table + 8) == Some(0x08090395)
}
