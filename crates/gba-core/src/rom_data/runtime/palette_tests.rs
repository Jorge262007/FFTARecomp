use super::*;
use crate::{Bus, Machine};

type Spans = Vec<(usize, usize)>;
fn encoded(kind: u8, seed: u16) -> Vec<u8> {
    let bytes: Vec<_> = (0..32u16)
        .flat_map(|i| (0x8000 | seed.wrapping_add(i * 17)).to_le_bytes())
        .collect();
    let mut source = vec![kind, 64, 0, 0];
    if kind == 0 {
        source.extend_from_slice(&bytes);
    } else {
        for chunk in bytes.chunks(8) {
            source.push(0);
            source.extend_from_slice(chunk);
        }
    }
    source
}
fn fixture(version: Version) -> (Machine, Arc<RuntimeResources>, Spans, Vec<u8>) {
    let first = if version == Version::Usa {
        0x56f4a8
    } else {
        0x76c64c
    };
    let offsets = [first, first + 0x100];
    let dims = super::super::tests::dimensions();
    let mut rom = vec![0x53; 0x1000000];
    for offset in if version == Version::Usa {
        [0x149510, 0xa398dc]
    } else {
        [0x14ced8, 0xd976e8]
    } {
        rom[offset..offset + 48].copy_from_slice(&dims.source_bytes());
    }
    let mut entries = Vec::new();
    let mut sources = Vec::new();
    let mut spans = Vec::new();
    for (id, (offset, kind)) in offsets.into_iter().zip([0, 0x10]).enumerate() {
        let input = encoded(kind, id as u16 * 71);
        rom[offset..offset + input.len()].copy_from_slice(&input);
        let block = bg_palettes::decode(&input).unwrap();
        sources.push(bg_palettes::PaletteSource {
            range: SourceRange {
                offset: offset as u32,
                length: input.len() as u32,
                sha256: ffta_rom::digest(&input),
                role: format!("synthetic/{id}"),
            },
            encoding: block.encoding,
        });
        entries.push(block.palette);
        spans.push((offset, input.len()));
    }
    // A valid unregistered ROM stream and a shared-source alias.
    let unrelated = encoded(0x10, 1234);
    rom[first + 0x200..first + 0x200 + unrelated.len()].copy_from_slice(&unrelated);
    entries.push(entries[0].clone());
    sources.push(bg_palettes::PaletteSource {
        range: sources[0].range.clone(),
        encoding: sources[0].encoding,
    });
    let mut resources = RuntimeResources::new(
        ffta_rom::digest(&rom),
        "synthetic-dataset".into(),
        "synthetic-obj".into(),
        version,
        dims,
    );
    resources.palettes = Some(Box::new(
        PaletteResources::new(
            bg_palettes::Palettes { entries },
            &sources,
            "synthetic-palette".into(),
        )
        .unwrap(),
    ));
    let resources = Arc::new(resources);
    let mut machine = Machine::new(rom.clone());
    machine
        .bus
        .set_rom_resources(Some(resources.clone()))
        .unwrap();
    (machine, resources, spans, rom)
}
fn poison(machine: &mut Machine, spans: &Spans) {
    for &(at, len) in spans {
        machine.bus.rom[at..at + len].fill(0xcd);
    }
}
fn restore(machine: &mut Machine, spans: &Spans, rom: &[u8]) {
    for &(at, len) in spans {
        machine.bus.rom[at..at + len].copy_from_slice(&rom[at..at + len]);
    }
}

#[test]
fn all_source_bytes_widths_mirrors_edges_and_native_api_are_file_backed() {
    for version in [Version::Usa, Version::Europe] {
        let (mut machine, resources, spans, original) = fixture(version);
        poison(&mut machine, &spans);
        let clock = machine.bus.clock;
        for real_bios in [false, true] {
            machine.bus.real_bios = real_bios;
            for mirror in [0x08000000u32, 0x0a000000, 0x0c000000] {
                for &(start, len) in &spans {
                    for index in start - 4..start + len + 4 {
                        let address = mirror + index as u32;
                        let ptr = &mut machine as *mut Machine as *mut std::ffi::c_void;
                        assert_eq!(machine.bus.read8(address), original[index]);
                        assert_eq!(
                            machine.bus.read16(address),
                            u16::from_le_bytes(original[index..index + 2].try_into().unwrap())
                        );
                        assert_eq!(
                            machine.bus.read32(address),
                            u32::from_le_bytes(original[index..index + 4].try_into().unwrap())
                        );
                        assert_eq!(
                            (crate::capi::RT_API.read8)(ptr, address),
                            original[index] as u32
                        );
                        assert_eq!(
                            (crate::capi::RT_API.read16)(ptr, address),
                            u16::from_le_bytes(original[index..index + 2].try_into().unwrap())
                                as u32
                        );
                        assert_eq!(
                            (crate::capi::RT_API.read32)(ptr, address),
                            u32::from_le_bytes(original[index..index + 4].try_into().unwrap())
                        );
                    }
                }
            }
        }
        assert_eq!(machine.bus.clock, clock);
        assert_eq!(resources.stats(), ResourceStats::default());
        assert!(resources.palette_stats().unwrap().source_bytes > 0);
        assert!(machine
            .bus
            .decoded_base_palette(0x08000000 + spans[0].0 as u32)
            .is_none());
        assert!(machine
            .bus
            .decoded_base_palette(0x02000000 + spans[1].0 as u32)
            .is_none());
    }
}

