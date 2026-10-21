//! Private-ROM producer discovery. Output is local evidence, not an asset catalog.
use super::*;
use crate::{make_machine, FRAME_STEP_GUARD};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    frame: u64,
    clock: u64,
    pc: u32,
    lr: u32,
    swi: u32,
    source: u32,
    destination: u32,
    argument2: u32,
    source_header: Vec<u8>,
    output_file: String,
    output_bytes: usize,
    output_sha256: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Trace {
    source_rom_sha256: String,
    frames: u64,
    calls: Vec<Call>,
}

#[derive(serde::Serialize)]
struct VerifiedSource {
    offset: usize,
    consumed: usize,
    encoded_sha256: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[test]
#[ignore = "requires exact private ROM and existing producer trace/output; no boot replay"]
fn decode_observed_lz77_matches_guest_outputs() {
    let root =
        PathBuf::from(std::env::var_os("FFTA_STATE_TEST_TRACE_ROOT").expect("existing trace root"));
    let report = root.join("lz77-verified.toml");
    assert!(
        !report.exists(),
        "preserve successful verification evidence"
    );
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
    let trace: Trace =
        toml::from_str(&fs::read_to_string(root.join("calls.toml")).unwrap()).unwrap();
    let hash = gba_core::ffta_rom::digest(&rom);
    assert!(matches!(
        hash.as_str(),
        gba_core::ffta_rom::SHA256 | gba_core::ffta_rom::EUROPE_SHA256
    ));
    assert_eq!(trace.source_rom_sha256, hash);
    let mut sources = Vec::new();
    for (index, call) in trace.calls.iter().enumerate() {
        if !matches!(call.swi, 0x11 | 0x12) {
            continue;
        }
        assert!((0x08..=0x0d).contains(&(call.source >> 24)));
        let offset = call.source as usize & 0x1ff_ffff;
        let encoded = &rom[offset..];
        assert_eq!(&encoded[..4], call.source_header);
        let decoded = gba_core::rom_data::lz77::decode(encoded, 2 * 1024 * 1024).unwrap();
        assert_eq!(call.output_file, format!("call-{index:04}.bin"));
        let observed = fs::read(root.join(&call.output_file)).unwrap();
        assert_eq!(observed.len(), call.output_bytes);
        assert_eq!(gba_core::ffta_rom::digest(&observed), call.output_sha256);
        assert_eq!(decoded.bytes, observed);
        let source = VerifiedSource {
            offset,
            consumed: decoded.consumed,
            encoded_sha256: gba_core::ffta_rom::digest(&encoded[..decoded.consumed]),
            decoded_bytes: decoded.bytes.len(),
            decoded_sha256: call.output_sha256.clone(),
        };
        eprintln!(
            "LZ77_VERIFIED offset={offset:08x} consumed={} decoded_bytes={} sha256={}",
            source.consumed, source.decoded_bytes, source.decoded_sha256
        );
        sources.push(source);
    }
    assert!(!sources.is_empty(), "no observed ROM LZ77 calls verified");
    #[derive(serde::Serialize)]
    struct Report {
        source_rom_sha256: String,
        sources: Vec<VerifiedSource>,
    }
    fs::write(
        report,
        toml::to_string_pretty(&Report {
            source_rom_sha256: hash,
            sources,
        })
        .unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires exact private ROM and absent FFTA_STATE_TEST_TRACE_ROOT; observes BIOS graphics producers"]
fn observe_boot_graphics_decompression() {
    crate::configure_headless_widescreen();
    let root =
        PathBuf::from(std::env::var_os("FFTA_STATE_TEST_TRACE_ROOT").expect("new trace root"));
    assert!(!root.exists(), "preserve previous discovery evidence");
    fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
    let source_rom_sha256 = gba_core::ffta_rom::digest(&rom);
    assert!(matches!(
        source_rom_sha256.as_str(),
        gba_core::ffta_rom::SHA256 | gba_core::ffta_rom::EUROPE_SHA256
    ));
    let frames: u64 = std::env::var("FFTA_STATE_TEST_FRAMES")
        .expect("explicit trace frame count")
        .parse()
        .unwrap();
    assert!((1..=12000).contains(&frames));
    let mut machine = make_machine(rom, None);
    let mut calls = Vec::new();
    for frame in 0..frames {
        machine.bus.keys = crate::demo_keys(frame);
        machine.bus.frame_ready = false;
        for _ in 0..FRAME_STEP_GUARD {
            let args = [
                machine.cpu.regs[0],
                machine.cpu.regs[1],
                machine.cpu.regs[2],
            ];
            let lr = machine.cpu.regs[14];
            let clock = machine.bus.clock;
            if let gba_core::StepEvent::Instr(instruction) = machine.step() {
                if let armv4t::Op::Swi { imm } = instruction.op {
                    let swi = if instruction.thumb { imm } else { imm >> 16 };
                    if (0x10..=0x15).contains(&swi) {
                        // Observations are unconditional instructions that really
                        // executed, never a PC guess before IRQ dispatch.
                        assert_eq!(instruction.cond, armv4t::Cond::Al);
                        assert!(calls.len() < 4096, "bound discovery output");
                        let source_header = if (0x08..=0x0d).contains(&(args[0] >> 24)) {
                            machine
                                .bus
                                .rom
                                .get((args[0] as usize & 0x1ff_ffff)..)
                                .and_then(|b| b.get(..4))
                                .unwrap()
                                .to_vec()
                        } else {
                            Vec::new()
                        };
                        let mut output_file = String::new();
                        let mut output_bytes = 0;
                        let mut output_sha256 = String::new();
                        if swi != 0x10 && source_header.len() == 4 {
                            let size =
                                u32::from_le_bytes(source_header[..].try_into().unwrap()) >> 8;
                            assert!((1..=2 * 1024 * 1024).contains(&size));
                            let (memory, offset) = match args[1] >> 24 {
                                0x02 => (&machine.bus.ewram, args[1] as usize & 0x3ffff),
                                0x03 => (&machine.bus.iwram, args[1] as usize & 0x7fff),
                                0x05 => (&machine.bus.palette, args[1] as usize & 0x3ff),
                                0x06 => {
                                    let offset = args[1] as usize & 0x1ffff;
                                    (
                                        &machine.bus.vram,
                                        if offset >= 0x18000 {
                                            offset - 0x8000
                                        } else {
                                            offset
                                        },
                                    )
                                }
                                _ => panic!("unhandled decompression destination {:08x}", args[1]),
                            };
                            let output = memory
                                .get(offset..offset + size as usize)
                                .expect("contiguous observed output extent");
                            output_file = format!("call-{:04}.bin", calls.len());
                            fs::write(root.join(&output_file), output).unwrap();
                            output_bytes = output.len();
                            output_sha256 = gba_core::ffta_rom::digest(output);
                        }
                        calls.push(Call {
                            frame,
                            clock,
                            pc: instruction.addr,
                            lr,
                            swi,
                            source: args[0],
                            destination: args[1],
                            argument2: args[2],
                            source_header,
                            output_file,
                            output_bytes,
                            output_sha256,
                        });
                    }
                }
            }
            if machine.bus.frame_ready {
                break;
            }
        }
        assert_eq!(machine.bus.frames, frame + 1, "incomplete trace frame");
        machine.bus.audio_buf.clear();
        machine.bus.hle_tap.clear();
        machine.bus.psg_tap.clear();
        machine.bus.fifo_tap.iter_mut().for_each(Vec::clear);
        if frame % 500 == 499 {
            eprintln!("GRAPHICS_TRACE frames={} calls={}", frame + 1, calls.len());
        }
    }
    eprintln!(
        "GRAPHICS_TRACE complete frames={frames} calls={}",
        calls.len()
    );
    fs::write(
        root.join("calls.toml"),
        toml::to_string_pretty(&Trace {
            source_rom_sha256,
            frames,
            calls,
        })
        .unwrap(),
    )
    .unwrap();
}
