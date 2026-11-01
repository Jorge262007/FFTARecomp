//! Focused EUR managed publication and typed read check; run on Wine.
use super::*;
use gba_core::{Bus, capi::RT_API};

#[test]
#[ignore = "private EUR ROM and accepted dataset; Wine only, Linux/USA pending"]
fn european_dynamic_fragments_usage_counts() {
    let root = PathBuf::from(std::env::var_os("FFTA_DYNAMIC_USAGE_ROOT").unwrap());
    assert!(!root.exists());
    fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let dataset = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let mut machine = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut machine, &dataset).unwrap();
    let owner = machine.bus.rom_resources().unwrap().clone();
    assert_eq!(owner.dynamic_raw_fragments_stats().unwrap().source_bytes, 0);
    let pointer = &mut machine as *mut _ as *mut std::ffi::c_void;
    machine.bus.rom[0x399700..0x39970c].fill(0xcd);
    machine.bus.rom[0x441388..0x441488].fill(0xcd);
    assert_eq!((RT_API.read16)(pointer, 0x08399700), u32::from(u16::from_le_bytes(rom[0x399700..0x399702].try_into().unwrap())));
    assert_eq!((RT_API.read32)(pointer, 0x08441388), u32::from_le_bytes(rom[0x441388..0x44138c].try_into().unwrap()));
    assert_eq!((RT_API.read16)(pointer, 0x083996ff), u32::from(u16::from_le_bytes(rom[0x3996ff..0x399701].try_into().unwrap())));
    let stats = owner.dynamic_raw_fragments_stats().unwrap();
    assert_eq!((stats.bus_reads, stats.source_bytes, stats.selector_bytes, stats.payload_bytes), (3, 7, 3, 4));
    machine.bus.read8(0x083996fe);
    assert_eq!(owner.dynamic_raw_fragments_stats().unwrap(), stats, "outside fragment does not increment counters");
    fs::write(root.join("result.json"), b"{\"status\":\"PASS\",\"bus_reads\":3,\"source_bytes\":7,\"selector_bytes\":3,\"payload_bytes\":4,\"mixed_edge\":true,\"Linux\":\"pending\",\"USA\":\"pending\",\"performance_measurements\":false}\n").unwrap();
}

#[test]
#[ignore = "private EUR ROM and managed publication; Wine only, Linux/USA pending"]
fn european_dynamic_fragments_managed_reads() {
    let root = PathBuf::from(std::env::var_os("FFTA_DYNAMIC_RAW_ROOT").unwrap());
    assert!(!root.exists());
    fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior: Manifest = toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    let mut source = Extraction::from_rom(&rom).unwrap();
    let dataset = root.join("offline dataset");
    extract(&dataset, &mut source).unwrap();
    assert_eq!((source.manifest.extractor_revision, source.manifest.decoder_revision, source.manifest.resources.len()), (106, 105, 96));
    for resource in &prior.resources {
        if resource.id != UI_CUSTOM_GRAPHICS_ID {
            assert_eq!(source.manifest.resources.iter().find(|entry| entry.id == resource.id), Some(resource));
            assert_eq!(fs::read(dataset.join(&resource.path)).unwrap(), fs::read(old.join(&resource.path)).unwrap());
        }
    }
    let entry = &source.manifest.resources[84];
    assert_eq!(entry.id, DYNAMIC_RAW_FRAGMENTS_ID);
    assert_eq!(entry.sources.len(), 7);
    assert_eq!(entry.sources.iter().map(|s| s.length).sum::<u32>(), 1304);
    let bytes = fs::read(dataset.join(DYNAMIC_RAW_FRAGMENTS_FILE)).unwrap();
    assert_eq!(parse_dynamic_raw_fragments(&bytes).unwrap(), source.dynamic_raw_fragments.as_ref().unwrap().decoded);
    let manifest = fs::read(dataset.join(MANIFEST_FILE)).unwrap();
    let mut machine = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut machine, &dataset).unwrap();
    let state = machine.save_state().unwrap();
    let owner = machine.bus.rom_resources().unwrap().clone();
    for range in &entry.sources {
        machine.bus.rom[range.offset as usize..(range.offset + range.length) as usize].fill(0xcd);
    }
    for range in &entry.sources {
        for at in range.offset..range.offset + range.length {
            assert_eq!(machine.bus.read8(0x0c000000 + at), rom[at as usize], "dynamic byte {at:08x}");
        }
        for edge in [range.offset, range.offset + range.length] {
            for delta in [-3i32, -1, 0, 1] {
                let at = (edge as i32 + delta) as usize;
                let pointer = &mut machine as *mut _ as *mut std::ffi::c_void;
                assert_eq!((RT_API.read16)(pointer, 0x0a000000 + at as u32), u32::from(u16::from_le_bytes(rom[at..at+2].try_into().unwrap())));
                assert_eq!((RT_API.read32)(pointer, 0x08000000 + at as u32), u32::from_le_bytes(rom[at..at+4].try_into().unwrap()));
            }
        }
    }
    // The adjacent fixed resource must still supply its bytes when both are poisoned.
    machine.bus.rom[0x441488..0x44148c].fill(0xab);
    assert_eq!(machine.bus.read8(0x08441488), rom[0x441488]);
    machine.bus.rom.copy_from_slice(&rom);
    assert_eq!(machine.save_state().unwrap(), state);
    // One original DMA transfer exercises the selected typed prefix and complete state/clock.
    let mut reference = crate::make_machine(rom.clone(), None);
    let mut decoded = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut decoded, &dataset).unwrap();
    for m in [&mut reference, &mut decoded] {
        m.bus.write32(0x040000d4, 0x08441388);
        m.bus.write32(0x040000d8, 0x02010000);
        m.bus.write16(0x040000dc, 32);
    }
    decoded.bus.rom[0x441388..0x4413c8].fill(0xcd);
    for m in [&mut reference, &mut decoded] { m.bus.write16(0x040000de, 0x8000); }
    decoded.bus.rom.copy_from_slice(&rom);
    assert_eq!(decoded.save_state().unwrap(), reference.save_state().unwrap(), "DMA full state and clock");
    let mut bad = parse_dynamic_raw_fragments(&bytes).unwrap();
    bad.banks[0].words[0] ^= 1;
    let bad_bytes = encode_dynamic_raw_fragments(&bad).unwrap();
    let mut bad_manifest = source.manifest.clone();
    bad_manifest.resources[84].sha256 = gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(DYNAMIC_RAW_FRAGMENTS_FILE), bad_bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE), toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();
    assert!(attach_from_directory(&mut machine, &dataset).is_err());
    assert!(std::sync::Arc::ptr_eq(machine.bus.rom_resources().unwrap(), &owner));
    assert_eq!(machine.save_state().unwrap(), state);
    fs::write(dataset.join(DYNAMIC_RAW_FRAGMENTS_FILE), bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE), manifest).unwrap();
    fs::write(root.join("result.json"), b"{\"status\":\"PASS\",\"source_bytes\":1304,\"spans\":7,\"Linux\":\"pending\",\"USA\":\"pending\",\"performance_measurements\":false}\n").unwrap();
}

