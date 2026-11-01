fn main(){
 let a:Vec<_>=std::env::args().collect();let rom=std::fs::read(&a[1]).unwrap();
 for range in &a[2..]{let(l,r)=range.split_once(':').unwrap();let mut p=u32::from_str_radix(l.trim_start_matches("0x"),16).unwrap();let r=u32::from_str_radix(r.trim_start_matches("0x"),16).unwrap();
 while p<r{let o=(p&0x1ffffff)as usize;let h=u16::from_le_bytes(rom[o..o+2].try_into().unwrap());
 if h&0xf800==0xf000 && o+4<=rom.len(){let lo=u16::from_le_bytes(rom[o+2..o+4].try_into().unwrap());if lo&0xf800==0xf800{let mut off=(((h&2047)as i32)<<12)|(((lo&2047)as i32)<<1);if off&0x400000!=0{off-=0x800000;}println!("{p:08x} {h:04x} {lo:04x} bl 0x{:08x}",p.wrapping_add(4).wrapping_add(off as u32));p+=4;continue;}}
 let literal=if h&0xf800==0x4800{let at=((p+4)&!3)+((h&255)as u32*4);let at=(at&0x1ffffff)as usize;format!(" value={:08x}",u32::from_le_bytes(rom[at..at+4].try_into().unwrap()))}else{String::new()};
 println!("{p:08x} {h:04x} {}{literal}",armv4t::decode_thumb(h,p).disasm());p+=2;
 }}
}
