use gba_core::{Machine,ffta_battle_mouse as mouse};
fn load(rom:&[u8],state:u8)->(Machine,usize){
 let mut m=Machine::new(rom.to_vec());let p="target/ui-followup6-4000";
 for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("{p}-{n}.bin")).unwrap());}
 assert!(m.bus.ffta_battle_mouse.enable_replay());m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;
 let b=(u32::from_le_bytes(m.bus.ewram[0x918c..0x9190].try_into().unwrap())-0x02000000)as usize;
 m.bus.ewram[b+8]=state;m.bus.ewram[b+9]=if state==6{7}else{6};m.bus.ewram[b+0x1b]=if state==7{60}else{0};
 m.bus.ewram[b+0x20..b+0x22].copy_from_slice(&1u16.to_le_bytes());m.bus.ewram[b+0x29]=2;m.bus.ewram[b+0x2a]=0;
 m.bus.iwram[..8].fill(0);m.bus.keys=0x3ff;(m,b)
}
fn call(m:&mut Machine,state:u8){
 m.cpu.cpsr=0xbf;m.cpu.regs[13]=0x03007b00;m.cpu.regs[14]=0x08000001;m.cpu.regs[15]=0x0801194c;m.cpu.branch=None;
 for _ in 0..2000000{m.step();if m.cpu.regs[15]==0x08000000 || (state==6 && m.cpu.regs[15]==0x08011b22){return;}}
 panic!("dialogue helper did not return {:x}",m.cpu.regs[15]);
}
fn main(){let rom=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
 for state in [6,7]{
  let(mut idle,b)=load(&rom,state);call(&mut idle,state);assert_eq!(idle.bus.ewram[b+8],state,"no click preserves wait");
  let(mut clicked,b)=load(&rom,state);assert!(mouse::MenuScene::read(&clicked.bus).is_some(),"menu remains behind speech");
  assert!(mouse::advance_dialogue(&mut clicked.bus));call(&mut clicked,state);
  assert_eq!(clicked.bus.ewram[b+8],if state==6{7}else{6},"click advances original routine");
  assert_eq!(clicked.bus.keys,0x3ff);assert_eq!(&clicked.bus.iwram[..8],&[0;8]);
  println!("DIALOGUE state={state} original_routine_advanced=true background_menu_present=true shared_keys_unchanged=true");
 }
}


