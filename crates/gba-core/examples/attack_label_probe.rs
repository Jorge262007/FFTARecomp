use gba_core::{Machine,ffta_battle_mouse as mouse};
fn main(){
 let prefix=std::env::args().nth(2).unwrap(); let mut m=Machine::new(std::fs::read(std::env::args().nth(1).unwrap()).unwrap());
 for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("{prefix}-{n}.bin")).unwrap());}
 m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;m.bus.ffta_battle_mouse.enable_replay();gba_core::mem::set_extra(164);
 for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 if std::env::args().any(|a|a=="--abilities-proof") {
  let list=mouse::battle_list(&m.bus).unwrap();assert_eq!((list.total,list.visible,list.first),(8,6,1));
  let base=list.cursor-0x20;
  let source=u32::from_le_bytes(m.bus.ewram[base+12..base+16].try_into().unwrap())as usize&0x3ffff;
  let width=m.bus.ewram[base+0x15]as usize;
  for target in [0u8,2,5,1] {
   assert!(mouse::menu_select(&mut m.bus,target,false));
   for row in 0..list.total as usize {for col in 0..width*2 {
    let at=source+(row*width*2+col)*2;
    let tile=u16::from_le_bytes(m.bus.ewram[at..at+2].try_into().unwrap());
    if matches!(tile>>12,11|12){assert_eq!(tile>>12,if row==list.first as usize+target as usize{11}else{12});}
   }}
  }
  assert!(mouse::pub_scroll_to(&mut m.bus,2));
  for row in 0..list.total as usize {for col in 0..width*2 {
   let at=source+(row*width*2+col)*2;
   let tile=u16::from_le_bytes(m.bus.ewram[at..at+2].try_into().unwrap());
   if matches!(tile>>12,11|12){assert_eq!(tile>>12,12,"scroll must leave every row unselected");}
  }}
  assert!(mouse::pub_scroll_unselected(&m.bus));
  assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x37f);
  println!("ABILITIES_PROOF total=8 visible=6 first=1 hover=single_highlight scroll=no_highlights paging=Down");
  for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 }
 let snapshot_scene=gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue);
 if let Some(n)=snapshot_scene.nodes[1] {println!("PHASE_PROOF {:?}",gba_core::ffta_ui_window::presentation(&m.bus,n,Some(0x2dd84)));}
 let s=&m.bus.ffta_dialogue; println!("ALPHA_PROOF main_translucent={} extra_translucent={}",s.pixels.iter().filter(|p|p.visible() && p.evb>0).count(),s.extra.pixels.iter().filter(|p|p.visible() && p.evb>0).count());
 println!("UI {:?} main {:?} extra {:?} native {:?} IO {:02x?}",gba_core::ffta_ui::Scene::capture(s),gba_core::ffta_dialogue::pixel_bounds(&s.pixels),gba_core::ffta_dialogue::pixel_bounds(&s.extra.pixels),s.native_bounds,&m.bus.io[..0x56]);
 for (name,pixels) in [("main",&s.pixels),("extra",&s.extra.pixels)] {let mut out=b"P6\n480 160\n255\n".to_vec();for p in pixels.iter().take(480*160){let v=if p.visible(){p.color}else{0x4210};for c in [v&31,(v>>5)&31,(v>>10)&31]{out.push((c*255/31)as u8);}}std::fs::write(format!("{prefix}-{name}.ppm"),out).unwrap();}
 for(name,width,pixels)in[("world",568,&m.bus.wide_framebuffer),("native",240,&m.bus.framebuffer)]{let mut out=format!("P6\n{width} 160\n255\n").into_bytes();for p in pixels.iter().take(width*160){for c in [*p&31,(*p>>5)&31,(*p>>10)&31]{out.push((c*255/31)as u8);}}std::fs::write(format!("{prefix}-{name}.ppm"),out).unwrap();}
}
