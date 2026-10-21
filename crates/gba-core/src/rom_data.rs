//! Versioned decoded base resources. No filesystem I/O or guest state.
//! Only exact, independently verified profiles are eligible for extraction.
use crate::ffta_rom::{self, Version};
use serde::{Deserialize, Serialize};

mod runtime;
pub use runtime::{BackgroundStats, PaletteRead, PaletteStats, ResourceStats, RuntimeResources};
pub mod bg_descriptors;
pub mod bg_packets;
pub mod bg_palettes;
pub mod lz77;

pub const SCHEMA: u32 = 1;
pub const EXTRACTOR_REV: u32 = 6;
pub const DECODER_REV: u32 = 5;
pub const OBJ_DIMENSIONS_ID: &str = "graphics.obj-dimensions";
pub const OBJ_DIMENSIONS_FILE: &str = "obj-dimensions.toml";
pub const BG_DESCRIPTORS_ID: &str = "graphics.bg-descriptors";
pub const BG_DESCRIPTORS_FILE: &str = "bg-descriptors.json";
pub const BG_DESCRIPTORS_FORMAT: &str = "bg-descriptors-json-v2";
pub const BG_BASE_PALETTES_ID: &str = "graphics.bg-base-palettes";
pub const BG_BASE_PALETTES_FILE: &str = "bg-base-palettes.json";
pub const BG_BASE_PALETTES_FORMAT: &str = "bg-base-palettes-v2-json";
pub const MAX_MANIFEST_FILE_BYTES: u64 = 64 * 1024;
pub const MAX_BG_BASE_PALETTES_FILE_BYTES: u64 = 1024 * 1024;
pub const MANIFEST_FILE: &str = "manifest.toml";
pub const MAX_FILE_BYTES: u64 = 16 * 1024;
pub const MAX_BG_DESCRIPTORS_FILE_BYTES: u64 = 256 * 1024;
pub const MAX_DATASET_BYTES: usize = 1280 * 1024;
pub const DIMENSIONS_SHA256: &str =
    "6ff886b23cde2726a4ceb9cba8c5569bbb4dc6a1185769547f78dfbcb7e2d991";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjSize {
    pub width: u16,
    pub height: u16,
}

/// Shape-major, size-minor indexed metadata. Shape 3 has no record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjDimensions {
    pub entries: [ObjSize; 12],
}

impl ObjDimensions {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 48 {
            return Err("OBJ dimensions require exactly 48 source bytes".into());
        }
        let result = Self {
            entries: std::array::from_fn(|i| ObjSize {
                width: u16::from_le_bytes([bytes[i * 4], bytes[i * 4 + 1]]),
                height: u16::from_le_bytes([bytes[i * 4 + 2], bytes[i * 4 + 3]]),
            }),
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), String> {
        // Hardware OBJ extents; the actual base ordering is authenticated by
        // the source digest, independently of these structural constraints.
        if self
            .entries
            .iter()
            .any(|e| !matches!(e.width, 8 | 16 | 32 | 64) || !matches!(e.height, 8 | 16 | 32 | 64))
        {
            return Err("Invalid OBJ dimension (expected 8, 16, 32 or 64 pixels)".into());
        }
        Ok(())
    }

    pub fn source_bytes(&self) -> [u8; 48] {
        let mut out = [0; 48];
        for (i, e) in self.entries.iter().enumerate() {
            out[i * 4..i * 4 + 2].copy_from_slice(&e.width.to_le_bytes());
            out[i * 4 + 2..i * 4 + 4].copy_from_slice(&e.height.to_le_bytes());
        }
        out
    }

