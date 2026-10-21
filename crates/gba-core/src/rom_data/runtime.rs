//! Immutable, authenticated base data. Only the verified OBJ/BG metadata ranges
//! are projected onto guest reads; no instruction, timing or DMA is skipped.
use super::*;
mod background;
mod palettes;
use background::BackgroundResources;
pub use background::BackgroundStats;
use palettes::PaletteResources;
pub use palettes::{PaletteRead, PaletteStats};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceStats {
    pub bus_reads: u64,
    pub bus_source_mask: u8,
    pub host_rom_reads: u64,
    pub host_ram_reads: u64,
    pub dynamic_ram_reads: u64,
    pub validation_reads: u64,
}

pub struct RuntimeResources {
    source_sha256: String,
    dataset_sha256: String,
    file_sha256: String,
    version: Version,
    dimensions: ObjDimensions,
    projection: [u8; 48],
    bus_reads: AtomicU64,
    bus_source_mask: AtomicU64,
    host_rom_reads: AtomicU64,
    host_ram_reads: AtomicU64,
    dynamic_ram_reads: AtomicU64,
    validation_reads: AtomicU64,
    background: Option<Box<BackgroundResources>>,
    palettes: Option<Box<PaletteResources>>,
}

impl RuntimeResources {
    /// The frontend supplies structures parsed from the very bytes whose file
    /// digest is in the manifest. Reauthenticate the base against the actual
    /// selected image; no editable/mod resource can enter through this API.
    pub fn from_base_files(rom: &[u8], files: BaseFiles<'_>) -> Result<Arc<Self>, String> {
        let source = Extraction::from_rom(rom)?;
        source.validate_files(files)?;
        let manifest = files.manifest;
        let dimensions = files.dimensions.0.clone();
        let background = files.background;
        let file_sha256 = manifest.resources[0].sha256.clone();
        let mut identity = b"FFTA-BASE-RESOURCES-v1\0".to_vec();
        identity.extend_from_slice(manifest.source_rom_sha256.as_bytes());
        identity.extend_from_slice(&manifest.schema.to_le_bytes());
        identity.extend_from_slice(&manifest.extractor_revision.to_le_bytes());
        identity.extend_from_slice(&manifest.decoder_revision.to_le_bytes());
        for resource in &manifest.resources {
            identity.extend_from_slice(resource.id.as_bytes());
            identity.push(0);
            identity.extend_from_slice(resource.sha256.as_bytes());
            identity.push(0);
        }
        let mut resources = Self::new(
            source.manifest.source_rom_sha256.clone(),
            ffta_rom::digest(&identity),
            file_sha256,
            source.version(),
            dimensions,
        );
        if let Some((decoded, _)) = background {
            let root = source.manifest.resources[1].sources[0].offset as usize;
            resources.background = Some(Box::new(BackgroundResources::new(
                root,
                decoded.clone(),
                manifest.resources[1].sha256.clone(),
            )?));
        }
        if let Some((decoded, _)) = files.palettes {
            resources.palettes = Some(Box::new(PaletteResources::new(
                decoded.clone(),
                &source.palettes.as_ref().unwrap().sources,
                manifest.resources[2].sha256.clone(),
            )?));
        }
        Ok(Arc::new(resources))
    }

