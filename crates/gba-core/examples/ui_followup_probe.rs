//! Checks desktop UI changes against private local recordings, never save files.
use gba_core::{Machine,ffta_battle_mouse as mouse};
fn load(rom:&[u8],prefix:&str)->Machine {
    let mut m=Machine::new(rom.to_vec());
    for (name,dst) in [("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)] {
        dst.copy_from_slice(&std::fs::read(format!("{prefix}-{name}.bin")).unwrap());
    }
    m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;
    m.bus.ffta_dialogue.separate=true;assert!(m.bus.ffta_battle_mouse.enable_replay());m
}
fn call(m:&mut Machine,pc:u32,args:[u32;4]) {
    m.cpu.cpsr=0xbf;m.cpu.regs[..4].copy_from_slice(&args);m.cpu.regs[13]=0x03007b00;
    m.cpu.regs[14]=0x08000001;m.cpu.regs[15]=pc;m.cpu.branch=None;
    for _ in 0..2000000 {m.step();if m.cpu.regs[15]==0x08000000{return;}}
    panic!("original helper failed to return {:x}",m.cpu.regs[15]);
}
fn main(){
    let rom=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let mut title=load(&rom,"target/ui-followup2-1750");mouse::set_mouse_visual(&mut title.bus,true);
    let feather:[u16;3]=std::array::from_fn(|i|u16::from_le_bytes(title.bus.oam[16*8+i*2..16*8+i*2+2].try_into().unwrap()));
    assert!(mouse::hide_mouse_indicator(&title.bus,feather));mouse::set_mouse_visual(&mut title.bus,false);
    assert!(!mouse::hide_mouse_indicator(&title.bus,feather));println!("TITLE_FEATHER mouse_hidden=true keyboard_restored=true attrs={feather:x?}");
    let mut map=load(&rom,"target/stage-town-light-29950");mouse::set_mouse_visual(&mut map.bus,true);
    for slot in [3,4] {let a=std::array::from_fn(|i|u16::from_le_bytes(map.bus.oam[slot*8+i*2..slot*8+i*2+2].try_into().unwrap()));assert!(!mouse::hide_mouse_indicator(&map.bus,a));}
    assert!(!mouse::hide_mouse_indicator(&map.bus,[0x1a,0x8068,0x98d0]));
    assert!(mouse::hide_mouse_indicator(&map.bus,[0x403c,0x13,0]));
    println!("MAP_POINTER hidden=true actor_and_destination_preserved=true");
    let mut saves=load(&rom,"target/ui-followup4-2600");
    let left=load(&rom,"target/ui-followup5-2800");
    let mut front=gba_core::ffta_mouse::LanguageMouse::new(&rom,true);
    let scene=front.scene(&saves.bus).expect("actual save-card scene");
    assert_eq!(scene.targets.iter().map(|t|t.index).collect::<Vec<_>>(),vec![60,61]);
    assert!(front.aim(&saves.bus,60.,100.,false));front.keys(&mut saves.bus,0x3ff);
    for off in [0x105,0x106,0x107,0x110,0x111] {let p=0x1152c+off;assert_eq!(saves.bus.ewram[p],left.bus.ewram[p],"slot metadata {off:x}");}
    assert!(front.aim(&saves.bus,170.,100.,true));
    let mut a=0;for f in 1..20{saves.bus.frames=f;a+=usize::from(front.keys(&mut saves.bus,0x3ff)==0x3fe);}assert_eq!(a,4);
    println!("SAVE_CARDS both_targets=true metadata_matches_original=true confirm_once=true");
    let mut town=load(&rom,"target/ui-followup6-3300");
    let scene=mouse::MenuScene::read(&town.bus).expect("actual map destination menu");println!("MAP_MENU scene={scene:?}");
    for target in [2,0,1] {
        assert!(mouse::menu_select(&mut town.bus,target,false));
        assert_eq!(mouse::MenuScene::read(&town.bus).unwrap().selected,target);
        assert_eq!(mouse::keys(&mut town.bus,0x3ff),0x3ff,"no simulated direction");
        println!("MAP_DIRECT target={target} cursor_index_immediate=true directional_keys=none");town.bus.ffta_battle_mouse.cancel();
        let map=0x16d82;let selected:Vec<_>=(0..3).filter(|row|(2..22).any(|x|{
            let at=map+row*96+x*2;
            u16::from_le_bytes(town.bus.ewram[at..at+2].try_into().unwrap())>>12==11
        })).collect();assert_eq!(selected,vec![target as usize]);
    }
    for (x,inside) in [(31.,false),(32.,true),(100.,true),(207.,true),(208.,false),(223.,false)] {
        assert_eq!(mouse::aim(&mut town.bus,x,72.,false),inside,"blue backing boundary {x}");
        town.bus.ffta_battle_mouse.cancel();
    }
    println!("MAP_BLUE_BOUNDARY native_x=32..208 outside_rejected=true");
    gba_core::mem::set_extra(22);
    let mut expected=None;
    for frame in [4199,4204,4210,4220,4250,4500,4600,4604,4610,4620,4650] {
        let mut pub_scene=load(&rom,&format!("target/pub-funds-cycle-{frame}"));
        for y in 0..160 {gba_core::ppu::render_scanline(&mut pub_scene.bus,y);}
        assert_eq!(pub_scene.bus.ffta_dialogue.funds_bounds,Some([160,136,240,160]));
        let pixels=&pub_scene.bus.ffta_dialogue.funds_pixels;
        println!("FUNDS_VISIBLE {}",pixels.iter().filter(|p|p.visible()).count()); assert!(pixels.iter().filter(|p|p.visible()).count()>900,"gold counter actually captured");
        let bytes:Vec<_>=pixels.iter().flat_map(|p|p.packed()).collect();
        let hash=gba_core::ffta_rom::digest(&bytes);
        if let Some(previous)=&expected {assert_eq!(&hash,previous,"counter pixels remain stable across submenu animation at {frame}");}
        else{expected=Some(hash.clone());}
        println!("PUB_FUNDS frame={frame} bounds=160,136,240,160 pixels_sha256={hash}");
    }
    let mut rumors=load(&rom,"target/rumors-counter-recheck-4500");
    for y in 0..160 {gba_core::ppu::render_scanline(&mut rumors.bus,y);}
    println!("RUMORS counter={:?} content={:?}",rumors.bus.ffta_dialogue.funds_bounds,rumors.bus.ffta_dialogue.service_list_bounds);
    assert!(rumors.bus.ffta_dialogue.service_list);
    // Save the extracted panel itself: a wrong cut cannot pass merely because
    // the same wrong pixels recur throughout an animation.
    for (name,pixels) in [("rumors-corrected-layer",&rumors.bus.ffta_dialogue.pixels),
        ("funds-corrected-layer",&load_and_render_funds(&rom))] {
        let mut ppm=b"P6\n480 160\n255\n".to_vec();
        for p in pixels {let c=if p.visible(){p.color}else{0x4210};for shift in [0,5,10]{ppm.push((((c>>shift)&31)*255/31)as u8);}}
        std::fs::write(format!("target/{name}.ppm"),ppm).unwrap();
    }

}


fn load_and_render_funds(rom:&[u8])->Vec<gba_core::ffta_dialogue::UiPixel>{
    let mut m=load(rom,"target/pub-funds-cycle-4199");
    for y in 0..160 {gba_core::ppu::render_scanline(&mut m.bus,y);}
    m.bus.ffta_dialogue.funds_pixels
}
