//! Research on copied RAM only. No save files are opened or written.
use gba_core::{Machine,Cpu};
use std::fs;
fn main(){
 let a:Vec<_>=std::env::args().collect();let rom=fs::read(&a[1]).unwrap();
 assert_eq!(gba_core::ffta_rom::version(&rom),Some(gba_core::ffta_rom::Version::Europe));
 let ew=fs::read(format!("{}.ewram",a[2])).unwrap();let iw=fs::read(format!("{}.iwram",a[2])).unwrap();
 let mut offers=std::collections::BTreeMap::new();
 for id in 0..406u16 {let code=rom[0x713b3c+70*id as usize+53];if code!=0 {offers.entry(code).or_insert(id);}}
 for (code,id) in [(0x96,384),(0x98,385),(0x9a,386),(0x9e,387),(0x9c,400)] {offers.insert(code,id);}
 let offers:Vec<_>=offers.into_iter().collect();
 for kind in 2..15+offers.len() as u8 {
  let mut m=Machine::new(rom.clone());m.bus.ewram.copy_from_slice(&ew);m.bus.iwram.copy_from_slice(&iw);
  m.bus.ewram[0x3fe00..0x3fe30].fill(0);m.bus.ewram[0x3fe00]=kind;
  m.cpu=Cpu::new();m.cpu.cpsr|=0xa0;m.cpu.regs[0]=0x0203fc00;m.cpu.regs[1]=0x0203fe00;
  m.cpu.regs[13]=0x0203f000;m.cpu.regs[14]=0x01000001;m.cpu.regs[15]=0x080ccf9c;
  if kind>=15 {m.cpu.regs[0]=u32::from(offers[(kind-15)as usize].1);m.cpu.regs[1]=0x0203fc00;m.cpu.regs[15]=0x080d5f3c;}
  let mut steps=0;
  while m.cpu.regs[15]!=0x01000000 && steps<2_000_000 {m.step();steps+=1;}
  assert!(steps<2_000_000,"constructor timed out");
  let u=&m.bus.ewram[0x3fc00..0x3fd08];let hp=u16::from_le_bytes(u[26..28].try_into().unwrap());
  let name=u32::from_le_bytes(u[..4].try_into().unwrap());
  println!("UNIT type={kind:02x} stored_type={:02x} job={:02x} level={} hp={hp} name={name:08x} steps={steps}",u[4],u[7],u[9]);
  if kind>=15 {
   let (code,mission)=offers[(kind-15)as usize];
   println!("OFFER code={code:02x} mission={mission} result={}",m.cpu.regs[0]);
   if m.cpu.regs[0]==0 {assert_eq!(hp,0);continue;}
  }
  assert!(hp>0 && ((0x08000000..0x09000000).contains(&name) || name==0x02001f1c));
 }
}
