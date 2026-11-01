//! Managed EUR packed lookup, boundaries and original native reader prefixes.
use super::*;
use gba_core::{Bus, Machine, capi::RT_API};
use gba_core::rom_data::conditional_packed_lookup::SOURCE_OFFSET;
use recomp_core::analyze::{Analysis, Block, View};
use std::collections::BTreeSet;

#[test]
#[ignore = "private EUR ROM and TinyCC; Wine only, Linux/USA pending"]
fn european_conditional_packed_managed_native_reads() {
    let root=PathBuf::from(std::env::var_os("FFTA_PACKED_RUNTIME_ROOT").unwrap());
    assert!(!root.exists());fs::create_dir_all(&root).unwrap();
    let rom=fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let old=PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior:Manifest=toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    let mut source=Extraction::from_rom(&rom).unwrap();let dataset=root.join("offline dataset");extract(&dataset,&mut source).unwrap();
    assert_eq!((source.manifest.extractor_revision,source.manifest.decoder_revision,source.manifest.resources.len()),(106,105,96));
    assert_eq!(prior.resources.len(),86);
    for r in &prior.resources {assert_eq!(source.manifest.resources.iter().find(|e|e.id==r.id),Some(r));assert_eq!(fs::read(dataset.join(&r.path)).unwrap(),fs::read(old.join(&r.path)).unwrap());}
    let entry=&source.manifest.resources[86];assert_eq!(entry.id,CONDITIONAL_PACKED_LOOKUP_ID);
    assert_eq!((entry.sources.len(),entry.sources[0].offset,entry.sources[0].length),(1,SOURCE_OFFSET,132));
    let bytes=fs::read(dataset.join(CONDITIONAL_PACKED_LOOKUP_FILE)).unwrap();
    assert_eq!(parse_conditional_packed_lookup(&bytes).unwrap(),source.conditional_packed_lookup.as_ref().unwrap().decoded);
    let manifest=fs::read(dataset.join(MANIFEST_FILE)).unwrap();let start=SOURCE_OFFSET as usize;let alias=0x6e72d0usize;
    let mut machine=crate::make_machine(rom.clone(),None);attach_from_directory(&mut machine,&dataset).unwrap();
    let state=machine.save_state().unwrap();let owner=machine.bus.rom_resources().unwrap().clone();
    machine.bus.rom[start..start+132].fill(0xcd);
    machine.bus.rom[start+132..start+136].fill(0xa7);
    machine.bus.rom[alias..alias+4].fill(0xa9);
    for at in start..start+132 {assert_eq!(machine.bus.read8(0x0c000000+at as u32),rom[at],"byte {at:06x}");}
    for edge in [start,start+132] {for delta in [-3isize,-1,0,1] {
        let at=(edge as isize+delta) as usize;let ptr=&mut machine as *mut Machine as *mut std::ffi::c_void;
        assert_eq!((RT_API.read16)(ptr,0x0a000000+at as u32),u32::from(u16::from_le_bytes(rom[at..at+2].try_into().unwrap())));
        assert_eq!((RT_API.read32)(ptr,0x08000000+at as u32),u32::from_le_bytes(rom[at..at+4].try_into().unwrap()));
    }}
    for at in alias..alias+4 {assert_eq!(machine.bus.read8(0x08000000+at as u32),rom[at],"prior metadata14 alias");}
    machine.bus.rom.copy_from_slice(&rom);assert_eq!(machine.save_state().unwrap(),state);
    let mut bad=parse_conditional_packed_lookup(&bytes).unwrap();bad.records[0].byte0^=1;
    let bad_bytes=encode_conditional_packed_lookup(&bad).unwrap();let mut bad_manifest=source.manifest.clone();bad_manifest.resources[86].sha256=gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(CONDITIONAL_PACKED_LOOKUP_FILE),bad_bytes).unwrap();fs::write(dataset.join(MANIFEST_FILE),toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();
    assert!(attach_from_directory(&mut machine,&dataset).is_err());assert!(std::sync::Arc::ptr_eq(machine.bus.rom_resources().unwrap(),&owner));assert_eq!(machine.save_state().unwrap(),state);
    fs::write(dataset.join(CONDITIONAL_PACKED_LOOKUP_FILE),bytes).unwrap();fs::write(dataset.join(MANIFEST_FILE),manifest).unwrap();
    let prefixes=[(0x132046u32,0x132068u32),(0x134176,0x13418c)];let mut visited=BTreeSet::new();
    for (start,end) in prefixes {for at in (start..end).step_by(2) {visited.insert(0x08000000+at);}}
    let view=View {rom:&rom,ewram:None,iwram:None,bios:None};
    let blocks=visited.iter().map(|&addr|Block {addr,thumb:true,instrs:vec![armv4t::decode_thumb(view.read16(addr),addr)]}).collect();
    let object=root.join("packed-native.o");recomp_core::tcc_memory::build(&Analysis {blocks},&view,&recomp_core::tcc_memory::library_path().unwrap(),&object,&mut |_| {}).unwrap();
    let (_library,table)=crate::load_native(&object).unwrap();let mut raw=crate::make_machine(rom.clone(),None);let mut native=crate::make_machine(rom.clone(),None);attach_from_directory(&mut native,&dataset).unwrap();
    let initial=raw.save_state().unwrap();let native_owner=native.bus.rom_resources().unwrap().clone();let mut served=0;let cases=[(0u8,0u8,0u16,0u8),(1,11,1,5),(5,65,33,5),(0,1,251,1)];
    for (prefix,(begin,end)) in prefixes.into_iter().enumerate() {for (bank,row,index,kind) in cases {
        let ptr=0x6df4d4+usize::from(bank)*4;let descriptor=u32::from_le_bytes(rom[ptr..ptr+4].try_into().unwrap())+u32::from(row)*8;
        for m in [&mut raw,&mut native] {m.load_state(&initial).unwrap();m.bus.ffta_sprites.enabled=false;m.cpu.regs=std::array::from_fn(|i|0x11000000+i as u32);m.cpu.regs[13]=0x03007c00;m.cpu.regs[0]=descriptor;m.cpu.regs[15]=0x08000000+begin;m.cpu.cpsr=0xbf;}
        native.bus.rom[start..start+132].fill(0xcd);native.bus.rom[alias..alias+4].fill(0xa9);
        let stop=if prefix==1 && kind!=5 {0x081341cc} else {0x08000000+end};
        while raw.cpu.regs[15]!=stop {raw.bus.frame_ready=false;raw.step();}
        let before=native_owner.conditional_packed_lookup_stats().unwrap().source_bytes;
        let prior_before=native_owner.metadata14_stats().unwrap().source_bytes;
        let mut dispatches=0;while native.cpu.regs[15]!=stop {assert!(dispatches<24,"pc {:08x}",native.cpu.regs[15]);native.bus.frame_ready=false;table.get(native.cpu.regs[15]|1).unwrap()(&RT_API,&mut native as *mut Machine as *mut std::ffi::c_void);dispatches+=1;}
        let used=native_owner.conditional_packed_lookup_stats().unwrap().source_bytes-before;
        let prior_used=native_owner.metadata14_stats().unwrap().source_bytes-prior_before;
        assert_eq!(used,if kind==5 {if prefix==0 {2} else {1}} else {0},"index {index} prefix {prefix}");
        if index==251 && prefix==0 {assert_eq!(prior_used,2);}
        served+=used;native.bus.rom.copy_from_slice(&rom);
        assert_eq!(native.save_state().unwrap(),raw.save_state().unwrap(),"index {index} prefix {prefix}");
    }}
    fs::write(root.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","resources":90,"previous_unchanged":86,"source_bytes":132,"native_prefixes":2,"descriptor_indices":[0,1,33,251],"typed_native_bytes":served,"metadata14_alias":true,"full_state_clock":true,"tamper_transaction":true,"Linux":"pending","USA":"pending","nativeWindows":"pending","performance_measurements":false})).unwrap()).unwrap();
}
