//! Opt-in reference catalogue for future UI mods. No OCR or guessed ROM offsets.
//! Exports addresses, geometry and hashes, never ROM/save bytes or game artwork.
use std::{io::Write,path::PathBuf};
use gba_core::{MemMap,ffta_ui::Scene};
#[derive(Default)]
pub struct Inspector {path:Option<PathBuf>,last:Option<u64>,disabled:bool}
fn hash(bytes:impl IntoIterator<Item=u8>,mut h:u64)->u64 {
    for b in bytes {h=(h^b as u64).wrapping_mul(0x100000001b3);}h
}
fn half(b:&[u8],o:usize)->u16 {u16::from_le_bytes([b[o],b[o+1]])}
impl Inspector {
    pub fn from_env()->Self {Self{path:std::env::var_os("FFTA_UI_REFS").map(PathBuf::from),..Self::default()}}
    pub fn observe(&mut self,mem:&MemMap,scene:&Scene) {
        if self.disabled || self.path.is_none(){return;}
        let (key,line)=reference_record(mem,scene);
        if self.last==Some(key){return;}
        let path=self.path.as_ref().unwrap();
        let result=(||->std::io::Result<()>{
            if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()){std::fs::create_dir_all(parent)?;}
            let mut f=std::fs::OpenOptions::new().create(true).append(true).open(path)?;
            writeln!(f,"{line}")
        })();
        match result {Ok(())=>self.last=Some(key),Err(e)=>{self.disabled=true;eprintln!("FFTA_UI_REFS disabled=true error={e}");}}
    }
}
fn reference_record(mem:&MemMap,scene:&Scene)->(u64,String) {
    use std::fmt::Write as _;
    let region=if mem.ffta_movement.europe_profile {"EUR"}else{"unknown"};
    let stage=gba_core::ffta_surface::Descriptor::read(mem).map(|s|s.id);
    let mut payload=format!("\"region\":\"{region}\",\"stage\":{},\"nodes\":[",stage.map_or("null".into(),|id|id.to_string()));
    let mut first=true;
    for n in scene.nodes.iter().flatten() {
        if !first {payload.push(',');}first=false;
        write!(payload,"{{\"id\":\"{}:{}\",\"slot\":{},\"role\":\"{}\",\"bounds\":{:?}}}",region,n.role.label(),n.slot,n.role.label(),n.bounds).unwrap();
    }
    payload.push_str("],\"planes\":[");
    for bg in [2usize,3] {
        if bg==3{payload.push(',');}
        let cnt=half(&mem.io,8+bg*2);
        let map=((cnt>>8)&31)as usize*0x800;
        let chars=((cnt>>2)&3)as usize*0x4000;
        write!(payload,"{{\"bg\":{bg},\"bgcnt\":{cnt},\"map_address\":{},\"character_address\":{},\"scroll\":[{},{}],\"coordinate_space\":\"unscrolled_bg\",\"glyph_candidates\":[",0x06000000+map,0x06000000+chars,half(&mem.io,0x10+bg*4)&511,half(&mem.io,0x12+bg*4)&511).unwrap();
        let mut first_tile=true;
        // Native BG2 glyph candidates. Blank tiles and all BG3 backing are
        // excluded; header/border tiles remain explicitly unclassified.
        if bg==2 && scene.nodes.iter().any(Option::is_some) {
            for y in 0..20 {for x in 0..30 {
                let entry=half(&mem.vram,map+(y*32+x)*2);
                let bpp8=cnt&128!=0;let bytes=if bpp8{64}else{32};
                let off=chars+(entry as usize&1023)*bytes;
                let Some(data)=mem.vram.get(off..off+bytes)else{continue;};
                if data.iter().all(|b|*b==0){continue;}
                if !first_tile{payload.push(',');}first_tile=false;
                let key=hash(data.iter().copied(),0xcbf29ce484222325);
                write!(payload,"{{\"x\":{},\"y\":{},\"entry\":{entry},\"tile_address\":{},\"content_hash\":\"{key:016x}\"}}",x*8,y*8,0x06000000+off).unwrap();
            }}
        }
        payload.push_str("]}");
    }
    payload.push_str("],\"objects\":[");
    let mut first=true;
    for object in gba_core::ffta_ui::object_references(mem,scene) {
        if !first{payload.push(',');}first=false;
        write!(payload,"{{\"oam_slot\":{},\"attrs\":{:?},\"position\":{:?},\"role\":\"{}\"}}",object.slot,object.attrs,object.position,object.role.label()).unwrap();
    }
    payload.push_str("],\"effect_planes\":[");
    let mut first=true;
    for bg in 0..4 {if let Some(fx)=gba_core::ffta_effects::layer(mem,bg) {
        if !first{payload.push(',');}first=false;
        write!(payload,"{{\"bg\":{},\"bgcnt\":{},\"map_address\":{},\"character_address\":{},\"affine\":{}}}",fx.bg,fx.control,fx.map,fx.characters,fx.affine).unwrap();
    }}
    // This intentionally does not claim a decoded text or a ROM string ID.
    payload.push_str("],\"decoded_text\":false,\"rom_string_address\":null");
    let key=hash(payload.bytes(),0xcbf29ce484222325);
    (key,format!("{{\"schema\":1,\"frame\":{},\"fingerprint\":\"{key:016x}\",{payload}}}",mem.frames))
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn references_track_tile_edits_without_claiming_decoded_text_or_exporting_bytes() {
        let mut mem=MemMap::new(vec![]);mem.ffta_movement.europe_profile=true;
        let mut scene=Scene::default();
        scene.nodes[0]=Some(gba_core::ffta_ui::Node{slot:0,role:gba_core::ffta_ui::Role::Dialogue,
            anchor:gba_core::ffta_ui::Anchor::Edge{right:false,vertical:0.},bounds:[0,0,240,64],length:true});
        mem.vram[0..2].copy_from_slice(&1u16.to_le_bytes());mem.vram[32]=0x11;
        let (a,line)=reference_record(&mem,&scene);println!("UI_REFS_TEST {line}");mem.frames+=1;
        assert_eq!(a,reference_record(&mem,&scene).0,"frame number is not content identity");
        assert!(line.contains("\"decoded_text\":false"));assert!(line.contains("\"content_hash\":"));
        mem.vram[32]=0x22;assert_ne!(a,reference_record(&mem,&scene).0);
        assert!(!line.contains("\"pixels\":"));assert!(!line.contains("\"rom_bytes\":"));
    }
}


