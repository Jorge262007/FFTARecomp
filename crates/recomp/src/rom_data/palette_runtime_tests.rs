//! Original interpreter/native producers with poisoned old palette storage.
use super::*;
use gba_core::{capi::RT_API, rom_data::bg_palettes::Catalog, Bus, Machine};
use recomp_core::analyze::{Analysis, Block, View};
use std::sync::Arc;

fn equal(left: &Machine, right: &Machine) {
    assert_eq!(left.cpu.regs, right.cpu.regs);
    assert_eq!(left.cpu.cpsr, right.cpu.cpsr);
    assert_eq!(left.bus.clock, right.bus.clock);
    assert!(left.bus.ewram == right.bus.ewram);
    assert!(left.bus.iwram == right.bus.iwram);
}

fn build_component(rom: &[u8], entry: u32, output: &Path) {
    let view = View {
        rom,
        ewram: None,
        iwram: None,
        bios: None,
    };
    let mut entries = vec![entry];
    // Select actual BIOS wrapper callees from BL pairs, not the unrelated
    // alternate-palette program. The general analyzer seeds the whole ROM.
    for address in (entry..entry + 256).step_by(2) {
        let instruction = armv4t::decode_thumb(view.read16(address), address);
        if matches!(instruction.op, armv4t::Op::Bx { .. }) {
            break;
        }
        let next = armv4t::decode_thumb(view.read16(address + 2), address + 2);
        if let Some((target, _)) = armv4t::fuse_thumb_bl(&instruction, &next) {
            if matches!(
                armv4t::decode_thumb(view.read16(target), target).op,
                armv4t::Op::Swi { imm: 0x0b | 0x11 }
            ) {
                entries.push(target);
            }
        }
    }
    entries.sort_unstable();
    entries.dedup();
    assert_eq!(entries.len(), 3);
    let mut blocks = Vec::new();
    for start in entries {
        let mut returned = false;
        for address in (start..start + 256).step_by(2) {
            let instruction = armv4t::decode_thumb(view.read16(address), address);
            let end = matches!(instruction.op, armv4t::Op::Bx { .. });
            let next = armv4t::decode_thumb(view.read16(address + 2), address + 2);
            let fused = armv4t::fuse_thumb_bl(&instruction, &next).is_some();
            let mut instrs = vec![instruction];
            if fused {
                instrs.push(next);
            }
            blocks.push(Block {
                addr: address,
                thumb: true,
                instrs,
            });
            if end {
                returned = true;
                break;
            }
        }
        assert!(returned, "bounded original function must return");
    }
    recomp_core::tcc_memory::build(
        &Analysis { blocks },
        &view,
        &recomp_core::tcc_memory::library_path().unwrap(),
        output,
        &mut |_| {},
    )
    .unwrap();
}

#[test]
#[ignore = "requires exact ROM, current dataset, TinyCC and new FFTA_STATE_TEST_PALETTE_RUNTIME_ROOT"]
fn original_interpreter_and_native_palette_producers_use_files_with_legacy_states() {
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_PALETTE_RUNTIME_ROOT").unwrap());
    assert!(!root.exists(), "preserve previous runtime evidence");
    let dataset = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let catalog = Catalog::from_rom(&rom).unwrap();
    let entry = match gba_core::ffta_rom::digest(&rom).as_str() {
        gba_core::ffta_rom::SHA256 => 0x0801a42c,
        gba_core::ffta_rom::EUROPE_SHA256 => 0x0801b59c,
        _ => panic!("exact image required"),
    };
    fs::create_dir_all(&root).unwrap();
    let object = root.join("producer.o");
    build_component(&rom, entry, &object);
    let (_library, table) = crate::load_native(&object).unwrap();
    let mut legacy = crate::make_machine(rom.clone(), None);
    let mut decoded = crate::make_machine(rom.clone(), None);
    let mut native = crate::make_machine(rom.clone(), None);
    let boot = legacy.save_state().unwrap();
    for machine in [&mut decoded, &mut native] {
        attach_from_directory(machine, &dataset).unwrap();
        assert_eq!(machine.save_state().unwrap(), boot);
        for source in &catalog.sources {
            let r = &source.range;
            machine.bus.rom[r.offset as usize..(r.offset + r.length) as usize].fill(0xcd);
        }
    }
    let mut calls = 0;
    let mut steps = 0;
    let mut dispatches = 0;
    const RETURN: u32 = 0x08000300;
    for environment in [0u8, 3, 0xfc, 0xff] {
        for id in 0..catalog.decoded.entries.len() {
            for argument in [id as u32, 0xffff] {
                for machine in [&mut legacy, &mut decoded, &mut native] {
                    machine.bus.write8(0x02002fc2, environment);
                    machine.bus.write16(0x02007f10, id as u16);
                    machine.cpu.cpsr = 0xbf;
                    machine.cpu.regs[0] = argument;
                    machine.cpu.regs[1] = 0x03003860;
                    machine.cpu.regs[13] = 0x03007c00;
                    machine.cpu.regs[14] = RETURN | 1;
                    machine.cpu.regs[15] = entry;
                }
                let mut iterations = 0;
                while native.cpu.regs[15] != RETURN {
                    iterations += 1;
                    assert!(iterations < 1000);
                    if native.bus.frame_ready {
                        assert!(legacy.bus.frame_ready && decoded.bus.frame_ready);
                        for machine in [&mut legacy, &mut decoded, &mut native] {
                            machine.bus.frame_ready = false;
                        }
                    }
                    let f = table
                        .get(native.cpu.regs[15] | 1)
                        .expect("compiled producer/wrapper block");
                    f(
                        &RT_API,
                        &mut native as *mut Machine as *mut std::ffi::c_void,
                    );
                    dispatches += 1;
                    while legacy.bus.clock < native.bus.clock {
                        legacy.step();
                        decoded.step();
                        equal(&legacy, &decoded);
                        steps += 1;
                    }
                    equal(&legacy, &native);
                }
                let expected = catalog.decoded.entries[id].source_bytes().unwrap();
                assert_eq!(&native.bus.iwram[0x3860..0x3860 + expected.len()], expected);
                calls += 1;
            }
        }
    }
    for machine in [&mut decoded, &mut native] {
        for source in &catalog.sources {
            let r = &source.range;
            machine.bus.rom[r.offset as usize..(r.offset + r.length) as usize]
                .copy_from_slice(&rom[r.offset as usize..(r.offset + r.length) as usize]);
        }
        assert_eq!(machine.save_state().unwrap(), legacy.save_state().unwrap());
        let stats = machine
            .bus
            .rom_resources()
            .unwrap()
            .palette_stats()
            .unwrap();
        assert_eq!(stats.lz77_calls, 1272);
        assert!(stats.bus_reads > 0 && stats.decoded_bytes > 0);
    }
    lifecycle(&mut decoded, &dataset, &root, &boot, &catalog);
    let resources = native.bus.rom_resources().unwrap();
    let stats = resources.palette_stats().unwrap();
    fs::write(root.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"calls":calls,"original_instructions":steps,"native_dispatches":dispatches,"lz77_calls":stats.lz77_calls,"decoded_bytes":stats.decoded_bytes,"bus_reads":stats.bus_reads,"source_bytes":stats.source_bytes,"full_states_equal":true,"source_poisoned":true,"lifecycle":true,"owned_bytes":resources.resident_bytes()+2*std::mem::size_of::<usize>(),"palette_owned_bytes":resources.palette_resident_bytes()})).unwrap()).unwrap();
    eprintln!("PALETTE_RUNTIME calls={calls} steps={steps} native_dispatches={dispatches} lz77_calls={} decoded_bytes={} bus_reads={} full_states=true lifecycle=true",stats.lz77_calls,stats.decoded_bytes,stats.bus_reads);
}

