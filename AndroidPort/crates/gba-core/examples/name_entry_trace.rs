//! Private-ROM keyboard transition trace; no user save writes.
fn main()->Result<(),Box<dyn std::error::Error>>{
 let a:Vec<_>=std::env::args().collect();
 let mut m=Box::new(gba_core::Machine::new(std::fs::read(&a[1])?));
 m.load_state(&std::fs::read(&a[2])?).map_err(std::io::Error::other)?;
 m.bus.ffta_dialogue.separate=true;
 println!("frame,task,state,page,active,dispcnt,bg0,bg1,x0,y0,x1,y1,oam0,oam1,tile");
 for f in 0..220 {
  m.bus.keys=match f{10..=15=>0x3fd,80..=85=>0x3f7,120..=125=>0x3df,135..=140=>0x3fe,_=>0x3ff};
  m.run_frame(5_000_000);m.bus.audio_buf.clear();
  let h=|v:&[u8],o|u16::from_le_bytes([v[o],v[o+1]]);
  let ui=m.bus.oam.chunks_exact(8).filter_map(|o|{let a=h(o,0);let b=h(o,2);let c=h(o,4);if a&0x300!=0x200 && matches!(c&1023,0x2d0|0x31c|0x324|0x32c|0x330){Some(format!("{:04x}/{:04x}/{:04x}",a,b,c))}else{None}}).collect::<Vec<_>>();
  eprintln!("DETAIL f={f} bg2={},{} bg3={},{} win={:04x}/{:04x} objs={}",h(&m.bus.io,24),h(&m.bus.io,26),h(&m.bus.io,28),h(&m.bus.io,30),h(&m.bus.io,0x40),h(&m.bus.io,0x44),ui.join(";"));
  println!("{f},{},{},{},{},{:04x},{:04x},{:04x},{},{},{},{},{:04x},{:04x},{:04x}",m.bus.iwram[0x27d1],m.bus.ewram[0x166db],m.bus.ewram[0x166da],gba_core::ffta_mouse::name_entry_active(&m.bus),h(&m.bus.io,0),h(&m.bus.io,8),h(&m.bus.io,10),h(&m.bus.io,16),h(&m.bus.io,18),h(&m.bus.io,20),h(&m.bus.io,22),h(&m.bus.oam,0),h(&m.bus.oam,2),h(&m.bus.oam,4));
 }
 Ok(())
}
