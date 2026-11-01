//! Shared semantic click targets, with Android screen-space UI composition.
use gba_core::{ffta_battle_mouse as battle, ffta_dialogue::{self as dialogue,PanelKind}, mem::{self,MemMap}};
use crate::{engine::Engine,ui_layout::Layout};

pub fn layout(bus:&MemMap,width:usize,height:usize,base:f32,index:usize)->Option<Layout>{
    let state=&bus.ffta_dialogue;
    let node=gba_core::ffta_ui::Scene::capture(state).nodes.get(index).copied().flatten()?;
    Some(crate::host_layout::host_ui_layout((width as u32,height as u32),base,state.style,node))
}

#[cfg(test)]
mod tests {
    use super::battle;
    use crate::ui_layout::Layout;
    #[test]
    fn native_panel_keeps_uniform_screen_scale_at_every_world_zoom(){
        for screen in [(720,720),(1920,1080),(2560,720)] {
            for base in [240.,284.,568.] {
                for size in [0.5,1.,1.5,2.] {
                    let panel=Layout::new_with_anchor(screen,base,true,size,[0.,0.],160.);
                    assert!((panel.rect[2]/480.-panel.rect[3]/160.).abs()<0.0001);
                    for _zoom in 0..=6 {
                        // World texture dimensions are deliberately absent from UI layout.
                        let at_zoom=Layout::new_with_anchor(screen,base,true,size,[0.,0.],160.);
                        assert_eq!(panel.rect,at_zoom.rect);
                    }
                    let bottom=panel.rect[1]+panel.rect[3];
                    assert!(bottom<=screen.1 as f32+0.51);
                }
            }
        }
    }
    #[test]
    fn dialogue_answers_select_the_touched_option_and_ignore_empty_space(){
        let mut engine=crate::engine::Engine::new(vec![0;0x13224]);
        engine.touch_enabled=true;
        let bus=&mut engine.machine.bus;
        bus.ffta_battle_mouse.enabled=true;
        bus.rom[0x11b66..0x11b70].copy_from_slice(&[9,0x4a,0x51,0x88,1,0x20,8,0x40,0,0x28]);
        bus.rom[0x13214..0x13224].copy_from_slice(&[0x0a,0x78,2,0x31,9,0x78,0x0b,0x31,0x51,0x43,0xa2,0x8e,0x89,0x18,0xe2,0x8e]);
        let base=0x24000;
        bus.ewram[0x918c..0x9190].copy_from_slice(&0x02024000u32.to_le_bytes());
        bus.ewram[base+8]=9;
        bus.ewram[base+0x34..base+0x36].copy_from_slice(&40u16.to_le_bytes());
        bus.ewram[base+0x36..base+0x38].copy_from_slice(&48u16.to_le_bytes());
        bus.ewram[base+0x43]=25;
        bus.keys=0x3ff;
        engine.tap(77./240.,50./160.,false);
        assert_eq!(battle::DialogueChoice::read(&engine.machine.bus).unwrap().selected,1);
        engine.tap(5./240.,5./160.,false);
        assert_eq!(battle::DialogueChoice::read(&engine.machine.bus).unwrap().selected,1);
        engine.tap(41./240.,50./160.,false);
        assert_eq!(battle::DialogueChoice::read(&engine.machine.bus).unwrap().selected,0);
        assert_eq!(engine.machine.bus.keys,0x3ff,"answering must not leak A into the menu behind the dialogue");
    }
    #[test]
    fn disabled_touch_never_changes_guest_buttons(){
        let mut engine=crate::engine::Engine::new(vec![0;192]);
        let keys=engine.machine.bus.keys;
        engine.tap(0.5,0.5,false);
        engine.tap(f32::NAN,0.5,false);
        engine.tap(0.5,0.5,true);
        assert_eq!(engine.machine.bus.keys,keys);
        assert!(!engine.machine.bus.ffta_battle_mouse.enabled);
    }
}
impl Engine {
    pub fn configure_touch(&mut self,enabled:bool,base:f32,size:f32,length:f32){
        // Name entry is a keyboard, independent of experimental stage tapping.
        let enabled=enabled || gba_core::ffta_mouse::name_entry_active(&self.machine.bus);
        if enabled!=self.touch_enabled {
            self.touch_enabled=enabled;
            self.mouse=gba_core::ffta_mouse::LanguageMouse::new(&self.machine.bus.rom,enabled);
            self.machine.bus.ffta_battle_mouse.set_enabled(enabled);
        }
        // Square/portrait devices still need the complete 240-pixel native UI.
        self.ui_base=if base.is_finite(){base.clamp(240.,1536.)}else{240.};
        self.machine.bus.ffta_dialogue.separate=true;
        self.machine.bus.ffta_dialogue.style.scale=if size.is_finite(){size.clamp(0.5,2.)}else{1.};
        self.machine.bus.ffta_dialogue.style.length=if length.is_finite(){length.clamp(1.,2.)}else{1.};
    }
    pub fn tap(&mut self,x:f32,y:f32,back:bool){
        if self.save_tap(x,y,back){return;}
        if !self.touch_enabled || !x.is_finite() || !y.is_finite() || !(0. ..1.).contains(&x) || !(0. ..1.).contains(&y){return;}
        if back {self.mouse.cancel();battle::back(&mut self.machine.bus);return;}
        let width=self.ui_window.0 as usize;let height=self.ui_window.1 as usize;
        let px=x*width as f32;let py=y*height as f32;
        let state=&self.machine.bus.ffta_dialogue;
        if state.naming_layout {
            if !self.ui_motion.panels[0].input(){return;}
            if let Some(source)=layout(&self.machine.bus,width,height,self.ui_base,0)
                .map(|l|self.ui_motion.layout(0,l)).and_then(|l|l.source(px,py)){
                self.mouse.aim(&self.machine.bus,(source%480)as f32-120.,(source/480)as f32,true);
            }
            return;
        }
        // Questions own input before the inactive battle/service menu behind them.
        // Use the rendered host panel, not the zoomed world, for answer bounds.
        if let Some(choice)=battle::DialogueChoice::read(&self.machine.bus){
            let scene=gba_core::ffta_ui::Scene::capture(state);
            let panel=scene.nodes.iter().enumerate().find_map(|(i,node)|
                node.filter(|n|matches!(n.role,gba_core::ffta_ui::Role::Dialogue|gba_core::ffta_ui::Role::Notice)).map(|_|i));
            let point=if let Some(i)=panel {
                layout(&self.machine.bus,width,height,self.ui_base,i)
                    .map(|l|self.ui_motion.layout(i,l)).and_then(|l|l.source(px,py))
                    .map(|source|((source%480)as f32-120.,(source/480)as f32))
            }else{
                Some(((px-self.world_rect[0])/self.world_rect[2]*mem::render_width()as f32-mem::extra()as f32,
                      (py-self.world_rect[1])/self.world_rect[3]*mem::render_height()as f32-mem::extra_y()as f32))
            };
            if let Some(target)=point.and_then(|(x,y)|choice.hit(x,y)){
                self.mouse.cancel();choice.select(&mut self.machine.bus,target,true);
            }
            return; // Empty space must never answer the currently highlighted choice.
        }
        let index=if matches!(state.extra.kind,PanelKind::Actions|PanelKind::Notice)&&state.extra.active(){Some(1)}
            else if state.action_layout||state.town_layout||state.shop_layout {Some(0)}else{None};
        let mut handled=false;
        if (battle::MenuScene::read(&self.machine.bus).is_some()||state.shop_layout)&&index.is_some(){
            let i=index.unwrap();
            if let Some(source)=layout(&self.machine.bus,width,height,self.ui_base,i).and_then(|l|l.source(px,py)){
                let mut nx=(source%480) as f32-120.;let ny=(source/480) as f32;
                if !(state.town_layout||state.shop_layout||state.site_layout||state.extra.kind==PanelKind::Notice){
                    let bounds=if i==0{dialogue::panel_bounds(&state.pixels,&state.backing)}else{dialogue::panel_bounds(&state.extra.pixels,&state.extra.backing)};
                    if let Some(b)=bounds{
                        let d=((b[2]-b[0]) as f32*(state.style.length.clamp(1.,2.)-1.)).round();
                        nx+=if b[0]+b[2]>=240{d}else{-d};
                    }
                }
                handled=battle::aim(&mut self.machine.bus,nx,ny,true);
            }
        }else{
            let nx=(px-self.world_rect[0])/self.world_rect[2]*mem::render_width() as f32-mem::extra() as f32;
            let ny=(py-self.world_rect[1])/self.world_rect[3]*mem::render_height() as f32-mem::extra_y() as f32;
            handled=if self.mouse.scene(&self.machine.bus).is_some(){self.mouse.aim(&self.machine.bus,nx,ny,true)}
                else if battle::facing_center(&self.machine.bus).is_some(){battle::aim_facing(&mut self.machine.bus,nx,ny,true)}
                else{battle::aim(&mut self.machine.bus,nx,ny,true)};
        }
        if !handled {self.mouse.cancel();battle::press(&mut self.machine.bus,0x3fe);}
        #[cfg(target_os="android")]
        eprintln!("FFTA_TOUCH click x={x:.3} y={y:.3} handled={handled}");
    }
    pub fn export_ui(&self,packed:&mut [u8],params:&mut [f32],background:&mut [i32]){
        params.fill(0.);
        if let Some(s)=self.saves.as_ref().filter(|s|s.screen_active(&self.machine.bus)){
            if s.delete_confirm.is_none(){
                let (k,ox,oy)=crate::ui_layout::letterbox_f(self.ui_window,240.,160.);
                let entries=crate::save_states::card_entries(&self.machine.bus,s);
                for (n,id)in s.visible_ids(crate::save_states::is_saving(&self.machine.bus)).iter().enumerate(){
                    let Some(card)=entries.get(id).and_then(|c|c.as_ref()).filter(|c|c.image.len()==240*160)else{continue;};
                    let i=8+n;let bounds=crate::save_states::CARDS[n];
                    params[i*9..i*9+4].copy_from_slice(&[ox+(bounds[0]+4)as f32*k,oy+(bounds[1]+39)as f32*k,63.*k,42.*k]);
                    params[i*9+4..i*9+8].copy_from_slice(&[ox,oy,240.*k,160.*k]);params[i*9+8]=2.;
                    for y in 0..160 {for x in 0..480 {
                        let pixel=super::engine::rgba_colors()[(card.image[y*240+x/2]&0x7fff)as usize];
                        let at=(i*480*160+y*480+x)*4;
                        packed[at..at+4].copy_from_slice(&pixel.to_le_bytes());
                    }}
                }
            }
            return;
        }
        for i in 0..8 {
            let (layout,pixels)=if i<4 {
                let Some(node)=self.ui_motion.drawing(i)else{continue;};
                (self.ui_motion.layout(i,crate::host_layout::host_ui_layout(self.ui_window,self.ui_base,self.machine.bus.ffta_dialogue.style,node)),self.ui_motion.panels[i].pixels())
            }else{
                let Some(layout)=self.battle_hud_motion.layout(i-4,Layout::centered(self.ui_window,self.ui_base,1.))else{continue;};
                (layout,self.battle_hud_motion.pixels(i-4))
            };
            params[i*9..i*9+4].copy_from_slice(&layout.rect);
            params[i*9+4..i*9+8].copy_from_slice(&layout.clip);
            let stats=i==1&&self.machine.bus.ffta_dialogue.extra.kind==PanelKind::Stats;
            params[i*9+8]=if stats{1.}else{0.};
            let offset=i*480*160*4;
            for(target,pixel)in packed[offset..offset+480*160*4].chunks_exact_mut(4).zip(pixels){target.copy_from_slice(pixel);}
            if stats {for(out,&pixel)in background.iter_mut().zip(&self.machine.bus.ffta_dialogue.extra.background){*out=super::engine::rgba_colors()[(pixel&0x7fff)as usize];}}
        }
    }
}
