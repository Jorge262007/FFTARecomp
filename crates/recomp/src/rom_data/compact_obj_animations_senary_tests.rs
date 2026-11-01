//! Source-authenticated compact OBJ projection and original EUR reader prefixes.
use super::*;
use gba_core::bus::Bus;
use gba_core::rom_data::compact_obj_animations_senary::{Catalog, CompactAnimationsSenary, ROOT, LIST_ROOT, SEQUENCE_ROOT, SELECTORS, LIST_INDICES, SPANS};

#[test]
#[ignore = "private EUR source and original reader check; Wine only, Linux/USA pending"]
fn european_senary_compact_obj_projection_matches_original_readers() {
    let out = PathBuf::from(std::env::var_os("FFTA_SENARY_COMPACT_OBJ_ROOT").unwrap());
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let catalog = Catalog::from_rom(&rom).unwrap();
    let model = &catalog.decoded;
    model.verify_source(&rom).unwrap();
    assert_eq!(catalog.sources.len(), 2);
    for ((start, end), (at, bytes, _)) in SPANS.into_iter().zip(model.source_spans().unwrap()) {
        assert_eq!(at, start);
        assert_eq!(bytes, rom[start as usize..end as usize]);
        for p in start..end { assert_eq!(model.byte_at(p), Some(rom[p as usize])); }
    }
    for p in [ROOT-1, 0x6c6b7e, 0x6c6b7f, 0x6c6c1e] { assert_eq!(model.byte_at(p),None); }
    assert_eq!(model.sequences.iter().map(|s|s.sequence.frames.len()).sum::<usize>(),65);
    assert_eq!(model.lists.iter().map(|s|s.list.objects.len()).sum::<usize>(),59);
    assert!(model.sequence(7).is_none()); assert!(model.list(60).is_none());
    let json=serde_json::to_vec(model).unwrap();assert_eq!(serde_json::from_slice::<CompactAnimationsSenary>(&json).unwrap(),*model);
    let mut invalid=model.clone();invalid.lists.pop();assert!(invalid.validate().is_err());
    invalid=model.clone();invalid.sequences[0].relative_offset+=2;assert!(invalid.validate().is_err());
    invalid=model.clone();invalid.sequences[0].sequence.frames[0].list=60;assert!(invalid.validate().is_err());
    invalid=model.clone();invalid.sequences[0].sequence.frames[0].duration^=1;assert!(invalid.validate().is_ok());assert!(invalid.verify_source(&rom).is_err());
    assert!(serde_json::from_str::<CompactAnimationsSenary>("{\"lists\":[],\"sequences\":[],\"extra\":true}").is_err());
    let mut machine = crate::make_machine(rom, None);
    machine.bus.ffta_sprites.enabled = false;
    let state = 0x02007f00;
    machine.bus.write32(state + 4, 0x08000000 + LIST_ROOT);
    machine.bus.write32(state + 8, 0x08000000 + SEQUENCE_ROOT);
    for selector in SELECTORS {
        let sequence = model.sequence(selector).unwrap();
        machine.cpu.regs = [0; 16]; machine.cpu.regs[3] = u32::from(selector);
        machine.cpu.regs[4] = state; machine.cpu.regs[15] = 0x08010870; machine.cpu.cpsr = 0xbf;
        let mut n = 0;
        while machine.cpu.regs[15] != 0x08010884 { assert!(n < 16); machine.bus.frame_ready=false; machine.step(); n += 1; }
        assert_eq!(machine.cpu.regs[3] as usize, sequence.frames.len());
        for (phase, frame) in sequence.frames.iter().enumerate() {
            machine.cpu.regs = [0; 16]; machine.cpu.regs[5] = u32::from(selector);
            machine.cpu.regs[3] = state; machine.cpu.regs[2] = phase as u32;
            machine.cpu.regs[15] = 0x08010e5a; machine.cpu.cpsr = 0xbf; n = 0;
            while machine.cpu.regs[15] != 0x08010e70 { assert!(n < 16); machine.bus.frame_ready=false; machine.step(); n += 1; }
            assert_eq!(machine.cpu.regs[0], u32::from(frame.list));
            machine.cpu.regs = [0; 16]; machine.cpu.regs[7] = u32::from(selector);
            machine.cpu.regs[4] = state; machine.cpu.regs[3] = phase as u32;
            machine.cpu.regs[15] = 0x08010cb2; machine.cpu.cpsr = 0xbf; n = 0;
            while machine.cpu.regs[15] != 0x08010cc8 { assert!(n < 16); machine.bus.frame_ready=false; machine.step(); n += 1; }
            assert_eq!(machine.cpu.regs[1], u32::from(frame.duration));
        }
    }
    for index in LIST_INDICES {
        machine.cpu.regs = [0; 16]; machine.cpu.regs[2] = u32::from(index);
        machine.cpu.regs[3] = state; machine.cpu.regs[15] = 0x08010e9a; machine.cpu.cpsr = 0xbf;
        let mut n = 0;
        while machine.cpu.regs[15] != 0x08010eb2 { assert!(n < 16); machine.bus.frame_ready=false; machine.step(); n += 1; }
        assert_eq!(machine.cpu.regs[1], 0x08000000 + model.list(index).unwrap().source_offset);
    }
    fs::write(out.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "status":"PASS", "spans":2, "source_bytes":756, "selectors":SELECTORS,
        "frames":65, "objects":59, "Linux":"pending", "USA":"pending", "nativeWindows":"pending", "performance_measurements":false
    })).unwrap()).unwrap();
}
