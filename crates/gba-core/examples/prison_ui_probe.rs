use gba_core::{Machine,ffta_battle_mouse as mouse};
fn main(){let mut m=Machine::new(std::fs::read(std::env::args().nth(1).unwrap()).unwrap());for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("target/prison-live-{n}.bin")).unwrap());}m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;assert!(m.bus.ffta_battle_mouse.enable_replay());gba_core::mem::set_extra(22);for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}let owner=0x243f4;
for(current,target)in[(1u16,8u16),(4,8),(8,8),(8,0),(1,0)]{
    m.bus.ewram[owner+32..owner+34].copy_from_slice(&current.to_le_bytes());
    m.bus.ewram[owner+36..owner+38].copy_from_slice(&target.to_le_bytes());
    for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
    let node=gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue).nodes[1].unwrap();
    assert_eq!(node.bounds,[0,72,72,144]);assert!(m.bus.ffta_dialogue.service_speech.active());
    println!("PRISON_ANIMATION current={current} target={target} fixed_bounds=true independent_speech=true");
let base=(u32::from_le_bytes(m.bus.ewram[0x918c..0x9190].try_into().unwrap())-0x02000000)as usize;
// A genuine external/menu wait must never become a blind dialogue confirm.
m.bus.ewram[base+8]=8;assert!(mouse::DialogueChoice::read(&m.bus).is_none());assert!(!mouse::advance_dialogue(&mut m.bus));
for target in [0u8,1]{
 m.bus.ewram[base+8]=9;m.bus.ewram[base+9]=5;m.bus.ewram[base+0x1b]=0;
 m.bus.ewram[base+0x34..base+0x36].copy_from_slice(&40u16.to_le_bytes());
 m.bus.ewram[base+0x36..base+0x38].copy_from_slice(&48u16.to_le_bytes());
 m.bus.ewram[base+0x41]=0;m.bus.ewram[base+0x42]=0;m.bus.ewram[base+0x43]=25;
 let choice=mouse::DialogueChoice::read(&m.bus).unwrap();assert_eq!(choice.hit(41.+target as f32*36.,50.),Some(target));assert_eq!(choice.hit(5.,5.),None);
 assert!(choice.select(&mut m.bus,target,true));m.bus.iwram[..8].fill(0);m.bus.keys=0x3ff;
 m.cpu.cpsr=0xbf;m.cpu.regs[13]=0x03007b00;m.cpu.regs[14]=0x08000001;m.cpu.regs[15]=0x0801194c;m.cpu.branch=None;
 let mut passed=false;for _ in 0..100000{m.step();if m.cpu.regs[15]==0x08011b7e{passed=true;break;}}
 assert!(passed,"question original confirm branch not reached");assert_eq!(m.bus.ewram[base+8],5);assert_eq!(m.bus.ewram[base+0x41],target);assert_eq!(&m.bus.iwram[..8],&[0;8]);assert_eq!(m.bus.keys,0x3ff);
 println!("PRISON_QUESTION answer={target} original_confirm_branch=true shared_keys_unchanged=true");
}
}
m.bus.ewram[owner+32..owner+34].copy_from_slice(&8u16.to_le_bytes());m.bus.ewram[owner+36..owner+38].copy_from_slice(&8u16.to_le_bytes());
println!("MENU {:?}",mouse::MenuScene::read(&m.bus));println!("UI {:?}",gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue));
for target in [2,0,1]{assert!(mouse::menu_select(&mut m.bus,target,false));assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x3ff);assert_eq!(mouse::MenuScene::read(&m.bus).unwrap().selected,target);println!("PRISON_TARGET target={target} direct=true no_keys=true");m.bus.ffta_battle_mouse.cancel();}
}
