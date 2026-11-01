//! Host-owned, translation-only UI transitions. Never change guest memory,
//! logical selectors, texture scale, alpha coefficients or world-camera state.
use gba_core::{ffta_ui::{Node,Anchor,Role},ffta_ui_window::Phase};
use crate::ui_layout::Layout;
use std::time::Instant;
const SECONDS:f64=0.15;
const DIALOGUE_EXIT_SECONDS:f64=0.22;
// The portrait producer slides OBJ tile 0x240 before/after the text window.
// Keep the last complete host canvas instead of caching those partial faces.
fn action_portrait_settled(mem:&gba_core::MemMap)->bool {
    let portrait=|a:[u16;3]|a[0]&0x2000!=0 && a[0]&0x300!=0x200
        && a[2]&0xc00<=0x400 && a[2]&1023==0x240;
    let host=mem.ffta_sprites.active.as_deref().unwrap_or(&[]);
    if host.iter().any(|s|portrait(s.attrs)) {
        return host.iter().any(|s|portrait(s.attrs) && s.x==0);
    }
    mem.oam.chunks_exact(8).any(|o| {
        let a=std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]]));
        portrait(a) && a[1]&511==0
    })
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Default)]
enum Mode {#[default] Waiting,Entering,Open,Exiting}
#[derive(Default)]
pub struct Panel {
    node:Option<Node>,pub owner:Option<usize>,pixels:Vec<[u8;4]>,
    mode:Mode,changed:f64,from:f32,length:f32,
    character:Option<(u8,u8)>,next_character:Option<(u8,u8)>,
}
impl Panel {
    fn level(&self,now:f64)->f32 {
        let duration=if self.mode==Mode::Exiting && self.node.is_some_and(|n|n.role==Role::Dialogue) {
            DIALOGUE_EXIT_SECONDS
        }else{SECONDS};
        let t=((now-self.changed)/duration).clamp(0.,1.)as f32;
        let ease=t*t*(3.-2.*t);
        match self.mode {Mode::Waiting=>0.,Mode::Open=>1.,
            Mode::Entering=>self.from+(1.-self.from)*ease,Mode::Exiting=>self.from*(1.-ease)}
    }
    fn identity(a:Node,b:Node)->bool {a.role==b.role && a.anchor==b.anchor && a.bounds==b.bounds}
    fn sync(&mut self,now:f64,node:Option<Node>,pixels:&[[u8;4]],phase:Phase) {
        let has_pixels=pixels.iter().any(|p|p[2]!=0 || p[3]!=16);
        let character_panel=|n:Node|matches!(n.role,Role::Stats|Role::ActionInfo);
        let replace_character=node.is_some_and(character_panel)
            && self.node.is_some_and(character_panel) && self.drawing(now).is_some()
            && has_pixels && phase==Phase::Open
            && (self.mode==Mode::Exiting
                || self.next_character.zip(self.character).is_some_and(|(a,b)|a!=b)
                || self.node.zip(node).is_some_and(|(a,b)|!Self::identity(a,b)));
        if replace_character {
            self.node=node;self.mode=Mode::Open;self.from=1.;self.changed=now;self.owner=None;
            self.pixels.clear();self.pixels.extend_from_slice(pixels);
            self.character=self.next_character;
            return;
        }
        if node.is_some_and(character_panel) && has_pixels && phase==Phase::Open {
            self.character=self.next_character;
        }
        // Intro speech can close without publishing a window Phase. Its
        // backing contracts by 32 pixels per frame. That is not a new panel:
        // keep the complete canvas and geometry while the host slides it out.
        let contracting=self.node.zip(node).is_some_and(|(old,new)|
            old.role==Role::Dialogue && new.role==Role::Dialogue && old.anchor==new.anchor
            && new.bounds[0]>=old.bounds[0] && new.bounds[1]>=old.bounds[1]
            && new.bounds[2]<=old.bounds[2] && new.bounds[3]<=old.bounds[3]
            && new.bounds[2]-new.bounds[0]+8<=old.bounds[2]-old.bounds[0]);
        let phase=if phase==Phase::Open && contracting {Phase::Exiting}else{phase};
        let closing_dialogue=self.node.is_some_and(|n|matches!(n.role,Role::Dialogue|Role::NameEntry))
            && (matches!(phase,Phase::Exiting|Phase::Hidden) || node.is_none() || !has_pixels);
        if let Some(node)=node.filter(|_|!closing_dialogue) {
            if self.node.is_none_or(|old|!Self::identity(old,node)) {
                self.node=Some(node);self.mode=Mode::Waiting;self.pixels.clear();self.owner=None;
            }
        }
        let Some(own)=self.node else{return;};
        if node.is_none() || matches!(phase,Phase::Exiting|Phase::Hidden) || phase==Phase::Open && !has_pixels {
            // The yellow ability capsule explicitly disappears on close.
            if own.role==Role::AttackName {self.clear();return;}
            if self.mode!=Mode::Exiting {
                let from=self.level(now);self.mode=Mode::Exiting;self.from=from;self.changed=now;
            }
            if self.level(now)<=0. {self.clear();}
            return;
        }
        if phase==Phase::Entering {
            // Wait for a complete guest canvas; never display its old reveal.
            // If this is a submenu replacing another at the same geometry,
            // keep the last complete image while it slides out.
            if matches!(self.mode,Mode::Open|Mode::Entering) {
                self.from=self.level(now);self.changed=now;self.mode=Mode::Exiting;
            }
            return;
        }
        if self.pixels.as_slice()!=pixels {self.pixels.clear();self.pixels.extend_from_slice(pixels);}
        if matches!(self.mode,Mode::Waiting|Mode::Exiting) {
            self.from=if self.mode==Mode::Exiting{self.level(now)}else{0.};
            self.changed=now;self.mode=Mode::Entering;
        } else if self.mode==Mode::Entering && self.level(now)>=1. {self.mode=Mode::Open;}
    }
    fn clear(&mut self){self.node=None;self.mode=Mode::Waiting;self.owner=None;self.pixels.clear();self.character=None;}
    pub fn pixels(&self)->&[[u8;4]]{&self.pixels}
    pub fn drawing(&self,now:f64)->Option<Node>{(self.level(now)>0. && !self.pixels.is_empty()).then_some(self.node).flatten()}
    pub fn input(&self)->bool{if self.node.is_some_and(|n|n.role==Role::NameEntry){self.mode==Mode::Open}else{matches!(self.mode,Mode::Open|Mode::Entering)}}
    pub fn layout(&self,mut layout:Layout,now:f64)->Layout {
        let Some(node)=self.node else{return layout;};
        let hidden=1.-self.level(now);let sx=layout.rect[2]/480.;let sy=layout.rect[3]/160.;
        let [mut l,t,mut r,b]=node.bounds.map(|v|v as f32);
        if node.length {
            let width=if node.role==Role::Dialogue{120.}else{r-l};
            let extra=width*(self.length.clamp(1.,2.)-1.);
            match node.anchor {
                Anchor::Edge{right,..}|Anchor::Corner{right,..}=>if right{l-=extra;}else{r+=extra;},
                _=>{},
            }
        }
        let direction=if matches!(node.role,Role::ActionInfo|Role::NameEntry){0}else{match node.anchor {
            Anchor::Edge{right,..}|Anchor::Corner{right,..}=>if right{1}else{0},
            Anchor::TopCenter|Anchor::Modal=>2,
            _=>if t+b>=160.{3}else{2},
        }};
        let distance=match direction {
            0=>layout.clip[0]-(layout.rect[0]+(r+120.)*sx)-8.,
            1=>layout.clip[0]+layout.clip[2]-(layout.rect[0]+(l+120.)*sx)+8.,
            2=>layout.clip[1]-(layout.rect[1]+b*sy)-8.,
            _=>layout.clip[1]+layout.clip[3]-(layout.rect[1]+t*sy)+8.,
        };
        layout.rect[if direction<2{0}else{1}]+=distance*hidden;
        if matches!(node.role,Role::ActionInfo|Role::ActionMenu) {
            // The texture is larger than the widget. Never let unrelated
            // atlas pixels outside its canonical rectangle flash at an edge.
            let left=(layout.rect[0]+(l+120.)*sx).max(layout.clip[0]);
            let top=(layout.rect[1]+t*sy).max(layout.clip[1]);
            let right=(layout.rect[0]+(r+120.)*sx).min(layout.clip[0]+layout.clip[2]);
            let bottom=(layout.rect[1]+b*sy).min(layout.clip[1]+layout.clip[3]);
            layout.clip=[left,top,(right-left).max(0.),(bottom-top).max(0.)];
        }
        layout
    }
}
pub struct Motion {pub panels:[Panel;4],clock:Instant,stage:Option<u16>}
impl Default for Motion {fn default()->Self{Self{panels:Default::default(),clock:Instant::now(),stage:None}}}
impl Motion {
    pub fn now(&self)->f64{self.clock.elapsed().as_secs_f64()}
    pub fn sync(&mut self,mem:&gba_core::MemMap,nodes:[Option<Node>;4],pixels:[&[[u8;4]];4]) {
        let stage=gba_core::ffta_surface::Descriptor::read(mem).map(|s|s.id);
        if self.stage!=stage {for p in &mut self.panels{p.clear();}self.stage=stage;}
        let now=self.now();
        let selected=gba_core::ffta_battle_mouse::selected_cell(mem);
        for i in 0..4 {
            // Modal deployment must never retain a ghost of the selected unit.
            if i==0 && mem.ffta_dialogue.battle_notice(){self.panels[i].clear();continue;}
            let cached=self.panels[i].owner;
            let candidate=nodes[i].and_then(|n|{
                if cached.is_some() || self.panels[i].node.is_none() || mem.frames%8==0 {
                    gba_core::ffta_ui_window::presentation(mem,n,cached)
                }else{None}
            });
            let phase=if nodes[i].is_some_and(|n|n.role==Role::NameEntry){
                match mem.ffta_dialogue.naming_phase {1=>Phase::Entering,3=>Phase::Exiting,_=>Phase::Open}
            }else if nodes[i].is_some_and(|n|n.role==Role::ActionInfo)
                && !action_portrait_settled(mem) {
                // No complete portrait yet, or its native exit has started.
                // Panel::sync freezes the complete canvas and retracts it.
                Phase::Entering
            }else{candidate.map_or(Phase::Open,|v|v.1)};
            self.panels[i].next_character=selected;
            self.panels[i].length=mem.ffta_dialogue.style.length;
            self.panels[i].sync(now,nodes[i],pixels[i],phase);
            if let Some((owner,_))=candidate{self.panels[i].owner=Some(owner);}
        }
    }
    pub fn drawing(&self,i:usize)->Option<Node>{self.panels[i].drawing(self.now())}
    pub fn layout(&self,i:usize,layout:Layout)->Layout{self.panels[i].layout(layout,self.now())}
}
#[cfg(test)]mod tests {
    use super::*;
    fn node(right:bool)->Node{Node{slot:0,role:Role::ActionMenu,anchor:Anchor::Corner{right,bottom:true},bounds:[160,64,240,152],length:false}}
    fn image()->Vec<[u8;4]>{vec![[1,0,14,9];480*160]}
    #[test]fn dialogue_exit_keeps_complete_canvas_and_geometry_while_guest_shrinks(){
        let mut p=Panel::default();let mut n=node(false);n.role=Role::Dialogue;
        let full=image();p.sync(0.,Some(n),&full,Phase::Open);
        p.sync(0.2,Some(n),&full,Phase::Open);
        let mut shrinking=n;shrinking.bounds=[170,80,220,120];
        p.sync(0.3,Some(shrinking),&[],Phase::Exiting);
        assert!(p.node.is_some_and(|own|Panel::identity(own,n)));assert_eq!(p.pixels(),full);
        assert_eq!(p.level(0.3),1.);
        p.sync(0.4,None,&[],Phase::Hidden);
        assert!(p.node.is_some_and(|own|Panel::identity(own,n)));assert_eq!(p.pixels(),full);
        assert!(p.level(0.4)>0. && p.level(0.4)<1.);
        p.sync(0.53,None,&[],Phase::Hidden);assert!(p.drawing(0.53).is_none());
    }
    #[test]fn upper_intro_dialogue_contracting_without_a_phase_keeps_its_full_canvas(){
        for right in [false,true] {
            let mut p=Panel::default();let full=image();
            let n=Node{slot:0,role:Role::Dialogue,anchor:Anchor::Edge{right,vertical:0.},
                bounds:if right{[35,13,240,67]}else{[0,13,205,67]},length:true};
            p.sync(0.,Some(n),&full,Phase::Open);p.sync(0.2,Some(n),&full,Phase::Open);
            let base=Layout{rect:[-120.,0.,480.,160.],clip:[0.,0.,568.,160.]};
            for (i,width)in[173,141,109,77,45,13].into_iter().enumerate(){
                let mut closing=n;closing.bounds=if right{[240-width,13,240,67]}else{[0,13,width,67]};
                p.sync(0.3+i as f64/60.,Some(closing),&image(),Phase::Open);
                assert!(p.node.is_some_and(|own|Panel::identity(own,n)));
                assert_eq!(p.pixels(),full);
            }
            assert!(p.drawing(0.45).is_some());assert!(!p.input());
            let at=p.layout(base,0.45).rect[0];
            if right{assert!(at>base.rect[0]);}else{assert!(at<base.rect[0]);}
            // The producer then publishes an empty full-screen rectangle.
            let mut empty=n;empty.bounds=[0,0,240,160];
            p.sync(0.45,Some(empty),&[],Phase::Open);
            assert!(p.node.is_some_and(|own|Panel::identity(own,n)));
            assert_eq!(p.pixels(),full);
            p.sync(0.53,None,&[],Phase::Open);assert!(p.drawing(0.53).is_none());
        }
    }
    #[test]fn empty_geometry_change_cannot_erase_a_top_dialogue_before_its_exit(){
        let mut p=Panel::default();let full=image();let mut n=node(false);
        n.role=Role::Dialogue;n.anchor=Anchor::Edge{right:false,vertical:0.};n.bounds=[0,13,205,67];
        p.sync(0.,Some(n),&full,Phase::Open);p.sync(0.2,Some(n),&full,Phase::Open);
        let mut empty=n;empty.bounds=[0,0,240,160];empty.anchor=Anchor::Edge{right:true,vertical:0.};
        p.sync(0.3,Some(empty),&[],Phase::Open);
        assert!(p.node.is_some_and(|own|Panel::identity(own,n)));
        assert_eq!(p.pixels(),full);assert_eq!(p.level(0.3),1.);
        assert!(p.drawing(0.4).is_some());
    }
    #[test]fn name_keyboard_waits_for_complete_native_canvas_and_owns_both_slides(){
        let mut p=Panel::default();let mut n=node(false);n.role=Role::NameEntry;
        n.anchor=Anchor::Center;n.bounds=[0,0,240,160];
        let full=image();let partial=vec![[0,0,0,16];480*160];
        p.sync(0.,Some(n),&partial,Phase::Entering);assert!(p.drawing(0.3).is_none());
        p.sync(0.4,Some(n),&full,Phase::Open);assert!(!p.input());
        let base=Layout{rect:[-120.,0.,480.,160.],clip:[0.,0.,568.,160.]};
        assert!(p.layout(base,0.45).rect[0]<base.rect[0]);
        p.sync(0.6,Some(n),&full,Phase::Open);assert!(p.input());
        p.sync(0.7,Some(n),&partial,Phase::Exiting);assert!(!p.input());
        assert_eq!(p.pixels(),full);assert!(p.layout(base,0.775).rect[0]<base.rect[0]);
        p.sync(0.9,None,&[],Phase::Hidden);assert!(p.drawing(0.9).is_none());
    }
    #[test]fn portrait_exit_retracts_complete_canvas_before_guest_face_disappears(){
        use gba_core::ffta_sprites::HostSprite;
        let mut mem=gba_core::MemMap::new(vec![]);
        for o in mem.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        mem.ffta_sprites.active=Some(vec![HostSprite {
            x:0,y:88,attrs:[0xa058,0xd000,0x640],..Default::default()
        }]);
        assert!(action_portrait_settled(&mem));
        let mut p=Panel::default();let mut n=node(false);
        n.role=Role::ActionInfo;n.bounds=[0,48,160,160];
        let full=image();p.sync(0.,Some(n),&full,Phase::Open);
        p.sync(0.2,Some(n),&full,Phase::Open);
        mem.ffta_sprites.active.as_mut().unwrap()[0].x=-16;
        assert!(!action_portrait_settled(&mem));
        // The guest's outgoing canvas has already lost its portrait pixels.
        let partial=vec![[0,0,0,16];480*160];
        p.sync(0.3,Some(n),&partial,Phase::Entering);
        assert_eq!(p.pixels(),full);
        let base=Layout{rect:[-120.,0.,480.,160.],clip:[0.,0.,568.,160.]};
        assert_eq!(p.layout(base,0.3).rect,base.rect);
        assert!(p.layout(base,0.375).rect[0]<base.rect[0]);
        assert!(p.drawing(0.375).is_some());
        p.sync(0.4,None,&[],Phase::Hidden);
        assert_eq!(p.pixels(),full);
        assert!(p.drawing(0.451).is_none());
        // A different selected unit still replaces the current one instantly.
        p.next_character=Some((1,2));p.sync(1.,Some(n),&full,Phase::Open);
        p.next_character=Some((2,2));p.sync(1.2,Some(n),&full,Phase::Open);
        assert_eq!(p.level(1.2),1.);
    }
    #[test]fn action_widget_does_not_expose_atlas_pixels_outside_its_frame(){
        let mut p=Panel::default();p.sync(0.,Some(node(true)),&image(),Phase::Open);
        let base=Layout{rect:[-120.,0.,480.,160.],clip:[0.,0.,568.,160.]};
        let layout=p.layout(base,0.2);
        assert!(layout.source(20.,90.).is_none());
        assert!(layout.source(170.,10.).is_none());
        assert!(layout.source(170.,90.).is_some());
    }
    #[test]fn character_and_target_panels_use_their_own_entry_and_exit_edge(){
        for right in [false,true] {for role in [Role::Stats,Role::ActionInfo] {
            let mut p=Panel::default();let mut n=node(right);n.role=role;
            let base=Layout{rect:[-120.,0.,480.,160.],clip:[0.,0.,568.,160.]};
            p.sync(0.,Some(n),&image(),Phase::Open);
            let from_right=role==Role::Stats && right;
            if from_right{assert!(p.layout(base,0.).rect[0]>p.layout(base,0.075).rect[0]);}
            else{assert!(p.layout(base,0.).rect[0]<p.layout(base,0.075).rect[0]);}
            assert_eq!(p.layout(base,0.15).rect,base.rect);
            p.sync(0.2,None,&[],Phase::Hidden);
            if from_right{
                assert!(p.layout(base,0.275).rect[0]>base.rect[0]);
                assert!(p.layout(base,0.35).rect[0]>p.layout(base,0.275).rect[0]);
            }else{
                assert!(p.layout(base,0.275).rect[0]<base.rect[0]);
                assert!(p.layout(base,0.35).rect[0]<p.layout(base,0.275).rect[0]);
            }
        }}
    }
    #[test]fn character_to_character_is_instant_but_empty_tile_slides_out(){
        let mut p=Panel::default();let mut n=node(false);n.role=Role::Stats;
        p.next_character=Some((2,3));p.sync(0.,Some(n),&image(),Phase::Open);
        assert_eq!(p.level(0.075),0.5);
        let other=vec![[2,0,14,9];480*160];
        p.next_character=Some((3,3));p.sync(0.08,Some(n),&other,Phase::Open);
        assert_eq!(p.level(0.08),1.);assert_eq!(p.pixels(),other);
        p.sync(0.1,None,&[],Phase::Hidden);
        assert!(p.level(0.175)>0. && p.level(0.175)<1.);
        assert_eq!(p.pixels(),other);
        // Another character while the previous panel exits replaces it now.
        n.anchor=Anchor::Corner{right:true,bottom:true};
        p.next_character=Some((4,3));p.sync(0.18,Some(n),&image(),Phase::Open);
        assert_eq!(p.level(0.18),1.);assert_eq!(p.pixels(),image());
        p.sync(0.2,None,&[],Phase::Hidden);p.sync(0.4,None,&[],Phase::Hidden);
        assert!(p.node.is_none());
        p.next_character=Some((5,3));p.sync(0.5,Some(n),&image(),Phase::Open);
        assert_eq!(p.level(0.5),0.);assert_eq!(p.level(0.575),0.5);
    }
    #[test]fn waits_for_complete_canvas_and_freezes_exact_alpha_on_exit(){
        let mut p=Panel::default();let n=node(true);let full=image();
        p.sync(0.,Some(n),&full,Phase::Entering);assert!(p.drawing(1.).is_none());
        p.sync(1.,Some(n),&full,Phase::Open);assert!(p.drawing(1.08).is_some());
        p.sync(1.2,Some(n),&full,Phase::Open);
        let broken=vec![[0,0,0,16];480*160];
        p.sync(1.3,Some(n),&broken,Phase::Exiting);assert_eq!(p.pixels(),full);
        assert!(p.drawing(1.36).is_some());assert!(!p.input());assert!(p.drawing(1.46).is_none());
    }
    #[test]fn slides_from_own_edge_without_resizing_or_changing_clip(){
        for right in [false,true] {
            let mut p=Panel::default();let mut n=node(right);n.role=Role::Stats;p.sync(0.,Some(n),&image(),Phase::Open);
            let base=Layout{rect:[-120.,0.,480.,160.],clip:[0.,0.,240.,160.]};
            let first=p.layout(base,0.);let middle=p.layout(base,0.075);let last=p.layout(base,0.15);
            assert_eq!(&first.rect[2..],&base.rect[2..]);assert!(first.clip[2]>=0. && first.clip[3]>=0.);assert_eq!(last.rect,base.rect);
            assert!(if right{first.rect[0]>middle.rect[0] && middle.rect[0]>last.rect[0]}else{first.rect[0]<middle.rect[0] && middle.rect[0]<last.rect[0]});
            let index=middle.source(100.,80.).unwrap();
            let reconstructed=middle.rect[0]+(index%480)as f32;
            assert!((reconstructed-100.).abs()<1.01);assert_eq!(index/480,80);
        }
    }
    #[test]fn extended_panel_starts_entirely_beyond_its_edge_and_empty_canvas_exits(){
        let mut n=node(false);n.bounds=[0,64,160,152];n.length=true;
        let mut p=Panel::default();p.length=2.;p.sync(0.,Some(n),&image(),Phase::Open);
        let base=Layout{rect:[-120.,0.,480.,160.],clip:[0.,0.,568.,160.]};
        let start=p.layout(base,0.);
        assert!(start.rect[0]+(320.+120.)<=0.);
        p.sync(0.2,Some(n),&image(),Phase::Open);
        p.sync(0.3,Some(n),&[],Phase::Open);
        assert!(p.drawing(0.36).is_some());assert!(p.drawing(0.46).is_none());
    }
    #[test]fn disappearance_uses_last_frame_and_attack_labels_disappear_immediately(){
        let mut p=Panel::default();p.sync(0.,Some(node(true)),&image(),Phase::Open);
        p.sync(0.2,None,&[],Phase::Hidden);assert!(p.drawing(0.25).is_some());
        assert!(p.drawing(0.36).is_none());
        let mut n=node(true);n.role=Role::AttackName;
        p.sync(1.,Some(n),&image(),Phase::Open);p.sync(1.2,None,&[],Phase::Hidden);assert!(p.drawing(1.2).is_none());
    }
}


