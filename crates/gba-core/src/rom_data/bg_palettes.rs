//! Complete base-palette banks. Alternate environment programs are separate.
use super::{bg_descriptors, lz77, SourceRange};
use crate::ffta_rom;
use serde::{Deserialize, Serialize};

pub const WORDS_PER_BANK: usize = 16;
pub const MAX_BANKS: usize = 16;
pub const MAX_OUTPUT_BYTES: usize = MAX_BANKS * WORDS_PER_BANK * 2;
pub const MAX_ENCODED_BYTES: usize = 4 + MAX_OUTPUT_BYTES + MAX_OUTPUT_BYTES.div_ceil(8);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    /// Preserve all16bits, including the unclassified high bit. No RGBA/alpha.
    pub banks: Vec<[u16; WORDS_PER_BANK]>,
    /// Exact consumed source representation for CPU/DMA/real-BIOS reads.
    /// This is compatibility metadata, not a recompression of decoded banks.
    pub encoded_source: Vec<u8>,
}

impl Palette {
    pub fn validate(&self) -> Result<(), String> {
        if self.banks.is_empty() || self.banks.len() > MAX_BANKS {
            return Err("Invalid base palette bank count".into());
        }
        if self.encoded_source.len() > MAX_ENCODED_BYTES {
            return Err("Base palette source representation exceeds its bound".into());
        }
        let (decoded, _, consumed) = decode_payload(&self.encoded_source)?;
        if consumed != self.encoded_source.len() || decoded != self.bank_bytes() {
            return Err("Base palette source representation and banks disagree".into());
        }
        Ok(())
    }

    fn bank_bytes(&self) -> Vec<u8> {
        self.banks
            .iter()
            .flatten()
            .flat_map(|word| word.to_le_bytes())
            .collect()
    }

