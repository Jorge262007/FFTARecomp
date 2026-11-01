//! Opt-in battle mouse. Original cursor routine commits the target and refreshes UI.
//! No ROM code, map, range, pathfinding or unit position is patched.
use crate::{
    ffta_rom::{self, Version},
    Machine, MemMap,
};
const RELEASED: u16 = 0x3ff;
#[derive(Clone, Copy, Debug)]
struct Profile {
    base: usize,
    start: u32,
    commit: u32,
    end: u32,
}
#[derive(Clone, Copy, Debug)]
struct Request {
    x: u8,
    y: u8,
    base: usize,
    deadline: u64,
    confirm: bool,
    wake_sent: bool,
    committed: Option<u64>,
}
#[derive(Default)]
pub struct State {
    pub(crate) shop:crate::ffta_shop_mouse::State,
    pub(crate) roster:crate::ffta_roster_mouse::State,
    profile: Option<Profile>,
    pub enabled: bool,
    request: Option<Request>,
    event: Option<u16>,
    pulse: u8,
    release: u8,
    key: u16,
    menu_profile: Option<MenuProfile>,
    menu_request: Option<MenuRequest>,
    pub_scroll:Option<(usize,u16,u64,u8)>,
    pub_scroll_neutral:bool,
    mouse_visual:bool,
    pub_scroll_closed:bool,
    dialogue_advance:Option<(usize,u64)>,
    dialogue_answer:Option<(usize,u64)>,
    pub(crate) card_redrawing:bool,
    pub(crate) bank_redrawing:bool,
    menu_cache: std::sync::atomic::AtomicUsize,
    menu_pitch_cache:std::sync::atomic::AtomicU64,
    buttons: std::collections::VecDeque<u16>,
    button_key: u16,
    button_hold: u8,
    button_release: u8,
    facing_key: Option<u16>,
    world_profile: Option<Profile>,
    world_request: Option<Request>,
}
#[derive(Clone, Copy)]
struct MenuProfile { start: u32, commit: u32, end: u32, cursor: u32 }
#[derive(Clone, Copy)]
struct MenuRequest {
    base: usize, target: u8, confirm: bool,
    last_selected: u8,
    context: [u16;5],
    committed: Option<u64>, deadline: u64,
}
/// A live instance of the game's vertical list, matched to its visible cursor.
/// Count and placement come from the instance, never the names of its entries.
#[derive(Clone, Copy, Debug)]
pub struct MenuScene {
    pub pitch:i32,
    pub count: u8,
    pub selected: u8,
    pub origin: [i32; 2],
    base: usize,
    binary: bool,
    horizontal: bool,
    split: i32,
    polling: bool,
    pub(crate) right: i32,
}
impl MenuProfile {
    fn new(rom: &[u8], version: Version) -> Option<Self> {
        let p = match version {
            Version::Usa => Self { start: 0x08028708, commit: 0x080287a0, end: 0x080287c0, cursor: 0x08017e81 },
            Version::Europe => Self { start: 0x08029944, commit: 0x080299dc, end: 0x080299fc, cursor: 0x08018fc5 },
        };
        let bytes = |pc: u32, n| rom.get((pc - 0x08000000) as usize..(pc - 0x08000000) as usize + n);
        (bytes(p.start, 12) == Some(&[0x70,0xb5,5,0x1c,9,4,9,0x0c,0x2e,0x1c,0x58,0x36])
            && bytes(p.commit, 6) == Some(&[0x24,6,0x21,0x0e,0x30,0x1c])
            && bytes(p.cursor & !1, 12) == Some(&[0x70,0xb5,0x87,0xb0,4,0x1c,0x23,0x1c,0x26,0x7c,4,0x20]))
            .then_some(p)
    }
}
impl MenuScene {

