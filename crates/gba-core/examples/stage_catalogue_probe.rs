//! Research-only original EUR stage loader. No save files opened or written.
use gba_core::{Machine,Cpu,Bus};
use std::{fs,io::Write};
fn call(m:&mut Machine,pc:u32,r0:u32,r1:u32)->Result<u32,String>{
 let mut cpu=Cpu::new();cpu.cpsr|=0xa0;cpu.regs[0]=r0;cpu.regs[1]=r1;
 cpu.regs[13]=0x0203f000;cpu.regs[14]=0x01000001;cpu.regs[15]=pc;m.cpu=cpu;
 for _ in 0..3_000_000 {if m.cpu.regs[15]==0x0801fe94{let sp=m.cpu.regs[13];m.bus.write32(sp+12,32768);}if m.cpu.regs[15]==0x01000000{return Ok(m.cpu.regs[0]);}m.step();}
 Err(format!("timeout at {pc:08x}: guest pc={:08x}",m.cpu.regs[15]))
}
fn main(){
 let a:Vec<_>=std::env::args().collect();let rom=fs::read(&a[1]).unwrap();
 assert_eq!(gba_core::ffta_rom::version(&rom),Some(gba_core::ffta_rom::Version::Europe));
 let ew=fs::read("target/native-combat.ewram").unwrap();let iw=fs::read("target/native-combat.iwram").unwrap();
 let first:u16=a[2].parse().unwrap();let last:u16=a[3].parse().unwrap();
 fs::create_dir_all("target/stage-catalogue").unwrap();
 for id in first..=last{
  let mut m=Machine::new(rom.clone());m.bus.ewram.copy_from_slice(&ew);m.bus.iwram.copy_from_slice(&iw);
  call(&mut m,0x0801b270,id as u32+1,0).unwrap();
  call(&mut m,0x0801b2cc,0,0).unwrap();
  let table=0x7662a8+88*id as usize;
  let offsets=[4usize,8];
  let mut failed=false;
  for (i,offset) in offsets.into_iter().enumerate(){
    let rel=u32::from_le_bytes(rom[table+offset..table+offset+4].try_into().unwrap());
    let pc=if i==0{0x08020318}else{0x0802045c};
    if let Err(e)=call(&mut m,pc,0x087662a8+rel,0){println!("ID {id} ERROR {e}");failed=true;break;}
  }
  if failed{continue;}
  println!("ID {id} ram={} load_state={} gfx_sha256={}",m.bus.read16(0x02007f10),m.bus.ewram[0x3e000],gba_core::ffta_rom::digest(&m.bus.vram[..0x4000]));
  fs::write(format!("target/stage-catalogue/{id:03}.vram"),&m.bus.vram).unwrap();
  fs::write(format!("target/stage-catalogue/{id:03}.ewram"),&m.bus.ewram).unwrap();
  m.bus.palette.copy_from_slice(&m.bus.iwram[0x3860..0x3c60]);
  // Reconstruct the original screen-block arrangement from calculated planes.
  for bg in 0..2u32{
   let base=call(&mut m,if bg==0{0x0801be8c}else{0x0801bea8},0,0).unwrap();
   for y in 0..32u32{for x in 0..64u32{
    let v=m.bus.read16(base+2*(y*64+x));
    let at=0x6000+bg as usize*0x1000+2*((x as usize/32)*1024+y as usize*32+x as usize%32);
    m.bus.vram[at..at+2].copy_from_slice(&v.to_le_bytes());
   }}
  }
  m.bus.io.fill(0);m.bus.io[..2].copy_from_slice(&0x0300u16.to_le_bytes());
  m.bus.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());m.bus.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
  for at in [16,20]{m.bus.io[at..at+2].copy_from_slice(&136u16.to_le_bytes());m.bus.io[at+2..at+4].copy_from_slice(&48u16.to_le_bytes());}
  for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
  let mut f=fs::File::create(format!("target/stage-catalogue/{id:03}.ppm")).unwrap();write!(f,"P6\n240 160\n255\n").unwrap();
  for &p in &m.bus.framebuffer{f.write_all(&[((p&31)*255/31)as u8,(((p>>5)&31)*255/31)as u8,(((p>>10)&31)*255/31)as u8]).unwrap();}
 }
}

