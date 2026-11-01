//! Read-only save loading probe. Original ROM/save are never written.
use gba_core::{Machine,ffta_mouse::LanguageMouse};
fn main()->Result<(),Box<dyn std::error::Error>> {
    let a:Vec<_>=std::env::args().collect();
    let rom=std::fs::read(a.get(1).ok_or("ROM required")?)?;
    assert_eq!(gba_core::ffta_rom::version(&rom),Some(gba_core::ffta_rom::Version::Europe));
    let save=std::fs::read(a.get(2).ok_or("save required")?)?;
    let prefix=a.get(3).ok_or("output prefix required")?;
    let mut mouse=LanguageMouse::new(&rom,true);
    let mut m=Machine::new(rom);m.bus.load_save_data(&save);
    let mut seen=std::collections::BTreeSet::new();let mut press=0;let mut key=0x3ff;
    let mut loaded_at=None;
    for frame in 0..6000 {
        if press==0 && frame%60==0 {
            if let Some(scene)=mouse.scene(&m.bus) {
                let base=scene.targets.first().map(|t|t.index).unwrap_or(255);
                if loaded_at.is_none() || base!=56 {
                    let target=match base {0=>Some(0),64=>Some(64),32=>Some(33),56=>Some(56+scene.selected),_=>None};
                    println!("SAVE_MENU frame={frame} base={base} selected={} target={target:?}",scene.selected);
                    if let Some(target)=target {
                        if !mouse.select(&mut m.bus,target){return Err("selection rejected".into());}
                        key=if base==64{0x3f7}else{0x3fe};press=4;seen.insert(base);
                        if base==56 && loaded_at.is_none(){loaded_at=Some(frame);}
                    }
                }
            }
        }
        if press==0 && loaded_at.is_none() && frame%120==100 && mouse.scene(&m.bus).is_none() {
            key=0x3f7;press=4;
        }
        m.bus.keys=if press>0 {press-=1;key}else{0x3ff};
        m.run_frame(5_000_000);
        if loaded_at.is_some_and(|f|frame>f+900){break;}
    }
    let mut ppm=format!("P6\n240 160\n255\n").into_bytes();
    for p in &m.bus.framebuffer {for shift in [0,5,10] {let v=((p>>shift)&31)as u8;ppm.push((v<<3)|(v>>2));}}
    std::fs::write(format!("{prefix}.ppm"),ppm)?;
    std::fs::write(format!("{prefix}.ewram"),&m.bus.ewram)?;
    std::fs::write(format!("{prefix}.iwram"),&m.bus.iwram)?;
    println!("SAVE_PROBE frames={} slot_selected={} stage={:?} save_written=false",m.bus.frames,loaded_at.is_some(),gba_core::ffta_surface::Descriptor::read(&m.bus).map(|s|s.id));
    if loaded_at.is_none(){return Err("save slot menu was not reached".into());}
    Ok(())
}
