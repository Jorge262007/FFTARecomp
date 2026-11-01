use gba_core::{MemMap,ppu};
fn main(){
    let mut m=MemMap::new(vec![]);
    let prefix=std::env::args().nth(1).unwrap_or("target/stage-town-light-29950".into());
    m.palette.copy_from_slice(&std::fs::read(format!("{prefix}-palette.bin")).unwrap());
    m.vram.copy_from_slice(&std::fs::read(format!("{prefix}-vram.bin")).unwrap());
    let o=std::fs::read(format!("{prefix}-oam.bin")).unwrap();
    m.io[..2].copy_from_slice(&0x1040u16.to_le_bytes());
    let mut out=vec![0u16;512*256];
    for slot in 0..32 {
        for obj in m.oam.chunks_exact_mut(8){obj[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        m.oam[..8].copy_from_slice(&o[slot*8..slot*8+8]);
        m.oam[0]=8;m.oam[2]=8;m.oam[3]&=0xfe;
        for y in 0..160 {ppu::render_scanline(&mut m,y);}
        println!("{slot}: {:x?}",(0..3).map(|i|u16::from_le_bytes(o[slot*8+i*2..slot*8+i*2+2].try_into().unwrap())).collect::<Vec<_>>());
        for y in 0..64 {for x in 0..64 {out[(slot/8*64+y)*512+slot%8*64+x]=m.framebuffer[y*240+x];}}
    }
    let mut ppm=b"P6\n512 256\n255\n".to_vec();
    for c in out {for shift in [0,5,10] {ppm.push((((c>>shift)&31)*255/31)as u8);}}
    std::fs::write("target/map-menu-sprite-sheet.ppm",ppm).unwrap();
}
