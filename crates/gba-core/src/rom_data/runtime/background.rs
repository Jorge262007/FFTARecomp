//! Immutable descriptor ownership and its fixed source projection. No I/O.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackgroundStats {
    pub bus_reads: u64,
    pub projected_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, Machine};

    fn fixture(version: Version) -> (Machine, Arc<RuntimeResources>, usize, usize) {
        let root = match version {
            Version::Usa => 0x569104,
            Version::Europe => 0x7662a8,
        };
        let absent = bg_descriptors::TransferLink {
            control_source: 0,
            data_source: 0,
            destination: 0,
        };
        let entries = (0..bg_descriptors::BASE_RECORDS)
            .map(|id| {
                let source = 0x10000 + id as u32 * 64;
                bg_descriptors::Descriptor {
                    graphics_source: source,
                    paired_planes_source: source + 4,
                    halfword_planes_source: source + 8,
                    palette_source: source + 12,
                    auxiliary_patches_source: source + 16,
                    transfer: absent.clone(),
                    extra_transfers: std::array::from_fn(|_| absent.clone()),
                    variant_source: 0,
                    selectors: [id as u8, (id + 19) as u8],
                    alternate_palette_key: (id * 7) as u8,
                    trailing_byte: (id * 13) as u8,
                }
            })
            .collect();
        let descriptors = bg_descriptors::Descriptors { entries };
        let bytes = descriptors.source_bytes().unwrap();
        let len = bytes.len();
        let dimensions = super::super::tests::dimensions();
        let obj_bases = match version {
            Version::Usa => [0x149510, 0xa398dc],
            Version::Europe => [0x14ced8, 0xd976e8],
        };
        let mut rom = vec![0x53; 0x1000000];
        for base in obj_bases {
            rom[base..base + 48].copy_from_slice(&dimensions.source_bytes());
        }
        rom[root..root + len].copy_from_slice(&bytes);
        let mut resources = RuntimeResources::new(
            ffta_rom::digest(&rom),
            "synthetic-dataset".into(),
            "synthetic-obj".into(),
            version,
            dimensions,
        );
        resources.background = Some(Box::new(
            BackgroundResources::new(root, descriptors, "synthetic-bg".into()).unwrap(),
        ));
        let resources = Arc::new(resources);
        let mut machine = Machine::new(rom);
        machine
            .bus
            .set_rom_resources(Some(resources.clone()))
            .unwrap();
        (machine, resources, root, len)
    }

    #[test]
    fn all_widths_waitstate_mirrors_edges_and_native_api_use_bg_projection() {
        for version in [Version::Usa, Version::Europe] {
            let (mut machine, resources, root, len) = fixture(version);
            let original = machine.bus.rom.clone();
            machine.bus.rom[root..root + len].fill(0xcd);
            let pointer = &mut machine as *mut Machine as *mut std::ffi::c_void;
            let clock = machine.bus.clock;
            for mirror in [0x08000000u32, 0x0a000000, 0x0c000000] {
                for offset in root - 4..root + len + 4 {
                    let address = mirror + offset as u32;
                    let half = u16::from_le_bytes(original[offset..offset + 2].try_into().unwrap());
                    let word = u32::from_le_bytes(original[offset..offset + 4].try_into().unwrap());
                    assert_eq!(machine.bus.read8(address), original[offset]);
                    assert_eq!(machine.bus.read16(address), half);
                    assert_eq!(machine.bus.read32(address), word);
                    assert_eq!(
                        (crate::capi::RT_API.read8)(pointer, address),
                        original[offset] as u32
                    );
                    assert_eq!((crate::capi::RT_API.read16)(pointer, address), half as u32);
                    assert_eq!((crate::capi::RT_API.read32)(pointer, address), word);
                }
            }
            let stats = resources.background_stats().unwrap();
            assert!(stats.bus_reads > (len * 3) as u64);
            assert!(stats.projected_bytes > stats.bus_reads);
            assert_eq!(machine.bus.clock, clock);
            assert_eq!(resources.stats(), ResourceStats::default());
            let wrong_root = if version == Version::Usa {
                0x7662a8
            } else {
                0x569104
            };
            assert_eq!(machine.bus.read32(0x08000000 + wrong_root), 0x53535353);
            assert_eq!(resources.background_stats().unwrap(), stats);
            assert!(resources.obj_resident_bytes() + 2 * std::mem::size_of::<usize>() <= 1024);
            assert!(resources.background_resident_bytes() <= 64 * 1024);
        }
    }

    #[test]
    fn cpu_and_both_dma_widths_copy_complete_file_backed_bg_with_legacy_timing() {
        for version in [Version::Usa, Version::Europe] {
            let (mut machine, resources, root, len) = fixture(version);
            let original = machine.bus.rom.clone();
            let mut legacy = Machine::new(original.clone());
            for target in [&mut machine, &mut legacy] {
                target.cpu.regs[15] = 0x02000000;
                target.cpu.regs[1] = 0x08000000 + root as u32;
                for (index, word) in [0xe5910000u32, 0xe1d120b0, 0xe5d13000]
                    .into_iter()
                    .enumerate()
                {
                    target.bus.write32(0x02000000 + index as u32 * 4, word);
                }
            }
            machine.bus.rom[root..root + len].fill(0xcd);
            for _ in 0..3 {
                legacy.step();
                machine.step();
                assert_eq!(machine.cpu.regs, legacy.cpu.regs);
                assert_eq!(machine.cpu.cpsr, legacy.cpu.cpsr);
                assert_eq!(machine.bus.clock, legacy.bus.clock);
            }
            for words in [false, true] {
                for target in [&mut machine, &mut legacy] {
                    target.bus.write32(0x040000d4, 0x08000000 + root as u32);
                    target.bus.write32(0x040000d8, 0x02014000);
                    target
                        .bus
                        .write16(0x040000dc, (len / if words { 4 } else { 2 }) as u16);
                    target
                        .bus
                        .write16(0x040000de, if words { 0x8400 } else { 0x8000 });
                }
                assert_eq!(machine.bus.ewram, legacy.bus.ewram);
                assert_eq!(machine.bus.clock, legacy.bus.clock);
                assert_eq!(
                    &machine.bus.ewram[0x14000..0x14000 + len],
                    &original[root..root + len]
                );
            }
            machine.bus.rom[root..root + len].copy_from_slice(&original[root..root + len]);
            assert!(machine.save_state().unwrap() == legacy.save_state().unwrap());
            assert!(resources.background_stats().unwrap().projected_bytes >= 2 * len as u64);
            assert_eq!(resources.stats(), ResourceStats::default());
        }
    }

    #[test]
    fn snapshots_preserve_bg_owner_counters_and_do_not_serialize_metadata() {
        let (mut machine, resources, root, _) = fixture(Version::Europe);
        let legacy = Machine::new(machine.bus.rom.clone());
        let clean = legacy.save_state().unwrap();
        assert_eq!(machine.save_state().unwrap(), clean);
        machine.bus.read32(0x08000000 + root as u32);
        let stats = resources.background_stats();
        machine.bus.ewram[11] = 23;
        let before = machine.save_state().unwrap();
        let mut bad = clean.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(machine.load_state(&bad).is_err());
        assert_eq!(machine.save_state().unwrap(), before);
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &resources
        ));
        machine.load_state(&clean).unwrap();
        assert_eq!(machine.save_state().unwrap(), clean);
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &resources
        ));
        assert_eq!(resources.background_stats(), stats);
        assert_eq!(resources.background_file_sha256(), Some("synthetic-bg"));
    }
}