    pub fn get(&self, shape: u8, size: u8) -> Option<&ObjSize> {
        if shape >= 3 || size >= 4 {
            return None;
        }
        self.entries.get(shape as usize * 4 + size as usize)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRange {
    pub offset: u32,
    pub length: u32,
    pub sha256: String,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub id: String,
    pub path: String,
    pub format: String,
    pub sha256: String,
    pub dependencies: Vec<String>,
    pub sources: Vec<SourceRange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub extractor_revision: u32,
    pub decoder_revision: u32,
    pub source_rom_sha256: String,
    pub source_rom_size: u32,
    pub profile: String,
    pub coverage: Vec<String>,
    pub resources: Vec<Resource>,
}

/// A source-authenticated extraction. This is not a mod loader.
pub struct Extraction {
    pub manifest: Manifest,
    pub dimensions: ObjDimensions,
    pub background: Option<bg_descriptors::Descriptors>,
    pub palettes: Option<bg_palettes::Catalog>,
}

/// Typed values borrowed from the exact captured file bytes. The frontend
/// parses them once; source validation and runtime binding cover the whole set.
#[derive(Clone, Copy)]
pub struct BaseFiles<'a> {
    pub manifest: &'a Manifest,
    pub dimensions: (&'a ObjDimensions, &'a [u8]),
    pub background: Option<(&'a bg_descriptors::Descriptors, &'a [u8])>,
    pub palettes: Option<(&'a bg_palettes::Palettes, &'a [u8])>,
}

impl Extraction {
    pub fn from_rom(rom: &[u8]) -> Result<Self, String> {
        let hash = ffta_rom::digest(rom);
        let (profile, ranges): (&str, &[(u32, &str)]) = match hash.as_str() {
            ffta_rom::SHA256 => ("usa", &[(0x149510, "direct"), (0xa398dc, "iwram-copy")]),
            ffta_rom::EUROPE_SHA256 => (
                "europe",
                &[(0x14ced8, "rom-table"), (0xd976e8, "iwram-copy")],
            ),
            _ => return Err("Decoded resources require a verified FFTA EUR or USA ROM".into()),
        };
        let catalog = bg_descriptors::Catalog::from_rom(rom)?;
        let palettes = bg_palettes::Catalog::from_descriptors(rom, &catalog)?;
        let mut result = Self::from_ranges(rom, hash, profile, ranges, DIMENSIONS_SHA256)?;
        result.background = Some(catalog.decoded);
        result.manifest.coverage.push(BG_DESCRIPTORS_ID.into());
        result.manifest.resources.push(Resource {
            id: BG_DESCRIPTORS_ID.into(),
            path: BG_DESCRIPTORS_FILE.into(),
            format: BG_DESCRIPTORS_FORMAT.into(),
            sha256: String::new(),
            dependencies: Vec::new(),
            sources: vec![catalog.source],
        });
        result.manifest.coverage.push(BG_BASE_PALETTES_ID.into());
        result.manifest.resources.push(Resource {
            id: BG_BASE_PALETTES_ID.into(),
            path: BG_BASE_PALETTES_FILE.into(),
            format: BG_BASE_PALETTES_FORMAT.into(),
            sha256: String::new(),
            dependencies: vec![BG_DESCRIPTORS_ID.into()],
            sources: palettes
                .sources
                .iter()
                .map(|source| source.range.clone())
                .collect(),
        });
        result.palettes = Some(palettes);
        Ok(result)
    }

    fn from_ranges(
        rom: &[u8],
        hash: String,
        profile: &str,
        ranges: &[(u32, &str)],
        expected: &str,
    ) -> Result<Self, String> {
        let mut sources = Vec::new();
        let mut dimensions = None;
        for &(offset, role) in ranges {
            let start = offset as usize;
            let bytes = rom
                .get(start..start.checked_add(48).ok_or("Source range overflow")?)
                .ok_or("Truncated OBJ dimensions source range")?;
            let sha256 = ffta_rom::digest(bytes);
            if sha256 != expected {
                return Err(format!("OBJ dimensions source mismatch at {offset:#x}"));
            }
            let decoded = ObjDimensions::decode(bytes)?;
            if dimensions.as_ref().is_some_and(|d| d != &decoded) {
                return Err("OBJ dimensions source aliases disagree".into());
            }
            dimensions = Some(decoded);
            sources.push(SourceRange {
                offset,
                length: 48,
                sha256,
                role: role.into(),
            });
        }
        Ok(Self {
            dimensions: dimensions.ok_or("Missing OBJ dimensions source")?,
            background: None,
            palettes: None,
            manifest: Manifest {
                schema: SCHEMA,
                extractor_revision: EXTRACTOR_REV,
                decoder_revision: DECODER_REV,
                source_rom_sha256: hash,
                source_rom_size: u32::try_from(rom.len()).map_err(|_| "ROM is too large")?,
                profile: profile.into(),
                coverage: vec![OBJ_DIMENSIONS_ID.into()],
                resources: vec![Resource {
                    id: OBJ_DIMENSIONS_ID.into(),
                    path: OBJ_DIMENSIONS_FILE.into(),
                    format: "obj-dimensions-v1".into(),
                    // File bytes are defined by the frontend serializer.
                    sha256: String::new(),
                    dependencies: Vec::new(),
                    sources,
                }],
            },
        })
    }

