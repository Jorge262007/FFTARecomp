//! Private local RE harness. Reads a supplied save copy; never writes saves.
use gba_core::{Machine,Cpu,Bus};
use std::{fs,io::Write};
fn call(m:&mut Machine,pc:u32,a:u32,b:u32) {
 let mut c=Cpu::new(); c.cpsr|=0x20|0x80;c.regs[0]=a;c.regs[1]=b;
 c.regs[13]=0x0203f000;c.regs[14]=0x01000001;c.regs[15]=pc;
 let saved=std::mem::replace(&mut m.cpu,c);
 for n in 0..2_000_000 {
  if m.cpu.regs[15]==0x01000000 {println!("CALL pc={pc:08x} steps={n} r0={:08x}",m.cpu.regs[0]);m.cpu=saved;return}
  m.step();
 }
 panic!("call timed out {:08x}",m.cpu.regs[15]);
}
fn main(){
 let a:Vec<_>=std::env::args().collect();let rom=fs::read(&a[1]).unwrap();
 assert_eq!(gba_core::ffta_rom::version(&rom),Some(gba_core::ffta_rom::Version::Europe));
 let mut m=Machine::new(rom);
 let mission:usize=a[4].parse().unwrap();assert!(mission<406);
 let mut mouse=gba_core::ffta_mouse::LanguageMouse::new(&m.bus.rom,true);
 let script=fs::read_to_string(&a[5]).unwrap();
 let events:Vec<_>=script.lines().skip(1).filter_map(|s|{let mut p=s.split_whitespace();Some((p.next()?.parse::<u64>().ok()?,p.next()?.to_owned()))}).collect();
 let mut ev=0;let mut keys=0x3ff;
 for frame in 0..27000 {
  // Title -> Continue. Do not issue buttons after the probe.
  while ev<events.len() && events[ev].0==frame {
   if let Some(n)=events[ev].1.strip_prefix("mouse:"){mouse.select(&mut m.bus,n.parse().unwrap());}
   else {keys=u16::from_str_radix(&events[ev].1,16).unwrap();}ev+=1;
  }
  m.bus.keys=keys;
  if frame>=24800 {m.bus.keys=if frame%60<3 {0x3fe}else{0x3ff};}
  gba_core::ffta_battle_mouse::replay_keys(&mut m.bus);
  if frame==23000 {println!("WIN {:?}",gba_core::ffta_debug::request_victory(&mut m.bus));}
  gba_core::ffta_debug::tick(&mut m.bus);
  if frame==24800 {
   fs::write(format!("{}-before.ewram",a[3]),&m.bus.ewram).unwrap();
   fs::write(format!("{}-before.iwram",a[3]),&m.bus.iwram).unwrap();
   // Only a private harness: build temporary units outside the clan and dump
   // their statistics. Do not publish them or alter a save file.
   for kind in 3..=14u8 {
    m.bus.ewram[0x3fe00..0x3fe30].fill(0);
    m.bus.ewram[0x3fe00]=kind;
    call(&mut m,0x080ccf9c,0x0203fc00,0x0203fe00);
    let p=0x3fc00;
    println!("UNIT kind={kind} name={:08x} type={} job={} level={} hp={} mp={}",u32::from_le_bytes(m.bus.ewram[p..p+4].try_into().unwrap()),m.bus.ewram[p+4],m.bus.ewram[p+7],m.bus.ewram[p+9],u16::from_le_bytes(m.bus.ewram[p+26..p+28].try_into().unwrap()),u16::from_le_bytes(m.bus.ewram[p+30..p+32].try_into().unwrap()));
   }
   let location=m.bus.rom[0x713b3c+70*mission+0x45];let desc=0x0871c760+12*u32::from(location);
   let selector=m.bus.read8(desc);let event=m.bus.read16(desc+2);let alternate=m.bus.read16(desc+4);
   println!("MISSION id={mission} location={location} descriptor={desc:08x} selector={selector} event={event} alternate={alternate}");
   m.bus.write16(0x02003c28,mission as u16);m.bus.write8(0x02002191,location);
   call(&mut m,0x08009d04,desc,selector as u32);
   let e=m.bus.read16(0x02002170);call(&mut m,0x08009be8,e as u32,0);
   call(&mut m,0x0800a0bc,0,0);
   call(&mut m,0x0800394c,2,0);
  }
  if frame%1000==0 || frame==24800 {let scene=m.bus.iwram[0x20];println!("FRAME {frame} scene={scene} event={} next={} battle={:08x}",m.bus.read16(0x02002192),m.bus.read16(0x02002194),m.bus.read32(0x03002804));}
  m.run_frame(5_000_000);m.bus.audio_buf.clear();
 }
 fs::write(format!("{}-after.ewram",a[3]),&m.bus.ewram).unwrap();
 fs::write(format!("{}-after.iwram",a[3]),&m.bus.iwram).unwrap();
 let mut f=fs::File::create(format!("{}-after.ppm",a[3])).unwrap();write!(f,"P6\n240 160\n255\n").unwrap();
 for p in &m.bus.framebuffer {f.write_all(&[((p&31)*255/31)as u8,(((p>>5)&31)*255/31)as u8,(((p>>10)&31)*255/31)as u8]).unwrap();}
}
