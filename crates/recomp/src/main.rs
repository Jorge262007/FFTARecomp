//! Static recompiler CLI.
//!
//! Current commands exercise the decoder against real ROMs:
//!   dis <rom> [--addr HEX] [--count N] [--thumb]   disassemble from an address
//!   entry-scan <dir>                               decode every ROM's entry point

use std::path::Path;
use std::process::ExitCode;

use recomp_core::{analyze, build_library, labels, Compiler, FileLabels, LabelSource};

mod ff_audio;
mod fast_audio_rate;
mod audio_profile;
mod profile_window;
mod fps;
mod fullscreen;
mod ui_pixels;
mod perf;
mod packfile;
mod play;
mod rom_switch;
mod rom_data;
mod save_path;
mod save_state;
mod save_backups;
#[cfg(any(target_os = "linux", test))]
mod linux_pad;

use armv4t::{decode_arm, decode_thumb, Op};
use gba_core::{is_self_loop, Machine, StepEvent};

const ROM_BASE: u32 = 0x0800_0000;

/// Native shared-library extension on the host platform. The translation
/// pipeline links with the system `cc -shared`, which produces a PE DLL
/// on Windows, an ELF .so on Linux, and a Mach-O dylib on macOS — name
/// the artifact accordingly so loaders and humans agree on what it is.
const LIB_EXT: &str = if cfg!(windows) {
    "dll"
} else if cfg!(target_os = "macos") {
    "dylib"
} else {
    "so"
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Version probe (the launcher uses this as a stale-binary handshake).
    if args.first().map(String::as_str) == Some("--version") {
        println!("recomp {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    // `recomp help [cmd]`, `recomp <cmd> --help`, and bare `--help`/`-h`
    // all land in help; the first non-help token names the topic.
    if args.iter().any(|a| a == "--help" || a == "-h")
        || args.first().map(String::as_str) == Some("help")
    {
        let topic = args
            .iter()
            .find(|a| *a != "--help" && *a != "-h" && *a != "help")
            .map(String::as_str);
        print_help(topic);
        return ExitCode::SUCCESS;
    }
    let result = match args.first().map(String::as_str) {
        Some("dis") => cmd_dis(&args[1..]),
        Some("controllers") => {
            if let Some(g) = build_gilrs() {
                for (_, pad) in g.gamepads() {
                    println!("gilrs: {}", pad.name());
                }
            }
            if let Some(mut d) = input_config::directinput::DirectInput::new() {
                for name in d.names() {
                    if args.iter().any(|a| a == "--poll") {
                        match d.poll(&name) {
                            Some(pad) => println!(
                                "{name} polling=OK pressed_buttons={} axes={:?} pov={:?}",
                                pad.buttons.iter().filter(|&&v| v).count(),
                                pad.axes,
                                pad.pov
                            ),
                            None => println!("{name} polling=FAILED"),
                        }
                    } else {
                        println!("{name}");
                    }
                }
            }
            Ok(())
        }
        Some("entry-scan") => cmd_entry_scan(&args[1..]),
        Some("run") => cmd_run(&args[1..]),
        Some("frames") => cmd_frames(&args[1..]),
        Some("play") => play::cmd_play(&args[1..]),
        Some("build") => cmd_build(&args[1..]),
        Some("mp2k-scan") => cmd_mp2k_scan(&args[1..]),
        Some("engine-scan") => cmd_engine_scan(&args[1..]),
        Some("rom-data") => rom_data::cmd(&args[1..]),
        Some("runc") => cmd_runc(&args[1..]),
        Some("verify") => cmd_verify(&args[1..]),
        Some("labels") => cmd_labels(&args[1..]),
        // A packaged binary (manifest beside the executable) plays its
        // pinned title when launched bare — double-click behavior.
        None if packfile::load().is_some() => play::cmd_play(&[]),
        _ => {
            print_help(None);
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `(name, synopsis, flag lines, description)` per command — the single
/// source for both the overview listing and per-command help.
const HELP: &[(&str, &str, &[&str], &str)] = &[
    ("rom-data", "recomp rom-data <extract|verify> <rom> <dataset-directory> | prepare <rom> [resources-root]", &[],
     "Extract, verify or prepare source-authenticated decoded OBJ dimensions, BG descriptors and base palettes for exact FFTA EUR/USA images. Managed preparation reuses or repairs versioned application data. Use --rom-data with play/frames/runc to consume verified base data; the source ROM is still required."),
    ("controllers", "recomp controllers [--poll]", &["--poll    sample each DirectInput controller"], "List detected gilrs and Windows DirectInput controllers without loading a ROM."),
    ("build", "recomp build <rom> [--ram] [--bios file] [--labels file] [--gamedb file]", &[
        "--ram    profile a short interpreter run first, translating the RAM-resident \
code and computed-branch targets it discovers (recommended; play's cache builds use it)",
        "--bios FILE    experimental: recompile a real 16 KB BIOS image too (region 0) \
and disable all BIOS HLE; output goes to out/<stem>-bios.<dylib|so|dll>",
        "--labels FILE    union an explicit label map (instead of relying on the \
beside-the-image file); used by packaging so a symlinked or renamed image still finds \
its map",
        "--gamedb FILE    seed from the shipped gamedb.sqlite (function boundaries for the \
image's sha256) instead of label files; mutually exclusive with --labels",
    ], "Translate a ROM image to a native shared library at out/<stem>.<dylib|so|dll> (host platform). \
Emits C11 in bounded chunks and compiles them with cc. Label files (<rom>.labels \
or the recorder's accumulator) contribute extra entry-point seeds automatically."),
    ("play", "recomp play <rom> [--interp] [--stats] [--status] [--record-labels] [--bios file] [--no-bios] [--benchmark config.toml]", &[
        "--rom-data        opt-in verified decoded game data (normal enablement awaits equivalence/cost gates)",
        "--benchmark FILE   developer-only bounded GUI replay (TOML config; no cartridge writes)",
        "--interp           force the interpreter (skip/ignore native translation)",
        "--stats            print performance readouts (frame time, native vs fallback)",
        "--status           emit machine-readable lifecycle lines on stdout (used by the launcher)",
        "--record-labels    record interpreter-fallback entry points; the next translation \
covers them (accumulates across sessions)",
        "--bios FILE    boot a specific real BIOS image (recompiled; no BIOS HLE)",
        "--no-bios      force BIOS HLE even when an image is installed",
    ], "Windowed play. Boots the real BIOS when one is installed ($GBA_RECOMP_BIOS, \
gba_bios.bin next to the executable, or the user config dir — the launcher's first-launch \
setup installs it); BIOS HLE otherwise. First launch translates the image into the \
per-user cache (one-time, progress printed); later launches load it instantly. Reads \
input.cfg/av.cfg from the shared config directory."),
    ("runc", "recomp runc <rom> [--frames N] [--out img.ppm] [--input file] [--record-labels] [--bios file]", &[
        "--rom-data         prepare and consume source-verified decoded base game data",
        "--frames N         frames to run (default 600)",
        "--out PATH         write the final frame as a PPM",
        "--input FILE       replay a recorded input script (see play's RECOMP_RECORD_INPUT)",
        "--sav FILE         preload a save copy; runc never writes it",
        "--state FILE       resume a snapshot read-only, mutually exclusive with --sav",
        "                   frames/input/zoom/warm-up are relative to the loaded frame",
        "--record-labels    record fallback entry points as labels (headless soak runs)",
        "--bios FILE     run a real-BIOS build (loads out/<stem>-bios.<dylib|so|dll>; no BIOS HLE)",
    ], "Run the recompiled output from out/<stem>.<dylib|so|dll> headless (build first); \
prints frame hash and native/fallback counts."),
    ("verify", "recomp verify <rom> [--frames N] [--reuse] [--dump prefix] [--input file] [--bios file]", &[
        "--frames N       frames to compare (default 600)",
        "--reuse          skip the rebuild if the out/<stem> library exists",
        "--dump PREFIX    write both final frames as PREFIX.interp.ppm / PREFIX.recomp.ppm",
        "--input FILE     replay a recorded input script on both sides (instead of demo taps)",
        "--bios FILE      verify with a real recompiled BIOS on both sides (no BIOS HLE)",
    ], "Differential check: run the interpreter and the recompiled output, compare \
frame hashes, print MATCH or MISMATCH (exit code follows)."),
    ("run", "recomp run <rom> [--max-steps N] [--trace] [--hist] [--bios file]", &[
        "--max-steps N    stop after N interpreter steps",
        "--trace          disassemble every executed instruction to stderr",
        "--hist           print a hot-PC histogram at exit",
        "--bios FILE      execute a real 16 KB BIOS image (boot from the reset vector; no HLE)",
    ], "Headless interpreter run; the conformance-suite driver (a pass parks with r12=0)."),
    ("frames", "recomp frames <rom> [--frames N] [--out img.ppm] [--keys MASK] [--demo] [--input file] [--sav file] [--bios file]", &[
        "--rom-data         prepare and consume source-verified decoded base game data",
        "--frames N      frames to run (default 600)",
        "--out PATH      write the final frame as a PPM",
        "--keys MASK     hold a KEYINPUT mask for the whole run (hex)",
        "--demo          deterministic demo input: periodic Start, then A taps",
        "--input FILE    replay a recorded input script (see play's RECOMP_RECORD_INPUT)",
        "--sav FILE      preload a save file",
        "--bios FILE     execute a real 16 KB BIOS image (boot from the reset vector; no HLE)",
    ], "Headless boot to frame N on the interpreter; prints the frame hash plus \
boot diagnostics (DISPCNT/PC/SWI and live disassembly at PC)."),
    ("labels", "recomp labels <show|import|export> <rom> [file]", &[
        "show              sources, entry counts, snapshot status, cache key",
        "import FILE       union a shared label file into this image's accumulator \
(v1 lines or v2 TOML, auto-detected)",
        "export [FILE]     write a shareable label file (default <rom>.labels.toml; \
a non-.toml FILE writes v1 lines)",
    ], "Inspect and exchange label files — entry points discovered at runtime or by \
disassembly tooling that feed translation coverage (see BUILDING.md and docs/labels.md)."),
    ("dis", "recomp dis <rom> [--addr HEX] [--count N] [--thumb]", &[
        "--addr HEX    start address (default ROM base)",
        "--count N     instructions to print (default 16)",
        "--thumb       decode as Thumb",
    ], "Disassemble ROM bytes at an address."),
    ("entry-scan", "recomp entry-scan <dir>", &[],
     "Decode the entry branch of every image in a directory — a corpus sanity check."),
    ("mp2k-scan", "recomp mp2k-scan <rom|dir>", &[],
     "Report MP2K/M4A audio-driver detection (signatures, hook addresses) for one image or a directory."),
    ("engine-scan", "recomp engine-scan <rom|dir>", &[],
     "Classify the audio engine (MP2K, GAX lineage, others) for one image or a directory."),
];

/// `recomp help` / `recomp help <cmd>` / `recomp <cmd> --help`.
fn print_help(topic: Option<&str>) {
    if let Some(name) = topic {
        if let Some((_, synopsis, flags, desc)) = HELP.iter().find(|(n, ..)| *n == name) {
            eprintln!("usage: {synopsis}");
            eprintln!();
            eprintln!("{desc}");
            if !flags.is_empty() {
                eprintln!();
                for f in *flags {
                    eprintln!("  {f}");
                }
            }
            return;
        }
        eprintln!("unknown command {name:?}");
        eprintln!();
    }
    eprintln!("usage: recomp <command> [args]");
    eprintln!();
    for (_, synopsis, _, _) in HELP {
        eprintln!("  {synopsis}");
    }
    eprintln!();
    eprintln!("'recomp help <command>' for details; BUILDING.md for the full reference");
}

/// Recorded input script: header line `gba-input v1`, then one
/// `<frame> <hexmask>` line per key-state change (KEYINPUT-style,
/// active-low). Keys hold their value between change points; before the
/// first point everything is released. Recorded by play under
/// RECOMP_RECORD_INPUT=<path>, replayed by frames/runc/verify --input —
/// the bridge from a live repro to a deterministic headless one.
struct InputScript(Vec<(u64, u16)>, Vec<(u64,u16)>);

impl InputScript {
    fn load(path: &str) -> Result<InputScript, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Self::parse(&text, path)
    }
    fn parse(text: &str, path: &str) -> Result<InputScript, String> {
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("gba-input v1") {
            return Err(format!("{path}: not a gba-input v1 file"));
        }
        let mut points = Vec::new();
        let mut mouse = Vec::new();
        for l in lines {
            let l = l.trim();
            if l.is_empty() {
                continue;
            }
            let (f, k) = l
                .split_once(' ')
                .ok_or_else(|| format!("{path}: bad line {l:?}"))?;
            if let Some(index)=k.strip_prefix("mouse:") {
                let index:u16=index.parse().map_err(|_|format!("{path}: bad mouse target"))?;
                if !(256..544).contains(&index) && !(4096..42496).contains(&index) && !u8::try_from(index).ok().is_some_and(gba_core::ffta_mouse::LanguageMouse::valid_target) {return Err(format!("{path}: mouse target out of range"));}
                mouse.push((f.parse().map_err(|_|format!("{path}: bad frame"))?,index));continue;
            }
            points.push((
                f.parse()
                    .map_err(|e| format!("{path}: bad frame in {l:?}: {e}"))?,
                u16::from_str_radix(k, 16)
                    .map_err(|e| format!("{path}: bad mask in {l:?}: {e}"))?,
            ));
        }
        // Equal-frame events retain recording order (release then press).
        points.sort_by_key(|&(frame, _)| frame);
        mouse.sort_by_key(|&(frame,_)|frame);
        Ok(InputScript(points,mouse))
    }

    fn apply_mouse(&self,mem:&mut gba_core::mem::MemMap,frame:u64)->Result<(),String> {
        let start=self.1.partition_point(|&(f,_)|f<frame);
        for &(_,index) in self.1[start..].iter().take_while(|&&(f,_)|f==frame) {
            if index>=4096 {
                mem.ffta_battle_mouse.enable_replay();let point=index-4096;
                if !gba_core::ffta_battle_mouse::world_select(mem,(point%240)as f32,(point/240)as f32,false){return Err(format!("world mouse replay mismatch at frame {frame}"));}
                continue;
            }
            if index>=512 {
                mem.ffta_battle_mouse.enable_replay();
                if !gba_core::ffta_battle_mouse::menu_select(mem,(index-512)as u8,false){return Err(format!("battle menu replay mismatch at frame {frame}"));}
                continue;
            }
            if index>=256 {
                mem.ffta_battle_mouse.enable_replay();let cell=index-256;
                if !gba_core::ffta_battle_mouse::select(mem,(cell%16)as u8,(cell/16)as u8,false){return Err(format!("battle mouse replay mismatch at frame {frame}"));}
                continue;
            }
            let index=index as u8;
            let mut mouse=gba_core::ffta_mouse::LanguageMouse::new(&mem.rom,true);
            if !mouse.select(mem,index){return Err(format!("mouse replay scene/ROM mismatch at frame {frame}"));}
        }
        gba_core::ffta_battle_mouse::replay_keys(mem);
        Ok(())
    }
    fn keys_at(&self, frame: u64) -> u16 {
        match self.0.partition_point(|&(f, _)| f <= frame) {
            0 => 0x3FF,
            n => self.0[n - 1].1,
        }
    }
}

/// Load and sanity-check a real BIOS image for --bios runs.
fn load_bios_file(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() != input_config::BIOS_SIZE {
        return Err(format!(
            "{}: expected a {}-byte BIOS image, got {} bytes",
            path.display(),
            input_config::BIOS_SIZE,
            bytes.len()
        ));
    }
    Ok(bytes)
}

/// Construct the machine for a run: real-BIOS mode when an image is
/// given (boots from the reset vector, no HLE), HLE boot otherwise.
fn make_machine(rom: Vec<u8>, bios: Option<&[u8]>) -> Machine {
    let mut machine=match bios {
        Some(b) => Machine::new_with_bios(rom, b),
        None => Machine::new(rom),
    };
    machine.bus.ffta_dialogue.separate=std::env::var("FFTA_UI_LAYER").as_deref()!=Ok("0");
    machine
}

/// Sanity-check a cartridge image before booting it, so a stray file
/// fails with a clear message instead of a cryptic crash downstream.
fn validate_rom(path: &str, rom: &[u8]) -> Result<(), String> {
    if rom.len() < 0xC0 {
        return Err(format!(
            "{path}: not a GBA cartridge image ({} bytes is smaller than the cartridge header)",
            rom.len()
        ));
    }
    if rom.len() > 32 << 20 {
        return Err(format!(
            "{path}: not a GBA cartridge image ({} MB exceeds the 32 MB cartridge space)",
            rom.len() >> 20
        ));
    }
    // Every cartridge or multiboot image begins with an ARM branch to its
    // entry point; an odd dump is worth a warning but not a refusal.
    if rom[3] != 0xEA {
        eprintln!("{path}: entry point is not an ARM branch — this may not be a GBA image");
    }
    Ok(())
}

fn parse_hex(s: &str) -> Result<u32, String> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(s, 16).map_err(|e| format!("bad hex value {s:?}: {e}"))
}

fn cmd_dis(args: &[String]) -> Result<(), String> {
    let mut rom_path = None;
    let mut addr = ROM_BASE;
    let mut count = 16usize;
    let mut thumb = false;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--addr" => addr = parse_hex(it.next().ok_or("--addr needs a value")?)?,
            "--count" => {
                count = it
                    .next()
                    .ok_or("--count needs a value")?
                    .parse()
                    .map_err(|e| format!("bad count: {e}"))?
            }
            "--thumb" => thumb = true,
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    let rom_path = rom_path.ok_or("missing ROM path")?;
    let rom = std::fs::read(&rom_path).map_err(|e| format!("{rom_path}: {e}"))?;

    let mut pc = addr;
    for _ in 0..count {
        let off = pc.wrapping_sub(ROM_BASE) as usize;
        if thumb {
            let Some(bytes) = rom.get(off..off + 2) else {
                break;
            };
            let half = u16::from_le_bytes([bytes[0], bytes[1]]);
            let instr = decode_thumb(half, pc);
            println!("{pc:08x}:     {half:04x}  {}", instr.disasm());
            pc += 2;
        } else {
            let Some(bytes) = rom.get(off..off + 4) else {
                break;
            };
            let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let instr = decode_arm(word, pc);
            println!("{pc:08x}: {word:08x}  {}", instr.disasm());
            pc += 4;
        }
    }
    Ok(())
}

/// Every licensed cartridge image begins with an ARM branch over the header to the
/// entry point. Decoding it across a whole corpus is a cheap end-to-end
/// smoke test of the ARM decoder against real-world data.
fn cmd_entry_scan(args: &[String]) -> Result<(), String> {
    let dir = args.first().ok_or("missing directory")?;
    let mut total = 0u32;
    let mut ok = 0u32;

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    entries.sort();

    for path in &entries {
        total += 1;
        match check_entry(path) {
            Ok(line) => {
                ok += 1;
                println!("OK   {line}  {}", file_name(path));
            }
            Err(e) => println!("FAIL {e}  {}", file_name(path)),
        }
    }
    println!("\n{ok}/{total} ROM entry points decoded as a branch");
    if ok == total && total > 0 {
        Ok(())
    } else {
        Err("entry scan had failures".into())
    }
}

/// Run a ROM in the interpreter until it parks in an unconditional
/// self-loop (the idiom every test ROM ends with), then dump CPU state.
fn cmd_run(args: &[String]) -> Result<(), String> {
    let mut rom_path = None;
    let mut max_steps = 200_000_000u64;
    let mut trace = false;
    let mut hist = false;
    let mut bios: Option<Vec<u8>> = None;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--max-steps" => {
                max_steps = it
                    .next()
                    .ok_or("--max-steps needs a value")?
                    .parse()
                    .map_err(|e| format!("bad max-steps: {e}"))?
            }
            "--trace" => trace = true,
            "--hist" => hist = true,
            "--bios" => bios = Some(load_bios_file(it.next().ok_or("--bios needs a value")?)?),
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    let rom_path = rom_path.ok_or("missing ROM path")?;
    let rom = std::fs::read(&rom_path).map_err(|e| format!("{rom_path}: {e}"))?;

    let mut m = make_machine(rom, bios.as_deref());
    let mut counts: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();

    let mut steps = 0u64;
    while steps < max_steps {
        let event = m.step();
        steps += 1;
        if let StepEvent::Instr(instr) = event {
            if trace {
                eprintln!("{:08x} [{}]: {}", instr.addr, m.bus.clock, instr.disasm());
            }
            // Histogram the tail of the run (where the steady state lives).
            if hist && steps > max_steps.saturating_sub(500_000) {
                *counts.entry(instr.addr).or_default() += 1;
            }
            // Parked: unconditional branch to itself, with no way out.
            if is_self_loop(&instr) && !m.bus.irq_pending() {
                break;
            }
        }
    }

    if hist {
        let mut top: Vec<_> = counts.into_iter().collect();
        top.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        for (addr, n) in top.iter().take(10) {
            println!("hot {addr:08x}: {n}");
        }
    }

    println!("steps: {steps} frames: {}", m.bus.frames);
    for r in 0..16 {
        print!("r{r}={:08x} ", m.cpu.regs[r]);
        if r % 4 == 3 {
            println!();
        }
    }
    println!(
        "cpsr={:08x} mode={:?} thumb={}",
        m.cpu.cpsr,
        m.cpu.mode(),
        m.cpu.thumb()
    );
    {
        use gba_core::Bus as _;
        println!("ewram[0]={:08x}", m.bus.read32(0x0200_0000));
        let dispstat = m.bus.read16(0x0400_0004);
        let biosflags = m.bus.read16(0x0300_7FF8);
        println!("ie={:04x} if={:04x} ime={} dispstat={dispstat:04x} halted={} armed={} biosflags={biosflags:04x}", m.bus.reg_ie, m.bus.reg_if, m.bus.ime, m.bus.halted, m.bus.intr_wait_armed);
    }
    Ok(())
}

/// Run a ROM for N frames headless, print a framebuffer hash, and
/// optionally write the final frame as a binary PPM.
fn cmd_frames(args: &[String]) -> Result<(), String> {
    configure_headless_widescreen();
    let mut rom_path = None;
    let mut use_rom_data = false;
    let mut frames = 60u64;
    let mut out: Option<String> = None;
    let mut keys = 0x3FFu16; // active-low: nothing pressed
    let mut demo = false; // verify-style Start/A taps (menus need edges)
    let mut input: Option<InputScript> = None; // recorded-session replay
    let mut sav: Option<String> = None; // load backup media before boot
    let mut bios: Option<Vec<u8>> = None; // real-BIOS execution (no HLE)

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--rom-data" => use_rom_data = true,
            "--frames" => {
                frames = it
                    .next()
                    .ok_or("--frames needs a value")?
                    .parse()
                    .map_err(|e| format!("bad frames: {e}"))?
            }
            "--out" => out = Some(it.next().ok_or("--out needs a value")?.to_string()),
            "--keys" => keys = parse_hex(it.next().ok_or("--keys needs a value")?)? as u16,
            "--demo" => demo = true,
            "--input" => {
                input = Some(InputScript::load(
                    it.next().ok_or("--input needs a value")?,
                )?)
            }
            "--sav" => sav = Some(it.next().ok_or("--sav needs a value")?.to_string()),
            "--bios" => bios = Some(load_bios_file(it.next().ok_or("--bios needs a value")?)?),
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    let rom_path = rom_path.ok_or("missing ROM path")?;
    let rom = std::fs::read(&rom_path).map_err(|e| format!("{rom_path}: {e}"))?;

    let mut m = make_machine(rom, bios.as_deref());
    if use_rom_data { rom_data::prepare_and_attach(&mut m, false)?; }
    m.bus.keys = keys;
    if let Some(p) = &sav {
        let data = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
        m.bus.load_save_data(&data);
        eprintln!("loaded {p}");
    }

    // Diagnostic (RECOMP_DUMP_AUDIO=<prefix>): capture the full audio taps
    // headless — <prefix>.mixed.wav / .psg.wav (PCM16 mono @ tap rate)
    // and <prefix>.fifoN.bin ((i8 sample, u32le period) DAC events).
    let dump_audio = std::env::var("RECOMP_DUMP_AUDIO").ok();
    let mut mixed: Vec<i16> = Vec::new();
    let mut psg: Vec<i16> = Vec::new();
    let mut fifo_ev: [Vec<(i8, u32)>; 2] = [Vec::new(), Vec::new()];
    let mut hle: Vec<f32> = Vec::new();
    if dump_audio.is_some() {
        m.bus.tap_channels = true;
    }
    // RECOMP_MP2K=1: arm the HLE shadow mixer headless — the
    // differential self-check then runs under `frames`, which is the
    // autonomous validation path for the shadow (no ears required).
    let mp2k_on = std::env::var_os("RECOMP_MP2K").is_some();
    if mp2k_on {
        m.bus.tap_channels = true;
        eprintln!("hle: {}", arm_audio_hle(&mut m, None));
    }
    // Diagnostic (RECOMP_COST_FROM=N): from frame N on, attribute charged
    // cycles to the PC that incurred them; dump the histogram at exit.
    let cost_from: Option<u64> = std::env::var("RECOMP_COST_FROM")
        .ok()
        .and_then(|v| v.parse().ok());
    let mut cost: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    // Triage (RECOMP_WATCH=hexaddr): print a guest word at every frame
    // where it changes — pairs with the reference harness's REF_WATCH
    // for state-timeline diffs.
    let watch: Option<u32> = std::env::var("RECOMP_WATCH")
        .ok()
        .and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok());
    let mut watch_last: Option<u32> = None;
    for _ in 0..frames {
        if demo {
            m.bus.keys = demo_keys(m.bus.frames);
        }
        if let Some(s) = &input {
            m.bus.keys = s.keys_at(m.bus.frames);
            let frame=m.bus.frames;s.apply_mouse(&mut m.bus,frame)?;
        }
        if let Some(a) = watch {
            use gba_core::Bus as _;
            let v = m.bus.read32(a);
            if watch_last != Some(v) {
                eprintln!("WATCH f={} [{a:08x}]={v:08x}", m.bus.frames);
                watch_last = Some(v);
            }
        }
        if cost_from.is_some_and(|n| m.bus.frames >= n) {
            m.bus.frame_ready = false;
            let mut steps = 0u64;
            while !m.bus.frame_ready && steps < 5_000_000 {
                let pc = m.cpu.regs[15];
                let c0 = m.bus.clock;
                m.step();
                *cost.entry(pc).or_insert(0) += m.bus.clock - c0;
                steps += 1;
            }
        } else {
            m.run_frame(5_000_000);
        }
        if dump_audio.is_some() {
            mixed.extend(m.bus.audio_buf.drain(..));
            psg.extend(m.bus.psg_tap.drain(..));
            for f in 0..2 {
                fifo_ev[f].append(&mut m.bus.fifo_tap[f]);
            }
        }
        if mp2k_on {
            hle.extend(m.bus.hle_tap.drain(..));
        }
    }
    if !cost.is_empty() {
        let mut v: Vec<(u32, u64)> = cost.into_iter().collect();
        v.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
        let total: u64 = v.iter().map(|&(_, c)| c).sum();
        eprintln!("COST total={total} cycles over traced frames; top PCs:");
        for (pc, c) in v.iter().take(40) {
            eprintln!("  COST {pc:08x} {c}");
        }
    }
    if let Some(h) = m.bus.mp2k.as_deref() {
        let (corr, ratio) = h.last_correlation();
        eprintln!(
            "mp2k: hooks={} stale={} bad_waves={} corr={corr:.3} ratio={ratio:.2} gain={:.2} mode={} pauses={} proven={} engaged={} active={}{}",
            h.hooks,
            h.stale_ticks,
            h.bad_waves,
            h.gain(),
            h.count_mode(),
            h.vf.pauses,
            h.vf.proven,
            h.engaged,
            h.active,
            h.vf.reverted
                .as_deref()
                .map(|m| format!(" PAUSED: {m}"))
                .unwrap_or_default()
        );
    }
    if let Some(g) = m.bus.gax.as_deref() {
        let (corr, ratio) = g.last_correlation();
        eprintln!(
            "gax: hooks={} stale={} bad_waves={} corr={corr:.3} ratio={ratio:.2} gain={:.2} pauses={} proven={} engaged={} active={}{}",
            g.hooks,
            g.stale_ticks,
            g.bad_waves,
            g.gain(),
            g.vf.pauses,
            g.vf.proven,
            g.engaged,
            g.active,
            g.vf.reverted
                .as_deref()
                .map(|m| format!(" PAUSED: {m}"))
                .unwrap_or_default()
        );
    }
    if let Some(r) = m.bus.rdrv.as_deref() {
        let (corr, ratio) = r.last_correlation();
        eprintln!(
            "rdrv: hooks={} stale={} bad_waves={} corr={corr:.3} ratio={ratio:.2} gain={:.2} pauses={} proven={} engaged={} active={}{}",
            r.hooks,
            r.stale_ticks,
            r.bad_waves,
            r.gain(),
            r.vf.pauses,
            r.vf.proven,
            r.engaged,
            r.active,
            r.vf.reverted
                .as_deref()
                .map(|m| format!(" PAUSED: {m}"))
                .unwrap_or_default()
        );
        if std::env::var_os("RECOMP_RDRV_DISC").is_some() {
            eprintln!("{}", r.disc_report());
        }
    }
    if let Some(prefix) = &dump_audio {
        write_wav(
            &format!("{prefix}.mixed.wav"),
            &mixed,
            gba_core::mem::AUDIO_RATE_HZ,
            2,
        )?;
        write_wav(
            &format!("{prefix}.psg.wav"),
            &psg,
            gba_core::mem::AUDIO_RATE_HZ,
            2,
        )?;
        if !hle.is_empty() {
            let pcm: Vec<i16> = hle
                .iter()
                .map(|v| (v * 32768.0).clamp(-32768.0, 32767.0) as i16)
                .collect();
            write_wav(
                &format!("{prefix}.hle.wav"),
                &pcm,
                gba_core::mem::AUDIO_RATE_HZ,
                2,
            )?;
        }
        for f in 0..2 {
            let mut raw = Vec::with_capacity(fifo_ev[f].len() * 5);
            for &(s, p) in &fifo_ev[f] {
                raw.push(s as u8);
                raw.extend_from_slice(&p.to_le_bytes());
            }
            let path = format!("{prefix}.fifo{f}.bin");
            std::fs::write(&path, raw).map_err(|e| format!("{path}: {e}"))?;
        }
        eprintln!(
            "audio dump: {} mixed stereo frames, fifo events A={} B={}, underruns A={} B={}, refills A={} B={}, pushes A={} B={}",
            mixed.len() / 2,
            fifo_ev[0].len(),
            fifo_ev[1].len(),
            m.bus.fifo_underruns[0],
            m.bus.fifo_underruns[1],
            m.bus.fifo_refills[0],
            m.bus.fifo_refills[1],
            m.bus.fifo_pushes[0],
            m.bus.fifo_pushes[1],
        );
    }

    // Triage: dump RAM snapshots for offline disassembly.
    if let Some(p) = std::env::var_os("RECOMP_DUMP_IWRAM") {
        std::fs::write(&p, &m.bus.iwram).map_err(|e| e.to_string())?;
        eprintln!("dumped iwram to {}", p.to_string_lossy());
    }
    // Triage: dump PPU state (<prefix>.{oam,vram,pal,io}) for offline
    // sprite/tile decoding.
    if let Some(p) = std::env::var_os("RECOMP_DUMP_PPU") {
        let p = p.to_string_lossy();
        std::fs::write(format!("{p}.oam"), &m.bus.oam).map_err(|e| e.to_string())?;
        std::fs::write(format!("{p}.vram"), &m.bus.vram).map_err(|e| e.to_string())?;
        std::fs::write(format!("{p}.pal"), &m.bus.palette).map_err(|e| e.to_string())?;
        std::fs::write(format!("{p}.io"), &m.bus.io).map_err(|e| e.to_string())?;
        eprintln!("dumped ppu state to {p}.*");
    }

    // FNV-1a over the framebuffer for cheap regression hashing.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &px in &m.bus.framebuffer {
        for b in px.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x1_0000_01b3);
        }
    }
    {
        use gba_core::Bus as _;
        let dispcnt = m.bus.read16(0x0400_0000);
        println!(
            "frames: {} hash: {hash:016x} dispcnt: {dispcnt:04x} pc: {:08x} mode: {:?} unhandled_swis: {:x}",
            m.bus.frames, m.cpu.regs[15], m.cpu.mode(), m.bus.unhandled_swis
        );
    }

    // Live disassembly around the final PC (reads through the bus, so
    // RAM-resident code is visible) — invaluable for stuck-loop triage.
    {
        use gba_core::Bus as _;
        let pc = m.cpu.regs[15];
        let start = pc.saturating_sub(12);
        for i in 0..8 {
            if m.cpu.thumb() {
                let a = start + i * 2;
                let h = m.bus.read16(a & !1);
                eprintln!("  {a:08x}: {}", decode_thumb(h, a & !1).disasm());
            } else {
                let a = start + i * 4;
                let w = m.bus.read32(a & !3);
                eprintln!("  {a:08x}: {}", decode_arm(w, a & !3).disasm());
            }
        }
    }

    if let Some(path) = out {
        let width = gba_core::mem::render_width();
        let mut ppm = format!("P6\n{width} 160\n255\n").into_bytes();
        let pixels = if gba_core::mem::extra() == 0 {
            &m.bus.framebuffer[..]
        } else {
            &m.bus.wide_framebuffer[..width * 160]
        };
        for &px in pixels {
            let r = (px & 31) as u8;
            let g = ((px >> 5) & 31) as u8;
            let b = ((px >> 10) & 31) as u8;
            ppm.extend_from_slice(&[r << 3 | r >> 2, g << 3 | g >> 2, b << 3 | b >> 2]);
        }
        std::fs::write(&path, ppm).map_err(|e| format!("{path}: {e}"))?;
        println!("wrote {path}");
    }
    rom_data::report_usage(&m);
    Ok(())
}

