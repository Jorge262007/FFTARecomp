//! Managed EUR bank-zero composition with the preceding descriptor owner.
use super::*;
use gba_core::{Bus,Machine,capi::RT_API};
use gba_core::rom_data::conditional_descriptor_bank_zero::{BANK_ROOT,SOURCE_SPANS};
use recomp_core::analyze::{Analysis,Block,View};
use std::collections::BTreeSet;

#[test]
#[ignore = "private EUR ROM and TinyCC; Wine only, Linux/USA pending"]
fn european_conditional_descriptor_bank_zero_managed_native_reads() {
    let root=PathBuf::from(std::env::var_os("FFTA_DESCRIPTOR_ZERO_RUNTIME_ROOT").unwrap());assert!(!root.exists());fs::create_dir_all(&root).unwrap();
    let rom=fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();let old=PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());let prior:Manifest=toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    let mut source=Extraction::from_rom(&rom).unwrap();let dataset=root.join("offline dataset");extract(&dataset,&mut source).unwrap();assert_eq!((source.manifest.extractor_revision,source.manifest.decoder_revision,source.manifest.resources.len()),(106,105,96));assert_eq!(prior.resources.len(),88);
    for r in &prior.resources {assert_eq!(source.manifest.resources.iter().find(|e|e.id==r.id),Some(r));assert_eq!(fs::read(dataset.join(&r.path)).unwrap(),fs::read(old.join(&r.path)).unwrap());}
    let entry=&source.manifest.resources[88];assert_eq!(entry.id,CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID);assert_eq!(entry.sources.len(),2);for (s,(offset,length,_,_)) in entry.sources.iter().zip(SOURCE_SPANS) {assert_eq!((s.offset,s.length),(offset,length));}
    let bytes=fs::read(dataset.join(CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE)).unwrap();assert_eq!(parse_conditional_descriptor_bank_zero(&bytes).unwrap(),source.conditional_descriptor_bank_zero.as_ref().unwrap().decoded);let manifest=fs::read(dataset.join(MANIFEST_FILE)).unwrap();
    let mut machine=crate::make_machine(rom.clone(),None);attach_from_directory(&mut machine,&dataset).unwrap();let state=machine.save_state().unwrap();let owner=machine.bus.rom_resources().unwrap().clone();
    for (offset,length,_,_) in SOURCE_SPANS {machine.bus.rom[offset as usize..(offset+length) as usize].fill(0xcd);}
    // Adjacent e98 pointer1 and bank1 row0 must compose with the new spans.
    machine.bus.rom[0x6df4d8..0x6df4dc].fill(0xa8);machine.bus.rom[0x6df5b4..0x6df5bc].fill(0xa9);
    for (offset,length,_,_) in SOURCE_SPANS {
        for at in offset..offset+length {assert_eq!(machine.bus.read8(0x0c000000+at),rom[at as usize],"byte {at:06x}");}
        for edge in [offset as usize,(offset+length) as usize] {for delta in [-3isize,-1,0,1] {
            let at=(edge as isize+delta) as usize;let ptr=&mut machine as *mut Machine as *mut std::ffi::c_void;
            assert_eq!((RT_API.read16)(ptr,0x0a000000+at as u32),u32::from(u16::from_le_bytes(rom[at..at+2].try_into().unwrap())));
            assert_eq!((RT_API.read32)(ptr,0x08000000+at as u32),u32::from_le_bytes(rom[at..at+4].try_into().unwrap()));
        }}
    }
    machine.bus.rom[BANK_ROOT as usize..BANK_ROOT as usize+8].fill(0xab);for at in BANK_ROOT..BANK_ROOT+8 {assert_eq!(machine.bus.read8(0x08000000+at),0xab,"row0 remains raw");}
    machine.bus.rom.copy_from_slice(&rom);assert_eq!(machine.save_state().unwrap(),state);
    let mut bad=parse_conditional_descriptor_bank_zero(&bytes).unwrap();bad.records[0].field0^=1;let bad_bytes=encode_conditional_descriptor_bank_zero(&bad).unwrap();let mut bad_manifest=source.manifest.clone();bad_manifest.resources[88].sha256=gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE),bad_bytes).unwrap();fs::write(dataset.join(MANIFEST_FILE),toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();assert!(attach_from_directory(&mut machine,&dataset).is_err());assert!(std::sync::Arc::ptr_eq(machine.bus.rom_resources().unwrap(),&owner));assert_eq!(machine.save_state().unwrap(),state);fs::write(dataset.join(CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE),bytes).unwrap();fs::write(dataset.join(MANIFEST_FILE),manifest).unwrap();
    let prefixes=[(0x0d097eu32,0x0d0986u32),(0x137a5c,0x137a5e),(0x137a66,0x137a68)];let mut visited=BTreeSet::new();for (a,b) in prefixes {for at in (a..b).step_by(2) {visited.insert(0x08000000+at);}}
    let view=View {rom:&rom,ewram:None,iwram:None,bios:None};let blocks=visited.iter().map(|&addr|Block {addr,thumb:true,instrs:vec![armv4t::decode_thumb(view.read16(addr),addr)]}).collect();let object=root.join("bank-zero-native.o");recomp_core::tcc_memory::build(&Analysis {blocks},&view,&recomp_core::tcc_memory::library_path().unwrap(),&object,&mut |_| {}).unwrap();let (_library,table)=crate::load_native(&object).unwrap();
    let mut raw=crate::make_machine(rom.clone(),None);let mut native=crate::make_machine(rom.clone(),None);attach_from_directory(&mut native,&dataset).unwrap();let initial=raw.save_state().unwrap();let native_owner=native.bus.rom_resources().unwrap().clone();let cases=[(0u8,0u8),(0,1),(0,15),(0,16),(1,0)];let mut served=0;
    for (prefix,(begin,end)) in prefixes.into_iter().enumerate() {for (bank,row) in cases {
        let po=0x6df4d4+usize::from(bank)*4;let bankptr=u32::from_le_bytes(rom[po..po+4].try_into().unwrap());let desc=bankptr+u32::from(row)*8;
        for m in [&mut raw,&mut native] {m.load_state(&initial).unwrap();m.bus.ffta_sprites.enabled=false;m.cpu.regs=std::array::from_fn(|i|0x11000000+i as u32);m.cpu.regs[13]=0x03007c00;m.cpu.regs[1]=desc+4;m.cpu.regs[6]=u32::from(bank);m.cpu.regs[15]=0x08000000+begin;m.cpu.cpsr=0xbf;}
        for (offset,length,_,_) in SOURCE_SPANS {native.bus.rom[offset as usize..(offset+length) as usize].fill(0xcd);}native.bus.rom[0x6df4d8..0x6df4dc].fill(0xa8);native.bus.rom[0x6df5b4..0x6df5bc].fill(0xa9);
        while raw.cpu.regs[15]!=0x08000000+end {raw.bus.frame_ready=false;raw.step();}
        let before=native_owner.conditional_descriptor_bank_zero_stats().unwrap().source_bytes;let prior_before=native_owner.conditional_descriptor_banks_stats().unwrap().source_bytes;let mut n=0;
        while native.cpu.regs[15]!=0x08000000+end {assert!(n<12,"pc {:08x}",native.cpu.regs[15]);native.bus.frame_ready=false;table.get(native.cpu.regs[15]|1).unwrap()(&RT_API,&mut native as *mut Machine as *mut std::ffi::c_void);n+=1;}
        let used=native_owner.conditional_descriptor_bank_zero_stats().unwrap().source_bytes-before;let prior_used=native_owner.conditional_descriptor_banks_stats().unwrap().source_bytes-prior_before;
        let expected=if prefix==0 {if bank==0 {4} else {0}} else if bank==0 && (1..=15).contains(&row) {if prefix==1 {1} else {2}} else {0};
        let prior_expected=if prefix==0 {if bank==1 {4} else {0}} else if (bank==0 && row==16)||(bank==1 && row==0) {if prefix==1 {1} else {2}} else {0};
        assert_eq!(used,expected,"new bank {bank} row {row} prefix {prefix}");assert_eq!(prior_used,prior_expected,"prior bank {bank} row {row} prefix {prefix}");served+=used;native.bus.rom.copy_from_slice(&rom);assert_eq!(native.save_state().unwrap(),raw.save_state().unwrap(),"bank {bank} row {row} prefix {prefix}");
    }}
    fs::write(root.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","resources":90,"previous_unchanged":88,"source_bytes":124,"native_prefixes":3,"cases_per_prefix":5,"typed_native_bytes":served,"record0_raw":true,"prior_owner_composition":true,"full_state_clock":true,"tamper_transaction":true,"Linux":"pending","USA":"pending","nativeWindows":"pending","performance_measurements":false})).unwrap()).unwrap();
}
