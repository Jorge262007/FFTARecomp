//! Fixed-resolution battle HUD groups, independent of camera zoom.
use crate::{MemMap,ffta_dialogue::UiPixel};
pub const LAYERS:usize=4;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Kind { Left, Right, Start, Preview, PreviewBackdrop, Arrow, Dispatch }
impl Kind {fn layer(self)->usize {match self {
    Self::Left|Self::Right|Self::Arrow=>0,Self::Preview|Self::PreviewBackdrop=>1,
    Self::Start=>2,Self::Dispatch=>3,
}}}
fn kind(a:[u16;3])->Option<Kind>{
    let [a0,_,a2]=a;
    if a0&0x0300!=0 || a0>>14==3{return None;}
    if a2&0xfc00==0 && matches!(a2,0x31c|0x320|0x324){return Some(Kind::Start);}
    if a2==0x2f8{return Some(Kind::Left);}
    if a2==0x2f0{return Some(Kind::Right);}
    if a2==0x12e0{return Some(Kind::Arrow);}
    if matches!(a2,0x300|0x308|0x30c){return Some(Kind::Dispatch);}
    // Reserved preview allocation; palette and bit depth vary between troops.
    // Field actors have a different tile allocation and/or priority.
    if a2&0x0c00==0 && (0x80..0x120).contains(&(a2&1023)) {return Some(Kind::Preview);}
    if matches!(a2,0xc2e4|0xc2ec){return Some(Kind::Preview);}
    if matches!(a2,0xb5f0|0xb600) && a0&0x0c00==0x0400 {return Some(Kind::PreviewBackdrop);}
    None
}
pub(crate) fn owns(mem:&MemMap,a:[u16;3])->bool {
    !mem.ffta_dialogue.naming_layout && mem.ffta_dialogue.battle_hud_overlay && crate::ppu::ffta_terrain(mem,0) && kind(a).is_some()
}
const SIZES:[[(i32,i32);4];3]=[
    [(8,8),(16,16),(32,32),(64,64)],[(16,8),(32,8),(32,16),(64,32)],[(8,16),(8,32),(16,32),(32,64)]];
