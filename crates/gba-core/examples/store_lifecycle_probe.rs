use gba_core::{Machine,ffta_battle_mouse as mouse};
fn load(rom:&[u8],prefix:&str)->Machine{let mut m=Machine::new(rom.to_vec());for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("{prefix}-{n}.bin")).unwrap());}m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;assert!(m.bus.ffta_battle_mouse.enable_replay());m}
fn call(m:&mut Machine,cursor:usize){run(m,cursor,false)}
fn run(m:&mut Machine,cursor:usize,hover:bool){let complete:Vec<_>=m.bus.ffta_dialogue.pixels.iter().map(|p|p.packed()).collect();let mut tested=false;m.cpu.cpsr=0xbf;m.cpu.regs[0]=0x02000000+cursor as u32;m.cpu.regs[13]=0x03007b00;m.cpu.regs[14]=0x08000001;m.cpu.regs[15]=0x08071030;m.cpu.branch=None;for _ in 0..2000000{
if !hover && !tested && m.cpu.regs[15]==0x08070696 {
    // Original helper just cleared its glyph buffer. Render a complete frame
    // right at this worst-case point rather than trying to catch it live.
    for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
    let shown:Vec<_>=m.bus.ffta_dialogue.pixels.iter().map(|p|p.packed()).collect();
    assert_eq!(shown,complete,"partial page must never become visible");tested=true;
    println!("STORE_SCROLL_DURING_CLEAR previous_page_preserved=true");
}
if hover {assert!(!matches!(m.cpu.regs[15],0x0807067a|0x08070692|0x080706c8),"hover must never clear or rebuild item glyphs");}m.step();if m.cpu.regs[15]==0x08000000{for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}return;}}panic!("original callback did not return {:x}",m.cpu.regs[15]);}
fn main(){let rom=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();gba_core::mem::set_extra(22);
 for frame in [4104,4110,4120,4150,4300]{let mut m=load(&rom,&format!("target/store-lifecycle-{frame}"));for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}let scene=gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue);println!("STORE_ROOT frame={frame} nodes={:?}",scene.nodes);assert!(scene.nodes.iter().flatten().any(|n|n.role==gba_core::ffta_ui::Role::ServiceMenu));}
 let mut root=load(&rom,"target/store-lifecycle-4300");
 for target in [2,0,3,1] {assert!(mouse::menu_select(&mut root.bus,target,false));assert_eq!(mouse::MenuScene::read(&root.bus).unwrap().selected,target);assert_eq!(mouse::keys(&mut root.bus,0x3ff),0x3ff);println!("STORE_ROOT_DIRECT target={target} immediate=true keys=none");root.bus.ffta_battle_mouse.cancel();}
 for prefix in ["target/store-inventory-4800","target/store-inventory-6000"]{let mut m=load(&rom,prefix);for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}let scene=gba_core::ffta_ui::Scene::capture(&m.bus.ffta_dialogue);
 let node=scene.nodes[0].expect("inventory canvas");assert_eq!(node.bounds,[0,0,240,160]);assert_eq!(node.anchor,gba_core::ffta_ui::Anchor::PanelCenter);
 let mut ppm=b"P6\n480 160\n255\n".to_vec();for p in &m.bus.ffta_dialogue.pixels{let c=if p.visible(){p.color}else{0x4210};for sh in [0,5,10]{ppm.push((((c>>sh)&31)*255/31)as u8);}}
 std::fs::write(format!("{prefix}-corrected-layer.ppm"),ppm).unwrap();
 let l=gba_core::ffta_shop_mouse::list(&m.bus).expect("live inventory");println!("STORE_LIST {prefix} total={} first={} visible={}",l.total,l.first,l.visible);
 for first in [l.total-l.visible,0,1]{assert!(gba_core::ffta_shop_mouse::scroll_to(&mut m.bus,first));call(&mut m,l.cursor);let after=gba_core::ffta_shop_mouse::list(&m.bus).unwrap();assert_eq!(after.first,first);assert_eq!(m.bus.keys,0x3ff);println!("STORE_SCROLL first={first} original_redraw=true no_keys=true");}
 assert!(mouse::MenuScene::read(&m.bus).is_some());for target in [3,0,2]{assert!(mouse::menu_select(&mut m.bus,target,false));assert_eq!(mouse::keys(&mut m.bus,0x3ff),0x3ff);run(&mut m,l.cursor,true);assert_eq!(mouse::MenuScene::read(&m.bus).unwrap().selected,target);println!("STORE_HOVER target={target} direct_original_redraw=true keys=none");m.bus.ffta_battle_mouse.cancel();}
 }
}
