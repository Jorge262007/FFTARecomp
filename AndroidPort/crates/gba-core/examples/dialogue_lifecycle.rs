//! Trace semantic dialogue lifetime from a clean boot using private user input.
fn main()->Result<(),Box<dyn std::error::Error>>{
 let a:Vec<_>=std::env::args().collect();
 let mut m=Box::new(gba_core::Machine::new(std::fs::read(&a[1])?));
 m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;
 gba_core::mem::set_extra(22);gba_core::mem::set_extra_y(0);
 let input=std::fs::read_to_string(&a[2])?;
 let events=input.lines().skip(1).filter_map(|s|{let mut p=s.split_whitespace();Some((p.next()?.parse::<usize>().ok()?,u16::from_str_radix(p.next()?,16).ok()?))}).collect::<Vec<_>>();
 let mut cursor=0;let mut keys=0x3ff;let mut last=String::new();
 for f in 0..a.get(3).and_then(|s|s.parse().ok()).unwrap_or(6000){
  while cursor<events.len() && events[cursor].0<=f {keys=events[cursor].1;cursor+=1;}
  m.bus.keys=keys;m.run_frame(5_000_000);m.bus.audio_buf.clear();
  let scene=gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue);
  let desc=scene.nodes[0].map(|n|format!("{:?} {:?} {:?} phase={:?} ink={:?}",n.role,n.anchor,n.bounds,gba_core::ffta_ui_window::presentation(&m.bus,n,None),gba_core::ffta_dialogue::pixel_bounds(&m.bus.ffta_dialogue.pixels))).unwrap_or("none".into());
  if desc!=last {
   println!("f={f} stage={:?} {desc}",gba_core::ffta_surface::Descriptor::read(&m.bus).map(|s|s.id));
   if f>1000 && desc.contains("AttackName") && last.contains("Dialogue"){
    std::fs::write(format!("AndroidPort/target/dialogue-closing-{f}.state"),m.save_state())?;
   }
   last=desc;
  }
 }
 Ok(())
}
