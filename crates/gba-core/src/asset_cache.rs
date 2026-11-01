//! Original VRAM asset cache, separate from any final-frame presentation cache.
//! Decodes 4/8-bpp 8x8 assets into 16x16 RGB555 textures without interpolation.
//! The native compositor samples those textures at native coordinates; display
//! replication preserves the same texels at 2x. No new visual detail is invented.
use std::{
    cell::RefCell,
    sync::{Mutex, OnceLock},
};
const EMPTY: u16 = 0xffff;
const SLOTS: usize = 1024;
struct Asset {
    source: Vec<u8>,
    palette: Vec<u8>,
    scaled: [u16; 256],
}
struct Cache {
    slots: Vec<Option<Box<Asset>>>,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            slots: (0..SLOTS).map(|_| None).collect(),
        }
    }
}
// Some PPU builds use short-lived zoom workers. Return their caches to a
// bounded pool instead of allocating/decoding all assets again every frame.
fn pool() -> &'static Mutex<Vec<Cache>> {
    static POOL: OnceLock<Mutex<Vec<Cache>>> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(Vec::new()))
}
struct Local(Option<Cache>);
impl Default for Local {
    fn default() -> Self {
        Self(Some(
            pool()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pop()
                .unwrap_or_default(),
        ))
    }
}
impl Drop for Local {
    fn drop(&mut self) {
        if let Some(cache) = self.0.take() {
            let mut pool = pool().lock().unwrap_or_else(|e| e.into_inner());
            if pool.len() < 4 {
                pool.push(cache);
            }
        }
    }
}
thread_local! { static CACHE:RefCell<Local> = RefCell::new(Local::default()); }
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let on = match std::env::var("FFTA_ASSET_CACHE").as_deref() {
            Ok("0") => false,
            Ok("1") => true,
            _ => false,
        };
        if on {
            eprintln!("video: VRAM asset cache enabled (8x8 -> 16x16, RAM, no interpolation)");
        }
        on
    })
}
impl Cache {
    fn row(
        &mut self,
        vram: &[u8],
        pal: &[u8],
        base: usize,
        bpp8: bool,
        bank: usize,
        y: usize,
    ) -> [u16; 8] {
        let source = &vram[base..base + if bpp8 { 64 } else { 32 }];
        let colors = &pal[bank * 2..bank * 2 + if bpp8 { 512 } else { 32 }];
        let slot = (base / 32 + bank * 31 + usize::from(bpp8) * 17) % SLOTS;
        let asset = &mut self.slots[slot];
        if !asset
            .as_ref()
            .is_some_and(|a| a.source == source && a.palette == colors[2..])
        {
            let a = asset.get_or_insert_with(|| {
                Box::new(Asset {
                    source: Vec::new(),
                    palette: Vec::new(),
                    scaled: [EMPTY; 256],
                })
            });
            a.source.clear();
            a.source.extend_from_slice(source);
            a.palette.clear();
            // Index zero is transparent, not an asset color. The sky can
            // animate this entry each scanline without invalidating textures.
            a.palette.extend_from_slice(&colors[2..]);
            for yy in 0..8 {
                for x in 0..8 {
                    let index = if bpp8 {
                        source[yy * 8 + x] as usize
                    } else {
                        ((source[yy * 4 + x / 2] >> (4 * (x % 2))) & 15) as usize
                    };
                    let color = if index == 0 {
                        EMPTY
                    } else {
                        u16::from_le_bytes([colors[index * 2], colors[index * 2 + 1]]) & 0x7fff
                    };
                    let out = yy * 32 + x * 2;
                    for off in [0, 1, 16, 17] {
                        a.scaled[out + off] = color;
                    }
                }
            }
        }
        let image = &asset.as_ref().unwrap().scaled;
        std::array::from_fn(|x| image[(y * 2) * 16 + x * 2])
    }
}
/// `bank` is the first palette index: BG=0..240, OBJ=256..496;
/// 8-bpp callers pass 0 or 256. Fully validate live content on every tile use.
pub fn row(vram: &[u8], pal: &[u8], base: usize, bpp8: bool, bank: usize, y: usize) -> [u16; 8] {
    CACHE.with(|cache| {
        cache
            .borrow_mut()
            .0
            .as_mut()
            .unwrap()
            .row(vram, pal, base, bpp8, bank, y)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_assets_preserve_transparency_and_rebuild_after_vram_and_palette_edits() {
        let mut cache = Cache::default();
        let mut vram = vec![0u8; 0x18000];
        let mut palette = vec![0; 1024];
        palette[2..4].copy_from_slice(&0x001fu16.to_le_bytes());
        vram[0] = 0x11;
        assert_eq!(
            &cache.row(&vram, &palette, 0, false, 0, 0)[..3],
            &[31, 31, EMPTY]
        );
        assert_eq!(cache.slots[0].as_ref().unwrap().scaled[0..2], [31, 31]);
        assert_eq!(cache.slots[0].as_ref().unwrap().scaled[16..18], [31, 31]);
        palette[2..4].copy_from_slice(&0x03e0u16.to_le_bytes());
        assert_eq!(cache.row(&vram, &palette, 0, false, 0, 0)[0], 0x03e0);
        vram[0] = 0;
        assert_eq!(cache.row(&vram, &palette, 0, false, 0, 0)[0], EMPTY);
        vram[0x10000] = 1;
        palette[514..516].copy_from_slice(&0x7c00u16.to_le_bytes());
        assert_eq!(cache.row(&vram, &palette, 0x10000, true, 256, 0)[0], 0x7c00);
        assert_eq!(cache.row(&vram, &palette, 0x10000, true, 256, 0)[1], EMPTY);
    }
}