/// Dev triage: report MP2K driver detection over a ROM or a directory
/// of ROMs (symlinks followed) — validates the signature scan against
/// the corpus and reports where each hook would land.
fn cmd_mp2k_scan(args: &[String]) -> Result<(), String> {
    let target = args.first().ok_or("missing ROM or directory")?;
    let path = Path::new(target);
    let mut files: Vec<std::path::PathBuf> = if path.is_dir() {
        std::fs::read_dir(path)
            .map_err(|e| format!("{target}: {e}"))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "gba"))
            .collect()
    } else {
        vec![path.to_path_buf()]
    };
    files.sort();
    let (mut hits, mut total) = (0u32, 0u32);
    for f in &files {
        let rom = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        total += 1;
        let sigs = gba_core::mp2k::detect(&rom);
        if sigs.is_empty() {
            println!("---- {}", file_name(f));
        } else {
            hits += 1;
            let entries = sigs
                .iter()
                .map(|sig| {
                    format!(
                        "{:#010x} ({})",
                        sig.sound_main_ram & !1,
                        if sig.sound_main_ram & 1 != 0 {
                            "thumb"
                        } else {
                            "arm"
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join(" / ");
            println!(
                "MP2K {} SoundMain@{:#08x} SoundMainRAM={entries}",
                file_name(f),
                sigs[0].sound_main_off,
            );
        }
    }
    println!("\n{hits}/{total} images carry the stock MP2K driver signature");
    Ok(())
}

/// Arm the appropriate engine HLE shadow for this image (enhanced
/// path). Returns a description line for diagnostics.
///
/// `pin` is the packaged [runtime] engine-hle value: a packager that
/// verified one engine pins it so runtime skips classifying the rest
/// (and "off" never arms a shadow at all).
fn arm_audio_hle(m: &mut Machine, pin: Option<&str>) -> String {
    match pin {
        Some("off") => return "engine HLE pinned off by package".into(),
        Some("m4a") => {
            let sigs = gba_core::mp2k::detect(&m.bus.rom);
            if sigs.is_empty() {
                return "DEGRADED: package pins engine-hle = m4a but no driver signature found \
                        — per-channel enhancement active"
                    .into();
            }
            m.bus.mp2k = Some(Box::new(gba_core::mp2k::Mp2kHle::new(&sigs)));
            return "M4A/MP2K — HLE shadow armed (pinned by package)".into();
        }
        Some("gax") => {
            // v1 detects statically; the banner-era shadow self-validates
            // against the 'GAX3' work-block magic at runtime, so arming it
            // blind under a pin is safe — it simply never engages on a
            // mismatch.
            if let Some(sig) = gba_core::gax::detect_v1(&m.bus.rom) {
                m.bus.gax = Some(Box::new(gba_core::gax::GaxHle::new(sig)));
            } else {
                m.bus.gax = Some(Box::new(gba_core::gax::GaxHle::new_v3()));
            }
            return "GAX — HLE shadow armed (pinned by package)".into();
        }
        Some("rdrv") => {
            if let Some(sig) = gba_core::rdrv::detect(&m.bus.rom) {
                m.bus.rdrv = Some(Box::new(gba_core::rdrv::RdrvHle::new(sig)));
                return "in-house (R) — HLE shadow armed (pinned by package)".into();
            }
            return "DEGRADED: package pins engine-hle = rdrv but no driver found — \
                    per-channel enhancement active"
                .into();
        }
        Some(other) => {
            eprintln!("DEGRADED: unknown engine-hle pin {other:?} — auto-detecting");
        }
        None => {}
    }
    match gba_core::engine::classify(&m.bus.rom) {
        gba_core::engine::Engine::M4a(sigs) => {
            let desc = format!(
                "M4A/MP2K — HLE shadow armed (SoundMainRAM {})",
                sigs.iter()
                    .map(|s| format!("{:#010x}", s.sound_main_ram & !1))
                    .collect::<Vec<_>>()
                    .join("/")
            );
            m.bus.mp2k = Some(Box::new(gba_core::mp2k::Mp2kHle::new(&sigs)));
            desc
        }
        gba_core::engine::Engine::Gax(ver) => {
            if let Some(sig) = gba_core::gax::detect_v1(&m.bus.rom) {
                let desc = "GAX v1 lineage — HLE shadow armed (state block located at runtime)"
                    .to_string();
                m.bus.gax = Some(Box::new(gba_core::gax::GaxHle::new(sig)));
                desc
            } else if ver.is_some() {
                // Banner era (v2/v3): the work block self-identifies at
                // runtime ('GAX3' magic + structural validation).
                m.bus.gax = Some(Box::new(gba_core::gax::GaxHle::new_v3()));
                format!(
                    "GAX {} — HLE shadow armed (work block located at runtime)",
                    ver.as_deref().unwrap_or("?")
                )
            } else {
                "GAX (early, unrecognized revision) — per-channel enhancement active".into()
            }
        }
        gba_core::engine::Engine::Rdiag => {
            if let Some(sig) = gba_core::rdrv::detect(&m.bus.rom) {
                m.bus.rdrv = Some(Box::new(gba_core::rdrv::RdrvHle::new(sig)));
                "in-house (R) — HLE shadow armed (mixer blob located at runtime)".into()
            } else {
                "in-house (R, unrecognized revision) — per-channel enhancement active".into()
            }
        }
        other => format!(
            "{} — per-channel enhancement active (engine HLE not yet available)",
            other.describe()
        ),
    }
}

/// Dev triage: audio-engine classification over a ROM or directory.
fn cmd_engine_scan(args: &[String]) -> Result<(), String> {
    let target = args.first().ok_or("missing ROM or directory")?;
    let path = Path::new(target);
    let mut files: Vec<std::path::PathBuf> = if path.is_dir() {
        std::fs::read_dir(path)
            .map_err(|e| format!("{target}: {e}"))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "gba"))
            .collect()
    } else {
        vec![path.to_path_buf()]
    };
    files.sort();
    let mut tally: std::collections::BTreeMap<String, u32> = Default::default();
    for f in &files {
        let rom = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let engine = gba_core::engine::classify(&rom);
        let d = engine.describe();
        *tally
            .entry(d.split(' ').next().unwrap_or("?").to_string())
            .or_default() += 1;
        println!("{:24} {}", d, file_name(f));
    }
    println!();
    for (k, v) in tally {
        println!("{v:4}  {k}");
    }
    Ok(())
}

fn file_name(p: &Path) -> &str {
    p.file_name().and_then(|s| s.to_str()).unwrap_or("?")
}

fn check_entry(path: &Path) -> Result<String, String> {
    let mut header = [0u8; 4];
    let rom = std::fs::read(path).map_err(|e| e.to_string())?;
    header.copy_from_slice(rom.get(..4).ok_or("ROM shorter than 4 bytes")?);
    let word = u32::from_le_bytes(header);
    let instr = decode_arm(word, ROM_BASE);
    match instr.op {
        Op::Branch {
            link: false,
            target,
        } => Ok(format!("{word:08x} -> {target:08x}")),
        _ => Err(format!("{word:08x} decoded as {:?}", instr.disasm())),
    }
}

/// Apply a live A/V change the in-game menu made. Video settings rebuild
/// the cheap derived objects in place (color LUT, temporal response,
/// grid); the temporal history resets, which is a single frame and
/// invisible. Enhanced audio flips the shared crossfade target — both
/// paths are always warm, so the callback slews between them seamlessly.
/// Only the output gamut stays restart-required (baked into the GPU
/// presenter); the menu persists it to av.cfg for the next launch.
#[allow(clippy::too_many_arguments)]
fn apply_av_change(
    what: menu::Changed,
    av: &input_config::AvConfig,
    streams: &std::sync::Arc<std::sync::Mutex<AudioStreams>>,
    screen_kind: &mut screen::ScreenKind,
    response: &mut screen::ResponseMode,
    lut: &mut screen::ColorLut,
    temporal: &mut screen::Temporal,
    grid_params: &mut screen::present::GridParams,
    lut_target: screen::color::Primaries,
) {
    match what {
        menu::Changed::Audio => {
            // Live: the producer keeps both rings filled; the callback
            // crossfades to the selected path over ~40 ms.
            let mut st = streams.lock().unwrap();
            st.configure_audio(av);
        }
        menu::Changed::Overlay | menu::Changed::Fullscreen => {}
        menu::Changed::Gamut => {
            // Restart-required: persisted to av.cfg by the caller, applied
            // on the next launch. Live rebuild would tear down the GPU
            // presenter mid-session.
        }
        menu::Changed::Vsync => {
            // Applied directly to the presenter by the caller (the
            // presenter isn't threaded through here); persisted to av.cfg
            // like everything else.
        }
        menu::Changed::Video => {
            *screen_kind =
                screen::ScreenKind::by_name(&av.screen).unwrap_or(screen::ScreenKind::Frontlit);
            *response =
                screen::ResponseMode::by_name(&av.response).unwrap_or(screen::ResponseMode::Smart);
            *lut = screen::ColorLut::build_with_brightness(
                &screen::ColorSettings {
                    screen: *screen_kind,
                    darken: input_config::AvConfig::knob(&av.screen_darken)
                        .map(f64::from)
                        .unwrap_or(f64::NAN),
                    target: lut_target,
                },
                f64::from(av.video_brightness),
            );
            *temporal = screen::Temporal::new(
                gba_core::mem::render_width() * 160,
                *response,
                0,
                input_config::AvConfig::knob(&av.response_keep)
                    .unwrap_or_else(|| screen::blend::default_rho(*screen_kind)),
            );
            *grid_params = screen::present::GridParams::with_strength(
                input_config::AvConfig::knob(&av.grid)
                    .unwrap_or_else(|| screen_kind.default_grid_strength()),
            );
        }
    }
}

/// One machine-readable lifecycle line on stdout, for the launcher
/// supervising a `play --status` child. Flushed immediately: the reader
/// is a pipe, and a buffered line is a frozen progress bar.
fn status_line(s: &str) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "STATUS {s}");
    let _ = out.flush();
}

/// Translation cache revision. Bump on ANY change that can invalidate a
/// cached translation: emitter output, runtime ABI, or emulation
/// semantics — `--ram` builds bake interpreter-profiled state (seeds +
/// RAM snapshots) into the translation, so HLE/core fixes count too.
/// Rev history: 1 = initial; 2 = HuffUnComp HLE, whole-block RAM
/// guards, conditional-BL link fix; 3 = MidiKey2Freq HLE + sound
/// FIFO/DMA timing fixes (profiled state shifts); 4 = SWI ends blocks
/// (halt ordering vs following instructions — scanline-effect timing).
// Regional EUR sprite guards require rebuilding translations made by older engines.
// Event-budget entry/continuation guards require regenerating native blocks.
// Rev 8 carries a cycle allowance across native chain groups.
const TRANSLATION_REV: u32 = 8;

// Keep the owning context alive for every translated function pointer.
#[allow(dead_code)] // These fields retain native allocations until emulation stops.
enum NativeLibrary {
    Dll(libloading::Library),
    TinyCc(recomp_core::tcc_memory::NativeCode),
}

