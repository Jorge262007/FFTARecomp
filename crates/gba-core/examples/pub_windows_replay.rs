//! Replay read-only Pub recordings (kept outside source); no assets embedded.
use gba_core::{MemMap,ffta_ui_window::{self,Phase},ffta_ui::{Scene,Anchor,Role}};
fn main(){
    let folder=std::path::PathBuf::from(std::env::args_os().nth(1).expect("recording folder"));
    let rom=std::fs::read(std::env::args_os().nth(2).expect("ROM path")).unwrap();
    let mut m=MemMap::new(rom);m.ffta_dialogue.separate=true;m.ffta_movement.enabled=true;m.ffta_movement.europe_profile=true;
    m.ewram.copy_from_slice(&std::fs::read(folder.join("0x324ff9ef8-262144")).unwrap());
    m.vram.copy_from_slice(&std::fs::read(folder.join("0x324ff9f58-98304")).unwrap());
    gba_core::mem::set_extra(164);gba_core::mem::set_extra_y(0);
    let mut files:Vec<_>=std::fs::read_dir(folder.join("replay")).unwrap().map(|e|e.unwrap().path()).filter(|p|p.extension().is_some_and(|e|e=="packet")).collect();files.sort();
    let mut counts=[[0;4];3];let mut checked=0;let mut root_header=0;let mut cached=[None;3];
    for file in files {
        let b=std::fs::read(&file).unwrap();assert_eq!(b.len(),104534);
        let mut at=0;
        m.ewram[0x20000..0x30000].copy_from_slice(&b[at..at+65536]);at+=65536;
        m.io[..86].copy_from_slice(&b[at..at+86]);at+=86;
        m.vram[0x5000..0x6000].copy_from_slice(&b[at..at+4096]);at+=4096;
        m.oam[..].copy_from_slice(&b[at..at+1024]);at+=1024;
        m.vram[0x8000..0x10000].copy_from_slice(&b[at..at+32768]);at+=32768;
        m.palette[..].copy_from_slice(&b[at..at+1024]);at+=1024;
        assert_eq!(at,b.len());
        let windows=ffta_ui_window::pub_windows(&m,cached);cached=windows.map(|w|w.map(|w|w.owner));
        let original=m.ewram.clone();
        for y in 0..160{gba_core::ppu::render_scanline(&mut m,y);}
        assert_eq!(m.ewram,original);
        let scene=Scene::capture(&m.ffta_dialogue);
        for (kind,w)in windows.into_iter().enumerate(){if let Some(w)=w{
            counts[kind][match w.phase{Phase::Entering=>0,Phase::Open=>1,Phase::Exiting=>2,Phase::Hidden=>3}]+=1;
            if !w.visible(){continue;}
            checked+=1;
            match kind {
                0=>{let n=scene.nodes[1].unwrap();assert_eq!(n.bounds,w.bounds,"{}",file.display());assert_eq!(n.anchor,Anchor::Corner{right:false,bottom:true});
                    if w.phase==Phase::Open && m.ffta_dialogue.extra.pixels[72*480..80*480].iter().any(|p|p.visible()){root_header+=1;}
                },
                1=>{let n=scene.nodes[0].unwrap();assert_eq!(n.bounds,w.bounds,"{}",file.display());assert_eq!(n.role,Role::ServiceContent);assert_eq!(n.anchor,Anchor::PanelCenter);},
                2=>{assert!(m.ffta_dialogue.service_speech.active());assert_eq!(m.ffta_dialogue.service_speech.bounds,Some(w.bounds));assert_eq!(w.bounds,[0,0,240,80]);},_=>unreachable!()
            }
        }}
    }
    for i in 0..3{assert!(counts[i][0]>0 && counts[i][1]>0 && counts[i][2]>0,"missing phase in family {i}: {:?}",counts[i]);}
    assert!(root_header>0,"portrait margin must not capture the neighboring MENU header");
    println!("PUB_WINDOWS_REPLAY checked={checked} phases={counts:?} root_header_samples={root_header} canonical_bounds_stable=true anchors_stable=true ewram_unchanged=true");
}
