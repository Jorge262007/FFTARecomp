//! Presentation ownership from the original window object, BEFORE its cursor
//! exists and AFTER input is disabled. Initially verified for the EUR Pub root.
//! No timers, retained pixels, translated labels or guest-memory writes.
use crate::MemMap;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Phase {Entering,Open,Exiting,Hidden}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Window {pub owner:usize,pub bounds:[i32;4],pub frame_bounds:[i32;4],pub current:[u16;2],pub target:[u16;2],pub phase:Phase}
impl Window {pub fn visible(self)->bool{self.phase!=Phase::Hidden}}
fn half(b:&[u8],o:usize)->Option<u16>{Some(u16::from_le_bytes(b.get(o..o+2)?.try_into().ok()?))}
fn word(b:&[u8],o:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(o..o+4)?.try_into().ok()?))}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Kind {Root,Content,Speech}
impl Kind {pub fn index(self)->usize{match self{Self::Root=>0,Self::Content=>1,Self::Speech=>2}}}
fn at(mem:&MemMap,owner:usize)->Option<(Kind,Window)>{
    let e=&mem.ewram;
    if word(e,owner)?!=0x06005800 || word(e,owner+4)?!=0x06005000{return None;}
    let stage=crate::ffta_surface::Descriptor::read(mem).map(|s|s.id);
    let store=stage==Some(65);
    let kind=match (half(e,owner+20)?,half(e,owner+22)?,word(e,owner+24)?) {
        (0x0180,0x0c20,0x00010005)=>Kind::Root,
        (0x00c0,0x0c20,0x00010005)=>Kind::Content,
        (0x0030,0x0b20,0x00040015)=>Kind::Speech,
        _=>return None,
    };
    let [x,y,w,h]:[u8;4]=e.get(owner+16..owner+20)?.try_into().ok()?;
    if w==0 || h==0 || x as u16+w as u16>30 || y as u16+h as u16>20
        || kind==Kind::Root && (x!=0 || y!=9) || store && kind==Kind::Content && (x!=1 || y!=0 || w!=26 || h!=11){return None;}
    let current=[half(e,owner+32)?,half(e,owner+34)?];
    let target=[half(e,owner+36)?,half(e,owner+38)?];
    if current[0]>w as u16 || target[0]>w as u16 || current[1]>h as u16 || target[1]>h as u16{return None;}
    let phase=if target[0]==0 || target[1]==0 {
        if current[0]==0 || current[1]==0{Phase::Hidden}else{Phase::Exiting}
    }else if current==target {Phase::Open}else{Phase::Entering};
    let bounds=if matches!(stage,Some(65|66|67)) && kind==Kind::Content {[0,0,240,160]}
    else if kind==Kind::Speech {
        // The verified portrait extends one tile above/below its text frame.
        [x as i32*8,(y as i32*8-8).max(0),(x as i32+w as i32)*8,((y as i32+h as i32)*8+8).min(160)]
    }else{[x as i32*8,y as i32*8,((x as i32+w as i32)*8+8).min(240),(y as i32+h as i32)*8]};
    Some((kind,Window{owner,bounds,frame_bounds:[x as i32*8,y as i32*8,(x as i32+w as i32)*8,(y as i32+h as i32)*8],current,target,phase}))
}
/// All measured Pub families share the same animated window protocol.
/// A single scan resolves missing owners; cached objects are revalidated.
pub fn pub_windows(mem:&MemMap,cached:[Option<usize>;3])->[Option<Window>;3]{
    if !mem.ffta_movement.europe_profile || !mem.ffta_movement.enabled
        || crate::ffta_surface::Descriptor::read(mem).is_none_or(|s|!matches!(s.id,62|63|65|66|67))
        || [12,14].map(|o|half(&mem.io,o))!=[Some(0x0b08),Some(0x0a08)]{return [None;3];}
    let mut out:[Option<Window>;3]=std::array::from_fn(|i|cached[i].and_then(|o|at(mem,o)).filter(|(k,_)|k.index()==i).map(|(_,w)|w));
    if out.iter().all(|w|w.is_some_and(Window::visible)){return out;}
    let mut ambiguous=[false;3];
    for owner in (0..mem.ewram.len().saturating_sub(44)).step_by(4){
        if word(&mem.ewram,owner)!=Some(0x06005800){continue;}
        if let Some((kind,w))=at(mem,owner){
            let i=kind.index();
            match out[i] {
                Some(old) if old.owner==owner=>{},
                Some(old) if old.visible() && w.visible()=>{ambiguous[i]=true;},
                Some(old) if old.visible()=>{},
                _=>out[i]=Some(w),
            }
        }
    }
    for i in 0..3{if ambiguous[i]{out[i]=None;}}
    out
}
pub fn pub_root(mem:&MemMap,cached:Option<usize>)->Option<Window>{pub_windows(mem,[cached,None,None])[0]}

