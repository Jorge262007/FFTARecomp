use gba_core::{Machine,ffta_battle_mouse as mouse};
fn main(){
 let mut m=Machine::new(std::fs::read(std::env::args().nth(1).unwrap()).unwrap());
 for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("target/urgent-event-{n}.bin")).unwrap());}
 m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;m.bus.ffta_battle_mouse.enable_replay();gba_core::mem::set_extra(164);
 for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 println!("MENU {:?} UI {:?}",mouse::MenuScene::read(&m.bus),gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue));
 assert_eq!(mouse::map_award_bounds(&m.bus),Some([0,48,104,104]));
 assert!(!mouse::aim(&mut m.bus,130.,88.,true));
 assert!(mouse::aim(&mut m.bus,24.,88.,true));
 assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x3fe,"OK generates one confirmation");
 let native=m.bus.framebuffer.clone();
 gba_core::mem::set_extra(0);for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 assert_eq!(native,m.bus.framebuffer,"widescreen fix leaves original native composition intact");
 gba_core::mem::set_extra(164);for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 println!("AWARD_PROOF native_preserved=true ok_click=true outside_rejected=true");
 for o in (0..m.bus.ewram.len()-32).step_by(4){let p=u32::from_le_bytes(m.bus.ewram[o..o+4].try_into().unwrap());if [0x08018fc5,0x080397ed].contains(&p){println!("CURSOR {o:x} {:02x?}",&m.bus.ewram[o.saturating_sub(32)..o+32]);}}
 println!("IO {:02x?}",&m.bus.io[..96]);println!("IWRAM {:02x?}",&m.bus.iwram[0x2800..0x2820]);
 for i in 0..8 {let o=0x9e0+i*0x4c;println!("WINDOW {i} {:02x?}",&m.bus.iwram[o..o+0x4c]);}
 for(name,width,pixels)in[("fixed",568,&m.bus.wide_framebuffer),("native",240,&m.bus.framebuffer)]{let mut out=format!("P6\n{width} 160\n255\n").into_bytes();for p in pixels.iter().take(width*160){for c in [*p&31,(*p>>5)&31,(*p>>10)&31]{out.push((c*255/31)as u8);}}std::fs::write(format!("target/urgent-event-{name}.ppm"),out).unwrap();}
}
