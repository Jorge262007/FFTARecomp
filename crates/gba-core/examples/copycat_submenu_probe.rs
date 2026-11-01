use gba_core::{Machine,ffta_battle_mouse as mouse};
fn main(){
 let mut m=Machine::new(std::fs::read(std::env::args().nth(1).unwrap()).unwrap());
 for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("target/copycat-submenu-{n}.bin")).unwrap());}
 m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;m.bus.ffta_battle_mouse.enable_replay();gba_core::mem::set_extra(164);
 for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 println!("MENU {:?} AWARD {:?}",mouse::MenuScene::read(&m.bus),mouse::map_award_bounds(&m.bus));
 let scene=mouse::MenuScene::read(&m.bus).expect("active CARDS child");
 assert_eq!((scene.count,scene.origin),(6,[136,56]));
 assert_eq!(scene.hit(240.,64.),None);
 assert!(!mouse::aim(&mut m.bus,24.,88.,true),"inactive SWAP parent cannot receive a click while CARDS is open");
 assert!(!mouse::aim(&mut m.bus,250.,64.,true),"no invisible hit area beyond the panel");
 for target in [5,0,3,1,4,2] {
  m.bus.ffta_battle_mouse.cancel();
  assert!(mouse::aim(&mut m.bus,160.,64.+16.*target as f32,false));
  assert_eq!(mouse::MenuScene::read(&m.bus).unwrap().selected,target);
  assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x3ff,"hover cannot emit a direction");
 }
 m.bus.ffta_battle_mouse.cancel();
 assert!(mouse::aim(&mut m.bus,160.,112.,true));
 m.bus.frames+=2;
 assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x3fe,"selected child row confirms once");
 m.bus.ffta_battle_mouse.cancel();
 assert!(mouse::pub_scroll_by(&mut m.bus,false));
 assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x37f,"wheel uses original page update");
 m.bus.ffta_battle_mouse.cancel();
 m.bus.ewram[0xff22..0xff24].copy_from_slice(&15u16.to_le_bytes());
 m.bus.ewram[0xff39]=0;m.bus.ewram[0xff36..0xff38].fill(0);
 assert_eq!(mouse::MenuScene::read(&m.bus).unwrap().count,5);
 assert!(mouse::aim(&mut m.bus,160.,128.,false));
 assert_eq!(mouse::MenuScene::read(&m.bus).unwrap().selected,4);
 assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x3ff,"later pages retain direct selection");
 println!("CARDS_PROOF direct_hover=true child_priority=true outside_rejected=true click=true wheel=true last_page=true");
 for i in 0..8 {let o=0x9e0+i*0x4c;println!("WINDOW {i} {:02x?}",&m.bus.iwram[o..o+0x4c]);}
 println!("LIST {:02x?}",&m.bus.ewram[0xfec0..0xff48]);
 for(name,width,pixels)in[("fixed",568,&m.bus.wide_framebuffer),("native",240,&m.bus.framebuffer)]{let mut out=format!("P6\n{width} 160\n255\n").into_bytes();for p in pixels.iter().take(width*160){for c in [*p&31,(*p>>5)&31,(*p>>10)&31]{out.push((c*255/31)as u8);}}std::fs::write(format!("target/copycat-submenu-{name}.ppm"),out).unwrap();}
}