/// Read-only presentation phase for host transitions. Cached addresses are
/// validated against family, geometry and current BG resources on every use.
/// Unknown panels fall back to visibility; no guest input/state is changed.
pub fn presentation(mem:&MemMap,node:crate::ffta_ui::Node,cached:Option<usize>)->Option<(usize,Phase)> {
    use crate::ffta_ui::Role;
    let family=match node.role {
        Role::ServiceMenu=>Some((0x0180,0x0c20,0x00010005)),
        Role::ServiceContent if mem.ffta_dialogue.service_list=>Some((0x00c0,0x0c20,0x00010005)),
        Role::Dialogue=>Some((0x0030,0x0b20,0x00040015)),
        Role::ActionMenu=>Some((0x0140,0x0c20,0x00010005)),
        Role::Notice=>Some((0x0800,0x0b20,0x0003000d)),
        Role::AttackName=>Some((0x0404,0x0b20,0x00030032)),
        _=>None,
    }?;
    let maps=[12,14].map(|r|Some(0x06000000+(((half(&mem.io,r)?>>8)&31)as u32)*0x800));
    let matches=|o:usize|->Option<Phase>{
        let e=&mem.ewram;
        if word(e,o)!=maps[0] || word(e,o+4)!=maps[1]
            || (half(e,o+20)?,half(e,o+22)?,word(e,o+24)?)!=family{return None;}
        let [x,y,w,h]:[u8;4]=e.get(o+16..o+20)?.try_into().ok()?;
        if w==0 || w>30 || h==0 || h>20 || x>=32 || y as u16+h as u16>20{return None;}
        let rect=[x as i32*8,y as i32*8,(x as i32+w as i32)*8,(y as i32+h as i32)*8];
        if rect[2]<=node.bounds[0] || rect[0]>=node.bounds[2]
            || rect[3]<=node.bounds[1] || rect[1]>=node.bounds[3]{return None;}
        if node.role==Role::ActionMenu && (node.bounds[0]!=(rect[0]-8).max(0)
            || node.bounds[1]!=rect[1] || node.bounds[2]!=(rect[2]+8).min(240)
            || node.bounds[3]!=(rect[3]+16).min(160)){return None;}
        let current=[half(e,o+32)?,half(e,o+34)?];let target=[half(e,o+36)?,half(e,o+38)?];
        if current[0]>w as u16 || current[1]>h as u16 || target[0]>w as u16 || target[1]>h as u16{return None;}
        Some(if target[0]==0 || target[1]==0 {
            if current[0]==0 || current[1]==0{Phase::Hidden}else{Phase::Exiting}
        }else if current==target {Phase::Open}else{Phase::Entering})
    };
    // A closing parent is not the owner of an open child, even where their
    // screen rectangles overlap. Prefer the active matching constructor.
    let cached_phase=cached.and_then(|o|matches(o).map(|p|(o,p)));
    if cached_phase.is_some_and(|(_,p)|matches!(p,Phase::Open|Phase::Entering)){return cached_phase;}
    let mut found=cached_phase;
    for o in (0..mem.ewram.len().saturating_sub(44)).step_by(4){
        if let Some(phase)=matches(o){
            if matches!(phase,Phase::Open|Phase::Entering){return Some((o,phase));}
            found=Some((o,phase));
        }
    }
    found
}
#[cfg(test)]mod tests{
    use super::*;
    fn scene()->MemMap{
        let mut m=MemMap::new(vec![]);m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
        for(o,v)in[(0,0x1f40u16),(8,0x4c03),(10,0x4e02),(12,0x0b08),(14,0x0a08)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        m.ewram[0x7f10..0x7f12].copy_from_slice(&62u16.to_le_bytes());let o=0x1000;
        for(a,v)in[(0,0x06005800u32),(4,0x06005000),(24,0x00010005)]{m.ewram[o+a..o+a+4].copy_from_slice(&v.to_le_bytes());}
        m.ewram[o+16..o+20].copy_from_slice(&[0,9,11,11]);
        for(a,v)in[(20,0x0180u16),(22,0x0c20),(34,11),(38,11)]{m.ewram[o+a..o+a+2].copy_from_slice(&v.to_le_bytes());}m
    }
    #[test]fn action_child_uses_own_open_constructor_instead_of_closed_parent(){
        let mut m=scene();m.io[12..14].copy_from_slice(&0x5c08u16.to_le_bytes());m.io[14..16].copy_from_slice(&0x4a09u16.to_le_bytes());
        for(owner,rect,current,target)in[(0x1000,[22u8,8,8,11],[0u16,11],[0u16,11]),(0x1100,[19,10,11,9],[11,9],[11,9])] {
            for(a,v)in[(0,0x0600e000u32),(4,0x06005000),(24,0x00010005)]{m.ewram[owner+a..owner+a+4].copy_from_slice(&v.to_le_bytes());}
            m.ewram[owner+16..owner+20].copy_from_slice(&rect);
            for(a,v)in[(20,0x0140u16),(22,0x0c20),(32,current[0]),(34,current[1]),(36,target[0]),(38,target[1])]{m.ewram[owner+a..owner+a+2].copy_from_slice(&v.to_le_bytes());}
        }
        let n=crate::ffta_ui::Node{slot:1,role:crate::ffta_ui::Role::ActionMenu,anchor:crate::ffta_ui::Anchor::Corner{right:true,bottom:true},bounds:[144,80,240,160],length:true};
        assert_eq!(presentation(&m,n,Some(0x1000)),Some((0x1100,Phase::Open)));
        m.ewram[0x1120..0x1122].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(presentation(&m,n,Some(0x1000)),Some((0x1100,Phase::Entering)));
    }
    #[test]fn monster_bank_root_does_not_depend_on_its_parent_cursor(){
        let mut m=scene();m.ewram[0x7f10]=66;
        m.ewram[0x1010..0x1014].copy_from_slice(&[0,9,6,7]);
        for(current,target,phase)in[(0u16,6u16,Phase::Entering),(2,6,Phase::Entering),(6,6,Phase::Open),(6,0,Phase::Exiting),(1,0,Phase::Exiting),(0,0,Phase::Hidden)]{
            for(o,v)in[(0x1020,current),(0x1022,7),(0x1024,target),(0x1026,7)]{m.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());}
            let before=m.ewram.clone();let w=pub_windows(&m,[None;3])[0].unwrap();
            assert_eq!(w.bounds,[0,72,56,128]);assert_eq!(w.phase,phase);assert_eq!(m.ewram,before);
        }
    }
    #[test]fn prison_root_keeps_one_geometry_through_entry_and_exit(){
        let mut m=scene();m.ewram[0x7f10]=63;
        m.ewram[0x1010..0x1014].copy_from_slice(&[0,9,8,9]);
        for(current,target)in[(0,8),(2,8),(8,8),(8,0),(1,0)] {
            for(o,v)in[(0x1020,current as u16),(0x1022,9),(0x1024,target),(0x1026,9)]{m.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());}
            let w=pub_windows(&m,[None;3])[0].unwrap();assert!(w.visible());assert_eq!(w.bounds,[0,72,72,144]);
        }
    }
    #[test]fn store_inventory_keeps_attributes_with_frame_during_all_phases(){
        let mut m=scene();m.ewram[0x7f10]=65;
        m.ewram[0x1010..0x1014].copy_from_slice(&[1,0,26,11]);
        m.ewram[0x1014..0x1016].copy_from_slice(&0x00c0u16.to_le_bytes());
        for(current,target,phase)in[(0,26,Phase::Entering),(8,26,Phase::Entering),(26,26,Phase::Open),(26,0,Phase::Exiting),(3,0,Phase::Exiting)]{
            for(o,v)in[(0x1020,current as u16),(0x1022,11),(0x1024,target),(0x1026,11)]{m.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());}
            let w=pub_windows(&m,[None;3])[1].unwrap();assert_eq!(w.bounds,[0,0,240,160]);assert_eq!(w.phase,phase);
        }
        m.ewram[0x1010]=2;assert!(pub_windows(&m,[None;3])[1].is_none(),"unverified inventory frame excluded");
    }
    #[test]fn all_three_phases_share_geometry_without_a_cursor(){
        let mut m=scene();let mut cached=None;
        for(current,target,phase)in[(0,11,Phase::Entering),(2,11,Phase::Entering),(10,11,Phase::Entering),(11,11,Phase::Open),(11,0,Phase::Exiting),(9,0,Phase::Exiting),(1,0,Phase::Exiting),(0,0,Phase::Hidden)]{
            m.ewram[0x1020..0x1022].copy_from_slice(&(current as u16).to_le_bytes());m.ewram[0x1024..0x1026].copy_from_slice(&(target as u16).to_le_bytes());
            let before=m.ewram.clone();let w=pub_root(&m,cached).unwrap();cached=Some(w.owner);
            assert_eq!(w.phase,phase);assert_eq!(w.bounds,[0,72,96,160]);assert_eq!(m.ewram,before);
        }
        m.ewram[0x7f10]=64;assert!(pub_root(&m,cached).is_none());
    }
    #[test]fn dialog_and_content_keep_canonical_bounds_before_cursor_and_during_exit(){
        for(kind,rect,style,resource,typ,expected)in[
            (Kind::Speech,[0,1,30,8],0x0030u16,0x0b20u16,0x00040015u32,[0,0,240,80]),
            (Kind::Content,[4,1,22,15],0x00c0,0x0c20,0x00010005,[32,8,216,128])]{
            let mut m=scene();m.ewram[0x1010..0x1014].copy_from_slice(&rect);
            m.ewram[0x1014..0x1016].copy_from_slice(&style.to_le_bytes());m.ewram[0x1016..0x1018].copy_from_slice(&resource.to_le_bytes());m.ewram[0x1018..0x101c].copy_from_slice(&typ.to_le_bytes());
            for(current,target)in[(0,rect[2]),(4,rect[2]),(rect[2],rect[2]),(rect[2],0),(2,0)]{
                for(o,v)in[(0x1020,current as u16),(0x1022,rect[3]as u16),(0x1024,target as u16),(0x1026,rect[3]as u16)]{m.ewram[o..o+2].copy_from_slice(&v.to_le_bytes());}
                let w=pub_windows(&m,[None;3])[kind.index()].unwrap();assert!(w.visible());assert_eq!(w.bounds,expected);
            }
        }
    }
    #[test]fn reused_window_is_not_a_pub_root_after_its_style_changes(){let mut m=scene();m.ewram[0x1014]=0xc0;assert!(pub_root(&m,Some(0x1000)).is_none());}
}