/// Locate (or build) the cached native translation for this image.
/// Cache key is the ROM's SHA-256 under the `TRANSLATION_REV` directory,
/// so stale natives can never load; superseded revision dirs are swept.
/// With `status`, build progress is reported as `STATUS building <pct>`.
fn ensure_native(
    rom_path: &str,
    rom: &[u8],
    status: bool,
    bios: Option<&[u8]>,
    gamedb: Option<&gamedb::GameDb>,
) -> Result<(NativeLibrary, BlockTable), String> {
    let sha = rom_sha256(rom);
    let base = input_config::storage::portable_root()
        .map(|root| root.join("Data").join("cache"))
        .unwrap_or_else(|| dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("FFTARecomp"));
    let dir = base.join(format!("t{TRANSLATION_REV}"));
    // Sweep superseded revision directories — they can only hold stale
    // translations no current binary will ever load.
    if let Ok(entries) = std::fs::read_dir(&base) {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('t')
                && name[1..].chars().all(|c| c.is_ascii_digit())
                && *name != *format!("t{TRANSLATION_REV}")
            {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // Experimental EWRAM translations get their own cache entry so they
    // can never be loaded by (or shadow) a normal run.
    let mut suffix = if std::env::var_os("RECOMP_EWRAM").is_some() {
        "-e".to_string()
    } else {
        String::new()
    };
    if matches!(Compiler::detect().backend, recomp_core::Backend::MemoryTcc(_)) {
        suffix.push_str("-tccmem1");
    }
    // Keep Linux's -O2 translations separate from older -O1 artifacts.
    // The ROM cache is local to this machine, so this is safe across distros.
    if cfg!(target_os = "linux") && !matches!(Compiler::detect().backend, recomp_core::Backend::MemoryTcc(_)) {
        suffix.push_str("-o2");
    }
    if gba_core::ffta_log::enabled() {
        // Diagnostic natives contain instruction observers; keep them separate
        // from ordinary cached translations without changing the normal key.
        suffix.push_str("-ffta-log2");
    }
    if gba_core::ffta_rom::eligible(rom) {
        suffix.push_str("-ffta-spr3");
    }
    // Real-BIOS translations bake region-0 code and different boot/SWI
    // semantics into the dylib, so they key on the BIOS content too —
    // swapping the installed BIOS rebuilds rather than loading stale
    // natives, and HLE/real artifacts can never shadow each other.
    if let Some(b) = bios {
        let bsha = rom_sha256(b);
        suffix.push_str(&format!("-b{}", &bsha[..8]));
    }
    // The boundary source — the gamedb when it carries this image
    // (mapper-grade, full coverage), else label files beside it — drives
    // BOTH the build and the cache key, so a gamedb build and a file build
    // never shadow each other and grown coverage retranslates.
    let from_gamedb = gamedb.filter(|db| db.function_count(&sha).map(|n| n > 0).unwrap_or(false));
    let lbl = translation_labels(&sha, match from_gamedb {
        Some(db) => db.labels(&sha, rom.len())?,
        None => labels::load_all(rom_path, &sha, rom.len()),
    });
    let lsuffix = if lbl.is_empty() {
        String::new()
    } else {
        let blob = labels::Blob::load(&labels::blob_path(&sha));
        let d = lbl.digest(blob.as_ref());
        // 'g' = gamedb-sourced (exhaustive), 'l' = label files — distinct
        // key families so switching source always retranslates.
        let tag = if from_gamedb.is_some() { 'g' } else { 'l' };
        format!("-{tag}{:08x}", d as u32 ^ (d >> 32) as u32)
    };
    let lib_path = dir.join(format!("{sha}{suffix}{lsuffix}.{LIB_EXT}"));
    let lib_str = lib_path.to_str().ok_or("non-UTF8 cache path")?;
    if !lib_path.is_file() {
        // Superseded translations of this image (older label sets) can
        // never be loaded again — sweep them before building anew.
        // Same suffix family only, so normal and experimental entries
        // never evict each other.
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let stale = name
                    .strip_prefix(&sha)
                    .and_then(|r| r.strip_prefix(suffix.as_str()))
                    .is_some_and(|r| {
                        r.starts_with('.') || r.starts_with("-l") || r.starts_with("-g")
                    });
                if stale {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        eprintln!("first launch: translating image (one-time)...");
        if status {
            status_line("building 0");
        }
        // The gamedb path is the full map-driven recompile: exhaustive
        // in-function seeding (every decode point in every mapped function)
        // so the launcher gets complete native coverage, like a package.
        let file_source;
        let label_source: &dyn LabelSource = match from_gamedb {
            Some(db) => {
                eprintln!(
                    "seeding from gamedb: {} entries (exhaustive)",
                    lbl.rom.len()
                );
                db
            }
            None => {
                file_source = FileLabels {
                    rom_path,
                    explicit: None,
                };
                &file_source
            }
        };
        build_dylib(
            rom_path,
            true,
            bios,
            label_source,
            from_gamedb.is_some(),
            lib_str,
            &mut |pct, msg| {
                if status {
                    status_line(&format!("building {pct} {msg}"));
                } else {
                    term_progress(pct, msg);
                }
            },
        )?;
        if !status {
            eprintln!();
        }
    }
    let v = load_native(&lib_path)?;
    eprintln!("native translation: {} blocks", v.1.len);
    Ok(v)
}

/// Load a translation library and its block table from an explicit
/// path (the cache, an out/ artifact, or a package).
fn load_native(lib_path: &Path) -> Result<(NativeLibrary, BlockTable), String> {
    // TinyCC's relocatable cache is ELF ET_REL even on Windows. Existing
    // GCC DLLs/ELF shared libraries continue to use the OS loader.
    let mut header = [0u8; 18];
    use std::io::Read;
    let object = std::fs::File::open(lib_path)
        .and_then(|mut f| f.read_exact(&mut header)).is_ok()
        && &header[..4] == b"\x7fELF" && header[5] == 1
        && u16::from_le_bytes([header[16], header[17]]) == 1;
    if object {
        let code = recomp_core::tcc_memory::NativeCode::load(lib_path)?;
        let table = BlockTable::from_blocks(code.blocks()?)?;
        return Ok((NativeLibrary::TinyCc(code), table));
    }
    let lib = unsafe { libloading::Library::new(lib_path) }
        .map_err(|e| format!("{}: {e}", lib_path.display()))?;
    let table = BlockTable::load(&lib)?;
    Ok((NativeLibrary::Dll(lib), table))
}

/// Streams shared between the emulation loop (producer) and the cpal
/// callback (consumer). BOTH rings are always filled so the callback can
/// crossfade between the faithful mix (`mixed`) and the enhanced
/// per-channel taps live. `mixed` and `psg` are interleaved stereo.
#[derive(Default)]
struct AudioStreams {
    profile: Option<audio_profile::Profile>,
    mixed: std::collections::VecDeque<i16>,
    psg: std::collections::VecDeque<f32>,
    fifo: [std::collections::VecDeque<(i8, u32)>; 2],
    /// Target for the faithful↔enhanced crossfade: the callback slews
    /// toward 1.0 (enhanced) or 0.0 (faithful) over ~40 ms. The in-game
    /// menu flips it; both paths stay warm so the switch is seamless.
    enhanced_on: bool,
    /// Output volume cut: fraction removed from full volume (0.0 = full
    /// volume, 1.0 = silent). Stored as a cut so the derived `Default`
    /// means "full volume". Set from av.cfg and the in-game menu.
    vol_cut: f32,
    /// Temporary Fast silent/fallback mute; independent of the saved user choice.
    muted: bool,
    user_muted: bool,
    playback_speed: f64,
    /// Guest PCM pairs produced in the current Fast timeline, measured against
    /// the device sample clock by the callback (no host timestamp required).
    fast_produced: u64,
    normal_audio: Option<ff_audio::NormalAudio>,
    /// Producer invalidation also resets callback-owned DSP history. Accessed
    /// only under the existing stream mutex; not part of guest/save state.
    epoch: u64,
    /// MP2K HLE shadow-mixer stereo (65536 Hz grid, hardware-rail
    /// units like `psg`). When `hle_on`, the callback substitutes this
    /// for the FIFO A/B channels; reverting mid-session falls straight
    /// back to the per-channel interpolators.
    hle: std::collections::VecDeque<f32>,
    hle_on: bool,
    /// SOUNDCNT_H routing snapshot for the enhanced path, refreshed
    /// each frame by the producer: bit 0 = right side, bit 1 = left.
    route: [u8; 2],
    /// SOUNDCNT_H 50%-volume flags per Direct Sound channel.
    vol_half: [bool; 2],
    /// Producer-side ring drops and consumer-side underrun holds since
    /// the last report. Silent audio defects are still defects — the
    /// play loop surfaces nonzero deltas as DEGRADED.
    drops: u64,
    holds: u64,
    /// Sample pairs that crossed the hardware rail (soft-knee engaged)
    /// since the last report. Expected behavior on hot mixes, not a
    /// defect — surfaced as a stats line, never DEGRADED.
    clipped: u64,
    /// Frames the device callback has consumed since stream start — the
    /// play loop's liveness signal for the output device.
    consumed: u64,
    /// Set by the stream error callback; the play loop's watchdog turns
    /// it into a loud wall-clock fallback plus periodic retry.
    dead: bool,
}

impl AudioStreams {
    fn configure_audio(&mut self, av: &input_config::AvConfig) {
        self.enhanced_on = av.audio_enhanced;
        self.vol_cut = 1.0 - av.audio_volume.clamp(0.0, 1.0);
        self.user_muted = av.audio_mute;
    }

    fn volume_target(&self) -> f32 {
        if self.muted || self.user_muted { 0. } else { 1. - self.vol_cut }
    }

    /// Discard output from a timeline/speed that is no longer being played.
    fn discard_pending(&mut self) {
        self.mixed.clear();
        self.psg.clear();
        self.hle.clear();
        self.fifo.iter_mut().for_each(std::collections::VecDeque::clear);
        self.normal_audio = None;
        self.fast_produced = 0;
        self.epoch = self.epoch.wrapping_add(1);
    }
}

/// The premium path's consumer state: one interpolator per Direct Sound
/// channel at its own rate, PSG through the stereo grid resampler, and
/// a DC blocker per side.
struct Enhanced {
    psg: SincResampler,
    fifo: [FifoInterp; 2],
    /// MP2K HLE stream resampler — crossfaded against `fifo` as the
    /// shadow mixer proves/pauses.
    hle: SincResampler,
    /// Crossfade position (0 = per-channel path, 1 = HLE) and its
    /// per-sample slew (~40 ms time constant).
    xfade: f32,
    xfade_k: f32,
    dc: [DcBlock; 2],
}

/// Open the default output device and start the stream, loudly: every
/// failure path says what broke and that the session falls back to
/// silent, wall-clock-paced play (the loop also retries after a
/// mid-session device death).
fn start_audio(streams: std::sync::Arc<std::sync::Mutex<AudioStreams>>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let Some(device) = cpal::default_host().default_output_device() else {
        eprintln!("DEGRADED: no audio output device — silent session, wall-clock pacing");
        return None;
    };
    // Negotiate a config: the device default when its sample format is
    // renderable, else the first supported f32/i16/u16 layout (plenty of
    // ALSA/WASAPI devices default to i16 — refusing them would mean
    // silence on otherwise fine hardware).
    let renderable = |f: cpal::SampleFormat| {
        matches!(
            f,
            cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::U16
        )
    };
    let cfg = match device.default_output_config() {
        Ok(c) if renderable(c.sample_format()) => c,
        first => {
            let fallback = device.supported_output_configs().ok().and_then(|mut it| {
                it.find(|c| renderable(c.sample_format()))
                    .map(|c| c.with_max_sample_rate())
            });
            match (first, fallback) {
                (_, Some(c)) => c,
                (Ok(c), None) => {
                    eprintln!(
                        "DEGRADED: audio device offers no renderable sample format \
                         ({:?}) — silent session, wall-clock pacing",
                        c.sample_format()
                    );
                    return None;
                }
                (Err(e), None) => {
                    eprintln!(
                        "DEGRADED: audio device config unavailable ({e}) — silent \
                         session, wall-clock pacing"
                    );
                    return None;
                }
            }
        }
    };
    let fmt = cfg.sample_format();
    let built = match fmt {
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &cfg, streams),
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, &cfg, streams),
        _ => build_stream::<f32>(&device, &cfg, streams),
    };
    match built {
        Ok(stream) => {
            let _ = stream.play();
            eprintln!(
                "audio: {} Hz, {} ch, {fmt:?}",
                cfg.sample_rate().0,
                cfg.channels()
            );
            Some(stream)
        }
        Err(e) => {
            eprintln!("DEGRADED: audio stream failed ({e}) — silent session, wall-clock pacing");
            None
        }
    }
}

/// Build the typed output stream over the shared rings. Generic over the
/// device sample format — everything renders in f32 and converts on
/// write, so i16/u16-only devices work identically. Both audio paths run
/// every callback; the menu crossfades between them.
fn build_stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    cfg: &cpal::SupportedStreamConfig,
    streams: std::sync::Arc<std::sync::Mutex<AudioStreams>>,
) -> Result<cpal::Stream, String> {
    use cpal::traits::DeviceTrait;
    let mut output = output_callback::<T>(cfg.sample_rate().0 as f64, cfg.channels() as usize, streams.clone());
    device
        .build_output_stream(
            &cfg.config(),
            move |out: &mut [T], _| output(out),
            move |e| {
                eprintln!("audio error: {e}");
                let mut st = streams.lock().unwrap();
                st.dead = true;
                if let Some(profile) = st.profile.as_mut() {
                    profile.device_errors += 1;
                }
            },
            None,
        )
        .map_err(|e| e.to_string())
}

/// The actual device renderer, also callable without a device for parity and
/// starvation checks. Diagnostics only observe; DSP and queue policy are shared.
fn output_callback<T: cpal::SizedSample + cpal::FromSample<f32>>(
    rate: f64,
    channels: usize,
    streams: std::sync::Arc<std::sync::Mutex<AudioStreams>>,
) -> impl FnMut(&mut [T]) + Send + 'static {
    // Select once per stream; no diagnostic branches/counters in default
    // resampler loops and no boxed callback or per-buffer allocation.
    enum Callback<A, B> { Plain(A), Observed(B) }
    let profiling = streams.lock().unwrap().profile.is_some();
    let mut callback = if profiling {
        Callback::Observed(output_callback_impl::<T, true>(rate, channels, streams))
    } else {
        Callback::Plain(output_callback_impl::<T, false>(rate, channels, streams))
    };
    move |out: &mut [T]| match &mut callback {
        Callback::Plain(callback) => callback(out),
        Callback::Observed(callback) => callback(out),
    }
}

fn output_callback_impl<T: cpal::SizedSample + cpal::FromSample<f32>, const OBSERVE: bool>(
    rate: f64,
    channels: usize,
    streams: std::sync::Arc<std::sync::Mutex<AudioStreams>>,
) -> impl FnMut(&mut [T]) + Send + 'static {
    let src = gba_core::mem::AUDIO_RATE_HZ as f64;

    // Both paths run every callback; the menu crossfades between them.
    let mut eng = Enhanced {
        psg: SincResampler::new(src, rate),
        fifo: [FifoInterp::new(rate), FifoInterp::new(rate)],
        hle: SincResampler::new(src, rate),
        xfade: 0.0,
        xfade_k: (1.0 / (0.040 * rate)) as f32,
        dc: [DcBlock::new(rate), DcBlock::new(rate)],
    };
    // Faithful↔enhanced crossfade: start matched to the loaded setting so
    // there's no fade at launch, then slew (~40 ms) whenever the menu
    // flips `enhanced_on`.
    let mut mode_x = {
        let mut st = streams.lock().unwrap();
        if OBSERVE {
            if let Some(profile) = st.profile.as_mut() { profile.attach(rate as u32, channels); }
        }
        if st.enhanced_on { 1.0f32 } else { 0.0 }
    };
    let mode_k = (1.0 / (0.040 * rate)) as f32;
    let step = src / rate;
    let mut frac = 0.0f64;
    let mut last = (0i16, 0i16);
    // The hardware output is AC-coupled: DC at the DAC never reaches
    // the speaker. Model it on the faithful path too, or PSG duty
    // offsets thump on every note edge.
    let mut faithful_dc = [DcBlock::new(rate), DcBlock::new(rate)];
    // Master volume, slewed (~20 ms) so menu changes and mutes never click.
    let mut vol_cur: f32 = {
        let st = streams.lock().unwrap();
        st.volume_target()
    };
    let vol_k = (1.0 / (0.020 * rate)) as f32;
    let mut epoch = streams.lock().unwrap().epoch;
    let mut fast_clock = fast_audio_rate::Clock::new(src, rate);
    move |out: &mut [T]| {
        let begin = OBSERVE.then(std::time::Instant::now);
        let mut st = streams.lock().unwrap();
        let acquired = begin.map(|_| std::time::Instant::now());
        let observation = OBSERVE.then(|| (st.holds, st.mixed.len() / 2));
        let mut source_reads = [audio_profile::SourceReads::default(); 4];
        let normal_observation = if OBSERVE {
            st.normal_audio.as_ref().map(|queue| (st.epoch, queue.stats()))
        } else { None };
        let requested_speed = st.playback_speed.max(1.0);
        let normal_audio = st.normal_audio.is_some();
        let fast_audio = requested_speed > 1. && !normal_audio;
        if epoch != st.epoch {
            epoch = st.epoch;
            fast_clock.reset();
            eng.psg.reset();
            eng.hle.reset();
            eng.fifo.iter_mut().for_each(FifoInterp::reset);
            eng.dc.iter_mut().for_each(DcBlock::reset);
            eng.xfade = 0.0;
            faithful_dc.iter_mut().for_each(DcBlock::reset);
            frac = 0.0;
            last = (0, 0);
            mode_x = if st.enhanced_on && !normal_audio { 1.0 } else { 0.0 };
            // Ramp new audio in through the existing volume slew. No samples
            // from the abandoned timeline participate in this transition.
            vol_cur = 0.0;
        }
        // Liveness signal for the play loop's device watchdog.
        st.consumed += (out.len() / channels.max(1)) as u64;
        let fast_plan = fast_audio.then(|| fast_clock.begin(
            st.fast_produced, st.mixed.len() / 2, out.len() / channels.max(1), requested_speed));
        let waiting = fast_plan.is_some_and(|plan| !plan.ready);
        if waiting {
            // Explicit, observable re-priming; never repeat old source samples.
            // Keep the history reset while there is no new audible timeline.
            eng.psg.reset(); eng.hle.reset();
            eng.fifo.iter_mut().for_each(FifoInterp::reset);
            eng.dc.iter_mut().for_each(DcBlock::reset);
            faithful_dc.iter_mut().for_each(DcBlock::reset);
            eng.xfade = 0.; frac = 0.; last = (0, 0); vol_cur = 0.;
            out.fill(T::EQUILIBRIUM);
        } else {
        for frame in out.chunks_mut(channels) {
            let playback_speed = if fast_audio { fast_clock.next() } else { requested_speed };
            // Enhanced output (always computed so the ring stays
            // drained and the path is warm for a live switch).
            let (el, er) = {
                let e = &mut eng;
                let (pl, pr) = e.psg.next_clocked::<OBSERVE>(&mut st.psg, playback_speed, &mut source_reads[0], !fast_audio);
                let mut l = pl;
                let mut r = pr;
                {
                    // BOTH Direct Sound paths stay warm — the
                    // per-channel interpolators and the MP2K
                    // shadow stream — and a short equal-power-
                    // ish crossfade moves between them, so an
                    // HLE engage or pause is never an audible
                    // seam (both render the same music).
                    let a = e.fifo[0].next_with_clock::<OBSERVE>(&mut st.fifo[0], playback_speed, &mut source_reads[2], !fast_audio)
                        * if st.vol_half[0] { 0.5 } else { 1.0 };
                    let b = e.fifo[1].next_with_clock::<OBSERVE>(&mut st.fifo[1], playback_speed, &mut source_reads[3], !fast_audio)
                        * if st.vol_half[1] { 0.5 } else { 1.0 };
                    let (hl, hr) = e.hle.next_clocked::<OBSERVE>(&mut st.hle, playback_speed, &mut source_reads[1], !fast_audio);
                    let hr = hr * if st.vol_half[0] { 0.5 } else { 1.0 };
                    let hl = hl * if st.vol_half[1] { 0.5 } else { 1.0 };
                    let target = if st.hle_on { 1.0 } else { 0.0 };
                    e.xfade += (target - e.xfade) * e.xfade_k;
                    let xf = e.xfade;
                    let mut fl = 0.0;
                    let mut fr = 0.0;
                    if st.route[0] & 2 != 0 {
                        fl += a;
                    }
                    if st.route[0] & 1 != 0 {
                        fr += a;
                    }
                    if st.route[1] & 2 != 0 {
                        fl += b;
                    }
                    if st.route[1] & 1 != 0 {
                        fr += b;
                    }
                    let mut gl = 0.0;
                    let mut gr = 0.0;
                    if st.route[0] & 2 != 0 || st.route[1] & 2 != 0 {
                        gl = hl;
                    }
                    if st.route[0] & 1 != 0 || st.route[1] & 1 != 0 {
                        gr = hr;
                    }
                    l += fl * (1.0 - xf) + gl * xf;
                    r += fr * (1.0 - xf) + gr * xf;
                    // Stage 2: where the hardware would hard-clip
                    // the over-rail sum, saturate it softly
                    // instead — before the DC block, matching the
                    // hardware order (rail precedes the output
                    // coupling). Below the rail this is exact
                    // identity, so unclipped material is
                    // bit-identical to the pre-Stage-2 path.
                    if l.abs() > RAIL || r.abs() > RAIL {
                        st.clipped += 1;
                    }
                }
                (e.dc[0].next(soft_clip(l)), e.dc[1].next(soft_clip(r)))
            };
            // Faithful output (also always computed, also drained).
            let (fl, fr) = {
                // The emulated DAC runs on the 59.73 Hz frame
                // grid while the window paces 60 — steer the
                // faithful path too, or the ring overflows
                // into a burst-drop buzz at the cap.
                let pairs = (st.mixed.len() / 2) as f64;
                let target_pairs = RING_TARGET as f64 * playback_speed;
                let err = if normal_audio || fast_audio { 0.0 } else { (pairs - target_pairs) / target_pairs };
                frac += step * playback_speed * (1.0 + 0.02 * err.clamp(-1.0, 1.0));
                while frac >= 1.0 {
                    frac -= 1.0;
                    let queued = st.normal_audio.as_mut().map(|queue| queue.next_pair());
                    match queued {
                        Some(Some(pair)) => last = pair,
                        Some(None) => st.holds += 1,
                        None => match (st.mixed.pop_front(), st.mixed.pop_front()) {
                            (Some(l), Some(r)) => last = (l, r),
                            _ => st.holds += 1,
                        },
                    }
                }
                (
                    faithful_dc[0].next(last.0 as f32 / 32768.0),
                    faithful_dc[1].next(last.1 as f32 / 32768.0),
                )
            };
            // Crossfade between the two finished paths. Both are
            // level-matched (shared OUT_GAIN below), so a linear
            // blend across the ~40 ms slew is seam-free — the
            // music is the same, only the rendering differs.
            let target = if st.enhanced_on && !normal_audio { 1.0 } else { 0.0 };
            mode_x += (target - mode_x) * mode_k;
            let l = fl + (el - fl) * mode_x;
            let r = fr + (er - fr) * mode_x;
            // Calibrate the hardware rail (±0x200 units = ±0.5
            // here) toward device full scale. Both paths share
            // the gain so the A/V toggle stays level-matched;
            // it leaves ~2 dB above the rail for the enhanced
            // path's soft knee. The clamp is a safety net for
            // DC-block overshoot — faithful content was already
            // rail-clipped at the mix (the canon hard clip).
            let vol_target = st.volume_target();
            vol_cur += (vol_target - vol_cur) * vol_k;
            // Finish the exponential mute tail below -100 dB instead of
            // leaving tiny nonzero floating-point output indefinitely.
            if vol_target == 0. && vol_cur < 0.00001 { vol_cur = 0.; }
            let l = (l * OUT_GAIN * vol_cur).clamp(-1.0, 1.0);
            let r = (r * OUT_GAIN * vol_cur).clamp(-1.0, 1.0);
            match frame {
                [m] => *m = T::from_sample((l + r) * 0.5),
                [fl, fr, rest @ ..] => {
                    *fl = T::from_sample(l);
                    *fr = T::from_sample(r);
                    for ch in rest {
                        *ch = T::from_sample((l + r) * 0.5);
                    }
                }
                [] => {}
            }
        }
        }
        if let (Some(begin), Some(acquired), Some((holds, mixed))) = (begin, acquired, observation) {
            let end = std::time::Instant::now();
            let holds = st.holds.saturating_sub(holds);
            let enhanced_target = st.enhanced_on && !normal_audio;
            let configured_target = [
                enhanced_target,
                enhanced_target && st.hle_on && st.route.iter().any(|&route| route != 0),
                enhanced_target && !st.hle_on && st.route[0] != 0,
                enhanced_target && !st.hle_on && st.route[1] != 0,
            ];
            if let Some(profile) = st.profile.as_mut() {
                if let Some((epoch, stats)) = normal_observation {
                    profile.normal_audio(epoch, stats);
                }
                if let Some(plan) = fast_plan {
                    profile.fast_audio(plan, out.len() / channels.max(1), mixed);
                }
                profile.callback([begin, acquired, end], out.len() / channels.max(1), holds, mixed);
                profile.sources(source_reads, configured_target);
            }
        }
    }
}

const SINC_TAPS: usize = 24;
const SINC_PHASES: usize = 128;
/// Ring fill the rate control steers toward (~62 ms at the 65536 Hz tap).
const RING_TARGET: usize = 4096;
/// Hardware clip rail in the float mix domain (±0x200 units = ±0.5).
const RAIL: f32 = 0.5;
/// Output gain calibrating the rail toward device full scale. 1.6 puts
/// the rail ~2 dB under full scale — the headroom the enhanced path's
/// soft knee lands over-rail peaks in.
const OUT_GAIN: f32 = 1.6;

/// Stage-2 soft-knee saturator (enhanced path only): exact identity
/// through the hardware rail, then a tanh knee compressing the over-rail
/// region (the six-channel sum can reach 3x the rail) into the headroom
/// above it, asymptoting at full scale after OUT_GAIN. C1 at the knee.
/// Faithful mode keeps the hard rail — clipping is sometimes the
/// intended texture (the research note's "GBA speaker" mono examples).
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= RAIL {
        return x;
    }
    let h = 1.0 / OUT_GAIN - RAIL; // headroom above the rail
    (RAIL + h * ((a - RAIL) / h).tanh()).copysign(x)
}

