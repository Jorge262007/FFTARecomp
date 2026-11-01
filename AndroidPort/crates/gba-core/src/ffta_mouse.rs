//! Experimental semantic hit layer. Direct verified selector state; no ROM patching.
use crate::{
    ffta_rom::{self, Version},
    interaction_grid::{EventId, InteractionGrid, Region},
    mem::{self, MemMap},
};
const LANGUAGE_CONTEXT: u16 = 1;
const RELEASED: u16 = 0x3ff;
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub index: u8,
    pub bounds: [f32; 4],
}
#[derive(Clone, Debug)]
pub struct LanguageScene {
    pub selected: u8,
    pub targets: Vec<Target>,
    selection_offset: usize,
    base: u8,
    confirm_key: u16,
}
fn half(data: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(o..o + 2)?.try_into().ok()?))
}
fn name_profile(mem:&MemMap)->Option<(usize,u8)> {
    if mem.rom.get(0x12e81a..0x12e81e)==Some(&[1,0x30,0x20,0x70])
        && mem.rom.get(0x12e6a0..0x12e6a4)==Some(&[0xfe,0xd2,0x3a,8]) {Some((0x3ad2fe,3))}
    else if mem.rom.get(0x12ac2e..0x12ac32)==Some(&[1,0x30,0x20,0x70])
        && mem.rom.get(0x12aab4..0x12aab8)==Some(&[0x20,0x83,0x3a,8]) {Some((0x3a8320,2))}
    else {None}
}
/// Original name-entry task, including its confirmation cursor. Read-only and
/// independent of whether experimental mouse/touch input is enabled.
pub fn name_entry_active(mem:&MemMap)->bool {
    if mem.iwram.get(0x27d1)!=Some(&2) || half(&mem.io,0)!=Some(0x1f40)
        || half(&mem.io,8)!=Some(0x4c03) || half(&mem.io,10)!=Some(0x4e02)
        || name_profile(mem).is_none(){return false;}
    // Cursor OAM changes during confirmation; the name frame stays allocated.
    let frame=mem.oam.chunks_exact(8).any(|o|{
        let a0=half(o,0).unwrap();let a2=half(o,4).unwrap();
        a0&0x300!=0x200 && a2&0xfc00==0x5400 && a2&1023==0x2d0
    });
    frame || match half(&mem.oam,4){
        Some(0x120)=>mem.ewram.get(0x166db)==Some(&3),
        // Story questions reuse the same interpreter pointer and choice dot.
        // Only the live name frame may classify a confirmation as name entry.
        Some(0x125..=0x127)=>false,
        _=>mem.ffta_dialogue.naming_phase==3 && half(&mem.io,24).is_some_and(|s|s&511!=0),
    }
}
impl LanguageScene {
    fn read(mem: &MemMap) -> Option<Self> {
        Self::startup(mem)
            .or_else(|| Self::frontend(mem))
            .or_else(|| Self::keyboard(mem))
            .or_else(|| Self::name_confirmation(mem))
    }
    fn startup(mem: &MemMap) -> Option<Self> {
        if half(&mem.io, 0)? != 0x1140 || half(&mem.io, 8)? != 0x3000 {
            return None;
        }
        let pointer = u32::from_le_bytes(mem.ewram.get(0x1c3d4..0x1c3d8)?.try_into().ok()?);
        let offset = pointer.checked_sub(0x02000000)? as usize;
        let selected = *mem.ewram.get(offset.checked_add(2)?)?;
        if selected >= 5 {
            return None;
        }
        if half(&mem.oam, 4)? != 0x308c || half(&mem.oam, 12)? != 0x3090 {
            return None;
        }
        let mut targets = [Target {
            index: 0,
            bounds: [0.; 4],
        }; 5];
        for i in 0..5 {
            let flag = (2 + i * 3) * 8;
            let text = flag + 8;
            let tail = flag + 16;
            let y = 36 + i * 18;
            if half(&mem.oam, flag)? != 28 + i as u16 * 18
                || half(&mem.oam, flag + 2)? != 0x8054
                || half(&mem.oam, flag + 4)? != 0x2000 + i as u16 * 16
                || half(&mem.oam, text)? != 0x4000 + y as u16
                || half(&mem.oam, text + 2)? != 0x8070
                || half(&mem.oam, text + 4)? & 0x0fff != 0x50 + i as u16 * 12
                || half(&mem.oam, tail)? != y as u16
                || half(&mem.oam, tail + 2)? != 0x4090
                || half(&mem.oam, tail + 4)? & 0x0fff != 0x58 + i as u16 * 12
            {
                return None;
            }
            targets[i] = Target {
                index: i as u8,
                bounds: [84., y as f32 - 1., 160., y as f32 + 17.],
            };
        }
        Some(Self {
            selected,
            targets: targets.to_vec(),
            selection_offset: offset + 2,
            base: 0,
            confirm_key: 0x3fe,
        })
    }
    fn frontend(mem: &MemMap) -> Option<Self> {
        // Region-specific title task and descriptor, never inferred from battle tiles.
        let usa=mem.rom.get(0x136084..0x136090)==Some(&[0xf0,0xb5,0x4f,0x46,0x46,0x46,0xc0,0xb4,0x82,0xb0,0x0f,0x1c]);
        let (manager_expected,task,handler,selector,load,mode_offset)=if usa {
            (0xf3dc,0xf3bc,0x08136085,0x136a78,0x89072,0x8993)
        }else{(0xf3d4,0xf3b8,0x08139d29,0x13a734,0x8be72,0x8994)};
        if !matches!(mem.iwram.get(0x27d1),Some(1|4|11)) {
            return None;
        }
        let word = |o| {
            Some(u32::from_le_bytes(
                mem.ewram.get(o..o + 4)?.try_into().ok()?,
            ))
        };
        let title_manager=word(0xf3b0)?.checked_sub(0x02000000).map(|v|v as usize);
        if matches!(mem.iwram[0x27d1],4|11) {
            // The shared save task owns its manager through IWRAM. On the
            // world map f3b0 contains a BG address, not the title manager.
            let task_pointer=u32::from_le_bytes(mem.iwram.get(0x2804..0x2808)?.try_into().ok()?);
            let shared=task_pointer==0x0813dd75
                && mem.rom.get(0x13dd74..0x13dd7c)==Some(&[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46]);
            let manager=if shared {
                u32::from_le_bytes(mem.iwram.get(0x27d8..0x27dc)?.try_into().ok()?)
                    .checked_sub(0x02000000)? as usize
            }else{title_manager.filter(|m|*m==manager_expected)?};
            if manager&3!=0 || manager.checked_add(0x2270)?>mem.ewram.len(){return None;}
            if half(&mem.io, 0)? != 0x1f40
                || half(&mem.io, 8)? != 0xad0b
                || half(&mem.io, 10)? != 0xac09
                || mem.ewram.get(manager + 0x2158 + 0x104) != Some(&2)
                || mem.rom.get(load..load+4) != Some(&[0x28, 0x40, 0x30, 0x70])
            {
                return None;
            }
            let offset = manager + 0x2158 + 0x105;
            let selected = *mem.ewram.get(offset)?;
            if selected >= 2 {
                return None;
            }
            return Some(Self {
                selected,
                selection_offset: offset,
                base: if mem.iwram[0x27d1]==11{60}else{56},
                confirm_key: 0x3fe,
                targets: (0..2)
                    .map(|i| Target {
                        index: if mem.iwram[0x27d1]==11{60+i}else{56+i},
                        bounds: if mem.iwram[0x27d1]==11 {
                            [16.+112.*i as f32,48.,120.+112.*i as f32,148.]
                        }else{[64.,68.+16.*i as f32,if shared{160.}else{169.},84.+16.*i as f32]},
                    })
                    .collect(),
            });
        }
        let manager=title_manager.filter(|m|*m==manager_expected)?;
        if half(&mem.io, 0)? != 0xf740
            || half(&mem.io, 8)? != 0x3d83
            || half(&mem.io, 10)? != 0x3e0a
            || word(task+4)? != handler
        {
            return None;
        }
        if word(task)? == 5 {
            return Some(Self {
                selected: 0,
                selection_offset: usize::MAX,
                base: 64,
                confirm_key: 0x3f7,
                targets: vec![Target {
                    index: 64,
                    bounds: [62., 106., 180., 118.],
                }],
            });
        }
        if word(task)? != 7 || mem.rom.get(selector..selector+4) != Some(&[1, 0x30, 8, 0x70]) {
            return None;
        }
        let mode = *mem.ewram.get(manager + mode_offset)?;
        if usa && mode>1{return None;}
        let (base, count, offset, x, y, width, step) = match mode {
            0 => (32, if usa{3}else{4}, 0x8962, 8., 72., 104., 16.),
            1 => (40, 3, 0x8963, 150., 80., 50., 16.),
            2 => (48, 5, 0x8964, 156., 69., 76., 13.),
            _ => return None,
        };
        let selected = *mem.ewram.get(manager + offset)?;
        if selected >= count {
            return None;
        }
        Some(Self {
            selected,
            selection_offset: manager + offset,
            base,
            confirm_key: 0x3fe,
            targets: (0..count)
                .map(|i| Target {
                    index: base + i,
                    bounds: {
                        let top=y+step*i as f32+if usa && mode==0 && i==2{8.}else{0.};
                        // Target the eight-pixel label, not the entire row
                        // spacing: empty space between options is not a hit.
                        let label_width=if mode==0 {
                            [64.,72.,72.,64.][i as usize]
                        }else{(width as f32).min(64.)};
                        [x+if mode==0{16.}else{0.},top+2.,x+label_width,top+10.]
                    },
                })
                .collect(),
        })
    }
    fn keyboard(mem: &MemMap) -> Option<Self> {
        let (rows,pages)=name_profile(mem)?;
        if mem.iwram.get(0x27d1) != Some(&2)
            || half(&mem.io, 0)? != 0x1f40
            || half(&mem.io, 8)? != 0x4c03
            || half(&mem.io, 10)? != 0x4e02
            || half(&mem.oam, 4)? != 0x120
            || mem.ewram.get(0x166db) != Some(&3)
        {
            return None;
        }
        let page = *mem.ewram.get(0x166da)?;
        let col = *mem.ewram.get(0x166f4)?;
        let row = *mem.ewram.get(0x166f5)?;
        if page >= pages || col >= 10 || row >= 6 {
            return None;
        }
        // The original navigation uses this ROM table: six row widths per page.
        // Character resource pointers are not a rectangular code table.
        let counts = mem
            .rom
            .get(rows + page as usize * 6..rows + (page as usize + 1) * 6)?;
        if counts.iter().any(|&n| n == 0 || n > 10) {
            return None;
        }
        let mut targets = Vec::new();
        for i in 0..60u8 {
            if i % 10 >= counts[(i / 10) as usize] {
                continue;
            }
            let x = 12. + (i % 10) as f32 * 16.;
            let y = 36. + (i / 10) as f32 * 16.;
            targets.push(Target {
                index: 80 + i,
                bounds: [x, y, x + 16., y + 16.],
            });
        }
        for (i, bounds) in [
            [9., 12., 43., 30.],
            [43., 12., 80., 30.],
            [80., 12., 115., 30.],
            [190., 48., 234., 84.],
            [175., 8., 236., 30.],
        ]
        .into_iter()
        .enumerate()
        {
            if i<3 && i>=pages as usize {continue;}
            targets.push(Target {
                index: 144 + i as u8,
                bounds,
            });
        }
        Some(Self {
            selected: col + row * 10,
            targets,
            selection_offset: 0x166f4,
            base: 80,
            confirm_key: 0x3fe,
        })
    }
    fn name_confirmation(mem: &MemMap) -> Option<Self> {
        if !name_entry_active(mem) || mem.iwram.get(0x27d1) != Some(&2)
            || half(&mem.io, 0)? != 0x1f40
            || half(&mem.io, 8)? != 0x4c03
            || !(0x125..=0x127).contains(&half(&mem.oam, 4)?)
            || mem.rom.get(0x11bf8..0x11bfc) != Some(&[0x51, 0x40, 1, 0x70])
        {
            return None;
        }
        let ptr = u32::from_le_bytes(mem.ewram.get(0x918c..0x9190)?.try_into().ok()?);
        if ptr != 0x02017370 {
            return None;
        }
        let selected = *mem.ewram.get(0x173b1)?;
        if selected >= 2 {
            return None;
        }
        Some(Self {
            selected,
            selection_offset: 0x173b1,
            base: 152,
            confirm_key: 0x3fe,
            targets: vec![
                Target {
                    index: 152,
                    bounds: [50., 66., 86., 82.],
                },
                Target {
                    index: 153,
                    bounds: [86., 66., 122., 82.],
                },
            ],
        })
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<u8> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        self.targets
            .iter()
            .find(|t| x >= t.bounds[0] && x < t.bounds[2] && y >= t.bounds[1] && y < t.bounds[3])
            .map(|t| t.index)
    }
}
#[derive(Default)]
pub struct LanguageMouse {
    grid: InteractionGrid,
    active_base: Option<u8>,
    enabled: bool,
    target: Option<u8>,
    confirm: bool,
    pulse: u8,
    release: u8,
    key: u16,
    deadline: u64,
    selection_event: Option<u8>,
    usa: bool,
    typing: std::collections::VecDeque<NameKey>,
    typing_ready: bool,
}
#[derive(Clone,Copy)]
enum NameKey { Glyph{page:u8,cell:u8}, Page(u8), Delete, Accept }
impl LanguageMouse {
    pub fn new(rom: &[u8], enabled: bool) -> Self {
        let version=ffta_rom::version(rom);
        let usa=version==Some(Version::Usa);
        let valid = enabled && ((usa && rom.get(0x136084..0x136090)==Some(&[0xf0,0xb5,0x4f,0x46,0x46,0x46,0xc0,0xb4,0x82,0xb0,0x0f,0x1c])
            && rom.get(0x136a78..0x136a7c)==Some(&[1,0x30,8,0x70])) || (version==Some(Version::Europe)
            && rom.get(0x13ffca..0x13ffce) == Some(&[0x85, 0x70, 0x39, 0x68])));
        Self {
            enabled: valid,
            usa,
            ..Self::default()
        }
    }
    pub fn scene(&self, mem: &MemMap) -> Option<LanguageScene> {
        if self.enabled {
            if self.usa {LanguageScene::frontend(mem).or_else(||LanguageScene::keyboard(mem))}else{LanguageScene::read(mem)}
        } else {
            None
        }
    }
    fn cancel_pending(&mut self) {
        self.target = None;
        self.confirm = false;
        self.pulse = 0;
        self.release = 0;
        self.selection_event = None;
    }
    pub fn cancel(&mut self) {
        self.cancel_pending();
        self.typing.clear();self.typing_ready=false;
        self.grid.reset_pointer();
    }
    pub fn is_name_input(&self,mem:&MemMap)->bool {self.scene(mem).is_some_and(|s|s.base==80)}
    /// Queue original keyboard selections and A/B/Start, rather than writing a name buffer.
    pub fn type_text(&mut self,mem:&MemMap,text:&str)->bool {
        if !self.is_name_input(mem) {return false;}
        let keys:Vec<_>=text.chars().filter_map(|c|match c {
            'A'..='Z'=>Some(NameKey::Glyph{page:0,cell:c as u8-b'A'}),
            'a'..='z'=>Some(NameKey::Glyph{page:0,cell:30+c as u8-b'a'}),
            '0'..='9'=>Some(NameKey::Glyph{page:2,cell:c as u8-b'0'}),
            _=>None,
        }).collect();
        if keys.is_empty(){return text.chars().any(|c|!c.is_control());}
        if self.typing.is_empty(){self.cancel_pending();self.typing_ready=false;}
        for key in keys {if self.typing.len()<64 {self.typing.push_back(key);}}
        self.deadline=mem.frames+600;true
    }
    pub fn name_command(&mut self,mem:&MemMap,accept:bool)->bool {
        if !self.is_name_input(mem){return false;}
        if self.typing.is_empty(){self.cancel_pending();self.typing_ready=false;}
        if self.typing.len()<64 {self.typing.push_back(if accept{NameKey::Accept}else{NameKey::Delete});}
        self.deadline=mem.frames+600;true
    }
    fn lose_scene(&mut self) {
        self.cancel();
        self.grid.clear();
    }
    fn publish_targets(&mut self, scene: &LanguageScene) {
        let ex = mem::extra() as f32;
        let ey = mem::extra_y() as f32;
        let regions = scene
            .targets
            .iter()
            .map(|t| Region {
                event: Some(EventId {
                    context: LANGUAGE_CONTEXT,
                    option: t.index as u16,
                }),
                bounds: t.bounds,
                layer: 0,
            })
            .collect::<Vec<_>>();
        self.grid.update(
            [
                -ex,
                -ey,
                mem::render_width() as f32 - ex,
                mem::render_height() as f32 - ey,
            ],
            &regions,
        );
    }
    pub fn back(&mut self, mem: &MemMap) {
        if self.scene(mem).is_some_and(|s| s.base != 0 && s.base != 64) {
            self.cancel();
            self.key = 0x3fd;
            self.pulse = 4;
            self.release = 4;
            self.deadline = mem.frames + 20;
        }
    }
    pub fn valid_target(index: u8) -> bool {
        matches!(index,0..=4|32..=35|40..=42|48..=52|56..=57|60..=61|64|80..=139|144..=148|152..=153)
    }
    pub fn grid_metrics(&self) -> (usize, u64) {
        (self.grid.cell_count(), self.grid.revision())
    }
    pub fn aim(&mut self, mem: &MemMap, x: f32, y: f32, confirm: bool) -> bool {
        let Some(scene) = self.scene(mem) else {
            self.lose_scene();
            return false;
        };
        if self.active_base != Some(scene.base) {
            self.cancel_pending();
            self.active_base = Some(scene.base);
        }
        self.publish_targets(&scene);
        let update = self.grid.pointer(x, y, confirm);
        let Some(event) = update.hovered else {
            self.cancel_pending();
            return false;
        };
        if event.context != LANGUAGE_CONTEXT {
            self.cancel_pending();
            return false;
        }
        if scene.base==80 && confirm {
            let index=event.option as u8;
            let action=match index {
                80..=139=>Some(NameKey::Glyph{page:mem.ewram[0x166da],cell:index-80}),
                144..=146=>Some(NameKey::Page(index-144)),
                147=>Some(NameKey::Accept),148=>Some(NameKey::Delete),_=>None,
            };
            if let Some(action)=action {
                if self.typing.len()<64 {
                    if self.typing.is_empty(){self.typing_ready=matches!(action,NameKey::Glyph{cell,..} if cell==scene.selected);}
                    self.typing.push_back(action);self.target=None;self.confirm=false;
                    self.deadline=mem.frames+180+self.typing.len()as u64*20;
                }
                return true;
            }
        }
        if update.entered.is_some() || update.clicked.is_some() {
            let index = event.option as u8;
            if self.target != Some(index) {
                self.cancel_pending();
            }
            self.target = Some(index);
            self.confirm |= confirm;
            self.deadline = mem.frames + 180;
        }
        true
    }
    /// Commit only the verified selector byte; the original game renders the result.
    pub fn select(&mut self, mem: &mut MemMap, index: u8) -> bool {
        let Some(scene) = self.scene(mem) else {
            return false;
        };
        if !scene.targets.iter().any(|t| t.index == index) {
            return false;
        }
        if scene.base == 0 && mem.rom.get(0x13ffca..0x13ffce) != Some(&[0x85, 0x70, 0x39, 0x68]) {
            return false;
        }
        let value = index - scene.base;
        if scene.base == 80 {
            if index >= 144 {
                return false;
            }
            if scene.selected != value {
                mem.ewram[scene.selection_offset] = value % 10;
                mem.ewram[scene.selection_offset + 1] = value / 10;
                self.selection_event = Some(index);
            }
            return true;
        }
        if scene.selection_offset != usize::MAX && scene.selected != value {
            if matches!(scene.base,56|60) {
                self.active_base=Some(scene.base);self.target=Some(index);
                self.deadline=mem.frames+180;
                return true;
            }
            mem.ewram[scene.selection_offset] = value;
            self.selection_event = Some(index);
        }
        true
    }
    pub fn take_selection_event(&mut self) -> Option<u8> {
        self.selection_event.take()
    }
    /// One emulated frame, including during fast-forward. Real buttons win.
    pub fn keys(&mut self, mem: &mut MemMap, physical: u16) -> u16 {
        if !self.enabled {
            return physical;
        }
        if physical != RELEASED {
            self.cancel();
            return physical;
        }
        let Some(scene) = self.scene(mem) else {
            self.lose_scene();
            return physical;
        };
        if self.active_base != Some(scene.base) {
            self.cancel_pending();
            self.active_base = Some(scene.base);
        }
        self.publish_targets(&scene);
        if self
            .target
            .is_some_and(|target| !scene.targets.iter().any(|t| t.index == target))
        {
            self.cancel();
            return physical;
        }
        if mem.frames > self.deadline {
            self.cancel();
            return physical;
        }
        if self.pulse > 0 {
            self.pulse -= 1;
            return self.key;
        }
        if self.release > 0 {
            self.release -= 1;
            return physical;
        }
        if scene.base==80 {
            if let Some(input)=self.typing.front().copied() {
                let key=match input {
                    NameKey::Glyph{page,cell}=>{
                        let pages=name_profile(mem).map_or(3,|p|p.1);
                        let page=if page==2 {pages-1}else{page};
                        let current=mem.ewram[0x166da];
                        if current!=page {
                            self.typing_ready=false;
                            if (current+1)%pages==page {0x2ff}else{0x1ff}
                        }else if !self.typing_ready || scene.selected!=cell {
                            if !self.select(mem,80+cell){self.cancel();return physical;}
                            self.typing_ready=true;self.release=2;return physical;
                        }else {
                            self.typing.pop_front();self.typing_ready=false;0x3fe
                        }
                    }
                    NameKey::Page(desired)=>{
                        let current=mem.ewram[0x166da];
                        let pages=name_profile(mem).map_or(3,|p|p.1);
                        if desired==current {self.typing.pop_front();return physical;}
                        if (current+1)%pages==desired {0x2ff}else{0x1ff}
                    }
                    NameKey::Delete=>{self.typing.pop_front();0x3fd}
                    NameKey::Accept=>{self.typing.pop_front();0x3f7}
                };
                self.key=key;self.pulse=3;self.release=4;return key;
            }
        }else {self.typing.clear();self.typing_ready=false;}
        let Some(target) = self.target else {
            return physical;
        };
        if scene.base == 80 && target >= 144 {
            let key = match target {
                144..=146 if self.confirm => {
                    let page = mem.ewram[0x166da];
                    let desired = target - 144;
                    if desired == page {
                        self.target = None;
                        self.confirm = false;
                        return physical;
                    }
                    let pages=name_profile(mem).map_or(3,|p|p.1);
                    if desired == (page + 1) % pages {
                        0x2ff
                    } else {
                        0x1ff
                    }
                }
                147 if self.confirm => 0x3f7,
                148 if self.confirm => 0x3fd,
                _ => {
                    self.target = None;
                    return physical;
                }
            };
            self.target = None;
            self.confirm = false;
            self.key = key;
            self.pulse = 3;
            self.release = 4;
            return key;
        }
        if scene.selected + scene.base != target {
            if matches!(scene.base,56|60) {
                // Two save cards: let the original publisher update index,
                // border/title palette, cursor and card metadata together.
                // One left/right edge reaches the other card directly.
                self.key=match (scene.base,target>scene.selected+scene.base) {
                    (56,true)=>0x37f,(56,false)=>0x3bf,(_,true)=>0x3ef,_=>0x3df,
                };
                self.pulse=3;self.release=4;
                return self.key;
            }
            if !self.select(mem, target) {
                self.cancel();
                return physical;
            }
            // DMA publishes new text palettes and quill on the next two frames.
            self.release = 2;
            return physical;
        }
        let key = if self.confirm {
            self.target = None;
            self.confirm = false;
            scene.confirm_key
        } else {
            self.target = None;
            return physical;
        };
        self.key = key;
        self.pulse = 3;
        self.release = 4;
        key
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn menu() -> MemMap {
        let mut rom = vec![0; 0x13ffce];
        rom[0x13ffca..].copy_from_slice(&[0x85, 0x70, 0x39, 0x68]);
        let mut m = MemMap::new(rom);
        m.io[..2].copy_from_slice(&0x1140u16.to_le_bytes());
        m.io[8..10].copy_from_slice(&0x3000u16.to_le_bytes());
        m.ewram[0x1c3d4..0x1c3d8].copy_from_slice(&0x0200f3d4u32.to_le_bytes());
        for (slot, attrs) in [(0, [0x26, 0x4041, 0x308c]), (1, [0x8026, 0x51, 0x3090])] {
            for (j, a) in attrs.into_iter().enumerate() {
                m.oam[slot * 8 + j * 2..slot * 8 + j * 2 + 2].copy_from_slice(&u16::to_le_bytes(a));
            }
        }
        for i in 0..5 {
            for (part, attrs) in [
                [28 + i * 18, 0x8054, 0x2000 + i * 16],
                [0x4024 + i * 18, 0x8070, 0x50 + i * 12],
                [36 + i * 18, 0x4090, 0x58 + i * 12],
            ]
            .into_iter()
            .enumerate()
            {
                let o = (2 + i as usize * 3 + part) * 8;
                for (j, a) in attrs.into_iter().enumerate() {
                    m.oam[o + j * 2..o + j * 2 + 2].copy_from_slice(&(a as u16).to_le_bytes());
                }
            }
        }
        m
    }
    #[test]
    fn complete_scene_required_and_hit_regions_do_not_overlap() {
        let mut m = menu();
        let scene = LanguageScene::read(&m).unwrap();
        for y in 0..160 {
            for x in 0..240 {
                assert!(
                    scene
                        .targets
                        .iter()
                        .filter(|t| x as f32 >= t.bounds[0]
                            && (x as f32) < t.bounds[2]
                            && y as f32 >= t.bounds[1]
                            && (y as f32) < t.bounds[3])
                        .count()
                        <= 1
                );
            }
        }
        for (i, t) in scene.targets.iter().enumerate() {
            assert_eq!(
                scene.hit(120., (t.bounds[1] + t.bounds[3]) * 0.5),
                Some(i as u8)
            );
        }
        assert_eq!(scene.hit(70., 40.), None);
        assert_eq!(scene.hit(f32::NAN, 40.), None);
        m.oam[6 * 8 + 4] ^= 1;
        assert!(LanguageScene::read(&m).is_none());
    }
    #[test]
    fn language_grid_matches_targets_and_reuses_cells_without_repeat_events() {
        mem::set_extra(0);
        mem::set_extra_y(0);
        let mut m = menu();
        let mut mouse = LanguageMouse {
            enabled: true,
            ..Default::default()
        };
        let scene = mouse.scene(&m).unwrap();
        assert!(mouse.aim(&m, 120., 116., false));
        assert_eq!(mouse.grid_metrics(), (600, 1));
        for y in 0..160 {
            for x in 0..240 {
                assert_eq!(
                    mouse
                        .grid
                        .hit(x as f32 + 0.5, y as f32 + 0.5)
                        .map(|e| e.option as u8),
                    scene.hit(x as f32 + 0.5, y as f32 + 0.5)
                );
            }
        }
        assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
        assert_eq!(mouse.take_selection_event(), Some(4));
        for x in [85., 96., 111., 125., 145., 159.] {
            assert!(mouse.aim(&m, x, 116., false));
            mouse.keys(&mut m, RELEASED);
            assert_eq!(mouse.take_selection_event(), None);
        }
        assert_eq!(mouse.grid_metrics(), (600, 1));
        m.io[0] = 0;
        assert!(!mouse.aim(&m, 120., 116., true));
        assert_eq!(mouse.grid.cell_count(), 0);
    }
    #[test]
    fn input_edges_release_and_confirmation_is_once() {
        let mut m = menu();
        let mut mouse = LanguageMouse {
            enabled: true,
            ..Default::default()
        };
        assert!(mouse.aim(&m, 120., 62., true));
        let original = m.ewram.clone();
        assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
        let changed: Vec<_> = m
            .ewram
            .iter()
            .zip(original.iter())
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect();
        assert_eq!(changed, vec![0xf3d6]);
        assert_eq!(m.ewram[0xf3d6], 1);
        assert_eq!(mouse.take_selection_event(), Some(1));
        assert_eq!(mouse.take_selection_event(), None);
        let mut keys = vec![RELEASED];
        for _ in 0..10 {
            m.frames += 1;
            keys.push(mouse.keys(&mut m, RELEASED));
        }
        assert_eq!(&keys[..3], &[RELEASED; 3]);
        assert_eq!(&keys[3..7], &[0x3fe; 4]);
        assert_eq!(&keys[7..], &[RELEASED; 4]);
        assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
        assert!(mouse.aim(&m, 120., 80., false));
        assert_eq!(mouse.keys(&mut m, 0x3ef), 0x3ef);
        assert!(mouse.target.is_none());
    }
    #[test]
    fn all_language_pairs_jump_in_one_write_without_direction_buttons() {
        for from in 0..5 {
            for to in 0..5 {
                let mut m = menu();
                m.ewram[0xf3d6] = from;
                let mut mouse = LanguageMouse {
                    enabled: true,
                    ..Default::default()
                };
                assert!(mouse.aim(&m, 120., 44. + 18. * to as f32, false));
                assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
                assert_eq!(m.ewram[0xf3d6], to);
            }
        }
        let mut m = menu();
        let mut mouse = LanguageMouse {
            enabled: true,
            ..Default::default()
        };
        m.rom[0x13ffca] ^= 1;
        assert!(!mouse.select(&mut m, 4));
        assert_eq!(m.ewram[0xf3d6], 0);
    }
    #[test]
    fn disabled_wrong_rom_and_scene_loss_never_inject_input() {
        let mut m = menu();
        let mut mouse = LanguageMouse::new(&m.rom, true);
        assert!(!mouse.aim(&m, 120., 62., true));
        assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
        mouse.enabled = true;
        mouse.aim(&m, 120., 62., true);
        m.io[0] = 0;
        assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
        assert!(mouse.target.is_none());
    }
    fn frontend(mode: u8) -> MemMap {
        let mut m = menu();
        m.iwram[0x27d1] = 1;
        m.io[..2].copy_from_slice(&0xf740u16.to_le_bytes());
        m.io[8..10].copy_from_slice(&0x3d83u16.to_le_bytes());
        m.io[10..12].copy_from_slice(&0x3e0au16.to_le_bytes());
        m.ewram[0xf3b0..0xf3b4].copy_from_slice(&0x0200f3d4u32.to_le_bytes());
        m.ewram[0xf3b8..0xf3bc].copy_from_slice(&7u32.to_le_bytes());
        m.ewram[0xf3bc..0xf3c0].copy_from_slice(&0x08139d29u32.to_le_bytes());
        m.ewram[0xf3d4 + 0x8994] = mode;
        m.rom[0x13a734..0x13a738].copy_from_slice(&[1, 0x30, 8, 0x70]);
        m
    }
    #[test]
    fn usa_title_and_main_menu_use_their_own_task_and_selection_offsets() {
        for mode in [0,1] {
            for from in 0..3 {for to in 0..3 {
                let mut m=frontend(0);
                m.rom[0x136084..0x136090].copy_from_slice(&[0xf0,0xb5,0x4f,0x46,0x46,0x46,0xc0,0xb4,0x82,0xb0,0x0f,0x1c]);
                m.rom[0x136a78..0x136a7c].copy_from_slice(&[1,0x30,8,0x70]);
                m.ewram[0xf3b0..0xf3b4].copy_from_slice(&0x0200f3dcu32.to_le_bytes());
                m.ewram[0xf3bc..0xf3c0].copy_from_slice(&7u32.to_le_bytes());
                m.ewram[0xf3c0..0xf3c4].copy_from_slice(&0x08136085u32.to_le_bytes());
                m.ewram[0xf3dc+0x8993]=mode;
                let offset=0xf3dc+if mode==0{0x8962}else{0x8963};
                m.ewram[offset]=from;
                let mut mouse=LanguageMouse{enabled:true,usa:true,..Default::default()};
                let scene=mouse.scene(&m).unwrap();
                assert_eq!(scene.targets.len(),3);
                if mode==0 {assert_eq!(scene.targets[2].bounds,[24.,114.,80.,122.]);}
                let target=scene.targets[to as usize];
                let before=m.ewram.clone();
                assert!(mouse.aim(&m,(target.bounds[0]+target.bounds[2])/2.,(target.bounds[1]+target.bounds[3])/2.,false));
                assert_eq!(mouse.keys(&mut m,RELEASED),RELEASED);
                assert_eq!(m.ewram[offset],to);
                assert!(m.ewram.iter().zip(&before).enumerate().all(|(i,(a,b))|a==b||i==offset));
                assert!(!mouse.select(&mut m,35)); // USA has no Language option.
                m.ewram[0xf3bc]=5;
                mouse.cancel();
                assert!(mouse.aim(&m,110.,112.,true));
                mouse.keys(&mut m,RELEASED);
                assert_eq!(mouse.keys(&mut m,RELEASED),0x3f7);
                m.rom[0x136084]^=1;
                assert!(mouse.scene(&m).is_none());
            }}
        }
    }
    #[test]
    fn save_card_hover_uses_original_redraw_without_partial_selector_write(){
        for from in 0..2 {
            let mut m=frontend(0);
            m.iwram[0x27d1]=11;
            for(o,v)in [(0,0x1f40u16),(8,0xad0b),(10,0xac09)]{
                m.io[o..o+2].copy_from_slice(&v.to_le_bytes());
            }
            m.rom[0x8be72..0x8be76].copy_from_slice(&[0x28,0x40,0x30,0x70]);
            m.ewram[0xf3d4+0x2158+0x104]=2;
            let offset=0xf3d4+0x2158+0x105;
            m.ewram[offset]=from;
            let before=m.ewram.clone();
            let mut mouse=LanguageMouse{enabled:true,..Default::default()};
            let target=mouse.scene(&m).unwrap().targets[(1-from)as usize];
            assert!(mouse.aim(&m,(target.bounds[0]+target.bounds[2])*0.5,90.,false));
            assert_eq!(mouse.keys(&mut m,RELEASED),if from==0{0x3ef}else{0x3df});
            assert_eq!(m.ewram,before,"the guest handler must redraw border, title and cursor together");
        }
    }
    #[test]
    fn map_system_load_uses_the_shared_task_manager_not_title_globals(){
        for state in [4,11] {
            let mut m=frontend(0);m.iwram[0x27d1]=state;
            for(o,v)in [(0,0x1f40u16),(8,0xad0b),(10,0xac09)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
            m.ewram[0xf3b0..0xf3b4].copy_from_slice(&0x06006800u32.to_le_bytes());
            m.iwram[0x27d8..0x27dc].copy_from_slice(&0x0200f3a4u32.to_le_bytes());
            m.iwram[0x2804..0x2808].copy_from_slice(&0x0813dd75u32.to_le_bytes());
            m.rom[0x13dd74..0x13dd7c].copy_from_slice(&[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46]);
            m.rom[0x8be72..0x8be76].copy_from_slice(&[0x28,0x40,0x30,0x70]);
            let offset=0xf3a4+0x2158+0x105;m.ewram[offset-1]=2;
            let mut mouse=LanguageMouse{enabled:true,..Default::default()};
            let scene=mouse.scene(&m).unwrap();assert_eq!(scene.selection_offset,offset);
            let t=scene.targets[1];assert!(mouse.aim(&m,(t.bounds[0]+t.bounds[2])*0.5,(t.bounds[1]+t.bounds[3])*0.5,false));
            let before=m.ewram.clone();assert_eq!(mouse.keys(&mut m,RELEASED),if state==4{0x37f}else{0x3ef});assert_eq!(m.ewram,before);
            m.iwram[0x2804..0x2808].fill(0);assert!(mouse.scene(&m).is_none());
            m.iwram[0x2804..0x2808].copy_from_slice(&0x0813dd75u32.to_le_bytes());
            m.iwram[0x27d8..0x27dc].copy_from_slice(&0x0203fffcu32.to_le_bytes());assert!(mouse.scene(&m).is_none());
        }
    }
    #[test]
    fn frontend_targets_leave_horizontal_margins_and_vertical_gaps(){
        for mode in 0..3 {
            let mem=frontend(mode);
            let scene=LanguageScene::frontend(&mem).unwrap();
            for t in &scene.targets {
                let [l,top,r,bottom]=t.bounds;let y=(top+bottom)*0.5;
                assert_eq!(scene.hit((l+r)*0.5,y),Some(t.index));
                assert_eq!(scene.hit(l-1.,y),None);
                assert_eq!(scene.hit(r+1.,y),None);
                assert_eq!(scene.hit((l+r)*0.5,bottom+1.),None);
                assert!(r-l<=72.);
            }
        }
    }
    #[test]
    fn frontend_all_pairs_direct_selection_and_context_boundaries() {
        for (mode, base, count, offset) in
            [(0, 32, 4, 0x8962), (1, 40, 3, 0x8963), (2, 48, 5, 0x8964)]
        {
            for from in 0..count {
                for to in 0..count {
                    let mut m = frontend(mode);
                    m.ewram[0xf3d4 + offset] = from;
                    let before = m.ewram.clone();
                    let mut mouse = LanguageMouse {
                        enabled: true,
                        ..Default::default()
                    };
                    let t = mouse.scene(&m).unwrap().targets[to as usize];
                    assert!(mouse.aim(
                        &m,
                        (t.bounds[0] + t.bounds[2]) * 0.5,
                        (t.bounds[1] + t.bounds[3]) * 0.5,
                        false
                    ));
                    assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
                    assert_eq!(m.ewram[0xf3d4 + offset], to);
                    assert!(m
                        .ewram
                        .iter()
                        .zip(before.iter())
                        .enumerate()
                        .all(|(i, (a, b))| a == b || i == 0xf3d4 + offset));
                    assert!(!mouse.select(&mut m, if base == 32 { 40 } else { 32 }));
                }
            }
        }
        let mut m = frontend(0);
        let mut mouse = LanguageMouse {
            enabled: true,
            ..Default::default()
        };
        mouse.aim(&m, 30., 76., true);
        mouse.keys(&mut m, RELEASED);
        assert_eq!(mouse.keys(&mut m, RELEASED), 0x3fe);
        m.ewram[0xf3d4 + 0x8994] = 1;
        assert_eq!(mouse.keys(&mut m, RELEASED), RELEASED);
        m.rom[0x13a734] ^= 1;
        assert!(mouse.scene(&m).is_none());
    }
    #[test]
    fn title_click_uses_start_and_back_uses_b() {
        let mut m = frontend(0);
        m.ewram[0xf3b8] = 5;
        let mut mouse = LanguageMouse {
            enabled: true,
            ..Default::default()
        };
        assert!(mouse.aim(&m, 100., 110., true));
        assert_eq!(mouse.keys(&mut m, RELEASED), 0x3f7);
        mouse.back(&m); // Press Start has no back action.
        m.ewram[0xf3b8] = 7;
        mouse.keys(&mut m, RELEASED);
        mouse.back(&m);
        assert_eq!(mouse.keys(&mut m, RELEASED), 0x3fd);
    }
    #[test]
    fn name_keyboard_excludes_blank_cells_and_moves_both_coordinates() {
        let mut m = menu();
        m.iwram[0x27d1] = 2;
        m.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
        m.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        m.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
        m.oam[4..6].copy_from_slice(&0x120u16.to_le_bytes());
        m.ewram[0x166db] = 3;
        m.ewram[0x166e8..0x166ec].copy_from_slice(&0x02030000u32.to_le_bytes());
        m.ewram[0x30000] = 1;
        m.ewram[0x30000 + 58 * 2] = 1;
        m.rom.resize(0x3ad310, 0);
        m.rom[0x12e6a0..0x12e6a4].copy_from_slice(&[0xfe, 0xd2, 0x3a, 8]);
        m.rom[0x3ad2fe..0x3ad304].copy_from_slice(&[10, 10, 6, 10, 10, 9]);
        m.rom[0x12e81a..0x12e81e].copy_from_slice(&[1, 0x30, 0x20, 0x70]);
        let mut mouse = LanguageMouse {
            enabled: true,
            ..Default::default()
        };
        assert!(!mouse.select(&mut m, 139));
        assert!(mouse.select(&mut m, 138));
        assert_eq!(&m.ewram[0x166f4..0x166f6], &[8, 5]);
        assert!(mouse.aim(&m, 60., 20., true));
        assert_eq!(mouse.keys(&mut m, RELEASED), 0x2ff);
    }
    fn typing_fixture(usa:bool)->(MemMap,LanguageMouse){
        let mut m=menu();m.iwram[0x27d1]=2;
        for(o,v)in[(0,0x1f40u16),(8,0x4c03),(10,0x4e02)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.oam[4..6].copy_from_slice(&0x120u16.to_le_bytes());m.ewram[0x166db]=3;
        m.rom.resize(0x3ad310,0);
        let (literal,check,table)=if usa{(0x12aab4,0x12ac2e,0x3a8320)}else{(0x12e6a0,0x12e81a,0x3ad2fe)};
        m.rom[literal..literal+4].copy_from_slice(&(0x08000000u32+table as u32).to_le_bytes());
        m.rom[check..check+4].copy_from_slice(&[1,0x30,0x20,0x70]);
        m.rom[table..table+18].fill(10);
        let mouse=LanguageMouse{enabled:true,usa,..Default::default()};(m,mouse)
    }
    #[test]
    fn naming_tabs_require_click_and_repeated_glyph_clicks_remain_independent(){
        let(mut m,mut mouse)=typing_fixture(false);
        assert!(mouse.aim(&m,60.,20.,false));
        assert_eq!(mouse.keys(&mut m,RELEASED),RELEASED);
        assert!(mouse.aim(&m,60.,20.,true));
        assert_eq!(mouse.keys(&mut m,RELEASED),0x2ff);
        mouse.cancel();
        for _ in 0..2 {
            assert!(mouse.aim(&m,14.,38.,true));
            assert_eq!(mouse.keys(&mut m,RELEASED),0x3fe);
            for _ in 0..9 {m.frames+=1;mouse.keys(&mut m,RELEASED);}
        }
    }
    #[test]fn keyboard_click_queue_keeps_letters_and_start_in_order(){
        let(mut m,mut mouse)=typing_fixture(false);
        assert!(mouse.aim(&m,14.,38.,true));
        assert!(mouse.aim(&m,30.,38.,true));
        assert!(mouse.aim(&m,208.,55.,true));
        assert_eq!(mouse.typing.len(),3);
        assert!(matches!(mouse.typing[0],NameKey::Glyph{cell:0,..}));
        assert!(matches!(mouse.typing[1],NameKey::Glyph{cell:1,..}));
        assert!(matches!(mouse.typing[2],NameKey::Accept));
        let mut edges=Vec::new();let mut last=RELEASED;
        for _ in 0..80 {m.frames+=1;let k=mouse.keys(&mut m,RELEASED);
            if k!=RELEASED && last==RELEASED{edges.push(k);}last=k;}
        assert_eq!(edges,vec![0x3fe,0x3fe,0x3f7]);
    }
    #[test]fn story_question_is_not_the_name_keyboard(){
        let(mut m,mouse)=typing_fixture(false);
        m.oam[4..6].copy_from_slice(&0x125u16.to_le_bytes());
        m.ewram[0x918c..0x9190].copy_from_slice(&0x02017370u32.to_le_bytes());
        m.rom[0x11bf8..0x11bfc].copy_from_slice(&[0x51,0x40,1,0x70]);
        m.ewram[0x173b1]=0;
        assert!(!name_entry_active(&m));
        assert!(mouse.scene(&m).is_none(),"a tutorial question must use dialogue coordinates, not the name-confirmation hitboxes");
        m.oam[8..10].copy_from_slice(&0x80u16.to_le_bytes());
        m.oam[12..14].copy_from_slice(&0x56d0u16.to_le_bytes());
        assert!(name_entry_active(&m));
        assert!(LanguageScene::name_confirmation(&m).is_some());
    }
    #[test]fn name_frame_owns_transition_even_when_cursor_is_temporarily_absent(){
        let(mut m,_)=typing_fixture(false);
        m.oam[4..6].copy_from_slice(&0x9334u16.to_le_bytes());
        m.oam[8..10].copy_from_slice(&0x0080u16.to_le_bytes());
        m.oam[12..14].copy_from_slice(&0x56d0u16.to_le_bytes());
        assert!(name_entry_active(&m));
        m.oam[8..10].copy_from_slice(&0x0200u16.to_le_bytes());
        assert!(!name_entry_active(&m));
    }
    #[test]
    fn typed_names_select_case_and_numeric_page_then_emit_one_a_per_character(){
        for usa in [false,true]{
            let(mut m,mut mouse)=typing_fixture(usa);let pages=if usa{2}else{3};
            assert!(mouse.type_text(&m,"Az9"));
            let mut previous=RELEASED;let mut entered=Vec::new();
            for f in 0..150{
                m.frames=f;let key=mouse.keys(&mut m,RELEASED);
                if key!=previous{match key{
                    0x2ff=>m.ewram[0x166da]=(m.ewram[0x166da]+1)%pages,
                    0x1ff=>m.ewram[0x166da]=(m.ewram[0x166da]+pages-1)%pages,
                    0x3fe=>entered.push((m.ewram[0x166da],m.ewram[0x166f4],m.ewram[0x166f5])),_=>{}}}
                previous=key;
            }
            assert_eq!(entered,vec![(0,0,0),(0,5,5),(pages-1,9,0)]);
            mouse.name_command(&m,false);mouse.name_command(&m,true);
            let mut edges=Vec::new();
            for f in 150..190{m.frames=f;let k=mouse.keys(&mut m,RELEASED);if k!=previous&&k!=RELEASED{edges.push(k);}previous=k;}
            assert_eq!(edges,vec![0x3fd,0x3f7]);
        }
    }
}