#[derive(Clone,Copy)]
struct Piece {a:[u16;3],kind:Kind,x:i32,y:i32}
impl Piece {
    fn rect(self)->[i32;4] {let(w,h)=SIZES[(self.a[0]>>14)as usize][(self.a[1]>>14)as usize];[self.x,self.y,self.x+w,self.y+h]}
}
fn union(a:[i32;4],b:[i32;4])->[i32;4]{[a[0].min(b[0]),a[1].min(b[1]),a[2].max(b[2]),a[3].max(b[3])]}
/// Complete native canvases. Host motion owns reveal/retraction; guest slide
/// coordinates only establish relative positions within each group.
pub fn capture_layers(mem:&MemMap,out:&mut [Vec<[u8;4]>;LAYERS]) {
    if mem.ffta_dialogue.naming_layout || !mem.ffta_dialogue.battle_hud_overlay || !crate::ppu::ffta_terrain(mem,0)
        || mem.ffta_dialogue.battle_notice() {for o in out{o.clear();}return;}
    let host=mem.ffta_sprites.active.as_deref().unwrap_or(&[]);
    let mut pieces=Vec::with_capacity(32);
    let mut push=|a:[u16;3],coordinates:Option<(i32,i32)>|{
        if let Some(kind)=kind(a) {
            let (x,y)=coordinates.unwrap_or_else(||{let x=(a[1]&511)as i32;let y=(a[0]&255)as i32;
                (if x>=256{x-512}else{x},if y>=160{y-256}else{y})});
            pieces.push(Piece{a,kind,x,y});
        }
    };
    for slot in 0..128 {
        let mut replaced=false;
        for s in host.iter().filter(|s|s.slot==slot) {replaced|=s.replaces;push(s.attrs,Some((s.x,s.y)));}
        if !replaced {let o=&mem.oam[slot*8..slot*8+6];
            push(std::array::from_fn(|i|u16::from_le_bytes([o[i*2],o[i*2+1]])),None);}
    }
    for s in host.iter().filter(|s|s.slot>=128){push(s.attrs,Some((s.x,s.y)));}
    let bounds=|kind|pieces.iter().filter(|p|p.kind==kind).map(|p|p.rect()).reduce(union);
    let start=bounds(Kind::Start);let preview=bounds(Kind::PreviewBackdrop);let dispatch=bounds(Kind::Dispatch);
    let mut used=[false;LAYERS];for p in &pieces{used[p.kind.layer()]=true;}
    for (o,used) in out.iter_mut().zip(used){if used{o.resize(480*160,UiPixel::default().packed());o.fill(UiPixel::default().packed());}else{o.clear();}}
    pieces.sort_by_key(|p|(p.a[2]>>10)&3);
    for p in pieces {
        let a=p.a;let k=p.kind;let [x,y,r,b]=p.rect();let(w,h)=(r-x,b-y);
        let(dx,dy)=match k {
            Kind::Start=>{let q=start.unwrap();(120-(q[0]+q[2])/2,16-q[1])},
            Kind::Preview|Kind::PreviewBackdrop=>preview.map_or((0,0),|q|(172-q[0],104-q[1])),
            Kind::Dispatch=>{let q=dispatch.unwrap();(179-q[0],128-q[1])},
            _=>(0,0),
        };
        let eight=a[0]&0x2000!=0;let tile_step=if eight{2}else{1};
        let stride=if u16::from_le_bytes([mem.io[0],mem.io[1]])&64!=0{w as usize/8*tile_step}else{32};
        let output=&mut out[k.layer()];
        for v in 0..h {for u in 0..w {
            let tx=x+dx+u+120;let ty=y+dy+v;
            if !(0..480).contains(&tx)||!(0..160).contains(&ty){continue;}
            let su=if a[1]&0x1000!=0{w-1-u}else{u}as usize;
            let sv=if a[1]&0x2000!=0{h-1-v}else{v}as usize;
            let tile=(a[2]as usize&1023)+(sv/8)*stride+(su/8)*tile_step;
            let addr=0x10000+if eight{tile/2*64+(sv%8)*8+su%8}else{tile*32+(sv%8)*4+su%8/2};
            let Some(&byte)=mem.vram.get(addr)else{continue;};
            let index=if eight{byte}else{(byte>>((su%2)*4))&15};if index==0{continue;}
            let pal=0x200+if eight{index as usize*2}else{(a[2]as usize>>12)*32+index as usize*2};
            let color=u16::from_le_bytes([mem.palette[pal],mem.palette[pal+1]]);
            let dst=&mut output[ty as usize*480+tx as usize];
            if dst[2]==0 {*dst=if k==Kind::PreviewBackdrop {
                let alpha=u16::from_le_bytes([mem.io[0x52],mem.io[0x53]]);
                UiPixel{color,eva:(alpha as u8&31).min(16),evb:((alpha>>8)as u8&31).min(16)}.packed()
            }else{UiPixel::opaque(color).packed()};}
        }}
    }
}
/// Combined canvas retained for diagnostics.
pub fn capture(mem:&MemMap,out:&mut Vec<[u8;4]>) {
    let mut layers=std::array::from_fn(|_|Vec::new());capture_layers(mem,&mut layers);
    if layers.iter().all(Vec::is_empty){out.clear();return;}
    out.resize(480*160,UiPixel::default().packed());out.fill(UiPixel::default().packed());
    for layer in layers {for (dst,p) in out.iter_mut().zip(layer){if p[2]!=0{*dst=p;}}}
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scene()->MemMap {
        let mut m=MemMap::new(vec![]);
        m.ffta_dialogue.separate=true;m.ffta_dialogue.battle_hud_overlay=true;
        for (o,v) in [(0,0x1340u16),(8,0x4c03),(10,0x4e02)] {
            m.io[o..o+2].copy_from_slice(&v.to_le_bytes());
        }
        for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}
        for (slot,a) in [[0x4010u16,0x8011,0x02f8],[0x4010,0x80c7,0x02f0],
            [0x40a8,0x8060,0x031c],[0x9080,0x00c4,0x0080],[0x4080,0x80bc,0xc2e4],[0x0468,0x80ac,0xb5f0]].into_iter().enumerate() {
            for (i,v) in a.into_iter().enumerate(){m.oam[slot*8+i*2..slot*8+i*2+2].copy_from_slice(&v.to_le_bytes());}
            let base=0x10000+(a[2]as usize&1023)*32;m.vram[base..base+128].fill(0x11);
            let bank=(a[2]as usize>>12)*32;m.palette[0x202+bank..0x204+bank].copy_from_slice(&31u16.to_le_bytes());
        }
        m.io[0x52]=14;m.io[0x53]=9;
        m
    }
    #[test]
    fn complete_hud_canvas_is_identical_at_every_zoom() {
        let mut m=scene();let mut expected=Vec::new();capture(&m,&mut expected);
        assert!(expected[16*480+120+104][2]!=0,"start label centered inside native top row");
        assert!(expected[128*480+120+188][2]!=0,"circular base included");
        assert!(expected[128*480+120+196][2]!=0,"walking miniature included");
        assert_eq!(&expected[104*480+120+172][2..],&[14,9],"blue oval keeps guest transparency");
        assert!(owns(&m,[0x0468,0x80ac,0xb5f0]));
        assert!(!owns(&m,[0x0068,0x80ac,0xb1f0]),"do not steal the target-stat atlas");
        for step in 0..=6 {
            m.ffta_dialogue.view_zoom=1.122462f32.powi(step);
            let mut actual=Vec::new();capture(&m,&mut actual);assert_eq!(actual,expected);
        }
        m.io[8]=0;capture(&m,&mut expected);assert!(expected.is_empty());
    }
    #[test]
    fn world_actor_atlas_is_not_owned_by_hud() {
        let m=scene();assert!(!owns(&m,[0x806e,0x00cf,0x0800]));
        assert!(owns(&m,[0x4010,0x8011,0x02f8]));
    }
}