/// Polyphase interpolation table: (SINC_PHASES + 1) rows of SINC_TAPS
/// coefficients, Hann-windowed sinc at cutoff `fc` (cycles per source
/// sample), each row normalized to unity DC gain.
fn build_sinc_table(fc: f64) -> Vec<f32> {
    use std::f64::consts::PI;
    let center = (SINC_TAPS / 2 - 1) as f64;
    let half = SINC_TAPS as f64 / 2.0;
    let mut table = Vec::with_capacity((SINC_PHASES + 1) * SINC_TAPS);
    for p in 0..=SINC_PHASES {
        let phi = p as f64 / SINC_PHASES as f64;
        let mut row = [0.0f64; SINC_TAPS];
        let mut sum = 0.0;
        for (k, c) in row.iter_mut().enumerate() {
            let t = k as f64 - center - phi;
            let sinc = if t.abs() < 1e-9 {
                2.0 * fc
            } else {
                (2.0 * PI * fc * t).sin() / (PI * t)
            };
            let win = if t.abs() < half {
                0.5 * (1.0 + (PI * t / half).cos())
            } else {
                0.0
            };
            *c = sinc * win;
            sum += *c;
        }
        table.extend(row.iter().map(|c| (c / sum) as f32));
    }
    table
}

/// One-pole DC blocker, fc ~ 10 Hz: y[n] = x[n] - x[n-1] + r*y[n-1].
/// Kills bias offsets, held-DAC ledges, and underrun pops.
struct DcBlock {
    r: f32,
    x1: f32,
    y1: f32,
}

