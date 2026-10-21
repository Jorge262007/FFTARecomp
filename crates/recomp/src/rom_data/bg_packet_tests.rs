//! Original-instruction oracles for offline background packet interpretation.
use super::*;
use gba_core::{
    bus::Bus,
    rom_data::bg_packets::{self, Limits, Patches},
};

const SOURCE: u32 = 0x0201_4000;
const RETURN: u32 = 0x0800_0300;
const STACK: u32 = 0x0300_7c00;
const LIMITS: Limits = Limits {
    destination_bytes: 0x4000,
    written_bytes: 0x10000,
    packets: 1024,
};

fn invoke(machine: &mut gba_core::Machine, entry: u32) -> usize {
    machine.cpu.regs[13] = STACK;
    machine.cpu.regs[14] = RETURN | 1;
    machine.cpu.regs[15] = entry;
    machine.cpu.cpsr = 0xbf; // System, Thumb, IRQ masked; component invocation.
    for steps in 1..=1_000_000 {
        machine.step();
        if machine.cpu.regs[15] == RETURN {
            return steps;
        }
    }
    panic!("original packet routine did not return");
}

fn write_halfwords(expected: &mut [u8], base: usize, patches: &Patches<u16>) {
    for patch in &patches.packets {
        for (index, value) in patch.values.iter().enumerate() {
            let at = base + patch.byte_offset as usize + index * 2;
            expected[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
}

fn assert_ewram(actual: &[u8], expected: &[u8]) {
    assert_eq!(actual.len(), expected.len());
    let differences: Vec<_> = actual
        .iter()
        .zip(expected)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .take(8)
        .map(|(offset, (a, b))| (format!("{:08x}", 0x0200_0000 + offset), *a, *b))
        .collect();
    assert!(
        differences.is_empty(),
        "EWRAM first differences (address,actual,expected): {differences:?}"
    );
}

fn check_planes(rom: &[u8], input: &[u8], entry: u32, budget: u32) -> (usize, usize) {
    let decoded = bg_packets::decode_planes(input, LIMITS).unwrap();
    let mut machine = crate::make_machine(rom.to_vec(), None);
    machine.bus.ewram[0x91a0..0xd1a0].fill(0x5a);
    machine.bus.ewram[0x14000..0x14000 + input.len()].copy_from_slice(input);
    let mut expected = machine.bus.ewram.clone();
    let mut writes = 0;
    for patch in &decoded.calculated_plane_writes().unwrap().packets {
        for (index, value) in patch.values.iter().enumerate() {
            let at = 0x91a0 + patch.byte_offset as usize + index * 4;
            expected[at..at + 4].copy_from_slice(&value.to_le_bytes());
            writes += 1;
        }
    }
    let mut cursor = SOURCE + 4;
    let mut calls = 0;
    let mut steps = 0;
    loop {
        assert!(calls < 1024, "original chunk writer did not finish");
        machine.cpu.regs[0] = cursor;
        machine.cpu.regs[1] = decoded
            .dictionary_offset
            .map_or(0, |offset| SOURCE + offset as u32);
        machine.cpu.regs[2] = (decoded.header >> 31) | ((decoded.header >> 29) & 2);
        machine.cpu.regs[3] = 0; // Full calculated planes; no cropped VRAM view.
        for (offset, value) in [(0, 0), (4, 0), (8, 0), (12, budget)] {
            machine.bus.write32(STACK + offset, value);
        }
        steps += invoke(&mut machine, entry);
        calls += 1;
        cursor = machine.cpu.regs[0];
        if cursor == 0 {
            break;
        }
        assert!((SOURCE + 4..SOURCE + input.len() as u32).contains(&cursor));
    }
    assert_ewram(&machine.bus.ewram, &expected);
    assert!(writes > 0);
    (steps, calls)
}

#[test]
#[ignore = "requires exact private ROM and prior graphics trace; invokes original packet code without boot replay"]
fn decoded_packets_match_original_regional_writers() {
    crate::configure_headless_widescreen();
    let root =
        PathBuf::from(std::env::var_os("FFTA_STATE_TEST_TRACE_ROOT").expect("existing trace"));
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
    let hash = gba_core::ffta_rom::digest(&rom);
    // Independently read original regional calls/literals; no guest code in fixtures.
    let (plane, halfwords, staged) = match hash.as_str() {
        gba_core::ffta_rom::SHA256 => (0x0801_ed24, 0x0801_f098, 0x0801_efb0),
        gba_core::ffta_rom::EUROPE_SHA256 => (0x0801_fe94, 0x0802_0208, 0x0802_0120),
        _ => panic!("exact supported image required"),
    };
    let trace: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("calls.toml")).unwrap()).unwrap();
    assert_eq!(trace["source_rom_sha256"].as_str().unwrap(), hash);
    let calls = trace["calls"].as_array().unwrap();
    let mut instructions = 0;
    let mut invocations = 0;
    for (index, size, base) in [(1, 0x200, 0x7cb0), (2, 0x2000, 0xd1a0)] {
        let input = fs::read(root.join(format!("call-{index:04}.bin"))).unwrap();
        assert_eq!(
            gba_core::ffta_rom::digest(&input),
            calls[index]["output_sha256"].as_str().unwrap()
        );
        let decoded = bg_packets::decode_halfwords(
            &input,
            None,
            Limits {
                destination_bytes: size,
                ..LIMITS
            },
        )
        .unwrap();
        let mut machine = crate::make_machine(rom.clone(), None);
        machine.bus.ewram[base..base + size].fill(0x5a);
        machine.bus.ewram[0x14000..0x14000 + input.len()].copy_from_slice(&input);
        if index == 2 {
            machine.bus.write32(0x0200_80ac, SOURCE);
            machine.bus.write32(0x0200_80b4, 0);
        }
        let mut expected = machine.bus.ewram.clone();
        write_halfwords(&mut expected, base, &decoded);
        if index == 2 {
            expected[0x80ac..0x80b0].fill(0);
        }
        machine.cpu.regs[0] = SOURCE;
        machine.cpu.regs[1] = 0;
        instructions += invoke(&mut machine, if index == 1 { halfwords } else { staged });
        invocations += 1;
        assert_ewram(&machine.bus.ewram, &expected);
        eprintln!(
            "BG_PACKETS halfwords payload={index} runs={} values={} consumed={} bytes={}",
            decoded.packets.len(),
            decoded
                .packets
                .iter()
                .map(|p| p.values.len())
                .sum::<usize>(),
            decoded.consumed,
            input.len()
        );
    }
    let input = fs::read(root.join("call-0003.bin")).unwrap();
    assert_eq!(
        gba_core::ffta_rom::digest(&input),
        calls[3]["output_sha256"].as_str().unwrap()
    );
    for budget in [0, 2048] {
        let (steps, count) = check_planes(&rom, &input, plane, budget);
        instructions += steps;
        invocations += count;
        eprintln!("BG_PACKETS planes budget={budget} invocations={count} steps={steps}");
    }
    // Check the narrow dictionary branch independently of the captured wide one.
    let mut narrow = vec![0x10, 0, 0, 0x80, 0, 0, 2, 0, 1, 0, 0];
    narrow.resize(16, 0);
    narrow.extend_from_slice(&[0x12, 0, 0x34, 0x92, 0xcd, 0xab]);
    let (steps, count) = check_planes(&rom, &narrow, plane, 0);
    instructions += steps;
    invocations += count;
    // Preserve the contradictory literal probe: the isolated original routine
    // writes these words to region0, not to the calculated-plane base.
    instructions += check_literal_destination(&rom, plane);
    invocations += 1;
    eprintln!("BG_PACKETS complete region={hash} original_invocations={invocations} instructions={instructions} all_ewram=true indexed_planes=true literal_destination_region0=true source_trace_reused=true");
}

fn check_literal_destination(rom: &[u8], entry: u32) -> usize {
    struct Writes<'a> {
        memory: &'a mut gba_core::mem::MemMap,
        region0: Vec<(u32, u32)>,
    }
    impl Bus for Writes<'_> {
        fn read8(&mut self, address: u32) -> u8 {
            self.memory.read8(address)
        }
        fn read16(&mut self, address: u32) -> u16 {
            self.memory.read16(address)
        }
        fn read32(&mut self, address: u32) -> u32 {
            self.memory.read32(address)
        }
        fn write8(&mut self, address: u32, value: u8) {
            self.memory.write8(address, value);
        }
        fn write16(&mut self, address: u32, value: u16) {
            self.memory.write16(address, value);
        }
        fn write32(&mut self, address: u32, value: u32) {
            if address < 0x0200_0000 {
                self.region0.push((address, value));
            }
            self.memory.write32(address, value);
        }
    }
    let input = [0, 0, 0, 0, 0, 0, 4, 0, 0x34, 0x12, 0xcd, 0xab, 0, 0];
    let decoded = bg_packets::decode_planes(&input, LIMITS).unwrap();
    assert!(decoded.calculated_plane_writes().is_err());
    let value = decoded.payload.packets[0].values[0];
    let mut machine = crate::make_machine(rom.to_vec(), None);
    machine.bus.ewram[0x91a0..0xd1a0].fill(0x5a);
    machine.bus.ewram[0x14000..0x14000 + input.len()].copy_from_slice(&input);
    let expected = machine.bus.ewram.clone();
    machine.cpu.regs[0] = SOURCE + 4;
    machine.cpu.regs[1] = 0;
    machine.cpu.regs[2] = 0;
    machine.cpu.regs[3] = 0;
    machine.cpu.regs[13] = STACK;
    machine.cpu.regs[14] = RETURN | 1;
    machine.cpu.regs[15] = entry;
    machine.cpu.cpsr = 0xbf;
    let mut bus = Writes {
        memory: &mut machine.bus,
        region0: Vec::new(),
    };
    for steps in 1..1000 {
        gba_core::exec::step_hle(&mut machine.cpu, &mut bus);
        if machine.cpu.regs[15] == RETURN {
            assert_eq!(bus.region0, [(0, value)]);
            assert_ewram(&bus.memory.ewram, &expected);
            return steps;
        }
    }
    panic!("literal destination probe did not return");
}
