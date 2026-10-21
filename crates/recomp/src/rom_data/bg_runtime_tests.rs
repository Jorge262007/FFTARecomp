//! Real-file provenance through original regional code and native translation.
use super::*;
use gba_core::{capi::RT_API, rom_data::bg_descriptors::Catalog, Bus, Machine};
use recomp_core::analyze::{Analysis, Block, View};
use std::{cell::Cell, sync::Arc};

thread_local! {
    static SHELLS: Cell<u64> = const { Cell::new(0) };
    static NATIVE_BG: Cell<(u32, u32, u64)> = const { Cell::new((0, 0, 0)) };
}
extern "C" fn counted_read8(pointer: *mut std::ffi::c_void, address: u32) -> u32 {
    NATIVE_BG.with(|count| {
        let (start, end, reads) = count.get();
        if (start..end).contains(&address) {
            count.set((start, end, reads + 1));
        }
    });
    (RT_API.read8)(pointer, address)
}
extern "C" fn counted_interp(pointer: *mut std::ffi::c_void) -> u32 {
    SHELLS.with(|count| count.set(count.get() + 1));
    (RT_API.interp_one)(pointer)
}

fn equal_execution(left: &Machine, right: &Machine) {
    assert_eq!(left.cpu.regs, right.cpu.regs);
    assert_eq!(left.cpu.cpsr, right.cpu.cpsr);
    assert_eq!(left.bus.clock, right.bus.clock);
    assert!(left.bus.ewram == right.bus.ewram);
    assert!(left.bus.iwram == right.bus.iwram);
}

