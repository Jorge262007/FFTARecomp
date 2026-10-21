//! Optional host view of calculated terrain planes, inferred from their DMA.
//! Guest writes and the existing terrain-edge policy remain unchanged.
use crate::mem::MemMap;
#[derive(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StageState {
    pub enabled: bool,
    pub bases: [Option<u32>; 2],
    pub origins: [Option<(i32, i32)>; 2],
    pub valid: [bool; 2],
}
impl StageState {
    pub fn new(rom: &[u8]) -> Self {
        Self {
            enabled: std::env::var("FFTA_PATCH_STAGE").as_deref() != Ok("0")
                && std::env::var("FFTA_PATCH_BG").as_deref() != Ok("0")
                && crate::ffta_rom::map_producers_verified(rom),
            ..Self::default()
        }
    }
    pub fn observe_dma(
        &mut self,
        src: u32,
        dst: u32,
        count: u32,
        unit: u32,
        control: u16,
        ram: &[u8],
        vram: &[u8],
    ) {
        if !self.enabled {
            return;
        }
        for bg in 0..2 {
            let start = 0x06006000 + bg as u32 * 0x1000;
            if dst < start + 0x1000 && dst.saturating_add(count * unit) > start {
                self.valid[bg] = false;
            }
        }
        if unit != 2 || count == 0 || count > 32 || control & 0x01e0 != 0 {
            return;
        }
        let half = |off| u16::from_le_bytes([ram[off], ram[off + 1]]);
        let word = |off| u32::from_le_bytes(ram[off..off + 4].try_into().unwrap());
        if half(0x7f48) != 512
            || half(0x7f4a) != 256
            || word(0x7f54) != 0x06007000
            || word(0x7f58) != 0x06006000
        {
            return;
        }
        let origin = (
            half(0x7f68) as i16 as i32 >> 3,
            half(0x7f6a) as i16 as i32 >> 3,
        );
        for bg in 0..2 {
            let target = 0x06006000 + bg as u32 * 0x1000;
            if !(target..target + 0x1000).contains(&dst) {
                continue;
            }
            let di = (dst - target) / 2;
            let x = (di / 1024) * 32 + di % 32;
            let y = (di % 1024) / 32;
            let offset = 2 * ((y as i32 + origin.1) * 64 + x as i32 + origin.0);
            let base = src as i64 - offset as i64;
            if base < 0x02000000 || base + 0x2000 > 0x02040000 || base & 1 != 0 {
                return;
            }
            self.bases[bg] = Some(base as u32);
            self.origins[bg] = Some(origin);
            self.valid[bg] = (0..32).all(|y| {
                (0..64).all(|x| {
                    let sx = x + origin.0;
                    let sy = y + origin.1;
                    let expected = if (0..64).contains(&sx) && (0..64).contains(&sy) {
                        let offset =
                            (base - 0x02000000) as usize + 2 * (sy as usize * 64 + sx as usize);
                        half(offset)
                    } else {
                        0
                    };
                    let offset = (target - 0x06000000) as usize
                        + 2 * ((x as usize / 32) * 1024 + y as usize * 32 + x as usize % 32);
                    expected == u16::from_le_bytes([vram[offset], vram[offset + 1]])
                })
            });
        }
    }
}
pub fn entry(mem: &MemMap, bg: usize, x: i32, y: i32) -> Option<u16> {
    let index = layer(mem, bg)?;
    let (ox, oy) = mem.ffta_stage.origins[index]?;
    let base = mem.ffta_stage.bases[index]?;
    let x = x.div_euclid(8) + ox;
    let y = y.div_euclid(8) + oy;
    if !(0..64).contains(&x) || !(0..64).contains(&y) {
        return None;
    }
    let offset = (base - 0x02000000) as usize + 2 * (y as usize * 64 + x as usize);
    Some(u16::from_le_bytes([
        mem.ewram[offset],
        mem.ewram[offset + 1],
    ]))
}
/// A verified terrain plane follows its map assignment, not its BG slot.
/// Menus may hide one plane or assign the terrain pair to BG0/BG1.
pub fn layer(mem: &MemMap, bg: usize) -> Option<usize> {
    if bg > 3 || !mem.ffta_stage.enabled || mem.io[0] & 7 != 0 {
        return None;
    }
    let off = 8 + bg * 2;
    let cnt = u16::from_le_bytes([mem.io[off], mem.io[off + 1]]);
    let index = match cnt {
        0x4c03 => 0,
        0x4e02 => 1,
        _ => return None,
    };
    if !mem.ffta_stage.valid[index]
        || mem.ffta_stage.bases[index].is_none()
        || mem.ffta_stage.origins[index].is_none()
    {
        return None;
    }
    Some(index)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn infer_source_from_second_screen_block_and_verify_crop() {
        let mut s = StageState {
            enabled: true,
            ..StageState::default()
        };
        let mut ram = vec![0; 0x40000];
        let mut vram = vec![0; 0x18000];
        for (off, value) in [
            (0x7f48, 512u16),
            (0x7f4a, 256),
            (0x7f68, 120),
            (0x7f6a, 280),
        ] {
            ram[off..off + 2].copy_from_slice(&value.to_le_bytes());
        }
        ram[0x7f54..0x7f58].copy_from_slice(&0x06007000u32.to_le_bytes());
        ram[0x7f58..0x7f5c].copy_from_slice(&0x06006000u32.to_le_bytes());
        ram[0xb1a0 + 2 * (35 * 64 + 47)..0xb1a0 + 2 * (35 * 64 + 47) + 2]
            .copy_from_slice(&7u16.to_le_bytes());
        vram[0x6800..0x6802].copy_from_slice(&7u16.to_le_bytes());
        s.observe_dma(
            0x0200b1a0 + 2 * (35 * 64 + 47),
            0x06006800,
            17,
            2,
            0x8000,
            &ram,
            &vram,
        );
        assert_eq!(s.bases[0], Some(0x0200b1a0));
        assert_eq!(s.origins[0], Some((15, 35)));
        assert!(s.valid[0]);
        vram[0x6000] = 1;
        s.observe_dma(
            0x0200b1a0 + 2 * (35 * 64 + 47),
            0x06006800,
            17,
            2,
            0x8000,
            &ram,
            &vram,
        );
        assert!(!s.valid[0]);
    }
}
