//! Source-authenticated metadata for the verified background descriptor prefix.
//! Linked payloads and optional programs are separate resources, not decoded here.
use super::SourceRange;
use crate::ffta_rom;
use serde::{Deserialize, Serialize};

pub const RECORD_BYTES: usize = 88;
pub const BASE_RECORDS: usize = 162;
pub const MAX_RECORDS: usize = 512;
pub const SOURCE_SHA256: &str = "2766525e0889e0ec2f3646dda7a95c143fe68931ee19923b6808b5d989519ce6";

/// The sources are relative to the catalog root, not to this record. A zero
/// triple is absent; an active triple has two sources and an absolute VRAM
/// destination. Transfer sizes/program semantics require payload decoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferLink {
    pub control_source: u32,
    pub data_source: u32,
    pub destination: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    /// The first producer's format is still unclassified.
    pub graphics_source: u32,
    pub paired_planes_source: u32,
    pub halfword_planes_source: u32,
    pub palette_source: u32,
    pub auxiliary_patches_source: u32,
    pub transfer: TransferLink,
    pub extra_transfers: [TransferLink; 4],
    /// Optional relative source, not a boolean despite the presence getter.
    pub variant_source: u32,
    /// Original conditional selectors; their game-level names are unverified.
    pub selectors: [u8; 2],
    pub alternate_palette_key: u8,
    /// Retain the complete record even where no reader is known yet.
    pub trailing_byte: u8,
}

