//! Original base-palette producers, including raw and compressed sources.
use super::*;
use gba_core::{rom_data::bg_palettes::Catalog, Bus};

#[test]
#[ignore = "requires exact private ROM and absent FFTA_STATE_TEST_PALETTE_ROOT"]
fn base_palette_sources_match_every_original_record() {
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_PALETTE_ROOT").unwrap());
    assert!(!root.exists(), "preserve previous palette evidence");
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let catalog = Catalog::from_rom(&rom).unwrap();
    let hash = gba_core::ffta_rom::digest(&rom);
    let entry = match hash.as_str() {
        gba_core::ffta_rom::SHA256 => 0x0801_a42c,
        gba_core::ffta_rom::EUROPE_SHA256 => 0x0801_b59c,
        _ => panic!("exact supported image required"),
    };
    let mut expected = Vec::new();
    let mut high_bit_words = 0;
    for palette in &catalog.decoded.entries {
        let bytes = palette.source_bytes().unwrap();
        high_bit_words += bytes
            .chunks_exact(2)
            .filter(|word| u16::from_le_bytes([word[0], word[1]]) & 0x8000 != 0)
            .count();
        expected.push(bytes);
    }
    assert!(high_bit_words > 0, "palette words must preserve bit15");
    let mut machine = crate::make_machine(rom, None);
    let mut calls = 0;
    let mut instructions = 0;
    const RETURN: u32 = 0x0800_0300;
    const DEST: usize = 0x3860;
    for environment in [0u8, 3, 0xfc, 0xff] {
        machine.bus.write8(0x0200_2fc2, environment);
        for (id, bytes) in expected.iter().enumerate() {
            machine.bus.write16(0x0200_7f10, id as u16);
            for argument in [id as u32, 0xffff] {
                machine.bus.iwram[DEST - 16..DEST + 512 + 16].fill(0xcd);
                let ewram = machine.bus.ewram.clone();
                machine.cpu.regs[0] = argument;
                machine.cpu.regs[1] = 0x0300_0000 + DEST as u32;
                machine.cpu.regs[13] = 0x0300_7c00;
                machine.cpu.regs[14] = RETURN | 1;
                machine.cpu.regs[15] = entry;
                machine.cpu.cpsr = 0xbf;
                let mut returned = false;
                for _ in 0..1000 {
                    machine.step();
                    instructions += 1;
                    if machine.cpu.regs[15] == RETURN {
                        returned = true;
                        break;
                    }
                }
                assert!(returned, "original palette producer must return");
                assert_eq!(machine.cpu.regs[13], 0x0300_7c00);
                assert!(
                    machine.bus.ewram == ewram,
                    "base producer leaves EWRAM unchanged"
                );
                assert_eq!(
                    &machine.bus.iwram[DEST..DEST + bytes.len()],
                    bytes,
                    "record={id} env={environment} argument={argument}"
                );
                assert!(machine.bus.iwram[DEST - 16..DEST]
                    .iter()
                    .all(|&b| b == 0xcd));
                assert!(machine.bus.iwram[DEST + bytes.len()..DEST + 512 + 16]
                    .iter()
                    .all(|&b| b == 0xcd));
                calls += 1;
            }
        }
    }
    assert_eq!(gba_core::ffta_rom::digest(&machine.bus.rom), hash);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "source_rom_sha256": hash, "records": expected.len(), "calls": calls,
        "instructions": instructions, "high_bit_words_in_record_catalog": high_bit_words,
        "complete_output_and_guards_match": true, "ewram_unchanged": true,
        "scope": "original base-environment producer; alternate programs/runtime replacement not covered"
    })).unwrap()).unwrap();
    eprintln!("BG_PALETTE_ORIGINAL records=162 calls={calls} instructions={instructions} high_bit_words={high_bit_words} guards=true ewram_unchanged=true");
}