#[cfg(test)]mod dialogue_exit_tests {
    use super::*;
    #[test]fn dialogue_phase_tracks_the_verified_window_outside_a_service(){
        use crate::ffta_ui::{Node,Role,Anchor};
        let mut m=MemMap::new(vec![]);let o=0x1000;
        m.io[12..14].copy_from_slice(&0x0b08u16.to_le_bytes());
        m.io[14..16].copy_from_slice(&0x0a08u16.to_le_bytes());
        for(a,v)in[(0,0x06005800u32),(4,0x06005000),(24,0x00040015)]{
            m.ewram[o+a..o+a+4].copy_from_slice(&v.to_le_bytes());
        }
        m.ewram[o+16..o+20].copy_from_slice(&[0,12,30,8]);
        for(a,v)in[(20,0x0030u16),(22,0x0b20),(32,30),(34,8)]{
            m.ewram[o+a..o+a+2].copy_from_slice(&v.to_le_bytes());
        }
        let n=Node{slot:0,role:Role::Dialogue,anchor:Anchor::Edge{right:false,vertical:160.},bounds:[0,88,240,160],length:true};
        assert!(!m.ffta_dialogue.service_speech.active());
        assert_eq!(presentation(&m,n,Some(o)),Some((o,Phase::Exiting)));
        m.ewram[o+24..o+28].fill(0);
        assert!(presentation(&m,n,Some(o)).is_none());
    }
}
