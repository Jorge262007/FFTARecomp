//! Local regression against a user-provided RAM snapshot; no game data included.
use gba_core::Machine;
fn main(){
 let mut m=Machine::new(std::fs::read(std::env::args().nth(1).expect("ROM path")).unwrap());
 for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("target/battle-current-{n}.bin")).unwrap());}
 m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;
 gba_core::mem::set_extra(164);gba_core::mem::set_extra_y(0);
 let original=(m.bus.ewram.clone(),m.bus.vram.clone(),m.bus.oam.clone());let mut expected=None;
 for clip in [0x0102u16,0x10e0,0x80c0,0xe0f0,0x0000] {
  for(o,v)in[(0,0x3f40u16),(0x40,clip),(0x44,0x0108),(0x48,31),(0x4a,32)]{m.bus.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
  for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
  assert!(m.bus.ffta_dialogue.action_layout);
  let ui=gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue);
  assert_eq!(ui.nodes[0].unwrap().bounds,[0,48,160,160]);
  let pixels=(m.bus.ffta_dialogue.pixels.clone(),m.bus.ffta_dialogue.extra.pixels.clone());
  assert!(pixels.0.iter().any(|p|p.visible()));assert!(pixels.1.iter().any(|p|p.visible()));
  if let Some(ref old)=expected{assert_eq!(&pixels,old,"old WIN clipping must not resize/reveal a second copy of either host panel");}else{expected=Some(pixels);}
  assert_eq!((&m.bus.ewram,&m.bus.vram,&m.bus.oam),(&original.0,&original.1,&original.2));
  println!("BATTLE_CLIP window={clip:04x} identical_host_pixels=true guest_memory_unchanged=true");
 }
}
