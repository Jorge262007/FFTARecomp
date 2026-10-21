//! Host view of the movement planes. Never changes guest RAM, VRAM or timing.
use crate::mem::MemMap;

#[derive(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MovementState {
    pub enabled: bool,
    pub europe_profile: bool,
    pub origins: [Option<(i32, i32)>; 2],
    pub valid: [bool; 2],
    pub zoom_output: bool,
    pub sky_edges: [u16; 2],
}
impl MovementState {
    pub fn new(rom: &[u8]) -> Self {
        let enabled = std::env::var("FFTA_PATCH_BG").as_deref() != Ok("0")
            && std::env::var("FFTA_PATCH_MOVEMENT").as_deref() != Ok("0")
            && crate::ffta_rom::map_producers_verified(rom);
        Self {
            enabled,
            europe_profile: crate::ffta_rom::version(rom) == Some(crate::ffta_rom::Version::Europe),
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
        ewram: &[u8],
        vram: &[u8],
    ) {
        if self.enabled {
            for bg in 0..2 {
                let target = 0x06005000 + bg as u32 * 0x800;
                if dst < target + 0x800 && dst.saturating_add(count * unit) > target {
                    self.valid[bg] = false;
                }
            }
        }
        if !self.enabled || unit != 2 || count == 0 || count > 32 || control & 0x01e0 != 0 {
            return;
        }
        for bg in 0..2 {
            let source = 0x02003cb0 + bg as u32 * 0x2000;
            let target = 0x06005800 - bg as u32 * 0x800;
            if !(source..source + 0x2000).contains(&src) || !(target..target + 0x800).contains(&dst)
            {
                continue;
            }
            let si = (src - source) / 2;
            let di = (dst - target) / 2;
            let origin = (
                (si % 64) as i32 - (di % 32) as i32,
                (si / 64) as i32 - (di / 32) as i32,
            );
            let index = 1 - bg;
            self.origins[index] = Some(origin);
            // Verify every original map word against the expected RAM crop.
            // Mismatches leave the existing PPU path active.
            self.valid[index] = (0..32).all(|y| {
                (0..32).all(|x| {
                    let sx = x + origin.0;
                    let sy = y + origin.1;
                    let expected = if (0..64).contains(&sx) && (0..64).contains(&sy) {
                        let off =
                            (source - 0x02000000) as usize + 2 * (sy as usize * 64 + sx as usize);
                        u16::from_le_bytes([ewram[off], ewram[off + 1]])
                    } else {
                        0
                    };
                    let off = (target - 0x06000000) as usize + 2 * (y as usize * 32 + x as usize);
                    expected == u16::from_le_bytes([vram[off], vram[off + 1]])
                })
            });
        }
    }
}

/// Return the actual calculated entry, including outside the 256-pixel crop.
pub fn entry(mem: &MemMap, bg: usize, x: i32, y: i32) -> Option<u16> {
    if bg > 1 || !mem.ffta_movement.enabled || !mem.ffta_movement.valid[bg] {
        return None;
    }
    let (ox, oy) = mem.ffta_movement.origins[bg]?;
    let tx = x.div_euclid(8) + ox;
    let ty = y.div_euclid(8) + oy;
    if !(0..64).contains(&tx) || !(0..64).contains(&ty) {
        return Some(0);
    }
    let base = if bg == 0 { 0x5cb0 } else { 0x3cb0 };
    let off = base + 2 * (ty as usize * 64 + tx as usize);
    Some(u16::from_le_bytes([mem.ewram[off], mem.ewram[off + 1]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crop_validation_requires_every_original_word() {
        let mut state = MovementState {
            enabled: true,
            ..MovementState::default()
        };
        let mut ram = vec![0; 0x40000];
        let mut vram = vec![0; 0x18000];
        ram[0x5cb0 + 2 * (35 * 64 + 33)..0x5cb0 + 2 * (35 * 64 + 33) + 2]
            .copy_from_slice(&0xa094u16.to_le_bytes());
        vram[0x5000..0x5002].copy_from_slice(&0xa094u16.to_le_bytes());
        state.observe_dma(
            0x02005cb0 + 2 * (35 * 64 + 33),
            0x06005000,
            31,
            2,
            0x8000,
            &ram,
            &vram,
        );
        assert_eq!(state.origins[0], Some((33, 35)));
        assert!(state.valid[0]);
        vram[0x5002] = 1;
        state.observe_dma(
            0x02005cb0 + 2 * (35 * 64 + 33),
            0x06005000,
            31,
            2,
            0x8000,
            &ram,
            &vram,
        );
        assert!(!state.valid[0]);
    }
}
