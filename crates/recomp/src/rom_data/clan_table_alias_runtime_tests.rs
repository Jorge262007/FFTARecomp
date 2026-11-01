//! Focused EUR clan-table alias publication and original selector checks.
use super::*;
use gba_core::{capi::RT_API, Bus};

#[test]
#[ignore = "private EUR ROM and managed publication; Wine only, Linux/USA pending"]
fn european_clan_alias_managed_reads() {
    let root = PathBuf::from(std::env::var_os("FFTA_CLAN_ALIAS_ROOT").unwrap());
    assert!(!root.exists());
    fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior: Manifest = toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    assert_eq!((prior.extractor_revision, prior.decoder_revision, prior.resources.len()), (94, 93, 85));
    let mut source = Extraction::from_rom(&rom).unwrap();
    let dataset = root.join("offline dataset");
    extract(&dataset, &mut source).unwrap();
    assert_eq!((source.manifest.extractor_revision, source.manifest.decoder_revision, source.manifest.resources.len()), (106, 105, 96));
    assert_eq!(&source.manifest.coverage[..prior.coverage.len()], prior.coverage);
    let slot = source.manifest.resources.iter().position(|r| r.id == UI_CUSTOM_GRAPHICS_ID).unwrap();
    for (now, previous) in source.manifest.resources.iter().zip(&prior.resources) {
        assert_eq!(now.id, previous.id);
        if now.id != UI_CUSTOM_GRAPHICS_ID {
            assert_eq!(now, previous);
            assert_eq!(fs::read(dataset.join(&now.path)).unwrap(), fs::read(old.join(&previous.path)).unwrap());
        }
    }
    let entry = &source.manifest.resources[slot];
    assert_eq!(entry.sources.len(), prior.resources[slot].sources.len() + 5);
    assert_eq!(&entry.sources[..prior.resources[slot].sources.len()], prior.resources[slot].sources);
    let bytes = fs::read(dataset.join(UI_CUSTOM_GRAPHICS_FILE)).unwrap();
    let manifest = fs::read(dataset.join(MANIFEST_FILE)).unwrap();
    let current = parse_ui_graphics(&bytes).unwrap();
    let previous = parse_ui_graphics(&fs::read(old.join(UI_CUSTOM_GRAPHICS_FILE)).unwrap()).unwrap();
    assert_eq!(current.banks, previous.banks);
    assert_eq!(current.shared_obj_banks, previous.shared_obj_banks);
    assert_eq!(&current.selectors[..previous.selectors.len()], previous.selectors);
    let alias = current.selectors.last().unwrap();
    let clan = previous.selectors.iter().find(|s| s.id == "clan").unwrap();
    assert_eq!(alias.id, "clan-table-alias");
    assert_eq!(alias.fallback_variant, None);
    for i in 0..5 {
        assert_eq!(alias.pointers[i].source_offset, 0x71e5e0 + i as u32 * 4);
        assert_eq!(alias.pointers[i].bank_offset, clan.pointers[i].bank_offset);
        assert_eq!(alias.select(i as u8), Some(&alias.pointers[i]));
    }
    assert_eq!(alias.select(5), None);
    assert_eq!(alias.select(255), None);
    let mut reference = crate::make_machine(rom.clone(), None);
    let mut typed = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut typed, &dataset).unwrap();
    let initial = reference.save_state().unwrap();
    let owner = typed.bus.rom_resources().unwrap().clone();
    typed.bus.rom[0x71e5e0..0x71e5f4].fill(0xcd);
    for at in 0x71e5e0..0x71e5f4 {
        assert_eq!(typed.bus.read8(0x0c000000 + at as u32), rom[at]);
    }
    let pointer = &mut typed as *mut _ as *mut std::ffi::c_void;
    for edge in [0x71e5e0usize, 0x71e5f4] {
        for delta in [-3isize, -1, 0, 1] {
            let at = (edge as isize + delta) as usize;
            assert_eq!((RT_API.read16)(pointer, 0x0a000000 + at as u32), u32::from(u16::from_le_bytes(rom[at..at + 2].try_into().unwrap())));
            assert_eq!((RT_API.read32)(pointer, 0x08000000 + at as u32), u32::from_le_bytes(rom[at..at + 4].try_into().unwrap()));
        }
    }
    assert!(owner.ui_graphics_stats().unwrap().selector_bytes >= 20);
    typed.bus.rom.copy_from_slice(&rom);
    assert_eq!(typed.save_state().unwrap(), initial);
    // Run actual selector prefixes to their pointer-load boundary. Out-of-range
    // indices keep their original address arithmetic and any existing owners.
    let prefixes = [(0x23be6, 0x23bee, 4, 1), (0x32768, 0x32770, 0, 1),
        (0x4fc04, 0x4fc0c, 2, 1), (0x5e734, 0x5e73c, 0, 1),
        (0x70f26, 0x70f2e, 5, 6), (0x70f64, 0x70f6c, 5, 6)];
    for (start, end, address_reg, table_reg) in prefixes {
        for language in [0u8, 1, 2, 3, 4, 5, 255] {
            for m in [&mut reference, &mut typed] {
                m.load_state(&initial).unwrap();
                m.cpu.regs = std::array::from_fn(|i| 0x11000000 + i as u32);
                m.cpu.regs[address_reg] = 0x02001f6e;
                m.cpu.regs[table_reg] = 0x0871e5e0;
                m.cpu.regs[15] = 0x08000000 + start;
                m.cpu.cpsr = 0xbf;
                m.bus.ewram[0x1f6e] = language;
            }
            typed.bus.rom[0x71e5e0..0x71e5f4].fill(0xcd);
            for m in [&mut reference, &mut typed] {
                while m.cpu.regs[15] != 0x08000000 + end { m.step(); }
            }
            typed.bus.rom.copy_from_slice(&rom);
            assert_eq!(typed.save_state().unwrap(), reference.save_state().unwrap(), "prefix {start:06x}, language {language}");
        }
    }
    let mut bad = current.clone();
    bad.selectors.last_mut().unwrap().pointers[0].bank_offset = clan.pointers[1].bank_offset;
    let bad_bytes = encode_ui_graphics(&bad).unwrap();
    let mut bad_manifest = source.manifest.clone();
    bad_manifest.resources[slot].sha256 = gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(UI_CUSTOM_GRAPHICS_FILE), bad_bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE), toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();
    let before = typed.save_state().unwrap();
    assert!(attach_from_directory(&mut typed, &dataset).is_err());
    assert!(std::sync::Arc::ptr_eq(typed.bus.rom_resources().unwrap(), &owner));
    assert_eq!(typed.save_state().unwrap(), before);
    fs::write(dataset.join(UI_CUSTOM_GRAPHICS_FILE), bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE), manifest).unwrap();
    fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS", "alias_bytes":20,"existing_banks":5,"selector_prefixes":42,"full_state_clock":true,"Linux":"pending","USA":"pending","nativeWindows":"pending","performance_measurements":false})).unwrap()).unwrap();
}
