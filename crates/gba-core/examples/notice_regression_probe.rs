use gba_core::{Machine,ffta_battle_mouse as mouse};
fn main(){
 let mut m=Machine::new(std::fs::read(std::env::args().nth(1).unwrap()).unwrap());
 for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("target/notice-regression-{n}.bin")).unwrap());}
 m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;m.bus.ffta_battle_mouse.enable_replay();gba_core::mem::set_extra(164);
 for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 let s=&m.bus.ffta_dialogue;
 println!("UI {:?} main {:?} extra {:?} native {:?} IO {:02x?}",gba_core::ffta_ui::Scene::capture(s),gba_core::ffta_dialogue::pixel_bounds(&s.pixels),gba_core::ffta_dialogue::pixel_bounds(&s.extra.pixels),s.native_bounds,&m.bus.io[..0x56]);
 for (name,pixels) in [("main",&s.pixels),("extra",&s.extra.pixels)] {let mut out=b"P6\n480 160\n255\n".to_vec();for p in pixels.iter().take(480*160){let v=if p.visible(){p.color}else{0x4210};for c in [v&31,(v>>5)&31,(v>>10)&31]{out.push((c*255/31)as u8);}}std::fs::write(format!("target/notice-regression-{name}.ppm"),out).unwrap();}
 for(name,width,pixels)in[("world",568,&m.bus.wide_framebuffer),("native",240,&m.bus.framebuffer)]{let mut out=format!("P6\n{width} 160\n255\n").into_bytes();for p in pixels.iter().take(width*160){for c in [*p&31,(*p>>5)&31,(*p>>10)&31]{out.push((c*255/31)as u8);}}std::fs::write(format!("target/notice-regression-{name}.ppm"),out).unwrap();}
}
