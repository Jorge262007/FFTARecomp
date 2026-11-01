use gba_core::Machine;
fn main(){let rom=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();let mut m=Machine::new(rom);let p="target/store-inventory-4800";
for(n,d)in[("ewram",&mut m.bus.ewram),("iwram",&mut m.bus.iwram),("vram",&mut m.bus.vram),("io",&mut m.bus.io),("oam",&mut m.bus.oam),("palette",&mut m.bus.palette)]{d.copy_from_slice(&std::fs::read(format!("{p}-{n}.bin")).unwrap());}
m.bus.ffta_movement.enabled=true;m.bus.ffta_movement.europe_profile=true;m.bus.ffta_dialogue.separate=true;gba_core::mem::set_extra(22);
for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}let before=m.bus.ffta_dialogue.pixels.clone();let oam=m.bus.oam.clone();let mut regions=Vec::new();
for o in oam.chunks_exact(8){let a:[u16;3]=std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]]));if a[0]&0xff00==0&&a[1]&0xfe00==0&&(0x2c0..=0x2d3).contains(&a[2]){regions.push(((a[1]&511)as usize,(a[0]&255)as usize));}}
assert!(!regions.is_empty());let owner=0x243f8;m.bus.ewram[owner+36..owner+38].fill(0);
for y in 0..160{gba_core::ppu::render_scanline(&mut m.bus,y);}let after=&m.bus.ffta_dialogue.pixels;let mut changes=0;
for(i,(a,b))in before.iter().zip(after).enumerate(){if a.packed()!=b.packed(){changes+=1;let x=i%480;let y=i/480;assert!(regions.iter().any(|(l,t)|x>=l+120&&x<l+128&&y>=*t&&y<t+8),"non-digit changed {x},{y}");}}
assert!(changes>0);assert_eq!(m.bus.oam,oam);println!("STORE_NUMBER_EXIT changed_pixels={changes} digits={} only_digits_changed=true oam_unchanged=true",regions.len());
let mut ppm=b"P6\n480 160\n255\n".to_vec();for p in after{let c=if p.visible(){p.color}else{0x4210};for sh in [0,5,10]{ppm.push((((c>>sh)&31)*255/31)as u8);}}std::fs::write("target/store-number-exit-proof.ppm",ppm).unwrap();
}