    pub fn set_file_hashes(&mut self, files: &[&[u8]]) -> Result<(), String> {
        if self.manifest.resources.len()
            != 1 + usize::from(self.background.is_some()) + usize::from(self.palettes.is_some())
            || files.len() != self.manifest.resources.len()
        {
            return Err("Resource set does not match extraction".into());
        }
        for (resource, bytes) in self.manifest.resources.iter_mut().zip(files) {
            resource.sha256 = ffta_rom::digest(bytes);
        }
        Ok(())
    }

    /// The expected manifest comes from the selected source, never the files.
    /// A self-consistent edited resource is still an invalid *base* dataset.
    pub fn validate_files(&self, files: BaseFiles<'_>) -> Result<(), String> {
        if self.palettes.is_some() && self.background.is_none()
            || self.manifest.resources.len()
                != 1 + usize::from(self.background.is_some()) + usize::from(self.palettes.is_some())
        {
            return Err("Extraction resource set is inconsistent".into());
        }
        let (dimensions, file_bytes) = files.dimensions;
        if file_bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("OBJ dimensions file exceeds size limit".into());
        }
        let mut total = file_bytes.len();
        let mut expected = self.manifest.clone();
        expected.resources[0].sha256 = ffta_rom::digest(file_bytes);
        match (&self.background, files.background) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_DESCRIPTORS_FILE_BYTES {
                    return Err("Background descriptor file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[1].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected background descriptor resource".into()),
        }
        match (&self.palettes, files.palettes) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_BASE_PALETTES_FILE_BYTES {
                    return Err("Base palette file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[2].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected base palette resource".into()),
        }
        if total > MAX_DATASET_BYTES {
            return Err("Resource files exceed total size limit".into());
        }
        if files.manifest != &expected {
            return Err(
                "Resource manifest does not match the source/profile/schema or file hash".into(),
            );
        }
        dimensions.validate()?;
        if dimensions != &self.dimensions
            || ffta_rom::digest(&dimensions.source_bytes())
                != self.manifest.resources[0].sources[0].sha256
        {
            return Err("Decoded OBJ dimensions do not reproduce the selected ROM source".into());
        }
        if let (Some(expected), Some((decoded, _))) = (&self.background, files.background) {
            let source = &self.manifest.resources[1].sources[0];
            decoded.validate(source.offset, self.manifest.source_rom_size as usize)?;
            if decoded != expected || ffta_rom::digest(&decoded.source_bytes()?) != source.sha256 {
                return Err(
                    "Decoded background descriptors do not reproduce the selected ROM source"
                        .into(),
                );
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.palettes, files.palettes) {
            decoded.validate()?;
            if decoded != &expected.decoded {
                return Err(
                    "Decoded base palettes do not reproduce the selected ROM source".into(),
                );
            }
        }
        Ok(())
    }

