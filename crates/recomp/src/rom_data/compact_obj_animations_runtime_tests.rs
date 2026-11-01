//! Managed compact OBJ projection, gaps, transactional load and native reader.
use super::*;
use gba_core::{Bus, Machine, capi::RT_API};
use gba_core::rom_data::compact_obj_animations::{SPANS, SEQUENCE_ROOT, SELECTORS};
use recomp_core::analyze::{Analysis, Block, View};
use std::collections::BTreeSet;

#[test]
#[ignore = "private EUR ROM and TinyCC; Wine only, Linux/USA pending"]
fn european_compact_obj_managed_native_reads() {
    let root = PathBuf::from(std::env::var_os("FFTA_COMPACT_OBJ_RUNTIME_ROOT").unwrap());
    assert!(!root.exists()); fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior: Manifest = toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    assert_eq!(prior.resources.len(), 90);
    let mut source = Extraction::from_rom(&rom).unwrap();
    let dataset = root.join("offline dataset"); extract(&dataset, &mut source).unwrap();
    assert_eq!((source.manifest.extractor_revision, source.manifest.decoder_revision, source.manifest.resources.len()), (106,105,96));
    for resource in &prior.resources {
        assert_eq!(source.manifest.resources.iter().find(|r| r.id == resource.id), Some(resource));
        assert_eq!(fs::read(dataset.join(&resource.path)).unwrap(), fs::read(old.join(&resource.path)).unwrap());
    }
    let entry = &source.manifest.resources[90]; assert_eq!(entry.id, COMPACT_OBJ_ANIMATIONS_ID);
    assert_eq!(entry.sources.iter().map(|s|(s.offset,s.length)).collect::<Vec<_>>(), SPANS.iter().map(|(a,b)|(*a,b-a)).collect::<Vec<_>>());
    let bytes = fs::read(dataset.join(COMPACT_OBJ_ANIMATIONS_FILE)).unwrap();
    assert_eq!(parse_compact_obj_animations(&bytes).unwrap(), source.compact_obj_animations.as_ref().unwrap().decoded);
    let manifest = fs::read(dataset.join(MANIFEST_FILE)).unwrap();
    let mut machine = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut machine, &dataset).unwrap();
    let state = machine.save_state().unwrap(); let owner = machine.bus.rom_resources().unwrap().clone();
    for (start,end) in SPANS { machine.bus.rom[start as usize..end as usize].fill(0xcd); }
    for (start,end) in SPANS {
        for at in start..end { assert_eq!(machine.bus.read8(0x0c000000+at), rom[at as usize]); }
        for edge in [start,end] { for delta in [-3i32,-1,0,1] {
            let at = edge.wrapping_add_signed(delta) as usize;
            let ptr = &mut machine as *mut Machine as *mut std::ffi::c_void;
            assert_eq!((RT_API.read16)(ptr, 0x0a000000+at as u32), u32::from(u16::from_le_bytes(rom[at..at+2].try_into().unwrap())));
            assert_eq!((RT_API.read32)(ptr, 0x08000000+at as u32), u32::from_le_bytes(rom[at..at+4].try_into().unwrap()));
        }}
    }
    machine.bus.rom.copy_from_slice(&rom); assert_eq!(machine.save_state().unwrap(),state);
    let mut bad = parse_compact_obj_animations(&bytes).unwrap(); bad.sequences[0].frames[0].duration ^= 1;
    let bad_bytes = encode_compact_obj_animations(&bad).unwrap(); let mut bad_manifest = source.manifest.clone();
    bad_manifest.resources[90].sha256 = gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(COMPACT_OBJ_ANIMATIONS_FILE),bad_bytes).unwrap();
    fs::write(dataset.join(MANIFEST_FILE),toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();
    assert!(attach_from_directory(&mut machine,&dataset).is_err());
    assert!(std::sync::Arc::ptr_eq(machine.bus.rom_resources().unwrap(),&owner)); assert_eq!(machine.save_state().unwrap(),state);
    fs::write(dataset.join(COMPACT_OBJ_ANIMATIONS_FILE),bytes).unwrap(); fs::write(dataset.join(MANIFEST_FILE),manifest).unwrap();
    let visited: BTreeSet<_> = (0x010870..0x010884).step_by(2).map(|at|0x08000000+at).collect();
    let view = View {rom:&rom,ewram:None,iwram:None,bios:None};
    let blocks = visited.iter().map(|&addr|Block {addr,thumb:true,instrs:vec![armv4t::decode_thumb(view.read16(addr),addr)]}).collect();
    let object = root.join("compact-obj-native.o");
    recomp_core::tcc_memory::build(&Analysis {blocks},&view,&recomp_core::tcc_memory::library_path().unwrap(),&object,&mut |_|{}).unwrap();
    let (_library,table)=crate::load_native(&object).unwrap();
    let mut raw=crate::make_machine(rom.clone(),None);let mut native=crate::make_machine(rom.clone(),None);
    attach_from_directory(&mut native,&dataset).unwrap();let initial=raw.save_state().unwrap();
    let native_owner=native.bus.rom_resources().unwrap().clone();let mut served=0;
    for index in SELECTORS.into_iter().chain([8]) {
        for m in [&mut raw,&mut native] {
            m.load_state(&initial).unwrap();m.bus.ffta_sprites.enabled=false;
            m.bus.write32(0x02007f08,0x08000000+SEQUENCE_ROOT);
            m.cpu.regs=std::array::from_fn(|i|0x11000000+i as u32);
            m.cpu.regs[3]=u32::from(index);m.cpu.regs[4]=0x02007f00;
            m.cpu.regs[13]=0x03007c00;m.cpu.regs[15]=0x08010870;m.cpu.cpsr=0xbf;
        }
        for (start,end) in SPANS {native.bus.rom[start as usize..end as usize].fill(0xcd);}
        while raw.cpu.regs[15]!=0x08010884 {raw.bus.frame_ready=false;raw.step();}
        let before=native_owner.compact_obj_animations_stats().unwrap().source_bytes;let mut steps=0;
        while native.cpu.regs[15]!=0x08010884 {assert!(steps<16);native.bus.frame_ready=false;table.get(native.cpu.regs[15]|1).unwrap()(&RT_API,&mut native as *mut Machine as *mut std::ffi::c_void);steps+=1;}
        let used=native_owner.compact_obj_animations_stats().unwrap().source_bytes-before;
        assert_eq!(used,if index==8 {0} else {4});served+=used;
        native.bus.rom.copy_from_slice(&rom);assert_eq!(native.save_state().unwrap(),raw.save_state().unwrap(),"selector {index}");
    }
    fs::write(root.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","resources":91,"previous_unchanged":90,"spans":5,"source_bytes":2028,"native_prefixes":1,"selectors":[0,1,2,3,4,5,6,7,8,9,10],"typed_native_bytes":served,"gap_fallback":true,"tamper_transaction":true,"full_state_clock":true,"Linux":"pending","USA":"pending","nativeWindows":"pending","performance_measurements":false})).unwrap()).unwrap();
}