impl DcBlock {
    fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }

    fn new(out_hz: f64) -> DcBlock {
        DcBlock {
            r: 1.0 - (2.0 * std::f64::consts::PI * 10.0 / out_hz) as f32,
            x1: 0.0,
            y1: 0.0,
        }
    }

    fn next(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

/// Fixed-ratio polyphase windowed-sinc resampler over an interleaved
/// stereo queue, with gentle (max ±2%) consumption-rate control against
/// the queue fill. Band-limits to the narrower of the two Nyquists, so
/// downsampling to a 44.1/48 kHz device does not fold images the way
/// the zero-order-hold path does. Used for the PSG grid stream.
struct SincResampler {
    table: Vec<f32>,
    fast_tables: [Vec<f32>; 3],
    /// 1..8.25 in 1/8 steps. Built once with the stream, never in the callback.
    /// Adjacent conservative cutoffs blend continuously at fractional speeds.
    adaptive_tables: Vec<Vec<f32>>,
    /// Last SINC_TAPS source samples per side, oldest first.
    hist: [[f32; SINC_TAPS]; 2],
    ratio: f64,
    frac: f64,
}

impl SincResampler {
    fn reset(&mut self) {
        self.hist = [[0.0; SINC_TAPS]; 2];
        self.frac = 0.0;
    }

    fn new(src_hz: f64, out_hz: f64) -> SincResampler {
        // 10% under the narrower Nyquist leaves the window transition
        // band room.
        SincResampler {
            table: build_sinc_table(0.45 * (out_hz / src_hz).min(1.0)),
            fast_tables: [2.0, 4.0, 8.0]
                .map(|speed| build_sinc_table(0.45 * (out_hz / (src_hz * speed)).min(1.0))),
            adaptive_tables: (0..=58).map(|i| {
                let speed = 1. + i as f64 / 8.;
                build_sinc_table(0.45 * (out_hz / (src_hz * speed)).min(1.0))
            }).collect(),
            hist: [[0.0; SINC_TAPS]; 2],
            ratio: src_hz / out_hz,
            frac: 0.0,
        }
    }

    #[cfg(test)]
    fn next(&mut self, q: &mut std::collections::VecDeque<f32>) -> (f32, f32) {
        self.next_with_speed(q, 1.0)
    }

    #[cfg(test)]
    fn next_with_speed(
        &mut self,
        q: &mut std::collections::VecDeque<f32>,
        speed: f64,
    ) -> (f32, f32) {
        self.next_observed::<false>(q, speed, &mut audio_profile::SourceReads::default())
    }

    #[cfg(test)]
    fn next_observed<const OBSERVE: bool>(
        &mut self,
        q: &mut std::collections::VecDeque<f32>,
        speed: f64,
        reads: &mut audio_profile::SourceReads,
    ) -> (f32, f32) {
        self.next_clocked::<OBSERVE>(q, speed, reads, true)
    }

    fn next_clocked<const OBSERVE: bool>(
        &mut self, q: &mut std::collections::VecDeque<f32>, speed: f64,
        reads: &mut audio_profile::SourceReads, steer: bool,
    ) -> (f32, f32) {
        // Steer consumption toward the target fill; an underrun repeats
        // the newest pair (a one-sample ZOH the DC blocker smooths).
        let pairs = (q.len() / 2) as f64;
        let target = RING_TARGET as f64 * speed;
        let err = if steer { (pairs - target) / target } else { 0. };
        self.frac += self.ratio * speed * (1.0 + 0.02 * err.clamp(-1.0, 1.0));
        while self.frac >= 1.0 {
            self.frac -= 1.0;
            if OBSERVE { reads.requests += 1; }
            let (l, r) = match (q.pop_front(), q.pop_front()) {
                (Some(l), Some(r)) => (l, r),
                _ => {
                    if OBSERVE { reads.empty += 1; }
                    (self.hist[0][SINC_TAPS - 1], self.hist[1][SINC_TAPS - 1])
                }
            };
            for (h, s) in self.hist.iter_mut().zip([l, r]) {
                h.copy_within(1.., 0);
                h[SINC_TAPS - 1] = s;
            }
        }
        let table = match speed as u32 {
            2 => &self.fast_tables[0],
            4 => &self.fast_tables[1],
            8 => &self.fast_tables[2],
            _ => &self.table,
        };
        let row = &table[(self.frac * SINC_PHASES as f64) as usize * SINC_TAPS..][..SINC_TAPS];
        if !steer {
            let position = (speed.clamp(1., 8.25) - 1.) * 8.;
            let index = position.floor() as usize;
            // Both filters are at or below the current output Nyquist. Using
            // floor(speed) alone admitted aliasing at fractional Fast speeds.
            let first = &self.adaptive_tables[(index + 1).min(58)];
            let second = &self.adaptive_tables[(index + 2).min(58)];
            let offset = (self.frac * SINC_PHASES as f64) as usize * SINC_TAPS;
            let weight = position.fract() as f32;
            let mut output = [0.; 2];
            for k in 0..SINC_TAPS {
                let coefficient = first[offset + k] + (second[offset + k] - first[offset + k]) * weight;
                output[0] += coefficient * self.hist[0][k];
                output[1] += coefficient * self.hist[1][k];
            }
            return (output[0], output[1]);
        }
        (
            row.iter()
                .zip(self.hist[0].iter())
                .map(|(c, s)| c * s)
                .sum(),
            row.iter()
                .zip(self.hist[1].iter())
                .map(|(c, s)| c * s)
                .sum(),
        )
    }
}

/// Renders one Direct Sound channel as band-limited steps (the blip-buf
/// idea): each DAC transition deposits a windowed-sinc impulse — the
/// step's derivative — into a small future ring at its exact fractional
/// output position, and a leaky integrator reconstructs the waveform.
/// Edges land jitter-free at the channel's own timer rate (which may
/// change at any event), and crucially the DAC staircase spectrum — the
/// ZOH images above the mix rate's Nyquist — survives up to the device
/// Nyquist. That brightness is part of the hardware's canon sound;
/// smooth sample interpolation removes it and audibly dulls transients
/// (reference-trace comparison on a square-timbre chime). Channels
/// driven above the device rate degrade gracefully: overlapping
/// deposits sum to a proper low-pass.
struct FifoInterp {
    /// Windowed-sinc impulse table (rows sum to 1 = unit step).
    table: Vec<f32>,
    /// Future deposits, ring-indexed by output sample.
    fut: [f32; SINC_TAPS * 2],
    pos: usize,
    /// Leaky integrator state. The tiny leak only bounds drift; real
    /// output coupling is the shared DC blocker downstream.
    y: f32,
    /// Current DAC level, for edge deltas.
    level: f32,
    /// Master cycles until the next DAC event is due.
    t_next: f64,
    /// Current source sample spacing in master cycles.
    period: f64,
    /// Master cycles per output sample.
    out_step: f64,
}

impl FifoInterp {
    fn reset(&mut self) {
        self.fut.fill(0.0);
        self.pos = 0;
        self.y = 0.0;
        self.level = 0.0;
        self.t_next = 512.0;
        self.period = 512.0;
    }

    #[cfg(test)]
    fn next_with_speed_observed<const OBSERVE: bool>(
        &mut self,
        q: &mut std::collections::VecDeque<(i8, u32)>,
        speed: f64,
        reads: &mut audio_profile::SourceReads,
    ) -> f32 {
        self.next_with_clock::<OBSERVE>(q, speed, reads, true)
    }

    fn next_with_clock<const OBSERVE: bool>(
        &mut self, q: &mut std::collections::VecDeque<(i8, u32)>, speed: f64,
        reads: &mut audio_profile::SourceReads, steer: bool,
    ) -> f32 {
        let base = self.out_step;
        self.out_step = base * speed;
        let sample = self.next_clocked::<OBSERVE>(q, reads, steer);
        self.out_step = base;
        sample
    }
    fn new(out_hz: f64) -> FifoInterp {
        FifoInterp {
            table: build_sinc_table(0.45),
            fut: [0.0; SINC_TAPS * 2],
            pos: 0,
            y: 0.0,
            level: 0.0,
            t_next: 512.0,
            period: 512.0, // placeholder until the first DAC event
            out_step: (1u64 << 24) as f64 / out_hz,
        }
    }

    #[cfg(test)]
    fn next(&mut self, q: &mut std::collections::VecDeque<(i8, u32)>) -> f32 {
        self.next_observed::<false>(q, &mut audio_profile::SourceReads::default())
    }
    #[cfg(test)]
    fn next_observed<const OBSERVE: bool>(
        &mut self, q: &mut std::collections::VecDeque<(i8, u32)>,
        reads: &mut audio_profile::SourceReads,
    ) -> f32 {
        self.next_clocked::<OBSERVE>(q, reads, true)
    }

    fn next_clocked<const OBSERVE: bool>(
        &mut self, q: &mut std::collections::VecDeque<(i8, u32)>,
        reads: &mut audio_profile::SourceReads, steer: bool,
    ) -> f32 {
        // Steer toward ~62 ms queued at the channel's current rate.
        let target = ((1u64 << 24) as f64 / self.period / 16.0).max(64.0);
        let err = if steer { (q.len() as f64 - target) / target } else { 0. };
        let stepc = self.out_step * (1.0 + 0.02 * err.clamp(-1.0, 1.0));

        self.t_next -= stepc;
        while self.t_next <= 0.0 {
            // The event fires inside this output sample, at fraction phi
            // of the sample period.
            let phi = (1.0 + self.t_next / stepc).clamp(0.0, 1.0);
            if OBSERVE { reads.requests += 1; }
            let new_level = match q.pop_front() {
                Some((s, p)) => {
                    self.period = (p as f64).max(1.0);
                    s as f32 * (64.0 / 32768.0)
                }
                None => {
                    if OBSERVE { reads.empty += 1; }
                    self.level // DAC holds; an inactive warm channel can be empty.
                }
            };
            let delta = new_level - self.level;
            self.level = new_level;
            if delta != 0.0 {
                let row =
                    &self.table[(phi * SINC_PHASES as f64) as usize * SINC_TAPS..][..SINC_TAPS];
                for (k, c) in row.iter().enumerate() {
                    self.fut[(self.pos + k) % (SINC_TAPS * 2)] += delta * c;
                }
            }
            self.t_next += self.period;
        }

        self.y = self.y * 0.9999 + self.fut[self.pos];
        self.fut[self.pos] = 0.0;
        self.pos = (self.pos + 1) % (SINC_TAPS * 2);
        self.y
    }
}

/// Vendored community controller mapping DB (zlib; see THIRD-PARTY.md).
/// gilrs bundles its own snapshot, but it lags newer controllers — this
/// fresher copy is fed on top so the packaged play binary recognizes the
/// broadest possible set of pads with no external file dependency.
const CONTROLLER_DB: &str = include_str!("../../../assets/gamecontrollerdb.txt");

/// Build a gilrs context with the vendored DB layered over the built-in
/// mappings. Falls back to a plain context if the builder rejects (the
/// dummy backend on headless hosts), so the play loop never hard-fails
/// over input init.
fn build_gilrs() -> Option<gilrs::Gilrs> {
    gilrs::GilrsBuilder::new()
        .add_mappings(CONTROLLER_DB)
        .build()
        .ok()
        .or_else(|| gilrs::Gilrs::new().ok())
}

/// Gilrs button name (the launcher stores `{:?}` of the button) to a
/// gilrs `Button`. Stick-direction tokens are handled elsewhere (they
/// read axes, not buttons) and return `None` here.
#[cfg(not(target_os = "linux"))]
fn gilrs_button(name: &str) -> Option<gilrs::Button> {
    use gilrs::Button::*;
    Some(match name {
        "South" => South,
        "East" => East,
        "North" => North,
        "West" => West,
        "C" => C,
        "Z" => Z,
        "LeftTrigger" => LeftTrigger,
        "LeftTrigger2" => LeftTrigger2,
        "RightTrigger" => RightTrigger,
        "RightTrigger2" => RightTrigger2,
        "Select" => Select,
        "Start" => Start,
        "Mode" => Mode,
        "LeftThumb" => LeftThumb,
        "RightThumb" => RightThumb,
        "DPadUp" => DPadUp,
        "DPadDown" => DPadDown,
        "DPadLeft" => DPadLeft,
        "DPadRight" => DPadRight,
        _ => return None,
    })
}

/// Is a stick token pushed past the deadzone right now? `(horizontal,
/// positive)` from the token; gilrs reports both sticks' Y as up-positive,
/// matching the token directions.
#[cfg(not(target_os = "linux"))]
fn stick_token_active(gp: &gilrs::Gamepad, tok: input_config::StickToken, deadzone: f32) -> bool {
    use gilrs::Axis::*;
    let (horizontal, positive) = tok.axis();
    let axis = match (tok.left(), horizontal) {
        (true, true) => LeftStickX,
        (true, false) => LeftStickY,
        (false, true) => RightStickX,
        (false, false) => RightStickY,
    };
    let v = gp.value(axis);
    if positive {
        v > deadzone
    } else {
        v < -deadzone
    }
}

/// Is the configured pad binding (button name or stick token) active?
#[cfg(not(target_os = "linux"))]
fn pad_pressed(gp: &gilrs::Gamepad, name: &str, deadzone: f32) -> bool {
    match input_config::pad_binding(name) {
        input_config::PadBinding::Button(b) => {
            gilrs_button(b).is_some_and(|btn| gp.is_pressed(btn))
        }
        input_config::PadBinding::Stick(t) => stick_token_active(gp, t, deadzone),
    }
}

/// Statically recompile a ROM: analyze, emit C, compile to a shared
/// library with the system C compiler.
fn cmd_build(args: &[String]) -> Result<(), String> {
    let mut rom_path = None;
    let mut ram = false;
    let mut bios: Option<Vec<u8>> = None;
    let mut explicit_labels: Option<String> = None;
    let mut gamedb_path: Option<String> = None;
    let mut output: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--ram" => ram = true,
            "--out" => output = Some(it.next().ok_or("--out needs a value")?.to_string()),
            "--bios" => bios = Some(load_bios_file(it.next().ok_or("--bios needs a value")?)?),
            "--labels" => {
                explicit_labels = Some(it.next().ok_or("--labels needs a value")?.to_string())
            }
            "--gamedb" => {
                gamedb_path = Some(it.next().ok_or("--gamedb needs a value")?.to_string())
            }
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    if gamedb_path.is_some() && explicit_labels.is_some() {
        return Err("--gamedb and --labels are mutually exclusive".into());
    }
    let rom_path = &rom_path.ok_or("missing ROM path")?;
    std::fs::create_dir_all("out").map_err(|e| e.to_string())?;
    let stem = Path::new(rom_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("game")
        .to_string();
    // Real-BIOS builds get their own artifact: the translation bakes in
    // different boot/SWI semantics, so it must never shadow an HLE build.
    let suffix = if bios.is_some() { "-bios" } else { "" };
    // Either source produces a `Labels`; bind both so the chosen one
    // outlives the `&dyn LabelSource` handed to the build.
    let file_source;
    let db_source;
    let label_source: &dyn LabelSource = match &gamedb_path {
        Some(p) => {
            db_source = gamedb::GameDb::open(Path::new(p))?;
            &db_source
        }
        None => {
            file_source = FileLabels {
                rom_path,
                explicit: explicit_labels.as_deref(),
            };
            &file_source
        }
    };
    let exhaustive = std::env::var_os("RECOMP_EXHAUSTIVE").is_some();
    let r = build_dylib(
        rom_path,
        ram,
        bios.as_deref(),
        label_source,
        exhaustive,
        &output.unwrap_or_else(|| format!("out/{stem}{suffix}.{LIB_EXT}")),
        &mut term_progress,
    );
    eprintln!();
    r
}

/// Use identical hint unions for cache identity and compilation. Existing
/// file/database entries and metadata remain intact; growth changes the digest.
fn translation_labels(sha: &str, mut source: labels::Labels) -> labels::Labels {
    source.rom.extend(gba_core::ffta_rom::native_rom_entries(sha).iter().copied());
    source
}

/// The build pipeline behind `cmd_build`, parameterized on the output
/// path so the play runtime can target its own translation cache.
/// Intermediates land next to `lib_path` as `<stem>.<i>.{c,o}`.
/// `progress` receives a whole-build percentage (monotonic, 0..=100);
/// phase weights are rough but the dominant compile phase is exact
/// (blocks compiled / total blocks).
fn build_dylib(
    rom_path: &str,
    ram: bool,
    bios: Option<&[u8]>,
    label_source: &dyn LabelSource,
    exhaustive: bool,
    lib_path: &str,
    progress: &mut dyn FnMut(u8, &str),
) -> Result<(), String> {
    let rom = std::fs::read(rom_path).map_err(|e| format!("{rom_path}: {e}"))?;

    // Phase budget, weighted by image size: bigger images spend nearly
    // all their build inside the C compiler, so the profiling slice of
    // the bar shrinks as the image grows (the bar should track wall
    // time, not phase count).
    let rom_mb = ((rom.len() >> 20) as u64).max(1);
    let prof_end = (64 / rom_mb).clamp(4, 20) as u8;

    // Profile-guided RAM discovery: run the interpreter briefly, recording
    // control-transfer targets in EWRAM/IWRAM, then translate the observed
    // RAM-resident code from the end-of-run snapshot (content-guarded at
    // execution time).
    // RECOMP_EWRAM=1 (experimental): seed and translate EWRAM-resident
    // code too, under the same whole-block content guards as IWRAM.
    // Default-off pending the resident-vs-streamed-overlay measurements
    // for the labels design.
    let ewram_xlat = std::env::var_os("RECOMP_EWRAM").is_some();
    let (seeds, ewram, iwram) = if !ram {
        (Vec::new(), Vec::new(), Vec::new())
    } else {
        // Profile under the same boot semantics the output will run with:
        // a real-BIOS build profiles a real-BIOS run.
        let mut m = make_machine(rom.clone(), bios);
        let mut seeds = std::collections::BTreeSet::new();
        let mut prev_end = 0u32;
        let mut steps = 0u64;
        let mut last_pct = 0u8;
        let mut last_frame = u64::MAX;
        // Profile until the title has actually made sound, not a fixed
        // boot window: silent-boot titles install their IWRAM mixer
        // early but first execute it when music starts (often behind a
        // menu), which a 240-frame profile never sees. Demo input
        // drives the menus; engagement = the first nonzero audio
        // output sample (FIFO init-priming is silent zeros, so pushes
        // alone are a false signal), then a margin seeds the mixer's
        // hot paths. RAM-seed quiescence extends further if discovery
        // is still live; caps bound titles that never make sound.
        const PROFILE_MIN_FRAMES: u64 = 240;
        const PROFILE_MAX_FRAMES: u64 = 1800;
        const PROFILE_AUDIO_MARGIN: u64 = 120;
        let mut audio_from: Option<u64> = None;
        let mut audio_base: Option<i16> = None;
        // The window counts frames of *game* execution: under a real
        // BIOS the boot animation runs ~230 frames before the cart
        // entry is ever reached, which would otherwise consume the
        // whole window (measured: 0 entry points profiled). The boot
        // chime is also not the soundtrack — audio engagement only
        // counts after cart handoff.
        let mut cart_frame0: Option<u64> = None;
        loop {
            if cart_frame0.is_none() && m.cpu.regs[15] >= 0x0800_0000 {
                cart_frame0 = Some(m.bus.frames);
            }
            let game_frames = cart_frame0.map_or(0, |f0| m.bus.frames - f0);
            let done = cart_frame0.is_some()
                && (game_frames >= PROFILE_MAX_FRAMES
                    || audio_from.is_some_and(|f0| {
                        game_frames >= (f0 + PROFILE_AUDIO_MARGIN).max(PROFILE_MIN_FRAMES)
                    }));
            if done || steps >= 200_000_000 {
                break;
            }
            steps += 1;
            if m.bus.frames != last_frame {
                last_frame = m.bus.frames;
                m.bus.keys = demo_keys(game_frames);
                if cart_frame0.is_none() {
                    // Still in the BIOS logo: discard chime output so it
                    // can't trip the engagement detector.
                    m.bus.audio_buf.clear();
                    audio_base = None;
                } else if audio_from.is_none() && !m.bus.audio_buf.is_empty() {
                    // Engagement = output CHANGING, not merely nonzero
                    // (a constant bias level is still silence). The
                    // buffer must be drained: production self-caps
                    // when nothing consumes it.
                    let base = *audio_base.get_or_insert(m.bus.audio_buf[0]);
                    if m.bus.audio_buf.iter().any(|&s| s != base) {
                        audio_from = Some(game_frames);
                    }
                    m.bus.audio_buf.clear();
                }
                // The profiling slice of the bar; the endpoint moves
                // until audio engages, so estimate it (the report only
                // ever advances). The label says what we're waiting on.
                let est_end = match audio_from {
                    None => PROFILE_MAX_FRAMES,
                    Some(f0) => {
                        PROFILE_MAX_FRAMES.min((f0 + PROFILE_AUDIO_MARGIN).max(PROFILE_MIN_FRAMES))
                    }
                };
                let pct =
                    (game_frames * prof_end as u64 / est_end.max(1)).min(prof_end as u64) as u8;
                if pct > last_pct {
                    last_pct = pct;
                    let msg = match (audio_from, game_frames) {
                        (None, f) if f < 90 => "powering on the cartridge\u{2026}",
                        (None, _) => "running the intro, listening for the soundtrack\u{2026}",
                        (Some(_), _) => "soundtrack heard \u{2014} studying how it plays\u{2026}",
                    };
                    progress(pct, msg);
                }
            }
            // Seed observed control-transfer targets in IWRAM and ROM.
            // ROM targets recover code static traversal can't reach
            // (computed branches, handlers installed by RAM code) and
            // need no guard — ROM is immutable. IWRAM blocks are
            // content-guarded at execution time. EWRAM stays excluded
            // by default: it commonly holds streamed overlays, where
            // entry guards detect the swap but the hash-then-interpret
            // cycle is pure overhead; RECOMP_EWRAM opts in (resident
            // EWRAM engines, where the guard always passes).
            // Real-BIOS builds also seed observed BIOS entries: the
            // vectors + pointer sweep cover the static reachables, but
            // indirect returns (IntrWait re-entry, handler exits) are
            // only visible dynamically — and the BIOS is as immutable
            // as ROM, so they need no guard either.
            let seedable = |pc: u32| {
                pc >> 24 == 3
                    || (ewram_xlat && pc >> 24 == 2)
                    || (0x08..=0x0D).contains(&(pc >> 24))
                    || (bios.is_some() && pc < 0x4000)
            };
            let end = match m.step() {
                StepEvent::Instr(instr) => Some(instr.addr.wrapping_add(instr.size())),
                StepEvent::Idle => None,
            };
            let pc = m.cpu.regs[15];
            if pc != prev_end && seedable(pc) {
                seeds.insert(pc | m.cpu.thumb() as u32);
            }
            if let Some(e) = end {
                prev_end = e;
            }
        }
        println!(
            "profiled {} RAM entry points over {} frames",
            seeds.len(),
            m.bus.frames
        );
        progress(
            prof_end,
            &format!(
                "test drive done \u{2014} {} live code paths spotted",
                seeds.len()
            ),
        );
        (
            seeds.into_iter().collect::<Vec<u32>>(),
            m.bus.ewram.clone(),
            m.bus.iwram.clone(),
        )
    };

    // Label-file seeds: runtime-discovered entry points (recorded
    // play/runc sessions, community files) join the profile seeds.
    // ROM entries are immutable and unguarded; IWRAM entries translate
    // from the recorder's local content snapshot, overlaid onto the
    // profile snapshot, and run behind the whole-block content guards
    // like every RAM-resident block.
    let sha = rom_sha256(&rom);
    let mut seeds = seeds;
    let mut iwram = iwram;
    let lbl = translation_labels(&sha, label_source.labels(&sha, rom.len())?);
    if !lbl.is_empty() {
        let mut set: std::collections::BTreeSet<u32> = seeds.iter().copied().collect();
        let before = set.len();
        set.extend(&lbl.rom);
        // RECOMP_EXHAUSTIVE (full-recomp packaging): a soak proves the
        // covered paths, not all paths — computed jump-table targets
        // inside a function appear only when play reaches them. With
        // complete boundaries (`end` records from a mapper-grade set),
        // every decode point inside every mapped function becomes a
        // seed, so any computed target the game can ever take has a
        // native block. Pool words inside ranges translate to junk
        // blocks nothing dispatches to — size, not correctness.
        if exhaustive {
            let mut dense = 0usize;
            for (&key, &end) in &lbl.ends {
                if !(0x08..=0x0D).contains(&(key >> 24)) {
                    continue;
                }
                let step = if key & 1 != 0 { 2 } else { 4 };
                let thumb = key & 1;
                let mut a = key & !1;
                while a + step <= end {
                    set.insert(a | thumb);
                    a += step;
                }
                dense += 1;
            }
            println!("labels: exhaustive in-function seeding over {dense} bounded functions");
        }
        let rom_added = set.len() - before;
        let mut iw_added = 0usize;
        let mut iw_unbacked = 0usize;
        if !lbl.iwram.is_empty() {
            match labels::Blob::load(&labels::blob_path(&sha)) {
                Some(blob) => {
                    if iwram.is_empty() {
                        iwram = vec![0; labels::IWRAM_LEN];
                    }
                    for i in 0..labels::IWRAM_LEN {
                        if blob.mask[i] != 0 {
                            iwram[i] = blob.img[i];
                        }
                    }
                    for &key in &lbl.iwram {
                        if blob.valid_at(key) && set.insert(key) {
                            iw_added += 1;
                        }
                    }
                }
                None => iw_unbacked = lbl.iwram.len(),
            }
        }
        seeds = set.into_iter().collect();
        println!("labels: +{rom_added} rom, +{iw_added} iwram seeds");
        if iw_unbacked != 0 {
            println!(
                "labels: {iw_unbacked} iwram entries lack a local snapshot — \
run play/runc --record-labels to capture one"
            );
        }
    }

    // Real-BIOS translation seeds: the exception vectors, plus a sweep of
    // every aligned word in the image that looks like an in-BIOS code
    // pointer — the SWI dispatcher reaches its handlers through an
    // address table (an indexed load, not a PC-relative literal), which
    // recursive traversal alone cannot follow. Junk seeds are harmless:
    // unreached blocks just bloat the output (translate-everything).
    if let Some(b) = bios {
        seeds.extend([0x00u32, 0x08, 0x18]);
        let mut swept = 0usize;
        for off in (0..b.len() - 3).step_by(4) {
            let v = u32::from_le_bytes(b[off..off + 4].try_into().unwrap());
            let t = v & !1;
            if v != 0 && t < 0x4000 && (v & 1 == 1 || v & 3 == 0) {
                seeds.push(if v & 1 == 1 { t | 1 } else { t });
                swept += 1;
            }
        }
        println!("bios: seeded 3 vectors + {swept} swept pointer words");
    }

    let view = analyze::View {
        rom: &rom,
        ewram: if ram && ewram_xlat {
            Some(&ewram)
        } else {
            None
        },
        iwram: if !iwram.is_empty() {
            Some(&iwram)
        } else {
            None
        },
        bios,
    };
    // Hand the assembled view + seeds to the engine: analyze → emit C →
    // compile each unit → link. It emits the `blocks:`/`wrote` report lines
    // on stdout (the packager's contract) at the same points the inline
    // pipeline did.
    let cc = Compiler::detect();
    eprintln!("compiler: {}", cc.describe());
    build_library(&view, &seeds, &cc, Path::new(lib_path), prof_end, progress)?;
    Ok(())
}

type BlockFn = extern "C" fn(*const gba_core::capi::RtApi, *mut std::ffi::c_void) -> u32;

/// Direct-indexed block lookup. Keys are `guest address | thumb bit`;
/// nearly all live in the ROM window (and IWRAM for --ram builds), so
/// those get dense tables — lookup is a subtract, compare, and load —
/// with a HashMap holding any stragglers.
struct BlockTable {
    rom: Vec<Option<BlockFn>>,
    iwram: Vec<Option<BlockFn>>,
    /// Dense EWRAM table, allocated only when an EWRAM block exists
    /// (experimental RECOMP_EWRAM builds).
    ewram: Vec<Option<BlockFn>>,
    /// Dense BIOS table (region 0), allocated only for --bios builds.
    bios: Vec<Option<BlockFn>>,
    other: std::collections::HashMap<u32, BlockFn>,
    len: usize,
}

const IWRAM_BASE: u32 = 0x0300_0000;
const EWRAM_BASE: u32 = 0x0200_0000;

impl BlockTable {
    fn load(lib: &libloading::Library) -> Result<BlockTable, String> {
        use gba_core::capi::RcgBlock;
        let blocks: &[RcgBlock] = unsafe {
            let blocks: libloading::Symbol<*const RcgBlock> =
                lib.get(b"rcg_blocks").map_err(|e| e.to_string())?;
            let count: libloading::Symbol<*const u64> =
                lib.get(b"rcg_block_count").map_err(|e| e.to_string())?;
            std::slice::from_raw_parts(*blocks, **count as usize)
        };
        Self::from_blocks(blocks)
    }

    fn from_blocks(blocks: &[gba_core::capi::RcgBlock]) -> Result<BlockTable, String> {
        let mut rom_max = 0usize;
        for b in blocks {
            let r = b.key.wrapping_sub(ROM_BASE) as usize;
            if r < 0x0200_0000 {
                rom_max = rom_max.max(r + 1);
            }
        }
        let any_ewram = blocks
            .iter()
            .any(|b| (b.key.wrapping_sub(EWRAM_BASE) as usize) < 0x4_0000);
        let any_bios = blocks.iter().any(|b| b.key < 0x4000);
        let mut t = BlockTable {
            rom: vec![None; rom_max],
            iwram: vec![None; 0x8000],
            ewram: vec![None; if any_ewram { 0x4_0000 } else { 0 }],
            bios: vec![None; if any_bios { 0x4000 } else { 0 }],
            other: std::collections::HashMap::new(),
            len: blocks.len(),
        };
        // Diagnostic: RECOMP_IWRAM_MAX=<hex> drops IWRAM natives above
        // the threshold at load time (cheap block bisection, no rebuild).
        let iwram_max = std::env::var("RECOMP_IWRAM_MAX")
            .ok()
            .and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok());
        for b in blocks {
            let r = b.key.wrapping_sub(ROM_BASE) as usize;
            let w = b.key.wrapping_sub(IWRAM_BASE) as usize;
            if (b.key as usize) < t.bios.len() {
                t.bios[b.key as usize] = Some(b.func);
            } else if r < t.rom.len() {
                t.rom[r] = Some(b.func);
            } else if w < t.iwram.len() {
                if let Some(mx) = iwram_max {
                    if b.key & !1 > mx {
                        continue;
                    }
                }
                t.iwram[w] = Some(b.func);
            } else {
                let e = b.key.wrapping_sub(EWRAM_BASE) as usize;
                if e < t.ewram.len() {
                    t.ewram[e] = Some(b.func);
                } else {
                    t.other.insert(b.key, b.func);
                }
            }
        }
        Ok(t)
    }

    #[inline(always)]
    fn get(&self, key: u32) -> Option<BlockFn> {
        let r = key.wrapping_sub(ROM_BASE) as usize;
        if r < self.rom.len() {
            return self.rom[r];
        }
        let w = key.wrapping_sub(IWRAM_BASE) as usize;
        if w < self.iwram.len() {
            return self.iwram[w];
        }
        let e = key.wrapping_sub(EWRAM_BASE) as usize;
        if e < self.ewram.len() {
            return self.ewram[e];
        }
        if (key as usize) < self.bios.len() {
            return self.bios[key as usize];
        }
        if self.other.is_empty() {
            None
        } else {
            self.other.get(&key).copied()
        }
    }
}

/// Drive the machine to the next completed frame using translated blocks
/// where available and the interpreter elsewhere, with interrupt
/// machinery serviced at block boundaries — the same execution sequence
/// as the original whole-run dispatch loop, bounded per frame. Returns
/// (native blocks run, fallback steps); the frame is incomplete only if
/// `max_steps` was exhausted (the caller's stall guard).
fn run_frame_native(
    m: &mut Machine,
    table: &BlockTable,
    mptr: *mut std::ffi::c_void,
    max_steps: u64,
) -> (u64, u64) {
    run_frame_native_observed(m, table, mptr, max_steps, |_, _, _| {})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeDispatch {
    Native = 0,
    Missing = 1,
    System = 2,
    Hook = 3,
}

/// The cost census observes the same dispatch sequence as normal execution.
/// The no-op specialization removes observation work from the release hot path.
fn run_frame_native_observed(
    m: &mut Machine,
    table: &BlockTable,
    mptr: *mut std::ffi::c_void,
    max_steps: u64,
    mut observe: impl FnMut(NativeDispatch, u32, u64),
) -> (u64, u64) {
    use gba_core::capi::RT_API;
    m.bus.frame_ready = false;
    let mut native = 0u64;
    let mut fallback = 0u64;
    let mut steps = 0u64;
    // Diagnostics are configured for the frame, not for each guest block.
    // In particular, querying the Windows environment used to add an
    // environment lookup to every successful native dispatch.
    let trace_iwram = std::env::var_os("RECOMP_TRACE_IWRAM").is_some();
    let trace_fallback = trace_fallback();
    // Fallback-entry detection state (RECOMP_TRACE_FALLBACK): end address
    // of the previous fallback instruction; MAX = last step wasn't a
    // straight-line fallback continuation.
    let mut fb_prev_end = u32::MAX;
    while !m.bus.frame_ready && steps < max_steps {
        steps += 1;
        let observed_pc = m.cpu.regs[15];
        let observed_clock = m.bus.clock;
        // Interrupt machinery and sleep states go through Machine::step.
        // (The IRQ-return-stub check is HLE-only: under a real BIOS that
        // address is ordinary BIOS code.)
        if m.bus.halted
            || (!m.bus.real_bios
                && m.cpu.regs[15] == gba_core::machine::IRQ_RETURN_ADDR
                && m.cpu.mode() == gba_core::Mode::Irq)
            || (m.bus.irq_pending() && !m.cpu.flag(gba_core::cpu::FLAG_I))
        {
            m.step();
            observe(
                NativeDispatch::System,
                observed_pc,
                m.bus.clock - observed_clock,
            );
            continue;
        }
        let key = m.cpu.regs[15] | m.cpu.thumb() as u32;
        m.ffta_log_sprite_entry(key);
        if m.bus.ffta_battle_mouse.interprets(m.cpu.regs[15])
            || m.ffta_sprite_interpret(m.cpu.regs[15])
        {
            m.step();
            observe(
                NativeDispatch::Hook,
                observed_pc,
                m.bus.clock - observed_clock,
            );
            continue;
        }
        // Real-BIOS mode: native blocks bypass the interpreter's fetch
        // hook, so maintain the executing-in-BIOS flag at dispatch
        // (region-0 reads return real bytes only while it is set).
        if m.bus.real_bios {
            m.bus.pc_in_bios = key < 0x4000;
        }
        // MP2K HLE hook at block granularity (SoundMainRAM is a call
        // target, so its entry is always a dispatch point). The
        // interpreter fallback re-checks inside step(); the hook is
        // idempotent within a tick.
        if let Some(h) = m.bus.mp2k.as_deref() {
            if h.active && h.hook_match(key) {
                m.bus.mp2k_frame_hook(key);
            }
        }
        if let Some(g) = m.bus.gax.as_deref() {
            if g.hook_match(key) {
                let r0 = m.cpu.regs[0];
                m.bus.gax_frame_hook(key, r0);
            }
        }
        if let Some(rr) = m.bus.rdrv.as_deref() {
            if rr.hook_match(key) {
                m.bus.rdrv_frame_hook(key);
            }
        }
        let kind = match table.get(key) {
            Some(f) => {
                // Diagnostic: census of executed IWRAM natives.
                if trace_iwram && key >> 24 == 3 {
                    IWRAM_HITS.with(|h| *h.borrow_mut().entry(key).or_insert(0u64) += 1);
                }
                f(&RT_API, mptr);
                native += 1;
                fb_prev_end = u32::MAX;
                NativeDispatch::Native
            }
            None => {
                if NO_INTERP.load(std::sync::atomic::Ordering::Relaxed) {
                    eprintln!(
                        "DEFECT: full recomp missing native code at {:08x} ({}) — this \
code path is not covered by the package's translation. Report this address to the \
packager (it becomes a label; the rebuilt package covers it).",
                        key & !1,
                        if key & 1 != 0 { "thumb" } else { "arm" },
                    );
                    std::process::exit(2);
                }
                // Diagnostic (RECOMP_TRACE_FALLBACK): census of fallback
                // *entries* — the dispatch keys that would be labels.
                // Straight-line continuation inside a fallback run is not
                // an entry; the same discontinuity filter as the build
                // profiler applies.
                if trace_fallback {
                    if key & !1 != fb_prev_end {
                        let new = FALLBACK_ENTRIES.with(|h| {
                            let mut h = h.borrow_mut();
                            let len0 = h.len();
                            *h.entry(key).or_insert(0u64) += 1;
                            h.len() != len0
                        });
                        if new
                            && key >> 24 == 3
                            && FALLBACK_COLLECT.load(std::sync::atomic::Ordering::Relaxed)
                        {
                            capture_iwram(&m.bus.iwram, key);
                        }
                    }
                    fb_prev_end = match m.step() {
                        StepEvent::Instr(i) => i.addr.wrapping_add(i.size()),
                        StepEvent::Idle => u32::MAX,
                    };
                } else {
                    m.step();
                }
                fallback += 1;
                NativeDispatch::Missing
            }
        };
        observe(kind, observed_pc, m.bus.clock - observed_clock);
    }
    (native, fallback)
}

/// Per-frame stall guard for the native loop: generous beyond any real
/// frame (a frame is ~280K cycles) while still bounding a wedged run.
const FRAME_STEP_GUARD: u64 = 200_000_000;

thread_local! {
    /// Diagnostic (RECOMP_TRACE_IWRAM): executed-IWRAM-native census.
    static IWRAM_HITS: std::cell::RefCell<std::collections::HashMap<u32, u64>> =
        std::cell::RefCell::new(std::collections::HashMap::new());

    /// Diagnostic (RECOMP_TRACE_FALLBACK): fallback-entry census —
    /// dispatch keys (addr | thumb) that missed the block table, with
    /// hit counts. Entries only, not straight-line continuations.
    static FALLBACK_ENTRIES: std::cell::RefCell<std::collections::HashMap<u32, u64>> =
        std::cell::RefCell::new(std::collections::HashMap::new());

    /// --record-labels: IWRAM content accumulated at the moment each
    /// new IWRAM entry point is discovered (the code is certainly live
    /// right then; an end-of-session snapshot could miss swapped
    /// overlays). Saved as the image's local snapshot blob.
    static IWRAM_CAP: std::cell::RefCell<Option<Box<labels::Blob>>> =
        std::cell::RefCell::new(None);
}

/// Fold the live IWRAM into the capture accumulator: first capture
/// fills everything; later new-entry events force-refresh a window
/// around the new entry (its code is live NOW; elsewhere, first-seen
/// content stands and the runtime guards reject anything stale).
fn capture_iwram(iwram: &[u8], entry: u32) {
    IWRAM_CAP.with(|c| {
        let mut c = c.borrow_mut();
        let blob = c.get_or_insert_with(|| Box::new(labels::Blob::new()));
        let n = iwram.len().min(labels::IWRAM_LEN);
        for i in 0..n {
            if blob.mask[i] == 0 {
                blob.img[i] = iwram[i];
                blob.mask[i] = 1;
            }
        }
        let at = (entry & !1) as usize & (labels::IWRAM_LEN - 1);
        let (lo, hi) = (at.saturating_sub(64), (at + 2048).min(n));
        blob.img[lo..hi].copy_from_slice(&iwram[lo..hi]);
        blob.mask[lo..hi].fill(1);
    });
}

/// Census collection runs under RECOMP_TRACE_FALLBACK (prints at exit)
/// or --record-labels (persists entries as a label file).
static FALLBACK_COLLECT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Full-recomp mode (packaged with `interpreter = false`): a dispatch
/// miss halts loudly instead of interpreting. The package was gated on
/// a zero-fallback soak at build time, so reaching this is a coverage
/// defect the packager needs to hear about, never something to play
/// through silently at degraded fidelity.
static NO_INTERP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn trace_fallback() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RECOMP_TRACE_FALLBACK").is_some())
        || FALLBACK_COLLECT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Persist this run's fallback census as labels: ROM entries become
/// analyzer seeds on the next build; IWRAM/EWRAM entries are recorded
/// as reserved presence records (not yet translatable). Union-merges
/// into the per-image accumulator under the config dir.
fn record_labels(rom_len: usize, sha: &str) -> Result<(), String> {
    let mut new = labels::Labels::default();
    FALLBACK_ENTRIES.with(|h| {
        for (&key, _) in h.borrow().iter() {
            match key >> 24 {
                0x08..=0x0D if ((key & !1 & 0x01FF_FFFF) as usize) < rom_len => {
                    new.rom.insert(key);
                }
                0x03 => {
                    new.iwram.insert(key);
                }
                0x02 => new.reserved.push(format!(
                    "ewram {:08x} {}",
                    key & !1,
                    if key & 1 != 0 { "t" } else { "a" }
                )),
                _ => {}
            }
        }
    });
    let path = labels::config_path(sha);
    let mut all = match path.is_file() {
        true => labels::Labels::load(&path, sha, rom_len)?,
        false => labels::Labels::default(),
    };
    let (rom0, iw0) = (all.rom.len(), all.iwram.len());
    all.merge(new);
    all.save(&path, sha)?;
    // Persist the IWRAM content captured at entry discovery: this
    // session's bytes override prior sessions where captured (they
    // reflect the overlays actually seen; guards reject the rest).
    IWRAM_CAP.with(|c| -> Result<(), String> {
        let Some(cap) = c.borrow_mut().take() else {
            return Ok(());
        };
        let bp = labels::blob_path(sha);
        let mut blob = labels::Blob::load(&bp).unwrap_or_else(labels::Blob::new);
        for i in 0..labels::IWRAM_LEN {
            if cap.mask[i] != 0 {
                blob.img[i] = cap.img[i];
                blob.mask[i] = 1;
            }
        }
        blob.save(&bp)?;
        eprintln!(
            "labels: iwram snapshot updated ({} bytes valid)",
            blob.valid_bytes()
        );
        Ok(())
    })?;
    eprintln!(
        "labels: +{} rom, +{} iwram entries recorded ({} rom, {} iwram total) -> {}",
        all.rom.len() - rom0,
        all.iwram.len() - iw0,
        all.rom.len(),
        all.iwram.len(),
        path.display()
    );
    if all.rom.len() > rom0 || all.iwram.len() > iw0 {
        eprintln!("labels: the next translation rebuild covers the new entries");
    }
    Ok(())
}

/// Label-file tooling: inspect, import a shared file, export for
/// sharing. The local IWRAM snapshot is shown but never exported.
fn cmd_labels(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: recomp labels <show|import|export> <rom> [file]";
    let sub = args.first().ok_or(USAGE)?.as_str();
    let rom_path = args.get(1).ok_or(USAGE)?;
    let rom = std::fs::read(rom_path).map_err(|e| format!("{rom_path}: {e}"))?;
    let sha = rom_sha256(&rom);
    match sub {
        "show" => {
            let all = labels::load_all(rom_path, &sha, rom.len());
            println!("image {sha}");
            println!(
                "labels: {} rom, {} iwram, {} named, {} reserved (ewram)",
                all.rom.len(),
                all.iwram.len(),
                all.names.len(),
                all.reserved.len()
            );
            match labels::Blob::load(&labels::blob_path(&sha)) {
                Some(b) => println!(
                    "iwram snapshot: {} bytes valid ({})",
                    b.valid_bytes(),
                    labels::blob_path(&sha).display()
                ),
                None => println!("iwram snapshot: none (run play/runc --record-labels)"),
            }
            if !all.is_empty() {
                let blob = labels::Blob::load(&labels::blob_path(&sha));
                let d = all.digest(blob.as_ref());
                println!("cache key suffix: -l{:08x}", d as u32 ^ (d >> 32) as u32);
            }
            Ok(())
        }
        "import" => {
            let file = args.get(2).ok_or("import needs a label file")?;
            let incoming = labels::Labels::load(std::path::Path::new(file), &sha, rom.len())?;
            // Enriched (named) sets go to the TOML accumulator — the v1
            // file can't carry names; address-only sets keep using it.
            let enriched = !incoming.names.is_empty() || !incoming.ends.is_empty();
            let path = if enriched {
                labels::config_toml_path(&sha)
            } else {
                labels::config_path(&sha)
            };
            let mut all = match path.is_file() {
                true => labels::Labels::load(&path, &sha, rom.len())?,
                false => labels::Labels::default(),
            };
            let (r0, i0) = (all.rom.len(), all.iwram.len());
            all.merge(incoming);
            if enriched {
                all.save_toml(&path, &sha)?;
            } else {
                all.save(&path, &sha)?;
            }
            println!(
                "imported: +{} rom, +{} iwram ({} rom, {} iwram total) -> {}",
                all.rom.len() - r0,
                all.iwram.len() - i0,
                all.rom.len(),
                all.iwram.len(),
                path.display()
            );
            if all.iwram.len() > 0 && labels::Blob::load(&labels::blob_path(&sha)).is_none() {
                println!(
                    "note: iwram entries activate after a local --record-labels session \
captures their content"
                );
            }
            Ok(())
        }
        "export" => {
            let default = format!("{}.labels.toml", rom_path.trim_end_matches(".gba"));
            let out = args.get(2).cloned().unwrap_or(default);
            let all = labels::load_all(rom_path, &sha, rom.len());
            if all.is_empty() && all.reserved.is_empty() {
                return Err("nothing to export — record some labels first".into());
            }
            // The extension picks the format: .toml = v2 interchange
            // (names/ends preserved), anything else = v1 lines.
            if out.ends_with(".toml") {
                all.save_toml(std::path::Path::new(&out), &sha)?;
            } else {
                all.save(std::path::Path::new(&out), &sha)?;
            }
            println!(
                "exported {} rom + {} iwram entries ({} named) -> {out} (addresses and \
names only; the local iwram snapshot is never exported)",
                all.rom.len(),
                all.iwram.len(),
                all.names.len()
            );
            Ok(())
        }
        other => Err(format!("unknown labels subcommand {other:?}; {USAGE}")),
    }
}

/// Single-line terminal build progress: bar, percent, phase label.
fn term_progress(pct: u8, msg: &str) {
    use std::io::Write;
    let filled = pct as usize * 28 / 100;
    eprint!(
        "\r\x1b[K  [{}{}] {pct:>3}%  {msg}",
        "\u{25a0}".repeat(filled),
        "\u{00b7}".repeat(28 - filled)
    );
    let _ = std::io::stderr().flush();
}

/// SHA-256 of the image, hex — the cross-tool image identity.
fn rom_sha256(rom: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(rom)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Dump the fallback-entry census: per-region totals plus the hottest
/// entries. Resident code shows as few entries with huge counts;
/// streamed overlays as many entries with small counts.
fn print_fallback_census() {
    if !trace_fallback() {
        return;
    }
    let region_name = |key: u32| match key >> 24 {
        0 => "bios",
        2 => "ewram",
        3 => "iwram",
        8..=0xD => "rom",
        _ => "other",
    };
    FALLBACK_ENTRIES.with(|h| {
        let h = h.borrow();
        let mut per: std::collections::BTreeMap<&str, (u64, u64)> = Default::default();
        for (&k, &n) in h.iter() {
            let e = per.entry(region_name(k)).or_insert((0, 0));
            e.0 += 1;
            e.1 += n;
        }
        eprintln!("FALLBACK census: {} distinct entries", h.len());
        for (r, (distinct, hits)) in &per {
            eprintln!("  {r:>5}: {distinct} entries, {hits} hits");
        }
        let mut v: Vec<(u32, u64)> = h.iter().map(|(&k, &n)| (k, n)).collect();
        v.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        for (k, n) in v.iter().take(25) {
            eprintln!(
                "  {} {:08x} {} {n}",
                region_name(*k),
                k & !1,
                if k & 1 != 0 { "t" } else { "a" }
            );
        }
    });
}

/// Run a recompiled image: translated blocks where available, interpreter
/// fallback elsewhere, interrupt machinery at block boundaries.
fn cmd_runc(args: &[String]) -> Result<(), String> {
    configure_headless_widescreen();
    let zoom_schedule = HeadlessZoom::from_env(gba_core::mem::extra())?;
    let verify_frames = std::env::var("FFTA_PATCH_VERIFY_FRAMES").as_deref() == Ok("1");
    let verify_state = std::env::var("FFTA_BENCH_STATE").as_deref() == Ok("1");
    let mut frame_times = perf::FrameTimes::new(
        std::env::var("FFTA_BENCH_FRAME_TIMES").as_deref() == Ok("1"),
    );
    let mut ppu_profile = perf::PpuProfile::new(
        std::env::var("FFTA_BENCH_PPU").as_deref() == Ok("1"),
    );
    let mut rom_path = None;
    let mut use_rom_data = false;
    let mut frames = 600u64;
    let mut out: Option<String> = None;
    let mut input: Option<InputScript> = None;
    let mut sav: Option<String> = None;
    let mut state: Option<String> = None;
    let mut bios: Option<Vec<u8>> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--rom-data" => use_rom_data = true,
            "--frames" => {
                frames = it
                    .next()
                    .ok_or("--frames needs a value")?
                    .parse()
                    .map_err(|e| format!("bad frames: {e}"))?
            }
            "--out" => out = Some(it.next().ok_or("--out needs a value")?.to_string()),
            "--input" => {
                input = Some(InputScript::load(
                    it.next().ok_or("--input needs a value")?,
                )?)
            }
            "--record-labels" => FALLBACK_COLLECT.store(true, std::sync::atomic::Ordering::Relaxed),
            "--sav" => sav = Some(it.next().ok_or("--sav needs a value")?.to_string()),
            "--state" => state = Some(it.next().ok_or("--state needs a value")?.to_string()),
            "--bios" => bios = Some(load_bios_file(it.next().ok_or("--bios needs a value")?)?),
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    if state.is_some() && sav.is_some() {
        return Err("--state and --sav are mutually exclusive".into());
    }
    // Avoid writing a diagnostic image over a supplied snapshot, including symlinks.
    if let (Some(state), Some(out)) = (&state, &out) {
        if std::fs::canonicalize(state).ok().is_some_and(|source| {
            std::fs::canonicalize(out).ok().as_ref() == Some(&source)
        }) {
            return Err("--out would overwrite the supplied --state file".into());
        }
    }
    let rom_path = rom_path.ok_or("missing ROM path")?;
    let rom = std::fs::read(&rom_path).map_err(|e| format!("{rom_path}: {e}"))?;
    let stem = Path::new(&rom_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("game")
        .to_string();
    let suffix = if bios.is_some() { "-bios" } else { "" };
    let lib_path = format!("out/{stem}{suffix}.{LIB_EXT}");
    let lib_path = std::env::var("FFTA_BENCH_LIBRARY").unwrap_or(lib_path);

    let (_lib, table) = load_native(Path::new(&lib_path))?;
    println!("loaded {} translated blocks", table.len);

    if gba_core::ffta_log::enabled() {
        println!(
            "FFTA_REPLAY_NATIVE_PRODUCERS first={} second={} battle={} other={}",
            table.get(0x080014c9).is_some(),
            table.get(0x080016d5).is_some(),
            table.get(0x030065a9).is_some(),
            table.get(0x030068a5).is_some()
        );
    }

    let mut m = make_machine(rom, bios.as_deref());
    if use_rom_data { rom_data::prepare_and_attach(&mut m, false)?; }
    if let Some(path) = sav {
        let data = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
        println!(
            "FFTA_REPLAY_SAVE path={path} bytes={} sha256={}",
            data.len(),
            gba_core::ffta_rom::digest(&data)
        );
        m.bus.load_save_data(&data);
    }
    // RECOMP_MP2K=1: arm the HLE shadow under native dispatch — the
    // hook-at-block-boundary path play uses, validated headless here.
    if std::env::var_os("RECOMP_MP2K").is_some() {
        m.bus.tap_channels = true;
        eprintln!("hle: {}", arm_audio_hle(&mut m, None));
    }
    if let Some(path) = state {
        save_state::load(&mut m, Path::new(&path))?;
        println!("FFTA_REPLAY_STATE frame={} machine_sha256={}",
            m.bus.frames, gba_core::ffta_rom::digest(&m.save_state()?));
    }
    // Preserve guest frame/clock values: only the replay controls use an offset.
    let replay_first = m.bus.frames;
    let replay_end = replay_first.checked_add(frames).ok_or("replay frame range overflow")?;
    let mptr = &mut m as *mut Machine as *mut std::ffi::c_void;

    let mut native_blocks = 0u64;
    let mut fallback_steps = 0u64;
    let mut edge_capture = FftaEdgeCapture::new();
    let bench_from = std::env::var("FFTA_BENCH_FROM")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|offset| replay_first.checked_add(offset).ok_or("benchmark frame range overflow"))
        .transpose()?;
    // Benchmark-only common warm-up: reach the same guest scene before enabling
    // the requested sprite setting. Normal play/runc behaviour is unchanged.
    let mut character_csv=std::env::var("FFTA_CHARACTER_CSV").ok().map(std::fs::File::create).transpose().map_err(|e|e.to_string())?;
    let character_from=11340u64;let character_to=12540u64;
    if let Some(file)=character_csv.as_mut(){use std::io::Write;writeln!(file,"frame,roster_valid,present,drawn,present_ids,drawn_ids,duplicates,missing").map_err(|e|e.to_string())?;}
    let bench_sprite = m.bus.ffta_sprites.enabled;
    if bench_from.is_some() {
        m.bus.ffta_sprites.enabled = false;
    }
    let mut bench_start = None;
    // Diagnostic (RECOMP_COST_FROM=N): from frame N on, attribute charged
    // cycles to the dispatch key (block entry or interp PC) that incurred
    // them; dump the histogram at exit.
    let cost_from: Option<u64> = std::env::var("RECOMP_COST_FROM")
        .ok()
        .and_then(|v| v.parse().ok());
    let mut cost: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    let mut cost_dispatches = [0u64; 4];
    while m.bus.frames < replay_end {
        let before = m.bus.frames;
        if character_csv.is_some(){gba_core::ffta_sprites::set_character_metrics(before+1>=character_from&&before+1<=character_to);}
        if let Some(zoom) = &zoom_schedule {
            zoom.apply(before - replay_first);
        }
        if bench_start.is_none() && bench_from.is_some_and(|from| before >= from) {
            let mut scene = m.bus.ewram.clone();
            scene.extend_from_slice(&m.bus.iwram);
            scene.extend_from_slice(&m.bus.io);
            scene.extend_from_slice(&m.bus.oam);
            println!(
                "FFTA_BENCH_SCENE frame={before} guest_sha256={} sprite_requested={bench_sprite}",
                gba_core::ffta_rom::digest(&scene)
            );
            m.bus.ffta_sprites.enabled = bench_sprite;
            bench_start = Some((before, std::time::Instant::now()));
        }
        if let Some(s) = &input {
            m.bus.keys = s.keys_at(before - replay_first);
            s.apply_mouse(&mut m.bus,before - replay_first)?;
        }
        let frame_start = frame_times.start(bench_from.is_none_or(|from| before >= from));
        let ppu_capture = gba_core::ppu_profile::Capture::start_detailed(
            ppu_profile.enabled() && bench_from.is_none_or(|from| before >= from),
            ppu_profile.detailed(),
        );
        let (n, f) = if cost_from.is_some_and(|n| m.bus.frames >= n) {
            run_frame_native_observed(
                &mut m,
                &table,
                mptr,
                FRAME_STEP_GUARD,
                |kind, pc, cycles| {
                    *cost.entry(pc).or_insert(0) += cycles;
                    cost_dispatches[kind as usize] += 1;
                },
            )
        } else {
            run_frame_native(&mut m, &table, mptr, FRAME_STEP_GUARD)
        };
        native_blocks += n;
        fallback_steps += f;
        let ppu_timings = ppu_capture.finish();
        frame_times.finish(frame_start);
        if m.bus.frames == before {
            return Err("step guard exceeded before a complete frame".into());
        }
        ppu_profile.record(ppu_timings);
        if let Some(capture) = edge_capture.as_mut() {
            capture.sample(&m);
        }
        gba_core::ffta_sprites::trace_frame(&m.bus);
        gba_core::ffta_battle_mouse::verification(&m.bus);
        if (character_from..=character_to).contains(&m.bus.frames){if let Some(file)=character_csv.as_mut(){use std::io::Write;writeln!(file,"{}",gba_core::ffta_sprites::tutorial_metric(&m.bus)).map_err(|e|e.to_string())?;}}
        movement_replay_observation(&m)?;
        if verify_frames {
            println!(
                "FFTA_EQ frame={} framebuffer={} oam={}",
                m.bus.frames,
                gba_core::ffta_rom::digest(
                    &m.bus
                        .framebuffer
                        .iter()
                        .flat_map(|p| p.to_le_bytes())
                        .collect::<Vec<_>>()
                ),
                gba_core::ffta_rom::digest(&m.bus.oam)
            );
        }
    }
    if !cost.is_empty() {
        eprintln!("COST_DISPATCH native={} missing={} system={} hook={}",
            cost_dispatches[0], cost_dispatches[1], cost_dispatches[2], cost_dispatches[3]);
        let mut v: Vec<(u32, u64)> = cost.into_iter().collect();
        v.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
        let total: u64 = v.iter().map(|&(_, c)| c).sum();
        eprintln!("COST total={total} cycles over traced frames; top PCs:");
        for (pc, c) in v.iter().take(40) {
            eprintln!("  COST {pc:08x} {c}");
        }
    }

    if let Some((first, start)) = bench_start {
        let elapsed = start.elapsed().as_secs_f64();
        let measured = m.bus.frames - first;
        println!("FFTA_BENCH first={first} last={} frames={measured} seconds={elapsed:.6} ms_per_frame={:.6} width={}",m.bus.frames,elapsed*1000.0/measured as f64,gba_core::mem::render_width());
    }
    frame_times.report();
    ppu_profile.report();
    // Snapshot hashing is opt-in and outside the measured interval. It covers
    // serialized CPU/hardware/save state, unlike a final framebuffer hash.
    if verify_state {
        println!("FFTA_FINAL_STATE sha256={}", gba_core::ffta_rom::digest(&m.save_state()?));
    }
    gba_core::ppu::dump_ffta_text_layers(&m.bus)?;

    if let Some(capture) = edge_capture {
        capture.finish()?;
    }
    gba_core::ffta_sprites::verification(&m.bus);
    // FNV-1a framebuffer hash — must match the interpreter's `frames` run.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &px in &m.bus.framebuffer {
        for b in px.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x1_0000_01b3);
        }
    }
    println!(
        "frames: {} hash: {hash:016x} native_blocks: {native_blocks} fallback_steps: {fallback_steps}",
        m.bus.frames
    );
    if let Some(h) = m.bus.mp2k.as_deref() {
        let (corr, ratio) = h.last_correlation();
        eprintln!(
            "mp2k: hooks={} stale={} bad_waves={} corr={corr:.3} ratio={ratio:.2} gain={:.2} mode={} pauses={} proven={} engaged={} active={}{}",
            h.hooks,
            h.stale_ticks,
            h.bad_waves,
            h.gain(),
            h.count_mode(),
            h.vf.pauses,
            h.vf.proven,
            h.engaged,
            h.active,
            h.vf.reverted
                .as_deref()
                .map(|m| format!(" PAUSED: {m}"))
                .unwrap_or_default()
        );
    }
    if let Some(g) = m.bus.gax.as_deref() {
        let (corr, ratio) = g.last_correlation();
        eprintln!(
            "gax: hooks={} stale={} bad_waves={} corr={corr:.3} ratio={ratio:.2} gain={:.2} pauses={} proven={} engaged={} active={}{}",
            g.hooks,
            g.stale_ticks,
            g.bad_waves,
            g.gain(),
            g.vf.pauses,
            g.vf.proven,
            g.engaged,
            g.active,
            g.vf.reverted
                .as_deref()
                .map(|m| format!(" PAUSED: {m}"))
                .unwrap_or_default()
        );
    }
    if let Some(r) = m.bus.rdrv.as_deref() {
        let (corr, ratio) = r.last_correlation();
        eprintln!(
            "rdrv: hooks={} stale={} bad_waves={} corr={corr:.3} ratio={ratio:.2} gain={:.2} pauses={} proven={} engaged={} active={}{}",
            r.hooks,
            r.stale_ticks,
            r.bad_waves,
            r.gain(),
            r.vf.pauses,
            r.vf.proven,
            r.engaged,
            r.active,
            r.vf.reverted
                .as_deref()
                .map(|m| format!(" PAUSED: {m}"))
                .unwrap_or_default()
        );
    }
    if std::env::var_os("RECOMP_TRACE_IWRAM").is_some() {
        IWRAM_HITS.with(|h| {
            let mut v: Vec<_> = h.borrow().iter().map(|(&k, &n)| (k, n)).collect();
            v.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            for (k, n) in v.iter().take(30) {
                println!("iwram native {k:08x}: {n}");
            }
            println!("iwram natives executed: {}", v.len());
        });
    }
    print_fallback_census();
    if FALLBACK_COLLECT.load(std::sync::atomic::Ordering::Relaxed) {
        record_labels(m.bus.rom.len(), &rom_sha256(&m.bus.rom))?;
    }
    dump_frame(&m, out)?;
    rom_data::report_usage(&m);
    Ok(())
}

