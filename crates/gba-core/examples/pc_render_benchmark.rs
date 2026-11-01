//! Deterministic maximum-zoom PPU benchmark from a private state, no save writes.
fn hash(h:&mut u64,bytes:impl IntoIterator<Item=u8>){for b in bytes{*h=(*h^b as u64).wrapping_mul(1099511628211);}}
fn main()->Result<(),Box<dyn std::error::Error>>{
 let a:Vec<_>=std::env::args().collect();
 let rom=std::fs::read(&a[1])?;let state=std::fs::read(&a[2])?;
 for ex in [164,448] {
  let mut m=Box::new(gba_core::Machine::new(rom.clone()));m.load_state(&state).map_err(std::io::Error::other)?;
  gba_core::mem::set_extra(ex);gba_core::mem::set_extra_y(80);gba_core::ppu::set_parallel_zoom(true);
  m.bus.ffta_dialogue.separate=true;m.bus.ffta_dialogue.battle_hud_overlay=true;
  let initial:[u16;2]=std::array::from_fn(|i|u16::from_le_bytes([m.bus.io[16+i*4],m.bus.io[17+i*4]]));
  let mut times=Vec::new();let mut checksum=14695981039346656037u64;
  for f in 0..96 {
   for bg in 0..2{m.bus.io[16+bg*4..18+bg*4].copy_from_slice(&((initial[bg]+f*4)&511).to_le_bytes());}
   m.bus.frames+=1;
   let t=std::time::Instant::now();
   for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
   gba_core::ppu::render_ffta_zoom_rows(&mut m.bus);
   if f>=16{times.push(t.elapsed().as_secs_f64()*1000.);}
   hash(&mut checksum,m.bus.wide_framebuffer[..gba_core::mem::render_width()*320].iter().flat_map(|v|v.to_le_bytes()));
   hash(&mut checksum,m.bus.framebuffer.iter().flat_map(|v|v.to_le_bytes()));
   for panel in [&m.bus.ffta_dialogue.pixels,&m.bus.ffta_dialogue.extra.pixels]{
    hash(&mut checksum,panel.iter().flat_map(|p|[p.color as u8,(p.color>>8)as u8,p.eva,p.evb]));
   }
  }
  let mean=times.iter().sum::<f64>()/times.len()as f64;times.sort_by(f64::total_cmp);
  println!("PPU_BENCH width={} height=320 frames={} stage={:?} mean_ms={mean:.3} p95_ms={:.3} checksum={checksum:016x}",240+ex*2,times.len(),gba_core::ffta_surface::Descriptor::read(&m.bus).map(|s|s.id),times[times.len()*95/100]);
 }
 Ok(())
}