// A deliberately bounded analysis of the two actual functions. The general
// analyzer always seeds the cartridge entry and would translate unrelated code.
fn component(rom: &[u8], entry: u32, output: &Path) {
    let view = View {
        rom,
        ewram: None,
        iwram: None,
        bios: None,
    };
    let mut entries = vec![entry];
    for address in (entry..entry + 128).step_by(2) {
        let high = armv4t::decode_thumb(view.read16(address), address);
        let low = armv4t::decode_thumb(view.read16(address + 2), address + 2);
        if let Some((target, _)) = armv4t::fuse_thumb_bl(&high, &low) {
            entries.push(target);
            break;
        }
    }
    assert_eq!(entries.len(), 2, "original selector calls the query helper");
    let mut blocks = Vec::new();
    for start in entries {
        let mut returned = false;
        for address in (start..start + 128).step_by(2) {
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
    let library = recomp_core::tcc_memory::library_path().expect("existing TinyCC library");
    recomp_core::tcc_memory::build(&Analysis { blocks }, &view, &library, output, &mut |_| {})
        .unwrap();
}

#[test]
#[ignore = "requires exact private ROM, existing e3 dataset, TinyCC and new FFTA_STATE_TEST_BG_RUNTIME_ROOT"]
fn regional_file_backed_selectors_native_and_lifecycle() {
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_BG_RUNTIME_ROOT").unwrap());
    assert!(!root.exists(), "preserve previous evidence");
    let dataset = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let catalog = Catalog::from_rom(&rom).unwrap();
    let entry = match gba_core::ffta_rom::digest(&rom).as_str() {
        gba_core::ffta_rom::SHA256 => 0x0801_db98,
        gba_core::ffta_rom::EUROPE_SHA256 => 0x0801_ed08,
        _ => panic!("exact regional image required"),
    };
    fs::create_dir_all(&root).unwrap();
    let object = root.join("selector.o");
    component(&rom, entry, &object);
    let (_library, table) = crate::load_native(&object).unwrap();
    let mut legacy = crate::make_machine(rom.clone(), None);
    let mut decoded = crate::make_machine(rom.clone(), None);
    let mut native = crate::make_machine(rom.clone(), None);
    let boot = legacy.save_state().unwrap();
    for machine in [&mut decoded, &mut native] {
        attach_from_directory(machine, &dataset).unwrap();
        assert_eq!(machine.save_state().unwrap(), boot);
        let resources = machine.bus.rom_resources().unwrap();
        assert_eq!(
            resources.background_file_sha256().unwrap(),
            gba_core::ffta_rom::digest(&fs::read(dataset.join(BG_DESCRIPTORS_FILE)).unwrap())
        );
        assert!(resources.obj_resident_bytes() + 2 * std::mem::size_of::<usize>() <= 1024);
        assert!(resources.background_resident_bytes() <= 64 * 1024);
        let offset = catalog.source.offset as usize;
        machine.bus.rom[offset..offset + catalog.source.length as usize].fill(0xcd);
    }
    let api = gba_core::capi::RtApi {
        interp_one: counted_interp,
        read8: counted_read8,
        ..RT_API
    };
    SHELLS.with(|count| count.set(0));
    NATIVE_BG.with(|count| {
        count.set((
            0x08000000 + catalog.source.offset,
            0x08000000 + catalog.source.offset + catalog.source.length,
            0,
        ))
    });
    let mut cases = 0usize;
    let mut instructions = 0usize;
    let mut dispatches = 0usize;
    const RETURN: u32 = 0x0800_0300;
    for environment in [0u8, 1, 2, 3, 0xfc, 0xfd, 0xfe, 0xff] {
        for id in 0..=catalog.decoded.entries.len() {
            let sentinel = id == catalog.decoded.entries.len();
            let id = if sentinel { 0xff } else { id as u16 };
            for argument in [id as u32, 0xffff] {
                for machine in [&mut legacy, &mut decoded, &mut native] {
                    machine.bus.write8(0x0200_2fc2, environment);
                    machine.bus.write16(0x0200_7f10, id);
                    machine.cpu.regs[0] = argument;
                    machine.cpu.regs[13] = 0x0300_7c00;
                    machine.cpu.regs[14] = RETURN | 1;
                    machine.cpu.regs[15] = entry;
                    machine.cpu.cpsr = 0xbf;
                }
                let before = native
                    .bus
                    .rom_resources()
                    .unwrap()
                    .background_stats()
                    .unwrap();
                let mut blocks = 0;
                while native.cpu.regs[15] != RETURN {
                    blocks += 1;
                    assert!(blocks < 1000, "native selector must return");
                    if native.bus.frame_ready {
                        assert!(legacy.bus.frame_ready && decoded.bus.frame_ready);
                        for machine in [&mut legacy, &mut decoded, &mut native] {
                            machine.bus.frame_ready = false;
                        }
                    }
                    let function = table
                        .get(native.cpu.regs[15] | 1)
                        .expect("native block present");
                    function(&api, &mut native as *mut Machine as *mut std::ffi::c_void);
                    dispatches += 1;
                    while legacy.bus.clock < native.bus.clock {
                        legacy.step();
                        decoded.step();
                        equal_execution(&legacy, &decoded);
                        instructions += 1;
                    }
                    equal_execution(&legacy, &native);
                }
                let expected = if sentinel {
                    0xff
                } else {
                    catalog.decoded.entries[id as usize].selected_selector(environment)
                };
                assert_eq!(legacy.cpu.regs[0], expected as u32);
                if !sentinel {
                    assert!(
                        native
                            .bus
                            .rom_resources()
                            .unwrap()
                            .background_stats()
                            .unwrap()
                            .bus_reads
                            > before.bus_reads
                    );
                }
                cases += 1;
            }
        }
    }
    let shells = SHELLS.with(Cell::get);
    let native_bg_reads = NATIVE_BG.with(|count| count.get().2);
    assert!(
        native_bg_reads > 0,
        "translated byte loads must call the BG bus API"
    );
    let offset = catalog.source.offset as usize;
    for machine in [&mut decoded, &mut native] {
        machine.bus.rom[offset..offset + catalog.source.length as usize]
            .copy_from_slice(&rom[offset..offset + catalog.source.length as usize]);
        assert_eq!(legacy.save_state().unwrap(), machine.save_state().unwrap());
        assert_eq!(
            machine.bus.rom_resources().unwrap().stats().bus_reads,
            0,
            "BG counters are distinct"
        );
    }
    lifecycle(&mut decoded, &dataset, &root, &boot, offset);
    let resources = native.bus.rom_resources().unwrap();
    let stats = resources.background_stats().unwrap();
    fs::write(root.join("result.txt"), format!("cases={cases}\ninstructions={instructions}\nnative_dispatches={dispatches}\ninterpreter_shells={shells}\nnative_bg_api_reads={native_bg_reads}\nbg_reads={}\nprojected_bytes={}\nobj_shared_bytes={}\nbg_owned_bytes={}\nsource_poisoned=true\nfull_states_equal=true\nlifecycle=true\n", stats.bus_reads, stats.projected_bytes, resources.obj_resident_bytes() + 2 * std::mem::size_of::<usize>(), resources.background_resident_bytes())).unwrap();
    eprintln!("BG_RUNTIME cases={cases} instructions={instructions} native_dispatches={dispatches} interpreter_shells={shells} native_bg_api_reads={native_bg_reads} bg_reads={} projected_bytes={} lifecycle=true", stats.bus_reads, stats.projected_bytes);
}

fn lifecycle(machine: &mut Machine, dataset: &Path, root: &Path, boot: &[u8], offset: usize) {
    let local = root.join("dataset");
    fs::create_dir(&local).unwrap();
    for name in [
        MANIFEST_FILE,
        OBJ_DIMENSIONS_FILE,
        BG_DESCRIPTORS_FILE,
        BG_BASE_PALETTES_FILE,
    ] {
        fs::copy(dataset.join(name), local.join(name)).unwrap();
    }
    attach_from_directory(machine, &local).unwrap();
    let resources = machine.bus.rom_resources().unwrap().clone();
    let before = machine.save_state().unwrap();
    fs::write(local.join(BG_DESCRIPTORS_FILE), b"invalid").unwrap();
    assert!(attach_from_directory(machine, &local).is_err());
    fs::remove_file(local.join(BG_DESCRIPTORS_FILE)).unwrap();
    assert!(attach_from_directory(machine, &local).is_err());
    assert!(Arc::ptr_eq(
        machine.bus.rom_resources().unwrap(),
        &resources
    ));
    assert_eq!(before, machine.save_state().unwrap());
    let mut fresh = crate::make_machine(machine.bus.rom.clone(), None);
    assert!(attach_from_directory(&mut fresh, &local).is_err());
    assert!(fresh.bus.rom_resources().is_none());
    let save = root.join("saves/test.sav");
    fs::create_dir_all(save.parent().unwrap()).unwrap();
    let mut slots = crate::save_state::Slots::new(&save, &machine.bus.rom);
    slots
        .save(machine, 1, &vec![[0; 4]; 240 * 160], false)
        .unwrap();
    machine.cpu.regs[4] = 37;
    let modified = machine.save_state().unwrap();
    let count = resources.background_stats().unwrap();
    slots.load(machine, 1).unwrap();
    assert_eq!(machine.save_state().unwrap(), before);
    slots.undo(machine).unwrap();
    assert_eq!(machine.save_state().unwrap(), modified);
    machine.load_state(boot).unwrap();
    assert!(Arc::ptr_eq(
        machine.bus.rom_resources().unwrap(),
        &resources
    ));
    assert_eq!(resources.background_stats().unwrap(), count);
    let length = machine.bus.save_data().unwrap().len();
    let backups = crate::save_backups::Backups::new(&save);
    fs::write(&save, vec![7; length]).unwrap();
    backups.capture_startup().unwrap();
    let generation = backups.choices(length)[0].generation.unwrap();
    backups.restore(generation, machine, boot).unwrap();
    assert!(Arc::ptr_eq(
        machine.bus.rom_resources().unwrap(),
        &resources
    ));
    assert_eq!(resources.background_stats().unwrap(), count);
    assert_eq!(machine.bus.save_data().unwrap(), vec![7; length]);
    let expected = u32::from_le_bytes(machine.bus.rom[offset..offset + 4].try_into().unwrap());
    machine.bus.rom[offset..offset + 4].fill(0xcd);
    assert_eq!(machine.bus.read32(0x08000000 + offset as u32), expected);
    machine.bus.rom[offset..offset + 4].copy_from_slice(&expected.to_le_bytes());
    assert!(resources.background_stats().unwrap().bus_reads > count.bus_reads);
}

#[test]
#[ignore = "requires FFTA_STATE_TEST_ROM and FFTA_STATE_TEST_NATIVE local fixtures"]
fn background_file_matches_legacy_native_boot_states_images_audio_and_dispatch() {
    use crate::{
        arm_audio_hle, demo_keys, load_native, make_machine, run_frame_native, FRAME_STEP_GUARD,
    };
    crate::configure_headless_widescreen();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
    let library = std::env::var_os("FFTA_STATE_TEST_NATIVE").expect("Native fixture");
    let (_library, table) = load_native(Path::new(&library)).unwrap();
    let mut legacy = make_machine(rom.clone(), None);
    let mut decoded = make_machine(rom.clone(), None);
    for machine in [&mut legacy, &mut decoded] {
        arm_audio_hle(machine, None);
        machine.bus.tap_channels = true;
    }
    let directory = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    attach_from_directory(&mut decoded, &directory).unwrap();
    assert_eq!(legacy.save_state().unwrap(), decoded.save_state().unwrap());
    let mut dispatch = (0u64, 0u64);
    let mut audio = [0u64; 5];
    for frame in 0..1200 {
        let mut counts = [(0, 0); 2];
        for (index, machine) in [&mut legacy, &mut decoded].into_iter().enumerate() {
            machine.bus.keys = demo_keys(machine.bus.frames);
            let pointer = machine as *mut gba_core::Machine as *mut std::ffi::c_void;
            counts[index] = run_frame_native(machine, &table, pointer, FRAME_STEP_GUARD);
            assert!(machine.bus.frame_ready);
        }
        assert_eq!(counts[0], counts[1], "dispatch at frame {frame}");
        if frame % 300 == 0 {
            eprintln!("BG_BOOT progress={frame}/1200");
        }
        dispatch.0 += counts[0].0;
        dispatch.1 += counts[0].1;
        assert_eq!(
            legacy.bus.framebuffer, decoded.bus.framebuffer,
            "framebuffer at {frame}"
        );
        assert_eq!(
            legacy.bus.wide_framebuffer, decoded.bus.wide_framebuffer,
            "wide framebuffer at {frame}"
        );
        assert_eq!(legacy.bus.oam, decoded.bus.oam, "OAM at {frame}");
        assert_eq!(
            legacy.bus.audio_buf, decoded.bus.audio_buf,
            "mixed audio at {frame}"
        );
        assert_eq!(legacy.bus.psg_tap, decoded.bus.psg_tap, "PSG at {frame}");
        assert_eq!(legacy.bus.fifo_tap, decoded.bus.fifo_tap, "FIFO at {frame}");
        assert_eq!(legacy.bus.hle_tap.len(), decoded.bus.hle_tap.len());
        assert!(
            legacy
                .bus
                .hle_tap
                .iter()
                .zip(&decoded.bus.hle_tap)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "HLE bits at {frame}"
        );
        assert!(
            legacy.save_state().unwrap() == decoded.save_state().unwrap(),
            "full machine state at {frame}"
        );
        for (total, len) in audio.iter_mut().zip([
            decoded.bus.audio_buf.len(),
            decoded.bus.psg_tap.len(),
            decoded.bus.fifo_tap[0].len(),
            decoded.bus.fifo_tap[1].len(),
            decoded.bus.hle_tap.len(),
        ]) {
            *total += len as u64;
        }
        for machine in [&mut legacy, &mut decoded] {
            machine.bus.audio_buf.clear();
            machine.bus.psg_tap.clear();
            machine.bus.hle_tap.clear();
            machine.bus.fifo_tap.iter_mut().for_each(Vec::clear);
        }
    }
    let resources = decoded.bus.rom_resources().unwrap();
    let stats = resources.stats();
    let bg = resources.background_stats().unwrap();
    let palette = resources.palette_stats().unwrap();
    assert!(
        palette.lz77_calls > 0 && palette.decoded_bytes > 0,
        "new boot must consume decoded palette banks"
    );
    eprintln!(
        "PALETTE_BOOT lz77_calls={} decoded_bytes={} bus_reads={}",
        palette.lz77_calls, palette.decoded_bytes, palette.bus_reads
    );
    assert!(
        bg.bus_reads > 0,
        "new boot criterion must exercise BG metadata"
    );
    assert!(dispatch.0 > 0, "native code must actually execute");
    // In-view boot sprites need no guest width reads: the original draw code
    // fetches dimensions conditionally for clipping/flips. Guest-use coverage
    // is forced by the separate original regional instruction matrix, rather
    // than attributing reads to this boot replay that never occur here.
    if gba_core::mem::extra() > 0 {
        assert!(
            stats.host_rom_reads + stats.host_ram_reads > 0,
            "typed sprite consumers must execute"
        );
    }
    eprintln!(
        "BG_BOOT bg_reads={} projected_bytes={}",
        bg.bus_reads, bg.projected_bytes
    );
    eprintln!("BG_BOOT frames=1200 native={} fallback={} audio={audio:?} full_states=1200 bus_reads={} host_rom_reads={} host_ram_reads={} dynamic_ram_reads={} bus_source_mask={}",
        dispatch.0, dispatch.1, stats.bus_reads, stats.host_rom_reads, stats.host_ram_reads, stats.dynamic_ram_reads, stats.bus_source_mask);
}
