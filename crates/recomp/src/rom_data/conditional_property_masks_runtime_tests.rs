//! Managed EUR mask projection, mixed-owner edges and original native prefix.
use super::*;
use gba_core::{Bus, Machine, capi::RT_API};
use gba_core::rom_data::conditional_property_masks::{RECORD_COUNT, SOURCE_OFFSET};
use recomp_core::analyze::{Analysis, Block, View};
use std::collections::BTreeSet;

#[test]
#[ignore = "private EUR ROM and TinyCC; Wine only, Linux/USA pending"]
fn european_conditional_property_masks_managed_native_reads() {
    let root = PathBuf::from(std::env::var_os("FFTA_PROPERTY_MASKS_RUNTIME_ROOT").unwrap());
    assert!(!root.exists()); fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior: Manifest = toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    assert_eq!(prior.resources.len(), 89);
    let mut source = Extraction::from_rom(&rom).unwrap();
    let dataset = root.join("offline dataset"); extract(&dataset, &mut source).unwrap();
    assert_eq!((source.manifest.extractor_revision, source.manifest.decoder_revision, source.manifest.resources.len()), (106, 105, 96));
    for resource in &prior.resources {
        assert_eq!(source.manifest.resources.iter().find(|r| r.id == resource.id), Some(resource));
        assert_eq!(fs::read(dataset.join(&resource.path)).unwrap(), fs::read(old.join(&resource.path)).unwrap());
    }
    let entry = &source.manifest.resources[89];
    assert_eq!(entry.id, CONDITIONAL_PROPERTY_MASKS_ID);
    assert_eq!((entry.sources[0].offset, entry.sources[0].length), (SOURCE_OFFSET, 140));
    let bytes = fs::read(dataset.join(CONDITIONAL_PROPERTY_MASKS_FILE)).unwrap();
    assert_eq!(parse_conditional_property_masks(&bytes).unwrap(), source.conditional_property_masks.as_ref().unwrap().decoded);
    let manifest = fs::read(dataset.join(MANIFEST_FILE)).unwrap();
    let mut machine = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut machine, &dataset).unwrap();
    let state = machine.save_state().unwrap(); let owner = machine.bus.rom_resources().unwrap().clone();
    let start = SOURCE_OFFSET as usize; let end = start + RECORD_COUNT * 4;
    machine.bus.rom[start..end].fill(0xcd);
    // The preceding descriptor and following item have independent typed owners.
    machine.bus.rom[start - 4..start].fill(0xa8);
    machine.bus.rom[end..end + 4].fill(0xa9);
    for at in start..end { assert_eq!(machine.bus.read8(0x0c000000 + at as u32), rom[at]); }
    for edge in [start, end] { for delta in [-3isize, -1, 0, 1] {
        let at = (edge as isize + delta) as usize;
        let ptr = &mut machine as *mut Machine as *mut std::ffi::c_void;
        assert_eq!((RT_API.read16)(ptr, 0x0a000000 + at as u32), u32::from(u16::from_le_bytes(rom[at..at + 2].try_into().unwrap())));
        assert_eq!((RT_API.read32)(ptr, 0x08000000 + at as u32), u32::from_le_bytes(rom[at..at + 4].try_into().unwrap()));
    }}
    machine.bus.rom.copy_from_slice(&rom); assert_eq!(machine.save_state().unwrap(), state);
    let mut bad = parse_conditional_property_masks(&bytes).unwrap(); bad.words[0] ^= 1;
    let bad_bytes = encode_conditional_property_masks(&bad).unwrap(); let mut bad_manifest = source.manifest.clone();
    bad_manifest.resources[89].sha256 = gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(CONDITIONAL_PROPERTY_MASKS_FILE), bad_bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE), toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();
    assert!(attach_from_directory(&mut machine, &dataset).is_err());
    assert!(std::sync::Arc::ptr_eq(machine.bus.rom_resources().unwrap(), &owner));
    assert_eq!(machine.save_state().unwrap(), state);
    fs::write(dataset.join(CONDITIONAL_PROPERTY_MASKS_FILE), bytes).unwrap(); fs::write(dataset.join(MANIFEST_FILE), manifest).unwrap();
    let mut visited = BTreeSet::new(); for at in (0x0ceea6..0x0ceeac).step_by(2) { visited.insert(0x08000000 + at); }
    let view = View { rom: &rom, ewram: None, iwram: None, bios: None };
    let blocks = visited.iter().map(|&addr| Block { addr, thumb:true, instrs:vec![armv4t::decode_thumb(view.read16(addr), addr)] }).collect();
    let object = root.join("property-masks-native.o");
    recomp_core::tcc_memory::build(&Analysis { blocks }, &view, &recomp_core::tcc_memory::library_path().unwrap(), &object, &mut |_| {}).unwrap();
    let (_library, table) = crate::load_native(&object).unwrap();
    let mut raw = crate::make_machine(rom.clone(), None); let mut native = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut native, &dataset).unwrap(); let initial = raw.save_state().unwrap();
    let native_owner = native.bus.rom_resources().unwrap().clone(); let mut served = 0;
    for index in [0u32, 1, 34, 35, 255, u32::MAX] {
        for m in [&mut raw, &mut native] {
            m.load_state(&initial).unwrap(); m.bus.ffta_sprites.enabled = false;
            m.cpu.regs = std::array::from_fn(|i| 0x11000000 + i as u32);
            m.cpu.regs[0] = index; m.cpu.regs[5] = 0x086e0b44;
            m.cpu.regs[13] = 0x03007c00; m.cpu.regs[15] = 0x080ceea6; m.cpu.cpsr = 0xbf;
        }
        native.bus.rom[start..end].fill(0xcd);
        while raw.cpu.regs[15] != 0x080ceeac { raw.bus.frame_ready = false; raw.step(); }
        let before = native_owner.conditional_property_masks_stats().unwrap().source_bytes; let mut steps = 0;
        while native.cpu.regs[15] != 0x080ceeac {
            assert!(steps < 8); native.bus.frame_ready = false;
            table.get(native.cpu.regs[15] | 1).unwrap()(&RT_API, &mut native as *mut Machine as *mut std::ffi::c_void); steps += 1;
        }
        let used = native_owner.conditional_property_masks_stats().unwrap().source_bytes - before;
        assert_eq!(used, if index < 35 { 4 } else { 0 }, "index {index}"); served += used;
        native.bus.rom.copy_from_slice(&rom); assert_eq!(native.save_state().unwrap(), raw.save_state().unwrap(), "index {index}");
    }
    fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS", "resources":90, "previous_unchanged":89, "source_bytes":140, "native_prefixes":1, "indices":[0,1,34,35,255,4294967295u32], "typed_native_bytes":served, "prior_owner_composition":true, "tamper_transaction":true, "full_state_clock":true, "Linux":"pending", "USA":"pending", "nativeWindows":"pending", "performance_measurements":false})).unwrap()).unwrap();
}