    /// One immutable frame-local index. Every candidate still passes the original
    /// readers; no address/negative result is retained across guest execution.
    pub(crate) fn layout_menus(mem:&MemMap)->(Option<Self>,Option<Self>,Option<Self>,bool){
        let Some(p)=mem.ffta_battle_mouse.menu_profile else{return (None,None,None,false);};
        if h(&mem.io,0).is_none_or(|v|v&0x0f07!=0x0f00)
            || [8,10,12,14].map(|o|h(&mem.io,o))!=[Some(0x4c03),Some(0x4e02),Some(0x0b08),Some(0x0a08)] {
            return (None,None,None,false);
        }
        let mut offsets=[0usize;256];let mut count=0;
        for offset in Self::cursor_offsets(mem,p.cursor,0x20,mem.ewram.len().saturating_sub(0x18),None){
            if count==offsets.len(){return (Self::site_menu(mem),Self::pub_menu(mem),Self::shop_menu(mem),true);}
            offsets[count]=offset;count+=1;
        }
        let known=Some(&offsets[..count]);
        let parent=Self::bank_window_candidates(mem,false,known).or_else(||Self::parent_window_candidates(mem,false,known));
        let town=Self::text_list_candidates(mem,p.cursor,None,known);
        let site=parent.or_else(||town.filter(|m|mem.ewram[m.base+0x10]&15==7 && (2..=12).contains(&m.count)));
        let store=crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==65);
        let pub_menu=town.filter(|m|!store && mem.ewram[m.base+0x10]&7==7 && m.count==4 && m.origin==[8,92]);
        let shop_menu=town.filter(|m|mem.ewram[m.base+0x10]&7==7 && (m.count==3 || m.count==4 && store) && m.origin==[8,92]);
        (site,pub_menu,shop_menu,count!=0)
    }
    fn cursor_offsets<'a>(mem:&'a MemMap,callback:u32,start:usize,end:usize,
        known:Option<&'a[usize]>)->impl Iterator<Item=usize>+'a {
        let mut index=0;let mut next=start;
        std::iter::from_fn(move||{
            if let Some(offsets)=known {
                while let Some(&offset)=offsets.get(index) {
                    index+=1;if (start..end).contains(&offset){return Some(offset);}
                }
                return None;
            }
            while next<end {
                let offset=next;next+=4;
                if w(&mem.ewram,offset)==Some(callback){return Some(offset);}
            }
            None
        })
    }

    /// Presentation-only indoor policy. Uses verified existing window readers,
    /// never translated labels. Pub/shop and other service roots are interiors;
    /// nested lists retain the last root choice until the town planes disappear.
    /// Other non-shop services are a policy grouping, not a semantic prison ID.
    pub fn cinematic_interior_hint(mem:&MemMap)->Option<bool> {
        Self::cinematic_location_hint(mem).map(|kind|kind!=0)
    }
    /// 0 outdoors/bank, 1 verified Pub root, 2 shop, 3 other town service.
    /// None preserves the last root identity through dialogue/nested windows.
    pub fn cinematic_location_hint(mem:&MemMap)->Option<u8> {
        if !mem.ffta_movement.europe_profile || [8,10,12,14].map(|o|h(&mem.io,o))
            !=[Some(0x4c03),Some(0x4e02),Some(0x0b08),Some(0x0a08)] {return Some(0);}
        // Reuse a single town-list scan instead of invoking three wrappers
        // which each search the same EWRAM for the same cursor component.
        if Self::shop_inventory(mem).is_some() {return Some(2);}
        if Self::bank_window(mem,false).is_some() {return Some(0);}
        let menu=Self::town_menu(mem)?;
        let flags=mem.ewram[menu.base+0x10];
        if flags&7==7 && menu.count==3 && menu.origin==[8,92] {return Some(2);}
        if flags&7==7 && menu.count==4 && menu.origin==[8,92] {return Some(1);}
        if flags&15==7 && (2..=12).contains(&menu.count) {return Some(3);}
        None
    }
    pub fn read(mem: &MemMap) -> Option<Self> {
        if mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice && mem.ffta_dialogue.extra.active() {return Self::notice_menu(mem);}
        let s = &mem.ffta_battle_mouse;
        let p = s.menu_profile.filter(|_| s.enabled)?;
        if h(&mem.io,0)?&0x1007!=0x1000 {return None;}
        if let Some(list)=pub_list(mem) {
            let base=list.cursor-0x20;
            let count=(list.total-list.first).min(list.visible)as u8;
            let selected=mem.ewram[list.cursor+17];
            let x=mem.ewram[base+0x13]as i32*8;
            let y=h(&mem.ewram,list.cursor+10)?as i16 as i32;
            let right=(x+(mem.ewram[base+0x15]as i32+2)*8).min(240);
            if selected<count {
                return Some(Self{pitch:Self::measure_pitch(mem,list.cursor,selected,x,right,y,count),
                    count,selected,origin:[x,y],base:list.cursor,binary:false,horizontal:false,split:0,polling:true,right});
            }
        }
        if let Some(scene)=Self::bank_menu(mem) {return Some(scene);}
        if let Some(scene)=Self::parent_window(mem) {return Some(scene);}
        if let Some(scene)=Self::shop_inventory(mem) {return Some(scene);}
        if let Some(scene)=Self::window_list(mem,p.cursor==0x08018fc5) {return Some(scene);}
        let battle_layers=!(h(&mem.io, 0)? & 0x0f07 != 0x0f00
            || h(&mem.io, 8)? != 0x4c03 || h(&mem.io, 10)? != 0x4e02
            || !matches!((h(&mem.io,12)?, h(&mem.io,14)?), (0x5c08,0x4a09) | (0x0b08,0x0a08)));
        if battle_layers {if let Some(choice)=Self::binary_window(mem,p.cursor){return Some(choice);}}
        if battle_layers && mem.ffta_dialogue.action_layout && h(&mem.io,12)==Some(0x5c08) {
            // The cursor's OAM is one publication behind direct selection.
            // Its owned list remains authoritative through blink/refresh.
            if let Some(list)=Self::text_list(mem,p.cursor,None){return Some(list);}
        }
        let mut cursor = None;
        for obj in mem.oam.chunks_exact(8) {
            let a = h(obj,0)?; let b = h(obj,2)?; let c = h(obj,4)?;
            let tile=c&0x3ff;
            let marker=(a&0xff00==0 && (0x124..=0x127).contains(&tile))
                || (a&0xff00==0x4000 && (0x120..=0x123).contains(&tile));
            if marker && b & 0xfe00 == 0
                && a & 255 < 160 && b & 511 < 240 {
                if cursor.is_some() { return None; }
                cursor = Some([(b & 511) as i32, (a & 255) as i32]);
            }
        }
        let cursor = cursor.or_else(|| Self::town_menu(mem).and_then(|scene| {
            Some([h(&mem.ewram,scene.base+8)? as i16 as i32,
                h(&mem.ewram,scene.base+10)? as i16 as i32+h(&mem.ewram,scene.base+14)? as i16 as i32])
        })).or_else(|| {
            // The game's arrow blinks. A pending request was armed while it
            // was visible; retain its verified instance through that blink.
            let r=s.menu_request?;
            if mem.frames>r.deadline || r.context!=[0,8,10,12,14].map(|o|h(&mem.io,o).unwrap_or(0)){return None;}
            let c=if w(&mem.ewram,r.base)==Some(p.cursor){r.base}else{r.base+0x58};
            if w(&mem.ewram,c)!=Some(p.cursor){return None;}
            let flags=*mem.ewram.get(c+0x10)?;
            if flags&15==0{return None;}
            Some([h(&mem.ewram,c+8)? as i16 as i32+if flags&4!=0{15}else if flags&8!=0{24}else{0},
                h(&mem.ewram,c+10)? as i16 as i32+h(&mem.ewram,c+14)? as i16 as i32+if flags&4!=0{1}else{0}])
        })?;
        let shared=||Self::town_menu(mem).or_else(||Self::text_list(mem,p.cursor,Some(cursor)));
        // Spell lists share the cursor component with their parent Action
        // window. The live child owns pointer selection before any parent cache.
        if battle_layers && h(&mem.io,12)==Some(0x5c08) {
            if let Some(child)=Self::text_list(mem,p.cursor,Some(cursor)) {return Some(child);}
        }
        if !battle_layers {return shared();}
        // The two-button confirmation has no text-list pointer or count.
        // Its original handler toggles cursor+0x11 and moves to base+0x80/88.
        let binary_pc = if p.cursor==0x08017e81 {0x29350}else{0x2a58c};
        if mem.rom.get(binary_pc..binary_pc+8)==Some(&[0xf0,0xb5,4,0x1c,0x26,0x1c,0x58,0x36]) {
            for base in (0..mem.ewram.len().saturating_sub(0x90)).step_by(4) {
                if mem.ewram[base+8]!=10 || w(&mem.ewram,base+0x58)!=Some(p.cursor)
                    || w(&mem.ewram,base+0x5c)!=Some(0) || mem.ewram[base+0x68]&4==0 {continue;}
                let selected=mem.ewram[base+0x69];
                if selected>1 {continue;}
                let origin=[h(&mem.ewram,base+0x84)? as i16 as i32,h(&mem.ewram,base+0x86)? as i16 as i32];
                let row=base+0x80+selected as usize*8;
                let x=h(&mem.ewram,row+4)? as i16 as i32;
                let y=h(&mem.ewram,row+6)? as i16 as i32;
                if origin[0]<0 || origin[0]>160 || origin[1]<0 || origin[1]>128
                    || h(&mem.ewram,base+0x8e)? as i32!=origin[1]+16
                    || (cursor!=[x+9,y+4] && !s.menu_request.is_some_and(|r|r.base==base && r.target==selected)) {continue;}
                return Some(Self{pitch:16,count:2,selected,origin,base,binary:true,horizontal:false,split:0,polling:false,right:origin[0]+80});
            }
        }
        let read = |base: usize| -> Option<Self> {
            let count = *mem.ewram.get(base+9)?;
            let selected = *mem.ewram.get(base+0x69)?;
            if count == 0 || count > 32 || selected >= count
                || w(&mem.ewram,base+0x58)? != p.cursor
                || w(&mem.ewram,base+0x5c)? != 0x02000000+base as u32+0x38
                || mem.ewram.get(base+0x68)? & 4 == 0
                || h(&mem.ewram,base+0x66)? != selected as u16*16 { return None; }
            let origin = [h(&mem.ewram,base+0x60)? as i16 as i32, h(&mem.ewram,base+0x62)? as i16 as i32];
            let publishing = s.menu_request.is_some_and(|r| r.base==base
                && r.committed.is_some_and(|f| mem.frames<=f+2));
            if origin[0] < 0 || origin[0] >= 224 || origin[1] < 0
                || origin[1]+count as i32*16 > 160
                || (!publishing && cursor != [origin[0]+15,origin[1]+selected as i32*16+1]) { return None; }
            Some(Self {pitch:16, count, selected, origin, base, binary:false,horizontal:false,split:0,polling:false,right:240 })
        };
        if let Some(scene) = s.menu_request.filter(|r|r.committed.is_some()).and_then(|r|read(r.base)) {
            return Some(scene);
        }
        // A cached instance must still match the current visible cursor. A
        // submenu can reuse its parent's coordinates, so prefer child lists.
        let mut candidates = Vec::new();
        if let Some(scene) = s.menu_cache.load(std::sync::atomic::Ordering::Relaxed).checked_sub(1).and_then(read) {
            if w(&mem.ewram,scene.base+4) != Some(0) { return Some(scene); }
        }
        for base in (0..mem.ewram.len().saturating_sub(0x70)).step_by(4) {
            if w(&mem.ewram,base+0x58) == Some(p.cursor) {
                if let Some(scene) = read(base) { candidates.push(scene); }
            }
        }
        candidates.sort_by_key(|v| w(&mem.ewram,v.base+4) != Some(0));
        let Some(result) = candidates.pop()else{return shared();};
        if candidates.last().is_some_and(|v| w(&mem.ewram,v.base+4) != Some(0)) { return None; }
        s.menu_cache.store(result.base+1,std::sync::atomic::Ordering::Relaxed);
        Some(result)
    }
    /// Other users of the same original cursor/text-list component (shop,
    /// pub). Use the live visible row count and text geometry, not labels.
    pub fn town_menu(mem:&MemMap)->Option<Self> {
        let p=mem.ffta_battle_mouse.menu_profile?;
        if h(&mem.io,0)?&0x0f07!=0x0f00 || h(&mem.io,8)?!=0x4c03
            || h(&mem.io,10)?!=0x4e02 || h(&mem.io,12)?!=0x0b08 || h(&mem.io,14)?!=0x0a08 {return None;}
        Self::text_list(mem,p.cursor,None)
    }
    /// Measured Pub root window, not its Rumors/Missions LIST subwindows.
    pub(crate) fn pub_menu(mem:&MemMap)->Option<Self> {
        if crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==65){return None;}
        let menu=Self::town_menu(mem)?;
        (mem.ewram[menu.base+0x10]&7==7 && menu.count==4
            && menu.origin==[8,92]).then_some(menu)
    }
    /// Shop root uses the same window component, with Buy/Sell/Leave rows.
    pub(crate) fn shop_menu(mem:&MemMap)->Option<Self> {
        let menu=Self::town_menu(mem)?;
        (mem.ewram[menu.base+0x10]&7==7 && (menu.count==3 || menu.count==4
            && crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==65))
            && menu.origin==[8,92]).then_some(menu)
    }
    /// Other active town-root windows share the list callback and style, but
    /// may use a different row count and placement (for example Monster Bank).
    /// LIST and Pub submenus use styles 03 and 0F and remain unchanged.
    pub(crate) fn site_menu(mem:&MemMap)->Option<Self> {
        if let Some(menu)=Self::bank_window(mem,false) {return Some(menu);}
        if let Some(menu)=Self::parent_window(mem) {return Some(menu);}
        let menu=Self::town_menu(mem)?;
        (mem.ewram[menu.base+0x10]&0x0f==7 && (2..=12).contains(&menu.count))
            .then_some(menu)
    }
    /// Rendering must not depend on a cursor being enabled during transitions.
    /// This is read-only and never makes a hidden list clickable.
    pub(crate) fn service_menu_visual(mem:&MemMap)->Option<Self> {
        // An active LIST submenu owns its own pixels. A disabled root left
        // in RAM must not carve that submenu into the old MENU rectangle.
        if let Some(menu)=Self::pub_menu(mem).or_else(||Self::shop_menu(mem)).or_else(||Self::site_menu(mem))
            .or_else(||Self::parent_window_state(mem,true)) {return Some(menu);}
        if Self::town_menu(mem).is_some() {return None;}
        let p=mem.ffta_battle_mouse.menu_profile?;
        if [8,10,12,14].map(|o|h(&mem.io,o)) !=
            [Some(0x4c03),Some(0x4e02),Some(0x0b08),Some(0x0a08)] {return None;}
        let mut result=None;
        for cursor in (0x20..mem.ewram.len().saturating_sub(0x20)).step_by(4) {
            if w(&mem.ewram,cursor)!=Some(p.cursor)
                || w(&mem.ewram,cursor+4)!=Some(0x02000000+cursor as u32-0x20) {continue;}
            let list=cursor-0x20;
            let count=mem.ewram[list+0x16];
            let selected=mem.ewram[cursor+0x11];
            let x=mem.ewram[list+0x13]as i32*8;
            let y=h(&mem.ewram,cursor+10)?as i16 as i32;
            let right=(x+(mem.ewram[list+0x15]as i32+2)*8).min(240);
            // Measured root geometry, independent of language and cursor flags.
            let flags=mem.ewram[cursor+0x10]&15;
            let entering=flags==7 && (-64..160).contains(&y);
            if !matches!(count,3|4) || selected>=count || x!=8 || (y!=92 && !entering) || right<=x {continue;}
            if result.is_some(){return None;}
            result=Some(Self{pitch:16,count,selected,origin:[x,y],base:cursor,
                binary:false,horizontal:false,split:0,polling:true,right});
        }
        result
    }
    /// Measured Monster Bank root. Unlike Pub/Shop this is a parent window
    /// with its cursor at +58; the options sit LEFT of its cursor anchor.
    /// The handler, updater and short ROM signatures identify both regions.
    pub(crate) fn bank_menu(mem:&MemMap)->Option<Self> {
        Self::bank_window(mem,true)
    }
    /// Town parent-window lists (including service menus), with the same
    /// verified cursor component as Monster Bank. Read geometry from their
    /// owner instead of mistaking the cursor anchor for the text's left edge.
    fn parent_window(mem:&MemMap)->Option<Self> {Self::parent_window_state(mem,false)}
    fn parent_window_state(mem:&MemMap,visual:bool)->Option<Self> {Self::parent_window_candidates(mem,visual,None)}
    fn parent_window_candidates(mem:&MemMap,visual:bool,known:Option<&[usize]>)->Option<Self> {
        let p=mem.ffta_battle_mouse.menu_profile?;
        if !matches!(p.cursor,0x08017e81|0x08018fc5){return None;}
        if h(&mem.io,0)?&0x0f07!=0x0f00 || [8,10,12,14].map(|o|h(&mem.io,o))
            !=[Some(0x4c03),Some(0x4e02),Some(0x0b08),Some(0x0a08)] {return None;}
        let function=|pc:u32|->bool {
            if pc&1==0{return false;}
            pc.checked_sub(0x08000001).and_then(|o|h(&mem.rom,o as usize))
                .is_some_and(|op|op&0xff00==0xb500)
        };
        let mut result=None;
        for cursor in Self::cursor_offsets(mem,p.cursor,0x58,mem.ewram.len()-0x18,known) {
            if w(&mem.ewram,cursor)!=Some(p.cursor) {continue;}
            let base=cursor-0x58;
            if w(&mem.ewram,cursor+4)!=Some(0x02000000+base as u32+0x38)
                || !function(w(&mem.ewram,base)?) || !function(w(&mem.ewram,base+4)?)
                || (!matches!(mem.ewram[cursor+0x10]&15,7|15) && !(visual && mem.ewram[cursor+0x10]&15==0)) {continue;}
            let count=mem.ewram[base+9];let selected=mem.ewram[cursor+17];
            let x=mem.ewram[base+0x1c]as i32*8;
            let right=x+mem.ewram[base+0x1e]as i32*8;
            let y=h(&mem.ewram,cursor+10)?as i16 as i32;
            if !(2..=12).contains(&count) || selected>=count || right<=x+8 || right>240
                || y<0 || y+count as i32*16>160
                || h(&mem.ewram,cursor+14)?!=selected as u16*16 {continue;}
            if result.is_some(){return None;}
            result=Some(Self{pitch:16,count,selected,origin:[x,y],base,binary:false,horizontal:false,split:0,polling:true,right});
        }
        result
    }
    fn bank_window(mem:&MemMap,active_only:bool)->Option<Self> {Self::bank_window_candidates(mem,active_only,None)}
    fn bank_window_candidates(mem:&MemMap,active_only:bool,known:Option<&[usize]>)->Option<Self> {
        let p=mem.ffta_battle_mouse.menu_profile?;
        if h(&mem.io,0)?&0x0f07!=0x0f00 || [8,10,12,14].map(|o|h(&mem.io,o))
            !=[Some(0x4c03),Some(0x4e02),Some(0x0b08),Some(0x0a08)] {return None;}
        let (handler,updater,stack)=if p.cursor==0x08018fc5 {(0x5aee4,0x5e7b4,0x85)}else{(0x58d40,0x5c594,0x83)};
        if mem.rom.get(handler..handler+12)!=Some(&[0xf0,0xb5,stack,0xb0,0x0e,0x1c,0,6,5,0x0e,0x31,0x78])
            || mem.rom.get(updater..updater+12)!=Some(&[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46,0xe0,0xb4,0x8e,0xb0]) {return None;}
        let read=|base:usize|->Option<Self> {
            let callback=w(&mem.ewram,base)?;
            let root=callback==0x08000001+handler as u32;
            // Measured EUR empty-list message switches handler while retaining
            // the same window. Keep its frame and text on the same UI canvas.
            let list_pc=if p.cursor==0x08018fc5 {0x5b624}else{0x59454};
            let list_sig=if p.cursor==0x08018fc5 {[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46,0xe0,0xb4,0x86,0xb0]}
                else{[0xf0,0xb5,0x4f,0x46,0x46,0x46,0xc0,0xb4,0x84,0xb0,0x0f,0x1c]};
            let list=!active_only && callback==0x08000001+list_pc as u32
                && mem.rom.get(list_pc..list_pc+12)==Some(&list_sig);
            if (!root && !list)
                || w(&mem.ewram,base+4)?!=0x08000001+updater as u32
                || w(&mem.ewram,base+0x58)?!=p.cursor
                || w(&mem.ewram,base+0x5c)?!=0x02000000+base as u32+0x38
                || (active_only && *mem.ewram.get(base+0x68)?&4==0) {return None;}
            let count=*mem.ewram.get(base+9)?;let selected=*mem.ewram.get(base+0x69)?;
            let x=*mem.ewram.get(base+0x1c)? as i32*8;
            let right=x+*mem.ewram.get(base+0x1e)? as i32*8;
            let y=h(&mem.ewram,base+0x62)? as i16 as i32;
            if count==0 || count>12 || selected>=count || h(&mem.ewram,base+0x66)?!=selected as u16*16
                || right<=x+8 || right>240 || y<0 || y+count as i32*16>160 {return None;}
            Some(Self{pitch:16,count,selected,origin:[x,y],base,binary:false,horizontal:false,split:0,polling:true,right})
        };
        let cache=&mem.ffta_battle_mouse.menu_cache;
        if let Some(menu)=cache.load(std::sync::atomic::Ordering::Relaxed).checked_sub(1).and_then(read) {return Some(menu);}
        let mut found=None;
        for cursor in Self::cursor_offsets(mem,p.cursor,0x58,mem.ewram.len().saturating_sub(0x18),known) {
            let base=cursor-0x58;
            if let Some(menu)=read(base) {if found.is_some(){return None;}found=Some(menu);}
        }
        if let Some(menu)=found {cache.store(menu.base+1,std::sync::atomic::Ordering::Relaxed);}
        found
    }
    pub(crate) fn shop_inventory(mem:&MemMap)->Option<Self> {
        let p=mem.ffta_battle_mouse.menu_profile?;
        let pc=if p.cursor==0x08018fc5 {0x71030}else{0x6eb48};
        if mem.rom.get(pc..pc+12)!=Some(&[0xf0,0xb5,0x87,0xb0,6,0x1c,0x11,0x4f,0x39,0x68,0x11,0x4a])
            || h(&mem.io,0)?&0x4f07!=0x0f00 || h(&mem.io,8)?!=0x4c03 || h(&mem.io,10)?!=0x4e02
            || h(&mem.io,12)?!=0x0b08 || h(&mem.io,14)?!=0x0a08 {return None;}
        let mut found=None;
        for cursor in (0x20..mem.ewram.len()-0x20).step_by(4) {
            let list=cursor-0x20;
            if w(&mem.ewram,cursor)!=Some(0x08000001+pc as u32)
                || w(&mem.ewram,cursor+4)!=Some(0x02000000+list as u32)
                || mem.ewram[cursor+16]&1==0 {continue;}
            let count=h(&mem.ewram,list+0x18)?;
            let visible=mem.ewram[list+0x16] as u16;
            let selected=mem.ewram[cursor+17];
            let first=h(&mem.ewram,list+0x1a)?;
            if count==0 || visible==0 || visible>8 || first>=count {continue;}
            // Cursor indices are page-relative; list+1A is its first item.
            let count=(count-first).min(visible);
            if selected as u16>=count {continue;}
            let x=mem.ewram[list+0x13]as i32*8;
            let y=h(&mem.ewram,cursor+10)? as i16 as i32;
            let right=(x+(mem.ewram[list+0x15]as i32+2)*8).min(240);
            if x<0 || right<=x || y<0 || y+count as i32*16>160 {continue;}
            if found.is_some(){return None;}
            found=Some(Self{pitch:16,count:count.min(visible)as u8,selected,origin:[x,y],base:cursor,binary:false,horizontal:false,split:0,polling:true,right});
        }
        found
    }
    fn text_list(mem:&MemMap,callback:u32,visible_cursor:Option<[i32;2]>)->Option<Self> {Self::text_list_candidates(mem,callback,visible_cursor,None)}
    fn text_list_candidates(mem:&MemMap,callback:u32,visible_cursor:Option<[i32;2]>,known:Option<&[usize]>)->Option<Self> {
        let mut result=None;
        for cursor in Self::cursor_offsets(mem,callback,0x20,mem.ewram.len().saturating_sub(0x20),known) {
            if w(&mem.ewram,cursor)!=Some(callback)
                || w(&mem.ewram,cursor+4)!=Some(0x02000000+cursor as u32-0x20)
                || mem.ewram[cursor+0x10]&0x0f==0 {continue;}
            let list=cursor-0x20;
            let count=mem.ewram[list+0x16];let selected=mem.ewram[cursor+0x11];
            if count==0 || count>12 || selected>=count {continue;}
            let x=mem.ewram[list+0x13]as i32*8;
            let y=h(&mem.ewram,cursor+0xa)? as i16 as i32;
            let flags=mem.ewram[cursor+0x10];
            // Measured town styles: wide LIST arrow (03), compact MENU arrow
            // (0F). Do not mistake the battle cursor style (07) for a town UI.
            let root_window=(matches!(count,3|4) && x==8 && y==92 && flags&7==7)
                || (flags&0x0f==7 && (2..=12).contains(&count));
            if visible_cursor.is_none() && !matches!(flags&15,3|15) && !root_window {continue;}
            let arrow_x=h(&mem.ewram,cursor+8)? as i16 as i32
                +if flags&4!=0 {15}else if flags&8!=0 {24}else{0};
            let arrow_y=y+h(&mem.ewram,cursor+0xe)? as i16 as i32+if flags&4!=0 {1}else{0};
            let visible_cursor=visible_cursor.unwrap_or([arrow_x,arrow_y]);
            // The wide arrow's animation resource shifts its OAM piece up
            // to eight pixels left of the logical anchor (measured in pub).
            let same_x=if flags&4!=0 {visible_cursor[0]==arrow_x}else{(-8..=0).contains(&(visible_cursor[0]-arrow_x))};
            let right=(x+(mem.ewram[list+0x15]as i32+2)*8).min(240);
            let pitch=Self::measure_pitch(mem,cursor,selected,x,right,y,count);
            let publishing=mem.ffta_battle_mouse.menu_request.is_some_and(|r|r.base==cursor&&mem.frames<=r.deadline)
                && same_x && (visible_cursor[1]-y).rem_euclid(pitch)==if flags&4!=0{1}else{0}
                && visible_cursor[1]>=y && visible_cursor[1]<y+count as i32*pitch;
            if x<0 || x>=right || y<0 || y+count as i32*pitch>160
                || (!publishing && (!same_x || visible_cursor[1]!=arrow_y)) {continue;}
            if result.is_some(){return None;}
            result=Some(Self{pitch,count,selected,origin:[x,y],base:cursor,binary:false,horizontal:false,split:0,polling:true,right});
        }
        result
    }
    /// Match the two identical choice bullets in the actual BG2 glyph pixels.
    /// The original cursor supplies the template and selected option; no labels,
    /// language widths or assumed Yes/No coordinates enter the hit targets.
    fn notice_menu(mem:&MemMap)->Option<Self> {
        if !mem.ffta_battle_mouse.enabled || mem.ffta_battle_mouse.menu_profile.is_none(){return None;}
        let bounds=mem.ffta_dialogue.extra.bounds?;
        let mut marker=None;
        for obj in mem.oam.chunks_exact(8) {
            let attrs=[h(obj,0)?,h(obj,2)?,h(obj,4)?];
            let x=(attrs[1]&511)as i32;let y=(attrs[0]&255)as i32;
            if crate::ffta_dialogue::notice_cursor(attrs,x,y,Some(bounds)) {
                if marker.is_some(){return None;}marker=Some((x,y));
            }
        }
        let (cx,cy)=marker?;
        let cnt=h(&mem.io,12)?;
        if cnt&0x80!=0{return None;}
        let map=((cnt>>8)&31)as usize*0x800;
        let chars=((cnt>>2)&3)as usize*0x4000;
        let pixel=|x:i32,y:i32|->Option<u8>{
            if !(0..240).contains(&x)||!(0..160).contains(&y){return None;}
            let wide=cnt&0x4000!=0;let tall=cnt&0x8000!=0;
            let x=(x+h(&mem.io,0x18)?as i32)&if wide{511}else{255};
            let y=(y+h(&mem.io,0x1a)?as i32)&if tall{511}else{255};
            let block=(y/256)*if wide{2}else{1}+x/256;
            let a=h(&mem.vram,map+block as usize*0x800+(((y/8%32)*32+x/8%32)*2)as usize)?;
            let tx=if a&0x400!=0{7-x%8}else{x%8};
            let ty=if a&0x800!=0{7-y%8}else{y%8};
            let v=*mem.vram.get(chars+(a&1023)as usize*32+ty as usize*4+tx as usize/2)?;
            Some((v>>(4*(tx%2)))&15)
        };
        let sample=|x|->Option<[u8;64]>{let mut out=[0;64];for y in 0..8 {for dx in 0..8 {out[(y*8+dx)as usize]=pixel(x+dx,cy+y)?;}}Some(out)};
        let template=sample(cx)?;
        let ink=template.iter().filter(|p|**p!=0).count();
        if !(8..=56).contains(&ink){return None;}
        let choices:Vec<_>=(bounds[0]..bounds[2]-7).filter(|x|sample(*x).as_ref()==Some(&template)).collect();
        if choices.len()!=2 || choices[1]-choices[0]<16{return None;}
        let selected=choices.iter().position(|x|*x==cx)?as u8;
        let right=(choices[1]+8..bounds[2]-8).filter(|x|(0..12).any(|dy|pixel(*x,cy-1+dy).is_some_and(|p|p!=0))).last()?+2;
        Some(Self{pitch:16,count:2,selected,origin:[choices[0],cy-4],base:0x40000,
            binary:true,horizontal:true,split:choices[1],polling:false,right})
    }
    pub fn is_confirmation(&self)->bool {self.binary}
    fn binary_window(mem:&MemMap,callback:u32)->Option<Self>{
        let pc=if callback==0x08017e81{0x29350}else if callback==0x08018fc5{0x2a58c}else{return None;};
        if mem.rom.get(pc..pc+8)!=Some(&[0xf0,0xb5,4,0x1c,0x26,0x1c,0x58,0x36]){return None;}
        let mut result=None;
        for base in (0..mem.ewram.len().saturating_sub(0x90)).step_by(4){
            if mem.ewram[base+8]!=10 || w(&mem.ewram,base+0x58)!=Some(callback)
                || w(&mem.ewram,base+0x5c)!=Some(0) || mem.ewram[base+0x68]&4==0 {continue;}
            let selected=mem.ewram[base+0x69];if selected>1{continue;}
            let origin=[h(&mem.ewram,base+0x84)?as i16 as i32,h(&mem.ewram,base+0x86)?as i16 as i32];
            if !(0..=160).contains(&origin[0]) || !(0..=128).contains(&origin[1])
                || h(&mem.ewram,base+0x8e)?as i32!=origin[1]+16 {continue;}
            if result.is_some(){return None;}
            result=Some(Self{pitch:16,count:2,selected,origin,base,binary:true,horizontal:false,split:0,polling:false,right:origin[0]+80});
        }
        result
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<u8> {
        let left=if self.binary || self.polling {self.origin[0]}else{self.origin[0]+8};
        let right=self.right;
        if !x.is_finite() || !y.is_finite() || x < left as f32 || x >= right as f32
            || y < self.origin[1] as f32 || y >= (self.origin[1]+if self.horizontal {16}else{self.count as i32*self.pitch}) as f32 { return None; }
        if self.horizontal {return Some(u8::from(x>=self.split as f32));}
        Some(((y-self.origin[1] as f32)/self.pitch as f32) as u8)
    }
    fn measure_pitch(mem:&MemMap,cursor:usize,selected:u8,left:i32,right:i32,top:i32,count:u8)->i32 {
        use std::sync::atomic::Ordering;
        let key=((cursor as u64)<<24)|((count as u64)<<16)|(((right-left).clamp(0,255)as u64)<<8);
        let offset=h(&mem.ewram,cursor+14).unwrap_or(0)as i32;
        let measured=if selected>0 && offset%selected as i32==0 {
            let p=offset/selected as i32;(8..=24).contains(&p).then_some(p)
        }else{None};
        if let Some(p)=measured {mem.ffta_battle_mouse.menu_pitch_cache.store(key|p as u64,Ordering::Relaxed);return p;}
        let cache=mem.ffta_battle_mouse.menu_pitch_cache.load(Ordering::Relaxed);
        if cache&!255==key && cache&255!=0{return (cache&255)as i32;}
        // At row zero the cursor has no displacement. Measure blank-separated
        // glyph rows in BG2, never text labels or a guessed language spacing.
        let cnt=h(&mem.io,12).unwrap_or(0);
        let map=((cnt>>8)&31)as usize*0x800;let chars=((cnt>>2)&3)as usize*0x4000;
        let sx=h(&mem.io,0x18).unwrap_or(0)as i32;let sy=h(&mem.io,0x1a).unwrap_or(0)as i32;
        let pixel=|x:i32,y:i32|->bool {
            let x=(x+sx)&if cnt&0x4000!=0{511}else{255};
            let y=(y+sy)&if cnt&0x8000!=0{511}else{255};
            let block=y/256*if cnt&0x4000!=0{2}else{1}+x/256;
            let Some(a)=h(&mem.vram,map+block as usize*0x800+((y/8%32*32+x/8%32)*2)as usize)else{return false;};
            let x=if a&0x400!=0{7-x%8}else{x%8};let y=if a&0x800!=0{7-y%8}else{y%8};
            if cnt&128!=0 {mem.vram.get(chars+(a as usize&1023)*64+y as usize*8+x as usize).is_some_and(|v|*v!=0)}
            else {mem.vram.get(chars+(a as usize&1023)*32+y as usize*4+x as usize/2).is_some_and(|v|v>>(4*(x%2))&15!=0)}
        };
        let mut starts=[0;4];let mut n=0;let mut active=false;
        if left>=0 && right<=240 && right-left>=24 && top>=0 && count>=2 {
            for y in top..(top+count as i32*24).min(160) {
                let ink=(left+8..right-8).any(|x|pixel(x,y));
                if ink && !active {starts[n]=y;n+=1;if n==4{break;}}
                active=ink;
            }
        }
        if n>=2 {
            let mut gaps:Vec<_>=starts[..n].windows(2).map(|v|v[1]-v[0]).collect();gaps.sort_unstable();
            let p=gaps[gaps.len()/2];
            if (8..=24).contains(&p) && gaps.iter().all(|v|(v-p).abs()<=1) {
                mem.ffta_battle_mouse.menu_pitch_cache.store(key|p as u64,Ordering::Relaxed);return p;
            }
        }
        16
    }
    /// Shared window-list component, observed in the world-map town menu.
    /// Only visible, unscrolled instances with verified callback/layout qualify.
    fn window_list(mem:&MemMap,europe:bool)->Option<Self> {
        let pc=if europe {0x397ec}else{0x38028};
        if mem.rom.get(pc..pc+12)!=Some(&[0xf0,0xb5,0x87,0xb0,0x84,0x46,0x0e,0x1c,0x34,0x1c,0x31,0x7c]) {return None;}
        let manager=offset(w(&mem.iwram,0x2810)?,0x9cc+0xcc)?;
        let mut found=None;
        for slot in 0..8 {
            let list=manager+0x9cc+slot*0xcc;
            let cursor=list+0x20;
            if w(&mem.ewram,cursor)!=Some(0x08000001+pc as u32)
                || w(&mem.ewram,cursor+4)!=Some(0x02000000+list as u32) {continue;}
            let window=*mem.ewram.get(list-0x48)? as usize;
            if window>=8 {continue;}
            let win=0x9e0+window*0x4c;
            if mem.iwram.get(win)?&2==0 {continue;}
            let count=h(&mem.ewram,list+0x18)?;
            let visible=*mem.ewram.get(list+0x16)? as u16;
            let raw_selected=*mem.ewram.get(cursor+0x11)?;
            let battle=europe && w(&mem.iwram,0x2804)==Some(0x08025115);
            let cards=europe && map_award_bounds(mem).is_some();
            if cards && mem.ewram[cursor+16]&3!=3 {continue;}
            if count==0 || visible==0 || visible>16 || (!battle && !cards && count>visible) {continue;}
            let first=if battle || cards {h(&mem.ewram,list+0x1a)?}else{0};
            if first>=count {continue;}
            let shown=(count-first).min(visible);
            let selected=if (raw_selected as u16)<shown {raw_selected}
                else if (raw_selected as u16)>=first && (raw_selected as u16)-first<shown {(raw_selected as u16-first)as u8}
                else{continue;};
            let x=mem.iwram[win+0x34]as i8 as i32*8+if cards{8}else{0};
            let y=(mem.iwram[win+0x35]as i8 as i32+mem.iwram[win+0x49]as i8 as i32-1)*8
                +h(&mem.ewram,cursor+0xa)? as i16 as i32-4;
            let right=if cards{240.min(x+mem.iwram[win+0x36]as i32*8)}else{x+mem.iwram[win+0x36]as i32*8};
            let pitch=Self::measure_pitch(mem,cursor,selected,x,right,y,shown as u8);
            if x<0 || right>240 || right<=x || y<0 || y+shown as i32*pitch>160 {continue;}
            if found.is_some(){return None;}
            found=Some(Self{pitch,count:shown as u8,selected,origin:[x,y],base:cursor,binary:false,horizontal:false,split:0,polling:true,right});
        }
        found
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub x: u8,
    pub y: u8,
    pub height: u8,
    pub range: u8,
    pub center: [f32; 2],
}
#[derive(Debug)]
pub struct Scene {
    pub width: u8,
    pub height: u8,
    pub selected: (u8, u8),
    pub cells: Vec<Cell>,
    base: usize,
}
fn h(data: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(o..o + 2)?.try_into().ok()?))
}
fn w(data: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(o..o + 4)?.try_into().ok()?))
}
fn offset(pointer: u32, length: usize) -> Option<usize> {
    let o = pointer.checked_sub(0x02000000)? as usize;
    (o.checked_add(length)? <= 0x40000).then_some(o)
}
impl State {
    pub fn new(rom: &[u8]) -> Self {
        let p = match ffta_rom::version(rom) {
            Some(Version::Usa) => Profile {
                base: 0xf3ac,
                start: 0x08024ad8,
                commit: 0x08025132,
                end: 0x0802516e,
            },
            Some(Version::Europe) => Profile {
                base: 0xf3ac,
                start: 0x08025cec,
                commit: 0x08026346,
                end: 0x08026382,
            },
            _ => return Self::default(),
        };
        if rom.get((p.start - 0x08000000) as usize..(p.start - 0x08000000) as usize + 4)
            != Some(&[0xf0, 0xb5, 0x57, 0x46])
            || rom.get((p.commit - 0x08000000) as usize..(p.commit - 0x08000000) as usize + 4)
                != Some(&[0, 0x99, 0x88, 0x89])
        {
            return Self::default();
        }
        Self {
            shop:crate::ffta_shop_mouse::State::new(rom,ffta_rom::version(rom).unwrap()),
            profile: Some(p),
            world_profile: world_profile(rom,ffta_rom::version(rom).unwrap()),
            menu_profile: MenuProfile::new(rom, ffta_rom::version(rom).unwrap()),
            enabled: std::env::var("FFTA_MOUSE").as_deref() == Ok("1"),
            ..Default::default()
        }
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.cancel();self.mouse_visual=false;self.pub_scroll_neutral=false;self.pub_scroll_closed=false;
        self.enabled=enabled && self.profile.is_some();
    }
    pub fn enable_replay(&mut self) -> bool {
        self.enabled = self.profile.is_some();
        self.enabled
    }
    pub fn cancel(&mut self) {
        self.dialogue_advance=None;self.dialogue_answer=None;
        self.shop.cancel();self.roster.cancel();
        self.world_request=None;
        self.request = None;
        self.menu_request = None;
        self.pub_scroll=None;
        self.buttons.clear();
        self.button_hold=0;
        self.button_release=0;
        self.facing_key=None;
        self.event = None;
        self.pulse = 0;
        self.release = 0;
    }
    pub fn take_event(&mut self) -> Option<u16> {
        self.event.take()
    }
    pub(crate) fn record_shop_event(&mut self,event:u16){self.event=Some(event);}
    pub(crate) fn queue_shop_key(&mut self,key:u16){
        self.request=None;self.menu_request=None;self.world_request=None;self.shop.cancel();
        self.pulse=0;self.release=0;
        if self.buttons.len()<16 {self.buttons.push_back(key);}
    }
    pub fn interprets(&self, pc: u32) -> bool {
        self.enabled
            && (self.roster.interprets(pc) || (self.bank_redrawing && (0x0805b624..0x0805c2b8).contains(&pc))
                || ((self.dialogue_advance.is_some() || self.dialogue_answer.is_some()) && (0x0801194c..0x08011bfc).contains(&pc))
                || (self.pub_scroll.is_some() && ((0x0806196c..0x080619d6).contains(&pc)
                || (0x080619d8..0x08061a88).contains(&pc)
                || (0x080540a8..0x080547e0).contains(&pc)
                || (0x0805b624..0x0805c2b8).contains(&pc))) || self.shop.interprets(pc) || (self.request.is_some_and(|r| r.committed.is_none())
                && self.profile.is_some_and(|p| (p.start..p.end).contains(&pc)))
                || (self.menu_request.is_some()
                    && self.menu_profile.is_some_and(|p| (p.start..p.end).contains(&pc)))
                || (self.world_request.is_some()
                    && self.world_profile.is_some_and(|p|(p.start..p.end).contains(&pc)))
                || (self.menu_request.is_some() && self.menu_profile.is_some_and(|p| {
                    let start=if p.cursor==0x08018fc5 {0x08018cac}else{0x08017b68};
                    let setter=(p.cursor&!1)+0x130;
                    (start..start+0x34).contains(&pc) || (setter..setter+12).contains(&pc)
                        || (p.cursor==0x08018fc5 && ((0x0806196c..0x08061978).contains(&pc)
                            || (0x080540a8..0x080547e0).contains(&pc)
                || (0x0805b624..0x0805c2b8).contains(&pc)))
                })))
    }
}
impl Scene {
    pub fn read(mem: &MemMap) -> Option<Self> {
        let state = &mem.ffta_battle_mouse;
        let p = state.profile.filter(|_| state.enabled)?;
        if MenuScene::read(mem).is_some() {
            return None;
        }
        let display=h(&mem.io,0)?;
        // Story text owns A/B even when a suspended field cursor retains its mode.
        if display&0xc00==0xc00 && h(&mem.io,12)?==0x0b08 && h(&mem.io,14)?==0x0a08 {
            return None;
        }
        if display & 0x1007 != 0x1000
            || !((display&0xc00==0xc00 && h(&mem.io, 12)? == 0x4c03 && h(&mem.io, 14)? == 0x4e02)
                || (display&0x300==0x300 && h(&mem.io, 8)? == 0x4c03 && h(&mem.io, 10)? == 0x4e02))
        {
            return None;
        }
        let b = p.base;
        let owner = w(&mem.ewram, b)?;
        offset(owner, 0x30)?;
        let mode = *mem.ewram.get(b + 4)?;
        if !matches!(mode, 1 | 2) {
            return None;
        }
        let width = *mem.ewram.get(0x7f18)?;
        let height = *mem.ewram.get(0x7f19)?;
        if width == 0 || height == 0 || width > 16 || height > 16 {
            return None;
        }
        let table = offset(w(&mem.ewram, 0x7f14)?, width as usize * height as usize * 2)?;
        let min_x = mem.ewram[0x7f1a];
        let min_y = mem.ewram[0x7f1c];
        let bound_w = mem.ewram[0x7f1d];
        let bound_h = mem.ewram[0x7f1f];
        if bound_w == 0
            || bound_h == 0
            || min_x as u16 + bound_w as u16 > width as u16
            || min_y as u16 + bound_h as u16 > height as u16
        {
            return None;
        }
        let selected = (h(&mem.ewram, b + 12)? as u8, h(&mem.ewram, b + 16)? as u8);
        if selected.0 >= width || selected.1 >= height {
            return None;
        }
        let ox = h(&mem.ewram, 0x7f60)? as i16 as i32 - h(&mem.ewram, 0x7f64)? as i16 as i32;
        let oy = h(&mem.ewram, 0x7f62)? as i16 as i32 - h(&mem.ewram, 0x7f66)? as i16 as i32;
        let mut cells = Vec::new();
        for y in min_y..min_y + bound_h {
            for x in min_x..min_x + bound_w {
                let i = table + 2 * (y as usize * width as usize + x as usize);
                let elevation = mem.ewram[i];
                if elevation == 0 || mem.ewram[i + 1] & 8 != 0 {
                    continue;
                }
                let range = range_value(mem, x, y);
                cells.push(Cell {
                    x,
                    y,
                    height: elevation,
                    range,
                    center: [
                        (16 * (x as i32 - y as i32) + ox) as f32,
                        (8 * (x as i32 + y as i32 + 1 - elevation as i32) + oy) as f32,
                    ],
                });
            }
        }
        Some(Self {
            width,
            height,
            selected,
            cells,
            base: b,
        })
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<Cell> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        self.cells
            .iter()
            .filter(|c| (x - c.center[0]).abs() / 16. + (y - c.center[1]).abs() / 8. < 1.)
            .max_by_key(|c| (c.x as u16 + c.y as u16, c.height))
            .copied()
    }
}
/// Exact packed-quarter style query from USA0801D58C / EUR0801E6FC.
/// Read-only selected cell for visual HUD identity. Available independently
/// of mouse mode, so keyboard and gamepad use the same replacement policy.
pub fn selected_cell(mem:&MemMap)->Option<(u8,u8)> {
    let p=mem.ffta_battle_mouse.profile?;
    let width=*mem.ewram.get(0x7f18)?;let height=*mem.ewram.get(0x7f19)?;
    if width==0 || height==0 || width>16 || height>16{return None;}
    let x=h(&mem.ewram,p.base+12)?;let y=h(&mem.ewram,p.base+16)?;
    (x<width as u16 && y<height as u16).then_some((x as u8,y as u8))
}
pub fn range_value(mem: &MemMap, x: u8, y: u8) -> u8 {
    if x >= 16 || y >= 16 || mem.ewram[0x8168] == 0 || mem.ewram[0x8169] != 3 {
        return 0;
    }
    let i = y as usize * 16 + x as usize;
    let a = mem.ewram[0x8190 + i];
    let b = mem.ewram[0x8290 + i];
    let v = if a != 0 { a } else { b };
    for shift in [6, 4, 2, 0] {
        let n = (v >> shift) & 3;
        if n != 0 {
            return n;
        }
    }
    0
}
pub fn select(mem: &mut MemMap, x: u8, y: u8, confirm: bool) -> bool {
    let Some(scene) = Scene::read(mem) else {
        return false;
    };
    if !scene.cells.iter().any(|c| c.x == x && c.y == y) {
        return false;
    }
    let state = &mut mem.ffta_battle_mouse;
    state.menu_request = None;
    if let Some(r) = state.request.as_mut() {
        if (r.x, r.y) == (x, y) && r.base == scene.base {
            r.confirm |= confirm;
            r.deadline = mem.frames + 120;
            return true;
        }
    }
    state.pulse = 0;
    state.release = 0;
    state.request = Some(Request {
        x,
        y,
        base: scene.base,
        deadline: mem.frames + 120,
        confirm,
        wake_sent: false,
        committed: if scene.selected == (x, y) {
            Some(mem.frames)
        } else {
            None
        },
    });
    if scene.selected != (x, y) {
        state.event = Some(256 + y as u16 * 16 + x as u16);
    }
    true
}
/// Map destination lists use a wider cursor model than their blue backing.
/// Gate on the backing's actual nontransparent pixels, not that cursor width.
fn map_menu_contains_blue(mem:&MemMap,scene:MenuScene,x:f32,y:f32)->bool {
    if w(&mem.ewram,scene.base)!=Some(0x080397ed)
        || h(&mem.io,8)!=Some(0xaa03) || h(&mem.io,10)!=Some(0xac02){return true;}
    if !x.is_finite() || !y.is_finite() || !(0. ..240.).contains(&x) || !(0. ..160.).contains(&y){return false;}
    let Some(cnt)=h(&mem.io,12)else{return false;};
    let px=(x as usize+h(&mem.io,0x18).unwrap_or(0)as usize)%if cnt&0x4000!=0{512}else{256};
    let py=(y as usize+h(&mem.io,0x1a).unwrap_or(0)as usize)%if cnt&0x8000!=0{512}else{256};
    let block=px/256+py/256*if cnt&0x4000!=0{2}else{1};
    let off=((cnt>>8)&31)as usize*0x800+block*0x800+((py%256/8)*32+px%256/8)*2;
    let Some(tile)=h(&mem.vram,off)else{return false;};
    if tile>>12!=12{return false;}
    let tx=if tile&0x400!=0{7-px%8}else{px%8};
    let ty=if tile&0x800!=0{7-py%8}else{py%8};
    let chars=((cnt>>2)&3)as usize*0x4000;
    if cnt&0x80!=0 {mem.vram.get(chars+(tile&1023)as usize*64+ty*8+tx).is_some_and(|p|*p!=0)}
    else{mem.vram.get(chars+(tile&1023)as usize*32+ty*4+tx/2).is_some_and(|p|(p>>(tx%2*4))&15!=0)}
}
/// Item/ability award on the world map (for example SWAP / Copycat / OK).
/// Its framed resource is not a destination list; the only action is confirm.
pub fn map_award_bounds(mem:&MemMap)->Option<[i32;4]> {
    if !mem.ffta_battle_mouse.enabled || !mem.ffta_movement.europe_profile
        || [8,10,12,14].map(|o|h(&mem.io,o))!=[Some(0xaa03),Some(0xac02),Some(0x2e09),Some(0x2f08)]
        || h(&mem.io,0)?&0x0f07!=0x0f00 {return None;}
    let tile=|x:usize,y:usize|h(&mem.vram,0x7800+(y*32+x)*2);
    let mut result=None;
    for y in 0..16 {for x in 0..28 {
        if tile(x,y)!=Some(0xc005) || tile(x,y+2)!=Some(0xf12b)
            || tile(x,y+3)!=Some(0xf12d){continue;}
        for r in x+4..30 {if tile(r,y)!=Some(0xc405){continue;}
            for b in y+4..20 {if tile(x,b)==Some(0xc009)&&tile(r,b)==Some(0xc409){
                if result.is_some(){return None;}
                result=Some([x as i32*8,y as i32*8,(r+1)as i32*8,(b+1)as i32*8]);
            }}
        }
    }}
    result
}
pub fn aim(mem: &mut MemMap, x: f32, y: f32, confirm: bool) -> bool {
    // Facing is a world-space prompt, never a selectable UI rectangle.
    if let Some(prompt)=FacingPrompt::read(mem) { return prompt.aim(mem,x,y,confirm); }
    if let Some([l,t,r,b])=map_award_bounds(mem) {
        if let Some(scene)=MenuScene::read(mem) {
            return if let Some(index)=scene.hit(x,y){menu_select(mem,index,confirm)}
                else{mem.ffta_battle_mouse.cancel();false};
        }
        mem.ffta_battle_mouse.cancel();
        let inside=x.is_finite() && y.is_finite() && x>=(l+8)as f32
            && x<(r-8)as f32 && y>=(t+32)as f32 && y<(b-8)as f32;
        if inside && confirm {press(mem,0x3fe);}
        return inside;
    }
    if mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice && mem.ffta_dialogue.extra.active() {
        return if let Some(scene)=MenuScene::read(mem) {
            if let Some(index)=scene.hit(x,y){menu_select(mem,index,confirm)}else{mem.ffta_battle_mouse.cancel();false}
        }else{mem.ffta_battle_mouse.cancel();false};
    }
    if crate::ffta_roster_mouse::Scene::read(mem).is_some(){return crate::ffta_roster_mouse::aim(mem,x,y,confirm);}
    if crate::ffta_shop_mouse::aim(mem,x,y,confirm){return true;}
    if world_cursor(mem).is_some() {return world_select(mem,x,y,confirm);}
    if let Some(scene) = MenuScene::read(mem) {
        if !map_menu_contains_blue(mem,scene,x,y){mem.ffta_battle_mouse.cancel();return false;}
        return if let Some(index) = scene.hit(x,y) { menu_select(mem,index,confirm) }
        else { mem.ffta_battle_mouse.cancel(); false };
    }
    let cell = Scene::read(mem).and_then(|s| s.hit(x, y));
    let Some(c) = cell else {
        mem.ffta_battle_mouse.cancel();
        return false;
    };
    select(mem, c.x, c.y, confirm)
}
pub fn back(mem: &mut MemMap) {
    mem.ffta_battle_mouse.pub_scroll_closed=true;
    crate::ffta_roster_mouse::dismiss(mem);
    press(mem,0x3fd);
}
fn world_profile(rom:&[u8],version:Version)->Option<Profile> {
    let (start,x,y)=match version {Version::Europe=>(0x38994,0x38b3a,0x38c0e),Version::Usa=>(0x37428,0x375ce,0x376a2)};
    if rom.get(start..start+12)!=Some(&[0xf0,0xb5,0x16,0x4a,0x10,0x88,0xf0,0x26,6,0x40,0x15,0x49])
        || rom.get(x..x+2)!=Some(&[0xe0,0x80]) || rom.get(y..y+2)!=Some(&[0x20,0x81]) {return None;}
    Some(Profile{base:0,start:0x08000000+start as u32,commit:0x08000000+x as u32,end:0x08000000+y as u32+8})
}
/// World-map cursor: the original object owns world X/Y and vertical scroll.
/// Its screen sprite is a derived result, not the navigation state.
fn world_cursor(mem:&MemMap)->Option<(usize,[u16;2],u16)> {
    let s=&mem.ffta_battle_mouse;
    if !s.enabled || s.world_profile.is_none() || h(&mem.io,0)?&0x0f07!=0x0f00
        || h(&mem.io,8)?!=0xaa03 || h(&mem.io,10)?!=0xac02 {return None;}
    let manager=offset(w(&mem.iwram,0x2810)?,0xa0)?;
    let base=offset(w(&mem.ewram,manager+4)?,0x90)?;
    // 0x20 is also present while hovering a visitable destination. The
    // original movement handler still accepts input in that state.
    if mem.ewram[base]&3!=1 {return None;}
    let x=h(&mem.ewram,base+6)?;let y=h(&mem.ewram,base+8)?;
    let scroll=h(&mem.ewram,base+12)?;
    if !(14..=224).contains(&x) || !(22..=330).contains(&y) || scroll>330 {return None;}
    Some((base,[x,y],scroll))
}
pub fn world_select(mem:&mut MemMap,x:f32,y:f32,confirm:bool)->bool {
    let Some((base,current,scroll))=world_cursor(mem) else{return false;};
    if !x.is_finite() || !y.is_finite() || !(0.0..240.0).contains(&x) || !(0.0..160.0).contains(&y){return false;}
    let x=x.round().clamp(14.,224.)as u8;let y=y.round().clamp(0.,159.)as u8;
    let s=&mut mem.ffta_battle_mouse;
    if let Some(r)=s.world_request.as_mut(){if r.base==base && r.x==x && r.y==y {r.confirm|=confirm;r.deadline=mem.frames+120;return true;}}
    s.cancel();
    let same=current==[x as u16,(y as u16+scroll).clamp(22,330)];
    s.world_request=Some(Request{x,y,base,confirm,deadline:mem.frames+120,wake_sent:false,committed:same.then_some(mem.frames)});
    if !same{s.event=Some(4096+y as u16*240+x as u16);}
    true
}
/// Plain A/B also work in dialogue, forecasts and unsupported menu contexts.
pub fn press(mem:&mut MemMap,key:u16)->bool {press_sequence(mem,&[key])}
fn press_sequence(mem:&mut MemMap,keys:&[u16])->bool {
    if !mem.ffta_battle_mouse.enabled {return false;}
    let state=&mut mem.ffta_battle_mouse;
    state.cancel();state.buttons.extend(keys.iter().copied());true
}
/// User reference: a horizontal/vertical cross through the actor's feet.
/// Each quadrant owns one projected facing direction, without rotation or
/// unequal weights. The cross is logical only; no overlay is drawn.
#[cfg(test)]
fn facing_direction(center:[f32;2],x:f32,y:f32)->Option<u16> {
    stable_facing_direction(center,x,y,None)
}
fn stable_facing_direction(center:[f32;2],x:f32,y:f32,previous:Option<u16>)->Option<u16> {
    let dx=x-center[0]; let dy=y-center[1];
    if !dx.is_finite() || !dy.is_finite() {return None;}
    if dx*dx+dy*dy<36. {return previous;}
    Some(if dy>=0. {
        if dx>=0. {0x3ef}else{0x37f}
    } else if dx<0. {0x3df}else{0x3bf})
}
/// A validated orientation prompt, read once per pointer event.
#[derive(Clone,Copy,Debug)]
pub struct FacingPrompt { center:[f32;2] }
impl FacingPrompt {
    pub fn read(mem:&MemMap)->Option<Self> {facing_ground_center(mem).map(|center|Self{center})}
    pub fn center(self)->[f32;2] {self.center}
    pub fn aim(self,mem:&mut MemMap,x:f32,y:f32,confirm:bool)->bool {
    if !x.is_finite() || !y.is_finite() {return false;}
    // Once accepted, later mouse motion must not replace the queued A pulse.
    if mem.ffta_battle_mouse.buttons.contains(&0x3fe)
        || (mem.ffta_battle_mouse.button_key==0x3fe && mem.ffta_battle_mouse.button_hold>0) {return true;}
    let direction=stable_facing_direction(self.center,x,y,mem.ffta_battle_mouse.facing_key);
    if confirm {
        let mut sequence=Vec::with_capacity(2);
        if let Some(key)=direction {sequence.push(key);}
        sequence.push(0x3fe);
        press_sequence(mem,&sequence);
        mem.ffta_battle_mouse.facing_key=direction;
    } else if let Some(key)=direction {
        if mem.ffta_battle_mouse.facing_key!=Some(key) {
            press_sequence(mem,&[key]);
            mem.ffta_battle_mouse.facing_key=Some(key);
        }
    }
    true
    }
}
pub fn aim_facing(mem:&mut MemMap,x:f32,y:f32,confirm:bool)->bool {
    FacingPrompt::read(mem).is_some_and(|prompt|prompt.aim(mem,x,y,confirm))
}
pub fn facing_center(mem:&MemMap)->Option<[f32;2]> {
    FacingPrompt::read(mem).map(FacingPrompt::center)
}
/// Validate the animated four-arrow prompt, but anchor input to the ground
/// cell of the actor. The arrows float above the head and are not its origin.
fn facing_ground_center(mem:&MemMap)->Option<[f32;2]> {
    let s=&mem.ffta_battle_mouse;let p=s.profile.filter(|_|s.enabled)?;
    if *mem.ewram.get(p.base+4)?!=0 {return None;}
    // Fixed storage avoids allocating a Vec on every pointer/frame query.
    let mut pieces=[(0i32,0i32,0u16,false);4];let mut n=0;
    for o in mem.oam.chunks_exact(8) {
        let a=h(o,0)?;let b=h(o,2)?;let c=h(o,4)?;
        if a&0xff00==0 && b&0xee00==0 && matches!(c,0x135..=0x13a) {
            if n==4 {return None;}
            pieces[n]=((b&511)as i32,(a&255)as i32,c,b&0x1000!=0);n+=1;
        }
    }
    if n!=4 {return None;}
    let &(x,y,_,_)=pieces.iter().find(|v|(0x135..=0x138).contains(&v.2)&&!v.3)?;
    for (ex,ey,flip,top) in [(x,y,false,true),(x+16,y,true,true),(x,y+8,false,false),(x+16,y+8,true,false)] {
        if !pieces.iter().any(|&(px,py,t,f)|px==ex && py==ey && f==flip
            && if top {(0x135..=0x138).contains(&t)}else{(0x139..=0x13a).contains(&t)}) {return None;}
    }
    let width=*mem.ewram.get(0x7f18)? as usize;
    let height=*mem.ewram.get(0x7f19)? as usize;
    let cell_x=h(&mem.ewram,p.base+12)? as usize;
    let cell_y=h(&mem.ewram,p.base+16)? as usize;
    if width==0 || height==0 || width>16 || height>16 || cell_x>=width || cell_y>=height {return None;}
    let table=offset(w(&mem.ewram,0x7f14)?,width*height*2)?;
    let elevation=*mem.ewram.get(table+2*(cell_y*width+cell_x))? as i32;
    if elevation==0 {return None;}
    let ox=h(&mem.ewram,0x7f60)? as i16 as i32-h(&mem.ewram,0x7f64)? as i16 as i32;
    let oy=h(&mem.ewram,0x7f62)? as i16 as i32-h(&mem.ewram,0x7f66)? as i16 as i32;
    Some([(16*(cell_x as i32-cell_y as i32)+ox)as f32,
        (8*(cell_x as i32+cell_y as i32+1-elevation)+oy)as f32])
}
pub fn keys(mem: &mut MemMap, physical: u16) -> u16 {
    crate::ffta_roster_mouse::tick(mem);
    if physical&2==0{crate::ffta_roster_mouse::dismiss(mem);}
    if physical != RELEASED {
        if physical&2==0 {mem.ffta_battle_mouse.pub_scroll_closed=true;}
        mem.ffta_battle_mouse.pub_scroll_neutral=false;
        mem.ffta_battle_mouse.cancel();
        return physical;
    }
    if !mem.ffta_battle_mouse.enabled {
        return physical;
    }
    if let Some((base,deadline))=mem.ffta_battle_mouse.dialogue_advance {
        if mem.frames<=deadline && dialogue_wait(mem)==Some(base) {return physical;}
        mem.ffta_battle_mouse.dialogue_advance=None;
    }
    if let Some((base,deadline))=mem.ffta_battle_mouse.dialogue_answer {
        if mem.frames<=deadline && DialogueChoice::read(mem).is_some_and(|c|c.base==base){return physical;}
        mem.ffta_battle_mouse.dialogue_answer=None;
    }
    if let Some(key)=crate::ffta_shop_mouse::keys(mem){return key;}
    {
        let s=&mut mem.ffta_battle_mouse;
        if s.button_hold>0 {s.button_hold-=1;return s.button_key;}
        if s.button_release>0 {s.button_release-=1;return physical;}
        if let Some(key)=s.buttons.pop_front(){s.button_key=key;s.button_hold=3;s.button_release=4;return key;}
    }
    if let Some(key)=crate::ffta_roster_mouse::keys(mem){return key;}
    if facing_center(mem).is_some(){return physical;}
    if map_award_bounds(mem).is_some() {
        if let Some(scene)=MenuScene::read(mem){return menu_keys(mem,scene,physical);}
    }
    if let Some((base,_,_))=world_cursor(mem) {
        let frame=mem.frames;let s=&mut mem.ffta_battle_mouse;
        if s.pulse>0 {s.pulse-=1;return s.key;}
        if s.release>0 {s.release-=1;return physical;}
        if let Some(r)=s.world_request {
            if r.base!=base || frame>r.deadline {s.cancel();return physical;}
            if let Some(f)=r.committed {
                if frame>=f+2 {s.world_request=None;if r.confirm{s.key=0x3fe;s.pulse=3;s.release=4;return s.key;}}
                return physical;
            }
            return 0x36f; // wake both original X and Y stores
        }
        return physical;
    }
    mem.ffta_battle_mouse.facing_key=None;
    if let Some(list)=pub_list(mem).or_else(||battle_list(mem)) {
        if let Some((base,target,deadline,phase))=mem.ffta_battle_mouse.pub_scroll {
            if base!=list.cursor || mem.frames>deadline || target==list.first {mem.ffta_battle_mouse.pub_scroll=None;return physical;}
            // The guest polls this menu less often than the host renders.
            // One-frame alternating edges can fall entirely between polls.
            // Keep both press and release visible for four guest frames.
            if battle_list(mem).is_some() {
                if let Some(scene)=MenuScene::read(mem){
                    let edge=if target>list.first{scene.count.saturating_sub(1)}else{0};
                    commit_window_menu(mem,scene,edge);
                }
            }
            mem.ffta_battle_mouse.pub_scroll=Some((base,target,deadline,(phase+1)%8));
            return if phase>=4 {physical}else if target>list.first {0x37f}else{0x3bf};
        }
    }else if crate::ffta_shop_mouse::list(mem).is_none(){mem.ffta_battle_mouse.pub_scroll_neutral=false;mem.ffta_battle_mouse.pub_scroll_closed=false;}
    if let Some(scene) = MenuScene::read(mem) {
        return menu_keys(mem,scene,physical);
    }
    if mem.ffta_battle_mouse.menu_request.is_some() {
        mem.ffta_battle_mouse.cancel();
    }
    let Some(scene) = Scene::read(mem) else {
        mem.ffta_battle_mouse.cancel();
        return physical;
    };
    let frame = mem.frames;
    let s = &mut mem.ffta_battle_mouse;
    if s.pulse > 0 {
        s.pulse -= 1;
        return s.key;
    }
    if s.release > 0 {
        s.release -= 1;
        return physical;
    }
    let Some(r) = s.request else {
        return physical;
    };
    if frame > r.deadline || r.base != scene.base {
        s.cancel();
        return physical;
    }
    if r.committed.is_none() {
        if r.wake_sent {
            return physical;
        }
        s.request.as_mut().unwrap().wake_sent = true;
        return 0x3ef;
    }
    if let Some(committed) = r.committed {
        if scene.selected != (r.x, r.y) {
            s.cancel();
            return physical;
        }
        if r.confirm && frame >= committed + 2 {
            s.request = None;
            s.key = 0x3fe;
            s.pulse = 3;
            s.release = 4;
            return s.key;
        }
    }
    physical
}
/// A recorded virtual Right must wake the original cursor routine without
/// being mistaken for a new physical input that cancels its queued destination.
pub fn replay_keys(mem: &mut MemMap) {
    if (!mem.ffta_battle_mouse.roster.pending() && !mem.ffta_battle_mouse.shop.pending() && mem.ffta_battle_mouse.request.is_none() && mem.ffta_battle_mouse.menu_request.is_none() && mem.ffta_battle_mouse.world_request.is_none()
        && mem.ffta_battle_mouse.pub_scroll.is_none()
        && mem.ffta_battle_mouse.dialogue_advance.is_none()
        && mem.ffta_battle_mouse.pulse == 0 && mem.ffta_battle_mouse.release == 0
        && mem.ffta_battle_mouse.buttons.is_empty() && mem.ffta_battle_mouse.button_hold==0
        && mem.ffta_battle_mouse.button_release==0) || mem.keys != RELEASED {
        return;
    }
    // Drive both the wake-up pulse and the deferred confirm pulse in native
    // and interpreter replays. The windowed runner calls `keys` every frame;
    // headless replay must mirror that state machine after a mouse event.
    mem.keys = keys(mem, RELEASED);
}
/// Original paginated model: total entries and first visible index are not
/// inferred from blinking scroll arrows or the number of currently drawn labels.
#[derive(Clone,Copy,Debug)]
pub struct PubList {pub cursor:usize,pub total:u16,pub visible:u16,pub first:u16}
pub fn set_mouse_visual(mem:&mut MemMap,active:bool){mem.ffta_battle_mouse.mouse_visual=active && mem.ffta_battle_mouse.enabled;}
pub fn mouse_visual(mem:&MemMap)->bool{mem.ffta_battle_mouse.enabled && mem.ffta_battle_mouse.mouse_visual}
pub fn pub_scroll_visible(mem:&MemMap)->bool{!mem.ffta_battle_mouse.pub_scroll_closed}
pub fn hide_mouse_indicator(mem:&MemMap,a:[u16;3])->bool {
    if !mouse_visual(mem){return false;}
    let map=h(&mem.io,0).is_some_and(|v|v&0x0f07==0x0f00) && h(&mem.io,8)==Some(0xaa03) && h(&mem.io,10)==Some(0xac02);
    // Measured map MENU pointer (OBJ slot 4). The 09C8..09DF pieces are
    // the actor, not a pointer; rendering those must never be suppressed.
    // Free map feather: square 16x16, palette 0, alternating tiles 0028/002C (measured OAM 0).
    // It is distinct from the horizontal pointer used by destination menus.
    let feather=map && ((a[0]&0xff00==0x4000 && a[1]&0xc000==0 && a[2]==0)
        || (a[0]&0xff00==0 && a[1]&0xc000==0x4000 && matches!(a[2],0x0028|0x002c)));
    let title=h(&mem.io,0)==Some(0xf740) && h(&mem.io,8)==Some(0x3d83)
        && h(&mem.io,10)==Some(0x3e0a) && a[0]&0xff00==0x4000 && a[1]&0xc000==0x8000 && a[2]==0x40ee;
    let ui=&mem.ffta_dialogue;
    // Round 8x8 choice bullets are feedback, not sword navigation cursors.
    // Keep them even if a selector is between callbacks during mouse input.
    if crate::ffta_dialogue::list_marker(a)
        && (ui.confirmation_marker || a[0]&0xc000==0 && (0x124..=0x127).contains(&(a[2]&1023))) {return false;}
    let menu=ui.action_layout || ui.service_layout || ui.town_layout || ui.shop_layout || ui.site_layout
        || ui.extra.kind==crate::ffta_dialogue::PanelKind::Notice
        || (h(&mem.io,8)==Some(0x4c03) && h(&mem.io,10)==Some(0x4e02)
            && matches!(h(&mem.io,12),Some(0x0b08|0x5c08)));
    let sword=(menu || crate::ffta_roster_mouse::active(mem)) && crate::ffta_dialogue::list_marker(a) && a[2]&1023<=0x127
        && !crate::ffta_dialogue::continuation(a);
    let language=h(&mem.io,0)==Some(0x1140) && h(&mem.io,8)==Some(0x3000)
        && matches!(a[2],0x308c|0x3090);
    feather || title || sword || language
}
/// EUR dispatch table: state 6 waits for a page, 7 waits before continuing.
/// Choice state 9, external wait 8 and active printing are excluded.
fn verified_dialogue(mem:&MemMap)->Option<usize>{
    if !mem.ffta_battle_mouse.enabled
        || mem.rom.get(0x1194c..0x11954)!=Some(&[0x30,0xb5,0x03,0x48,0x03,0x68,0x05,0x1c])
        || mem.rom.get(0x11aa4..0x11ab0)!=Some(&[0x13,0x48,0x40,0x88,0x2b,0x68,0x1a,0x8c,0x02,0x40,0x00,0x2a])
        || mem.rom.get(0x11b28..0x11b32)!=Some(&[0x07,0x48,0x2a,0x68,0x41,0x88,0x10,0x8c,0x08,0x40]) {return None;}
    let base=offset(w(&mem.ewram,0x918c)?,0x44)?;
    (mem.ewram[base+9]<=11).then_some(base)
}
fn dialogue_wait(mem:&MemMap)->Option<usize>{
    let base=verified_dialogue(mem)?;
    (matches!(mem.ewram[base+8],6|7) && h(&mem.ewram,base+0x20)?&3!=0).then_some(base)
}
/// Only printing and text-page waits. Choices, external events, name input,
/// menus and battles are barriers; the skip never generates an A key.
pub fn skippable_dialogue(mem:&MemMap)->bool {
    verified_dialogue(mem).is_some_and(|base|matches!(mem.ewram[base+8],5|6|7))
}
/// Scripted EUR story sequence. Keep running through camera, walk, delay and
/// external waits, but never through an input choice or a live battle selector.
pub fn sequence_skip_barrier(mem:&MemMap)->bool {
    if !mem.ffta_battle_mouse.enabled {return true;}
    if verified_dialogue(mem).is_some_and(|b|mem.ewram[b+8]==9){return true;}
    // The field cursor becomes interactive at battle control, whereas story
    // actors and their movement callbacks may remain alive throughout scenes.
    Scene::read(mem).is_some() || MenuScene::read(mem).is_some()
        || FacingPrompt::read(mem).is_some()
}
/// Hold-to-skip is rearmed only by releasing the button. Hitting a barrier
/// consumes the hold, even if dialogue starts again while it remains down.
#[derive(Default)]
pub struct DialogueSkip {held_since:Option<f64>,active:bool,blocked:bool}
impl DialogueSkip {
    pub fn update(&mut self,held:bool,now:f64,skippable:bool)->bool {
        if !held {*self=Self::default();return false;}
        let start=*self.held_since.get_or_insert(now);
        if self.blocked{return false;}
        if self.active && !skippable {self.active=false;self.blocked=true;return false;}
        if now-start>=1.0 {
            if !skippable {self.blocked=true;return false;}
            self.active=true;
        }
        self.active
    }
}

