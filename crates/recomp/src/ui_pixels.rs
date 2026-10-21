//! Cache the inputs to pure UI packing, not a second copy of its output.
use gba_core::ffta_dialogue::{pack_panel, UiPixel};
use std::ops::Deref;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Packing {
    Raw,
    Panel { length_bits: u32, right: bool },
}

#[derive(Default)]
pub(crate) struct Pixels {
    pixels: Vec<[u8; 4]>,
    source: Vec<u32>,
    backing: Vec<u32>,
    packing: Option<Packing>,
    revision: Option<u64>,
    #[cfg(test)]
    packs: u64,
}

// This is a bijection over all UiPixel fields, not a hash. Explicit packing
// avoids relying on the Rust struct layout, alignment or host endianness.
fn input_word(UiPixel { color, eva, evb }: UiPixel) -> u32 {
    // Exhaustive destructuring makes new UiPixel fields a compile-time review.
    u32::from(color) | (u32::from(eva) << 16) | (u32::from(evb) << 24)
}

fn same_input(snapshot: &[u32], current: &[UiPixel]) -> bool {
    snapshot.len() == current.len()
        && snapshot
            .iter()
            .zip(current)
            .all(|(&old, pixel)| old == input_word(*pixel))
}

impl Pixels {
    pub fn raw(&mut self, source: &[UiPixel]) {
        self.update(source, &[], Packing::Raw);
    }

    pub fn panel(&mut self, source: &[UiPixel], backing: &[UiPixel], length: f32, right: bool) {
        self.update(
            source,
            backing,
            Packing::Panel {
                length_bits: length.to_bits(),
                right,
            },
        );
    }

    fn update(&mut self, source: &[UiPixel], backing: &[UiPixel], packing: Packing) {
        if self.packing == Some(packing)
            && same_input(&self.source, source)
            && same_input(&self.backing, backing)
        {
            return;
        }
        match packing {
            Packing::Raw => {
                self.pixels.resize(source.len(), [0; 4]);
                for (out, pixel) in self.pixels.iter_mut().zip(source) {
                    *out = pixel.packed();
                }
            }
            Packing::Panel { length_bits, right } => {
                pack_panel(
                    source,
                    backing,
                    f32::from_bits(length_bits),
                    right,
                    &mut self.pixels,
                );
            }
        }
        self.source.clear();
        self.source
            .extend(source.iter().map(|pixel| input_word(*pixel)));
        self.backing.clear();
        self.backing
            .extend(backing.iter().map(|pixel| input_word(*pixel)));
        self.packing = Some(packing);
        // Input changes conservatively invalidate even when the result happens
        // to be equal. No second packed-buffer comparison or output copy.
        self.revision = Some(screen::ui_layer::fresh_pixels_revision());
        #[cfg(test)]
        {
            self.packs += 1;
        }
    }

    pub fn revision(&self) -> Option<u64> {
        self.revision
    }

    pub fn clear(&mut self) {
        self.pixels.clear();
        self.source.clear();
        self.backing.clear();
        self.packing = None;
        self.revision = None;
    }
}

impl Deref for Pixels {
    type Target = [[u8; 4]];
    fn deref(&self) -> &Self::Target {
        &self.pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<UiPixel> {
        let mut out = vec![UiPixel::default(); 480 * 160];
        for y in 98..132 {
            for x in 130..270 {
                out[y * 480 + x] = UiPixel {
                    color: ((x + y) & 0x7fff) as u16,
                    eva: 10,
                    evb: 6,
                };
            }
        }
        out
    }

    #[test]
    fn complete_input_key_skips_packing_and_detects_in_place_mutation() {
        let mut source = sample();
        let mut backing = source.clone();
        let mut cache = Pixels::default();
        cache.panel(&source, &backing, 1.25, false);
        let first = cache.revision();
        let capacity = cache.pixels.capacity();
        cache.panel(&source, &backing, 1.25, false);
        assert_eq!(cache.packs, 1);
        assert_eq!(cache.revision(), first);
        source[100 * 480 + 140].color ^= 1;
        cache.panel(&source, &backing, 1.25, false);
        assert_eq!(cache.packs, 2);
        backing[100 * 480 + 140].eva ^= 1;
        cache.panel(&source, &backing, 1.25, false);
        assert_eq!(cache.packs, 3);
        cache.panel(&source, &backing, 1.5, false);
        assert_eq!(cache.packs, 4);
        cache.panel(&source, &backing, 1.5, true);
        assert_eq!(cache.packs, 5);
        cache.panel(&source, &[], 1.5, true);
        assert_eq!(cache.packs, 6);
        cache.raw(&source);
        assert_eq!(cache.packs, 7);
        assert_eq!(cache.pixels.capacity(), capacity);
        cache.raw(&source);
        assert_eq!(cache.packs, 7);
        cache.clear();
        assert!(cache.is_empty() && cache.revision().is_none());
        cache.raw(&source);
        assert_eq!(cache.packs, 8);
        assert_ne!(cache.revision(), first);
    }

    #[test]
    fn cached_output_matches_direct_packing_for_blends_modes_styles_and_reactivation() {
        let mut source = sample();
        let mut backing = source.clone();
        let mut cache = Pixels::default();
        let mut reference = Vec::new();
        for pattern in 0..4 {
            if pattern == 1 {
                source[100 * 480 + 140].evb = 16;
            }
            if pattern == 2 {
                backing.fill(UiPixel::default());
            }
            if pattern == 3 {
                source.fill(UiPixel::default());
            }
            for right in [false, true] {
                for length in [1., 1.25, 1.5, 2.] {
                    for has_backing in [false, true] {
                        let back = if has_backing { backing.as_slice() } else { &[] };
                        pack_panel(&source, back, length, right, &mut reference);
                        cache.panel(&source, back, length, right);
                        assert_eq!(&*cache, &reference);
                        let token = cache.revision();
                        let packs = cache.packs;
                        cache.panel(&source, back, length, right);
                        assert_eq!(cache.packs, packs);
                        assert_eq!(cache.revision(), token);
                        cache.clear();
                        cache.panel(&source, back, length, right);
                        assert_eq!(&*cache, &reference);
                        assert_ne!(cache.revision(), token);
                    }
                }
            }
            cache.raw(&source);
            assert_eq!(
                &*cache,
                source.iter().map(|p| p.packed()).collect::<Vec<_>>()
            );
        }
        let mut other = Pixels::default();
        other.raw(&source);
        assert_ne!(other.revision(), cache.revision());
    }
}
