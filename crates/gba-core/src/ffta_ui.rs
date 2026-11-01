//! Host UI scene: semantic panels and native coordinates, independent of world zoom.
//! Pixels still originate from the verified GBA layers; no text is guessed/OCRed.
use crate::ffta_dialogue::{self as d,DialogueState,PanelKind};

pub const LAYERS:usize=4;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Role { Dialogue, ActionInfo, ActionMenu, Stats, Notice, ServiceMenu, ServiceContent, Funds, AttackName, NameEntry }
impl Role {pub fn label(self)->&'static str {match self {
    Self::Dialogue=>"dialogue",Self::ActionInfo=>"action_info",Self::ActionMenu=>"action_menu",
    Self::Stats=>"stats",Self::Notice=>"notice",Self::ServiceMenu=>"service_menu",Self::ServiceContent=>"service_content",Self::Funds=>"clan_funds",Self::AttackName=>"attack_name",Self::NameEntry=>"name_entry",
}}}
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Anchor {
    Edge {right:bool,vertical:f32},
    Corner {right:bool,bottom:bool},
    Center,
    Modal,
    PanelCenter,
    TopCenter,
}
#[derive(Clone,Copy,Debug)]
pub struct Node {
    pub slot:usize,
    pub role:Role,
    pub anchor:Anchor,
    /// Logical geometry; a blinking cursor cannot change the host origin.
    pub bounds:[i32;4],
    pub length:bool,
}
impl Node {
    pub fn fitted_scale_with_length(mut self,requested:f32,width:f32,length:f32)->f32 {
        if self.length && length.is_finite() {
            let amount=((if self.role==Role::Dialogue {120.}else{(self.bounds[2]-self.bounds[0])as f32})
                *(length.clamp(1.,2.)-1.)).round()as i32;
            match self.anchor {
                Anchor::Edge{right,..}|Anchor::Corner{right,..}=>{
                    if right {self.bounds[0]=(self.bounds[0]-amount).max(-120);}
                    else{self.bounds[2]=(self.bounds[2]+amount).min(360);}
                }
                _=>{}
            }
        }
        self.fitted_scale(requested,width)
    }
    /// Uniform scale only. Oversized panels fit the viewport instead of
    /// clipping their last translated label or stretching one axis.
    pub fn fitted_scale(self,requested:f32,viewport_width:f32)->f32 {
        let requested=if requested.is_finite(){requested.clamp(0.5,2.)}else{1.};
        let [l,t,r,b]=self.bounds.map(|v|v as f32);
        let (w,h)=match self.anchor {
            Anchor::Edge{right,vertical}=>(if right{240.-l}else{r},if vertical>=80.{160.-t}else{b}),
            Anchor::Corner{..}|Anchor::Modal|Anchor::PanelCenter|Anchor::TopCenter=>(r-l,b-t),
            Anchor::Center=>(2.*(120.-l).abs().max((r-120.).abs()),2.*(80.-t).abs().max((b-80.).abs())),
        };
        let available_height=if matches!(self.anchor,Anchor::Modal){(160.-t.max(0.)).max(1.)}else{160.};
        requested.min(viewport_width.max(1.)/w.max(1.)).min(available_height/h.max(1.)).max(0.5)
    }
}
#[derive(Clone,Copy,Debug)]
pub struct ObjectReference {pub slot:usize,pub attrs:[u16;3],pub position:[i32;2],pub role:Role}
/// References of verified UI object families, not all world sprites.
pub fn object_references(mem:&crate::MemMap,scene:&Scene)->Vec<ObjectReference> {
    if scene.nodes.iter().all(Option::is_none){return Vec::new();}
    mem.oam.chunks_exact(8).enumerate().filter_map(|(slot,o)|{
        let a:[u16;3]=std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]]));
        if a[0]&0x300==0x200{return None;}
        let x=(a[1]&511)as i32;let x=if x>=256{x-512}else{x};
        let y=(a[0]&255)as i32;let y=if y>=160{y-256}else{y};
        let role=if mem.ffta_dialogue.service_layout && d::service_counter(a,x,y) {Role::Funds}
        else if d::list_marker(a) {
            if mem.ffta_dialogue.extra.active(){Role::ActionMenu}else{Role::ServiceContent}
        }else if d::portrait(a,x) || (mem.ffta_dialogue.action_layout && d::action_portrait(a,x,y)) {
            if mem.ffta_dialogue.service_speech.active(){Role::Dialogue}else{scene.nodes[0].map(|n|n.role)?}
        }else if mem.ffta_dialogue.extra.kind==PanelKind::Stats
            && mem.ffta_dialogue.extra.contains_stats(a,x,y,1,1) {Role::Stats}
        else if d::continuation(a) {Role::Dialogue}else{return None;};
        Some(ObjectReference{slot,attrs:a,position:[x,y],role})
    }).collect()
}
pub fn funds_bounds(mem:&crate::MemMap)->Option<[i32;4]> {
    if !mem.ffta_dialogue.service_layout{return None;}
    let mut rectangles=Vec::new();
    const SIZES:[[(i32,i32);4];3]=[
        [(8,8),(16,16),(32,32),(64,64)],[(16,8),(32,8),(32,16),(64,32)],[(8,16),(8,32),(16,32),(32,64)]];
    for o in mem.oam.chunks_exact(8) {
        let a:[u16;3]=std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]]));
        let (x,y)=((a[1]&511)as i32,(a[0]&255)as i32);
        if !d::service_counter(a,x,y) || a[0]>>14>=3{continue;}
        let (w,h)=SIZES[(a[0]>>14)as usize][(a[1]>>14)as usize];
        // Keep complete geometry while the guest slides the counter through
        // the viewport edge. Presentation clips pixels, never its anchor.
        let r=[x,y,x+w,y+h];
        if r[0]<r[2] && r[1]<r[3]{rectangles.push(r);}
    }
    // The counter is a contiguous bottom-right widget, not the union of
    // every priority-zero number in the lower half of an inventory.
    let mut bounds=*rectangles.iter().max_by_key(|r|(r[3],r[2]))?;
    loop {
        let previous=bounds;
        rectangles.retain(|r|{
            let connected=r[0]<=bounds[2] && r[2]>=bounds[0]
                && r[1]<=bounds[3] && r[3]>=bounds[1];
            if connected {bounds=[bounds[0].min(r[0]),bounds[1].min(r[1]),bounds[2].max(r[2]),bounds[3].max(r[3])];}
            !connected
        });
        if bounds==previous{break;}
    }
    Some(bounds)
}
/// Clan Funds is the ten-by-three gold BG2 frame, not an IWRAM terrain window.
/// Match its four original frame corners; another panel borrowing this area
/// must never be cut out just because it occupies the same screen position.
pub fn funds_window(mem:&crate::MemMap)->Option<(usize,[i32;4])>{
    if !mem.ffta_dialogue.service_layout{return None;}
    let half=|b:&[u8],o|b.get(o..o+2).map(|v|u16::from_le_bytes(v.try_into().unwrap()));
    if half(&mem.io,12)!=Some(0x0b08) || half(&mem.io,14)!=Some(0x0a08){return None;}
    let base=0x5800;
    let tile=|x:usize,y:usize|half(&mem.vram,base+(y*32+x)*2);
    for y in 0..18 {for x in 0..21 {
        if tile(x,y)!=Some(0xd052) || tile(x+9,y)!=Some(0xd452)
            || tile(x,y+2)!=Some(0xd05e) || tile(x+9,y+2)!=Some(0xd062){continue;}
        return Some((2,[x as i32*8,y as i32*8,(x+10)as i32*8,(y+3)as i32*8]));
    }}
    None
}
#[derive(Clone,Debug,Default)]
pub struct Scene {pub nodes:[Option<Node>;LAYERS]}
impl Scene {
    /// Build once per emulated frame. Rendering, mouse and inspection consume
    /// the same descriptors rather than reclassifying independently.
    pub fn capture(state:&DialogueState)->Self {Self::capture_with_bounds(state,[None;2])}
    pub fn capture_with_bounds(state:&DialogueState,cached:[Option<[i32;4]>;2])->Self {
        let mut out=Self::default();
        if let Some((_,right))=state.layer().filter(|_|!state.battle_notice()) {
            let mut bounds=state.native_bounds.or(cached[0]).or_else(||d::panel_bounds(&state.pixels,&state.backing)).unwrap_or([0,0,240,160]);
            if state.action_layout {
                // This HUD is one fixed native canvas. Its animated visible
                // pixels (including long portrait ears) cannot resize/reanchor it.
                bounds=[0,48,160,160];
            }
            let (role,anchor,length)=if state.naming_layout {
                bounds=[0,0,240,160];(Role::NameEntry,Anchor::Center,false)
            }else if let Some(rect)=state.announcement_bounds {
                bounds=rect;
                (Role::Notice,Anchor::Modal,false)
            }else if state.attack_name {
                bounds=state.native_bounds.or(cached[0]).unwrap_or(bounds);
                (Role::AttackName,Anchor::TopCenter,false)
            }else if state.service_list {
                bounds=state.service_list_bounds.unwrap_or(bounds);
                (Role::ServiceContent,Anchor::PanelCenter,false)
            }else if state.service_layout {
                (Role::ServiceContent,Anchor::Edge{right:true,vertical:0.},false)
            }else if state.action_layout {
                (Role::ActionInfo,Anchor::Corner{right:false,bottom:true},true)
            }else if state.split_speech {
                (Role::Dialogue,Anchor::Edge{right,vertical:state.vertical_anchor()},true)
            }else if state.shop_layout || state.site_layout {
                (Role::ServiceContent,Anchor::Center,false)
            }else if state.town_layout {
                (Role::ServiceContent,Anchor::Edge{right:true,vertical:0.},false)
            }else{(Role::Dialogue,Anchor::Edge{right,vertical:state.vertical_anchor()},true)};
            out.nodes[0]=Some(Node{slot:0,role,anchor,bounds,length});
        }
        let panel=&state.extra;
        if state.separate && panel.active() {
            let bounds=if panel.kind==PanelKind::Actions {
                panel.bounds.or(cached[1]).or_else(||d::panel_bounds(&panel.pixels,&panel.backing))
            }else{state.stats_target.filter(|_|panel.kind==PanelKind::Stats).or(panel.bounds)};
            if let Some(mut bounds)=bounds {
                if panel.kind==PanelKind::Actions && !state.town_layout {
                    bounds[0]=(bounds[0]-8).max(0);bounds[2]=(bounds[2]+8).min(240);
                    bounds[3]=(bounds[3]+16).min(160);
                }
                if panel.kind==PanelKind::Stats && state.stats_target.is_none() {if let Some(ink)=d::pixel_bounds(&panel.pixels) {
                    bounds=[bounds[0].min(ink[0]),bounds[1].min(ink[1]),bounds[2].max(ink[2]),bounds[3].max(ink[3])];
                }}
                let role=if state.service_layout {Role::ServiceMenu}else{match panel.kind {
                    PanelKind::Stats=>Role::Stats,PanelKind::Notice=>Role::Notice,_=>Role::ActionMenu,
                }};
                let anchor=if panel.kind==PanelKind::Notice {Anchor::Modal}
                    else if state.service_layout {Anchor::Corner{right:false,bottom:true}}
                    else if state.split_speech {Anchor::Center}
                    else{Anchor::Corner{right:if panel.kind==PanelKind::Stats {state.stats_target.map_or(bounds[0]+bounds[2]>=240,|_|state.stats_right)}else{state.town_layout || bounds[0]+bounds[2]>=240},bottom:bounds[1]+bounds[3]>=160}};
                out.nodes[1]=Some(Node{slot:1,role,anchor,bounds,
                    length:!state.town_layout && panel.kind!=PanelKind::Notice});
            }
        }
        out
    }
}
#[cfg(test)]mod tests {
    #[test]fn battle_notice_hides_suspended_unit_hud_and_restores_it_after_cancel() {
        let mut s=DialogueState::default();s.separate=true;s.action_layout=true;
        s.placement=Some(d::Placement::new(false,568,0,Default::default()));
        s.pixels=vec![d::UiPixel::opaque(31);480*160];
        s.extra.kind=PanelKind::Notice;s.extra.bounds=Some([16,16,224,64]);
        let scene=Scene::capture(&s);
        assert!(scene.nodes[0].is_none());
        assert_eq!(scene.nodes[1].unwrap().role,Role::Notice);
        s.extra.kind=PanelKind::None;s.extra.bounds=None;
        assert!(!s.battle_notice());
        assert_eq!(Scene::capture(&s).nodes[0].unwrap().role,Role::ActionInfo);
    }

    use super::*;
    #[test]fn gold_counter_requires_its_whole_frame_not_an_unrelated_window(){
        let mut m=crate::MemMap::new(vec![]);m.ffta_dialogue.service_layout=true;
        m.io[12..14].copy_from_slice(&0x0b08u16.to_le_bytes());
        m.io[14..16].copy_from_slice(&0x0a08u16.to_le_bytes());
        // The old false-positive rectangle belongs to terrain, not Clan Funds.
        m.iwram[0xac6..0xac8].copy_from_slice(&0xc000u16.to_le_bytes());
        m.iwram[0xaf8..0xafc].copy_from_slice(&[19,18,11,2]);
        assert!(funds_window(&m).is_none());
        for(x,y,t)in[(20,17,0xd052u16),(29,17,0xd452),(20,19,0xd05e),(29,19,0xd062)]{
            let a=0x5800+(y*32+x)*2;m.vram[a..a+2].copy_from_slice(&t.to_le_bytes());
        }
        assert_eq!(funds_window(&m),Some((2,[160,136,240,160])));
        m.vram[0x5800+(19*32+29)*2]=0;
        assert!(funds_window(&m).is_none(),"overwritten by a list: no counter extraction");
    }
    #[test]fn funds_do_not_merge_disconnected_inventory_numbers_or_portraits() {
        let mut m=crate::Machine::new(vec![]);m.bus.ffta_dialogue.service_layout=true;
        for o in m.bus.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        for (slot,a) in [[0x4090u16,0x80a0,0x21a0],[0x0098,0x80a0,0x21b0],
            [0x0068,0x0098,0x21c0],[0x2090,0x80c0,0x2240]].into_iter().enumerate() {
            for (i,v) in a.iter().enumerate(){m.bus.oam[slot*8+i*2..slot*8+i*2+2].copy_from_slice(&v.to_le_bytes());}
        }
        assert_eq!(funds_bounds(&m.bus),Some([160,144,192,184]));
        assert!(d::service_object([0x0070,0x0070,0x21c0]));
        assert!(!d::service_object([0x2070,0x0070,0x2240]));
    }
    #[test]fn maximum_length_and_size_keep_extended_panels_inside_narrow_viewports() {
        for right in [false,true] {
            let n=Node{slot:1,role:Role::Stats,anchor:Anchor::Corner{right,bottom:true},
                bounds:if right{[152,104,240,152]}else{[0,104,88,152]},length:true};
            let size=n.fitted_scale_with_length(2.,284.,2.);
            assert!(176.*size<=284.01);
            assert_eq!(size,n.fitted_scale_with_length(2.,284.,2.));
        }
    }
    #[test]fn large_and_long_panels_fit_with_one_uniform_scale() {
        for bounds in [[0,80,240,160],[0,0,240,160],[152,104,240,152]] {
            for anchor in [Anchor::Corner{right:true,bottom:true},Anchor::Center,Anchor::Modal] {
                let n=Node{slot:0,role:Role::ActionMenu,anchor,bounds,length:true};
                let scale=n.fitted_scale(2.,284.);
                assert!(scale<=2. && scale.is_finite());
                assert!((bounds[2]-bounds[0])as f32*scale<=284.01);
                assert!((bounds[3]-bounds[1])as f32*scale<=160.01);
            }
        }
    }
    #[test]fn missing_panels_are_removed_immediately_and_stats_use_owner_geometry() {
        let mut s=DialogueState::default();s.separate=true;
        s.extra.kind=PanelKind::Stats;s.extra.bounds=Some([152,104,240,152]);
        // A partially entering OBJ must not replace its full owner bounds.
        s.extra.pixels=vec![d::UiPixel::default();480*160];
        s.extra.pixels[110*480+350]=d::UiPixel::opaque(31);
        let n=Scene::capture(&s).nodes[1].unwrap();
        assert_eq!(n.bounds,[152,104,240,152]);assert_eq!(n.role,Role::Stats);
        s.extra.kind=PanelKind::None;assert!(Scene::capture(&s).nodes.iter().all(Option::is_none));
    }
    #[test]fn speech_animation_scales_from_complete_backing_not_revealed_pixels() {
        let mut s=DialogueState::default();s.separate=true;
        s.native_bounds=Some([0,96,240,160]);
        s.placement=Some(d::Placement::new(false,568,0,Default::default()));
        s.attachment=Some((false,160.));
        s.pixels=vec![d::UiPixel::default();480*160];
        let expected=Scene::capture(&s).nodes[0].unwrap();
        for y in [96,104,128,152] {
            s.pixels.fill(d::UiPixel::default());
            s.pixels[y*480+120]=d::UiPixel::opaque(31);
            let n=Scene::capture(&s).nodes[0].unwrap();
            assert_eq!(n.bounds,expected.bounds);
            assert_eq!(n.fitted_scale(2.,568.),expected.fitted_scale(2.,568.));
        }
    }
    #[test]fn action_geometry_does_not_follow_partial_pixels_or_blinking_cursor() {
        let mut s=DialogueState::default();s.separate=true;
        s.extra.kind=PanelKind::Actions;s.extra.bounds=Some([160,72,240,152]);
        s.extra.pixels=vec![d::UiPixel::default();480*160];
        let expected=Scene::capture(&s).nodes[1].unwrap();
        for x in [160usize,176,208,232] {
            s.extra.pixels.fill(d::UiPixel::default());
            s.extra.pixels[100*480+x+120]=d::UiPixel::opaque(31);
            let current=Scene::capture(&s).nodes[1].unwrap();
            assert_eq!(current.bounds,expected.bounds);assert_eq!(current.anchor,expected.anchor);
            assert_eq!(current.fitted_scale(2.,284.),expected.fitted_scale(2.,284.));
        }
    }
    #[test]fn service_menu_and_content_have_opposite_stable_anchors() {
        let mut s=DialogueState::default();s.separate=true;s.service_layout=true;
        s.extra.kind=PanelKind::Actions;s.extra.bounds=Some([0,76,96,160]);
        let n=Scene::capture(&s).nodes[1].unwrap();
        assert_eq!(n.role,Role::ServiceMenu);
        assert_eq!(n.anchor,Anchor::Corner{right:false,bottom:true});
    }
}