/// Two-answer prompt positions come from the original cursor drawing routine.
#[derive(Clone,Copy,Debug)]
pub struct DialogueChoice {base:usize,pub bounds:[[i32;4];2],pub selected:u8}
impl DialogueChoice {
    pub fn read(mem:&MemMap)->Option<Self>{
        if !mem.ffta_battle_mouse.enabled
            || mem.rom.get(0x11b66..0x11b70)!=Some(&[0x09,0x4a,0x51,0x88,0x01,0x20,0x08,0x40,0x00,0x28])
            || mem.rom.get(0x13214..0x13224)!=Some(&[0x0a,0x78,0x02,0x31,0x09,0x78,0x0b,0x31,0x51,0x43,0xa2,0x8e,0x89,0x18,0xe2,0x8e]){return None;}
        let base=offset(w(&mem.ewram,0x918c)?,0x44)?;
        let selected=mem.ewram[base+0x41];
        if mem.ewram[base+8]!=9 || selected>1 || mem.ewram[base+0x42]>1{return None;}
        let x=h(&mem.ewram,base+0x34)?as i16 as i32;
        let y=h(&mem.ewram,base+0x36)?as i16 as i32;
        let step=mem.ewram[base+0x43]as i32+11;
        if x<0 || y<0 || y+16>160 || step<12 || x+2*step>240{return None;}
        Some(Self{base,selected,bounds:[[x,y,x+step,y+16],[x+step,y,x+2*step,y+16]]})
    }
    pub fn hit(self,x:f32,y:f32)->Option<u8>{
        self.bounds.iter().position(|[l,t,r,b]|x>=*l as f32&&x<*r as f32&&y>=*t as f32&&y<*b as f32).map(|i|i as u8)
    }
    pub fn select(self,mem:&mut MemMap,target:u8,confirm:bool)->bool{
        if target>1 || Self::read(mem).is_none_or(|c|c.base!=self.base){return false;}
        mem.ffta_battle_mouse.cancel();mem.ewram[self.base+0x41]=target;
        if confirm {
            if mem.ffta_dialogue.extra.kind==crate::ffta_dialogue::PanelKind::Notice {
                // NOTICE choices use the normal guest input path, like a real
                // A press. Do not depend on the dialogue interpreter PC hook.
                return press(mem,0x3fe);
            }
            mem.ffta_battle_mouse.dialogue_answer=Some((self.base,mem.frames+120));
        }
        true
    }
}
pub fn advance_dialogue(mem:&mut MemMap)->bool {
    let Some(base)=dialogue_wait(mem)else{return false;};
    mem.ffta_battle_mouse.cancel();
    mem.ffta_battle_mouse.dialogue_advance=Some((base,mem.frames+120));
    true
}
pub fn pub_scroll_unselected(mem:&MemMap)->bool {
    if crate::ffta_roster_mouse::unselected(mem) || crate::ffta_shop_mouse::unselected(mem){return true;}
    mem.ffta_battle_mouse.enabled && mem.ffta_battle_mouse.pub_scroll_neutral
        && (mem.ffta_dialogue.service_list && mem.ffta_dialogue.pub_window_owners[1].is_some()
            || mem.ffta_dialogue.action_layout)
}
pub fn pub_list(mem:&MemMap)->Option<PubList>{
    if !mem.ffta_battle_mouse.enabled || !mem.ffta_dialogue.service_list
        || mem.ffta_dialogue.pub_window_owners[1].is_none()
        || !matches!(crate::ffta_surface::Descriptor::read(mem)?.id,62|66|67){return None;}
    // The scrolling cursor's OAM position temporarily disagrees with its
    // logical row while the circular buffer advances. That visual cursor is
    // not the lifetime of the list (nor of its scrollbar/input request).
    let bounds=mem.ffta_dialogue.service_list_bounds?;
    let mut found=None;
    for cursor in (0x20..mem.ewram.len()-0x20).step_by(4) {
        let base=cursor-0x20;
        if w(&mem.ewram,cursor)!=Some(0x08018fc5)
            || w(&mem.ewram,cursor+4)!=Some(0x02000000+base as u32)
            || mem.ewram[cursor+16]&15!=3 {continue;}
        let total=h(&mem.ewram,base+0x18)?;let first=h(&mem.ewram,base+0x1a)?;
        let visible=mem.ewram[base+0x16]as u16;
        let x=mem.ewram[base+0x13]as i32*8;
        let y=h(&mem.ewram,cursor+10)?as i16 as i32;
        if total==0 || total>4096 || visible==0 || visible>16 || first>=total
            || x<bounds[0] || x>=bounds[2] || y<bounds[1] || y>=bounds[3] {continue;}
        if found.is_some(){return None;}
        found=Some(PubList{cursor,total,visible:visible.min(total),first});
    }
    found
}
/// Paged battle ability lists share the text-list component, not the Pub
/// window manager. Read its total/visible/first indices directly.
pub fn battle_list(mem:&MemMap)->Option<PubList>{
    if !mem.ffta_battle_mouse.enabled || !mem.ffta_dialogue.action_layout{return None;}
    let scene=MenuScene::read(mem)?;let cursor=scene.base;
    if w(&mem.ewram,cursor)!=Some(0x08018fc5) || mem.ewram.get(cursor+16)?&15!=7{return None;}
    let base=cursor.checked_sub(0x20)?;
    if w(&mem.ewram,cursor+4)!=Some(0x02000000+base as u32){return None;}
    let total=h(&mem.ewram,base+0x18)?;let first=h(&mem.ewram,base+0x1a)?;
    let visible=*mem.ewram.get(base+0x16)?as u16;
    if total==0 || total>256 || visible==0 || visible>12 || first>=total{return None;}
    Some(PubList{cursor,total,visible:visible.min(total),first})
}
pub fn pub_scroll_to(mem:&mut MemMap,first:u16)->bool{
    let Some(list)=pub_list(mem).or_else(||battle_list(mem))else{return false;};
    let first=first.min(list.total.saturating_sub(list.visible));
    let previous=mem.ffta_battle_mouse.pub_scroll.filter(|r|r.0==list.cursor);
    mem.ffta_battle_mouse.cancel();
    mem.ffta_battle_mouse.pub_scroll_neutral=true;
    mem.ffta_battle_mouse.pub_scroll=Some((list.cursor,first,mem.frames+600,previous.map_or(0,|r|r.3)));
    if let Some(scene)=MenuScene::read(mem){commit_window_menu(mem,scene,scene.selected);}
    true
}
pub fn pub_scroll_by(mem:&mut MemMap,up:bool)->bool{
    if map_award_bounds(mem).is_some() && MenuScene::read(mem).is_some() {
        // Let the original card-list updater page its data and redraw icons.
        press(mem,if up{0x3bf}else{0x37f});return true;
    }
    let Some(list)=pub_list(mem).or_else(||battle_list(mem))else{return false;};
    let first=mem.ffta_battle_mouse.pub_scroll.filter(|r|r.0==list.cursor).map_or(list.first,|r|r.1);
    pub_scroll_to(mem,if up{first.saturating_sub(1)}else{first.saturating_add(1)})
}
fn commit_window_menu(mem:&mut MemMap,mut scene:MenuScene,target:u8)->bool {
    // Monster Bank wraps the same text list in a parent object. Normalize
    // only this validated root to its owned cursor/list before publishing.
    let bank_list=pub_list(mem).is_some_and(|l|l.cursor==scene.base)
        && crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==66);
    let action_list=mem.ffta_movement.europe_profile && mem.ffta_dialogue.action_layout
        && w(&mem.ewram,scene.base)==Some(0x08018fc5)
        && mem.ewram[scene.base+16]&15==7;
    let bank_root=crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|s.id==66)
        && w(&mem.ewram,scene.base)==Some(0x0805aee5)
        && w(&mem.ewram,scene.base+4)==Some(0x0805e7b5)
        && scene.origin==[0,92] && scene.count==2;
    if bank_root {scene.base+=0x58;}

    let map=w(&mem.ewram,scene.base)==Some(0x080397ed);
    let store_root=w(&mem.ewram,scene.base)==Some(0x08018fc5)
        && crate::ffta_surface::Descriptor::read(mem).is_some_and(|s|matches!(s.id,63|65|67))
        && scene.origin==[8,92] && matches!(scene.count,3|4);
    if (!map && !store_root && !bank_root && !bank_list && !action_list) || target>=scene.count {return false;}
    let list=scene.base-0x20;
    let Some(address)=w(&mem.ewram,list+8)else{return false;};
    let width=mem.ewram[list+0x15]as usize;let stride=mem.ewram[list+0x1e]as usize;
    let rows=(scene.pitch/8)as usize;
    if width==0 || width>30 || stride<width || stride>64 || !(1..=3).contains(&rows)
        || (!bank_list && !action_list && !(map && map_award_bounds(mem).is_some()) && h(&mem.ewram,list+0x1a)!=Some(0)) {return false;}
    if action_list {
        // The ability builder republishes its packed RAM tilemap to VRAM.
        // Normalize that source too, otherwise hovered rows become orange
        // again on the next guest redraw, especially after scrolling.
        let source=w(&mem.ewram,list+12).unwrap_or(0);
        let total=h(&mem.ewram,list+0x18).unwrap_or(0)as usize;
        let first=h(&mem.ewram,list+0x1a).unwrap_or(0)as usize;
        if source>>24==2 && total>0 && total<=256 {
            let start=(source&0x3ffff)as usize;
            let size=total*rows*width*2;
            if start+size<=mem.ewram.len(){
                for row in 0..total {for i in 0..rows*width {
                    let at=start+(row*rows*width+i)*2;
                    let tile=u16::from_le_bytes([mem.ewram[at],mem.ewram[at+1]]);
                    if matches!(tile>>12,11|12){
                        let value=(tile&0xfff)|((if row==first+target as usize && !mem.ffta_battle_mouse.pub_scroll_neutral{11}else{12})<<12);
                        mem.ewram[at..at+2].copy_from_slice(&value.to_le_bytes());
                    }
                }}
            }
        }
    }
    let show_selection=!mem.ffta_battle_mouse.pub_scroll_neutral;
    let (data,offset)=match address>>24 {
        2=>(&mut mem.ewram,(address&0x3ffff)as usize),
        6=>(&mut mem.vram,(address&0x1ffff)as usize),
        _=>return false,
    };
    if offset+(scene.count as usize*rows-1)*stride*2+width*2>data.len(){return false;}
    // The original window copies this tilemap to VRAM. Change only label
    // palette banks B/C; item icons (D/E), frames and glyphs remain intact.
    for row in 0..scene.count as usize {for y in 0..rows {for x in 0..width {
        let at=offset+((row*rows+y)*stride+x)*2;
        let tile=u16::from_le_bytes([data[at],data[at+1]]);
        if matches!(tile>>12,11|12) {
            let tile=(tile&0x0fff)|((if row==target as usize && show_selection{11}else{12})<<12);
            data[at..at+2].copy_from_slice(&tile.to_le_bytes());
        }
    }}}
    mem.ewram[scene.base+17]=target;
    mem.ewram[scene.base+14..scene.base+16].copy_from_slice(&(target as u16*scene.pitch as u16).to_le_bytes());
    true
}
pub fn menu_select(mem: &mut MemMap, target: u8, confirm: bool) -> bool {
    let Some(scene) = MenuScene::read(mem) else { return false; };
    if target >= scene.count { return false; }
    mem.ffta_battle_mouse.pub_scroll_neutral=false;
    let mut direct=commit_window_menu(mem,scene,target);
    let state = &mut mem.ffta_battle_mouse;
    if let Some(r) = state.menu_request.as_mut() {
        if r.base == scene.base && r.target == target {
            r.confirm |= confirm;
            r.deadline = mem.frames + 120;
            return true;
        }
    }
    state.cancel();
    if crate::ffta_shop_mouse::select_item(mem,target){direct=true;}
    let state=&mut mem.ffta_battle_mouse;
    state.menu_request = Some(MenuRequest { base: scene.base, target, confirm,last_selected:scene.selected,
        context:[0,8,10,12,14].map(|o|h(&mem.io,o).unwrap_or(0)),
        committed: (direct || scene.selected == target).then_some(mem.frames), deadline: mem.frames+120 });
    if scene.selected != target && !scene.horizontal { state.event = Some(512+target as u16); }
    true
}
fn menu_keys(mem: &mut MemMap, scene: MenuScene, physical: u16) -> u16 {
    let frame = mem.frames;
    let state = &mut mem.ffta_battle_mouse;
    state.request = None;
    if state.pulse > 0 { state.pulse -= 1; return state.key; }
    if state.release > 0 { state.release -= 1; return physical; }
    let Some(r) = state.menu_request else { return physical; };
    if r.base != scene.base || frame > r.deadline || r.target >= scene.count {
        state.cancel(); return physical;
    }
    if (scene.binary || scene.polling) && r.committed.is_none() && scene.selected==r.target {
        state.menu_request.as_mut().unwrap().committed=Some(frame);
        state.release=2;
        return physical;
    }
    if scene.polling && r.committed.is_none() && scene.selected!=r.last_selected {
        // These menus consume press edges instead of repeating held input.
        // Release after every observed original step before asking for another.
        state.menu_request.as_mut().unwrap().last_selected=scene.selected;
        state.release=4;
        return physical;
    }
    if let Some(committed) = r.committed {
        if frame < committed+2 { return physical; }
        if scene.selected != r.target { state.cancel(); return physical; }
        state.menu_request = None;
        if r.confirm {
            state.key = 0x3fe; state.pulse = 3; state.release = 4;
            return state.key;
        }
    } else {
        // The menu polls on its own update cadence. Hold until its first
        // verified commit, then the entry hook suppresses any repeated step.
        return if scene.horizontal {if r.target>scene.selected {0x3ef}else{0x3df}}else if scene.polling && scene.selected>r.target {0x3bf}else{0x37f};
    }
    physical
}
impl Machine {
    fn ffta_shop_cursor_entry(&mut self) {
        let s=&self.bus.ffta_battle_mouse;
        let Some(p)=s.menu_profile.filter(|_|s.enabled)else{return;};
        let pc=(p.cursor&!1)+0x130;
        if self.cpu.regs[15]!=pc {return;}
        let Some(request)=s.menu_request.filter(|r|r.committed.is_none())else{return;};
        let Some(scene)=MenuScene::shop_menu(&self.bus).or_else(||MenuScene::shop_inventory(&self.bus))else{return;};
        if scene.base!=request.base || request.target>=scene.count || self.bus.frames>request.deadline
            || self.cpu.regs[0]!=0x02000000+scene.base as u32 {return;}
        let off=(pc-0x08000000)as usize;
        let caller=(self.cpu.regs[14]&!1).checked_sub(0x0800000a).map(|o|o as usize);
        if self.bus.rom.get(off..off+12)!=Some(&[0x09,0x06,0x09,0x0e,0x41,0x74,0x09,0x01,0xc1,0x81,0x70,0x47])
            || caller.and_then(|o|self.bus.rom.get(o..o+6))!=Some(&[0x48,0x46,0x4c,0x30,0x21,0x0e])
            || self.cpu.regs[9].checked_add(0x4c)!=Some(self.cpu.regs[0])
            || self.cpu.regs[6]!=self.cpu.regs[1] || self.cpu.regs[4]>>24!=self.cpu.regs[1] {return;}
        self.cpu.regs[1]=request.target as u32;
        self.cpu.regs[6]=request.target as u32;
        self.cpu.regs[4]=(request.target as u32)<<24;
        self.bus.ffta_battle_mouse.menu_request.as_mut().unwrap().committed=Some(self.bus.frames);
    }
    fn ffta_pub_cursor_entry(&mut self) {
        let s=&self.bus.ffta_battle_mouse;
        let Some(p)=s.menu_profile.filter(|_|s.enabled)else{return;};
        let pc=(p.cursor&!1)+0x130;
        if self.cpu.regs[15]!=pc {return;}
        let Some(request)=s.menu_request.filter(|r|r.committed.is_none())else{return;};
        let Some(scene)=MenuScene::pub_menu(&self.bus)else{return;};
        if scene.base!=request.base || request.target>=scene.count || self.bus.frames>request.deadline
            || self.cpu.regs[0]!=0x02000000+scene.base as u32 {return;}
        let off=(pc-0x08000000)as usize;
        let caller=(self.cpu.regs[14]&!1).checked_sub(0x08000000).map(|v|v as usize);
        // Original setter writes the cursor index and its row offset. Its
        // verified caller subsequently publishes the same index from r6.
        if self.bus.rom.get(off..off+12)!=Some(&[0x09,0x06,0x09,0x0e,0x41,0x74,0x09,0x01,0xc1,0x81,0x70,0x47])
            || caller.and_then(|o|self.bus.rom.get(o..o+8))!=Some(&[0x20,0x1c,0x3c,0x30,0x31,0x16,0x0b,0x22])
            || self.cpu.regs[6]>>24!=self.cpu.regs[1]&255 {return;}
        self.cpu.regs[1]=request.target as u32;
        self.cpu.regs[6]=(self.cpu.regs[6]&0x00ffffff)|((request.target as u32)<<24);
        self.bus.ffta_battle_mouse.menu_request.as_mut().unwrap().committed=Some(self.bus.frames);
    }
    fn ffta_pub_list_selection_entry(&mut self) {
        if self.bus.ffta_battle_mouse.pub_scroll.is_some() && self.cpu.regs[15]==0x08061a6e
            && pub_list(&self.bus).is_some()
            && self.bus.rom.get(0x61a6c..0x61a72)==Some(&[0x6d,0x20,0xe3,0xf0,0x49,0xfa]) {
            // Skip only the original list-navigation sound call, not audio.
            self.cpu.regs[15]=0x08061a72;
            return;
        }
        if self.cpu.regs[15]==0x080619ca {
            let Some(list)=pub_list(&self.bus)else{return;};
            let Some((base,target,deadline,_))=self.bus.ffta_battle_mouse.pub_scroll else{return;};
            if base!=list.cursor || self.bus.frames>deadline || target==list.first
                || self.cpu.regs[5].checked_add(0x5c)!=Some(0x02000000+list.cursor as u32)
                || self.bus.rom.get(0x619ca..0x619d6)!=Some(&[0x18,0xbc,0x98,0x46,0xa1,0x46,0xf0,0xbc,0x01,0xbc,0x00,0x47]) {return;}
            // Reuse the already-open original call frame. Populate every
            // intervening circular-buffer row before returning to the guest,
            // so a dragged thumb follows the pointer rather than key repeat.
            self.cpu.regs[4]=1;
            self.cpu.regs[6]=if target>list.first {1}else{u32::MAX};
            self.cpu.regs[7]=if target>list.first {0x80}else{0x40};
            // Restore the old highlight before the next page shift, exactly
            // as the normal input caller does. Its return resumes the same
            // page-update block without allocating another recursive frame.
            self.cpu.regs[0]=0x02000000+(list.cursor-0x20)as u32;
            self.cpu.regs[1]=self.bus.ewram[list.cursor+17]as u32;
            self.cpu.regs[2]=255;
            self.cpu.regs[14]=0x080619a9;
            self.cpu.regs[15]=0x08018c38;
            return;
        }
        if self.cpu.regs[15]!=0x0806196c{return;}
        let Some(list)=pub_list(&self.bus)else{return;};
        let state=&self.bus.ffta_battle_mouse;
        let target=if let Some(r)=state.menu_request.filter(|r|r.committed.is_none() && r.base==list.cursor && self.bus.frames<=r.deadline){r.target}
            else if let Some((base,target,deadline,_))=state.pub_scroll.filter(|r|r.0==list.cursor && self.bus.frames<=r.2){
                let _=(base,deadline);
                // Jump to the relevant edge immediately; subsequent original
                // page updates populate their circular glyph buffer normally.
                if target>list.first{list.visible.saturating_sub(1)as u8}else{0}
            }else{return;};
        if self.cpu.regs[0].checked_add(0x5c)!=Some(0x02000000+list.cursor as u32)
            || self.cpu.regs[3]!=0{return;}
        if self.bus.rom.get(0x6196c..0x61978)!=Some(&[0xf0,0xb5,0x4f,0x46,0x46,0x46,0xc0,0xb4,0x05,0x1c,0x1c,0x1c]){return;}
        // The caller has already restored the OLD row's palette. Change only
        // its proposed NEW row; let the original setter and repaint publish it.
        let proposed=self.cpu.regs[2];self.cpu.regs[2]=target as u32;
        if self.cpu.regs[4]==proposed{self.cpu.regs[4]=target as u32;}
    }
    // Card Keeper uses the shared list model but a separate page publisher.
    // Re-enter its original redraw inside the existing updater call frame.
    fn ffta_card_list_entry(&mut self) {
        let pc=self.cpu.regs[15];
        if !matches!(pc,0x08054320|0x08054276|0x08054380|0x08054694|0x080547de){return;}
        if crate::ffta_surface::Descriptor::read(&self.bus).is_none_or(|d|d.id!=67){return;}
        let Some(list)=pub_list(&self.bus)else{return;};
        if pc==0x08054694 {
            self.bus.ffta_battle_mouse.card_redrawing=true;
            self.bus.ffta_dialogue.shop_page_dirty=true;
            return;
        }
        if pc==0x080547de {self.bus.ffta_battle_mouse.card_redrawing=false;return;}
        if pc==0x08054380 {
            if let Some(r)=self.bus.ffta_battle_mouse.menu_request.filter(|r|r.base==list.cursor && self.bus.frames<=r.deadline) {
                if self.cpu.regs[0]==0x02000000+list.cursor as u32-0x20
                    && self.bus.rom.get(0x54380..0x54384)==Some(&[0xc4,0xf7,0x94,0xfc]) {
                    self.bus.ewram[list.cursor+17]=r.target;
                    self.bus.ewram[list.cursor+14..list.cursor+16].copy_from_slice(&(r.target as u16*16).to_le_bytes());
                    self.cpu.regs[1]=r.target as u32;
                    self.bus.ffta_battle_mouse.menu_request.as_mut().unwrap().committed=Some(self.bus.frames);
                }
            }
            return;
        }
        let Some((cursor,target,deadline,_))=self.bus.ffta_battle_mouse.pub_scroll else{return;};
        if cursor!=list.cursor || self.bus.frames>deadline{return;}
        if pc==0x08054276 {
            if self.bus.rom.get(0x54276..0x5427c)==Some(&[0x6d,0x20,0xf0,0xf0,0x44,0xfe]){self.cpu.regs[15]=0x0805427c;}
            return;
        }
        let Some(manager)=w(&self.bus.ewram,0xf428).and_then(|p|offset(p,0x1d46))else{return;};
        if list.cursor!=manager+0xd3c || target==list.first
            || self.cpu.regs[7]!=0x02000000+manager as u32+0xcec
            || self.bus.rom.get(0x54320..0x54328)!=Some(&[0x09,0x48,0x01,0x68,0x09,0x4a,0x88,0x18]){return;}
        self.bus.ewram[manager+0xd36..manager+0xd38].copy_from_slice(&target.to_le_bytes());
        self.cpu.regs[15]=0x08054262;
    }
    fn ffta_bank_list_entry(&mut self) {
        let pc=self.cpu.regs[15];
        if !matches!(pc,0x0805c1ec|0x0805c1de|0x0805c1e4){return;}
        if crate::ffta_surface::Descriptor::read(&self.bus).is_none_or(|s|s.id!=66){return;}
        if pc==0x0805c1e4 {self.bus.ffta_battle_mouse.bank_redrawing=false;return;}
        if pc==0x0805c1de && self.bus.ffta_battle_mouse.bank_redrawing {
            if self.bus.rom.get(0x5c1de..0x5c1e4)==Some(&[0x6d,0x20,0xe8,0xf0,0x90,0xfe]){self.cpu.regs[15]=0x0805c1e4;}
            return;
        }
        let Some(list)=pub_list(&self.bus)else{return;};
        let Some((cursor,target,deadline,_))=self.bus.ffta_battle_mouse.pub_scroll else{return;};
        if cursor!=list.cursor || self.bus.frames>deadline{return;}
        if pc==0x0805c1de {
            if self.bus.rom.get(0x5c1de..0x5c1e4)==Some(&[0x6d,0x20,0xe8,0xf0,0x90,0xfe]){self.cpu.regs[15]=0x0805c1e4;}
            return;
        }
        let Some(manager)=w(&self.bus.ewram,0xf428).and_then(|p|offset(p,0x1bb7))else{return;};
        if list.cursor!=manager+0xd40 || target==list.first
            || self.cpu.regs[7]!=0x02000000+manager as u32+0xcf0
            || self.bus.rom.get(0x5c1ec..0x5c1f4)!=Some(&[0x29,0x4d,0x28,0x68,0xd4,0x24,0x24,0x01]){return;}
        self.bus.ewram[manager+0x1bb5]=target as u8;
        self.bus.ffta_battle_mouse.bank_redrawing=true;
        self.bus.ffta_dialogue.shop_page_dirty=true;
        self.cpu.regs[15]=0x0805c0bc;
    }
    fn ffta_shared_list_entry(&mut self) {
        let s=&self.bus.ffta_battle_mouse;
        let Some(p)=s.menu_profile.filter(|_|s.enabled)else{return;};
        let pc=if p.cursor==0x08018fc5 {0x08018cac}else{0x08017b68};
        if self.cpu.regs[15]!=pc{return;}
        let Some(r)=s.menu_request else{return;};
        if self.bus.ffta_dialogue.service_list && self.bus.ffta_dialogue.pub_window_owners[1].is_some(){return;}
        let offset=(pc-0x08000000)as usize;
        if self.bus.rom.get(offset..offset+12)!=Some(&[0x10,0xb5,3,0x1c,9,6,9,0x16,0x58,0x8b,9,0x18]){return;}
        let Some(scene)=MenuScene::read(&self.bus)else{return;};
        if !scene.polling || scene.base!=r.base || self.cpu.regs[0]!=0x02000000+scene.base as u32-0x20{return;}
        if w(&self.bus.ewram,scene.base)==Some(0x080397ed) {return;}
        // The original selector adds a signed delta, clamps it, then its
        // caller updates the model, redraws the row and publishes the cursor.
        self.cpu.regs[1]=if r.committed.is_some(){0}else{(r.target as i32-scene.selected as i32)as u32};
    }
    fn ffta_world_mouse_entry(&mut self) {
        let s=&self.bus.ffta_battle_mouse;
        let Some(p)=s.world_profile.filter(|_|s.enabled)else{return;};
        let pc=self.cpu.regs[15];
        if pc!=p.commit && pc!=p.end-8{return;}
        let Some(r)=s.world_request else{return;};
        let Some((base,_,scroll))=world_cursor(&self.bus)else{return;};
        if r.base!=base || self.cpu.regs[4]!=0x02000000+base as u32 || self.bus.frames>r.deadline{return;}
        self.cpu.regs[0]=if pc==p.commit{r.x as u32}else{(r.y as u32+scroll as u32).clamp(22,330)};
        if pc==p.end-8 && r.committed.is_none() {self.bus.ffta_battle_mouse.world_request.as_mut().unwrap().committed=Some(self.bus.frames);}
    }
    pub fn ffta_battle_mouse_entry(&mut self) {
        if !self.cpu.thumb() {
            return;
        }
        let pc=self.cpu.regs[15];
        if matches!(pc,0x08011aac|0x08011b30) {
            if let Some((base,deadline))=self.bus.ffta_battle_mouse.dialogue_advance {
                let (state,pointer,key)=if pc==0x08011aac {(6,3,0)}else{(7,2,1)};
                if self.bus.frames<=deadline && dialogue_wait(&self.bus)==Some(base)
                    && self.bus.ewram[base+8]==state && self.cpu.regs[pointer]==0x02000000+base as u32 {
                    // Satisfy only this text routine's local input test.
                    // Neither hardware KEYINPUT nor shared key RAM receives A.
                    self.cpu.regs[key]|=h(&self.bus.ewram,base+0x20).unwrap_or(0)as u32&3;
                    self.bus.ffta_battle_mouse.dialogue_advance=None;
                }
            }
        }
        if pc==0x08011b6c {
            if let Some((base,deadline))=self.bus.ffta_battle_mouse.dialogue_answer {
                if self.bus.frames<=deadline && DialogueChoice::read(&self.bus).is_some_and(|c|c.base==base)
                    && self.cpu.regs[4]==0x0200918c && self.cpu.regs[2]==0x03000000 {
                    self.cpu.regs[1]|=1;
                    self.bus.ffta_battle_mouse.dialogue_answer=None;
                }
            }
        }
        self.ffta_world_mouse_entry();
        crate::ffta_roster_mouse::entry(self);
        crate::ffta_shop_mouse::entry(self);
        self.ffta_shop_cursor_entry();
        self.ffta_pub_cursor_entry();
        self.ffta_pub_list_selection_entry();
        self.ffta_card_list_entry();
        self.ffta_bank_list_entry();
        self.ffta_shared_list_entry();
        self.ffta_battle_menu_entry();
        let s = &self.bus.ffta_battle_mouse;
        let Some(p) = s.profile.filter(|_| s.enabled) else {
            return;
        };
        if self.cpu.regs[15] != p.commit {
            return;
        }
        let Some(r) = s.request.filter(|r| r.committed.is_none()) else {
            return;
        };
        let Some(scene) = Scene::read(&self.bus) else {
            self.bus.ffta_battle_mouse.cancel();
            return;
        };
        let sp = self.cpu.regs[13];
        let base = if sp >> 24 == 3 {
            w(&self.bus.iwram, (sp & 0x7fff) as usize)
        } else {
            None
        };
        if base != Some(0x02000000 + r.base as u32)
            || scene.base != r.base
            || self.bus.frames > r.deadline
        {
            self.bus.ffta_battle_mouse.cancel();
            return;
        }
        self.cpu.regs[10] = r.x as u32;
        self.cpu.regs[9] = r.y as u32;
        self.bus
            .ffta_battle_mouse
            .request
            .as_mut()
            .unwrap()
            .committed = Some(self.bus.frames);
        if crate::ffta_log::enabled() || std::env::var("FFTA_MOUSE_VERIFY").as_deref() == Ok("1") {
            eprintln!(
                "FFTA_MOUSE_BATTLE frame={} target={},{} pc={:08x} original_cursor_commit=true",
                self.bus.frames, r.x, r.y, p.commit
            );
        }
    }
    fn ffta_battle_menu_entry(&mut self) {
        let state = &self.bus.ffta_battle_mouse;
        let Some(p) = state.menu_profile.filter(|_| state.enabled) else { return; };
        if self.cpu.regs[15] == p.start {
            if let Some(r)=state.menu_request {
                if self.cpu.regs[0] == 0x02000000+r.base as u32 {
                    self.cpu.regs[1] &= !0xc0;
                    if r.committed.is_none() { self.cpu.regs[1] |= 0x80; }
                }
            }
            return;
        }
        if self.cpu.regs[15] != p.commit { return; }
        let Some(r) = state.menu_request.filter(|r| r.committed.is_none()) else { return; };
        let Some(scene) = MenuScene::read(&self.bus) else { self.bus.ffta_battle_mouse.cancel(); return; };
        if scene.base != r.base || self.cpu.regs[5] != 0x02000000+r.base as u32
            || self.cpu.regs[6] != self.cpu.regs[5]+0x58 || r.target >= scene.count
            || self.bus.frames > r.deadline { return; }
        self.cpu.regs[4] = r.target as u32;
        self.bus.ffta_battle_mouse.menu_request.as_mut().unwrap().committed = Some(self.bus.frames);
        if std::env::var("FFTA_MOUSE_VERIFY").as_deref() == Ok("1") {
            eprintln!("FFTA_MOUSE_MENU frame={} base={:08x} count={} target={} pc={:08x} original_commit=true",
                self.bus.frames,0x02000000+r.base as u32,scene.count,r.target,p.commit);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Machine {
        let mut m = Machine::new(vec![0; 192]);
        m.bus.ffta_battle_mouse = State {
            enabled: true,
            profile: Some(Profile {
                base: 0xf3ac,
                start: 0x08024ad8,
                commit: 0x08025132,
                end: 0x0802516e,
            }),
            ..Default::default()
        };
        for (o, v) in [(0, 0x1f40u16), (12, 0x4c03), (14, 0x4e02)] {
            m.bus.io[o..o + 2].copy_from_slice(&v.to_le_bytes());
        }
        m.bus.ewram[0xf3ac..0xf3b0].copy_from_slice(&0x02010000u32.to_le_bytes());
        m.bus.ewram[0xf3b0] = 1;
        m.bus.ewram[0x7f14..0x7f18].copy_from_slice(&0x02030000u32.to_le_bytes());
        m.bus.ewram[0x7f18] = 4;
        m.bus.ewram[0x7f19] = 3;
        m.bus.ewram[0x7f1d] = 4;
        m.bus.ewram[0x7f1f] = 3;
        for i in 0..12 {
            m.bus.ewram[0x30000 + i * 2] = 2;
        }
        m.cpu.cpsr = 0x3f;
        m.cpu.regs[13] = 0x03007000;
        m.bus.iwram[0x7000..0x7004].copy_from_slice(&0x0200f3acu32.to_le_bytes());
        m
    }
    #[test]
    fn free_map_feather_hides_only_with_mouse_and_preserves_actor_and_sites(){
        let mut m=fixture();
        for(o,v)in[(0,0x1f40u16),(8,0xaa03),(10,0xac02)]{m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let feather=[0x0059,0x409e,0x002c];
        set_mouse_visual(&mut m.bus,true);
        assert!(hide_mouse_indicator(&m.bus,feather));
        assert!(hide_mouse_indicator(&m.bus,[0x0059,0x409e,0x0028]),"other animation frame");
        for a in [[0x0053,0x4098,0x09d4],[0x4063,0x0098,0x09d8],[0x0021,0x8038,0x98d0]] {
            assert!(!hide_mouse_indicator(&m.bus,a));
        }
        set_mouse_visual(&mut m.bus,false);assert!(!hide_mouse_indicator(&m.bus,feather));
        assert!(!hide_mouse_indicator(&m.bus,[0x0059,0x409e,0x0028]));
        set_mouse_visual(&mut m.bus,true);
        m.bus.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        assert!(!hide_mouse_indicator(&m.bus,feather),"same tile outside map is untouched");
    }
    #[test]
    fn map_destination_hit_stops_at_blue_backing_not_wider_cursor_model(){
        let mut m=MemMap::new(vec![]);let base=0x1000;
        m.ewram[base..base+4].copy_from_slice(&0x080397edu32.to_le_bytes());
        for(o,v)in[(8,0xaa03u16),(10,0xac02),(12,0x2e09)] {m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.vram[0x8020..0x8040].fill(0x11);
        for y in 5..13 {for x in 4..26 {let o=0x7000+(y*32+x)*2;m.vram[o..o+2].copy_from_slice(&0xc001u16.to_le_bytes());}}
        let scene=MenuScene{pitch:16,count:3,selected:0,origin:[32,56],base,binary:false,horizontal:false,split:0,polling:true,right:224};
        for y in [56.,72.,88.] {
            for (x,inside) in [(31.,false),(32.,true),(100.,true),(207.,true),(208.,false),(223.,false)] {
                assert_eq!(scene.hit(x,y).is_some() && map_menu_contains_blue(&m,scene,x,y),inside);
            }
        }
    }
    #[test]
    fn dialogue_click_is_consumed_only_by_text_wait_without_a_key(){
        let mut m=fixture();let base=0x24000;
        m.bus.rom.resize(0x12000,0);
        m.bus.rom[0x1194c..0x11954].copy_from_slice(&[0x30,0xb5,0x03,0x48,0x03,0x68,0x05,0x1c]);
        m.bus.rom[0x11aa4..0x11ab0].copy_from_slice(&[0x13,0x48,0x40,0x88,0x2b,0x68,0x1a,0x8c,0x02,0x40,0x00,0x2a]);
        m.bus.rom[0x11b28..0x11b32].copy_from_slice(&[0x07,0x48,0x2a,0x68,0x41,0x88,0x10,0x8c,0x08,0x40]);
        m.bus.ewram[0x918c..0x9190].copy_from_slice(&(0x02000000+base as u32).to_le_bytes());
        m.bus.ewram[base+8]=6;m.bus.ewram[base+9]=5;
        m.bus.ewram[base+0x20]=3;m.bus.keys=RELEASED;
        assert!(advance_dialogue(&mut m.bus));
        assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
        let ram=m.bus.iwram.clone();
        m.cpu.regs[15]=0x08011aac;m.cpu.regs[3]=0x02000000+base as u32;m.cpu.regs[0]=0;
        m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[0],3);assert_eq!(m.bus.keys,RELEASED);assert_eq!(m.bus.iwram,ram);
        assert!(m.bus.ffta_battle_mouse.dialogue_advance.is_none());
        m.cpu.regs[0]=0;m.ffta_battle_mouse_entry();assert_eq!(m.cpu.regs[0],0);
        m.bus.ewram[base+8]=7;assert!(advance_dialogue(&mut m.bus));
        assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
        m.cpu.regs[15]=0x08011b30;m.cpu.regs[2]=0x02000000+base as u32;m.cpu.regs[1]=0;
        m.ffta_battle_mouse_entry();assert_eq!(m.cpu.regs[1],3);
        assert!(m.bus.ffta_battle_mouse.dialogue_advance.is_none());
        assert_eq!(m.bus.keys,RELEASED);assert_eq!(m.bus.iwram,ram);
        m.bus.ewram[base+8]=8;assert!(!advance_dialogue(&mut m.bus));
        m.bus.ewram[base+8]=9;assert!(!advance_dialogue(&mut m.bus));
        m.bus.ewram[base+8]=10;assert!(!advance_dialogue(&mut m.bus));
        m.bus.ewram[base+8]=5;assert!(!advance_dialogue(&mut m.bus));
        for state in 0..=12 {
            m.bus.ewram[base+8]=state;
            assert_eq!(skippable_dialogue(&m.bus),matches!(state,5|6|7),"dialogue state {state}");
        }
        m.bus.rom[0x1194c]=0;assert!(!skippable_dialogue(&m.bus),"reject unverified interpreter");
    }
    #[test]
    fn dimensions_diamonds_heights_and_invalid_cells() {
        let mut m = fixture();
        let scene = Scene::read(&m.bus).unwrap();
        assert_eq!((scene.width, scene.height, scene.cells.len()), (4, 3, 12));
        for c in scene.cells {
            let hit = Scene::read(&m.bus)
                .unwrap()
                .hit(c.center[0], c.center[1])
                .unwrap();
            assert_eq!((hit.x, hit.y), (c.x, c.y));
        }
        let old = Scene::read(&m.bus).unwrap().cells[6];
        m.bus.ewram[0x30000 + 6 * 2] = 4;
        let high = Scene::read(&m.bus).unwrap().cells[6];
        assert_eq!(high.center[0], old.center[0]);
        assert_eq!(high.center[1], old.center[1] - 16.);
        m.bus.ewram[0x30000 + 6 * 2] = 0;
        assert!(!select(&mut m.bus, 2, 1, false));
        assert!(!select(&mut m.bus, 4, 0, false));
        assert!(!aim(&mut m.bus, f32::NAN, 0., true));
        assert!(!aim(&mut m.bus, 10000., 10000., true));
    }
    #[test]
    fn hover_queues_target_and_only_original_commit_point_changes_registers() {
        let mut m = fixture();
        let before = m.bus.ewram.clone();
        assert!(select(&mut m.bus, 3, 2, false));
        assert_eq!(m.bus.ewram, before);
        assert_eq!(m.bus.ffta_battle_mouse.take_event(), Some(291));
        assert_eq!(keys(&mut m.bus, RELEASED), 0x3ef);
        m.cpu.regs[15] = 0x08025130;
        m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[10], 0);
        m.cpu.regs[15] = 0x08025132;
        m.ffta_battle_mouse_entry();
        assert_eq!((m.cpu.regs[10], m.cpu.regs[9]), (3, 2));
        assert_eq!(m.bus.ewram, before);
        m.bus.ewram[0xf3b8..0xf3ba].copy_from_slice(&3u16.to_le_bytes());
        m.bus.ewram[0xf3bc..0xf3be].copy_from_slice(&2u16.to_le_bytes());
        for f in 1..10 {
            m.bus.frames = f;
            assert_eq!(keys(&mut m.bus, RELEASED), RELEASED);
        }
    }
    #[test]
    fn click_confirms_once_after_commit_and_physical_input_cancels() {
        let mut m = fixture();
        assert!(select(&mut m.bus, 0, 0, true));
        let mut output = Vec::new();
        for f in 0..12 {
            m.bus.frames = f;
            output.push(keys(&mut m.bus, RELEASED));
        }
        assert_eq!(&output[..2], &[RELEASED; 2]);
        assert_eq!(&output[2..6], &[0x3fe; 4]);
        assert!(output[6..].iter().all(|&k| k == RELEASED));
        assert!(select(&mut m.bus, 3, 2, true));
        assert_eq!(keys(&mut m.bus, 0x3df), 0x3df);
        assert!(m.bus.ffta_battle_mouse.request.is_none());
    }
    #[test]
    fn scene_change_or_wrong_cursor_stack_never_applies_pending_target() {
        let mut m = fixture();
        select(&mut m.bus, 3, 2, false);
        m.cpu.regs[15] = 0x08025132;
        m.bus.iwram[0x7000] ^= 4;
        m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[10], 0);
        assert!(m.bus.ffta_battle_mouse.request.is_none());
        m.bus.ewram[0xf3b0] = 0;
        assert!(Scene::read(&m.bus).is_none());
        assert!(!select(&mut m.bus, 0, 0, true));
    }
    fn menu_fixture() -> Machine {
        let mut m=fixture();
        m.bus.ffta_battle_mouse.menu_profile=Some(MenuProfile {
            start:0x08028708,commit:0x080287a0,end:0x080287c0,cursor:0x08017e81,
        });
        for (o,v) in [(8,0x4c03u16),(10,0x4e02),(12,0x5c08),(14,0x4a09)] {
            m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
        menu_instance(&mut m,0x2dd70,4,0,84,false);
        m
    }
    fn menu_instance(m:&mut Machine,base:usize,count:u8,selected:u8,y:u16,child:bool) {
        m.bus.ewram[base+9]=count;
        m.bus.ewram[base+0x58..base+0x5c].copy_from_slice(&0x08017e81u32.to_le_bytes());
        m.bus.ewram[base+0x5c..base+0x60].copy_from_slice(&(0x02000000+base as u32+0x38).to_le_bytes());
        for (o,v) in [(base+0x60,160u16),(base+0x62,y),(base+0x66,selected as u16*16)] {
            m.bus.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
        if child { m.bus.ewram[base+4..base+8].copy_from_slice(&0x0202dd58u32.to_le_bytes()); }
        m.bus.ewram[base+0x68]=7;m.bus.ewram[base+0x69]=selected;
        for (o,v) in [(0,y+selected as u16*16+1),(2,175),(4,0x125)] {
            m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
    }
    #[test]
    fn bank_window_uses_its_own_bounds_and_survives_cursor_blink_in_both_regions() {
        for europe in [false,true] {
            let mut m=menu_fixture();m.bus.ewram.fill(0);m.bus.oam.fill(0);
            let(callback,handler,updater,stack)=if europe {(0x08018fc5u32,0x5aee4,0x5e7b4,0x85)}else{(0x08017e81u32,0x58d40,0x5c594,0x83)};
            m.bus.ffta_battle_mouse.menu_profile.as_mut().unwrap().cursor=callback;
            m.bus.rom.resize(0x60000,0);
            m.bus.rom[handler..handler+12].copy_from_slice(&[0xf0,0xb5,stack,0xb0,0x0e,0x1c,0,6,5,0x0e,0x31,0x78]);
            m.bus.rom[updater..updater+12].copy_from_slice(&[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46,0xe0,0xb4,0x8e,0xb0]);
            for(o,v)in[(0,0x3f40u16),(12,0x0b08),(14,0x0a08)] {m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
            let base=0x23c4c;
            for(o,v)in[(base,0x08000001+handler as u32),(base+4,0x08000001+updater as u32),(base+0x58,callback),(base+0x5c,0x02000000+base as u32+0x38)] {m.bus.ewram[o..o+4].copy_from_slice(&v.to_le_bytes());}
            m.bus.ewram[base+9]=2;m.bus.ewram[base+0x1e]=6;m.bus.ewram[base+0x68]=15;
            m.bus.ewram[base+0x60..base+0x62].copy_from_slice(&24u16.to_le_bytes());
            m.bus.ewram[base+0x62..base+0x64].copy_from_slice(&92u16.to_le_bytes());
            let scene=MenuScene::read(&m.bus).unwrap();
            assert_eq!((scene.origin,scene.right),([0,92],48));
            assert_eq!(scene.hit(12.,98.),Some(0));assert_eq!(scene.hit(12.,116.),Some(1));
            assert_eq!(scene.hit(60.,116.),None);
            assert!(aim(&mut m.bus,12.,116.,false));
            assert_eq!(m.bus.ffta_battle_mouse.menu_request.unwrap().target,1);
            m.bus.ffta_battle_mouse.set_enabled(false);
            assert!(MenuScene::bank_menu(&m.bus).is_some()); // visual fix also works without mouse
            // Regression: bank speech must not inherit the centered menu canvas.
            crate::mem::set_extra(0);crate::mem::set_extra_y(0);
            m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=europe;
            m.bus.ffta_bg_edge=crate::ppu::FftaBgEdge::Transparent;
            m.bus.ffta_dialogue.separate=true;m.bus.ffta_dialogue.style.scale=1.5;
            m.bus.vram.fill(0);m.bus.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
            m.bus.vram[0x8020..0x8040].fill(0x11);m.bus.palette[2..4].copy_from_slice(&0x001fu16.to_le_bytes());
            for (cols,rows) in [(12..30,2..7),(0..6,10..15)] {for y in rows {for x in cols.clone(){let o=0x5000+(y*32+x)*2;m.bus.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());}}}
            for o in m.bus.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
            for(i,a)in[0xa000u16,0xc0d0,0x0240].into_iter().enumerate(){m.bus.oam[i*2..i*2+2].copy_from_slice(&a.to_le_bytes());}
            m.bus.vram[0x14800..0x15000].fill(3);m.bus.palette[0x206..0x208].copy_from_slice(&0x7c00u16.to_le_bytes());
            let oam=m.bus.oam.clone();
            for y in 0..160{crate::ppu::render_scanline(&mut m.bus,y);}let native=m.bus.framebuffer.clone();
            crate::mem::set_extra(164);
            for y in 0..160{crate::ppu::render_scanline(&mut m.bus,y);}
            assert!(m.bus.ffta_dialogue.split_speech);assert!(m.bus.ffta_dialogue.layer().unwrap().1);
            let index=|x:usize,y:usize|y*480+x+120;
            assert!(!m.bus.ffta_dialogue.pixels[index(16,84)].visible());assert!(m.bus.ffta_dialogue.extra.pixels[index(16,84)].visible());
            assert!(m.bus.ffta_dialogue.pixels[index(120,20)].visible());assert!(!m.bus.ffta_dialogue.extra.pixels[index(120,20)].visible());
            assert_eq!(m.bus.framebuffer,native);assert_eq!(m.bus.oam,oam);
            crate::mem::set_extra(0);crate::mem::set_extra_y(0);
            let list_pc=if europe{0x5b624}else{0x59454};
            let sig=if europe{[0xf0,0xb5,0x57,0x46,0x4e,0x46,0x45,0x46,0xe0,0xb4,0x86,0xb0]}
                else{[0xf0,0xb5,0x4f,0x46,0x46,0x46,0xc0,0xb4,0x84,0xb0,0x0f,0x1c]};
            m.bus.rom[list_pc..list_pc+12].copy_from_slice(&sig);
            m.bus.ewram[base..base+4].copy_from_slice(&(0x08000001u32+list_pc as u32).to_le_bytes());
            m.bus.ewram[base+0x68]=0;
            assert!(MenuScene::site_menu(&m.bus).is_some());
            m.bus.rom[list_pc]^=1;
            assert!(MenuScene::site_menu(&m.bus).is_none());
            m.bus.rom[handler]^=1;assert!(MenuScene::bank_menu(&m.bus).is_none());
        }
    }
    #[test]
    fn town_service_parent_uses_owner_geometry_and_rejects_unowned_cursors() {
        for callback in [0x08017e81u32] {for flags in [7,15] {
            let mut m=menu_fixture();m.bus.ewram.fill(0);
            m.bus.rom.resize(0x1000,0);
            m.bus.ffta_battle_mouse.menu_profile.as_mut().unwrap().cursor=callback;
            for(o,v)in[(12,0x0b08u16),(14,0x0a08)]{m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
            m.bus.rom[0x800..0x802].copy_from_slice(&0xb5f0u16.to_le_bytes());
            m.bus.rom[0x900..0x902].copy_from_slice(&0xb570u16.to_le_bytes());
            let base=0x24000;let cursor=base+0x58;
            for(o,v)in[(base,0x08000801u32),(base+4,0x08000901),(cursor,callback),(cursor+4,0x02000000+base as u32+0x38)]{
                m.bus.ewram[o..o+4].copy_from_slice(&v.to_le_bytes());
            }
            m.bus.ewram[base+9]=3;m.bus.ewram[base+0x1c]=2;m.bus.ewram[base+0x1e]=10;
            m.bus.ewram[cursor+16]=flags;
            m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&60u16.to_le_bytes());
            let scene=MenuScene::read(&m.bus).unwrap();
            let shared=MenuScene::layout_menus(&m.bus);
            let key=|m:Option<MenuScene>|m.map(|m|(m.base,m.count,m.selected,m.origin,m.right));
            assert_eq!(key(shared.0),key(MenuScene::site_menu(&m.bus)));
            assert_eq!(key(shared.1),key(MenuScene::pub_menu(&m.bus)));
            assert_eq!(key(shared.2),key(MenuScene::shop_menu(&m.bus)));
            assert_eq!((scene.origin,scene.right),([16,60],96));
            assert_eq!(scene.hit(30.,98.),Some(2));
            assert!(aim(&mut m.bus,30.,98.,false));
            assert_eq!(m.bus.ffta_battle_mouse.menu_request.unwrap().target,2);
            assert!(MenuScene::site_menu(&m.bus).is_some());
            m.bus.ewram[cursor+4..cursor+8].fill(0);
            assert!(MenuScene::parent_window(&m.bus).is_none());
        }}
    }
    #[test]
    fn usa_pub_root_detaches_from_dialogue_without_guest_buffer_changes() {
        use crate::{mem,ppu,ffta_dialogue::{PanelKind,LAYER_WIDTH,LAYER_INSET}};
        mem::set_extra(22);mem::set_extra_y(0);
        let mut m=menu_fixture();m.bus.ewram.fill(0);
        m.bus.ffta_movement.enabled=true;
        m.bus.ffta_bg_edge=crate::ppu::FftaBgEdge::Transparent;
        for(o,v)in[(12,0x0b08u16),(14,0x0a08)]{m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let list=0x25000;let cursor=list+0x20;
        for(o,v)in[(cursor,0x08017e81u32),(cursor+4,0x02000000+list as u32)]{m.bus.ewram[o..o+4].copy_from_slice(&v.to_le_bytes());}
        m.bus.ewram[list+0x13]=1;m.bus.ewram[list+0x15]=10;m.bus.ewram[list+0x16]=4;
        m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&92u16.to_le_bytes());m.bus.ewram[cursor+16]=15;
        m.bus.oam.fill(0);for o in m.bus.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        for(o,v)in[(0,93u16),(2,87),(4,0x127)]{m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.bus.vram.fill(0);m.bus.vram[0x8020..0x8040].fill(0x11);
        m.bus.vram[0x124e0..0x12500].fill(0x11);
        m.bus.palette[2..4].copy_from_slice(&31u16.to_le_bytes());
        m.bus.palette[0x202..0x204].copy_from_slice(&0x3e0u16.to_le_bytes());
        for (cols,rows) in [(0..12,10..20),(12..30,2..4)] {for y in rows {for x in cols.clone(){
            let o=0x5000+(y*32+x)*2;m.bus.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
        }}}
        for y in 0..160 {ppu::render_scanline(&mut m.bus,y);}
        let hardware=m.bus.framebuffer.clone();let oam=m.bus.oam.clone();let ram=m.bus.ewram.clone();
        m.bus.ffta_dialogue.separate=true;
        for y in 0..160 {ppu::render_scanline(&mut m.bus,y);}
        let s=&m.bus.ffta_dialogue;assert!(s.town_layout);assert_eq!(s.extra.kind,PanelKind::Actions);
        let at=|x,y|y*LAYER_WIDTH+x+LAYER_INSET as usize;
        assert_eq!(s.extra.pixels[at(8,96)].resolve(0),31);
        assert!(!s.pixels[at(8,96)].visible());
        assert_eq!(s.extra.pixels[at(87,93)].resolve(0),0x3e0);
        assert_eq!(s.pixels[at(160,16)].resolve(0),31);
        assert!(!s.extra.pixels[at(160,16)].visible());
        assert_eq!(m.bus.framebuffer,hardware);assert_eq!(m.bus.oam,oam);assert_eq!(m.bus.ewram,ram);
        mem::set_extra(0);mem::set_extra_y(0);
    }
    #[test]
    fn service_render_geometry_survives_disabled_cursor_without_enabling_mouse() {
        let mut m=menu_fixture();
        m.bus.io[12..14].copy_from_slice(&0x0b08u16.to_le_bytes());
        m.bus.io[14..16].copy_from_slice(&0x0a08u16.to_le_bytes());
        m.bus.ewram.fill(0);m.bus.oam.fill(0);
        let list=0x25000;let cursor=list+0x20;
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&0x08017e81u32.to_le_bytes());
        m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&(0x02000000u32+list as u32).to_le_bytes());
        m.bus.ewram[list+0x13]=1;
        m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&92u16.to_le_bytes());
        for count in [3,4] {for width in [9,16,25] {
            m.bus.ewram[list+0x16]=count;m.bus.ewram[list+0x15]=width;
            for flags in [0,7,15,0] {
                m.bus.ewram[cursor+0x10]=flags;
                let before=m.bus.ewram.clone();
                let menu=MenuScene::pub_menu(&m.bus).or_else(||MenuScene::shop_menu(&m.bus))
                    .or_else(||MenuScene::service_menu_visual(&m.bus)).unwrap();
                assert_eq!((menu.origin,menu.count,menu.right),([8,92],count,(8+(width as i32+2)*8).min(240)));
                assert_eq!(m.bus.ewram,before);
                if flags==0 {assert!(MenuScene::town_menu(&m.bus).is_none());}
            }
        }}
        m.bus.ewram[cursor+4]^=4;
        assert!(MenuScene::service_menu_visual(&m.bus).is_none(),"stale or unrelated owner must not match");
    }
    #[test]
    fn town_list_keeps_mouse_ownership_when_arrow_blinks() {
        let mut m=menu_fixture();
        m.bus.io[12..14].copy_from_slice(&0x0b08u16.to_le_bytes());
        m.bus.io[14..16].copy_from_slice(&0x0a08u16.to_le_bytes());
        m.bus.ewram.fill(0);m.bus.oam.fill(0);
        let list=0x25000;let cursor=list+0x20;
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&0x08017e81u32.to_le_bytes());
        m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&(0x02000000u32+list as u32).to_le_bytes());
        m.bus.ewram[list+0x13]=7;m.bus.ewram[list+0x15]=16;m.bus.ewram[list+0x16]=8;
        m.bus.ewram[cursor+0xa..cursor+0xc].copy_from_slice(&20u16.to_le_bytes());
        m.bus.ewram[cursor+0x10]=3;m.bus.ewram[cursor+0x11]=2;
        let scene=MenuScene::read(&m.bus).unwrap();
        assert_eq!((scene.count,scene.selected,scene.origin,scene.right),(8,2,[56,20],200));
        assert!(MenuScene::pub_menu(&m.bus).is_none());
        assert_eq!(scene.hit(100.,133.),Some(7));
        assert!(aim(&mut m.bus,100.,133.,false));
        assert_eq!(m.bus.ffta_battle_mouse.menu_request.unwrap().target,7);
        // Pub MENU: Rumors/Missions/Quit Mission/Leave, unlike the LIST submenu.
        m.bus.ffta_battle_mouse.cancel();
        m.bus.ewram[list+0x13]=1;m.bus.ewram[list+0x15]=9;m.bus.ewram[list+0x16]=4;
        m.bus.ewram[cursor+0xa..cursor+0xc].copy_from_slice(&92u16.to_le_bytes());
        m.bus.ewram[cursor+0x10]=15;m.bus.ewram[cursor+0x11]=0;
        let menu=MenuScene::read(&m.bus).unwrap();
        assert_eq!((menu.count,menu.origin,menu.right),(4,[8,92],96));
        assert!(MenuScene::pub_menu(&m.bus).is_some());
        // Lighting policy is independent of labels and mouse selection.
        m.bus.ffta_movement.europe_profile=true;
        assert_eq!(MenuScene::cinematic_interior_hint(&m.bus),Some(true));
        assert_eq!(MenuScene::cinematic_location_hint(&m.bus),Some(1));
        m.bus.ewram[list+0x16]=3; // Same owner switched to shop root.
        assert_eq!(MenuScene::cinematic_interior_hint(&m.bus),Some(true));
        assert_eq!(MenuScene::cinematic_location_hint(&m.bus),Some(2));
        m.bus.ewram[list+0x16]=4;
        m.bus.ffta_movement.europe_profile=false;
        // Labels determine the original window width. Do not pin English's
        // width: translated labels must retain the same root-panel ownership.
        for width in 9..=25 {
            m.bus.ewram[list+0x15]=width;
            for flags in [7,15] {
            m.bus.ewram[cursor+0x10]=flags;
            let root=MenuScene::pub_menu(&m.bus).unwrap();
            assert_eq!(root.right,(8+(width as i32+2)*8).min(240));
            }
        }
        m.bus.ewram[list+0x15]=9;
        assert!(aim(&mut m.bus,40.,145.,false));
        assert_eq!(m.bus.ffta_battle_mouse.menu_request.unwrap().target,3);
        // Monster Bank uses the same active root-window style, but a different
        // placement and row count from Pub/Shop. Keep list selection direct.
        m.bus.ffta_battle_mouse.cancel();
        m.bus.ewram[list+0x13]=2;m.bus.ewram[list+0x15]=10;m.bus.ewram[list+0x16]=5;
        m.bus.ewram[cursor+0xa..cursor+0xc].copy_from_slice(&36u16.to_le_bytes());
        m.bus.ewram[cursor+0x10]=7;m.bus.ewram[cursor+0x11]=0;
        let site=MenuScene::read(&m.bus).unwrap();
        assert_eq!((site.count,site.origin),(5,[16,36]));
        assert!(MenuScene::site_menu(&m.bus).is_some());
        assert!(MenuScene::pub_menu(&m.bus).is_none());
        assert!(aim(&mut m.bus,40.,108.,false));
        assert_eq!(m.bus.ffta_battle_mouse.menu_request.unwrap().target,4);
        m.bus.io[12..14].copy_from_slice(&0x5c08u16.to_le_bytes());
        assert!(MenuScene::town_menu(&m.bus).is_none());
    }
    #[test]
    fn shop_root_and_inventory_keep_direct_selection_and_quantity_ownership() {
        for callback in [0x08017e81u32,0x08018fc5] {
            let mut m=menu_fixture();m.bus.ewram.fill(0);
            m.bus.ffta_battle_mouse.menu_profile.as_mut().unwrap().cursor=callback;
            m.bus.io[12..14].copy_from_slice(&0x0b08u16.to_le_bytes());
            m.bus.io[14..16].copy_from_slice(&0x0a08u16.to_le_bytes());
            let list=0x25000;let cursor=list+0x20;
            m.bus.ewram[cursor..cursor+4].copy_from_slice(&callback.to_le_bytes());
            m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&(0x02000000u32+list as u32).to_le_bytes());
            m.bus.ewram[list+0x13]=1;m.bus.ewram[list+0x15]=7;m.bus.ewram[list+0x16]=3;
            m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&92u16.to_le_bytes());
            m.bus.ewram[cursor+16]=15;
            assert!(MenuScene::shop_menu(&m.bus).is_some());
            assert!(MenuScene::pub_menu(&m.bus).is_none());
            let pc=(callback&!1)+0x130;let off=(pc-0x08000000)as usize;
            m.bus.rom.resize(0x72000,0);
            m.bus.rom[off..off+12].copy_from_slice(&[0x09,0x06,0x09,0x0e,0x41,0x74,0x09,0x01,0xc1,0x81,0x70,0x47]);
            m.bus.rom[0x2b360..0x2b366].copy_from_slice(&[0x48,0x46,0x4c,0x30,0x21,0x0e]);
            assert!(menu_select(&mut m.bus,2,false));
            m.cpu.regs[0]=0x02000000+cursor as u32;m.cpu.regs[1]=1;
            m.cpu.regs[4]=0x01000000;m.cpu.regs[6]=1;
            m.cpu.regs[9]=m.cpu.regs[0]-0x4c;m.cpu.regs[14]=0x0802b36b;m.cpu.regs[15]=pc;
            let before=m.bus.ewram.clone();m.ffta_battle_mouse_entry();
            assert_eq!((m.cpu.regs[1],m.cpu.regs[4],m.cpu.regs[6]),(2,0x02000000,2));
            assert_eq!(m.bus.ewram,before);
            for _ in 0..5 {m.step();}
            assert_eq!(m.bus.ewram[cursor+17],2);
            m.bus.ffta_battle_mouse.cancel();
            let inventory=if callback==0x08018fc5 {0x71030}else{0x6eb48};
            m.bus.rom[inventory..inventory+12].copy_from_slice(&[0xf0,0xb5,0x87,0xb0,6,0x1c,0x11,0x4f,0x39,0x68,0x11,0x4a]);
            m.bus.ewram[cursor..cursor+4].copy_from_slice(&(0x08000001+inventory as u32).to_le_bytes());
            m.bus.ewram[list+0x13]=2;m.bus.ewram[list+0x15]=24;m.bus.ewram[list+0x16]=4;
            m.bus.ewram[list+0x18..list+0x1a].copy_from_slice(&3u16.to_le_bytes());
            m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&20u16.to_le_bytes());
            m.bus.ewram[cursor+16]=3;m.bus.ewram[cursor+17]=0;
            let items=MenuScene::read(&m.bus).unwrap();
            assert_eq!((items.count,items.origin,items.right),(3,[16,20],224));
            assert_eq!(items.hit(100.,54.),Some(2));
            m.bus.io[..2].copy_from_slice(&0x5f40u16.to_le_bytes());
            assert!(MenuScene::read(&m.bus).is_none()); // quantity owns A/B
            m.bus.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
            m.bus.ewram[list+0x18]=5;assert_eq!(MenuScene::read(&m.bus).unwrap().count,4);
            m.bus.ewram[list+0x1a]=1;m.bus.ewram[cursor+17]=3;
            assert_eq!(MenuScene::read(&m.bus).unwrap().selected,3);
            m.bus.ewram[list+0x1a]=5;assert!(MenuScene::read(&m.bus).is_none());
        }
    }
    #[test]
    fn pub_direct_commit_updates_cursor_and_callers_model_for_both_profiles() {
        for callback in [0x08017e81u32,0x08018fc5] {
            let mut m=menu_fixture();m.bus.ewram.fill(0);
            m.bus.ffta_battle_mouse.menu_profile.as_mut().unwrap().cursor=callback;
            m.bus.io[12..14].copy_from_slice(&0x0b08u16.to_le_bytes());
            m.bus.io[14..16].copy_from_slice(&0x0a08u16.to_le_bytes());
            let list=0x25000;let cursor=list+0x20;
            m.bus.ewram[cursor..cursor+4].copy_from_slice(&callback.to_le_bytes());
            m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&(0x02000000u32+list as u32).to_le_bytes());
            m.bus.ewram[list+0x13]=1;m.bus.ewram[list+0x15]=12;m.bus.ewram[list+0x16]=4;
            m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&92u16.to_le_bytes());
            m.bus.ewram[cursor+16]=15;
            let pc=(callback&!1)+0x130;let off=(pc-0x08000000)as usize;
            m.bus.rom.resize(0x62000,0);
            m.bus.rom[off..off+12].copy_from_slice(&[0x09,0x06,0x09,0x0e,0x41,0x74,0x09,0x01,0xc1,0x81,0x70,0x47]);
            m.bus.rom[0x61c96..0x61c9e].copy_from_slice(&[0x20,0x1c,0x3c,0x30,0x31,0x16,0x0b,0x22]);
            assert!(menu_select(&mut m.bus,2,false));
            m.cpu.regs[0]=0x02000000+cursor as u32;m.cpu.regs[1]=1;
            m.cpu.regs[6]=0x01000000;m.cpu.regs[14]=0x08061c97;m.cpu.regs[15]=pc;
            assert!(m.bus.ffta_battle_mouse.interprets(pc));
            let before=m.bus.ewram.clone();
            m.ffta_battle_mouse_entry();
            assert_eq!((m.cpu.regs[1],m.cpu.regs[6]),(2,0x02000000));
            assert_eq!(m.bus.ewram,before); // guest instructions perform the stores
            for _ in 0..5 {m.step();}
            assert_eq!(m.bus.ewram[cursor+17],2);
            assert_eq!(h(&m.bus.ewram,cursor+14),Some(32));
            m.bus.ffta_battle_mouse.cancel();m.bus.ewram[cursor+17]=0;
            assert!(menu_select(&mut m.bus,2,false));
            m.bus.rom[off]^=1;m.cpu.regs[15]=pc;m.cpu.regs[1]=1;m.cpu.regs[6]=0x01000000;
            m.ffta_battle_mouse_entry();assert_eq!(m.cpu.regs[1],1);
            assert!(m.bus.ffta_battle_mouse.menu_request.unwrap().committed.is_none());
        }
    }
    #[test]
    fn menu_uses_live_count_and_leaf_cursor_not_option_names() {
        let mut m=menu_fixture();
        let menu=MenuScene::read(&m.bus).unwrap();
        assert_eq!((menu.count,menu.selected,menu.origin),(4,0,[160,84]));
        for row in 0..4 { assert_eq!(menu.hit(195.,85.+row as f32*16.),Some(row)); }
        assert_eq!(menu.hit(195.,149.),None);
        assert_eq!(menu.hit(f32::NAN,90.),None);
        menu_instance(&mut m,0x2dd70,4,3,84,false);
        menu_instance(&mut m,0x2def4,1,0,132,true);
        let child=MenuScene::read(&m.bus).unwrap();
        assert_eq!((child.base,child.count),(0x2def4,1));
        assert!(!menu_select(&mut m.bus,1,true));
        m.bus.ewram[0x2def4+0x58]^=1;
        assert_eq!(MenuScene::read(&m.bus).unwrap().base,0x2dd70);
        m.bus.oam[4]=0x20;
        assert!(MenuScene::read(&m.bus).is_none());
    }
    #[test]
    fn menu_jump_is_original_register_commit_and_click_confirms_once() {
        let mut m=menu_fixture();
        let before=m.bus.ewram.clone();
        assert!(menu_select(&mut m.bus,3,true));
        assert_eq!(m.bus.ffta_battle_mouse.take_event(),Some(515));
        assert_eq!(keys(&mut m.bus,RELEASED),0x37f);
        m.cpu.regs[15]=0x080287a0;m.cpu.regs[5]=0x0202dd70;m.cpu.regs[6]=0x0202ddc8;
        m.cpu.regs[4]=1;m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[4],3);
        assert_eq!(m.bus.ewram,before);
        menu_instance(&mut m,0x2dd70,4,3,84,false);
        let mut pressed=0;
        for frame in 1..12 {m.bus.frames=frame;pressed+=usize::from(keys(&mut m.bus,RELEASED)==0x3fe);}
        assert_eq!(pressed,4);
        assert!(menu_select(&mut m.bus,1,true));
        assert_eq!(keys(&mut m.bus,0x3bf),0x3bf);
        assert!(m.bus.ffta_battle_mouse.menu_request.is_none());
    }
    #[test]
    fn replay_drives_deferred_confirm_and_releases_the_whole_pulse() {
        let mut m=fixture();
        assert!(select(&mut m.bus,0,0,true));
        let mut keys_out=Vec::new();
        for f in 0..12 { m.bus.frames=f;m.bus.keys=RELEASED;replay_keys(&mut m.bus);keys_out.push(m.bus.keys); }
        assert_eq!(&keys_out[2..6],&[0x3fe;4]);
        assert!(keys_out[6..].iter().all(|&v|v==RELEASED));
    }
    #[test]
    fn notice_choices_follow_repeated_bullets_and_confirm_after_selection() {
        for second in [65,89,113] {
            let mut m=menu_fixture();
            m.bus.ffta_dialogue.extra.kind=crate::ffta_dialogue::PanelKind::Notice;
            m.bus.ffta_dialogue.extra.bounds=Some([16,16,224,64]);
            m.bus.oam.fill(0);for o in m.bus.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
            for(o,v)in[(0,44u16),(2,27),(4,0x125)]{m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
            m.bus.vram.fill(0);
            // Synthetic glyph pixels, deliberately unaligned to tile boundaries.
            let mut dot=|x:i32,y:i32| {
                let tile=(y/8*32+x/8)as u16+1;
                let o=0xe000+((y/8*32+x/8)*2)as usize;
                m.bus.vram[o..o+2].copy_from_slice(&tile.to_le_bytes());
                let o=0x8000+tile as usize*32+(y%8)as usize*4+(x%8)as usize/2;
                m.bus.vram[o]|=6<<(4*(x%2));
            };
            for x in [27,second] {for yy in 1..7 {for xx in 1..7 {
                if xx==1||xx==6||yy==1||yy==6 {dot(x+xx,44+yy);}
            }}}
            for y in 44..52 {for x in second+12..second+21 {dot(x,y);}}
            let scene=MenuScene::read(&m.bus).unwrap();
            assert_eq!(scene.hit(40.,47.),Some(0));
            assert_eq!(scene.hit(second as f32+15.,47.),Some(1));
            assert_eq!(scene.hit(40.,70.),None);
            assert!(aim(&mut m.bus,second as f32+15.,47.,true));
            assert_eq!(keys(&mut m.bus,RELEASED),0x3ef);
            m.bus.oam[2..4].copy_from_slice(&(second as u16).to_le_bytes());
            assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
            let mut confirms=0;
            for f in 1..16 {m.bus.frames=f;confirms+=usize::from(keys(&mut m.bus,RELEASED)==0x3fe);}
            assert_eq!(confirms,4);
            assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
            // Informational notices without a real choice cursor stay passive.
            m.bus.oam[..2].copy_from_slice(&0x200u16.to_le_bytes());
            assert!(MenuScene::read(&m.bus).is_none());
        }
    }
    #[test]
    fn binary_confirmation_waits_for_original_selection_then_confirms_once() {
        let mut m=menu_fixture();
        m.bus.rom.resize(0x29358,0);
        m.bus.rom[0x29350..0x29358].copy_from_slice(&[0xf0,0xb5,4,0x1c,0x26,0x1c,0x58,0x36]);
        let b=0x2dd70;
        m.bus.ewram[b+8]=10;m.bus.ewram[b+9]=0;
        m.bus.ewram[b+0x5c..b+0x60].fill(0);
        for(o,v)in[(b+0x60,74u16),(b+0x62,107),(b+0x84,80),(b+0x86,104),(b+0x8c,88),(b+0x8e,120),(0x40000,0)] {
            if o<0x40000 {m.bus.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());}
        }
        m.bus.oam[0..2].copy_from_slice(&108u16.to_le_bytes());
        m.bus.oam[2..4].copy_from_slice(&89u16.to_le_bytes());
        let scene=MenuScene::read(&m.bus).unwrap();assert!(scene.binary);
        let old_oam=m.bus.oam.clone();
        for obj in m.bus.oam.chunks_exact_mut(8){obj[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        assert!(MenuScene::read(&m.bus).unwrap().is_confirmation(),"cursor blink cannot disable a live confirmation");
        m.bus.oam=old_oam;
        set_mouse_visual(&mut m.bus,true);m.bus.ffta_dialogue.confirmation_marker=true;
        assert!(!hide_mouse_indicator(&m.bus,[108,89,0x125]),"binary choice dot is distinct from hidden menu swords");
        m.bus.ffta_dialogue.confirmation_marker=false;
        for tile in 0x124..=0x127 {
            assert!(!hide_mouse_indicator(&m.bus,[108,89,tile]),"choice dots survive selector publication gaps");
        }

        assert_eq!(scene.hit(110.,110.),Some(0));assert_eq!(scene.hit(110.,126.),Some(1));
        assert!(menu_select(&mut m.bus,1,true));
        assert_eq!(keys(&mut m.bus,RELEASED),0x37f);
        // The original handler changes cursor coordinates and selection;
        // OAM can still contain its previous frame during publication.
        m.bus.ewram[b+0x69]=1;
        m.bus.ewram[b+0x60..b+0x62].copy_from_slice(&82u16.to_le_bytes());
        m.bus.ewram[b+0x62..b+0x64].copy_from_slice(&123u16.to_le_bytes());
        let mut count=0;
        for f in 1..15 {m.bus.frames=f;count+=usize::from(keys(&mut m.bus,RELEASED)==0x3fe);}
        assert_eq!(count,4);
    }
    #[test]
    fn list_pitch_uses_live_cursor_and_reuses_it_at_row_zero() {
        let mut m=menu_fixture();let cursor=0x25020;
        m.bus.ewram[cursor+14..cursor+16].copy_from_slice(&36u16.to_le_bytes());
        assert_eq!(MenuScene::measure_pitch(&m.bus,cursor,3,160,240,48,6),12);
        m.bus.ewram[cursor+14..cursor+16].fill(0);
        assert_eq!(MenuScene::measure_pitch(&m.bus,cursor,0,160,240,48,6),12);
    }
    #[test]
    fn list_pitch_can_be_measured_from_glyph_rows_before_first_selection() {
        let mut m=menu_fixture();
        m.bus.io[12..14].copy_from_slice(&0x0b08u16.to_le_bytes());
        m.bus.vram[0x8020..0x8040].fill(0x11);
        for y in [2usize,4,6,8] {
            let o=0x5800+(y*32+21)*2;
            m.bus.vram[o..o+2].copy_from_slice(&1u16.to_le_bytes());
        }
        assert_eq!(MenuScene::measure_pitch(&m.bus,0x25020,0,160,240,16,4),16);
    }
    #[test]
    fn eur_battle_abilities_accept_paged_window_lists_without_accepting_invalid_owners() {
        let mut m=menu_fixture();
        m.bus.ffta_battle_mouse.menu_profile.as_mut().unwrap().cursor=0x08018fc5;
        m.bus.rom.resize(0x397f8,0);
        m.bus.rom[0x397ec..0x397f8].copy_from_slice(&[0xf0,0xb5,0x87,0xb0,0x84,0x46,0x0e,0x1c,0x34,0x1c,0x31,0x7c]);
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08025115u32.to_le_bytes());
        m.bus.iwram[0x2810..0x2814].copy_from_slice(&0x0200f3a4u32.to_le_bytes());
        let list=0xfd70;let cursor=list+0x20;let win=0x9e0+4*0x4c;
        m.bus.ewram[list-0x48]=4;m.bus.ewram[list+0x16]=6;
        m.bus.ewram[list+0x18..list+0x1a].copy_from_slice(&19u16.to_le_bytes());
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&0x080397edu32.to_le_bytes());
        m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&0x0200fd70u32.to_le_bytes());
        m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&44u16.to_le_bytes());
        m.bus.iwram[win]=2;m.bus.iwram[win+0x34]=4;m.bus.iwram[win+0x35]=2;
        m.bus.iwram[win+0x36]=22;m.bus.iwram[win+0x49]=1;
        let scene=MenuScene::read(&m.bus).unwrap();
        assert_eq!(scene.count,6);assert_eq!(scene.hit(70.,141.),Some(5));
        assert!(menu_select(&mut m.bus,5,false));
        m.bus.ewram[list+0x1a..list+0x1c].copy_from_slice(&6u16.to_le_bytes());
        m.bus.ewram[cursor+0x11]=11;
        m.bus.ewram[cursor+14..cursor+16].copy_from_slice(&60u16.to_le_bytes());
        let paged=MenuScene::window_list(&m.bus,true).unwrap();
        assert_eq!((paged.count,paged.selected,paged.pitch),(6,5,12));
        assert_eq!(paged.hit(70.,122.),Some(5));
        m.bus.iwram[win]=0;
        assert!(MenuScene::window_list(&m.bus,true).is_none());
    }
    #[test]
    fn mouse_priority_requires_enabled_mode_and_back_hides_scroll_immediately(){
        let mut m=fixture();m.bus.ffta_dialogue.service_layout=true;
        let sword=[0x4000,0,0x120];
        assert!(!hide_mouse_indicator(&m.bus,sword));
        set_mouse_visual(&mut m.bus,true);assert!(hide_mouse_indicator(&m.bus,sword));
        assert!(!hide_mouse_indicator(&m.bus,[0x803b,0x0088,0x2122]),"dialogue continuation remains visible");
        // Non-directional physical A must not steal pointer priority.
        keys(&mut m.bus,0x3fe);assert!(hide_mouse_indicator(&m.bus,sword));
        set_mouse_visual(&mut m.bus,false);assert!(!hide_mouse_indicator(&m.bus,sword));
        set_mouse_visual(&mut m.bus,true);assert!(hide_mouse_indicator(&m.bus,sword));
        assert!(pub_scroll_visible(&m.bus));back(&mut m.bus);assert!(!pub_scroll_visible(&m.bus));
        m.bus.ffta_battle_mouse.set_enabled(false);set_mouse_visual(&mut m.bus,true);
        assert!(!hide_mouse_indicator(&m.bus,sword));
        m.bus.ffta_battle_mouse.set_enabled(true);assert!(!mouse_visual(&m.bus));
        keys(&mut m.bus,0x3fd);assert!(!pub_scroll_visible(&m.bus));
    }
    #[test]
    fn europe_world_menu_publishes_absolute_target_without_visiting_intermediate_rows(){
        let mut m=menu_fixture();m.bus.rom.resize(0x39800,0);
        m.bus.ffta_battle_mouse.menu_profile.as_mut().unwrap().cursor=0x08018fc5;
        m.bus.rom[0x397ec..0x397f8].copy_from_slice(&[0xf0,0xb5,0x87,0xb0,0x84,0x46,0x0e,0x1c,0x34,0x1c,0x31,0x7c]);
        m.bus.rom[0x18cac..0x18cb8].copy_from_slice(&[0x10,0xb5,3,0x1c,9,6,9,0x16,0x58,0x8b,9,0x18]);
        m.bus.iwram[0x2810..0x2814].copy_from_slice(&0x0200f3a4u32.to_le_bytes());
        let list=0xfd70;let cursor=list+0x20;let win=0x9e0+4*0x4c;
        m.bus.ewram[list-0x48]=4;m.bus.ewram[list+0x16]=4;m.bus.ewram[list+0x18]=3;
        m.bus.ewram[list+8..list+12].copy_from_slice(&0x02028000u32.to_le_bytes());
        m.bus.ewram[list+0x15]=8;m.bus.ewram[list+0x1e]=8;
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&0x080397edu32.to_le_bytes());
        m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&0x0200fd70u32.to_le_bytes());
        m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&44u16.to_le_bytes());
        m.bus.iwram[win]=2;m.bus.iwram[win+0x34]=4;m.bus.iwram[win+0x35]=2;
        m.bus.iwram[win+0x36]=22;m.bus.iwram[win+0x49]=1;
        for from in 0..3 {for to in 0..3 {
            m.bus.ffta_battle_mouse.cancel();m.bus.ewram[cursor+17]=from;
            m.bus.ewram[cursor+14..cursor+16].copy_from_slice(&(from as u16*16).to_le_bytes());
            assert!(menu_select(&mut m.bus,to,false));
            assert_eq!(m.bus.ewram[cursor+17],to);
            assert_eq!(keys(&mut m.bus,RELEASED),RELEASED,"direct selection never emits a direction");
            // The actual publishers get an absolute index; an unrelated old
            // row query keeps its input unchanged for correct unhighlight.
            for caller in [0x080395cf,0x08039611,0x08012345] {
                m.cpu.regs[15]=0x08018cac;m.cpu.regs[0]=0x02000000+list as u32;
                m.cpu.regs[1]=1;m.cpu.regs[14]=caller;
                m.ffta_battle_mouse_entry();
                assert_eq!(m.cpu.regs[1],1,"all original queries remain untouched");
            }
        }}
    }
    #[test]
    fn world_window_list_uses_live_visible_count_and_original_navigation() {
        let mut m=menu_fixture();
        m.bus.rom.resize(0x38034,0);
        m.bus.rom[0x38028..0x38034].copy_from_slice(&[0xf0,0xb5,0x87,0xb0,0x84,0x46,0x0e,0x1c,0x34,0x1c,0x31,0x7c]);
        m.bus.iwram[0x2810..0x2814].copy_from_slice(&0x0200f3a4u32.to_le_bytes());
        let list=0xfd70;let cursor=list+0x20;let win=0x9e0+4*0x4c;
        m.bus.ewram[list-0x48]=4;
        m.bus.ewram[list+0x16]=4;
        m.bus.ewram[list+0x18..list+0x1a].copy_from_slice(&3u16.to_le_bytes());
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&0x08038029u32.to_le_bytes());
        m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&0x0200fd70u32.to_le_bytes());
        m.bus.ewram[cursor+0xa..cursor+0xc].copy_from_slice(&44u16.to_le_bytes());
        m.bus.iwram[win]=2;m.bus.iwram[win+0x34]=4;m.bus.iwram[win+0x35]=2;
        m.bus.iwram[win+0x36]=22;m.bus.iwram[win+0x49]=1;
        let scene=MenuScene::read(&m.bus).unwrap();
        assert!(scene.polling);assert_eq!((scene.count,scene.origin,scene.right),(3,[32,56],208));
        assert_eq!(scene.hit(70.,79.),Some(1));
        assert!(menu_select(&mut m.bus,1,true));
        assert_eq!(keys(&mut m.bus,RELEASED),0x37f);
        m.bus.ewram[cursor+0x11]=1;
        let mut presses=0;
        for f in 1..16 {m.bus.frames=f;presses+=usize::from(keys(&mut m.bus,RELEASED)==0x3fe);}
        assert_eq!(presses,4);
        m.bus.ewram[list+0x18]=5;
        assert!(!MenuScene::read(&m.bus).is_some_and(|s|s.polling));
    }
    #[test]
    fn world_target_changes_original_registers_and_defers_confirm() {
        let mut m=fixture();
        let p=Profile{base:0,start:0x08038994,commit:0x08038b3a,end:0x08038c16};
        m.bus.ffta_battle_mouse.world_profile=Some(p);
        m.bus.io[8..10].copy_from_slice(&0xaa03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0xac02u16.to_le_bytes());
        m.bus.iwram[0x2810..0x2814].copy_from_slice(&0x0200f3a4u32.to_le_bytes());
        m.bus.ewram[0xf3a8..0xf3ac].copy_from_slice(&0x02002c10u32.to_le_bytes());
        m.bus.ewram[0x2c10]=0x61; // visitable destination must retain mouse input
        for(o,v)in[(0x2c16,128u16),(0x2c18,240),(0x2c1c,156)]{m.bus.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());}
        assert!(world_select(&mut m.bus,190.,90.,true));
        assert_eq!(keys(&mut m.bus,RELEASED),0x36f);
        let before=m.bus.ewram.clone();
        m.cpu.regs[4]=0x02002c10;m.cpu.regs[15]=p.commit;m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[0],190);assert_eq!(m.bus.ewram,before);
        m.cpu.regs[15]=p.end-8;m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[0],246);assert_eq!(m.bus.ewram,before);
        m.bus.frames=1;m.cpu.regs[15]=p.commit;m.cpu.regs[0]=191;m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[0],190); // pending guest key release must not drift by one pixel
        let mut presses=0;
        for f in 1..15{m.bus.frames=f;presses+=usize::from(keys(&mut m.bus,RELEASED)==0x3fe);}
        assert_eq!(presses,4);
        m.bus.ewram[0x2c10]=0;
        assert!(!world_select(&mut m.bus,100.,90.,true));
    }
    #[test]
    fn shared_text_list_uses_live_visible_rows_with_wide_arrow() {
        let mut m=menu_fixture();
        m.bus.ewram[0x2dd70+9]=0;
        let cursor=0x25f08;let list=cursor-0x20;
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&0x08017e81u32.to_le_bytes());
        m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&(0x02000000+list as u32).to_le_bytes());
        m.bus.ewram[cursor+8..cursor+10].copy_from_slice(&40u16.to_le_bytes());
        m.bus.ewram[cursor+10..cursor+12].copy_from_slice(&20u16.to_le_bytes());
        m.bus.ewram[cursor+16]=3;
        m.bus.ewram[list+0x13]=7;m.bus.ewram[list+0x15]=16;m.bus.ewram[list+0x16]=8;
        for(o,v)in[(0,0x4014u16),(2,40),(4,0x120)]{m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        let scene=MenuScene::read(&m.bus).unwrap();
        assert_eq!((scene.count,scene.origin,scene.right),(8,[56,20],200));
        assert!(scene.polling);
        assert_eq!(scene.hit(80.,72.),Some(3));
        m.bus.oam[2..4].copy_from_slice(&32u16.to_le_bytes());
        assert_eq!(MenuScene::read(&m.bus).unwrap().count,8);
        m.bus.oam[2..4].copy_from_slice(&31u16.to_le_bytes());
        assert!(MenuScene::read(&m.bus).is_none());
        m.bus.oam[2..4].copy_from_slice(&32u16.to_le_bytes());
        assert!(menu_select(&mut m.bus,3,false));
        assert_eq!(keys(&mut m.bus,RELEASED),0x37f);
        m.bus.rom.resize(0x17b74,0);
        m.bus.rom[0x17b68..0x17b74].copy_from_slice(&[0x10,0xb5,3,0x1c,9,6,9,0x16,0x58,0x8b,9,0x18]);
        m.cpu.regs[15]=0x08017b68;m.cpu.regs[0]=0x02000000+list as u32;m.cpu.regs[1]=1;
        let before=m.bus.ewram.clone();m.ffta_battle_mouse_entry();
        assert_eq!(m.cpu.regs[1],3);assert_eq!(m.bus.ewram,before);
        m.bus.ewram[cursor+17]=1;
        m.bus.ewram[cursor+14..cursor+16].copy_from_slice(&16u16.to_le_bytes());
        assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
        for _ in 0..4 {assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);}
        assert_eq!(keys(&mut m.bus,RELEASED),0x37f);
        m.bus.ewram[cursor+17]=3;
        m.bus.ewram[cursor+14..cursor+16].copy_from_slice(&48u16.to_le_bytes());
        assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
        m.bus.io[1]=0;
        assert!(MenuScene::read(&m.bus).is_none());
    }
    #[test]
    fn plain_clicks_work_without_a_field_or_menu_and_physical_input_wins() {
        let mut m=fixture();m.bus.ewram[0xf3b0]=0;
        assert!(press(&mut m.bus,0x3fe));
        let out:Vec<_>=(0..12).map(|_|keys(&mut m.bus,RELEASED)).collect();
        assert_eq!(&out[..4],&[0x3fe;4]);
        assert!(out[4..].iter().all(|&v|v==RELEASED));
        back(&mut m.bus);assert_eq!(keys(&mut m.bus,RELEASED),0x3fd);
        assert_eq!(keys(&mut m.bus,0x3bf),0x3bf);
        assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
    }
    #[test]
    fn free_unit_selection_does_not_require_unused_ui_backgrounds() {
        let mut m=fixture();
        for(o,v)in[(0,0x1340u16),(8,0x4c03),(10,0x4e02),(12,0x0b08),(14,0x0a08)] {
            m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
        assert!(Scene::read(&m.bus).is_some());
        m.bus.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
        assert!(Scene::read(&m.bus).is_none());
        press(&mut m.bus,0x3fe);assert_eq!(keys(&mut m.bus,RELEASED),0x3fe);
    }
    #[test]
    fn facing_cross_origin_is_actor_ground_not_floating_indicator() {
        let mut m=fixture();m.bus.ewram[0xf3b0]=0;
        m.bus.ewram[0x7f18]=16;m.bus.ewram[0x7f19]=16;
        m.bus.ewram[0xf3b8..0xf3ba].copy_from_slice(&4u16.to_le_bytes());
        m.bus.ewram[0xf3bc..0xf3be].copy_from_slice(&11u16.to_le_bytes());
        m.bus.ewram[0x30000+2*(11*16+4)]=4;
        for(o,v)in[(0x7f60,256u16),(0x7f62,256),(0x7f64,24),(0x7f66,265)] {
            m.bus.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
        for(i,(a,b,c))in[(0x3bu16,0x6cu16,0x13au16),(0x33,0x6c,0x136),(0x33,0x107c,0x136),(0x3b,0x107c,0x13a)].into_iter().enumerate(){
            for(o,v)in[(i*8,a),(i*8+2,b),(i*8+4,c)] {m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        }
        assert_eq!(facing_center(&m.bus),Some([120.,87.]));
        // Below the floating arrow but above the feet must be upper-right.
        assert!(aim_facing(&mut m.bus,150.,75.,false));
        assert_eq!(keys(&mut m.bus,RELEASED),0x3bf);
        // A bobbing arrow cannot move the cross.
        for i in 0..4 {m.bus.oam[i*8]=m.bus.oam[i*8].wrapping_add(1);}
        assert_eq!(facing_center(&m.bus),Some([120.,87.]));
        // Camera scroll and elevation are read from the map, not guessed.
        m.bus.ewram[0x7f64..0x7f66].copy_from_slice(&34u16.to_le_bytes());
        m.bus.ewram[0x7f66..0x7f68].copy_from_slice(&270u16.to_le_bytes());
        assert_eq!(facing_center(&m.bus),Some([110.,82.]));
        m.bus.ewram[0x30000+2*(11*16+4)]=5;
        assert_eq!(facing_center(&m.bus),Some([110.,74.]));
        m.bus.ewram[0xf3b8..0xf3ba].copy_from_slice(&16u16.to_le_bytes());
        assert_eq!(facing_center(&m.bus),None);
    }
    #[test]
    fn facing_cross_matches_reference_and_assigns_equal_quadrants() {
        let c=[119.,57.];let mut counts=[0;4];
        let keys=[0x3ef,0x37f,0x3df,0x3bf];
        for degree in 0..3600 {
            let a=(degree as f32*0.1+0.025).to_radians();
            let key=stable_facing_direction(c,c[0]+100.*a.cos(),c[1]+100.*a.sin(),None).unwrap();
            counts[keys.iter().position(|k|*k==key).unwrap()]+=1;
            // No historical hysteresis may enlarge any of the four sectors.
            for old in keys {assert_eq!(stable_facing_direction(c,c[0]+100.*a.cos(),c[1]+100.*a.sin(),Some(old)),Some(key));}
        }
        assert_eq!(counts,[900;4]);
        for (degree,key) in [(359.,0x3bf),(1.,0x3ef),(89.,0x3ef),(91.,0x37f),(179.,0x37f),(181.,0x3df),(269.,0x3df),(271.,0x3bf)] {
            let a=(degree as f32).to_radians();
            assert_eq!(facing_direction(c,c[0]+100.*a.cos(),c[1]+100.*a.sin()),Some(key));
        }
        assert_eq!(stable_facing_direction(c,c[0]+1.,c[1],Some(0x3ef)),Some(0x3ef));
    }
    #[test]
    fn facing_sectors_cover_distant_viewport_and_preserve_isometric_directions() {
        let c=[119.,57.];
        for (dx,dy,key) in [(320.,160.,0x3ef),(-320.,-160.,0x3df),(-320.,160.,0x37f),(320.,-160.,0x3bf)] {
            assert_eq!(facing_direction(c,c[0]+dx,c[1]+dy),Some(key));
            assert_eq!(facing_direction([c[0]+100.,c[1]+50.],c[0]+100.+dx,c[1]+50.+dy),Some(key));
        }
        assert_eq!(facing_direction(c,c[0],c[1]),None);
        assert_eq!(facing_direction(c,f32::NAN,0.),None);
    }
    #[test]
    fn facing_detects_animated_highlights_from_live_prompt() {
        let mut m=fixture();m.bus.ewram[0xf3b0]=0;
        m.bus.ewram[0x7f60..0x7f62].copy_from_slice(&120u16.to_le_bytes());
        m.bus.ewram[0x7f62..0x7f64].copy_from_slice(&67u16.to_le_bytes());
        for (i,(a,b,c)) in [(0x3bu16,0x6cu16,0x13au16),(0x33,0x6c,0x136),(0x33,0x107c,0x137),(0x3b,0x107c,0x13a)].into_iter().enumerate() {
            for (o,v) in [(i*8,a),(i*8+2,b),(i*8+4,c)] {m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        }
        assert_eq!(facing_center(&m.bus),Some([120.,59.]));
        assert!(aim(&mut m.bus,200.,99.,false));
        assert_eq!(keys(&mut m.bus,RELEASED),0x3ef);
        for top in 0x135u16..=0x138 {for bottom in 0x139u16..=0x13a {
            for (i,t) in [(0,bottom),(1,top),(2,top),(3,bottom)] {
                m.bus.oam[i*8+4..i*8+6].copy_from_slice(&t.to_le_bytes());
            }
            assert_eq!(facing_center(&m.bus),Some([120.,59.]));
        }}
    }
    #[test]
    fn facing_arrows_turn_first_then_confirm_once_and_do_not_repeat_on_hover() {
        let mut m=fixture();m.bus.ewram[0xf3b0]=0;
        m.bus.ewram[0x7f60..0x7f62].copy_from_slice(&119u16.to_le_bytes());
        m.bus.ewram[0x7f62..0x7f64].copy_from_slice(&65u16.to_le_bytes());
        for(i,(x,y,t,flip))in[(107u16,49u16,0x135u16,0u16),(123,49,0x135,0x1000),(107,57,0x139,0),(123,57,0x139,0x1000)].into_iter().enumerate(){
            for(o,v)in[(i*8,y),(i*8+2,x|flip),(i*8+4,t)]{m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        }
        assert_eq!(facing_center(&m.bus),Some([119.,57.]));
        assert!(aim(&mut m.bus,151.,73.,false));
        let out:Vec<_>=(0..10).map(|_|keys(&mut m.bus,RELEASED)).collect();
        assert_eq!(out.iter().filter(|&&k|k==0x3ef).count(),4);
        aim(&mut m.bus,151.,73.,false);assert_eq!(keys(&mut m.bus,RELEASED),RELEASED);
        aim(&mut m.bus,151.,73.,true);
        let out:Vec<_>=(0..20).map(|_|keys(&mut m.bus,RELEASED)).collect();
        assert_eq!(out.iter().filter(|&&k|k==0x3ef).count(),4);
        assert_eq!(out.iter().filter(|&&k|k==0x3fe).count(),4);
        assert!(out.iter().position(|&k|k==0x3ef)<out.iter().position(|&k|k==0x3fe));
    }
}
/// Optional compact diagnostics; no guest writes or timing changes.
pub fn verification(mem: &MemMap) {
    if std::env::var("FFTA_MOUSE_VERIFY").as_deref() != Ok("1") {
        return;
    }
    let from = std::env::var("FFTA_MOUSE_VERIFY_FROM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(12550);
    if mem.frames < from || mem.frames % 10 != 0 {
        return;
    }
    let base = mem.ffta_battle_mouse.profile.map_or(0xf3ac, |p| p.base);
    println!(
        "FFTA_MOUSE_STATE frame={} mode={} selected={},{} scene={:?}",
        mem.frames,
        mem.ewram[base + 4],
        h(&mem.ewram, base + 12).unwrap_or(65535),
        h(&mem.ewram, base + 16).unwrap_or(65535),
        Scene::read(mem).map(|s| (s.width, s.height, s.cells.len()))
    );
}



#[cfg(test)]mod dialogue_skip_hold_tests {
    use super::*;
    #[test]fn requires_hold_and_release_after_a_decision(){
        let mut skip=DialogueSkip::default();
        assert!(!skip.update(true,0.,true));assert!(!skip.update(true,0.9,true));
        assert!(skip.update(true,1.,true));assert!(!skip.update(true,1.1,false));
        assert!(!skip.update(true,3.,true));assert!(!skip.update(false,3.1,true));
        assert!(!skip.update(true,4.,true));assert!(skip.update(true,5.,true));
    }
    #[test]fn holding_on_an_event_cannot_auto_skip_later_dialogue(){
        let mut skip=DialogueSkip::default();assert!(!skip.update(true,0.,false));
        assert!(!skip.update(true,1.,false));assert!(!skip.update(true,2.,true));
    }
}
