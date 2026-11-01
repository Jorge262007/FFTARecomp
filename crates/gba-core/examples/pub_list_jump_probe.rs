//! Executes the verified original row repaint on an isolated recording copy.
use gba_core::Machine;
fn call(m:&mut Machine,pc:u32,args:[u32;4],r4:u32){
    m.cpu.cpsr=0xbf;m.cpu.regs[..4].copy_from_slice(&args);m.cpu.regs[4]=r4;m.cpu.regs[13]=0x03007b00;m.cpu.regs[14]=0x08000001;m.cpu.regs[15]=pc;m.cpu.branch=None;
    for _ in 0..4000000{m.step();if m.cpu.regs[15]==0x08000000{return;}}
    panic!("native call did not return: pc={:x} list={:?}",m.cpu.regs[15],gba_core::ffta_battle_mouse::pub_list(&m.bus));
}
fn main(){
    let a:Vec<_>=std::env::args().collect();let p=std::path::Path::new(&a[1]);let mut m=Machine::new(std::fs::read(&a[2]).unwrap());
    for(name,dst)in[("0x12cedfb088-262144",&mut m.bus.ewram),("0x12cedfb0a0-32768",&mut m.bus.iwram),("0x12cedfb0b8-1024",&mut m.bus.io),("0x12cedfb0c8-1024",&mut m.bus.palette),("0x12cedfb0e8-98304",&mut m.bus.vram),("0x12cedfb100-1024",&mut m.bus.oam)]{dst.copy_from_slice(&std::fs::read(p.join(name)).unwrap());}
    m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;assert!(m.bus.ffta_battle_mouse.enable_replay());
    gba_core::mem::set_extra(164);for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}
    let list=gba_core::ffta_battle_mouse::pub_list(&m.bus).unwrap();assert_eq!((list.total,list.visible,list.first),(42,8,0));
    let oam=m.bus.oam.clone();
    for obj in m.bus.oam.chunks_exact_mut(8){obj[..2].copy_from_slice(&0x0200u16.to_le_bytes());}
    assert!(gba_core::ffta_battle_mouse::pub_list(&m.bus).is_some(),"scrollbar survives hidden animation cursor");
    m.bus.oam.copy_from_slice(&oam);
    println!("PUB_SCROLL_LIFETIME hidden_cursor=true list_visible=true");
    let base=list.cursor-32;let parent=0x02000000+list.cursor as u32-0x5c;
    // Remove the pre-existing corruption in this recorded copy with the
    // original unhighlight function, exactly as a newly opened list starts.
    for row in 0..8{call(&mut m,0x08018c38,[0x02000000+base as u32,row,255,0],0);}
    let mut old=0;
    for target in [2,6,1,7,0] {
        call(&mut m,0x08018c38,[0x02000000+base as u32,old,255,0],0);
        assert!(gba_core::ffta_battle_mouse::menu_select(&mut m.bus,target as u8,false));
        let proposed=if target>old{old+1}else{old.saturating_sub(1)};
        call(&mut m,0x0806196c,[parent,0,proposed,0],proposed);
        assert_eq!(m.bus.ewram[list.cursor+17],target as u8,"direct row jump");
        let mut selected=Vec::new();
        let map=u32::from_le_bytes(m.bus.ewram[base+8..base+12].try_into().unwrap())as usize-0x06000000;
        for row in 0..8{let start=map+row*128;let marked=(0..16).any(|x|u16::from_le_bytes(m.bus.vram[start+x*2..start+x*2+2].try_into().unwrap())>>12==11);if marked{selected.push(row);}}
        assert_eq!(selected,vec![target as usize],"exactly one row highlighted");
        println!("PUB_JUMP {old}->{target} selected={selected:?} intermediate_rows=0");
        m.bus.ffta_battle_mouse.cancel();old=target;
    }
    assert!(gba_core::ffta_battle_mouse::pub_scroll_to(&mut m.bus,2));
    assert!(gba_core::ffta_battle_mouse::pub_scroll_unselected(&m.bus));
    assert!(m.bus.ffta_battle_mouse.interprets(0x08061a66));
    m.cpu.regs[15]=0x08061a6e;m.ffta_battle_mouse_entry();
    assert_eq!(m.cpu.regs[15],0x08061a72,"scroll bypasses navigation sound call");
    println!("PUB_SCROLL neutral=true navigation_sound_skipped=true");
    // Exercise the real input queue before calling the guest page helpers.
    // Drag updates must not restart the pulse, and replay must wake scroll-only
    // requests just like the windowed runner does.
    let mut pulse=Vec::new();
    for _ in 0..16 {
        assert!(gba_core::ffta_battle_mouse::pub_scroll_to(&mut m.bus,2));
        m.bus.keys=0x3ff;
        gba_core::ffta_battle_mouse::replay_keys(&mut m.bus);
        pulse.push(m.bus.keys);
    }
    assert_eq!(pulse,[vec![0x37f;4],vec![0x3ff;4],vec![0x37f;4],vec![0x3ff;4]].concat());
    println!("PUB_SCROLL_INPUT click_and_drag press=4 release=4 replay_wakes=true");
    call(&mut m,0x08018c38,[0x02000000+base as u32,0,255,0],0);
    call(&mut m,0x0806196c,[parent,0x80,1,0],1);
    assert_eq!(m.bus.ewram[list.cursor+17],7,"scroll immediately reaches visible edge");
    assert_eq!(gba_core::ffta_battle_mouse::pub_list(&m.bus).unwrap().first,2,"one update reaches dragged page");
    for target in [34,0,17,2] {
        assert!(gba_core::ffta_battle_mouse::pub_scroll_to(&mut m.bus,target));
        let old=m.bus.ewram[list.cursor+17]as u32;
        call(&mut m,0x08018c38,[0x02000000+base as u32,old,255,0],0);
        call(&mut m,0x0806196c,[parent,0,old,0],old);
        assert_eq!(gba_core::ffta_battle_mouse::pub_list(&m.bus).unwrap().first,target);
        let scene=gba_core::ffta_battle_mouse::MenuScene::read(&m.bus).unwrap();
        for row in 0..scene.count {
            assert_eq!(scene.hit((scene.origin[0]+8)as f32,(scene.origin[1]+row as i32*scene.pitch+4)as f32),Some(row));
        }
        println!("PUB_SCROLL_DIRECT first={target} all_visible_rows_selectable=true");
    }

    m.bus.ffta_battle_mouse.cancel();
    let old=m.bus.ewram[list.cursor+17]as u32;
    call(&mut m,0x08018c38,[0x02000000+base as u32,old,255,0],0);
    assert!(gba_core::ffta_battle_mouse::menu_select(&mut m.bus,1,false));
    assert!(!gba_core::ffta_battle_mouse::pub_scroll_unselected(&m.bus));
    call(&mut m,0x0806196c,[parent,0,6,0],6);
    assert_eq!(m.bus.ewram[list.cursor+17],1);
    let map=u32::from_le_bytes(m.bus.ewram[base+8..base+12].try_into().unwrap())as usize-0x06000000;
    let marked:Vec<_>=(0..8).filter(|row|(0..16).any(|x|u16::from_le_bytes(m.bus.vram[map+row*128+x*2..map+row*128+x*2+2].try_into().unwrap())>>12==11)).collect();
    assert_eq!(marked,vec![1]);
    println!("PUB_JUMP_AFTER_SCROLL first=2 selected={marked:?} only_one_highlight=true");

}
