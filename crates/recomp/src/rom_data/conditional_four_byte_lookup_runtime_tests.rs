//! EUR managed publication, typed reads and original native prefixes.
use super::*;
use gba_core::{Bus, Machine, capi::RT_API};
use gba_core::rom_data::conditional_four_byte_lookup::{Lookup, SOURCE_OFFSET};
use recomp_core::analyze::{Analysis, Block, View};
use std::collections::BTreeSet;

#[test]
#[ignore = "private EUR ROM and TinyCC; Wine only, Linux/USA pending"]
fn european_conditional_lookup_managed_native_reads() {
    let root = PathBuf::from(std::env::var_os("FFTA_CONDITIONAL_RUNTIME_ROOT").unwrap());
    assert!(!root.exists());
    fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior: Manifest = toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    let mut source = Extraction::from_rom(&rom).unwrap();
    let dataset = root.join("offline dataset");
    extract(&dataset, &mut source).unwrap();
    assert_eq!((source.manifest.extractor_revision, source.manifest.decoder_revision, source.manifest.resources.len()), (106,105,96));
    assert_eq!(prior.resources.len(), 85);
    for resource in &prior.resources {
        assert_eq!(source.manifest.resources.iter().find(|entry| entry.id == resource.id), Some(resource));
        assert_eq!(fs::read(dataset.join(&resource.path)).unwrap(), fs::read(old.join(&resource.path)).unwrap());
    }
    let entry = &source.manifest.resources[85];
    assert_eq!(entry.id, CONDITIONAL_FOUR_BYTE_LOOKUP_ID);
    assert_eq!((entry.sources.len(),entry.sources[0].offset,entry.sources[0].length),(1,SOURCE_OFFSET,296));
    let bytes = fs::read(dataset.join(CONDITIONAL_FOUR_BYTE_LOOKUP_FILE)).unwrap();
    assert_eq!(parse_conditional_four_byte_lookup(&bytes).unwrap(), source.conditional_four_byte_lookup.as_ref().unwrap().decoded);
    let manifest = fs::read(dataset.join(MANIFEST_FILE)).unwrap();
    let mut machine = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut machine, &dataset).unwrap();
    let state = machine.save_state().unwrap();
    let owner = machine.bus.rom_resources().unwrap().clone();
    let start = SOURCE_OFFSET as usize;
    machine.bus.rom[start..start+296].fill(0xcd);
    for at in start..start+296 {
        assert_eq!(machine.bus.read8(0x0c000000 + at as u32),rom[at],"byte {at:06x}");
    }
    for edge in [start,start+296] {
        for delta in [-3isize,-1,0,1] {
            let at = (edge as isize + delta) as usize;
            let ptr = &mut machine as *mut Machine as *mut std::ffi::c_void;
            assert_eq!((RT_API.read16)(ptr,0x0a000000 + at as u32),u32::from(u16::from_le_bytes(rom[at..at+2].try_into().unwrap())));
            assert_eq!((RT_API.read32)(ptr,0x08000000 + at as u32),u32::from_le_bytes(rom[at..at+4].try_into().unwrap()));
        }
    }
    machine.bus.rom.copy_from_slice(&rom);
    assert_eq!(machine.save_state().unwrap(),state);
    let mut bad = parse_conditional_four_byte_lookup(&bytes).unwrap();
    bad.records[0].first ^= 1;
    let bad_bytes = encode_conditional_four_byte_lookup(&bad).unwrap();
    let mut bad_manifest = source.manifest.clone();
    bad_manifest.resources[85].sha256 = gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(CONDITIONAL_FOUR_BYTE_LOOKUP_FILE),bad_bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE),toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();
    assert!(attach_from_directory(&mut machine,&dataset).is_err());
    assert!(std::sync::Arc::ptr_eq(machine.bus.rom_resources().unwrap(),&owner));
    assert_eq!(machine.save_state().unwrap(),state);
    fs::write(dataset.join(CONDITIONAL_FOUR_BYTE_LOOKUP_FILE),bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE),manifest).unwrap();

    let prefixes = [(0x80302,0x8030e,0),(0x80316,0x8031e,1),(0x7710e,0x77110,2)];
    let mut visited = BTreeSet::new();
    for (start,end,_) in prefixes { for at in (start..end).step_by(2) { visited.insert(0x08000000+at); } }
    let view = View { rom:&rom,ewram:None,iwram:None,bios:None };
    let blocks = visited.iter().map(|&addr| Block {addr,thumb:true,instrs:vec![armv4t::decode_thumb(view.read16(addr),addr)]}).collect();
    let object = root.join("conditional-native.o");
    recomp_core::tcc_memory::build(&Analysis {blocks},&view,&recomp_core::tcc_memory::library_path().unwrap(),&object,&mut |_| {}).unwrap();
    let (_library,table) = crate::load_native(&object).unwrap();
    let mut raw = crate::make_machine(rom.clone(),None);
    let mut native = crate::make_machine(rom.clone(),None);
    attach_from_directory(&mut native,&dataset).unwrap();
    let initial = raw.save_state().unwrap();
    let native_owner = native.bus.rom_resources().unwrap().clone();
    let mut served = 0;
    for (start_pc,end_pc,field) in prefixes {
        for index in [0u32,1,73,74,255,u32::MAX] {
            let addr = Lookup::encoded_view(index);
            for m in [&mut raw,&mut native] {
                m.load_state(&initial).unwrap();
                m.bus.ffta_sprites.enabled = false;
                m.cpu.regs = std::array::from_fn(|i| 0x11000000+i as u32);
                m.cpu.regs[13] = 0x03007c00;
                m.cpu.cpsr = 0xbf;
                match field { 0 => {m.cpu.regs[0]=0x086e6dbc;m.cpu.regs[5]=index;}, 1 => m.cpu.regs[4]=addr, _ => m.cpu.regs[0]=addr }
                m.cpu.regs[15]=0x08000000+start_pc;
            }
            native.bus.rom[start..start+296].fill(0xcd);
            while raw.cpu.regs[15] != 0x08000000+end_pc {raw.bus.frame_ready=false;raw.step();}
            let before = native_owner.conditional_four_byte_lookup_stats().unwrap().source_bytes;
            let mut dispatches = 0;
            while native.cpu.regs[15] != 0x08000000+end_pc {
                assert!(dispatches<8);
                native.bus.frame_ready=false;
                table.get(native.cpu.regs[15]|1).unwrap()(&RT_API,&mut native as *mut Machine as *mut std::ffi::c_void);
                dispatches+=1;
            }
            let used = native_owner.conditional_four_byte_lookup_stats().unwrap().source_bytes-before;
            assert_eq!(used, if index < 74 {if field == 2 {1} else {2}} else {0},"index {index} field {field}");
            served+=used;
            native.bus.rom.copy_from_slice(&rom);
            assert_eq!(native.save_state().unwrap(),raw.save_state().unwrap(),"index {index} field {field}");
        }
    }
    fs::write(root.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","resources":90,"previous_unchanged":85,"source_bytes":296,"native_prefixes":3,"indices":[0,1,73,74,255,4294967295u32],"typed_native_bytes":served,"full_state_clock":true,"tamper_transaction":true,"Linux":"pending","USA":"pending","nativeWindows":"pending","performance_measurements":false})).unwrap()).unwrap();
}