/// Independent battle HUD transitions, with frozen complete pixels on exit.
pub struct BattleHud {panels:[Panel;4],clock:Instant,stage:Option<u16>}
impl Default for BattleHud {fn default()->Self{Self{panels:Default::default(),clock:Instant::now(),stage:None}}}
impl BattleHud {
    pub fn sync(&mut self,mem:&gba_core::MemMap,pixels:&[Vec<[u8;4]>;4]) {
        let stage=gba_core::ffta_surface::Descriptor::read(mem).map(|s|s.id);
        if stage!=self.stage {for p in &mut self.panels{p.clear();}self.stage=stage;}
        // Deployment confirmation owns the screen; do not leave a retained
        // miniature crossing over Begin Battle while its background retracts.
        if mem.ffta_dialogue.battle_notice() {for p in &mut self.panels{p.clear();}return;}
        let now=self.clock.elapsed().as_secs_f64();
        for i in 0..4 {
            let (anchor,bounds)=match i {
                0=>(Anchor::TopCenter,[0,8,240,104]),
                1=>(Anchor::Corner{right:true,bottom:true},[168,72,240,160]),
                2=>(Anchor::TopCenter,[64,16,176,24]),
                _=>(Anchor::Corner{right:true,bottom:true},[179,128,235,144]),
            };
            let node=(!pixels[i].is_empty()).then_some(Node{slot:i,role:Role::ServiceMenu,anchor,bounds,length:false});
            self.panels[i].sync(now,node,&pixels[i],Phase::Open);
        }
    }
    pub fn pixels(&self,i:usize)->&[[u8;4]]{self.panels[i].pixels()}
    pub fn layout(&self,i:usize,base:Layout)->Option<Layout>{
        let now=self.clock.elapsed().as_secs_f64();
        self.panels[i].drawing(now).map(|_|self.panels[i].layout(base,now))
    }
}

