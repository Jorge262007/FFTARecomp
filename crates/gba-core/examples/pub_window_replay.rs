//! Replays externally recorded window metadata/maps; contains no game assets.
use gba_core::{MemMap,ffta_ui_window::{self,Phase},ffta_ui::{Scene,Anchor}};
fn main(){
    let base=std::path::PathBuf::from(std::env::args_os().nth(1).expect("recording directory"));
    let mut counts=[0usize;4];let mut before_cursor=0;let mut checked=0;
    gba_core::mem::set_extra(164);gba_core::mem::set_extra_y(0);
    let rom=std::fs::read(std::env::args_os().nth(2).expect("ROM path for verified render profile")).unwrap();
    let mut m=MemMap::new(rom);m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
    m.ffta_dialogue.separate=true;m.ffta_dialogue.style.scale=1.5;
    m.ewram[0x7f10..0x7f12].copy_from_slice(&62u16.to_le_bytes());
    for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
    let mut cached=None;
    for sequence in ["sequence","root-return"] {
        for n in 0..10000 {
            let p=base.join(sequence).join(format!("frame-{n}"));
            let Ok(ew)=std::fs::read(p.with_extension("ew"))else{break;};
            let io=std::fs::read(p.with_extension("io")).unwrap();let maps=std::fs::read(p.with_extension("maps")).unwrap();
            m.ewram[0x25e80..0x25e80+ew.len()].copy_from_slice(&ew);
            m.io[..io.len()].copy_from_slice(&io);m.vram[0x5000..0x5000+maps.len()].copy_from_slice(&maps);
            let w=ffta_ui_window::pub_root(&m,cached);cached=w.map(|w|w.owner);
            let original=m.ewram.clone();
            for line in 0..160{gba_core::ppu::render_scanline(&mut m,line);}
            assert_eq!(m.ewram,original);
            if let Some(w)=w {
                counts[match w.phase{Phase::Entering=>0,Phase::Open=>1,Phase::Exiting=>2,Phase::Hidden=>3}]+=1;
                if w.visible(){
                    assert_eq!(m.ffta_dialogue.extra.bounds,Some(w.bounds),"{sequence} frame {n} {:?}",w.phase);
                    let node=Scene::capture(&m.ffta_dialogue).nodes[1].expect("visual owner in every phase");
                    assert_eq!(node.bounds,[0,72,96,160]);assert_eq!(node.anchor,Anchor::Corner{right:false,bottom:true});
                    let cursor=u32::from_le_bytes(m.ewram[0x25f08..0x25f0c].try_into().unwrap());
                    if cursor!=0x08018fc5{before_cursor+=1;}
                    checked+=1;
                }
            }
        }
    }
    assert!(counts[0]>0 && counts[1]>0 && counts[2]>0 && before_cursor>0);
    println!("PUB_REPLAY checked={checked} entering={} open={} exiting={} hidden={} before_cursor={before_cursor} bounds_stable=true anchor_stable=true ewram_unchanged=true",counts[0],counts[1],counts[2],counts[3]);
}
