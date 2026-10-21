//! Opt-in measurements and complete recorded-scene differential checks.
//! Fixtures and machine-specific results remain outside the repository.
use super::*;
use crate::{
    arm_audio_hle, load_native, make_machine, run_frame_native, InputScript, FRAME_STEP_GUARD,
};
use std::time::Instant;

#[derive(serde::Serialize)]
struct StartupObservation {
    cached: bool,
    source_io_ms: f64,
    source_decode_ms: f64,
    preparation_ms: f64,
    activation_ms: f64,
    total_after_source_read_ms: f64,
    resident_bytes: usize,
    generation: String,
}

#[test]
#[ignore = "requires a private ROM and a new FFTA_STATE_TEST_COST_ROOT output directory"]
fn measure_decoded_startup_costs() {
    let rom_path = std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture");
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_COST_ROOT").expect("cost output"));
    assert!(
        !root.exists(),
        "preserve earlier measurements; select a new output root"
    );
    fs::create_dir_all(&root).unwrap();
    let mut observations = Vec::new();
    for index in 0..7 {
        let dataset_root = root.join(format!("sample-{index}"));
        for cached in [false, true] {
            let io_start = Instant::now();
            let rom = fs::read(&rom_path).unwrap();
            let source_io_ms = io_start.elapsed().as_secs_f64() * 1000.0;
            // ROM allocation and machine construction belong to both paths.
            let mut machine = make_machine(rom, None);
            let start = Instant::now();
            let mut source = Extraction::from_rom(&machine.bus.rom).unwrap();
            let decoded_at = start.elapsed();
            let prepared =
                preparation::prepare(&dataset_root, &mut source, &mut |_| Ok(())).unwrap();
            let prepared_at = start.elapsed();
            assert_eq!(prepared.action, if cached { "reused" } else { "created" });
            attach_from_directory(&mut machine, &prepared.directory).unwrap();
            let activated_at = start.elapsed();
            let resources = machine.bus.rom_resources().unwrap();
            observations.push(StartupObservation {
                cached,
                source_io_ms,
                source_decode_ms: decoded_at.as_secs_f64() * 1000.0,
                preparation_ms: (prepared_at - decoded_at).as_secs_f64() * 1000.0,
                activation_ms: (activated_at - prepared_at).as_secs_f64() * 1000.0,
                total_after_source_read_ms: activated_at.as_secs_f64() * 1000.0,
                resident_bytes: resources.resident_bytes() + 2 * std::mem::size_of::<usize>(),
                generation: prepared
                    .directory
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            });
        }
    }
    #[derive(serde::Serialize)]
    struct Report {
        observations: Vec<StartupObservation>,
    }
    // Save every observation, including slower results; evaluate budgets separately.
    fs::write(
        root.join("startup.toml"),
        toml::to_string_pretty(&Report { observations }).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private ROM/native/input fixtures and explicit scene length; optional state/save"]
fn decoded_base_matches_recorded_scene_at_every_frame() {
    crate::configure_headless_widescreen();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
    let library = std::env::var_os("FFTA_STATE_TEST_NATIVE").expect("native fixture");
    let state = std::env::var_os("FFTA_STATE_TEST_STATE");
    let save = std::env::var_os("FFTA_STATE_TEST_SAVE");
    assert!(
        !(state.is_some() && save.is_some()),
        "choose a snapshot or cartridge save"
    );
    let input_path = std::env::var("FFTA_STATE_TEST_INPUT").expect("input fixture");
    let frames: u64 = std::env::var("FFTA_STATE_TEST_FRAMES")
        .expect("explicit frame count")
        .parse()
        .unwrap();
    assert!(frames > 0);
    let input = InputScript::load(&input_path).unwrap();
    let (_library, table) = load_native(Path::new(&library)).unwrap();
    let mut legacy = make_machine(rom.clone(), None);
    let mut decoded = make_machine(rom.clone(), None);
    let directory = tempfile::tempdir().unwrap();
    let mut source = Extraction::from_rom(&rom).unwrap();
    let dataset = match std::env::var_os("FFTA_STATE_TEST_DATASET") {
        Some(path) => PathBuf::from(path),
        None => {
            let dataset = directory.path().join("decoded");
            extract(&dataset, &mut source).unwrap();
            dataset
        }
    };
    attach_from_directory(&mut decoded, &dataset).unwrap();
    for machine in [&mut legacy, &mut decoded] {
        if let Some(path) = &state {
            crate::save_state::load(machine, Path::new(path)).unwrap();
        }
        if let Some(path) = &save {
            machine.bus.load_save_data(&fs::read(path).unwrap());
        }
    }
    // Compare the unmodified snapshot before configuring audio observation.
    assert_eq!(legacy.save_state().unwrap(), decoded.save_state().unwrap());
    for machine in [&mut legacy, &mut decoded] {
        // Loading a snapshot can overwrite these host shadows. Arm identically
        // AFTER loading so a zero-length HLE tap cannot masquerade as coverage.
        arm_audio_hle(machine, None);
        machine.bus.tap_channels = true;
        clear_audio(machine);
    }
    assert_eq!(legacy.save_state().unwrap(), decoded.save_state().unwrap());
    let first = legacy.bus.frames;
    let mut dispatch = [0u64; 2];
    let mut audio = [0u64; 5];
    let mut source_mask = 0;
    for relative in 0..frames {
        let mut counts = [(0, 0); 2];
        for (index, machine) in [&mut legacy, &mut decoded].into_iter().enumerate() {
            machine.bus.keys = input.keys_at(relative);
            input.apply_mouse(&mut machine.bus, relative).unwrap();
            let pointer = machine as *mut gba_core::Machine as *mut std::ffi::c_void;
            counts[index] = run_frame_native(machine, &table, pointer, FRAME_STEP_GUARD);
            assert_eq!(machine.bus.frames, first + relative + 1, "incomplete frame");
            assert!(machine.bus.frame_ready);
        }
        assert_eq!(counts[0], counts[1], "dispatch at {relative}");
        dispatch[0] += counts[0].0;
        dispatch[1] += counts[0].1;
        assert_eq!(
            legacy.bus.framebuffer, decoded.bus.framebuffer,
            "central pixels at {relative}"
        );
        assert_eq!(
            legacy.bus.wide_framebuffer, decoded.bus.wide_framebuffer,
            "wide pixels at {relative}"
        );
        assert_eq!(legacy.bus.oam, decoded.bus.oam, "OAM at {relative}");
        assert_eq!(
            legacy.bus.audio_buf, decoded.bus.audio_buf,
            "mixed audio at {relative}"
        );
        assert_eq!(legacy.bus.psg_tap, decoded.bus.psg_tap, "PSG at {relative}");
        assert_eq!(
            legacy.bus.fifo_tap, decoded.bus.fifo_tap,
            "FIFO at {relative}"
        );
        assert_eq!(legacy.bus.hle_tap.len(), decoded.bus.hle_tap.len());
        assert!(
            legacy
                .bus
                .hle_tap
                .iter()
                .zip(&decoded.bus.hle_tap)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "HLE at {relative}"
        );
        assert!(
            legacy.save_state().unwrap() == decoded.save_state().unwrap(),
            "full state at {relative}"
        );
        for (sum, len) in audio.iter_mut().zip([
            decoded.bus.audio_buf.len(),
            decoded.bus.psg_tap.len(),
            decoded.bus.fifo_tap[0].len(),
            decoded.bus.fifo_tap[1].len(),
            decoded.bus.hle_tap.len(),
        ]) {
            *sum += len as u64;
        }
        let mask = decoded.bus.rom_resources().unwrap().stats().bus_source_mask;
        if mask != source_mask {
            eprintln!(
                "ROM_DATA_SOURCE first_frame={} clock={} mask={mask}",
                decoded.bus.frames, decoded.bus.clock
            );
            source_mask = mask;
        }
        if relative % 500 == 499 {
            eprintln!("ROM_DATA_SCENE_PROGRESS frames={}/{}", relative + 1, frames);
        }
        clear_audio(&mut legacy);
        clear_audio(&mut decoded);
    }
    assert!(dispatch[0] > 0, "native dispatch must execute");
    assert!(
        audio.iter().all(|&count| count > 0),
        "all audio streams need actual samples: {audio:?}"
    );
    let stats = decoded.bus.rom_resources().unwrap().stats();
    let bg = decoded
        .bus
        .rom_resources()
        .unwrap()
        .background_stats()
        .unwrap();
    if std::env::var_os("FFTA_STATE_TEST_REQUIRE_BG").is_some() {
        assert!(
            bg.bus_reads > 0,
            "this scene must exercise BG metadata reads"
        );
    }
    let palette = decoded
        .bus
        .rom_resources()
        .unwrap()
        .palette_stats()
        .unwrap();
    if std::env::var_os("FFTA_STATE_TEST_REQUIRE_PALETTE").is_some() {
        assert!(
            palette.lz77_calls > 0 && palette.decoded_bytes > 0,
            "this scene must consume decoded palette banks"
        );
    }
    eprintln!("ROM_DATA_PALETTE_SCENE frames={frames} bus_reads={} source_bytes={} lz77_calls={} decoded_bytes={}", palette.bus_reads, palette.source_bytes, palette.lz77_calls, palette.decoded_bytes);
    if let Ok(mask) = std::env::var("FFTA_STATE_TEST_REQUIRE_SOURCE_MASK") {
        let required: u8 = mask.parse().unwrap();
        assert_eq!(
            stats.bus_source_mask & required,
            required,
            "required source ranges did not execute"
        );
    }
    assert!(
        stats.host_rom_reads + stats.host_ram_reads > 0,
        "typed consumers must execute"
    );
    eprintln!("ROM_DATA_SCENE frames={frames} first={first} native={} fallback={} full_states={frames} audio={audio:?} bus_reads={} host_rom_reads={} host_ram_reads={} dynamic_ram_reads={} bus_source_mask={}", dispatch[0], dispatch[1], stats.bus_reads, stats.host_rom_reads, stats.host_ram_reads, stats.dynamic_ram_reads, stats.bus_source_mask);
    eprintln!(
        "ROM_DATA_BG_SCENE frames={frames} bus_reads={} projected_bytes={}",
        bg.bus_reads, bg.projected_bytes
    );
}

fn clear_audio(machine: &mut gba_core::Machine) {
    machine.bus.audio_buf.clear();
    machine.bus.psg_tap.clear();
    machine.bus.hle_tap.clear();
    machine.bus.fifo_tap.iter_mut().for_each(Vec::clear);
}

#[test]
#[ignore = "requires the exact private USA ROM; follows clean-boot demo input to the original copier"]
fn usa_copied_obj_initializer_from_clean_boot_uses_decoded_source() {
    crate::configure_headless_widescreen();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("USA ROM fixture")).unwrap();
    assert_eq!(gba_core::ffta_rom::digest(&rom), gba_core::ffta_rom::SHA256);
    // Independently located USA literals and function entry. No EUR offset
    // arithmetic or copied guest instructions are used to construct this test.
    const ENTRY: u32 = 0x0808_f74c;
    const START: usize = 0xa38adc;
    const END: usize = 0xa3991c;
    const ALIAS: usize = 0xa398dc;
    const DEST: usize = 0x5f28;
    let word = |offset| u32::from_le_bytes(rom[offset..offset + 4].try_into().unwrap());
    assert_eq!(word(0x8f764), 0x0800_0000 + END as u32);
    assert_eq!(word(0x8f768), 0x0800_0000 + START as u32);
    assert_eq!(word(0x8f76c), 0x0300_0000 + DEST as u32);
    assert_eq!(DEST + ALIAS - START, 0x6d28);

    let mut source = Extraction::from_rom(&rom).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let dataset = directory.path().join("decoded");
    extract(&dataset, &mut source).unwrap();
    let mut decoded = make_machine(rom.clone(), None);
    attach_from_directory(&mut decoded, &dataset).unwrap();
    let max_frames: u64 = std::env::var("FFTA_STATE_TEST_FRAMES")
        .unwrap_or_else(|_| "12000".into())
        .parse()
        .unwrap();
    assert!((1..=30000).contains(&max_frames));
    for frame in 0..max_frames {
        decoded.bus.keys = crate::demo_keys(frame);
        decoded.bus.frame_ready = false;
        for _ in 0..FRAME_STEP_GUARD {
            if decoded.cpu.regs[15] == ENTRY {
                assert!(decoded.cpu.thumb());
                let checkpoint = decoded.save_state().unwrap();
                let entered_frame = decoded.bus.frames;
                let entered_clock = decoded.bus.clock;
                let return_pc = decoded.cpu.regs[14] & !1;
                let before = decoded.bus.rom_resources().unwrap().stats();
                let mut legacy = make_machine(rom.clone(), None);
                legacy.load_state(&checkpoint).unwrap();
                assert_eq!(legacy.save_state().unwrap(), checkpoint);
                let mut count = 0;
                let mut read_pc = None;
                while decoded.cpu.regs[15] != return_pc || count == 0 {
                    assert!(count < 10000, "original initializer did not return");
                    let pc = decoded.cpu.regs[15];
                    let reads = decoded.bus.rom_resources().unwrap().stats().bus_reads;
                    legacy.step();
                    decoded.step();
                    assert!(
                        legacy.save_state().unwrap() == decoded.save_state().unwrap(),
                        "initializer full state at step {count}/PC {pc:08x}"
                    );
                    if decoded.bus.rom_resources().unwrap().stats().bus_reads > reads {
                        assert!(
                            read_pc.replace(pc).is_none(),
                            "unexpected multiple copy instructions"
                        );
                    }
                    count += 1;
                }
                assert_eq!(
                    &decoded.bus.iwram[DEST..DEST + END - START],
                    &rom[START..END]
                );
                assert_eq!(
                    &decoded.bus.iwram[0x6d28..0x6d58],
                    &source.dimensions.source_bytes()
                );
                let after = decoded.bus.rom_resources().unwrap().stats();
                assert_eq!(after.bus_reads - before.bus_reads, 24);
                assert_eq!(after.bus_source_mask & 2, 2);
                let read_pc = read_pc.expect("original copier must read the file-backed alias");

                // A separate provenance probe poisons only the old raw table
                // AFTER authenticated activation. Original code, other source
                // bytes and the caller's real checkpoint are left intact.
                let mut file_only = make_machine(rom.clone(), None);
                attach_from_directory(&mut file_only, &dataset).unwrap();
                file_only.load_state(&checkpoint).unwrap();
                file_only.bus.rom[ALIAS..ALIAS + 48].fill(0xa5);
                for _ in 0..count {
                    file_only.step();
                }
                assert_eq!(
                    &file_only.bus.iwram[DEST..DEST + END - START],
                    &rom[START..END]
                );
                assert_eq!(file_only.bus.rom_resources().unwrap().stats().bus_reads, 24);
                assert_eq!(file_only.cpu.regs[15], return_pc);
                // Restore the test poison only after the guest returned, so
                // snapshot ROM identity can be compared without masking state.
                file_only.bus.rom[ALIAS..ALIAS + 48].copy_from_slice(&rom[ALIAS..ALIAS + 48]);
                assert!(file_only.save_state().unwrap() == legacy.save_state().unwrap());
                eprintln!("ROM_DATA_USA_INITIALIZER frame={entered_frame} clock={entered_clock} entry={ENTRY:08x} return={return_pc:08x} read_pc={read_pc:08x} steps={count} copied_bytes={} table_bytes=48 decoded_reads=24 poisoned_rom_probe=true full_states={count}", END-START);
                return;
            }
            decoded.step();
            if decoded.bus.frame_ready {
                break;
            }
        }
        assert_eq!(decoded.bus.frames, frame + 1, "incomplete boot frame");
        clear_audio(&mut decoded);
        if frame % 500 == 499 {
            eprintln!(
                "ROM_DATA_USA_BOOT frames={} pc={:08x}",
                decoded.bus.frames, decoded.cpu.regs[15]
            );
        }
    }
    panic!("original USA initializer was not reached in {max_frames} clean-boot frames");
}