    pub fn source_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        Ok(self.bank_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palettes {
    /// Descriptor IDs index these entries directly; aliases retain each ID.
    pub entries: Vec<Palette>,
}

impl Palettes {
    pub fn validate(&self) -> Result<(), String> {
        if self.entries.is_empty() || self.entries.len() > bg_descriptors::MAX_RECORDS {
            return Err("Invalid base palette record count".into());
        }
        for palette in &self.entries {
            palette.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Raw,
    Lz77,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    pub palette: Palette,
    pub encoding: Encoding,
    /// Header and consumed payload; alignment padding is excluded.
    pub consumed: usize,
}

fn decode_payload(input: &[u8]) -> Result<(Vec<u8>, Encoding, usize), String> {
    let header = input.get(..4).ok_or("Truncated base palette header")?;
    let size = (u32::from_le_bytes(header.try_into().unwrap()) >> 8) as usize;
    if size == 0 || size > MAX_OUTPUT_BYTES || size % (WORDS_PER_BANK * 2) != 0 {
        return Err("Invalid base palette decoded size/bank boundary".into());
    }
    let (bytes, encoding, consumed) = match header[0] {
        0 => (
            input
                .get(4..4 + size)
                .ok_or("Truncated raw base palette")?
                .to_vec(),
            Encoding::Raw,
            4 + size,
        ),
        0x10 => {
            let block = lz77::decode(input, MAX_OUTPUT_BYTES)?;
            (block.bytes, Encoding::Lz77, block.consumed)
        }
        _ => return Err("Unsupported base palette source encoding".into()),
    };
    Ok((bytes, encoding, consumed))
}

pub fn decode(input: &[u8]) -> Result<Block, String> {
    let (bytes, encoding, consumed) = decode_payload(input)?;
    let palette = Palette {
        encoded_source: input[..consumed].to_vec(),
        banks: bytes
            .chunks_exact(WORDS_PER_BANK * 2)
            .map(|bank| {
                std::array::from_fn(|index| {
                    u16::from_le_bytes([bank[index * 2], bank[index * 2 + 1]])
                })
            })
            .collect(),
    };
    palette.validate()?;
    Ok(Block {
        palette,
        encoding,
        consumed,
    })
}

pub struct PaletteSource {
    pub range: SourceRange,
    pub encoding: Encoding,
}

pub struct Catalog {
    pub decoded: Palettes,
    /// One provenance entry per descriptor ID, including shared-source aliases.
    pub sources: Vec<PaletteSource>,
}

impl Catalog {
    pub fn from_rom(rom: &[u8]) -> Result<Self, String> {
        let descriptors = bg_descriptors::Catalog::from_rom(rom)?;
        Self::from_descriptors(rom, &descriptors)
    }

    pub(crate) fn from_descriptors(
        rom: &[u8],
        descriptors: &bg_descriptors::Catalog,
    ) -> Result<Self, String> {
        let mut entries = Vec::new();
        let mut sources = Vec::new();
        for (id, descriptor) in descriptors.decoded.entries.iter().enumerate() {
            let offset = descriptors
                .source
                .offset
                .checked_add(descriptor.palette_source)
                .ok_or("Base palette source overflow")?;
            let input = rom
                .get(offset as usize..)
                .ok_or("Base palette source outside ROM")?;
            let block = decode(input)?;
            sources.push(PaletteSource {
                range: SourceRange {
                    offset,
                    length: block.consumed as u32,
                    sha256: ffta_rom::digest(&input[..block.consumed]),
                    role: format!("base-palette/{id}"),
                },
                encoding: block.encoding,
            });
            entries.push(block.palette);
        }
        let decoded = Palettes { entries };
        decoded.validate()?;
        Ok(Self { decoded, sources })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words() -> Vec<u8> {
        (0..32u16)
            .flat_map(|index| (0x8000 | index * 37).to_le_bytes())
            .collect()
    }

    #[test]
    fn raw_banks_preserve_all_bits_endianness_and_exact_extent() {
        let bytes = words();
        let mut input = vec![0, 64, 0, 0];
        input.extend_from_slice(&bytes);
        input.extend_from_slice(&[0xcc; 8]);
        let block = decode(&input).unwrap();
        assert_eq!(block.encoding, Encoding::Raw);
        assert_eq!(block.consumed, 68);
        assert_eq!(block.palette.banks.len(), 2);
        assert_eq!(block.palette.banks[0][0], 0x8000);
        assert_eq!(block.palette.source_bytes().unwrap(), bytes);
    }

    #[test]
    fn compressed_banks_match_raw_words_and_exclude_padding() {
        let bytes = words();
        let mut input = vec![0x10, 64, 0, 0];
        for group in bytes.chunks(8) {
            input.push(0);
            input.extend_from_slice(group);
        }
        let consumed = input.len();
        input.extend_from_slice(&[0xaa; 4]);
        let block = decode(&input).unwrap();
        assert_eq!(block.encoding, Encoding::Lz77);
        assert_eq!(block.consumed, consumed);
        assert_eq!(block.palette.source_bytes().unwrap(), bytes);
    }

    #[test]
    fn rejects_unknown_truncated_unaligned_and_oversized_sources_or_models() {
        for size in [0u32, 1, 31, 33, 544, 0xffffff] {
            let mut input = vec![0; 4];
            input[..4].copy_from_slice(&(size << 8).to_le_bytes());
            assert!(decode(&input).is_err());
        }
        assert!(decode(&[0, 32, 0]).is_err());
        assert!(decode(&[0, 32, 0, 0, 1]).is_err());
        assert!(decode(&[0x20, 32, 0, 0]).is_err());
        assert!(decode(&[0x10, 32, 0, 0, 0x80, 0, 0]).is_err());
        assert!(Palette {
            banks: vec![],
            encoded_source: vec![]
        }
        .validate()
        .is_err());
        assert!(Palette {
            banks: vec![[0; WORDS_PER_BANK]; MAX_BANKS + 1],
            encoded_source: vec![],
        }
        .validate()
        .is_err());
        assert!(Palettes { entries: vec![] }.validate().is_err());
    }
}
