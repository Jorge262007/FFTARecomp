//! Inspect the original LCD screenshot from a private save state; no ROM needed.
fn main()->Result<(),Box<dyn std::error::Error>>{
    let a:Vec<_>=std::env::args().collect();
    if a.len()<3{return Err("usage: state_preview <state> <output.ppm>".into());}
    if let Some(rom)=a.get(3){
        let mut m=Box::new(gba_core::Machine::new(std::fs::read(rom)?));
        m.load_state(&std::fs::read(&a[1])?).map_err(std::io::Error::other)?;
        for (i,o)in m.bus.oam.chunks_exact(8).enumerate(){
            let a:[u16;3]=std::array::from_fn(|j|u16::from_le_bytes([o[j*2],o[j*2+1]]));
            if a[0]&0x300==0x200 {continue;}
            println!("OAM {i:3} {:04x} {:04x} {:04x} x={} y={} tile={:03x} priority={}",a[0],a[1],a[2],a[1]&511,a[0]&255,a[2]&1023,a[2]>>10&3);
        }
    }
    let (_,pixels)=gba_core::state::preview(&std::fs::read(&a[1])?).map_err(std::io::Error::other)?;
    let mut out=b"P6\n240 160\n255\n".to_vec();
    for p in pixels{for shift in [0,5,10]{let c=((p>>shift)&31)as u8;out.push(c<<3|c>>2);}}
    std::fs::write(&a[2],out)?;Ok(())
}
