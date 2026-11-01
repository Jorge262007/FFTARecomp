use gba_core::{Machine,ffta_battle_mouse as mouse};
fn main(){
 let rom=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();let mut m=Machine::new(rom);
 let p=std::env::args().nth(2).unwrap_or("target/map-feather-normal-3480".into());
 for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("{p}-{n}.bin")).unwrap());}
 assert!(m.bus.ffta_battle_mouse.enable_replay());
 for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 let before=m.bus.framebuffer.clone();let oam=m.bus.oam.clone();
 mouse::set_mouse_visual(&mut m.bus,true);
 for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
 let mut changes=0;
 for(i,(a,b))in before.iter().zip(&m.bus.framebuffer).enumerate(){if a!=b{changes+=1;assert!((158..174).contains(&(i%240))&&(89..105).contains(&(i/240)),"changed outside feather at {i}");}}
 assert!(changes>0);assert_eq!(oam,m.bus.oam);
 let mut ppm=b"P6\n240 160\n255\n".to_vec();for c in &m.bus.framebuffer{for shift in [0,5,10]{ppm.push((((c>>shift)&31)*255/31)as u8);}}
 std::fs::write("target/map-feather-hidden-proof.ppm",ppm).unwrap();
 println!("MAP_FEATHER changed_pixels={changes} only_feather_bounds=true guest_oam_unchanged=true");
}