#[test]
fn lz77_swis_and_raw_cpuset_dma_preserve_full_states_and_guest_timing() {
    for version in [Version::Usa, Version::Europe] {
        let (mut decoded, resources, spans, rom) = fixture(version);
        let mut legacy = Machine::new(rom.clone());
        for swi in [0x11u16, 0x12, 0x0b] {
            for dest in [
                0x03003860u32,
                0x02012000,
                0x05000020,
                0x06000020,
                0x06000021,
                0x07000020,
            ] {
                for mirror in [0x08000000u32, 0x0a000000, 0x0c000000] {
                    for target in [&mut legacy, &mut decoded] {
                        target.cpu.cpsr = 0xbf;
                        target.cpu.regs[15] = 0x02000000;
                        target.cpu.regs[0] = mirror
                            + if swi == 0x0b {
                                spans[0].0 as u32 + 4
                            } else {
                                spans[1].0 as u32
                            };
                        target.cpu.regs[1] = dest;
                        target.cpu.regs[2] = 32;
                        target.bus.write16(0x02000000, 0xdf00 | swi);
                    }
                    poison(&mut decoded, &spans);
                    legacy.step();
                    decoded.step();
                    restore(&mut decoded, &spans, &rom);
                    assert!(
                        legacy.save_state().unwrap() == decoded.save_state().unwrap(),
                        "swi={swi} dest={dest:08x}"
                    );
                }
            }
        }
        assert_eq!(resources.palette_stats().unwrap().lz77_calls, 36);
        for word in [false, true] {
            poison(&mut decoded, &spans);
            for target in [&mut legacy, &mut decoded] {
                target
                    .bus
                    .write32(0x040000d4, 0x08000000 + spans[0].0 as u32 + 4);
                target.bus.write32(0x040000d8, 0x02014000);
                target.bus.write16(0x040000dc, if word { 16 } else { 32 });
                target
                    .bus
                    .write16(0x040000de, if word { 0x8400 } else { 0x8000 });
            }
            restore(&mut decoded, &spans, &rom);
            assert!(legacy.save_state().unwrap() == decoded.save_state().unwrap());
        }
        // Unregistered ROM streams and mutable RAM retain the old decoder.
        for source in [0x08000000 + spans[0].0 as u32 + 0x200, 0x02001000] {
            let input = encoded(0x10, 2345);
            for target in [&mut legacy, &mut decoded] {
                if source >> 24 == 2 {
                    for (i, &byte) in input.iter().enumerate() {
                        target.bus.write8(source + i as u32, byte);
                    }
                }
                target.cpu.regs[0] = source;
                target.cpu.regs[1] = 0x03003860;
                target.cpu.regs[15] = 0x02000000;
                target.bus.write16(0x02000000, 0xdf11);
            }
            let count = resources.palette_stats().unwrap().lz77_calls;
            legacy.step();
            decoded.step();
            assert_eq!(resources.palette_stats().unwrap().lz77_calls, count);
            assert!(legacy.save_state().unwrap() == decoded.save_state().unwrap());
        }
    }
}

#[test]
fn snapshot_and_owned_reads_keep_palette_identity_and_counters() {
    let (mut machine, resources, spans, rom) = fixture(Version::Europe);
    let legacy = Machine::new(rom);
    let snapshot = machine.save_state().unwrap();
    assert_eq!(snapshot, legacy.save_state().unwrap());
    let read = machine
        .bus
        .decoded_base_palette(0x08000000 + spans[1].0 as u32)
        .unwrap();
    let bytes = read.bytes().to_vec();
    let stats = resources.palette_stats().unwrap();
    machine.cpu.regs[4] = 123;
    let mut bad = snapshot.clone();
    let last = bad.len() - 1;
    bad[last] ^= 0x80;
    assert!(machine.load_state(&bad).is_err());
    assert_eq!(machine.cpu.regs[4], 123);
    machine.load_state(&snapshot).unwrap();
    assert!(Arc::ptr_eq(
        machine.bus.rom_resources().unwrap(),
        &resources
    ));
    assert_eq!(resources.palette_stats().unwrap(), stats);
    assert_eq!(resources.palette_file_sha256(), Some("synthetic-palette"));
    machine.bus.set_rom_resources(None).unwrap();
    assert_eq!(read.bytes(), bytes);
}
