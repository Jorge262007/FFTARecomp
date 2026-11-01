//! Managed EUR descriptor banks, original native prefixes and excluded ranges.
use super::*;
use gba_core::{Bus,Machine,capi::RT_API};
use gba_core::rom_data::conditional_descriptor_banks::{DescriptorBanks,SOURCE_SPANS,TABLE_ROOT};
use recomp_core::analyze::{Analysis,Block,View};
use std::collections::BTreeSet;

#[test]
#[ignore = "private EUR ROM and TinyCC; Wine only, Linux/USA pending"]
fn european_conditional_descriptor_banks_managed_native_reads() {
    let root=PathBuf::from(std::env::var_os("FFTA_DESCRIPTOR_RUNTIME_ROOT").unwrap());assert!(!root.exists());fs::create_dir_all(&root).unwrap();
    let rom=fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();let old=PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior:Manifest=toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    let mut source=Extraction::from_rom(&rom).unwrap();let dataset=root.join("offline dataset");extract(&dataset,&mut source).unwrap();
    assert_eq!((source.manifest.extractor_revision,source.manifest.decoder_revision,source.manifest.resources.len()),(106,105,96));assert_eq!(prior.resources.len(),87);
    for r in &prior.resources {assert_eq!(source.manifest.resources.iter().find(|e|e.id==r.id),Some(r));assert_eq!(fs::read(dataset.join(&r.path)).unwrap(),fs::read(old.join(&r.path)).unwrap());}
    let entry=&source.manifest.resources[87];assert_eq!(entry.id,CONDITIONAL_DESCRIPTOR_BANKS_ID);assert_eq!(entry.sources.len(),3);
    for (s,(offset,length,_,_)) in entry.sources.iter().zip(SOURCE_SPANS) {assert_eq!((s.offset,s.length),(offset,length));}
    let bytes=fs::read(dataset.join(CONDITIONAL_DESCRIPTOR_BANKS_FILE)).unwrap();assert_eq!(parse_conditional_descriptor_banks(&bytes).unwrap(),source.conditional_descriptor_banks.as_ref().unwrap().decoded);
    let manifest=fs::read(dataset.join(MANIFEST_FILE)).unwrap();let mut machine=crate::make_machine(rom.clone(),None);attach_from_directory(&mut machine,&dataset).unwrap();let state=machine.save_state().unwrap();let owner=machine.bus.rom_resources().unwrap().clone();
    for (offset,length,_,_) in SOURCE_SPANS {machine.bus.rom[offset as usize..(offset+length) as usize].fill(0xcd);}
    for (offset,length,_,_) in SOURCE_SPANS {
        for at in offset..offset+length {assert_eq!(machine.bus.read8(0x0c000000+at),rom[at as usize],"byte {at:06x}");}
        for edge in [offset as usize,(offset+length) as usize] {for delta in [-3isize,-1,0,1] {
            let at=(edge as isize+delta) as usize;let ptr=&mut machine as *mut Machine as *mut std::ffi::c_void;
            assert_eq!((RT_API.read16)(ptr,0x0a000000+at as u32),u32::from(u16::from_le_bytes(rom[at..at+2].try_into().unwrap())));
            assert_eq!((RT_API.read32)(ptr,0x08000000+at as u32),u32::from_le_bytes(rom[at..at+4].try_into().unwrap()));
        }}
    }
    // e99 owns pointer0; bank0 record0 and the bank22 gap remain raw.
    machine.bus.rom[TABLE_ROOT as usize..TABLE_ROOT as usize+4].fill(0xa8);
    machine.bus.rom[0x6df534..0x6df53c].fill(0xab);
    machine.bus.rom[0x6e0acc..0x6e0afc].fill(0xa9);
    for at in TABLE_ROOT..TABLE_ROOT+4 {assert_eq!(machine.bus.read8(0x08000000+at),rom[at as usize]);}
    for at in 0x6df534..0x6df53c {assert_eq!(machine.bus.read8(0x08000000+at),0xab);}
    for at in 0x6e0acc..0x6e0afc {assert_eq!(machine.bus.read8(0x08000000+at),0xa9);}
    machine.bus.rom.copy_from_slice(&rom);assert_eq!(machine.save_state().unwrap(),state);
    let mut bad=parse_conditional_descriptor_banks(&bytes).unwrap();bad.banks[0].records[0].field0^=1;let bad_bytes=encode_conditional_descriptor_banks(&bad).unwrap();let mut bad_manifest=source.manifest.clone();bad_manifest.resources[87].sha256=gba_core::ffta_rom::digest(&bad_bytes);
    fs::write(dataset.join(CONDITIONAL_DESCRIPTOR_BANKS_FILE),bad_bytes).unwrap();fs::write(dataset.join(MANIFEST_FILE),toml::to_string_pretty(&bad_manifest).unwrap()).unwrap();assert!(attach_from_directory(&mut machine,&dataset).is_err());assert!(std::sync::Arc::ptr_eq(machine.bus.rom_resources().unwrap(),&owner));assert_eq!(machine.save_state().unwrap(),state);fs::write(dataset.join(CONDITIONAL_DESCRIPTOR_BANKS_FILE),bytes).unwrap();fs::write(dataset.join(MANIFEST_FILE),manifest).unwrap();
    let prefixes=[(0x0d0ed4u32,0x0d0ee4u32),(0x080484,0x080486),(0x080496,0x08049a),(0x131dee,0x131df0),(0x0d0aae,0x0d0ab0)];let mut visited=BTreeSet::new();for (a,b) in prefixes {for at in (a..b).step_by(2) {visited.insert(0x08000000+at);}}
    let view=View {rom:&rom,ewram:None,iwram:None,bios:None};let blocks=visited.iter().map(|&addr|Block {addr,thumb:true,instrs:vec![armv4t::decode_thumb(view.read16(addr),addr)]}).collect();let object=root.join("descriptor-native.o");recomp_core::tcc_memory::build(&Analysis {blocks},&view,&recomp_core::tcc_memory::library_path().unwrap(),&object,&mut |_| {}).unwrap();let (_library,table)=crate::load_native(&object).unwrap();
    let mut raw=crate::make_machine(rom.clone(),None);let mut native=crate::make_machine(rom.clone(),None);attach_from_directory(&mut native,&dataset).unwrap();let initial=raw.save_state().unwrap();let native_owner=native.bus.rom_resources().unwrap().clone();let cases=[(0u8,0u8),(1,0),(1,141),(22,10),(22,11),(22,16),(23,8),(23,9)];let mut served=0;
    for (prefix,(begin,end)) in prefixes.into_iter().enumerate() {for (bank,row) in cases {
        let pointer_offset=TABLE_ROOT as usize+usize::from(bank)*4;let pointer=u32::from_le_bytes(rom[pointer_offset..pointer_offset+4].try_into().unwrap());let descriptor=DescriptorBanks::encoded_record_view(pointer,u32::from(row));let selected=(bank==1 && row<=141)||(bank==22 && row<11)||(bank==23 && row<9);
        for m in [&mut raw,&mut native] {m.load_state(&initial).unwrap();m.bus.ffta_sprites.enabled=false;m.cpu.regs=std::array::from_fn(|i|0x11000000+i as u32);m.cpu.regs[13]=0x03007c00;m.cpu.regs[0]=if prefix==0 {u32::from(bank)} else {0};m.cpu.regs[1]=u32::from(row);m.cpu.regs[3]=descriptor;m.cpu.regs[4]=descriptor;m.cpu.regs[6]=0x02010000;m.cpu.regs[15]=0x08000000+begin;m.cpu.cpsr=0xbf;}
        for (offset,length,_,_) in SOURCE_SPANS {native.bus.rom[offset as usize..(offset+length) as usize].fill(0xcd);}
        while raw.cpu.regs[15]!=0x08000000+end {raw.bus.frame_ready=false;raw.step();}
        let before=native_owner.conditional_descriptor_banks_stats().unwrap().source_bytes;let mut dispatches=0;
        while native.cpu.regs[15]!=0x08000000+end {assert!(dispatches<16,"pc {:08x}",native.cpu.regs[15]);native.bus.frame_ready=false;table.get(native.cpu.regs[15]|1).unwrap()(&RT_API,&mut native as *mut Machine as *mut std::ffi::c_void);dispatches+=1;}
        let used=native_owner.conditional_descriptor_banks_stats().unwrap().source_bytes-before;
        let expected=if prefix==0 {if bank!=0 {4} else {0}} else if selected {if prefix==4 {1} else {2}} else {0};
        assert_eq!(used,expected,"bank {bank} row {row} prefix {prefix}");served+=used;native.bus.rom.copy_from_slice(&rom);assert_eq!(native.save_state().unwrap(),raw.save_state().unwrap(),"bank {bank} row {row} prefix {prefix}");
    }}
    fs::write(root.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","resources":90,"previous_unchanged":87,"source_bytes":5564,"native_prefixes":5,"cases_per_prefix":8,"typed_native_bytes":served,"pointer0_typed_bank0_row0_and_gap_raw":true,"full_state_clock":true,"tamper_transaction":true,"Linux":"pending","USA":"pending","nativeWindows":"pending","performance_measurements":false})).unwrap()).unwrap();
}
