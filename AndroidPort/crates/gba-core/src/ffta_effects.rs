//! Effect planes are separate from terrain, movement grids and known UI maps.
//! This describes rendering resources, not a decoded spell ID.
use crate::MemMap;
#[derive(Clone,Copy,Debug)]
pub struct Layer {pub bg:usize,pub control:u16,pub map:u32,pub characters:u32,pub affine:bool}
/// Recorded Roulette reaper: BG3 borrows the dialogue map, with palette6
/// picture tiles 3A0..3FE and transparent tile3FF filling the rest. No text
/// or frame tiles are present. Test once per frame, not in scanline sampling.
pub(crate) fn shared_dialogue_picture(mem:&MemMap,bg:usize)->bool {
    let h=|o|u16::from_le_bytes([mem.io[o],mem.io[o+1]]);
    if bg!=3 || !mem.ffta_movement.europe_profile || !mem.ffta_movement.enabled
        || h(0)&0x0c07!=0x0800 || !crate::ppu::ffta_terrain(mem,0)
        || (h(12),h(14))!=(0x0b08,0x0a08){return false;}
    let mut count=0;
    for bytes in mem.vram[0x5000..0x5800].chunks_exact(2) {
        let entry=u16::from_le_bytes([bytes[0],bytes[1]]);
        if entry==0x03ff{continue;}
        if entry>>12!=6 || !(0x3a0..0x3ff).contains(&(entry&1023)){return false;}
        count+=1;
    }
    (16..=128).contains(&count)
}
/// The original screen owns all planes except reconstructed terrain, movement
/// and the finite world map. Unknown spell scripts can reuse UI resources;
/// their wrapping must never become extra copies in the host margins.
pub fn viewport_local(mem:&MemMap,bg:usize)->bool {
    if bg==0 && mem.ffta_dialogue.menu_pattern{return false;}
    if bg>=4 || !mem.ffta_movement.enabled || !mem.ffta_movement.europe_profile
        || !mem.ffta_dialogue.separate {return layer(mem,bg).is_some();}
    let h=|o|u16::from_le_bytes([mem.io[o],mem.io[o+1]]);
    for first in [0usize,2] {
        if (bg==first || bg==first+1) && h(8+first*2)==0x4c03 && h(10+first*2)==0x4e02{return false;}
    }
    if bg<2 && h(8)==0xaa03 && h(10)==0xac02{return false;}
    if crate::ppu::ffta_movement_grid(mem,bg) || crate::ffta_stage::layer(mem,bg).is_some(){return false;}
    true
}
pub fn layer(mem:&MemMap,bg:usize)->Option<Layer> {
    if bg>=4 || !mem.ffta_movement.europe_profile || !mem.ffta_movement.enabled{return None;}
    let h=|o|u16::from_le_bytes([mem.io[o],mem.io[o+1]]);
    let display=h(0);let mode=display&7;
    if mode>2 || display&0x80!=0 || display&(0x100<<bg)==0{return None;}
    let handler=u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().ok()?);
    // Spell scripts can change the active callback without changing the
    // loaded battle. A verified stage/terrain pair remains authoritative.
    let stage=crate::ffta_surface::Descriptor::read(mem);
    let battle=handler==0x08025115 || stage.is_some_and(|s|!matches!(s.id,62..=67));
    if !battle{return None;}
    let cnt=h(8+bg*2);
    if matches!(cnt&!3,0x4c00|0x4e00|0x0b08|0x0a08|0x5c08|0x4a08)
            && !shared_dialogue_picture(mem,bg)
        || crate::ppu::ffta_terrain(mem,bg) {return None;}
    Some(Layer{bg,control:cnt,map:0x06000000+(((cnt>>8)&31)as u32)*0x800,
        characters:0x06000000+(((cnt>>2)&3)as u32)*0x4000,
        affine:mode==2 || mode==1 && bg==2})
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn terrain_ui_and_nonbattle_never_become_effects() {
        let mut m=MemMap::new(vec![]);m.ffta_movement.europe_profile=true;m.ffta_movement.enabled=true;
        m.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
        m.iwram[0x2804..0x2808].copy_from_slice(&0x08025115u32.to_le_bytes());
        for cnt in [0x4c03u16,0x4e02,0x0b08,0x0a0b,0x5c08,0x4a09] {
            m.io[12..14].copy_from_slice(&cnt.to_le_bytes());assert!(layer(&m,2).is_none());
        }
        m.io[12..14].copy_from_slice(&0x1c84u16.to_le_bytes());assert!(layer(&m,2).is_some());
        m.iwram[0x2804..0x2808].fill(0);assert!(layer(&m,2).is_none());
    }
}
