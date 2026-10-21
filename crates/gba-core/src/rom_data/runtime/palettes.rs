//! Immutable decoded banks and byte-exact compatibility views from the file.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaletteStats {
    pub bus_reads: u64,
    pub source_bytes: u64,
    pub lz77_calls: u64,
    pub decoded_bytes: u64,
}

/// An owned borrow keeps decoded output alive while the bus is mutated.
pub struct PaletteRead {
    owner: Arc<RuntimeResources>,
    row: usize,
}

impl PaletteRead {
    pub fn bytes(&self) -> &[u8] {
        &self.owner.palettes.as_ref().unwrap().decoded_bytes[self.row]
    }
}

struct Binding {
    offset: usize,
    row: usize,
}

pub(super) struct PaletteResources {
    palettes: bg_palettes::Palettes,
    decoded_bytes: Vec<Vec<u8>>,
    bindings: Vec<Binding>,
    end: usize,
    file_sha256: String,
    bus_reads: AtomicU64,
    source_bytes: AtomicU64,
    lz77_calls: AtomicU64,
    decoded_byte_count: AtomicU64,
}

impl PaletteResources {
    #[inline]
    pub(super) fn source_page(index: usize) -> bool {
        matches!(index >> 12, 0x56f..=0x697 | 0x76c..=0x894)
    }

    pub(super) fn new(
        palettes: bg_palettes::Palettes,
        sources: &[bg_palettes::PaletteSource],
        file_sha256: String,
    ) -> Result<Self, String> {
        palettes.validate()?;
        if palettes.entries.len() != sources.len() {
            return Err("Palette binding count mismatch".into());
        }
        let mut decoded_bytes = Vec::new();
        let mut bindings = Vec::new();
        for (row, (palette, source)) in palettes.entries.iter().zip(sources).enumerate() {
            if palette.encoded_source.len() != source.range.length as usize
                || ffta_rom::digest(&palette.encoded_source) != source.range.sha256
                || (palette.encoded_source[0] == 0x10)
                    != (source.encoding == bg_palettes::Encoding::Lz77)
            {
                return Err("Palette file/source binding mismatch".into());
            }
            let offset = source.range.offset as usize;
            if offset
                .checked_add(palette.encoded_source.len())
                .is_none_or(|end| end > 0x0200_0000)
            {
                return Err("Palette binding outside cartridge window".into());
            }
            if !Self::source_page(offset)
                || !Self::source_page(offset + palette.encoded_source.len() - 1)
            {
                return Err("Palette binding outside fixed asset pages".into());
            }
            decoded_bytes.push(palette.source_bytes()?);
            bindings.push(Binding { offset, row });
        }
        bindings.sort_by_key(|binding| binding.offset);
        let mut unique: Vec<Binding> = Vec::new();
        for binding in bindings {
            if let Some(prior) = unique.last() {
                if prior.offset == binding.offset {
                    if palettes.entries[prior.row] != palettes.entries[binding.row] {
                        return Err("Palette aliases disagree".into());
                    }
                    continue;
                }
                if prior.offset + palettes.entries[prior.row].encoded_source.len() > binding.offset
                {
                    return Err("Palette source spans overlap".into());
                }
            }
            unique.push(binding);
        }
        let last = unique.last().ok_or("Missing palette bindings")?;
        let end = last.offset + palettes.entries[last.row].encoded_source.len();
        Ok(Self {
            palettes,
            decoded_bytes,
            bindings: unique,
            end,
            file_sha256,
            bus_reads: AtomicU64::new(0),
            source_bytes: AtomicU64::new(0),
            lz77_calls: AtomicU64::new(0),
            decoded_byte_count: AtomicU64::new(0),
        })
    }

    pub(super) fn resident_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.file_sha256.capacity()
            + self.bindings.capacity() * std::mem::size_of::<Binding>()
            + self.palettes.entries.capacity() * std::mem::size_of::<bg_palettes::Palette>()
            + self
                .palettes
                .entries
                .iter()
                .map(|palette| palette.banks.capacity() * 32 + palette.encoded_source.capacity())
                .sum::<usize>()
            + self.decoded_bytes.capacity() * std::mem::size_of::<Vec<u8>>()
            + self.decoded_bytes.iter().map(Vec::capacity).sum::<usize>()
    }

    pub(super) fn file_sha256(&self) -> &str {
        &self.file_sha256
    }

    pub(super) fn stats(&self) -> PaletteStats {
        PaletteStats {
            bus_reads: self.bus_reads.load(Ordering::Relaxed),
            source_bytes: self.source_bytes.load(Ordering::Relaxed),
            lz77_calls: self.lz77_calls.load(Ordering::Relaxed),
            decoded_bytes: self.decoded_byte_count.load(Ordering::Relaxed),
        }
    }

    fn byte(&self, index: usize) -> Option<u8> {
        let binding = self.bindings.get(
            self.bindings
                .partition_point(|binding| binding.offset <= index)
                .checked_sub(1)?,
        )?;
        let at = index - binding.offset;
        let palette = &self.palettes.entries[binding.row];
        let encoded = *palette.encoded_source.get(at)?;
        if palette.encoded_source[0] == 0 && at >= 4 {
            Some(self.decoded_bytes[binding.row][at - 4])
        } else {
            Some(encoded)
        }
    }

    pub(super) fn guest_read<const N: usize>(&self, index: usize, rom: &[u8]) -> Option<[u8; N]> {
        if index >= self.end || index.saturating_add(N) <= self.bindings[0].offset {
            return None;
        }
        let mut mapped = 0;
        let bytes = std::array::from_fn(|i| match self.byte(index + i) {
            Some(byte) => {
                mapped += 1;
                byte
            }
            None => rom[index + i],
        });
        if mapped == 0 {
            return None;
        }
        self.bus_reads.fetch_add(1, Ordering::Relaxed);
        self.source_bytes.fetch_add(mapped, Ordering::Relaxed);
        Some(bytes)
    }

    fn lz77_row(&self, index: usize) -> Option<usize> {
        let binding = &self.bindings[self
            .bindings
            .binary_search_by_key(&index, |binding| binding.offset)
            .ok()?];
        (self.palettes.entries[binding.row].encoded_source[0] == 0x10).then_some(binding.row)
    }
}

impl RuntimeResources {
    pub(crate) fn decoded_palette(self: &Arc<Self>, address: u32) -> Option<PaletteRead> {
        if !matches!(address >> 24, 0x08..=0x0d) {
            return None;
        }
        let palettes = self.palettes.as_ref()?;
        let row = palettes.lz77_row((address & 0x01ff_ffff) as usize)?;
        palettes.lz77_calls.fetch_add(1, Ordering::Relaxed);
        palettes
            .decoded_byte_count
            .fetch_add(palettes.decoded_bytes[row].len() as u64, Ordering::Relaxed);
        Some(PaletteRead {
            owner: self.clone(),
            row,
        })
    }
}

#[cfg(test)]
#[path = "palette_tests.rs"]
mod tests;