#[cfg(test)]
mod complete_hud_tests {
    use super::*;
    fn scene()->MemMap {
        let mut m=MemMap::new(vec![]);m.ffta_dialogue.battle_hud_overlay=true;
        for(o,v)in[(0,0x1340u16),(8,0x4c03),(10,0x4e02)]{m.io[o..o+2].copy_from_slice(&v.to_le_bytes());}
        for o in m.oam.chunks_exact_mut(8){o[..2].copy_from_slice(&0x200u16.to_le_bytes());}m
    }
    fn sprite(m:&mut MemMap,slot:usize,a:[u16;3]) {
        for(i,v)in a.into_iter().enumerate(){m.oam[slot*8+i*2..slot*8+i*2+2].copy_from_slice(&v.to_le_bytes());}
    }
    #[test]
    fn previews_support_other_palettes_and_eight_bit_tiles() {
        let mut m=scene();sprite(&mut m,0,[0x0080,0x00c4,0x20a0]);
        m.vram[0x11400..0x11420].fill(0x11);m.palette[0x242..0x244].copy_from_slice(&31u16.to_le_bytes());
        let mut layers=std::array::from_fn(|_|Vec::new());capture_layers(&m,&mut layers);
        assert_eq!(layers[1][128*480+316],UiPixel::opaque(31).packed());
        sprite(&mut m,0,[0x2080,0x00c4,0x00a0]);m.vram[0x11400..0x11440].fill(3);
        m.palette[0x206..0x208].copy_from_slice(&992u16.to_le_bytes());capture_layers(&m,&mut layers);
        assert_eq!(layers[1][128*480+316],UiPixel::opaque(992).packed());
        assert!(!owns(&m,[0x0080,0x00c4,0x28a0]),"field actor priority must remain in world");
    }
    #[test]
    fn dispatch_and_arrows_have_independent_canvases() {
        let mut m=scene();sprite(&mut m,0,[0x0080,0x4005,0x12e0]);
        sprite(&mut m,1,[0x4080,0x80b3,0x300]);sprite(&mut m,2,[0x0080,0x40d3,0x308]);
        sprite(&mut m,3,[0x8080,0x00e3,0x30c]);
        m.vram[0x15c00..0x15c80].fill(0x11);m.vram[0x16000..0x16200].fill(0x11);
        m.palette[0x222..0x224].copy_from_slice(&31u16.to_le_bytes());m.palette[0x202..0x204].copy_from_slice(&31u16.to_le_bytes());
        let mut layers=std::array::from_fn(|_|Vec::new());capture_layers(&m,&mut layers);
        assert!(!layers[0].is_empty());assert!(!layers[3].is_empty());assert!(layers[1].is_empty());
        assert!(layers[3][128*480+120+179][2]!=0,"original Dispatch origin is (179,128)");
        assert!(layers[3][144*480..].iter().all(|p|p[2]==0),"preserve original sixteen-pixel bottom margin");
    }
    #[test]
    fn native_slide_does_not_change_preview_canvas() {
        use crate::ffta_sprites::HostSprite;
        let mut m=scene();
        m.ffta_sprites.active=Some(vec![
            HostSprite{attrs:[0x0080,0x00c4,0x0080],x:196,y:128,slot:0,replaces:true,..Default::default()},
            HostSprite{attrs:[0x0468,0x80ac,0xb5f0],x:172,y:104,slot:1,replaces:true,..Default::default()},
        ]);
        m.vram[0x11000..0x11020].fill(0x11);m.vram[0x13e00..0x13e80].fill(0x11);
        m.palette[0x202..0x204].copy_from_slice(&31u16.to_le_bytes());m.io[0x52]=14;m.io[0x53]=9;
        let mut layers=std::array::from_fn(|_|Vec::new());capture_layers(&m,&mut layers);let expected=layers[1].clone();
        for p in m.ffta_sprites.active.as_mut().unwrap(){p.x+=91;p.y+=24;}
        capture_layers(&m,&mut layers);assert_eq!(layers[1],expected);
    }
}
