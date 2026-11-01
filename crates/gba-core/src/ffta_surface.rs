//! Read-only EUR stage identity from the original map loader, not image heuristics.
use crate::MemMap;
pub const STAGE_COUNT:u16=162;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Descriptor {pub id:u16,pub terrain_bg:usize,pub scroll:[u16;2],pub source_origin:Option<(i32,i32)>}
impl Descriptor {
 pub fn read(mem:&MemMap)->Option<Self>{
  if !mem.ffta_movement.europe_profile || mem.io[0]&0x87!=0{return None;}
  // The scene callback changes during cinematics. Terrain identity remains
  // the loaded stage ID plus its exact terrain BG pair, not the active script.
  let half=|b:&[u8],o|u16::from_le_bytes([b[o],b[o+1]]);
  let id=half(&mem.ewram,0x7f10);if id>=STAGE_COUNT{return None;}
  let display=half(&mem.io,0);
  let bg=[0usize,2].into_iter().find(|&bg|half(&mem.io,8+bg*2)==0x4c03 && half(&mem.io,10+bg*2)==0x4e02 && display&(3<<(8+bg))!=0)?;
  Some(Self{id,terrain_bg:bg,scroll:[half(&mem.io,0x10+bg*4)&511,half(&mem.io,0x12+bg*4)&511],source_origin:if mem.ffta_stage.valid[0]{mem.ffta_stage.origins[0]}else{None}})
 }
}
#[derive(Default)]
pub struct State {pub enabled:bool,pub stage:Option<Descriptor>,pub epoch:u64,pub mask:Vec<u8>,pub coordinates:Vec<u32>,pub scratch:Vec<u32>,pub size:(usize,usize)}
impl State {
 pub fn new()->Self{Self{enabled:false,..Self::default()}}
 pub fn begin(&mut self,stage:Option<Descriptor>,size:(usize,usize),frame:u64){
  if stage.map(|s|s.id)!=self.stage.map(|s|s.id){
   self.epoch+=1;
   if stage_log(){eprintln!("FFTA_STAGE frame={frame} epoch={} id={:?} bg={:?}",self.epoch,stage.map(|s|s.id),stage.map(|s|s.terrain_bg));}
  }
  self.stage=stage;self.size=size;
  if self.capture(){self.mask.resize(size.0*size.1,0);self.mask.fill(0);self.coordinates.resize(size.0*size.1,0);self.coordinates.fill(0);}else{self.mask.clear();self.coordinates.clear();}
 }
 pub fn capture(&self)->bool{self.enabled&&self.stage.is_some()}
}
fn stage_log()->bool{static ON:std::sync::OnceLock<bool>=std::sync::OnceLock::new();*ON.get_or_init(||std::env::var("FFTA_STAGE_LOG").as_deref()==Ok("1"))}
#[cfg(test)] mod tests{
 use super::*;
 #[test] fn inactive_or_invalid_stage_never_inherits_identity(){
  let mut m=MemMap::new(vec![]);m.ffta_movement.europe_profile=true;m.io[..2].copy_from_slice(&0x3f40u16.to_le_bytes());
  m.iwram[0x2804..0x2808].copy_from_slice(&0x08025115u32.to_le_bytes());
  for (i,v) in [0x4c03u16,0x4e02,0x0b08,0x0a08].iter().enumerate(){m.io[8+i*2..10+i*2].copy_from_slice(&v.to_le_bytes());}
  m.ewram[0x7f10..0x7f12].copy_from_slice(&5u16.to_le_bytes());assert_eq!(Descriptor::read(&m).unwrap().id,5);
  m.iwram[0x2804..0x2808].copy_from_slice(&0x08012345u32.to_le_bytes());
  assert_eq!(Descriptor::read(&m).unwrap().id,5,"cinematic callback must not unload the terrain shader");
  m.io[8..10].copy_from_slice(&0xaa03u16.to_le_bytes());m.io[10..12].copy_from_slice(&0xac02u16.to_le_bytes());assert!(Descriptor::read(&m).is_none());
  let mut s=State::new();s.enabled=true;s.begin(Some(Descriptor{id:5,terrain_bg:0,scroll:[0,0],source_origin:None}),(8,8),0);assert_eq!(s.mask.len(),64);
  s.begin(None,(8,8),1);assert!(s.mask.is_empty());assert_eq!(s.epoch,2);
 }
}


