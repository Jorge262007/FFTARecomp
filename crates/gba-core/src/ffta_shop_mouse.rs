//! Mouse targets from the original shop category widget, not translated labels.
use crate::{ffta_rom::Version,mem::MemMap,Machine};
#[derive(Clone,Copy)]
struct Profile { start:u32,commit:u32,end:u32,inventory:u32,handlers:[u32;2] }
#[derive(Clone,Copy)]
struct Request { base:usize,slot:u8,source:u8,ids:[u8;6],count:u8,deadline:u64,committed:Option<u64> }
#[derive(Default)]
pub(crate) struct State { profile:Option<Profile>,request:Option<Request>,deferred:Option<(f32,f32,bool)>,scroll:Option<(usize,u16,u8,u64)>,resume:Option<([u32;16],u32)>,hover_refresh:bool,neutral:bool }
impl State {
    pub(crate) fn new(rom:&[u8],version:Version)->Self {
        let start=match version {Version::Europe=>0x71490,Version::Usa=>0x6efa8};
        let inventory=match version {Version::Europe=>0x71030,Version::Usa=>0x6eb48};
        if rom.get(start..start+12)!=Some(&[0x00,0xb5,0x02,0x1c,0x09,0x04,0x09,0x0c,0x10,0x29,0x09,0xd0])
            || rom.get(start+0x30..start+0x34)!=Some(&[0x10,0x70,0x10,0x1c])
            || rom.get(inventory..inventory+12)!=Some(&[0xf0,0xb5,0x87,0xb0,6,0x1c,0x11,0x4f,0x39,0x68,0x11,0x4a]) {return Self::default();}
        Self {profile:Some(Profile{start:0x08000000+start as u32,commit:0x08000030+start as u32,
            end:0x0800003c+start as u32,inventory:0x08000001+inventory as u32,
            handlers:match version {Version::Usa=>[0x08069659,0x0806b6d1],Version::Europe=>[0x0806bb0d,0x0806db7d]}}),request:None,deferred:None,scroll:None,resume:None,hover_refresh:false,neutral:false}
    }
    pub(crate) fn cancel(&mut self) {self.request=None;self.deferred=None;self.scroll=None;self.neutral=false;}
    pub(crate) fn pending(&self)->bool {self.request.is_some()}
    pub(crate) fn interprets(&self,pc:u32)->bool {
        (self.request.is_some() && self.profile.is_some_and(|p|(p.start..p.end).contains(&pc)))
            || ((self.scroll.is_some() || self.resume.is_some()) && ((0x08070624..0x080710e0).contains(&pc)))
    }
}
fn half(b:&[u8],o:usize)->Option<u16>{Some(u16::from_le_bytes(b.get(o..o+2)?.try_into().ok()?))}
fn word(b:&[u8],o:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(o..o+4)?.try_into().ok()?))}
#[derive(Clone,Copy)]
struct Context {base:usize,count:u8,current:u8,ids:[u8;6],x:i32,y:i32,overlay:bool,cursor:usize}
fn inventory_owner(mem:&MemMap)->Option<(usize,usize)> {
    let p=mem.ffta_battle_mouse.shop.profile?;
    if half(&mem.io,0)?&0x0f07!=0x0f00 || [8,10,12,14].map(|o|half(&mem.io,o))
        !=[Some(0x4c03),Some(0x4e02),Some(0x0b08),Some(0x0a08)] {return None;}
    let manager=word(&mem.ewram,0xf428)?.checked_sub(0x02000000)? as usize;
    let cursor=manager.checked_add(0x14e0)?;
    if !p.handlers.contains(&word(&mem.ewram,cursor.checked_sub(0x54)?)?) || word(&mem.ewram,cursor)?!=p.inventory
        || word(&mem.ewram,cursor+4)?!=0x02000000+cursor as u32-0x20 {return None;}
    Some((manager,cursor))
}
/// Rendering owns the inventory before its item cursor/count is published.
pub(crate) fn visible(mem:&MemMap)->bool {inventory_owner(mem).is_some()}
fn context(mem:&MemMap)->Option<Context> {
    let (manager,cursor)=inventory_owner(mem)?;
    let base=manager.checked_add(0x4470)?;
    let count=*mem.ewram.get(base+7)?;let current=*mem.ewram.get(base)?;
    if count==0 || count>6 || current>=count {return None;}
    let mut ids=[255;6];ids[..count as usize].copy_from_slice(mem.ewram.get(base+1..base+1+count as usize)?);
    for i in 0..count as usize {if ids[i]>5 || ids[..i].contains(&ids[i]){return None;}}
    let x=*mem.ewram.get(base+0x78)? as i32*8;
    let y=*mem.ewram.get(base+0x79)? as i32*8;
    if x+count as i32*16>240 || y+16>160 {return None;}
    Some(Context{base,count,current,ids,x,y,overlay:half(&mem.io,0)?&0x4000!=0,cursor})
}
pub(crate) fn active(mem:&MemMap)->bool {context(mem).is_some()}
pub fn select_category(mem:&mut MemMap,slot:u8)->bool {
    if !mem.ffta_battle_mouse.enabled {return false;}
    let Some(c)=context(mem)else{return false;};
    if c.overlay || slot>=c.count {return false;}
    if mem.ffta_battle_mouse.shop.request.is_some_and(|r|r.base==c.base&&r.slot==slot&&r.ids==c.ids){return true;}
    mem.ffta_battle_mouse.cancel();
    if slot!=c.current {
        mem.ffta_battle_mouse.shop.request=Some(Request{base:c.base,slot,source:c.current,ids:c.ids,count:c.count,
            deadline:mem.frames+120,committed:None});
        mem.ffta_battle_mouse.record_shop_event(1024+slot as u16);
    }
    true
}
pub(crate) fn aim(mem:&mut MemMap,x:f32,y:f32,confirm:bool)->bool {
    if !mem.ffta_battle_mouse.enabled {return false;}
    let Some(c)=context(mem)else{return false;};
    if c.overlay {
        // Quantity uses an original digit cursor below the numeric field.
        // Click tens/units to choose a digit; wheel sends its original Up/Down.
        if confirm && half(&mem.io,0).is_some_and(|v|v&0x6000==0x4000) {
            let h=half(&mem.io,0x42).unwrap_or(0);let v=half(&mem.io,0x46).unwrap_or(0);
            let right=(h&255)as i32;let top=(v>>8)as i32;let bottom=(v&255)as i32;
            let column=(x/8.).floor()as i32*8;
            if y>=top as f32 && y<bottom as f32 && (column==right-16 || column==right-24) {
                let current=mem.oam.chunks_exact(8).find_map(|o| {
                    (half(o,4)==Some(0x112f)).then(||half(o,2).unwrap_or(0)&511)
                });
                if let Some(current)=current {
                    if current as i32!=column {mem.ffta_battle_mouse.queue_shop_key(if column==right-24 {0x3df}else{0x3ef});}
                    return true;
                }
            }
        }
        return false;
    }
    if !x.is_finite() || !y.is_finite(){return false;}
    if y>=c.y as f32 && y<(c.y+16)as f32 && x>=c.x as f32 && x<(c.x+6*16).min(240)as f32 {
        // Measured BG3 tile rows: Sell packs its live IDs contiguously,
        // e.g. IDs [1,2,3,5] are drawn at X=16,32,48,64, not 32,48,64,96.
        let slot=((x-c.x as f32)/16.)as u8;
        if confirm {
            if slot<c.count {select_category(mem,slot);}else{mem.ffta_battle_mouse.cancel();}
        }
        return true;
    }
    if mem.ffta_battle_mouse.shop.pending() {
        // Preserve the latest pointer/click until the new category has really
        // published its list. Never select an item from the previous category.
        mem.ffta_battle_mouse.shop.deferred=Some((x,y,confirm));return true;
    }
    let list=c.cursor-0x20;
    let top=half(&mem.ewram,c.cursor+10).unwrap_or(160)as f32;
    let capacity=mem.ewram[list+0x16].min(8)as f32;
    if x>=0. && x<240. && y>=top && y<top+capacity*16.
        && crate::ffta_battle_mouse::MenuScene::shop_inventory(mem).and_then(|s|s.hit(x,y)).is_none() {
        mem.ffta_battle_mouse.cancel();return true;
    }
    false
}
/// Live item count and first row from the original inventory, including Sell.
pub fn list(mem:&MemMap)->Option<crate::ffta_battle_mouse::PubList>{
    if !mem.ffta_battle_mouse.enabled{return None;}
    let c=context(mem)?;if mem.ffta_battle_mouse.shop.profile?.inventory!=0x08071031{return None;}
    if c.overlay || mem.ewram.get(c.cursor+16)?&3!=3{return None;}
    let b=c.cursor-32;let total=half(&mem.ewram,b+24)?;let first=half(&mem.ewram,b+26)?;
    let visible=(*mem.ewram.get(b+22)? as u16).min(4);
    if total==0 || total>255 || visible==0 || first>=total{return None;}
    Some(crate::ffta_battle_mouse::PubList{cursor:c.cursor,total,first,visible:visible.min(total)})
}
pub fn scroll_to(mem:&mut MemMap,first:u16)->bool{
    let Some(l)=list(mem)else{return false;};let first=first.min(l.total.saturating_sub(l.visible));
    mem.ffta_battle_mouse.cancel();
    if first!=l.first {
        mem.ffta_battle_mouse.shop.scroll=Some((l.cursor,first,0,mem.frames+120));
    }
    mem.ffta_battle_mouse.shop.hover_refresh=false;
    mem.ffta_battle_mouse.shop.neutral=true;true
}
pub(crate) fn redrawing(mem:&MemMap)->bool{
    mem.ffta_battle_mouse.shop.resume.is_some() && !mem.ffta_battle_mouse.shop.hover_refresh
}
pub(crate) fn unselected(mem:&MemMap)->bool{mem.ffta_battle_mouse.shop.neutral && list(mem).is_some()}
pub(crate) fn select_item(mem:&mut MemMap,target:u8)->bool{
    let Some(l)=list(mem)else{return false;};if target as u16>=l.visible{return false;}
    mem.ffta_battle_mouse.shop.neutral=false;
    if mem.ewram[l.cursor+17]==target{return true;}
    mem.ffta_battle_mouse.shop.hover_refresh=true;
    mem.ffta_battle_mouse.shop.scroll=Some((l.cursor,l.first,target,mem.frames+120));true
}
pub(crate) fn keys(mem:&mut MemMap)->Option<u16> {
    if let Some((cursor,_,_,deadline))=mem.ffta_battle_mouse.shop.scroll {
        if mem.frames<=deadline && list(mem).is_some_and(|l|l.cursor==cursor){return Some(0x3ff);}
        mem.ffta_battle_mouse.shop.scroll=None;
    }
    let r=mem.ffta_battle_mouse.shop.request?;
    let valid=context(mem).is_some_and(|c|!c.overlay&&c.base==r.base&&c.count==r.count&&c.ids==r.ids);
    if !valid || mem.frames>r.deadline {mem.ffta_battle_mouse.shop.cancel();return None;}
    if let Some(frame)=r.committed {
        if mem.frames>=frame+2 && context(mem).is_some_and(|c|c.current==r.slot) {
            let deferred=mem.ffta_battle_mouse.shop.deferred.take();
            mem.ffta_battle_mouse.shop.cancel();
            if let Some((x,y,confirm))=deferred {crate::ffta_battle_mouse::aim(mem,x,y,confirm);}
        }
        return Some(0x3ff);
    }
    Some(0x3ef) // wake the original Right branch once; its store receives the target
}
pub(crate) fn entry(m:&mut Machine) {
    if m.cpu.regs[15]==0x08070676 && m.bus.ffta_battle_mouse.shop.hover_refresh
        && m.bus.ffta_battle_mouse.shop.resume.is_some()
        && m.bus.rom.get(0x70676..0x7067c)==Some(&[0x1f,0x1c,0x03,0xa9,0x00,0x20]) {
        // Same page: preserve glyph tiles. Continue at the original cursor,
        // palette and detail refresh instead of clearing/rebuilding all rows.
        m.cpu.regs[15]=0x080706dc;
    }
    // Borrow the existing cursor callback call frame to redraw a complete
    // page with the original routine, then resume with all registers restored.
    if m.cpu.regs[15]==0x08071030 {
        if let Some((regs,cpsr))=m.bus.ffta_battle_mouse.shop.resume.take(){
            m.cpu.regs=regs;m.cpu.cpsr=cpsr;
            m.bus.ffta_battle_mouse.shop.hover_refresh=false;
        }else if let Some((cursor,first,selected,deadline))=m.bus.ffta_battle_mouse.shop.scroll {
            if m.bus.frames<=deadline && list(&m.bus).is_some_and(|l|l.cursor==cursor)
                && m.cpu.regs[0]==0x02000000+cursor as u32
                && m.bus.rom.get(0x70624..0x7062e)==Some(&[0xf0,0xb5,0x4f,0x46,0x46,0x46,0xc0,0xb4,0x84,0xb0]) {
                m.bus.ffta_battle_mouse.shop.resume=Some((m.cpu.regs,m.cpu.cpsr));
                if !m.bus.ffta_battle_mouse.shop.hover_refresh {
                    m.bus.ffta_dialogue.shop_page_dirty=true;
                }
                m.bus.ewram[cursor+17]=selected;
                m.bus.ewram[cursor+14..cursor+16].copy_from_slice(&(selected as u16*16).to_le_bytes());
                m.cpu.regs[0]=first as u32;m.cpu.regs[1]=selected as u32;m.cpu.regs[2]=1;
                m.cpu.regs[14]=0x08071031;m.cpu.regs[15]=0x08070624;
                m.bus.ffta_battle_mouse.shop.scroll=None;return;
            }
        }
    }
    let s=&m.bus.ffta_battle_mouse;
    let Some(p)=s.shop.profile.filter(|_|s.enabled)else{return;};
    if m.cpu.regs[15]!=p.commit {return;}
    let Some(r)=s.shop.request.filter(|r|r.committed.is_none())else{return;};
    let Some(c)=context(&m.bus)else{return;};
    if c.overlay || c.base!=r.base || c.current!=r.source || c.ids!=r.ids || c.count!=r.count || r.slot>=c.count
        || m.cpu.regs[2]!=0x02000000+c.base as u32 || m.bus.frames>r.deadline {return;}
    m.cpu.regs[0]=r.slot as u32;
    m.bus.ffta_battle_mouse.shop.request.as_mut().unwrap().committed=Some(m.bus.frames);
}
pub fn scroll(mem:&mut MemMap,up:bool)->bool {
    if let Some(l)=list(mem){
        let first=mem.ffta_battle_mouse.shop.scroll.filter(|r|r.0==l.cursor).map_or(l.first,|r|r.1);
        return scroll_to(mem,if up{first.saturating_sub(1)}else{first.saturating_add(1)});
    }
    if !mem.ffta_battle_mouse.enabled || context(mem).is_none()
        || half(&mem.io,0).is_some_and(|v|v&0x6000==0x6000){return false;}
    mem.ffta_battle_mouse.queue_shop_key(if up {0x3bf}else{0x37f});true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(version:Version,sell:bool)->Machine {
        let mut m=Machine::new(vec![]);m.cpu.cpsr=0x3f;
        let start=if version==Version::Europe {0x71490}else{0x6efa8};
        let inventory=if version==Version::Europe {0x71030}else{0x6eb48};
        m.bus.rom.resize(0x72000,0);
        m.bus.rom[start..start+12].copy_from_slice(&[0,0xb5,2,0x1c,9,4,9,0x0c,0x10,0x29,9,0xd0]);
        m.bus.rom[start+0x30..start+0x34].copy_from_slice(&[0x10,0x70,0x10,0x1c]);
        m.bus.rom[inventory..inventory+12].copy_from_slice(&[0xf0,0xb5,0x87,0xb0,6,0x1c,0x11,0x4f,0x39,0x68,0x11,0x4a]);
        m.bus.ffta_battle_mouse.shop=State::new(&m.bus.rom,version);m.bus.ffta_battle_mouse.enabled=true;
        for (o,v)in[(0,0x1f40u16),(8,0x4c03),(10,0x4e02),(12,0x0b08),(14,0x0a08)]{m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.bus.ewram[0xf428..0xf42c].copy_from_slice(&0x02001000u32.to_le_bytes());
        let cursor=0x24e0;let base=0x5470;
        m.bus.ewram[cursor..cursor+4].copy_from_slice(&(0x08000001+inventory as u32).to_le_bytes());
        let handler=m.bus.ffta_battle_mouse.shop.profile.unwrap().handlers[usize::from(sell)];
        m.bus.ewram[cursor-0x54..cursor-0x50].copy_from_slice(&handler.to_le_bytes());
        m.bus.ewram[cursor+4..cursor+8].copy_from_slice(&0x020024c0u32.to_le_bytes());
        let ids:&[u8]=if sell {&[1,2,3,5]}else{&[0,1,2,3,4,5]};
        m.bus.ewram[base+7]=ids.len()as u8;m.bus.ewram[base+1..base+1+ids.len()].copy_from_slice(ids);
        m.bus.ewram[base+0x78]=2;m
    }
    #[test]
    fn inventory_render_owner_survives_unpublished_and_empty_categories() {
        for version in [Version::Usa,Version::Europe] {for sell in [false,true] {
            let mut m=fixture(version,sell);
            assert!(visible(&m.bus));assert!(active(&m.bus));
            m.bus.ewram[0x5477]=0;
            assert!(visible(&m.bus));assert!(!active(&m.bus));
            assert!(!aim(&mut m.bus,32.,8.,true));
            m.bus.ewram[0x24e0..0x24e4].fill(0);
            assert!(!visible(&m.bus));
        }}
    }
    #[test]
    fn categories_use_available_ids_and_original_store_for_both_regions() {
        for version in [Version::Usa,Version::Europe] {for sell in [false,true] {
            let mut m=fixture(version,sell);let c=context(&m.bus).unwrap();
            for slot in 0..6 {
                assert!(aim(&mut m.bus,(c.x+slot as i32*16+8)as f32,8.,false));
                assert!(!m.bus.ffta_battle_mouse.shop.pending(),"hover never switches category");
                assert!(aim(&mut m.bus,(c.x+slot as i32*16+8)as f32,8.,true));
                if slot>=c.count {assert!(!m.bus.ffta_battle_mouse.shop.pending());continue;}
                if slot==context(&m.bus).unwrap().current {continue;}
                assert!(m.bus.ffta_battle_mouse.shop.pending());
                assert_eq!(keys(&mut m.bus),Some(0x3ef));
                let p=m.bus.ffta_battle_mouse.shop.profile.unwrap();
                m.cpu.regs[15]=p.commit;m.cpu.regs[2]=0x02000000+c.base as u32;m.cpu.regs[0]=1;
                let before=m.bus.ewram.clone();entry(&mut m);
                assert_eq!(m.cpu.regs[0],slot as u32);assert_eq!(m.bus.ewram,before);
                m.step();assert_eq!(m.bus.ewram[c.base],slot);
                m.bus.frames+=2;assert_eq!(keys(&mut m.bus),Some(0x3ff));
            }
        }}
    }
    #[test]
    fn unavailable_categories_and_context_changes_never_confirm_or_retarget() {
        let mut m=fixture(Version::Europe,true);
        assert!(aim(&mut m.bus,88.,8.,true));assert!(!m.bus.ffta_battle_mouse.shop.pending());
        assert!(!select_category(&mut m.bus,4));
        assert!(select_category(&mut m.bus,2));m.bus.ewram[0x5471]=0;
        assert_eq!(keys(&mut m.bus),None);assert!(!m.bus.ffta_battle_mouse.shop.pending());
        m.bus.ewram[0x5471]=1;assert!(select_category(&mut m.bus,2));
        assert_eq!(crate::ffta_battle_mouse::keys(&mut m.bus,0x3fd),0x3fd);
        assert!(!m.bus.ffta_battle_mouse.shop.pending());
        m.bus.io[..2].copy_from_slice(&0x5f40u16.to_le_bytes());
        assert!(!aim(&mut m.bus,56.,8.,true));assert!(scroll(&mut m.bus,false));
        m.bus.io[..2].copy_from_slice(&0x7f40u16.to_le_bytes());assert!(!scroll(&mut m.bus,false));
    }
    #[test]
    fn quick_item_click_waits_until_the_requested_category_is_published() {
        let mut m=fixture(Version::Europe,false);
        assert!(select_category(&mut m.bus,2));
        assert!(aim(&mut m.bus,40.,24.,true));
        assert!(m.bus.ffta_battle_mouse.shop.deferred.is_some());
        m.bus.ffta_battle_mouse.shop.request.as_mut().unwrap().committed=Some(0);
        m.bus.frames=4;assert_eq!(keys(&mut m.bus),Some(0x3ff));
        assert!(m.bus.ffta_battle_mouse.shop.pending()); // old category still in RAM
        m.bus.ewram[0x5470]=2;
        assert_eq!(keys(&mut m.bus),Some(0x3ff));
        assert!(!m.bus.ffta_battle_mouse.shop.pending());
        assert!(m.bus.ffta_battle_mouse.shop.deferred.is_none());
    }
    #[test]
    fn quantity_digits_and_wheel_use_original_controls_but_confirmation_does_not() {
        let mut m=fixture(Version::Europe,false);
        for(o,v)in[(0,0x5f40u16),(0x42,0x98c0),(0x46,0x5868)]{m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        for(o,v)in[(0,0x006bu16),(2,0x10b0),(4,0x112f)]{m.bus.oam[o..o+2].copy_from_slice(&v.to_le_bytes());}
        assert!(aim(&mut m.bus,170.,96.,true));
        assert_eq!(crate::ffta_battle_mouse::keys(&mut m.bus,0x3ff),0x3df);
        m.bus.ffta_battle_mouse.cancel();assert!(scroll(&mut m.bus,true));
        assert_eq!(crate::ffta_battle_mouse::keys(&mut m.bus,0x3ff),0x3bf);
        m.bus.io[..2].copy_from_slice(&0x7f40u16.to_le_bytes());
        assert!(!scroll(&mut m.bus,true));assert!(!aim(&mut m.bus,170.,96.,true));
    }
    #[test]
    fn wrong_function_bytes_disable_the_category_hook() {
        let mut m=fixture(Version::Europe,false);m.bus.rom[0x71490]^=1;
        assert!(State::new(&m.bus.rom,Version::Europe).profile.is_none());
    }
}
