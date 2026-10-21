//! Regional original-code oracles for the bounded descriptor metadata contract.
use super::*;
use gba_core::{
    bus::Bus,
    rom_data::bg_descriptors::{Catalog, Descriptors, BASE_RECORDS, RECORD_BYTES},
};

#[test]
#[ignore = "requires exact private ROM and absent FFTA_STATE_TEST_CATALOG_ROOT; no boot replay"]
fn descriptor_catalog_matches_source_and_original_selectors() {
    let root = PathBuf::from(
        std::env::var_os("FFTA_STATE_TEST_CATALOG_ROOT").expect("new catalog output root"),
    );
    assert!(!root.exists(), "preserve earlier catalog evidence");
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
    let source_hash = gba_core::ffta_rom::digest(&rom);
    let entry = match source_hash.as_str() {
        gba_core::ffta_rom::SHA256 => 0x0801_db98,
        gba_core::ffta_rom::EUROPE_SHA256 => 0x0801_ed08,
        _ => panic!("exact supported image required"),
    };
    let catalog = Catalog::from_rom(&rom).unwrap();
    assert_eq!(catalog.decoded.entries.len(), BASE_RECORDS);
    let offset = catalog.source.offset as usize;
    let length = catalog.source.length as usize;
    assert_eq!(
        catalog.decoded.source_bytes().unwrap(),
        rom[offset..offset + length]
    );
    assert!(
        Descriptors::decode(
            &rom[offset..offset + length + RECORD_BYTES],
            catalog.source.offset,
            rom.len()
        )
        .is_err(),
        "first payload is not a 163rd descriptor"
    );
    let encoded = toml::to_string_pretty(&catalog.decoded).unwrap();
    assert_eq!(encoded, toml::to_string_pretty(&catalog.decoded).unwrap());
    let reparsed: Descriptors = toml::from_str(&encoded).unwrap();
    reparsed.validate(catalog.source.offset, rom.len()).unwrap();
    assert_eq!(reparsed, catalog.decoded);
    assert_eq!(
        reparsed.source_bytes().unwrap(),
        rom[offset..offset + length]
    );
    let mut changed = rom.clone();
    changed[offset + 85] ^= 1;
    assert!(
        Catalog::from_rom(&changed).is_err(),
        "edited base must not pass source authentication"
    );

    let mut machine = crate::make_machine(rom, None);
    let mut expected = machine.bus.ewram.clone();
    let mut cases = 0;
    let mut steps = 0;
    for environment in [0u8, 1, 2, 3, 0xfc, 0xfd, 0xfe, 0xff] {
        machine.bus.write8(0x0200_2fc2, environment);
        expected[0x2fc2] = environment;
        for (id, descriptor) in catalog.decoded.entries.iter().enumerate() {
            machine.bus.write16(0x0200_7f10, id as u16);
            expected[0x7f10..0x7f12].copy_from_slice(&(id as u16).to_le_bytes());
            for argument in [id as u32, 0xffff] {
                steps += invoke(&mut machine, entry, argument);
                assert_eq!(
                    machine.cpu.regs[0],
                    descriptor.selected_selector(environment) as u32,
                    "id={id} environment={environment} argument={argument}"
                );
                assert!(
                    machine.bus.ewram == expected,
                    "selector must be read-only in EWRAM"
                );
                cases += 1;
            }
        }
        machine.bus.write16(0x0200_7f10, 0xff);
        expected[0x7f10..0x7f12].copy_from_slice(&0xffu16.to_le_bytes());
        for argument in [0xff, 0xffff] {
            steps += invoke(&mut machine, entry, argument);
            assert_eq!(machine.cpu.regs[0], 0xff);
            assert!(machine.bus.ewram == expected);
            cases += 1;
        }
    }
    // No base record has an alternate selector of FF. Verify the fallback with
    // a post-authentication counterfactual in memory, never a supported dataset.
    let original = machine.bus.rom[offset + 85];
    machine.bus.rom[offset + 85] = 0xff;
    let mut fallback = catalog.decoded.entries[0].clone();
    fallback.selectors[1] = 0xff;
    for environment in 0u8..4 {
        machine.bus.write8(0x0200_2fc2, environment);
        expected[0x2fc2] = environment;
        steps += invoke(&mut machine, entry, 0);
        assert_eq!(
            machine.cpu.regs[0],
            fallback.selected_selector(environment) as u32
        );
        assert!(machine.bus.ewram == expected);
        cases += 1;
    }
    machine.bus.rom[offset + 85] = original;
    assert_eq!(gba_core::ffta_rom::digest(&machine.bus.rom), source_hash);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("bg-descriptors.toml"), &encoded).unwrap();
    #[derive(serde::Serialize)]
    struct Report {
        source_rom_sha256: String,
        table: gba_core::rom_data::SourceRange,
        records: usize,
        selector_cases: usize,
        instructions: usize,
        decoded_file_sha256: String,
    }
    let report = Report {
        source_rom_sha256: source_hash,
        table: catalog.source,
        records: BASE_RECORDS,
        selector_cases: cases,
        instructions: steps,
        decoded_file_sha256: gba_core::ffta_rom::digest(encoded.as_bytes()),
    };
    fs::write(
        root.join("result.toml"),
        toml::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    eprintln!("BG_DESCRIPTORS records={BASE_RECORDS} selector_cases={cases} instructions={steps} source_roundtrip=true deterministic_toml=true extended_prefix_rejected=true edited_source_rejected=true");
}

fn invoke(machine: &mut gba_core::Machine, entry: u32, argument: u32) -> usize {
    const RETURN: u32 = 0x0800_0300;
    machine.cpu.regs[0] = argument;
    machine.cpu.regs[13] = 0x0300_7c00;
    machine.cpu.regs[14] = RETURN | 1;
    machine.cpu.regs[15] = entry;
    machine.cpu.cpsr = 0xbf;
    for steps in 1..1000 {
        machine.step();
        if machine.cpu.regs[15] == RETURN {
            return steps;
        }
    }
    panic!("original selector did not return");
}