/// Actual source pixel in the full calculated 64x64-tile terrain plane.
/// Zero means no terrain: OBJ, UI, backdrop and invalid/unverified map buffers.
pub fn coordinate(mem:&MemMap,bg:usize,x:usize,line:usize)->u32{
    let Some(index)=crate::ffta_stage::layer(mem,bg) else{return 0;};
    let Some((ox,oy))=mem.ffta_stage.origins[index] else{return 0;};
    let reg=|o:usize|u16::from_le_bytes([mem.io[o],mem.io[o+1]])as i32&511;
    let px=x as i32-crate::mem::extra()as i32+reg(0x10+bg*4)+ox*8;
    let py=line as isize as i32+reg(0x12+bg*4)+oy*8;
    if !(0..512).contains(&px)|| !(0..512).contains(&py){return 0;}
    1+py as u32*512+px as u32
}
/// Reconstruct only the two original terrain layers. No backdrop, UI or objects.
pub fn terrain_atlas(mem:&MemMap)->Option<Vec<[u8;4]>>{
    let stage=Descriptor::read(mem)?;
    if !mem.ffta_stage.valid.iter().all(|v|*v){return None;}
    let mut pixels=vec![[0;4];512*512];
    let half=|b:&[u8],o:usize|u16::from_le_bytes([b[o],b[o+1]]);
    // Layer 0 has priority 3; layer 1 has priority 2 and covers it.
    for index in 0..2{
        if !mem.ffta_stage.valid[index]{return None;}
        let base=mem.ffta_stage.bases[index]?.checked_sub(0x02000000)? as usize;
        if base+8192>mem.ewram.len(){return None;}
        let cnt=half(&mem.io,8+(stage.terrain_bg+index)*2);
        let chars=((cnt>>2)&3)as usize*0x4000;
        for ty in 0..64{for tx in 0..64{
            let entry=half(&mem.ewram,base+2*(ty*64+tx));
            let tile=chars+(entry as usize&1023)*32;let bank=(entry as usize>>12)*16;
            for y in 0..8{for x in 0..8{
                let sx=if entry&0x400!=0{7-x}else{x};let sy=if entry&0x800!=0{7-y}else{y};
                let byte=*mem.vram.get(tile+sy*4+sx/2)?;let color=(byte>>((sx&1)*4))&15;
                if color==0{continue;}
                let c=half(&mem.palette,(bank+color as usize)*2);
                let rgb=[0,5,10].map(|shift|{let v=((c>>shift)&31)as u8;v<<3|v>>2});pixels[(ty*8+y)*512+tx*8+x]=[rgb[0],rgb[1],rgb[2],255];
            }}
        }}
    }
    Some(pixels)
}

#[cfg(test)]mod plane_tests{
 use super::*;
 #[test]fn coordinates_follow_original_source_pixels_at_each_camera_and_zoom(){
  let mut m=MemMap::new(vec![]);m.ffta_stage.enabled=true;m.ffta_stage.valid=[true,true];
  m.ffta_stage.bases=[Some(0x02010000),Some(0x02012000)];m.ffta_stage.origins=[Some((10,12)),Some((10,12))];
  m.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());m.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
  crate::mem::set_extra(0);
  let before=coordinate(&m,0,30,20);
  m.io[0x10..0x12].copy_from_slice(&16u16.to_le_bytes());
  assert_eq!(coordinate(&m,0,14,20),before);
  // The renderer can crop the RAM plane while keeping its hardware scroll local.
  m.ffta_stage.origins[0]=Some((11,12));m.io[0x10..0x12].copy_from_slice(&8u16.to_le_bytes());
  assert_eq!(coordinate(&m,0,14,20),before);
  crate::mem::set_extra(164);assert_eq!(coordinate(&m,0,178,20),before);
  assert_eq!(coordinate(&m,4,178,20),0);assert_eq!(coordinate(&m,5,178,20),0);
  crate::mem::set_extra(0);
 }
 #[test]fn atlas_contains_only_original_terrain_planes_not_obj_ui_or_sky(){
  let mut m=MemMap::new(vec![]);m.ffta_movement.europe_profile=true;m.ffta_stage.enabled=true;
  m.ffta_stage.valid=[true,true];m.ffta_stage.origins=[Some((0,0));2];m.ffta_stage.bases=[Some(0x02010000),Some(0x02012000)];
  m.io[..2].copy_from_slice(&0x3f40u16.to_le_bytes());m.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());m.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
  m.iwram[0x2804..0x2808].copy_from_slice(&0x08025115u32.to_le_bytes());
  m.vram[32..64].fill(0x11);m.vram[64..96].fill(0x22);
  m.palette[2..4].copy_from_slice(&31u16.to_le_bytes());m.palette[4..6].copy_from_slice(&0x03e0u16.to_le_bytes());
  m.ewram[0x10000..0x10002].copy_from_slice(&1u16.to_le_bytes());m.ewram[0x10002..0x10004].copy_from_slice(&1u16.to_le_bytes());
  m.ewram[0x12000..0x12002].copy_from_slice(&2u16.to_le_bytes());
  let atlas=terrain_atlas(&m).unwrap();
  assert_eq!(atlas[0],[0,255,0,255]);assert_eq!(atlas[8],[255,0,0,255]);assert_eq!(atlas[16],[0,0,0,0]);
 }
}