/// Optional CPU timings. Present includes driver/vsync waiting, not GPU timestamps.
#[derive(Default)]
pub struct Timing {enabled:bool,frames:u64,emu:[f64;2],prepare:[f64;2],present_count:u64,present:[f64;2]}
impl Timing {
    pub fn from_env()->Self {Self{enabled:std::env::var("FFTA_UI_TIMING").as_deref()==Ok("1"),..Self::default()}}
    pub fn start(&self)->Option<std::time::Instant>{self.enabled.then(std::time::Instant::now)}
    pub fn prepared(&mut self,start:Option<std::time::Instant>,emulated:Option<std::time::Instant>,frame:u64,nodes:usize) {
        let (Some(start),Some(mid))=(start,emulated)else{return;};
        let emu=mid.duration_since(start).as_secs_f64()*1000.;
        let prep=mid.elapsed().as_secs_f64()*1000.;
        self.frames+=1;self.emu[0]+=emu;self.emu[1]=self.emu[1].max(emu);
        self.prepare[0]+=prep;self.prepare[1]=self.prepare[1].max(prep);
        if self.frames==120 {
            eprintln!("FFTA_UI_CPU frame={frame} panels={nodes} samples=120 emu_ppu_mean_ms={:.3} emu_ppu_max_ms={:.3} prepare_mean_ms={:.3} prepare_max_ms={:.3}",
                self.emu[0]/120.,self.emu[1],self.prepare[0]/120.,self.prepare[1]);
            self.frames=0;self.emu=[0.;2];self.prepare=[0.;2];
        }
    }
    pub fn presented(&mut self,start:Option<std::time::Instant>) {
        let Some(start)=start else{return;};let ms=start.elapsed().as_secs_f64()*1000.;
        self.present_count+=1;self.present[0]+=ms;self.present[1]=self.present[1].max(ms);
        if self.present_count==120 {
            eprintln!("FFTA_UI_PRESENT samples=120 cpu_and_wait_mean_ms={:.3} cpu_and_wait_max_ms={:.3} gpu_timestamp=false",self.present[0]/120.,self.present[1]);
            self.present_count=0;self.present=[0.;2];
        }
    }
}