    fn new(
        source_sha256: String,
        dataset_sha256: String,
        file_sha256: String,
        version: Version,
        dimensions: ObjDimensions,
    ) -> Self {
        let projection = dimensions.source_bytes();
        Self {
            source_sha256,
            dataset_sha256,
            file_sha256,
            version,
            dimensions,
            projection,
            bus_reads: AtomicU64::new(0),
            bus_source_mask: AtomicU64::new(0),
            host_rom_reads: AtomicU64::new(0),
            host_ram_reads: AtomicU64::new(0),
            dynamic_ram_reads: AtomicU64::new(0),
            validation_reads: AtomicU64::new(0),
            background: None,
            palettes: None,
        }
    }

    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn dataset_sha256(&self) -> &str {
        &self.dataset_sha256
    }
    pub fn file_sha256(&self) -> &str {
        &self.file_sha256
    }
    pub fn resource_id(&self) -> &'static str {
        OBJ_DIMENSIONS_ID
    }

    /// Shared/OBJ session allocation, excluding Arc's two reference counters.
    pub fn obj_resident_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.source_sha256.capacity()
            + self.dataset_sha256.capacity()
            + self.file_sha256.capacity()
    }

    pub fn background_resident_bytes(&self) -> usize {
        self.background.as_ref().map_or(0, |bg| bg.resident_bytes())
    }

    /// Complete owned session allocation, excluding the shared Arc headers.
    pub fn resident_bytes(&self) -> usize {
        self.obj_resident_bytes() + self.background_resident_bytes() + self.palette_resident_bytes()
    }

    pub fn background_file_sha256(&self) -> Option<&str> {
        self.background.as_ref().map(|bg| bg.file_sha256())
    }

    pub fn background_stats(&self) -> Option<BackgroundStats> {
        self.background.as_ref().map(|bg| bg.stats())
    }

    pub fn palette_resident_bytes(&self) -> usize {
        self.palettes
            .as_ref()
            .map_or(0, |palettes| palettes.resident_bytes())
    }
    pub fn palette_file_sha256(&self) -> Option<&str> {
        self.palettes
            .as_ref()
            .map(|palettes| palettes.file_sha256())
    }
    pub fn palette_stats(&self) -> Option<PaletteStats> {
        self.palettes.as_ref().map(|palettes| palettes.stats())
    }

    pub fn stats(&self) -> ResourceStats {
        ResourceStats {
            bus_reads: self.bus_reads.load(Ordering::Relaxed),
            bus_source_mask: self.bus_source_mask.load(Ordering::Relaxed) as u8,
            host_rom_reads: self.host_rom_reads.load(Ordering::Relaxed),
            host_ram_reads: self.host_ram_reads.load(Ordering::Relaxed),
            dynamic_ram_reads: self.dynamic_ram_reads.load(Ordering::Relaxed),
            validation_reads: self.validation_reads.load(Ordering::Relaxed),
        }
    }

    // This is a fixed regional mapping, never a manifest/range catalog search.
    fn source_offset(&self, index: usize, len: usize) -> Option<usize> {
        let overlaps = |base: usize| index < base + 48 && index.saturating_add(len) > base;
        match self.version {
            Version::Usa if overlaps(0x149510) => Some(0x149510),
            Version::Usa if overlaps(0xa398dc) => Some(0xa398dc),
            Version::Europe if overlaps(0x14ced8) => Some(0x14ced8),
            Version::Europe if overlaps(0xd976e8) => Some(0xd976e8),
            _ => None,
        }
    }

    /// Only the ROM read arms call this, after their GPIO/EEPROM and bounds
    /// checks, and only on the fixed metadata source pages. Crossing reads
    /// keep adjacent opaque ROM bytes without reading a migrated byte there.
    pub(crate) fn guest_read<const N: usize>(&self, index: usize, rom: &[u8]) -> Option<[u8; N]> {
        let Some(base) = self.source_offset(index, N) else {
            if let Some(bytes) = self
                .background
                .as_ref()
                .and_then(|background| background.guest_read(index, rom))
            {
                return Some(bytes);
            }
            return self.palettes.as_ref()?.guest_read(index, rom);
        };
        self.bus_reads.fetch_add(1, Ordering::Relaxed);
        let source_bit = if matches!(base, 0x149510 | 0x14ced8) {
            1
        } else {
            2
        };
        self.bus_source_mask.fetch_or(source_bit, Ordering::Relaxed);
        Some(std::array::from_fn(|i| {
            match index
                .checked_add(i)
                .and_then(|at| at.checked_sub(base))
                .and_then(|at| self.projection.get(at))
            {
                Some(&byte) => byte,
                None => rom[index + i],
            }
        }))
    }

    pub(crate) fn source_table(&self, offset: usize) -> Option<&[u8]> {
        if self.source_offset(offset, 48) != Some(offset) {
            return None;
        }
        self.validation_reads.fetch_add(1, Ordering::Relaxed);
        Some(&self.projection)
    }

    pub(crate) fn host_dimensions(
        &self,
        address: u32,
        ram_record: Option<&[u8]>,
    ) -> Option<(u16, u16)> {
        let offset = match address >> 24 {
            0x08..=0x0d => {
                let offset = (address & 0x1ffffff) as usize;
                let base = self.source_offset(offset, 4)?;
                let relative = offset.checked_sub(base)?;
                if relative % 4 != 0 || relative >= 48 {
                    return None;
                }
                self.host_rom_reads.fetch_add(1, Ordering::Relaxed);
                relative
            }
            0x03 if (0x03006d28..0x03006d58).contains(&address) && address & 3 == 0 => {
                let offset = (address - 0x03006d28) as usize;
                if ram_record != Some(&self.projection[offset..offset + 4]) {
                    self.dynamic_ram_reads.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                self.host_ram_reads.fetch_add(1, Ordering::Relaxed);
                offset
            }
            _ => return None,
        };
        let size = &self.dimensions.entries[offset / 4];
        Some((size.width, size.height))
    }

    #[cfg(test)]
    pub(crate) fn synthetic(rom: &[u8], version: Version, dimensions: ObjDimensions) -> Arc<Self> {
        Arc::new(Self::new(
            ffta_rom::digest(rom),
            "synthetic-dataset".into(),
            "synthetic-file".into(),
            version,
            dimensions,
        ))
    }
}