impl Descriptor {
    pub fn selected_selector(&self, environment: u8) -> u8 {
        if matches!(environment & 3, 1 | 2) && self.selectors[1] != 0xff {
            self.selectors[1]
        } else {
            self.selectors[0]
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptors {
    pub entries: Vec<Descriptor>,
}

impl Descriptors {
    pub fn decode(bytes: &[u8], root: u32, rom_size: usize) -> Result<Self, String> {
        if bytes.is_empty()
            || bytes.len() % RECORD_BYTES != 0
            || bytes.len() / RECORD_BYTES > MAX_RECORDS
        {
            return Err("Invalid background descriptor count/record boundary".into());
        }
        let entries = bytes
            .chunks_exact(RECORD_BYTES)
            .map(|record| {
                let word =
                    |offset| u32::from_le_bytes(record[offset..offset + 4].try_into().unwrap());
                Descriptor {
                    graphics_source: word(0),
                    paired_planes_source: word(4),
                    halfword_planes_source: word(8),
                    palette_source: word(12),
                    auxiliary_patches_source: word(16),
                    transfer: TransferLink {
                        control_source: word(20),
                        data_source: word(24),
                        destination: word(28),
                    },
                    extra_transfers: std::array::from_fn(|index| TransferLink {
                        control_source: word(32 + index * 4),
                        data_source: word(48 + index * 4),
                        destination: word(64 + index * 4),
                    }),
                    variant_source: word(80),
                    selectors: [record[84], record[85]],
                    alternate_palette_key: record[86],
                    trailing_byte: record[87],
                }
            })
            .collect();
        let result = Self { entries };
        result.validate(root, rom_size)?;
        Ok(result)
    }

    pub fn validate(&self, root: u32, rom_size: usize) -> Result<(), String> {
        if self.entries.is_empty() || self.entries.len() > MAX_RECORDS {
            return Err("Invalid background descriptor count".into());
        }
        let table_bytes = self.entries.len() * RECORD_BYTES;
        if (root as usize)
            .checked_add(table_bytes)
            .is_none_or(|end| end > rom_size)
        {
            return Err("Background descriptor source range is outside ROM".into());
        }
        let relative = |offset: u32, required: bool| -> Result<(), String> {
            if offset == 0 && !required {
                return Ok(());
            }
            let end = (root as usize)
                .checked_add(offset as usize)
                .and_then(|start| start.checked_add(4));
            if (offset as usize) < table_bytes || end.is_none_or(|end| end > rom_size) {
                return Err(
                    "Background descriptor relative source is outside payload bounds".into(),
                );
            }
            Ok(())
        };
        for entry in &self.entries {
            for source in [
                entry.graphics_source,
                entry.paired_planes_source,
                entry.halfword_planes_source,
                entry.palette_source,
                entry.auxiliary_patches_source,
            ] {
                relative(source, true)?;
            }
            for transfer in std::iter::once(&entry.transfer).chain(&entry.extra_transfers) {
                if transfer.control_source == 0
                    && transfer.data_source == 0
                    && transfer.destination == 0
                {
                    continue;
                }
                relative(transfer.control_source, true)?;
                relative(transfer.data_source, true)?;
                if !(0x0600_0000..0x0601_8000).contains(&transfer.destination)
                    || transfer.destination & 1 != 0
                {
                    return Err(
                        "Background descriptor transfer destination is outside aligned VRAM".into(),
                    );
                }
            }
            relative(entry.variant_source, false)?;
        }
        Ok(())
    }

    /// Exact metadata roundtrip; this does not reconstruct any linked payload.
    pub fn source_bytes(&self) -> Result<Vec<u8>, String> {
        if self.entries.is_empty() || self.entries.len() > MAX_RECORDS {
            return Err("Invalid background descriptor count".into());
        }
        let mut bytes = Vec::with_capacity(self.entries.len() * RECORD_BYTES);
        for entry in &self.entries {
            let mut words = [0u32; 21];
            words[..5].copy_from_slice(&[
                entry.graphics_source,
                entry.paired_planes_source,
                entry.halfword_planes_source,
                entry.palette_source,
                entry.auxiliary_patches_source,
            ]);
            words[5..8].copy_from_slice(&[
                entry.transfer.control_source,
                entry.transfer.data_source,
                entry.transfer.destination,
            ]);
            for (index, transfer) in entry.extra_transfers.iter().enumerate() {
                words[8 + index] = transfer.control_source;
                words[12 + index] = transfer.data_source;
                words[16 + index] = transfer.destination;
            }
            words[20] = entry.variant_source;
            for word in words {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            bytes.extend_from_slice(&[
                entry.selectors[0],
                entry.selectors[1],
                entry.alternate_palette_key,
                entry.trailing_byte,
            ]);
        }
        Ok(bytes)
    }
}

pub struct Catalog {
    pub source: SourceRange,
    pub decoded: Descriptors,
}

impl Catalog {
    pub fn from_rom(rom: &[u8]) -> Result<Self, String> {
        let (root, literal) = match ffta_rom::digest(rom).as_str() {
            ffta_rom::SHA256 => (0x569104, 0x1a4ac),
            ffta_rom::EUROPE_SHA256 => (0x7662a8, 0x1b61c),
            _ => return Err("Background catalog requires an exact supported ROM".into()),
        };
        let expected = (0x0800_0000u32 + root as u32).to_le_bytes();
        if rom.get(literal..literal + 4) != Some(&expected[..]) {
            return Err("Background catalog root literal mismatch".into());
        }
        let length = BASE_RECORDS * RECORD_BYTES;
        let bytes = rom
            .get(root..root + length)
            .ok_or("Truncated background catalog")?;
        if ffta_rom::digest(bytes) != SOURCE_SHA256 {
            return Err("Background catalog source mismatch".into());
        }
        let decoded = Descriptors::decode(bytes, root as u32, rom.len())?;
        Ok(Self {
            source: SourceRange {
                offset: root as u32,
                length: length as u32,
                sha256: SOURCE_SHA256.into(),
                role: "relative-bg-descriptor-prefix".into(),
            },
            decoded,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn synthetic() -> Descriptors {
        let transfer = TransferLink {
            control_source: 0x180,
            data_source: 0x1a0,
            destination: 0x0600_0020,
        };
        let descriptor = Descriptor {
            graphics_source: 0x100,
            paired_planes_source: 0x110,
            halfword_planes_source: 0x120,
            palette_source: 0x130,
            auxiliary_patches_source: 0x140,
            transfer: transfer.clone(),
            extra_transfers: std::array::from_fn(|_| transfer.clone()),
            variant_source: 0x1c0,
            selectors: [7, 23],
            alternate_palette_key: 156,
            trailing_byte: 0x81,
        };
        Descriptors {
            entries: vec![descriptor.clone(), descriptor],
        }
    }

    #[test]
    fn typed_roundtrip_preserves_relative_groups_and_unclassified_bytes() {
        let source = synthetic();
        let bytes = source.source_bytes().unwrap();
        let decoded = Descriptors::decode(&bytes, 0x100, 0x1000).unwrap();
        assert_eq!(decoded, source);
        assert_eq!(decoded.source_bytes().unwrap(), bytes);
    }

    #[test]
    fn selectors_mask_environment_and_preserve_fallback_and_zero() {
        let mut descriptor = synthetic().entries.remove(0);
        for environment in 0..=255 {
            assert_eq!(
                descriptor.selected_selector(environment),
                if matches!(environment % 4, 1 | 2) {
                    23
                } else {
                    7
                }
            );
        }
        descriptor.selectors[1] = 0;
        assert_eq!(descriptor.selected_selector(1), 0);
        descriptor.selectors[1] = 255;
        assert_eq!(descriptor.selected_selector(2), 7);
    }

    #[test]
    fn malformed_record_counts_pointers_and_partial_transfers_fail() {
        let source = synthetic();
        let bytes = source.source_bytes().unwrap();
        for end in 0..bytes.len() {
            if end % RECORD_BYTES != 0 || end == 0 {
                assert!(Descriptors::decode(&bytes[..end], 0x100, 0x1000).is_err());
            }
        }
        assert!(
            Descriptors::decode(&vec![0; RECORD_BYTES * (MAX_RECORDS + 1)], 0, usize::MAX).is_err()
        );
        assert!(Descriptors::decode(&bytes, u32::MAX, 0x1000).is_err());
        for (field, value) in [
            (0, 0),
            (4, 4),
            (8, 0xfffffff0),
            (12, 0xf00),
            (20, 0),
            (24, 0),
            (28, 0),
            (28, 0x08000000),
            (28, 0x06000001),
            (80, 1),
        ] {
            let mut bad = bytes.clone();
            bad[field..field + 4].copy_from_slice(&u32::to_le_bytes(value));
            assert!(
                Descriptors::decode(&bad, 0x100, 0x1000).is_err(),
                "field {field}"
            );
        }
        assert!(Catalog::from_rom(&bytes).is_err());
    }
}