pub(super) struct BackgroundResources {
    root: usize,
    descriptors: bg_descriptors::Descriptors,
    projection: Vec<u8>,
    file_sha256: String,
    bus_reads: AtomicU64,
    projected_bytes: AtomicU64,
}

impl BackgroundResources {
    pub(super) fn new(
        root: usize,
        descriptors: bg_descriptors::Descriptors,
        file_sha256: String,
    ) -> Result<Self, String> {
        let projection = descriptors.source_bytes()?;
        Ok(Self {
            root,
            descriptors,
            projection,
            file_sha256,
            bus_reads: AtomicU64::new(0),
            projected_bytes: AtomicU64::new(0),
        })
    }

    pub(super) fn resident_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.descriptors.entries.capacity()
                * std::mem::size_of::<bg_descriptors::Descriptor>()
            + self.projection.capacity()
            + self.file_sha256.capacity()
    }

    pub(super) fn file_sha256(&self) -> &str {
        &self.file_sha256
    }

    pub(super) fn stats(&self) -> BackgroundStats {
        BackgroundStats {
            bus_reads: self.bus_reads.load(Ordering::Relaxed),
            projected_bytes: self.projected_bytes.load(Ordering::Relaxed),
        }
    }

    pub(super) fn guest_read<const N: usize>(&self, index: usize, rom: &[u8]) -> Option<[u8; N]> {
        let end = self.root + self.projection.len();
        if index >= end || index.saturating_add(N) <= self.root {
            return None;
        }
        let mapped = end.min(index + N) - self.root.max(index);
        self.bus_reads.fetch_add(1, Ordering::Relaxed);
        self.projected_bytes
            .fetch_add(mapped as u64, Ordering::Relaxed);
        Some(std::array::from_fn(|i| {
            let at = index + i;
            if (self.root..end).contains(&at) {
                self.projection[at - self.root]
            } else {
                rom[at]
            }
        }))
    }
}