/// Differential verification: run N frames interpreted and N frames
/// recompiled (building if needed) and compare framebuffer hashes.
fn cmd_verify(args: &[String]) -> Result<(), String> {
    let mut rom_path = None;
    let mut frames = 600u64;
    let mut reuse = false;
    let mut dump: Option<String> = None;
    let mut input: Option<String> = None;
    let mut bios_path: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--frames" => {
                frames = it
                    .next()
                    .ok_or("--frames needs a value")?
                    .parse()
                    .map_err(|e| format!("bad frames: {e}"))?
            }
            // Triage helpers: --reuse skips the rebuild when the out/<stem> library
            // already exists; --dump <prefix> writes both final frames as
            // <prefix>.interp.ppm / <prefix>.recomp.ppm.
            "--reuse" => reuse = true,
            "--dump" => dump = Some(it.next().ok_or("--dump needs a value")?.to_string()),
            "--input" => input = Some(it.next().ok_or("--input needs a value")?.to_string()),
            "--bios" => bios_path = Some(it.next().ok_or("--bios needs a value")?.to_string()),
            other if rom_path.is_none() => rom_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    let rom_path = rom_path.ok_or("missing ROM path")?;
    let bios = bios_path.as_deref().map(load_bios_file).transpose()?;

    let stem = Path::new(&rom_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("game")
        .to_string();
    let suffix = if bios.is_some() { "-bios" } else { "" };
    if !(reuse && Path::new(&format!("out/{stem}{suffix}.{LIB_EXT}")).is_file()) {
        let mut build_args = vec![rom_path.clone()];
        if let Some(p) = &bios_path {
            build_args.extend(["--bios".to_string(), p.clone()]);
        }
        cmd_build(&build_args)?;
    }
    let script = input.as_deref().map(InputScript::load).transpose()?;
    let interp = run_hash(
        &rom_path,
        frames,
        false,
        script.as_ref(),
        bios.as_deref(),
        dump.as_ref().map(|p| format!("{p}.interp.ppm")),
    )?;
    let recomp = run_hash(
        &rom_path,
        frames,
        true,
        script.as_ref(),
        bios.as_deref(),
        dump.as_ref().map(|p| format!("{p}.recomp.ppm")),
    )?;
    let verdict = if interp == recomp {
        "MATCH"
    } else {
        "MISMATCH"
    };
    println!("verify {verdict} interp={interp:016x} recomp={recomp:016x}");
    if interp == recomp {
        Ok(())
    } else {
        Err("differential mismatch".into())
    }
}

/// Deterministic demo input for differential runs: taps Start, then A,
/// periodically — enough to leave title screens and menus in most games.
fn demo_keys(frame: u64) -> u16 {
    let mut keys = 0x3FFu16;
    let phase = frame % 180;
    if frame >= 240 {
        if phase < 10 {
            keys &= !(1 << 3); // Start
        } else if (60..70).contains(&phase) {
            keys &= !(1 << 0); // A
        }
    }
    keys
}

/// FNV-1a, used for cheap change detection on backup media.
/// Write save data atomically (temp + rename): a crash mid-write can
/// never leave a torn .sav. std::fs::rename replaces the destination on
/// every supported platform.
fn write_sav(path: &str, data: &[u8]) -> std::io::Result<()> {
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

fn fb_hash(m: &Machine) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &px in &m.bus.framebuffer {
        for b in px.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x1_0000_01b3);
        }
    }
    hash
}

fn run_hash(
    rom_path: &str,
    frames: u64,
    recompiled: bool,
    input: Option<&InputScript>,
    bios: Option<&[u8]>,
    dump: Option<String>,
) -> Result<u64, String> {
    let rom = std::fs::read(rom_path).map_err(|e| format!("{rom_path}: {e}"))?;
    let mut m = make_machine(rom, bios);
    let keys_at = |frame: u64| match input {
        Some(s) => s.keys_at(frame),
        None => demo_keys(frame),
    };
    if !recompiled {
        for _ in 0..frames {
            m.bus.keys = keys_at(m.bus.frames);
            let frame=m.bus.frames;if let Some(s)=input {s.apply_mouse(&mut m.bus,frame)?;}
            m.run_frame(5_000_000);
        }
        dump_frame(&m, dump)?;
        return Ok(fb_hash(&m));
    }
    let stem = Path::new(rom_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("game")
        .to_string();
    let suffix = if bios.is_some() { "-bios" } else { "" };
    let lib_path = format!("out/{stem}{suffix}.{LIB_EXT}");
    let (_lib, table) = load_native(Path::new(&lib_path))?;
    let mptr = &mut m as *mut Machine as *mut std::ffi::c_void;
    while m.bus.frames < frames {
        let before = m.bus.frames;
        m.bus.keys = keys_at(before);
        if let Some(s)=input {s.apply_mouse(&mut m.bus,before)?;}
        run_frame_native(&mut m, &table, mptr, FRAME_STEP_GUARD);
        if m.bus.frames == before {
            return Err("step guard exceeded".into());
        }
    }
    dump_frame(&m, dump)?;
    Ok(fb_hash(&m))
}

/// Minimal PCM16LE WAV writer for audio triage dumps.
fn write_wav(path: &str, samples: &[i16], rate: u32, channels: u16) -> Result<(), String> {
    let data_len = (samples.len() * 2) as u32;
    let block = channels as u32 * 2;
    let mut w = Vec::with_capacity(44 + samples.len() * 2);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * block).to_le_bytes());
    w.extend_from_slice(&(block as u16).to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        w.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, w).map_err(|e| format!("{path}: {e}"))
}

fn dump_frame(m: &Machine, path: Option<String>) -> Result<(), String> {
    let Some(path) = path else { return Ok(()) };
    let width = gba_core::mem::render_width();
    let mut ppm = format!("P6\n{width} 160\n255\n").into_bytes();
    let pixels = if gba_core::mem::extra() == 0 {
        &m.bus.framebuffer[..]
    } else {
        &m.bus.wide_framebuffer[..width * 160]
    };
    for &px in pixels {
        let r = (px & 31) as u8;
        let g = ((px >> 5) & 31) as u8;
        let b = ((px >> 10) & 31) as u8;
        ppm.extend_from_slice(&[r << 3 | r >> 2, g << 3 | g >> 2, b << 3 | b >> 2]);
    }
    std::fs::write(&path, ppm).map_err(|e| format!("{path}: {e}"))
}

fn configure_headless_widescreen() {
    let value = std::env::var("FFTA_WIDESCREEN").unwrap_or_default();
    let ex = match value.trim() {
        "16:10" => 8,
        "16:9" => 22,
        "20:9" => 58,
        "21:9" => 66,
        "32:9" => 164,
        other => other.parse::<usize>().unwrap_or(0),
    };
    gba_core::mem::set_extra(ex);
}