fn lifecycle(machine: &mut Machine, dataset: &Path, root: &Path, boot: &[u8], catalog: &Catalog) {
    let local = root.join("dataset");
    fs::create_dir(&local).unwrap();
    let names = [
        MANIFEST_FILE,
        OBJ_DIMENSIONS_FILE,
        BG_DESCRIPTORS_FILE,
        BG_BASE_PALETTES_FILE,
    ];
    for name in names {
        fs::copy(dataset.join(name), local.join(name)).unwrap();
    }
    attach_from_directory(machine, &local).unwrap();
    let resources = machine.bus.rom_resources().unwrap().clone();
    let before = machine.save_state().unwrap();
    fs::write(local.join(BG_BASE_PALETTES_FILE), b"broken").unwrap();
    assert!(attach_from_directory(machine, &local).is_err());
    fs::remove_file(local.join(BG_BASE_PALETTES_FILE)).unwrap();
    assert!(attach_from_directory(machine, &local).is_err());
    assert!(Arc::ptr_eq(
        machine.bus.rom_resources().unwrap(),
        &resources
    ));
    assert_eq!(machine.save_state().unwrap(), before);
    let save = root.join("saves/test.sav");
    fs::create_dir_all(save.parent().unwrap()).unwrap();
    let mut slots = crate::save_state::Slots::new(&save, &machine.bus.rom);
    slots
        .save(machine, 1, &vec![[0; 4]; 240 * 160], false)
        .unwrap();
    machine.cpu.regs[4] = 37;
    let modified = machine.save_state().unwrap();
    let counters = resources.palette_stats().unwrap();
    slots.load(machine, 1).unwrap();
    assert_eq!(machine.save_state().unwrap(), before);
    slots.undo(machine).unwrap();
    assert_eq!(machine.save_state().unwrap(), modified);
    machine.load_state(boot).unwrap();
    assert!(Arc::ptr_eq(
        machine.bus.rom_resources().unwrap(),
        &resources
    ));
    assert_eq!(resources.palette_stats().unwrap(), counters);
    let length = machine.bus.save_data().unwrap().len();
    fs::write(&save, vec![7; length]).unwrap();
    let backups = crate::save_backups::Backups::new(&save);
    backups.capture_startup().unwrap();
    let generation = backups.choices(length)[0].generation.unwrap();
    backups.restore(generation, machine, boot).unwrap();
    assert!(Arc::ptr_eq(
        machine.bus.rom_resources().unwrap(),
        &resources
    ));
    assert_eq!(resources.palette_stats().unwrap(), counters);
    let source = &catalog.sources[0].range;
    let expected = catalog.decoded.entries[0].source_bytes().unwrap();
    let original =
        machine.bus.rom[source.offset as usize..(source.offset + source.length) as usize].to_vec();
    machine.bus.rom[source.offset as usize..(source.offset + source.length) as usize].fill(0xcd);
    machine.cpu.regs[0] = 0x08000000 + source.offset;
    machine.cpu.regs[1] = 0x03003860;
    assert!(gba_core::hle::bios_call(
        &mut machine.cpu,
        &mut machine.bus,
        0x11
    ));
    assert_eq!(
        &machine.bus.iwram[0x3860..0x3860 + expected.len()],
        expected
    );
    machine.bus.rom[source.offset as usize..(source.offset + source.length) as usize]
        .copy_from_slice(&original);
    assert!(resources.palette_stats().unwrap().lz77_calls > counters.lz77_calls);
}