#[test]
#[ignore = "private EUR original dynamic routine/native comparison; Wine only, Linux/USA pending"]
fn european_dynamic_original_routine_native_equivalence() {
    use gba_core::Machine;
    use recomp_core::analyze::{Analysis, Block, View};
    use std::collections::BTreeSet;

    const STOP: u32 = 0x03007f00;
    const TASK: u32 = 0x02010000;
    let root = PathBuf::from(std::env::var_os("FFTA_DYNAMIC_ORIGINAL_ROOT").unwrap());
    assert!(!root.exists());
    fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let dataset = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let manifest: Manifest = toml::from_str(&fs::read_to_string(dataset.join(MANIFEST_FILE)).unwrap()).unwrap();
    let spans = &manifest.resources[84].sources;
    assert_eq!(spans.len(), 7);
    let cases: Vec<(u8, bool, u8)> = (7..=9).flat_map(|selector| [false, true].into_iter().flat_map(move |flag| (0..5).map(move |language| (selector, flag, language)))).collect();

    fn registers(m: &mut Machine, pc: u32) {
        m.cpu.regs = std::array::from_fn(|i| 0x11000000 + i as u32);
        m.cpu.regs[13] = 0x03007c00;
        m.cpu.regs[14] = STOP | 1;
        m.cpu.regs[15] = pc;
        m.cpu.cpsr = 0xbf;
    }
    fn setup(m: &mut Machine, selector: u8, flag: bool, language: u8) {
        m.bus.ffta_sprites.enabled = false;
        m.bus.ewram.fill(0);
        m.bus.iwram.fill(0);
        m.bus.vram.fill(0xa5);
        m.bus.write8(TASK + 2, selector);
        m.bus.write16(TASK + 0x48, 1);
        m.bus.write8(0x030028a0, if flag { 8 } else { 0 });
        m.bus.write8(0x02001f6e, language);
        registers(m, 0x080c9060);
        m.cpu.regs[0] = TASK;
    }
    fn expected_sources(rom: &[u8], selector: u8, flag: bool, language: u8) -> [u32; 2] {
        let table = if flag { 0x3996f4 } else { 0x39970c };
        let at = table + (usize::from(selector) - 4) * 4;
        let root = 0x71e6f8 + usize::from(language) * 4;
        let bank = u32::from_le_bytes(rom[root..root + 4].try_into().unwrap());
        std::array::from_fn(|i| bank + ((u16::from_le_bytes(rom[at + i * 2..at + i * 2 + 2].try_into().unwrap()) as u32) << 5))
    }
    fn check_queue(m: &mut Machine, sources: [u32; 2]) {
        assert_eq!(m.bus.read32(0x03002bcc), TASK + 0x10);
        for (i, src) in sources.into_iter().enumerate() {
            let node = TASK + 0x10 + 0x1c * i as u32;
            assert_eq!(m.bus.read32(node + 4), src);
            assert_eq!(m.bus.read32(node + 8), 0x06013800 + i as u32 * 0x40);
            assert_eq!(m.bus.read32(node + 12), 0x80000020);
        }
        assert_eq!(m.bus.read16(TASK + 0x48), 2);
    }
    fn check_transfer(m: &mut Machine, rom: &[u8], sources: [u32; 2]) {
        assert_eq!(m.bus.read32(0x03002bcc), 0);
        for (i, src) in sources.into_iter().enumerate() {
            let at = (src - 0x08000000) as usize;
            assert!(rom[at..at + 64].iter().any(|&b| b != 0xcd));
            assert_eq!(&m.bus.vram[0x13800 + 64 * i..0x13840 + 64 * i], &rom[at..at + 64]);
        }
    }
    fn interpreter(m: &mut Machine, visited: &mut BTreeSet<(u32, bool)>) {
        let mut steps = 0;
        while m.cpu.regs[15] != STOP {
            assert!(steps < 100_000, "interpreter pc {:08x}", m.cpu.regs[15]);
            assert!(m.cpu.thumb());
            assert_eq!(m.cpu.regs[15] >> 24, 8);
            visited.insert((m.cpu.regs[15], true));
            m.bus.frame_ready = false;
            m.step();
            steps += 1;
        }
    }
    let mut reference = crate::make_machine(rom.clone(), None);
    let initial = reference.save_state().unwrap();
    let mut visited = BTreeSet::new();
    let mut expected = Vec::new();
    for &(selector, flag, language) in &cases {
        reference.load_state(&initial).unwrap();
        setup(&mut reference, selector, flag, language);
        interpreter(&mut reference, &mut visited);
        let sources = expected_sources(&rom, selector, flag, language);
        check_queue(&mut reference, sources);
        registers(&mut reference, 0x08000800);
        interpreter(&mut reference, &mut visited);
        check_transfer(&mut reference, &rom, sources);
        expected.push(reference.save_state().unwrap());
    }
    assert!(!visited.iter().any(|&(pc, _)| pc == STOP));
    let view = View { rom: &rom, ewram: None, iwram: None, bios: None };
    let blocks = visited.iter().map(|&(addr, thumb)| {
        let mut instrs = vec![armv4t::decode_thumb(view.read16(addr), addr)];
        let next = armv4t::decode_thumb(view.read16(addr + 2), addr + 2);
        if armv4t::fuse_thumb_bl(&instrs[0], &next).is_some() { instrs.push(next); }
        Block { addr, thumb, instrs }
    }).collect();
    let object = root.join("dynamic-original.o");
    recomp_core::tcc_memory::build(&Analysis { blocks }, &view, &recomp_core::tcc_memory::library_path().unwrap(), &object, &mut |_| {}).unwrap();
    let (_library, table) = crate::load_native(&object).unwrap();
    let mut typed = crate::make_machine(rom.clone(), None);
    let mut native = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut typed, &dataset).unwrap();
    attach_from_directory(&mut native, &dataset).unwrap();
    for (i, &(selector, flag, language)) in cases.iter().enumerate() {
        let sources = expected_sources(&rom, selector, flag, language);
        for (label, m) in [("typed", &mut typed), ("native", &mut native)] {
            m.load_state(&initial).unwrap();
            setup(m, selector, flag, language);
            for span in spans { m.bus.rom[span.offset as usize..(span.offset + span.length) as usize].fill(0xcd); }
            for phase in 0..2 {
                if phase == 1 { check_queue(m, sources); registers(m, 0x08000800); }
                if label == "typed" {
                    interpreter(m, &mut BTreeSet::new());
                } else {
                    let mut dispatches = 0;
                    while m.cpu.regs[15] != STOP {
                        assert!(dispatches < 100_000, "native case {i} pc {:08x}", m.cpu.regs[15]);
                        m.bus.frame_ready = false;
                        table.get(m.cpu.regs[15] | u32::from(m.cpu.thumb())).unwrap_or_else(|| panic!("uncompiled native pc {:08x}", m.cpu.regs[15]))(&RT_API, m as *mut Machine as *mut std::ffi::c_void);
                        dispatches += 1;
                    }
                }
            }
            check_transfer(m, &rom, sources);
            m.bus.rom.copy_from_slice(&rom);
            assert_eq!(m.save_state().unwrap(), expected[i], "{label} full state/clock case {i}: selector {selector}, flag {flag}, language {language}");
        }
    }
    fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","selectors":[7,8,9],"flag_branches":2,"languages":5,"cases":cases.len(),"original_queue_and_dma":true,"full_state_clock_arms":["interpreter_raw","interpreter_typed","native_typed"],"source_poisoned":1304,"ordinary_gameplay":"pending","Linux":"pending","USA":"pending","nativeWindows":"pending","performance_measurements":false})).unwrap()).unwrap();
}