impl crate::MemMap {
    /// Transactional source binding. Base resources preserve the original image
    /// identity and guest bytes, so snapshots/native translations remain keyed
    /// to the original ROM. Modified resources require a different future API.
    pub fn set_rom_resources(
        &mut self,
        resources: Option<Arc<RuntimeResources>>,
    ) -> Result<(), String> {
        if let Some(data) = &resources {
            if data.source_sha256() != ffta_rom::digest(&self.rom) {
                return Err("Decoded game data belong to a different ROM".into());
            }
        }
        self.rom_resources = resources;
        Ok(())
    }

    pub fn rom_resources(&self) -> Option<&Arc<RuntimeResources>> {
        self.rom_resources.as_ref()
    }

    pub(crate) fn obj_dimensions(&self, address: u32) -> Option<(u16, u16)> {
        // Preserve illegal shape-3/out-of-table and genuinely changed guest RAM.
        let ram = if address >> 24 == 3 {
            let offset = (address & 0x7fff) as usize;
            self.iwram.get(offset..offset + 4)
        } else {
            None
        };
        self.rom_resources.as_ref()?.host_dimensions(address, ram)
    }

    pub(crate) fn obj_source_table(&self, offset: usize) -> Option<&[u8]> {
        if let Some(resources) = &self.rom_resources {
            if let Some(bytes) = resources.source_table(offset) {
                return Some(bytes);
            }
        }
        self.rom.get(offset..offset.checked_add(48)?)
    }

