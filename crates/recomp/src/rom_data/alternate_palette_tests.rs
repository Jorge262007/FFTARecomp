//! Original alternate-palette producers against private bounded-decoder output.
use super::*;
use gba_core::{rom_data::bg_descriptors::Catalog, Bus};

#[test]
#[ignore = "requires exact ROM, alternate inventory and absent FFTA_STATE_TEST_ALT_PALETTE_ROOT"]
fn alternate_palette_catalog_matches_original_producers() {
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_ALT_PALETTE_ROOT").unwrap());
    assert!(!root.exists(), "preserve previous oracle evidence");
    let inventory =
        PathBuf::from(std::env::var_os("FFTA_STATE_TEST_ALT_PALETTE_INVENTORY").unwrap());
    let data: serde_json::Value =
        serde_json::from_slice(&fs::read(inventory.join("inventory.json")).unwrap()).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let hash = gba_core::ffta_rom::digest(&rom);
    assert_eq!(data["source_sha256"].as_str().unwrap(), hash);
    let catalog = Catalog::from_rom(&rom).unwrap();
    let (entry, consumer, literals, tables) = match hash.as_str() {
        gba_core::ffta_rom::SHA256 => (
            0x0801_a42c,
            0x0800_543c,
            [0x1a4f8usize, 0x1a514],
            [0x427a04u32, 0x42cc80],
        ),
        gba_core::ffta_rom::EUROPE_SHA256 => (
            0x0801_b59c,
            0x0800_54a0,
            [0x1b668usize, 0x1b684],
            [0x489bb0u32, 0x48ee2c],
        ),
        _ => panic!("exact supported ROM required"),
    };
    for (literal, table) in literals.into_iter().zip(tables) {
        assert_eq!(
            &rom[literal..literal + 4],
            &(0x0800_0000 + table).to_le_bytes()
        );
    }
    assert_eq!(data["consumer"].as_u64().unwrap(), consumer as u64);
    let keys: Vec<_> = catalog
        .decoded
        .entries
        .iter()
        .map(|e| e.alternate_palette_key)
        .collect();
    assert_eq!(keys.iter().filter(|&&key| key == 255).count(), 5);
    assert_eq!(
        keys.iter()
            .copied()
            .filter(|&key| key != 255)
            .collect::<std::collections::BTreeSet<_>>(),
        (0..157u8).collect()
    );
    let mut machine = crate::make_machine(rom, None);
    const RETURN: u32 = 0x0800_0300;
    const DEST: usize = 0x3860;
    let mut calls = 0u64;
    let mut instructions = 0u64;
    for environment in [1u8, 2, 0xfd, 0xfe] {
        machine.bus.write8(0x0200_2fc2, environment);
        let group = usize::from(environment & 3 == 1);
        for (id, &key) in keys.iter().enumerate() {
            if key == 255 {
                // The original producer has no local guard. The caller's
                // exclusion contract remains unknown; do not invoke it here.
                continue;
            }
            let row = &data["tables"][group]["sources"][key as usize];
            let offset = row["offset"].as_u64().unwrap() as usize;
            let length = row["source_length"].as_u64().unwrap() as usize;
            let table = tables[group] as usize;
            assert_eq!(
                offset,
                table
                    + u16::from_le_bytes(
                        machine.bus.rom[table + key as usize * 2..table + key as usize * 2 + 2]
                            .try_into()
                            .unwrap()
                    ) as usize
            );
            assert_eq!(
                gba_core::ffta_rom::digest(&machine.bus.rom[offset..offset + length]),
                row["source_sha256"].as_str().unwrap()
            );
            let bytes = fs::read(inventory.join(row["file"].as_str().unwrap())).unwrap();
            assert_eq!(bytes.len(), 256);
            assert_eq!(
                gba_core::ffta_rom::digest(&bytes),
                row["decoded_sha256"].as_str().unwrap()
            );
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
                let mut entered = false;
                for _ in 0..50_000 {
                    machine.bus.frame_ready = false;
                    if machine.cpu.regs[15] == consumer {
                        assert!(!entered);
                        entered = true;
                        assert_eq!(machine.cpu.regs[0], 0x0300_0000 + DEST as u32);
                        assert_eq!(machine.cpu.regs[1], 0x0800_0000 + offset as u32);
                    }
                    machine.step();
                    instructions += 1;
                    if machine.cpu.regs[15] == RETURN {
                        returned = true;
                        break;
                    }
                }
                assert!(
                    returned && entered,
                    "original producer must use its consumer and return"
                );
                assert_eq!(machine.cpu.regs[13], 0x0300_7c00);
                assert!(machine.bus.ewram == ewram);
                assert_eq!(
                    &machine.bus.iwram[DEST..DEST + 256],
                    bytes,
                    "id={id} environment={environment}"
                );
                assert!(machine.bus.iwram[DEST - 16..DEST]
                    .iter()
                    .all(|&b| b == 0xcd));
                assert!(machine.bus.iwram[DEST + 256..DEST + 512 + 16]
                    .iter()
                    .all(|&b| b == 0xcd));
                calls += 1;
            }
        }
    }
    assert_eq!(calls, 1256);
    assert_eq!(gba_core::ffta_rom::digest(&machine.bus.rom), hash);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "source_rom_sha256":hash,"calls":calls,"original_instructions":instructions,
        "valid_records":157,"tables":2,"source_streams":314,"output_bytes_each":256,
        "original_consumer_entered":true,"complete_output_and_guards_match":true,
        "ewram_unchanged":true,"stack_restored":true,"unavailable_records_not_invoked":5,
        "scope":"original-output inventory oracle; typed production codec/runtime migration pending"
    })).unwrap()).unwrap();
    eprintln!("ALT_PALETTE_ORIGINAL calls={calls} original_instructions={instructions} streams=314 guards=true ewram_unchanged=true");
}
