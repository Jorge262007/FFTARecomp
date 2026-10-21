//! Private-ROM/native differential gate for the decoded base resource path.
use super::*;
use crate::{
    arm_audio_hle, demo_keys, load_native, make_machine, run_frame_native, FRAME_STEP_GUARD,
};

#[test]
#[ignore = "requires FFTA_STATE_TEST_ROM and FFTA_STATE_TEST_NATIVE local fixtures"]
fn decoded_base_matches_legacy_native_states_images_audio_and_dispatch() {
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
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("decoded");
    let mut source = Extraction::from_rom(&rom).unwrap();
    extract(&directory, &mut source).unwrap();
    attach_from_directory(&mut decoded, &directory).unwrap();
    assert_eq!(legacy.save_state().unwrap(), decoded.save_state().unwrap());
    let mut dispatch = (0u64, 0u64);
    let mut audio = [0u64; 5];
    for frame in 0..600 {
        let mut counts = [(0, 0); 2];
        for (index, machine) in [&mut legacy, &mut decoded].into_iter().enumerate() {
            machine.bus.keys = demo_keys(machine.bus.frames);
            let pointer = machine as *mut gba_core::Machine as *mut std::ffi::c_void;
            counts[index] = run_frame_native(machine, &table, pointer, FRAME_STEP_GUARD);
            assert!(machine.bus.frame_ready);
        }
        assert_eq!(counts[0], counts[1], "dispatch at frame {frame}");
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
    eprintln!("ROM_DATA_NATIVE frames=600 native={} fallback={} audio={audio:?} full_states=600 bus_reads={} host_rom_reads={} host_ram_reads={} dynamic_ram_reads={} bus_source_mask={}",
        dispatch.0, dispatch.1, stats.bus_reads, stats.host_rom_reads, stats.host_ram_reads, stats.dynamic_ram_reads, stats.bus_source_mask);
}