    #[inline]
    pub(crate) fn resource_page(index: usize) -> bool {
        matches!(index >> 12, 0x149 | 0x14c | 0xa39 | 0xd97
            | 0x569..=0x56c | 0x766..=0x769)
            || PaletteResources::source_page(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, Machine, MemMap};

    pub(crate) fn dimensions() -> ObjDimensions {
        ObjDimensions {
            entries: std::array::from_fn(|i| ObjSize {
                width: 8 << (i % 4),
                height: 8 << ((i + 1) % 4),
            }),
        }
    }

    fn machine(version: Version) -> (Machine, Arc<RuntimeResources>, Vec<usize>) {
        let dimensions = dimensions();
        let bases = match version {
            Version::Usa => vec![0x149510, 0xa398dc],
            Version::Europe => vec![0x14ced8, 0xd976e8],
        };
        let mut rom = vec![0x53; 0x1000000];
        for &base in &bases {
            rom[base..base + 48].copy_from_slice(&dimensions.source_bytes());
        }
        let resources = RuntimeResources::synthetic(&rom, version, dimensions);
        let mut machine = Machine::new(rom);
        machine
            .bus
            .set_rom_resources(Some(resources.clone()))
            .unwrap();
        (machine, resources, bases)
    }

    #[test]
    fn every_bus_width_mirror_boundary_uses_decoded_data_and_preserves_neighbors() {
        for version in [Version::Usa, Version::Europe] {
            let (mut machine, resources, bases) = machine(version);
            let original = machine.bus.rom.clone();
            for &base in &bases {
                machine.bus.rom[base..base + 48].fill(0xcd);
            }
            let before = machine.bus.clock;
            for &base in &bases {
                for mirror in [0x08000000, 0x0a000000, 0x0c000000] {
                    for offset in base - 4..base + 52 {
                        let address = mirror + offset as u32;
                        assert_eq!(machine.bus.read8(address), original[offset]);
                        assert_eq!(
                            machine.bus.read16(address),
                            u16::from_le_bytes(original[offset..offset + 2].try_into().unwrap())
                        );
                        assert_eq!(
                            machine.bus.read32(address),
                            u32::from_le_bytes(original[offset..offset + 4].try_into().unwrap())
                        );
                    }
                }
            }
            assert!(resources.stats().bus_reads > 0);
            assert_eq!(resources.stats().bus_source_mask, 3);
            assert_eq!(machine.bus.clock, before);
            assert!(resources.resident_bytes() + 2 * std::mem::size_of::<usize>() <= 1024);
            assert_eq!(machine.bus.read8(0x09000000), 0);
        }
    }

    #[test]
    fn original_cpu_instructions_and_dma_execute_with_unchanged_timing() {
        let (mut machine, resources, bases) = machine(Version::Usa);
        let rom = machine.bus.rom.clone();
        let mut reference = Machine::new(rom);
        // Original own-code LDR r0,[r1]; LDRH r2,[r1]; LDRB r3,[r1].
        for m in [&mut reference, &mut machine] {
            m.cpu.regs[15] = 0x02000000;
            m.cpu.regs[1] = 0x08149510;
            for (i, word) in [0xe5910000u32, 0xe1d120b0, 0xe5d13000]
                .into_iter()
                .enumerate()
            {
                m.bus.write32(0x02000000 + i as u32 * 4, word);
            }
        }
        for &base in &bases {
            machine.bus.rom[base..base + 48].fill(0xcd);
        }
        let pointer = &mut machine as *mut Machine as *mut std::ffi::c_void;
        assert_eq!(
            (crate::capi::RT_API.read8)(pointer, 0x08149510),
            resources.projection[0] as u32
        );
        assert_eq!(
            (crate::capi::RT_API.read16)(pointer, 0x08149510),
            u16::from_le_bytes(resources.projection[..2].try_into().unwrap()) as u32
        );
        assert_eq!(
            (crate::capi::RT_API.read32)(pointer, 0x08149510),
            u32::from_le_bytes(resources.projection[..4].try_into().unwrap())
        );
        for _ in 0..3 {
            reference.step();
            machine.step();
            assert_eq!(machine.cpu.regs, reference.cpu.regs);
            assert_eq!(machine.cpu.cpsr, reference.cpu.cpsr);
            assert_eq!(machine.bus.clock, reference.bus.clock);
        }
        for words in [false, true] {
            for m in [&mut reference, &mut machine] {
                m.bus.write32(0x040000d4, 0x08149510);
                m.bus.write32(0x040000d8, 0x03006d28);
                m.bus.write16(0x040000dc, if words { 12 } else { 24 });
                m.bus
                    .write16(0x040000de, if words { 0x8400 } else { 0x8000 });
            }
            assert_eq!(machine.bus.iwram, reference.bus.iwram);
            assert_eq!(machine.bus.clock, reference.bus.clock);
            assert_eq!(&machine.bus.iwram[0x6d28..0x6d58], &resources.projection);
        }
        assert!(resources.stats().bus_reads >= 39);
    }

    #[test]
    fn host_typed_and_ram_reads_preserve_dynamic_records_and_invalid_shapes() {
        let (mut machine, resources, bases) = machine(Version::Usa);
        for &base in &bases {
            machine.bus.rom[base..base + 48].fill(0xcd);
        }
        machine.bus.iwram[0x6d28..0x6d58].copy_from_slice(&resources.projection);
        for i in 0..12 {
            let expected = &resources.dimensions.entries[i];
            assert_eq!(
                machine.bus.obj_dimensions(0x08149510 + i as u32 * 4),
                Some((expected.width, expected.height))
            );
            assert_eq!(
                machine.bus.obj_dimensions(0x03006d28 + i as u32 * 4),
                Some((expected.width, expected.height))
            );
        }
        machine.bus.iwram[0x6d28..0x6d2a].copy_from_slice(&123u16.to_le_bytes());
        assert_eq!(machine.bus.obj_dimensions(0x03006d28), None);
        assert_eq!(machine.bus.obj_dimensions(0x03006d58), None);
        assert_eq!(machine.bus.obj_dimensions(0x08149540), None);
        assert_eq!(
            machine.bus.obj_source_table(0x149510),
            Some(resources.projection.as_slice())
        );
        assert_eq!(
            resources.stats(),
            ResourceStats {
                host_rom_reads: 12,
                host_ram_reads: 12,
                dynamic_ram_reads: 1,
                validation_reads: 1,
                ..ResourceStats::default()
            }
        );
    }

    #[test]
    fn snapshot_bytes_and_transactional_load_preserve_nonserialized_resources() {
        let (mut machine, resources, _) = machine(Version::Europe);
        let mut reference = Machine::new(machine.bus.rom.clone());
        let state = reference.save_state().unwrap();
        assert_eq!(state, machine.save_state().unwrap());
        machine.bus.read32(0x0814ced8);
        machine.bus.iwram[5] = 27;
        let count = resources.stats();
        let mut damaged = state.clone();
        *damaged.last_mut().unwrap() ^= 1;
        assert!(machine.load_state(&damaged).is_err());
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &resources
        ));
        assert_eq!(machine.bus.iwram[5], 27);
        machine.load_state(&state).unwrap();
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &resources
        ));
        assert_eq!(resources.stats(), count);
        assert_eq!(machine.save_state().unwrap(), state);
        reference
            .load_state(&machine.save_state().unwrap())
            .unwrap();
        let other = MemMap::new(vec![1; 256]);
        let mut incompatible = other;
        assert!(incompatible.set_rom_resources(Some(resources)).is_err());
        assert!(incompatible.rom_resources().is_none());
    }
}