#[cfg(test)]
mod battle_hud_transition_tests {
    use super::*;
    #[test]
    fn preview_retracts_cached_canvas_and_replacement_does_not_reenter() {
        let n=Node{slot:1,role:Role::ServiceMenu,anchor:Anchor::Corner{right:true,bottom:true},bounds:[168,72,240,160],length:false};
        let pixels=vec![[1,0,16,0];480*160];let mut p=Panel::default();
        p.sync(0.,Some(n),&pixels,Phase::Open);
        assert_eq!(p.level(0.),0.);assert!(p.level(0.075)>0. && p.level(0.075)<1.);
        p.sync(0.2,Some(n),&pixels,Phase::Open);assert_eq!(p.level(0.2),1.);
        let replacement=vec![[2,0,16,0];480*160];p.sync(0.3,Some(n),&replacement,Phase::Open);
        assert_eq!(p.level(0.3),1.);assert_eq!(p.pixels(),replacement);
        p.sync(0.4,None,&[],Phase::Open);assert_eq!(p.pixels(),replacement);
        assert!(p.level(0.475)>0. && p.level(0.475)<1.);
        let base=Layout::centered((1280,720),284.,1.);
        assert!(p.layout(base,0.475).rect[0]>base.rect[0]);
        p.sync(0.6,None,&[],Phase::Open);assert!(p.drawing(0.6).is_none());
    }
    #[test]
    fn start_banner_uses_top_reveal_instead_of_spawning() {
        let n=Node{slot:2,role:Role::ServiceMenu,anchor:Anchor::TopCenter,bounds:[64,16,176,24],length:false};
        let mut p=Panel::default();p.sync(0.,Some(n),&vec![[1,0,16,0];480*160],Phase::Open);
        let base=Layout::centered((1280,720),284.,1.);
        assert!(p.layout(base,0.05).rect[1]<base.rect[1]);
        assert_eq!(p.layout(base,0.2).rect,base.rect);
    }
}