/// Diagnostic replay view controls. An unset variable leaves the old runner intact.
struct HeadlessZoom {
    base_extra: usize,
    events: Vec<(u64, u32)>,
}
impl HeadlessZoom {
    fn max_level(base_extra: usize) -> u32 {
        let step = 1.122_462_f32;
        let by_cfg = (2.0_f32.ln() / step.ln() + 1e-3).floor();
        let by_w = (((240 + 2 * gba_core::mem::MAX_EXTRA) as f32 / (240 + 2 * base_extra) as f32)
            .ln()
            / step.ln())
        .floor();
        let by_h =
            (((160 + 2 * gba_core::mem::MAX_EXTRA_Y) as f32 / 160.0).ln() / step.ln()).floor();
        by_cfg.min(by_w).min(by_h).max(0.0) as u32
    }
    fn dimensions(base_extra: usize, level: u32) -> (usize, usize) {
        let z = if level == 0 {
            1.0
        } else {
            1.122_462_f32.powi(level as i32)
        };
        let ex = ((((240 + 2 * base_extra) as f32 * z - 240.0) / 2.0)
            .ceil()
            .max(0.0) as usize)
            .min(gba_core::mem::MAX_EXTRA);
        let ey =
            (((160.0 * z - 160.0) / 2.0).ceil().max(0.0) as usize).min(gba_core::mem::MAX_EXTRA_Y);
        (ex, ey)
    }
    fn parse(base_extra: usize, text: &str) -> Result<Self, String> {
        let mut events = Vec::new();
        for item in text.split(',') {
            let (frame, level) = item.trim().split_once(':').unwrap_or(("0", item.trim()));
            let frame = frame
                .parse::<u64>()
                .map_err(|_| "FFTA_ZOOM_LEVEL: invalid frame")?;
            let level = level
                .parse::<u32>()
                .map_err(|_| "FFTA_ZOOM_LEVEL: invalid level")?;
            if level > Self::max_level(base_extra) {
                return Err(format!(
                    "FFTA_ZOOM_LEVEL: level {level} exceeds {}",
                    Self::max_level(base_extra)
                ));
            }
            events.push((frame, level));
        }
        events.sort_by_key(|e| e.0);
        Ok(Self { base_extra, events })
    }
    fn from_env(base_extra: usize) -> Result<Option<Self>, String> {
        match std::env::var("FFTA_ZOOM_LEVEL") {
            Ok(text) => Self::parse(base_extra, &text).map(Some),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    fn apply(&self, frame: u64) {
        let level = self
            .events
            .iter()
            .rev()
            .find(|e| e.0 <= frame)
            .map_or(0, |e| e.1);
        let (ex, ey) = Self::dimensions(self.base_extra, level);
        if ex != gba_core::mem::extra() || ey != gba_core::mem::extra_y() || frame == 0 {
            gba_core::mem::set_extra(ex);
            gba_core::mem::set_extra_y(ey);
            eprintln!("FFTA_REPLAY_VIEW frame={frame} zoom_level={level} extra_x={ex} extra_y={ey} render_width={} render_height={}",240+2*ex,160+2*ey);
        }
    }
}

/// Read-only diagnostic snapshots. Raw tile counts are not logical cell counts.
fn movement_replay_observation(m: &Machine) -> Result<(), String> {
    if !gba_core::ffta_log::enabled() {
        return Ok(());
    }
    let Ok(prefix) = std::env::var("FFTA_MOVEMENT_DUMP_PREFIX") else {
        return Ok(());
    };
    let from = std::env::var("FFTA_MOVEMENT_FROM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(11340);
    if m.bus.frames < from {
        return Ok(());
    }
    let reg = |off| u16::from_le_bytes([m.bus.io[off], m.bus.io[off + 1]]);
    let rw = gba_core::mem::render_width();
    let rh = gba_core::mem::render_height();
    let counts = (0..2)
        .map(|bg| {
            m.bus.vram[0x5000 + bg * 0x800..0x5800 + bg * 0x800]
                .chunks_exact(2)
                .filter(|b| u16::from_le_bytes([b[0], b[1]]) & 1023 != 0)
                .count()
        })
        .collect::<Vec<_>>();
    let row_nonzero = m.bus.wide_framebuffer[..rw * rh]
        .chunks_exact(rw)
        .map(|r| r.iter().filter(|&&p| p != 0).count())
        .collect::<Vec<_>>();
    let full = m.bus.wide_framebuffer[..rw * rh]
        .iter()
        .flat_map(|p| p.to_le_bytes())
        .collect::<Vec<_>>();
    println!("FFTA_MOVEMENT frame={} dispcnt={:04x} bgcnt={:04x},{:04x} scroll={:?} extra_x={} extra_y={} map_nonzero_tiles={counts:?} framebuffer_sha256={} lower_rows_nonzero={} host_origins={:?} host_valid={:?} stage_origins={:?} stage_bases={:?} stage_valid={:?} status=RAW_NOT_LOGICAL_CELLS",
        m.bus.frames,reg(0),reg(8),reg(10),(0..2).map(|bg|(reg(0x10+bg*4)&511,reg(0x12+bg*4)&511)).collect::<Vec<_>>(),gba_core::mem::extra(),gba_core::mem::extra_y(),gba_core::ffta_rom::digest(&full),row_nonzero[160..].iter().sum::<usize>(),m.bus.ffta_movement.origins,m.bus.ffta_movement.valid,m.bus.ffta_stage.origins,m.bus.ffta_stage.bases,m.bus.ffta_stage.valid);
    let frames = std::env::var("FFTA_MOVEMENT_DUMP_FRAMES")
        .unwrap_or_else(|_| "11340,11341,11342,11343,11344,11345,11346,11347,12042,12540".into());
    if !frames
        .split(',')
        .any(|s| s.trim().parse::<u64>().ok() == Some(m.bus.frames))
    {
        return Ok(());
    }
    for (name, data) in [
        ("ewram", &m.bus.ewram),
        ("iwram", &m.bus.iwram),
        ("vram", &m.bus.vram),
        ("io", &m.bus.io),
        ("oam", &m.bus.oam),
        ("palette", &m.bus.palette),
    ] {
        let path = format!("{prefix}-{}-{name}.bin", m.bus.frames);
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, data).map_err(|e| e.to_string())?;
    }
    let path = format!("{prefix}-{}.ppm", m.bus.frames);
    let mut ppm = format!("P6\n{rw} {rh}\n255\n").into_bytes();
    for &p in &m.bus.wide_framebuffer[..rw * rh] {
        for c in [p & 31, (p >> 5) & 31, (p >> 10) & 31] {
            ppm.push((c << 3 | c >> 2) as u8);
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut file, &ppm).map_err(|e| e.to_string())?;
    println!(
        "FFTA_MOVEMENT_ROWS frame={} nonzero={row_nonzero:?}",
        m.bus.frames
    );
    Ok(())
}

struct FftaEdgeSnapshot {
    frame: u64,
    hofs: u16,
    vofs: u16,
    pixels: Vec<u16>,
}
/// Optional capture utility: records four register extrema without touching the guest.
struct FftaEdgeCapture {
    prefix: String,
    from: u64,
    width: usize,
    edges: [Option<FftaEdgeSnapshot>; 4],
}
impl FftaEdgeCapture {
    fn new() -> Option<Self> {
        Some(Self {
            prefix: std::env::var("FFTA_CAPTURE_BG_PREFIX").ok()?,
            from: std::env::var("FFTA_CAPTURE_BG_FROM")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            width: gba_core::mem::render_width(),
            edges: std::array::from_fn(|_| None),
        })
    }
    fn sample(&mut self, m: &Machine) {
        if m.bus.frames < self.from {
            return;
        }
        let Some(bg) = [0, 2]
            .into_iter()
            .find(|&bg| gba_core::ppu::ffta_terrain(&m.bus, bg))
        else {
            return;
        };
        let offset = 0x10 + 4 * bg;
        let hofs = u16::from_le_bytes([m.bus.io[offset], m.bus.io[offset + 1]]) & 511;
        let vofs = u16::from_le_bytes([m.bus.io[offset + 2], m.bus.io[offset + 3]]) & 511;
        for i in 0..4 {
            let update = self.edges[i].as_ref().is_none_or(|old| match i {
                0 => hofs < old.hofs,
                1 => hofs > old.hofs,
                2 => vofs < old.vofs,
                _ => vofs > old.vofs,
            });
            if update {
                self.edges[i] = Some(FftaEdgeSnapshot {
                    frame: m.bus.frames,
                    hofs,
                    vofs,
                    pixels: m.bus.wide_framebuffer[..self.width * 160].to_vec(),
                });
            }
        }
    }
    fn finish(self) -> Result<(), String> {
        for (name, snapshot) in ["left", "right", "top", "bottom"]
            .into_iter()
            .zip(self.edges)
        {
            let Some(snapshot) = snapshot else {
                println!("FFTA_CAPTURE edge={name} status=NO_MEDIDO terrain_signature_not_seen");
                continue;
            };
            let path = format!("{}-{name}.ppm", self.prefix);
            let mut ppm = format!("P6\n{} 160\n255\n", self.width).into_bytes();
            for px in snapshot.pixels {
                let r = (px & 31) as u8;
                let g = ((px >> 5) & 31) as u8;
                let b = ((px >> 10) & 31) as u8;
                ppm.extend_from_slice(&[r << 3 | r >> 2, g << 3 | g >> 2, b << 3 | b >> 2]);
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| format!("{path}: {e}"))?;
            std::io::Write::write_all(&mut file, &ppm).map_err(|e| e.to_string())?;
            println!(
                "FFTA_CAPTURE edge={name} frame={} hofs={} vofs={} path={path} status=MEDIDO",
                snapshot.frame, snapshot.hofs, snapshot.vofs
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_hints_preserve_sources_and_change_cache_identity_once() {
        let mut source = labels::Labels::default();
        source.rom.insert(0x0800_0101);
        source.iwram.insert(0x0300_0001);
        source.names.insert(0x0800_0101, "existing".into());
        source.ends.insert(0x0800_0101, 0x0800_0108);
        source.reserved.push("ewram 02000000 a".into());
        source.skipped = 2;
        let unknown = translation_labels(&"0".repeat(64), source);
        assert_eq!(unknown.rom.len(), 1);
        let before = unknown.digest(None);
        let merged = translation_labels(gba_core::ffta_rom::EUROPE_SHA256, unknown);
        assert_ne!(merged.digest(None), before);
        assert!(merged.rom.contains(&0x0800_0101));
        assert_eq!(merged.iwram.into_iter().collect::<Vec<_>>(), [0x0300_0001]);
        assert_eq!(merged.names[&0x0800_0101], "existing");
        assert_eq!(merged.ends[&0x0800_0101], 0x0800_0108);
        assert_eq!(merged.reserved, ["ewram 02000000 a"]);
        assert_eq!(merged.skipped, 2);
        // Rebuild a whole label value after the checks above consumed iwram.
        let mut again = labels::Labels::default();
        again.rom = merged.rom;
        let digest = again.digest(None);
        assert_eq!(translation_labels(gba_core::ffta_rom::EUROPE_SHA256, again).digest(None), digest);
        assert!(translation_labels(gba_core::ffta_rom::SHA256, labels::Labels::default()).rom.is_empty());
    }

    #[test]
    fn headless_state_rejects_conflicting_save_and_output_before_running() {
        let args = |values: &[&str]| values.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(cmd_runc(&args(&["missing.gba", "--state", "state.ffstate", "--sav", "save.sav"]))
            .unwrap_err().contains("mutually exclusive"));
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("slot.ffstate");
        std::fs::write(&state, b"source snapshot").unwrap();
        let path = state.to_str().unwrap();
        assert!(cmd_runc(&args(&["missing.gba", "--state", path, "--out", path]))
            .unwrap_err().contains("overwrite"));
        #[cfg(unix)]
        {
            let output = dir.path().join("image.ppm");
            std::os::unix::fs::symlink(&state, &output).unwrap();
            assert!(cmd_runc(&args(&["missing.gba", "--state", path, "--out", output.to_str().unwrap()]))
                .unwrap_err().contains("overwrite"));
        }
        assert_eq!(std::fs::read(state).unwrap(), b"source snapshot");
    }

    #[test]
    fn native_cost_observer_preserves_dispatch_and_guard_behavior() {
        extern "C" fn translated_step(
            _: *const gba_core::capi::RtApi,
            machine: *mut std::ffi::c_void,
        ) -> u32 {
            gba_core::capi::rt_interp_one(machine)
        }
        let blocks = [gba_core::capi::RcgBlock {
            key: ROM_BASE,
            func: translated_step,
        }];
        for (translated, halted, limit) in [
            (false, false, 100_000),
            (true, false, 100_000),
            (false, true, 1000),
            (true, false, 0),
            (true, false, 1),
        ] {
            let table = BlockTable::from_blocks(if translated { &blocks } else { &[] }).unwrap();
            // ARM branch to itself; no copyrighted game fixture required.
            let mut machine = Machine::new(0xeafffffeu32.to_le_bytes().to_vec());
            machine.bus.halted = halted;
            let checkpoint = machine.save_state().unwrap();
            let pointer = &mut machine as *mut Machine as *mut std::ffi::c_void;
            let expected_counts = run_frame_native(&mut machine, &table, pointer, limit);
            let expected_state = machine.save_state().unwrap();
            machine.load_state(&checkpoint).unwrap();
            let before = machine.bus.clock;
            let mut cycles = 0;
            let mut dispatches = [0; 4];
            let observed_counts = run_frame_native_observed(
                &mut machine,
                &table,
                pointer,
                limit,
                |kind, pc, charged| {
                    assert_eq!(pc, ROM_BASE);
                    cycles += charged;
                    dispatches[kind as usize] += 1;
                },
            );
            assert_eq!(observed_counts, expected_counts);
            assert!(
                machine.save_state().unwrap() == expected_state,
                "observer changed machine state"
            );
            assert_eq!(cycles, machine.bus.clock - before);
            assert_eq!(
                dispatches[NativeDispatch::Native as usize],
                observed_counts.0
            );
            assert_eq!(
                dispatches[NativeDispatch::Missing as usize],
                observed_counts.1
            );
            if halted {
                assert!(dispatches[NativeDispatch::System as usize] > 0);
                assert!(dispatches[NativeDispatch::System as usize] <= limit);
                assert_eq!(observed_counts, (0, 0));
            }
            if limit <= 1 {
                assert!(!machine.bus.frame_ready);
            } else if !halted {
                assert!(machine.bus.frame_ready);
            }
        }
    }

    #[test]
    #[ignore = "requires FFTA_STATE_TEST_ROM and FFTA_STATE_TEST_NATIVE local fixtures"]
    fn native_cost_observer_matches_game_frames_audio_and_full_state() {
        use sha2::Digest;
        let rom =
            std::fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
        let library =
            std::env::var_os("FFTA_STATE_TEST_NATIVE").expect("Native translation fixture");
        let (_library, table) = load_native(Path::new(&library)).unwrap();
        let mut machine = Machine::new(rom);
        arm_audio_hle(&mut machine, None);
        machine.bus.tap_channels = true;
        let checkpoint = machine.save_state().unwrap();
        let replay = |machine: &mut Machine, observed: bool| {
            let mut digest = sha2::Sha256::new();
            let mut counts = (0, 0);
            let mut dispatches = [0u64; 4];
            let mut charged = 0;
            let mut audio_counts = [0u64; 5];
            let start_clock = machine.bus.clock;
            let pointer = machine as *mut Machine as *mut std::ffi::c_void;
            for _ in 0..3000 {
                machine.bus.keys = demo_keys(machine.bus.frames);
                let before = machine.bus.frames;
                let ppu_capture = gba_core::ppu_profile::Capture::start(observed);
                let (n, f) = if observed {
                    run_frame_native_observed(
                        machine,
                        &table,
                        pointer,
                        FRAME_STEP_GUARD,
                        |kind, _, cycles| {
                            dispatches[kind as usize] += 1;
                            charged += cycles;
                        },
                    )
                } else {
                    run_frame_native(machine, &table, pointer, FRAME_STEP_GUARD)
                };
                if let Some(ppu) = ppu_capture.finish() {
                    assert_eq!(ppu.scanline_calls, 160);
                    assert!(ppu.zoom_calls <= 1);
                    assert!(ppu.scanlines + ppu.zoom <= ppu.frame);
                }
                assert_eq!(machine.bus.frames, before + 1);
                counts.0 += n;
                counts.1 += f;
                digest.update(machine.bus.frames.to_le_bytes());
                digest.update(machine.bus.clock.to_le_bytes());
                for (total, len) in audio_counts.iter_mut().zip([
                    machine.bus.audio_buf.len(), machine.bus.psg_tap.len(),
                    machine.bus.fifo_tap[0].len(), machine.bus.fifo_tap[1].len(),
                    machine.bus.hle_tap.len(),
                ]) {
                    *total += len as u64;
                    digest.update((len as u64).to_le_bytes());
                }
                for pixel in &machine.bus.framebuffer {
                    digest.update(pixel.to_le_bytes());
                }
                for pixel in &machine.bus.wide_framebuffer {
                    digest.update(pixel.to_le_bytes());
                }
                digest.update(&machine.bus.oam);
                for sample in machine.bus.audio_buf.drain(..) {
                    digest.update(sample.to_le_bytes());
                }
                for sample in machine.bus.psg_tap.drain(..) {
                    digest.update(sample.to_le_bytes());
                }
                for tap in &mut machine.bus.fifo_tap {
                    for (sample, period) in tap.drain(..) {
                        digest.update(sample.to_le_bytes());
                        digest.update(period.to_le_bytes());
                    }
                }
                for sample in machine.bus.hle_tap.drain(..) {
                    digest.update(sample.to_bits().to_le_bytes());
                }
            }
            if observed {
                assert_eq!(charged, machine.bus.clock - start_clock);
                assert_eq!(dispatches[NativeDispatch::Native as usize], counts.0);
                assert_eq!(dispatches[NativeDispatch::Missing as usize], counts.1);
                eprintln!(
                    "Observed native={:?} dispatches={dispatches:?} cycles={charged}",
                    counts
                );
                eprintln!("Observed audio samples mixed/PSG/FIFO0/FIFO1/HLE={audio_counts:?}");
            }
            (
                digest.finalize().to_vec(),
                machine.save_state().unwrap(),
                counts,
            )
        };
        let expected = replay(&mut machine, false);
        assert!(expected.2 .0 > 0, "Fixture must execute translated code");
        machine.load_state(&checkpoint).unwrap();
        let actual = replay(&mut machine, true);
        assert_eq!(actual.0, expected.0, "frame/OAM/audio transcript differs");
        assert!(
            actual.1 == expected.1,
            "full serialized machine state differs"
        );
        assert_eq!(actual.2, expected.2, "native/fallback counts differ");
    }

    #[test]
    #[ignore = "requires FFTA_STATE_TEST_ROM and FFTA_STATE_TEST_NATIVE local fixtures"]
    fn native_save_state_replays_identical_frames_and_machine_state() {
        use sha2::Digest;
        let rom = std::fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").expect("ROM fixture")).unwrap();
        let library = std::env::var_os("FFTA_STATE_TEST_NATIVE").expect("Native translation fixture");
        let (_library, table) = load_native(Path::new(&library)).unwrap();
        let mut machine = Machine::new(rom.clone());
        arm_audio_hle(&mut machine, None);
        machine.bus.tap_channels = true;
        let advance = |machine: &mut Machine| {
            machine.bus.keys = demo_keys(machine.bus.frames);
            let pointer = machine as *mut Machine as *mut std::ffi::c_void;
            let counts = run_frame_native(machine, &table, pointer, FRAME_STEP_GUARD);
            machine.bus.audio_buf.clear(); machine.bus.psg_tap.clear(); machine.bus.hle_tap.clear();
            machine.bus.fifo_tap.iter_mut().for_each(Vec::clear);
            counts.0
        };
        let mut native_blocks = 0;
        for _ in 0..600 { native_blocks += advance(&mut machine); }
        assert!(native_blocks > 0, "Fixture must actually execute native code");
        let checkpoint = machine.save_state().unwrap();
        let replay = |machine: &mut Machine| {
            let mut hashes = Vec::new();
            for _ in 0..120 {
                advance(machine);
                let bytes: Vec<u8> = machine.bus.framebuffer.iter().flat_map(|v| v.to_le_bytes()).collect();
                hashes.push(sha2::Sha256::digest(bytes).to_vec());
            }
            hashes
        };
        let expected_frames = replay(&mut machine);
        let expected_state = machine.save_state().unwrap();
        machine.load_state(&checkpoint).unwrap();
        assert_eq!(replay(&mut machine), expected_frames);
        assert_eq!(machine.save_state().unwrap(), expected_state);
        // A fresh machine represents a later launch with the same ROM.
        let mut fresh = Machine::new(rom);
        fresh.load_state(&checkpoint).unwrap();
        assert_eq!(replay(&mut fresh), expected_frames);
        assert_eq!(fresh.save_state().unwrap(), expected_state);
        // Exercise the actual slot undo path with native blocks and live HLE audio.
        let dir = tempfile::tempdir().unwrap();
        let mut slots = crate::save_state::Slots::new(&dir.path().join("test.sav"), &fresh.bus.rom);
        let preview = vec![[0,0,0,255]; crate::save_state::PREVIEW_W*crate::save_state::PREVIEW_H];
        slots.save(&fresh,1,&preview,false).unwrap();
        replay(&mut fresh);
        let before_load = fresh.save_state().unwrap();
        slots.load(&mut fresh,1).unwrap();
        assert_eq!(slots.undo(&mut fresh).unwrap(),crate::save_state::UndoEffect::Load(1));
        assert_eq!(fresh.save_state().unwrap(),before_load);
        let undone_frames = replay(&mut fresh);
        let undone_state = fresh.save_state().unwrap();
        let mut reference = Machine::new(fresh.bus.rom.clone());
        reference.load_state(&before_load).unwrap();
        assert_eq!(replay(&mut reference),undone_frames);
        assert_eq!(reference.save_state().unwrap(),undone_state);
    }

    #[test]
    fn discarded_audio_cannot_survive_in_callback_history() {
        use std::sync::{Arc, Mutex};
        // Each audible path has separate history: faithful DAC, PSG/HLE sinc,
        // FIFO future impulses/integrator, then the per-path DC blockers.
        for source in ["mixed", "psg", "hle", "fifo", "normal"] {
            for speed in [1., 2., 4., 8.] {
                let streams = Arc::new(Mutex::new(AudioStreams {
                    enhanced_on: source != "mixed" && source != "normal",
                    hle_on: source == "hle",
                    route: [3, 3],
                    playback_speed: speed,
                    ..Default::default()
                }));
                {
                    let mut st = streams.lock().unwrap();
                    match source {
                        "mixed" => st.mixed.extend(vec![8_000; 32_768]),
                        "psg" => st.psg.extend(vec![0.25; 32_768]),
                        "hle" => st.hle.extend(vec![0.25; 32_768]),
                        "fifo" => st.fifo[0].extend(vec![(64, 512); 16_384]),
                        "normal" => {
                            let mut queue = ff_audio::NormalAudio::new().unwrap();
                            queue.push(&vec![8_000; 32_768]).unwrap();
                            st.normal_audio = Some(queue);
                        }
                        _ => unreachable!(),
                    }
                    if source != "mixed" && source != "normal" {
                        // Production always supplies the faithful grid too;
                        // it is the authoritative Fast clock even in Enhanced.
                        st.mixed.extend(vec![0; 32_768]);
                    }
                }
                let mut callback = output_callback::<f32>(48_000., 2, streams.clone());
                let mut output = [0.; 1024];
                callback(&mut output);
                assert!(output.iter().any(|v| v.abs() > 0.001), "{source}/{speed} not primed");
                streams.lock().unwrap().discard_pending();
                callback(&mut output);
                let peak = output.iter().map(|v| v.abs()).fold(0., f32::max);
                assert_eq!(peak, 0., "old {source}/{speed} audio survives queue invalidation");
            }
        }
    }

    #[test]
    fn audio_transition_matches_fresh_history_with_new_samples() {
        use std::sync::{Arc, Mutex};
        fn feed(st: &mut AudioStreams, speed: f64, mode: &str) {
            st.playback_speed = speed;
            st.enhanced_on = mode == "enhanced" || mode == "hle";
            st.hle_on = mode == "hle";
            st.muted = mode == "silent";
            st.vol_cut = 0.3;
            st.route = [3, 3];
            st.vol_half = [true, false];
            let pcm: Vec<_> = (0..32_768).map(|i| ((i * 137 % 16_000) - 8_000) as i16).collect();
            if mode == "normal" {
                let mut queue = ff_audio::NormalAudio::new().unwrap();
                queue.push(&pcm).unwrap();
                st.normal_audio = Some(queue);
            } else if mode != "silent" {
                st.mixed.extend(&pcm);
                st.psg.extend(pcm.iter().map(|&s| s as f32 / 32768.));
                st.hle.extend(pcm.iter().map(|&s| -s as f32 / 32768.));
                for q in &mut st.fifo {
                    q.extend((0..16_384).map(|i| ((i % 127 - 64) as i8, 768)));
                }
            }
        }
        for rate in [44_100., 48_000.] {
            for observed in [false, true] {
                let streams = Arc::new(Mutex::new(AudioStreams {
                    profile: observed.then(crate::audio_profile::Profile::new),
                    ..Default::default()
                }));
                let mut callback = output_callback::<f32>(rate, 2, streams.clone());
                for (speed, mode) in [(4., "hle"), (2., "enhanced"), (8., "faithful"),
                    (1., "normal"), (1., "silent"), (1., "enhanced"), (4., "faithful")] {
                    let fresh = Arc::new(Mutex::new(AudioStreams::default()));
                    let mut reference = output_callback::<f32>(rate, 2, fresh.clone());
                    for shared in [&streams, &fresh] {
                        let mut st = shared.lock().unwrap();
                        // Also exercise multiple boundaries between callbacks.
                        st.discard_pending();
                        st.discard_pending();
                        feed(&mut st, speed, mode);
                    }
                    let mut actual = [0.; 1024];
                    let mut expected = [0.; 1024];
                    callback(&mut actual);
                    reference(&mut expected);
                    assert_eq!(actual, expected, "{rate}/{observed}/{mode}/{speed}");
                    if mode == "silent" {
                        assert!(actual.iter().all(|&s| s == 0.));
                    } else {
                        assert!(actual.iter().any(|s| s.abs() > 0.001));
                    }
                    let a = streams.lock().unwrap();
                    let b = fresh.lock().unwrap();
                    assert_eq!((&a.mixed, &a.psg, &a.hle, &a.fifo), (&b.mixed, &b.psg, &b.hle, &b.fifo));
                }
            }
        }
    }

    #[test]
    fn user_mute_ramps_output_but_keeps_consumption_volume_and_temporary_mute() {
        use std::sync::{Arc, Mutex};
        for mode in ["faithful", "enhanced", "fast", "normal"] {
            let mut av = input_config::AvConfig {
                audio_volume: 0.6, audio_enhanced: mode == "enhanced",
                audio_mute: true, ..Default::default()
            };
            let make = || {
                let mut st = AudioStreams { playback_speed: if mode == "fast" { 4. } else { 1. },
                    ..Default::default() };
                st.configure_audio(&av);
                if mode == "normal" { st.normal_audio = Some(ff_audio::NormalAudio::new().unwrap()); }
                Arc::new(Mutex::new(st))
            };
            let muted = make();
            let reference = make();
            reference.lock().unwrap().user_muted = false;
            let mut a = output_callback::<f32>(48_000., 2, muted.clone());
            let mut b = output_callback::<f32>(48_000., 2, reference.clone());
            let pcm: Vec<i16> = (0..if mode == "fast" {8192} else {2048})
                .map(|i| if i % 64 < 32 { 6000 } else { -6000 }).collect();
            let mut out_a = [0.; 1024];
            let mut out_b = [0.; 1024];
            for tick in 0..240 {
                av.audio_mute = tick < 5 || (30..90).contains(&tick) || (120..150).contains(&tick);
                if tick >= 60 { av.audio_volume = 0.3; }
                for shared in [&muted, &reference] {
                    let mut st = shared.lock().unwrap();
                    st.configure_audio(&av);
                    if Arc::ptr_eq(shared, &reference) { st.user_muted = false; }
                    st.muted = (120..180).contains(&tick);
                    if let Some(queue) = st.normal_audio.as_mut() { queue.push(&pcm).unwrap(); }
                    else {
                        st.mixed.extend(&pcm);
                        st.psg.extend(pcm.iter().map(|&v| v as f32 / 32768.));
                        st.fast_produced += (pcm.len() / 2) as u64;
                    }
                }
                a(&mut out_a); b(&mut out_b);
                if tick == 30 || tick == 90 {
                    let gain = |range: std::ops::Range<usize>| {
                        let values: Vec<_> = range.filter(|&i| out_b[i].abs() > 0.0001)
                            .map(|i| out_a[i] / out_b[i]).collect();
                        assert!(!values.is_empty());
                        values.iter().sum::<f32>() / values.len() as f32
                    };
                    let first = gain(0..40); let last = gain(984..1024);
                    if tick == 30 {
                        assert!(first > 0.98 && first <= 1.);
                        assert!((0.55..0.65).contains(&last));
                    } else {
                        assert!(first < 0.03);
                        assert!((0.35..0.45).contains(&last));
                    }
                }
                if tick < 5 || (85..90).contains(&tick) || (175..180).contains(&tick) {
                    assert!(out_a.iter().all(|&s| s == 0.), "{mode}/{tick} not silent");
                }
                if (85..90).contains(&tick) {
                    assert!(out_b.iter().any(|s| s.abs() > 0.001), "{mode} reference not audible");
                }
                if tick > 225 {
                    assert!(out_a.iter().any(|s| s.abs() > 0.001));
                    assert!(out_a.iter().zip(out_b).all(|(a,b)| (a-b).abs() < 0.00001));
                }
                let st = muted.lock().unwrap(); let other = reference.lock().unwrap();
                assert_eq!((st.consumed, st.holds), (other.consumed, other.holds));
                assert_eq!(st.mixed, other.mixed); assert_eq!(st.psg, other.psg);
                assert_eq!(st.normal_audio.as_ref().map(|q| q.stats().queued_pairs),
                    other.normal_audio.as_ref().map(|q| q.stats().queued_pairs));
            }
            let mut st = muted.lock().unwrap();
            av.audio_mute = true;
            st.configure_audio(&av);
            st.discard_pending(); // Fast/pause/load reset must retain user choice.
            assert!(st.user_muted);
            assert_eq!(st.volume_target(), 0.);
            av.audio_mute = false;
            st.muted = true; // Silent/fallback must survive user unmute/config edits.
            st.configure_audio(&av);
            assert_eq!(st.volume_target(), 0.);
            st.muted = false;
            assert!((st.volume_target() - 0.3).abs() < 0.00001);
        }
    }

    #[test]
    fn bounded_normal_audio_is_1x_and_profile_parity_survives_resync_and_discard() {
        use std::sync::{Arc, Mutex};
        let make = |observed: bool| Arc::new(Mutex::new(AudioStreams {
            profile: observed.then(crate::audio_profile::Profile::new),
            normal_audio: Some(ff_audio::NormalAudio::new().unwrap()),
            enhanced_on: true, playback_speed: 1., ..Default::default()
        }));
        let plain = make(false);
        let observed = make(true);
        let mut a = output_callback::<f32>(48_000., 2, plain.clone());
        let mut b = output_callback::<f32>(48_000., 2, observed.clone());
        let pcm: Vec<_> = (0..8192).map(|i| if i % 64 < 32 { 4000 } else { -4000 }).collect();
        let mut out_a = [0.; 1024];
        let mut out_b = [0.; 1024];
        for _ in 0..250 {
            for shared in [&plain, &observed] {
                shared.lock().unwrap().normal_audio.as_mut().unwrap().push(&pcm).unwrap();
            }
            a(&mut out_a); b(&mut out_b);
            assert_eq!(out_a, out_b);
            assert!(out_a.iter().any(|s| s.abs() > 0.001));
        }
        for shared in [&plain, &observed] {
            let mut st = shared.lock().unwrap();
            assert_eq!(st.holds, 0);
            let queue = st.normal_audio.as_ref().unwrap();
            assert!(queue.stats().resyncs > 0);
            assert!(queue.stats().queued_pairs <= ff_audio::MAX_PAIRS);
            // A callback consumes only the 1x source budget, not the faster
            // producer batch; a pending skip fade must disappear on epoch reset.
            let before = queue.stats().queued_pairs;
            drop(st);
            if Arc::ptr_eq(shared, &plain) { a(&mut out_a); }
            else { b(&mut out_b); }
            st = shared.lock().unwrap();
            let consumed = before - st.normal_audio.as_ref().unwrap().stats().queued_pairs;
            assert!((698..=700).contains(&consumed));
            st.discard_pending();
        }
        a(&mut out_a); b(&mut out_b);
        assert!(out_a.iter().all(|&s| s == 0.));
        assert_eq!(out_a, out_b);
    }

    #[test]
    fn fast_audio_follows_actual_production_instead_of_requested_speed() {
        use std::sync::{Arc, Mutex};
        for enhanced in [false, true] {
            let streams = Arc::new(Mutex::new(AudioStreams {
                playback_speed: 4., enhanced_on: enhanced, hle_on: true,
                route: [3, 3], ..Default::default()
            }));
            let mut callback = output_callback::<f32>(48_000., 2, streams.clone());
            let mut out = [0.; 1024];
            let mut produced = 0.;
            let mut fed = 0;
            let mut holds = 0;
            let mut audible = 0;
            for tick in 0..800 {
                // 2x real production, in guest-frame-sized batches, while 4x
                // is selected. This is the measured CPU-limited Fast case.
                produced += 65_536. * 2. * 512. / 48_000.;
                let pairs = ((produced as usize - fed) / 1097) * 1097;
                fed += pairs;
                {
                    let mut st = streams.lock().unwrap();
                    st.fast_produced += pairs as u64;
                    for i in 0..pairs {
                        let s = if (fed + i) % 64 < 32 { 4000 } else { -4000 };
                        st.mixed.extend([s, s]);
                        st.psg.extend([s as f32 / 32768.; 2]);
                        st.hle.extend([s as f32 / 32768.; 2]);
                        if i % 2 == 0 {
                            st.fifo[0].push_back(((s / 128) as i8, 512));
                            st.fifo[1].push_back(((s / 128) as i8, 512));
                        }
                    }
                }
                callback(&mut out);
                if tick == 400 { holds = streams.lock().unwrap().holds; }
                if tick > 400 {
                    audible += usize::from(out.iter().any(|s| s.abs() > 0.001));
                    assert!(streams.lock().unwrap().mixed.len() < 65_536,
                        "audio backlog grows instead of following the guest");
                }
            }
            let st = streams.lock().unwrap();
            assert_eq!(st.holds - holds, 0, "sustained Fast source starvation");
            assert_eq!(audible, 399, "zero holds must not hide silent output");
        }
    }

    #[test]
    fn accelerated_audio_consumes_at_the_selected_multiplier() {
        fn consumed(speed: f64) -> usize {
            let mut ring = std::collections::VecDeque::from(vec![0.0; 200_000]);
            let before = ring.len();
            let mut resampler = SincResampler::new(65_536.0, 48_000.0);
            for _ in 0..1000 {
                resampler.next_with_speed(&mut ring, speed);
            }
            (before - ring.len()) / 2
        }
        let normal = consumed(1.0);
        for speed in [2, 4, 8] {
            let fast = consumed(speed as f64);
            assert!(fast.abs_diff(normal * speed) <= speed);
        }
    }

    #[test]
    fn observed_source_reads_count_partial_pairs_and_dac_holds_without_changing_dsp() {
        use std::collections::VecDeque;
        use crate::audio_profile::SourceReads;
        for speed in [1., 2., 8.] {
            let mut plain = SincResampler::new(65_536., 48_000.);
            let mut observed = SincResampler::new(65_536., 48_000.);
            let mut a = VecDeque::from([0.25, -0.5, 0.75]);
            let mut b = a.clone(); let mut reads = SourceReads::default();
            let mut off = SourceReads::default();
            for _ in 0..100 {
                assert_eq!(plain.next_observed::<false>(&mut a, speed, &mut off),
                    observed.next_observed::<true>(&mut b, speed, &mut reads));
            }
            assert_eq!(a, b); assert!(a.is_empty());
            assert!(reads.requests > 100);
            assert_eq!(reads.empty, reads.requests - 1); // one full pair, one partial pair
            assert_eq!(off, SourceReads::default());
            assert_eq!((plain.frac, plain.hist), (observed.frac, observed.hist));
            let mut plain = FifoInterp::new(48_000.);
            let mut observed = FifoInterp::new(48_000.);
            let mut a = VecDeque::from([(12, 512), (-7, 512)]); let mut b = a.clone();
            let mut reads = SourceReads::default(); let mut off = SourceReads::default();
            for _ in 0..100 {
                assert_eq!(plain.next_with_speed_observed::<false>(&mut a, speed, &mut off),
                    observed.next_with_speed_observed::<true>(&mut b, speed, &mut reads));
            }
            assert_eq!(a, b); assert!(a.is_empty());
            assert_eq!(reads.empty, reads.requests - 2); // both real DAC events, then holds
            assert_eq!(off, SourceReads::default());
            assert_eq!((plain.pos, plain.y, plain.level, plain.t_next, plain.period, plain.out_step, plain.fut),
                (observed.pos, observed.y, observed.level, observed.t_next, observed.period, observed.out_step, observed.fut));
        }
    }

    #[test]
    fn callback_profile_preserves_samples_queues_and_legacy_counters() {
        use std::sync::{Arc, Mutex};
        fn check<T: cpal::SizedSample + cpal::FromSample<f32> + PartialEq + std::fmt::Debug>() {
            for (rate, channels) in [(44_100., 1), (48_000., 2), (48_000., 6)] {
                let plain = Arc::new(Mutex::new(AudioStreams::default()));
                let observed = Arc::new(Mutex::new(AudioStreams {
                    profile: Some(crate::audio_profile::Profile::new()),
                    ..Default::default()
                }));
                let mut a = output_callback::<T>(rate, channels, plain.clone());
                let mut b = output_callback::<T>(rate, channels, observed.clone());
                for round in 0..8 {
                    for streams in [&plain, &observed] {
                        let mut st = streams.lock().unwrap();
                        st.enhanced_on = round % 2 == 0;
                        st.hle_on = round % 3 == 0;
                        st.playback_speed = if round == 4 { 4. } else { 1. };
                        st.vol_cut = 0.25;
                        st.muted = round == 5;
                        st.route = [3, 3];
                        st.vol_half = [true, false];
                        // Alternate short input and starvation; leave DSP state
                        // alive across the transitions, as a real stream does.
                        if round % 2 == 0 {
                            st.mixed.extend((0..256).map(|i| (i * 93 - 12_000) as i16));
                            st.psg.extend((0..256).map(|i| (i as f32 - 128.) / 256.));
                            st.hle.extend((0..256).map(|i| (128. - i as f32) / 256.));
                            for q in &mut st.fifo {
                                q.extend((0..64).map(|i| ((i * 3 - 96) as i8, 512)));
                            }
                        }
                        if let Some(p) = st.profile.as_mut() {
                            p.set_context(crate::audio_profile::Context::of(round == 2, round != 0, round == 4));
                        }
                    }
                    let mut out_a = vec![T::EQUILIBRIUM; 512 * channels];
                    let mut out_b = out_a.clone();
                    a(&mut out_a);
                    b(&mut out_b);
                    assert_eq!(out_a, out_b);
                    let mut x = plain.lock().unwrap();
                    let mut y = observed.lock().unwrap();
                    assert_eq!((&x.mixed, &x.psg, &x.hle, &x.fifo), (&y.mixed, &y.psg, &y.hle, &y.fifo));
                    assert_eq!((x.holds, x.drops, x.clipped, x.consumed), (y.holds, y.drops, y.clipped, y.consumed));
                    if round == 4 {
                        assert_eq!(x.holds, 0, "Fast primes with silence instead of stale holds");
                    } else {
                        assert!(x.holds > 0, "starvation was not exercised");
                    }
                    // Legacy alarm windows must not affect diagnostic totals.
                    x.holds = 0;
                    y.holds = 0;
                }
            }
        }
        check::<f32>();
        check::<i16>();
        check::<u16>();
    }

    #[test]
    #[ignore = "optional local callback profiling overhead measurement"]
    fn callback_profile_overhead() {
        use std::sync::{Arc, Mutex};
        for pair in 0..5 {
            for enabled in if pair % 2 == 0 { [false, true] } else { [true, false] } {
                let streams = Arc::new(Mutex::new(AudioStreams {
                    profile: enabled.then(crate::audio_profile::Profile::new),
                    ..Default::default()
                }));
                let mut output = output_callback::<f32>(48_000., 2, streams.clone());
                let mut samples = [0.; 1024];
                for _ in 0..100 { output(&mut samples); }
                let begin = std::time::Instant::now();
                for _ in 0..1200 { output(&mut samples); }
                let elapsed = begin.elapsed();
                assert!(samples.iter().all(|v| *v == 0.));
                let st = streams.lock().unwrap();
                assert_eq!(st.consumed, 1300 * 512);
                assert!(st.holds > 0);
                eprintln!("FFTA_AUDIO_OBSERVER_OVERHEAD pair={pair} enabled={enabled} callbacks=1200 ns_per_callback={:.3}", elapsed.as_nanos() as f64 / 1200.);
            }
        }
    }

    #[test]
    fn headless_zoom_schedule_matches_window_limits() {
        assert_eq!(HeadlessZoom::max_level(164), 6);
        assert_eq!(HeadlessZoom::dimensions(164, 0), (164, 0));
        let zoom = HeadlessZoom::parse(164, "0:0,11340:6,12000:2").unwrap();
        assert_eq!(zoom.events, vec![(0, 0), (11340, 6), (12000, 2)]);
        assert!(HeadlessZoom::parse(164, "7").is_err());
        for level in 0..=6 {
            let (ex, ey) = HeadlessZoom::dimensions(164, level);
            println!("FFTA_ZOOM level={level} extra_x={ex} extra_y={ey}");
        }
    }

    #[test]
    fn replay_keeps_direct_mouse_events_separate_from_held_keys() {
        let input=InputScript::parse("gba-input v1\n0 3ff\n300 mouse:4\n304 mouse:1\n310 3fe\n314 3ff\n","own-fixture").unwrap();
        assert_eq!(input.1,vec![(300,4),(304,1)]);assert_eq!(input.keys_at(305),0x3ff);assert_eq!(input.keys_at(310),0x3fe);
        assert!(InputScript::parse("gba-input v1\n0 mouse:5\n","bad").is_err());
        assert!(InputScript::parse("gba-input v1\n0 mouse:25846\n","world").is_ok());
        assert!(InputScript::parse("gba-input v1\n0 mouse:42496\n","outside-world").is_err());
    }
    #[test]
    fn replay_preserves_last_event_at_equal_frame() {
        let input =
            InputScript::parse("gba-input v1\n10 3ff\n10 37f\n11 3ff\n", "own-fixture").unwrap();
        assert_eq!(input.keys_at(9), 0x3ff);
        assert_eq!(input.keys_at(10), 0x37f);
        assert_eq!(input.keys_at(11), 0x3ff);
    }

    #[test]
    fn fractional_fast_sinc_filters_do_not_select_the_unaccelerated_cutoff() {
        use std::collections::VecDeque;
        for rate in [44_100., 48_000.] {
            for speed in [1.75, 3.3, 5.75, 7.9] {
                let rms = |frequency: f64| {
                    let mut resampler = SincResampler::new(65_536., rate);
                    let mut queue = VecDeque::new();
                    for i in 0..100_000 {
                        let sample = (i as f64 * frequency * std::f64::consts::TAU / 65_536.).sin() as f32 * 0.5;
                        queue.extend([sample; 2]);
                    }
                    let mut squares = 0.;
                    for i in 0..6000 {
                        let (s, _) = resampler.next_clocked::<false>(&mut queue, speed,
                            &mut crate::audio_profile::SourceReads::default(), false);
                        if i >= 1000 { squares += (s as f64).powi(2); }
                    }
                    (squares / 5000.).sqrt()
                };
                let audible = rms(500.);
                let alias = rms(rate / (2. * speed) * 1.5);
                assert!(audible > 0.30, "passband {rate}/{speed}: {audible}");
                assert!(alias < 0.10, "alias {rate}/{speed}: {alias}");
            }
        }
    }

    #[test]
    fn sinc_rows_have_unity_dc_gain() {
        let r = SincResampler::new(65536.0, 48000.0);
        for p in 0..=SINC_PHASES {
            let sum: f32 = r.table[p * SINC_TAPS..][..SINC_TAPS].iter().sum();
            assert!((sum - 1.0).abs() < 1e-4, "phase {p}: sum {sum}");
        }
    }

    #[test]
    fn sinc_preserves_audible_sine() {
        // 1 kHz at the 65536 Hz tap, resampled to 48 kHz: RMS must come
        // through within a few percent.
        let mut r = SincResampler::new(65536.0, 48000.0);
        let mut q = std::collections::VecDeque::new();
        for n in 0..65536 {
            let s = (2.0 * std::f64::consts::PI * 1000.0 * n as f64 / 65536.0).sin();
            q.push_back(s as f32 * 0.5);
            q.push_back(s as f32 * 0.5);
        }
        let out: Vec<f32> = (0..40000)
            .map(|_| {
                let (l, r2) = r.next(&mut q);
                (l + r2) * 0.5
            })
            .collect();
        let rms = (out[2000..38000].iter().map(|v| v * v).sum::<f32>() / 36000.0).sqrt();
        let expect = 0.5 / 2f32.sqrt(); // amplitude 0.5 sine
        assert!(
            (rms - expect).abs() / expect < 0.03,
            "rms {rms} vs {expect}"
        );
    }

    /// Fill a FIFO event queue with a sine at the given timer period.
    fn fifo_sine(period: u32, hz: f64, n: usize) -> std::collections::VecDeque<(i8, u32)> {
        let src_hz = (1u64 << 24) as f64 / period as f64;
        (0..n)
            .map(|i| {
                let s = (2.0 * std::f64::consts::PI * hz * i as f64 / src_hz).sin();
                ((s * 100.0) as i8, period)
            })
            .collect()
    }

    #[test]
    fn fifo_interp_preserves_sine_at_mp2k_rate() {
        // 1 kHz at 13379 Hz (period 1254) — the MP2K default mix rate —
        // interpolated to 48 kHz: RMS within a few percent of the 8-bit
        // source amplitude.
        let mut f = FifoInterp::new(48000.0);
        let mut q = fifo_sine(1254, 1000.0, 40000);
        let out: Vec<f32> = (0..96000).map(|_| f.next(&mut q)).collect();
        let rms = (out[2000..90000].iter().map(|v| v * v).sum::<f32>() / 88000.0).sqrt();
        let expect = (100.0 * 64.0 / 32768.0) / 2f32.sqrt();
        assert!(
            (rms - expect).abs() / expect < 0.05,
            "rms {rms} vs {expect}"
        );
    }

    #[test]
    fn fifo_interp_follows_rate_change() {
        // Retune mid-stream 13379 Hz -> 32768 Hz; output stays bounded
        // and alive on both sides of the change.
        let mut f = FifoInterp::new(48000.0);
        let mut q = fifo_sine(1254, 1000.0, 4000);
        q.extend(fifo_sine(512, 1000.0, 8000));
        let out: Vec<f32> = (0..40000).map(|_| f.next(&mut q)).collect();
        assert!(out.iter().all(|v| v.abs() < 0.5));
        let tail_rms = (out[30000..40000].iter().map(|v| v * v).sum::<f32>() / 10000.0).sqrt();
        assert!(tail_rms > 0.05, "went silent after rate change: {tail_rms}");
    }

    #[test]
    fn fifo_interp_holds_above_device_rate() {
        // 65536 Hz channel into a 48 kHz device: the structural fallback
        // is sample-and-hold; output must stay bounded.
        let mut f = FifoInterp::new(48000.0);
        let mut q = fifo_sine(256, 1000.0, 16384);
        let out: Vec<f32> = (0..8000).map(|_| f.next(&mut q)).collect();
        assert!(out.iter().all(|v| v.abs() < 0.5));
    }

    #[test]
    fn soft_clip_identity_below_rail_knee_above() {
        // Bit-exact identity through the rail: unclipped material must
        // be untouched (the Stage-2 fidelity guarantee).
        for i in -1000..=1000i32 {
            let x = i as f32 * (RAIL / 1000.0);
            assert_eq!(soft_clip(x), x);
        }
        // Above the rail: monotonic, odd-symmetric, and bounded so that
        // post-OUT_GAIN output never exceeds device full scale.
        let mut prev = soft_clip(RAIL);
        for i in 1..=300 {
            let x = RAIL + i as f32 * 0.005;
            let y = soft_clip(x);
            assert!(y >= prev, "not monotonic at {x}");
            assert!(y * OUT_GAIN <= 1.0 + 1e-4, "exceeds full scale at {x}: {y}");
            assert_eq!(soft_clip(-x), -y);
            prev = y;
        }
        // C1 at the knee: slope just above the rail stays ~1.
        let d = (soft_clip(RAIL + 1e-3) - soft_clip(RAIL - 1e-3)) / 2e-3;
        assert!((d - 1.0).abs() < 0.02, "knee slope {d}");
        // The hardware's worst case (3x rail) lands just under full scale.
        assert!(soft_clip(3.0 * RAIL) * OUT_GAIN > 0.99);
    }
}