    pub fn version(&self) -> Version {
        if self.manifest.profile == "usa" {
            Version::Usa
        } else {
            Version::Europe
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> [u8; 48] {
        let mut bytes = [0; 48];
        for i in 0..12 {
            bytes[i * 4..i * 4 + 2].copy_from_slice(&(8u16 << (i % 4)).to_le_bytes());
            bytes[i * 4 + 2..i * 4 + 4].copy_from_slice(&(8u16 << ((i + 1) % 4)).to_le_bytes());
        }
        bytes
    }

    #[test]
    fn dimensions_roundtrip_and_invalid_inputs() {
        let bytes = fixture();
        let decoded = ObjDimensions::decode(&bytes).unwrap();
        assert_eq!(decoded.source_bytes(), bytes);
        assert_eq!(decoded.get(2, 3), Some(&decoded.entries[11]));
        assert!(decoded.get(3, 0).is_none());
        assert!(decoded.get(0, 4).is_none());
        assert!(decoded.get(255, 255).is_none());
        for len in 0..48 {
            assert!(ObjDimensions::decode(&bytes[..len]).is_err());
        }
        assert!(ObjDimensions::decode(&[0; 49]).is_err());
        let mut invalid = bytes;
        invalid[0..2].copy_from_slice(&256u16.to_le_bytes());
        assert!(ObjDimensions::decode(&invalid).is_err());
        assert!(ObjDimensions::decode(&[0; 48]).is_err());
    }

    #[test]
    fn independent_aliases_bounds_and_base_authentication() {
        let bytes = fixture();
        let hash = ffta_rom::digest(&bytes);
        let mut rom = bytes.repeat(2);
        let ranges = [(0, "direct"), (48, "iwram-copy")];
        let extraction =
            Extraction::from_ranges(&rom, "synthetic".into(), "test", &ranges, &hash).unwrap();
        let mut manifest = extraction.manifest.clone();
        manifest.resources[0].sha256 = ffta_rom::digest(b"file");
        extraction
            .validate_files(BaseFiles {
                manifest: &manifest,
                dimensions: (&extraction.dimensions, b"file"),
                background: None,
                palettes: None,
            })
            .unwrap();
        let mut edited = extraction.dimensions.clone();
        edited.entries[0].width = 64;
        assert!(extraction
            .validate_files(BaseFiles {
                manifest: &manifest,
                dimensions: (&edited, b"file"),
                background: None,
                palettes: None
            })
            .is_err());
        for mutate in [0, 1, 2, 3, 4, 5, 6, 7] {
            let mut bad = manifest.clone();
            match mutate {
                0 => bad.schema += 1,
                1 => bad.decoder_revision += 1,
                2 => bad.extractor_revision += 1,
                3 => bad.source_rom_sha256 = "edited".into(),
                4 => bad.resources[0].path = "../escape".into(),
                5 => bad.resources.push(bad.resources[0].clone()),
                6 => bad.resources[0].sources[0].offset += 1,
                _ => bad.coverage.push("unverified".into()),
            }
            assert!(extraction
                .validate_files(BaseFiles {
                    manifest: &bad,
                    dimensions: (&extraction.dimensions, b"file"),
                    background: None,
                    palettes: None
                })
                .is_err());
        }
        assert!(extraction
            .validate_files(BaseFiles {
                manifest: &manifest,
                dimensions: (&extraction.dimensions, b"other"),
                background: None,
                palettes: None
            })
            .is_err());
        rom[48] ^= 1;
        assert!(Extraction::from_ranges(&rom, "test".into(), "test", &ranges, &hash).is_err());
        assert!(
            Extraction::from_ranges(&bytes[..47], "test".into(), "test", &ranges, &hash).is_err()
        );
        assert!(
            Extraction::from_ranges(&rom, "test".into(), "test", &[(u32::MAX, "bad")], &hash)
                .is_err()
        );
        assert!(Extraction::from_rom(&rom).is_err());
    }

    #[test]
    #[ignore = "requires the user's exact USA and EUR ROMs; no game data is bundled"]
    fn regional_extractions_are_independent_and_roundtrip() {
        for (env, version, ranges) in [
            ("FFTA_TEST_ROM_USA", Version::Usa, 2),
            ("FFTA_TEST_ROM_EUR", Version::Europe, 2),
        ] {
            let rom = std::fs::read(std::env::var(env).unwrap()).unwrap();
            let extraction = Extraction::from_rom(&rom).unwrap();
            assert_eq!(extraction.version(), version);
            assert_eq!(extraction.manifest.resources[0].sources.len(), ranges);
            for source in &extraction.manifest.resources[0].sources {
                assert_eq!(
                    &rom[source.offset as usize..source.offset as usize + 48],
                    &extraction.dimensions.source_bytes()
                );
            }
            let mut changed = rom;
            changed[0] ^= 1;
            assert!(Extraction::from_rom(&changed).is_err());
        }
    }
}
