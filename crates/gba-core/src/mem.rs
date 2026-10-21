//! Memory map and memory-mapped hardware (hardware reference, "Memory Map"
//! and I/O register chapters).
//!
//! | Region  | Address    | Size   | Mirroring                      |
//! |---------|------------|--------|--------------------------------|
//! | BIOS    | 0x00000000 | 16 KB  | none                           |
//! | EWRAM   | 0x02000000 | 256 KB | every 256 KB through 0x02FFFFFF|
//! | IWRAM   | 0x03000000 | 32 KB  | every 32 KB through 0x03FFFFFF |
//! | I/O     | 0x04000000 | 1 KB   | (0x04000800 quirk unmodeled)   |
//! | Palette | 0x05000000 | 1 KB   | every 1 KB                     |
//! | VRAM    | 0x06000000 | 96 KB  | 64K+32K, 32K doubled, every 128K|
//! | OAM     | 0x07000000 | 1 KB   | every 1 KB                     |
//! | ROM     | 0x08000000 | ≤32 MB | ×3 waitstate images 08/0A/0C   |
//! | SRAM    | 0x0E000000 | 64 KB  | mirrored through 0x0FFFFFFF    |
//!
//! 8-bit write quirks: byte writes to palette and BG VRAM are duplicated
//! into the halfword; byte writes to OBJ VRAM and OAM are ignored.
//!
//! The bus also owns the master clock and the memory-mapped peripherals:
//! video timing events, interrupt latches (IE/IF/IME), the four timers,
//! and the four DMA channels. The `Machine` wrapper drives `tick()` and
//! handles CPU-side interrupt entry.

use crate::backup::{self, BackupKind, Eeprom, Flash};
use crate::bus::Bus;

fn ffta_oam_log_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("FFTA_OAM_LOG").as_deref() == Ok("1"))
}

fn trace_sound_dma() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("RECOMP_TRACE_SDMA").is_some())
}

const DISPSTAT: u32 = 0x0400_0004;
const VCOUNT: u32 = 0x0400_0006;
const KEYINPUT: u32 = 0x0400_0130;
const REG_IE: u32 = 0x0400_0200;
const REG_IF: u32 = 0x0400_0202;
const REG_IME: u32 = 0x0400_0208;
const HALTCNT: u32 = 0x0400_0301;

/// Interrupt source bits in IE/IF.
pub mod irq {
    pub const VBLANK: u16 = 1 << 0;
    pub const HBLANK: u16 = 1 << 1;
    pub const VCOUNT: u16 = 1 << 2;
    pub const TIMER0: u16 = 1 << 3;
    pub const DMA0: u16 = 1 << 8;
    pub const KEYPAD: u16 = 1 << 12;
}

/// Video timing grid (hardware reference, "LCD Dimensions and Timings").
pub const CYCLES_PER_SCANLINE: u64 = 1232;
pub const SCANLINES_PER_FRAME: u64 = 228;
pub const VISIBLE_SCANLINES: u64 = 160;
pub const VISIBLE_WIDTH: usize = 240;
/// Widescreen: extra columns rendered on each side of the 240-px visible
/// area. Chosen at startup with `set_extra` (0 = hardware exact). Buffers
/// are sized for `MAX_EXTRA`, so the value can change without reallocating.
///
/// 648 per side fits a 21:9 view zoomed out 5x (about 1530 columns).
pub const MAX_EXTRA: usize = 648;
/// Largest wide render width.
pub const MAX_RENDER_WIDTH: usize = VISIBLE_WIDTH + 2 * MAX_EXTRA;
/// Zoom-out: extra ROWS rendered above and below the 160-row screen.
/// 320 per side = 5x zoom at any aspect.
pub const MAX_EXTRA_Y: usize = 320;
/// Visible screen height in rows.
pub const VISIBLE_HEIGHT: usize = 160;
/// Largest render height.
pub const MAX_RENDER_HEIGHT: usize = VISIBLE_HEIGHT + 2 * MAX_EXTRA_Y;

static EXTRA_RT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Current extra columns per side (0..=MAX_EXTRA).
pub fn extra() -> usize {
    EXTRA_RT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Set extra columns per side (clamped to MAX_EXTRA). Call before running.
pub fn set_extra(n: usize) {
    EXTRA_RT.store(n.min(MAX_EXTRA), std::sync::atomic::Ordering::Relaxed);
}

/// Current wide render width: 240 + 2 * extra().
pub fn render_width() -> usize {
    VISIBLE_WIDTH + 2 * extra()
}

static EXTRA_Y_RT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Current extra rows above and below the screen (0..=MAX_EXTRA_Y).
pub fn extra_y() -> usize {
    EXTRA_Y_RT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Set extra rows per side (clamped to MAX_EXTRA_Y). Only change it BETWEEN
/// frames (the frame buffer layout depends on it).
pub fn set_extra_y(n: usize) {
    EXTRA_Y_RT.store(n.min(MAX_EXTRA_Y), std::sync::atomic::Ordering::Relaxed);
}

/// Current wide render height: 160 + 2 * extra_y().
pub fn render_height() -> usize {
    VISIBLE_HEIGHT + 2 * extra_y()
}
/// HBlank flag asserts at cycle ~1006 of the scanline, not at 960
/// (hardware-measured; external timing suites test this).
pub const HBLANK_FLAG_CYCLE: u64 = 1006;

#[derive(Default, Clone, Copy)]
#[derive(serde::Serialize, serde::Deserialize)]
struct Timer {
    reload: u16,
    control: u8,
    /// Counter value at `last_sync`.
    count: u16,
    /// Clock at which `count` was correct (prescaler-aligned).
    last_sync: u64,
}

impl Timer {
    fn enabled(&self) -> bool {
        self.control & 0x80 != 0
    }
    fn cascade(&self) -> bool {
        self.control & 0x04 != 0
    }
    fn irq_enabled(&self) -> bool {
        self.control & 0x40 != 0
    }
    fn prescaler_shift(&self) -> u32 {
        [0, 6, 8, 10][(self.control & 3) as usize]
    }
}

#[derive(Default, Clone, Copy)]
#[derive(serde::Serialize, serde::Deserialize)]
struct Dma {
    src: u32,
    dst: u32,
    count: u16,
    control: u16,
    /// Internal latches (set when the channel is enabled).
    cur_src: u32,
    cur_dst: u32,
}

impl Dma {
    fn enabled(&self) -> bool {
        self.control & 0x8000 != 0
    }
    fn timing(&self) -> u16 {
        (self.control >> 12) & 3
    }
    fn repeat(&self) -> bool {
        self.control & 0x0200 != 0
    }
    fn word(&self) -> bool {
        self.control & 0x0400 != 0
    }
    fn irq_enabled(&self) -> bool {
        self.control & 0x4000 != 0
    }
}

/// Video event phase within the current scanline.
#[derive(Clone, Copy, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
enum VPhase {
    Hblank,
    LineEnd,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MemMap {
    #[serde(skip)]
    pub ffta_diagnostics: Option<Box<crate::ffta_log::Diagnostics>>,
    #[serde(skip)]
    pub bios: Vec<u8>,
    pub ewram: Vec<u8>,
    pub iwram: Vec<u8>,
    pub io: Vec<u8>,
    pub palette: Vec<u8>,
    pub vram: Vec<u8>,
    pub oam: Vec<u8>,
    #[serde(skip)]
    pub rom: Vec<u8>,
    #[serde(skip)]
    pub(crate) rom_resources: Option<std::sync::Arc<crate::rom_data::RuntimeResources>>,
    /// Verified, per-cartridge terrain edge policy; never modifies guest memory.
    pub(crate) ffta_bg_edge: crate::ppu::FftaBgEdge,
    pub ffta_sprites: crate::ffta_sprites::SpriteState,
    pub ffta_movement: crate::ffta_movement::MovementState,
    pub ffta_stage: crate::ffta_stage::StageState,
    #[serde(skip)]
    pub ffta_battle_mouse: crate::ffta_battle_mouse::State,
    pub sram: Vec<u8>,

    /// Master cycle counter.
    pub clock: u64,

    // Interrupt state.
    pub reg_ie: u16,
    pub reg_if: u16,
    pub ime: bool,
    pub halted: bool,
    /// HLE IntrWait state: mask being waited on at 0x03007FF8.
    /// True between an IntrWait arming (discard done) and its completion.
    pub intr_wait_armed: bool,

    /// KEYINPUT value (active-low; 0x3FF = nothing pressed).
    pub keys: u16,
    /// KEYCNT condition edge tracker: the keypad IRQ raises when the
    /// match condition transitions false -> true.
    keypad_prev: bool,
    /// Wave RAM idle-bank shadow: the window at 0x90-0x9F (in `io`)
    /// shows the bank NOT selected for playback, so the playing bank's
    /// 16 bytes live here. A SOUND3CNT_L write that flips bit 6 swaps
    /// the two — click-free double-buffering, no data lost.
    wave_shadow: [u8; 16],

    timers: [Timer; 4],
    dma: [Dma; 4],

    // Video event tracking.
    vline_start: u64,
    vphase: VPhase,
    /// Set when a frame completes (start of VBlank); cleared by the runner.
    pub frame_ready: bool,
    pub frames: u64,

    /// 240×160 RGB555 output, written per scanline by the PPU.
    pub framebuffer: Vec<u16>,
    /// Widescreen/zoom output: render_width() x render_height() (stride =
    /// render_width()), column i = screen column i - extra(), row j =
    /// screen row j - extra_y(). Allocated for the maximum size.
    pub wide_framebuffer: Vec<u16>,
    #[serde(skip)]
    pub(crate) zoom_scratch: Vec<u16>,
    pub ffta_dialogue: crate::ffta_dialogue::DialogueState,
    /// Affine BG reference-point accumulators (bg2x, bg2y, bg3x, bg3y),
    /// 19.8 fixed point. Latched from BG2X.. at frame start and whenever
    /// the game writes them; stepped by PB/PD each scanline.
    pub aff_ref: [i32; 4],
    /// Reload request per affine BG (set by I/O writes to BGxX/BGxY).
    pub aff_dirty: [bool; 2],
    /// Bitmask of SWI numbers that were called but not HLE'd (triage aid).
    pub unhandled_swis: u64,
    /// BIOS read-protection value: reads of 0x0000_0000..0x4000 from
    /// outside the BIOS return the last-fetched BIOS opcode. Under HLE
    /// the guest never executes BIOS code, so this is a four-state
    /// machine stepped at the HLE seams (hardware values per the
    /// hardware reference): startup, after-SWI, in-IRQ, after-IRQ.
    pub bios_open: u32,
    /// True when a real BIOS image was loaded (`load_bios`): the guest
    /// executes the BIOS itself — no SWI/IRQ/IntrWait HLE — and
    /// `bios_open` is maintained from actual fetches.
    pub real_bios: bool,
    /// True while the CPU is fetching from the BIOS region (updated by
    /// `note_fetch`, and by the native dispatch loop at block entry).
    /// Region-0 data reads return real bytes only then.
    pub pc_in_bios: bool,
    /// Detected backup medium (drives 0x0D/0x0E region behavior).
    pub backup: BackupKind,
    pub flash: Option<Flash>,
    pub eeprom: Option<Eeprom>,
    /// Cartridge GPIO clock chip, present when the ROM links the Seiko RTC
    /// library. Intercepts 0x080000C4..C9 (the GPIO port) so the guest's
    /// clock reads come from the host's clock. `None` for non-RTC carts,
    /// whose 0xC4 region holds ordinary ROM data.
    pub rtc: Option<crate::rtc::Rtc>,
    /// Direct Sound FIFOs A/B: byte queues fed by writes/DMA at
    /// 0x040000A0/A4, drained one sample per attached-timer overflow.
    fifo: [std::collections::VecDeque<i8>; 2],
    /// Most recent sample popped from each FIFO (the DAC output level).
    pub fifo_sample: [i8; 2],
    /// Mixed stereo output (interleaved L, R) collected at
    /// `AUDIO_SAMPLE_CYCLES` intervals; drained by the frontend.
    pub audio_buf: Vec<i16>,
    /// Per-channel tap for the enhanced audio path (set by the frontend;
    /// observation only, never fed back). When on, every Direct Sound
    /// DAC event is recorded as (sample, driving-timer period in master
    /// cycles) — the channel's own PCM stream at its own rate — and
    /// `psg_tap` carries the PSG mix alone (interleaved stereo) on the
    /// sample grid.
    pub tap_channels: bool,
    pub fifo_tap: [Vec<(i8, u32)>; 2],
    pub psg_tap: Vec<i16>,
    /// MP2K HLE shadow mixer (Stage 3): present when the stock driver
    /// was detected and the enhanced path is on. The dispatch loops
    /// call `mp2k_frame_hook` when PC lands on `SoundMainRAM`.
    pub mp2k: Option<Box<crate::mp2k::Mp2kHle>>,
    /// GAX v1 HLE shadow mixer (same contract as `mp2k`; at most one
    /// engine shadow is armed per session).
    pub gax: Option<Box<crate::gax::GaxHle>>,
    /// In-house (R) driver HLE shadow mixer (same contract).
    pub rdrv: Option<Box<crate::rdrv::RdrvHle>>,
    /// Shadow-mixer stereo output on the tap grid (interleaved L, R;
    /// hardware-rail units scaled to ±0.5 like the play-side floats).
    pub hle_tap: Vec<f32>,
    /// Diagnostics: DAC events that found the FIFO empty (held level),
    /// and sound-DMA refills triggered at the low mark.
    pub fifo_underruns: [u64; 2],
    pub fifo_refills: [u64; 2],
    pub fifo_pushes: [u64; 2],
    audio_cursor: u64,
    apu: crate::apu::Apu,
    next_event: u64,
}

/// 65536 Hz on the 16.78 MHz master clock — the PWM grid most games
/// select via SOUNDBIAS, so the tap loses nothing the hardware kept.
pub const AUDIO_SAMPLE_CYCLES: u64 = 256;
/// The tap rate in Hz (master clock is exactly 2^24).
pub const AUDIO_RATE_HZ: u32 = ((1u64 << 24) / AUDIO_SAMPLE_CYCLES) as u32;

impl MemMap {
    pub(crate) fn validate_state(&self, current: &Self) -> Result<(), String> {
        let lengths = [(&self.ewram, 0x40000), (&self.iwram, 0x8000),
            (&self.io, 0x400), (&self.palette, 0x400), (&self.vram, 0x18000),
            (&self.oam, 0x400), (&self.sram, 0x10000)];
        if lengths.iter().any(|(v, n)| v.len() != *n)
            || self.framebuffer.len() != current.framebuffer.len()
            || self.wide_framebuffer.len() != current.wide_framebuffer.len()
            || self.backup != current.backup || self.real_bios != current.real_bios
            || self.flash.as_ref().map(|v| v.data.len()) != current.flash.as_ref().map(|v| v.data.len())
            || self.eeprom.as_ref().map(|v| v.data.len()) != current.eeprom.as_ref().map(|v| v.data.len())
            || self.rtc.is_some() != current.rtc.is_some()
            || self.vline_start > self.clock || self.audio_cursor > self.clock.saturating_add(AUDIO_SAMPLE_CYCLES)
            || self.fifo.iter().any(|f| f.len() > 32)
        {
            return Err("Invalid or incompatible machine state".into());
        }
        Ok(())
    }
    pub fn new(rom: Vec<u8>) -> MemMap {
        let backup = backup::detect(&rom);
        let flash = match backup {
            BackupKind::Flash64 => Some(Flash::new(false)),
            BackupKind::Flash128 => Some(Flash::new(true)),
            _ => None,
        };
        let eeprom = if backup == BackupKind::Eeprom {
            Some(Eeprom::new())
        } else {
            None
        };
        let rtc = if crate::rtc::detect(&rom) {
            Some(crate::rtc::Rtc::new())
        } else {
            None
        };
        MemMap {
            ffta_bg_edge: crate::ppu::FftaBgEdge::for_rom(&rom),
            ffta_sprites: crate::ffta_sprites::SpriteState::new(&rom),
            ffta_movement: crate::ffta_movement::MovementState::new(&rom),
            ffta_stage: crate::ffta_stage::StageState::new(&rom),
            ffta_battle_mouse: crate::ffta_battle_mouse::State::new(&rom),
            ffta_diagnostics: crate::ffta_log::enabled()
                .then(|| Box::new(crate::ffta_log::Diagnostics::default())),
            bios: vec![0; 0x4000],
            ewram: vec![0; 0x4_0000],
            iwram: vec![0; 0x8000],
            io: vec![0; 0x400],
            palette: vec![0; 0x400],
            vram: vec![0; 0x1_8000],
            oam: vec![0; 0x400],
            rom,
            rom_resources: None,
            sram: vec![0xFF; 0x1_0000],
            clock: 0,
            reg_ie: 0,
            reg_if: 0,
            ime: false,
            halted: false,
            intr_wait_armed: false,
            keys: 0x3FF,
            keypad_prev: false,
            wave_shadow: [0; 16],
            timers: [Timer::default(); 4],
            dma: [Dma::default(); 4],
            vline_start: 0,
            vphase: VPhase::Hblank,
            frame_ready: false,
            frames: 0,
            framebuffer: vec![0; VISIBLE_WIDTH * VISIBLE_SCANLINES as usize],
            wide_framebuffer: vec![0; MAX_RENDER_WIDTH * MAX_RENDER_HEIGHT],
            zoom_scratch: Vec::new(),
            ffta_dialogue: Default::default(),
            aff_ref: [0; 4],
            aff_dirty: [true; 2],
            unhandled_swis: 0,
            bios_open: 0xE129_F000, // after startup
            real_bios: false,
            pc_in_bios: false,
            backup,
            flash,
            eeprom,
            rtc,
            fifo: [
                std::collections::VecDeque::new(),
                std::collections::VecDeque::new(),
            ],
            fifo_sample: [0; 2],
            audio_buf: Vec::new(),
            tap_channels: false,
            fifo_tap: [Vec::new(), Vec::new()],
            psg_tap: Vec::new(),
            mp2k: None,
            gax: None,
            rdrv: None,
            hle_tap: Vec::new(),
            fifo_underruns: [0; 2],
            fifo_refills: [0; 2],
            fifo_pushes: [0; 2],
            audio_cursor: 0,
            apu: crate::apu::Apu::default(),
            next_event: HBLANK_FLAG_CYCLE,
        }
    }

    /// Load a real BIOS image and switch the bus to real-BIOS mode:
    /// region-0 reads return actual bytes while the CPU executes there,
    /// and the read-protection value tracks real fetches.
    pub fn load_bios(&mut self, bytes: &[u8]) {
        let n = bytes.len().min(self.bios.len());
        self.bios[..n].copy_from_slice(&bytes[..n]);
        self.real_bios = true;
    }

    /// Current backup-media contents, for .sav persistence.
    pub fn save_data(&self) -> Option<&[u8]> {
        match self.backup {
            BackupKind::None => None,
            BackupKind::Sram => Some(&self.sram),
            BackupKind::Flash64 | BackupKind::Flash128 => {
                self.flash.as_ref().map(|f| f.data.as_slice())
            }
            BackupKind::Eeprom => self.eeprom.as_ref().map(|e| e.data.as_slice()),
        }
    }

    /// Restore backup-media contents from a .sav file.
    pub fn load_save_data(&mut self, bytes: &[u8]) {
        match self.backup {
            BackupKind::None => {}
            BackupKind::Sram => {
                let n = bytes.len().min(self.sram.len());
                self.sram[..n].copy_from_slice(&bytes[..n]);
            }
            BackupKind::Flash64 | BackupKind::Flash128 => {
                if let Some(f) = &mut self.flash {
                    let n = bytes.len().min(f.data.len());
                    f.data[..n].copy_from_slice(&bytes[..n]);
                }
            }
            BackupKind::Eeprom => {
                if let Some(e) = &mut self.eeprom {
                    let n = bytes.len().min(e.data.len());
                    e.data[..n].copy_from_slice(&bytes[..n]);
                }
            }
        }
    }

    /// EEPROM is addressed through the top of the 0x0D waitstate-2 image.
    fn is_eeprom_addr(&self, addr: u32) -> bool {
        self.eeprom.is_some()
            && addr >> 24 == 0x0D
            && (self.rom.len() <= 0x100_0000 || addr & 0x01FF_FF00 == 0x01FF_FF00)
    }

    /// True when `addr` hits the cartridge GPIO port (0x080000C4..C9, in any
    /// ROM waitstate mirror) and this cart has the clock chip attached.
    fn is_gpio(&self, addr: u32) -> bool {
        self.rtc.is_some() && matches!(addr & 0x01FF_FFFF, 0xC4..=0xC9)
    }

    /// Current scanline (0..=227).
    pub fn scanline(&self) -> u64 {
        (self.vline_start / CYCLES_PER_SCANLINE) % SCANLINES_PER_FRAME
    }

    /// True when an enabled interrupt is latched (wakes Halt regardless of IME).
    pub fn irq_line(&self) -> bool {
        self.reg_ie & self.reg_if != 0
    }

    /// True when the CPU should take the IRQ exception.
    pub fn irq_pending(&self) -> bool {
        self.ime && self.irq_line()
    }

    /// Native scheduling allowance, derived without advancing guest time.
    /// A deferred event already due grants no cycles.
    pub(crate) fn cycles_until_event(&self) -> u64 {
        self.next_event.saturating_sub(self.clock)
    }

    /// Advance the master clock and process any due events, generating
    /// audio grid samples up to each event first so they see the DAC/PSG
    /// state that was current at their own instants.
    pub fn tick(&mut self, cycles: u64) {
        self.advance_clock::<true>(cycles);
    }

    /// Service events at their own clock, not at the end of a native tick batch.
    /// DMA steals extend the endpoint; they must not consume the caller's cycles.
    /// During an active DMA only timers may preempt it: processing video here
    /// could start the same still-enabled transfer again before it completes.
    fn advance_clock<const VIDEO: bool>(&mut self, cycles: u64) {
        let mut end = self.clock + cycles;
        loop {
            let event = if VIDEO {
                self.next_event
            } else {
                (0..4).filter_map(|i| self.timer_overflow_time(i)).min().unwrap_or(u64::MAX)
            };
            if event > end {
                break;
            }
            // A DMA may have crossed a video boundary while video was deferred.
            // Never rewind the clock when servicing that overdue event.
            self.clock = self.clock.max(event);
            self.pump_audio(self.clock);
            let before = self.clock;
            if VIDEO {
                self.process_events();
            } else {
                self.advance_timers();
            }
            end += self.clock - before;
        }
        self.clock = self.clock.max(end);
        self.pump_audio(self.clock);
        if !VIDEO {
            self.recompute_next_event();
        }
    }

    /// Generate audio samples on the 65536 Hz grid up to `until` (capped
    /// at the clock). Every path that advances the clock in bulk must
    /// pump before state changes land, or DAC transitions inside the
    /// gap collapse into one held ledge (an audible thump per gap).
    fn pump_audio(&mut self, until: u64) {
        let until = until.min(self.clock);
        // Android fast path: no mixer work before the next sample is due.
        if until < self.audio_cursor {
            return;
        }
        // The shadow mixers render on the same grid; hoisted out of
        // `self` so they can read guest memory while we mutate the taps.
        let mut mp2k = self.mp2k.take();
        let mut gax = self.gax.take();
        let mut rdrv = self.rdrv.take();
        while self.audio_cursor <= until {
            // Mix Direct Sound DAC levels with the PSG channels into a
            // stereo pair, honoring the SOUNDCNT routing the hardware
            // applies: per-side FIFO enables and the 50%/100% volume
            // bits (SOUNDCNT_H), per-channel/volume PSG routing inside
            // the APU (SOUNDCNT_L), and the master enable (SOUNDCNT_X).
            // Each side then clips at the hardware's 10-bit rail — that
            // clip is canon audible behavior on hot mixes.
            let (psg_l, psg_r) =
                self.apu
                    .sample(&self.io, &self.wave_shadow, AUDIO_SAMPLE_CYCLES as u32);
            let cnt_h = u16::from_le_bytes([self.io[0x82], self.io[0x83]]);
            let a = (self.fifo_sample[0] as i32 * 64) >> ((cnt_h >> 2 & 1) ^ 1);
            let b = (self.fifo_sample[1] as i32 * 64) >> ((cnt_h >> 3 & 1) ^ 1);
            let mut l = psg_l as i32;
            let mut r = psg_r as i32;
            if cnt_h & (1 << 9) != 0 {
                l += a;
            }
            if cnt_h & (1 << 8) != 0 {
                r += a;
            }
            if cnt_h & (1 << 13) != 0 {
                l += b;
            }
            if cnt_h & (1 << 12) != 0 {
                r += b;
            }
            if self.io[0x84] & 0x80 == 0 {
                l = 0;
                r = 0;
            }
            // Rail: ±0x200 hardware units; our scale is 32 per unit.
            let l = l.clamp(-16384, 16383) as i16;
            let r = r.clamp(-16384, 16383) as i16;
            if self.audio_buf.len() < 0x2_0000 {
                self.audio_buf.push(l);
                self.audio_buf.push(r);
            }
            if self.tap_channels && self.psg_tap.len() < 0x2_0000 {
                self.psg_tap.push(psg_l);
                self.psg_tap.push(psg_r);
            }
            if self.tap_channels {
                if let Some(h) = mp2k.as_deref_mut() {
                    if h.active && h.engaged {
                        let mem = crate::mp2k::MemView {
                            rom: &self.rom,
                            ewram: &self.ewram,
                            iwram: &self.iwram,
                        };
                        let (hl, hr) = h.render(&mem, self.fifo_sample);
                        if self.hle_tap.len() < 0x2_0000 {
                            self.hle_tap.push(hl);
                            self.hle_tap.push(hr);
                        }
                    }
                }
                if let Some(g) = gax.as_deref_mut() {
                    if g.active {
                        let mem = crate::mp2k::MemView {
                            rom: &self.rom,
                            ewram: &self.ewram,
                            iwram: &self.iwram,
                        };
                        if !g.engaged {
                            // The driver initializes some time into
                            // boot; probe until its function pointers
                            // appear and verify against the known
                            // mixer family.
                            g.try_engage(&mem);
                        } else {
                            let (hl, hr) = g.render(&mem, self.fifo_sample, self.audio_cursor);
                            if self.hle_tap.len() < 0x2_0000 {
                                self.hle_tap.push(hl);
                                self.hle_tap.push(hr);
                            }
                        }
                    }
                }
                if let Some(rr) = rdrv.as_deref_mut() {
                    if rr.active {
                        let mem = crate::mp2k::MemView {
                            rom: &self.rom,
                            ewram: &self.ewram,
                            iwram: &self.iwram,
                        };
                        if !rr.engaged {
                            // Probe until the driver's IWRAM blob copy
                            // appears, then harvest its layout.
                            rr.try_engage(&mem);
                        } else {
                            let (hl, hr) = rr.render(&mem, self.fifo_sample, self.audio_cursor);
                            if self.hle_tap.len() < 0x2_0000 {
                                self.hle_tap.push(hl);
                                self.hle_tap.push(hr);
                            }
                        }
                    }
                }
            }
            self.audio_cursor += AUDIO_SAMPLE_CYCLES;
        }
        self.mp2k = mp2k;
        self.gax = gax;
        self.rdrv = rdrv;
    }

    /// GAX HLE hook: dispatch loops call this when PC lands on one of
    /// the shadow's hook keys. `r0` is the guest's first argument
    /// register (the v3 mixer receives its arg block there).
    pub fn gax_frame_hook(&mut self, key: u32, r0: u32) {
        if let Some(mut g) = self.gax.take() {
            let mem = crate::mp2k::MemView {
                rom: &self.rom,
                ewram: &self.ewram,
                iwram: &self.iwram,
            };
            g.frame_hook(&mem, self.audio_cursor, key, r0);
            self.gax = Some(g);
        }
    }

    /// In-house (R) HLE hook: dispatch loops call this when PC lands
    /// on the shadow's mix or advance entries.
    pub fn rdrv_frame_hook(&mut self, key: u32) {
        if let Some(mut rr) = self.rdrv.take() {
            let mem = crate::mp2k::MemView {
                rom: &self.rom,
                ewram: &self.ewram,
                iwram: &self.iwram,
            };
            rr.frame_hook(&mem, self.audio_cursor, key);
            self.rdrv = Some(rr);
        }
    }

    /// MP2K HLE per-tick hook: the dispatch loops call this when PC
    /// lands on the detected SoundMainRAM entry.
    pub fn mp2k_frame_hook(&mut self, key: u32) {
        if let Some(mut h) = self.mp2k.take() {
            let mem = crate::mp2k::MemView {
                rom: &self.rom,
                ewram: &self.ewram,
                iwram: &self.iwram,
            };
            h.frame_hook(&mem, self.audio_cursor, key);
            self.mp2k = Some(h);
        }
    }

    /// Jump the clock to the next event (used while halted/waiting).
    pub fn skip_to_next_event(&mut self) {
        self.tick(self.next_event.saturating_sub(self.clock));
    }

    fn video_due(&self) -> u64 {
        match self.vphase {
            VPhase::Hblank => self.vline_start + HBLANK_FLAG_CYCLE,
            VPhase::LineEnd => self.vline_start + CYCLES_PER_SCANLINE,
        }
    }

    fn process_events(&mut self) {
        // Timers first (their overflow time may precede the video event).
        self.advance_timers();
        if self.clock >= self.video_due() {
            self.process_video_event();
        }
        self.recompute_next_event();
    }

    fn recompute_next_event(&mut self) {
        let mut next = self.video_due();
        for i in 0..4 {
            if let Some(t) = self.timer_overflow_time(i) {
                next = next.min(t);
            }
        }
        self.next_event = next;
    }

    fn process_video_event(&mut self) {
        match self.vphase {
            VPhase::Hblank => {
                let line = self.scanline();
                if line < VISIBLE_SCANLINES {
                    crate::ppu::render_scanline(self, line as usize);
                    self.trigger_dma(2); // HBlank timing (visible lines only)
                }
                if self.io[(DISPSTAT & 0x3FF) as usize] & 0x10 != 0 {
                    self.raise_irq(irq::HBLANK);
                }
                self.vphase = VPhase::LineEnd;
            }
            VPhase::LineEnd => {
                self.vline_start += CYCLES_PER_SCANLINE;
                self.vphase = VPhase::Hblank;
                self.check_keypad();
                let line = self.scanline();
                let dispstat = self.io[(DISPSTAT & 0x3FF) as usize];
                let vcount_setting = self.io[(DISPSTAT & 0x3FF) as usize + 1];
                if line == vcount_setting as u64 && dispstat & 0x20 != 0 {
                    self.raise_irq(irq::VCOUNT);
                }
                if line == VISIBLE_SCANLINES {
                    crate::ppu::render_ffta_zoom_rows(self);
                    self.frame_ready = true;
                    self.frames += 1;
                    if self.frames % 30 == 0 {
                        if let Some(log) = self.ffta_diagnostics.as_mut() {
                            log.sample(self.frames, &self.io, &self.ewram, &self.iwram, &self.vram);
                        }
                    }
                    if ffta_oam_log_enabled() && self.frames % 30 == 0 {
                        let sizes = [
                            [(8, 8), (16, 16), (32, 32), (64, 64)],
                            [(16, 8), (32, 8), (32, 16), (64, 32)],
                            [(8, 16), (8, 32), (16, 32), (32, 64)],
                        ];
                        let (mut enabled, mut visible, mut outside_x, mut vertical) = (0, 0, 0, 0);
                        for entry in self.oam.chunks_exact(8) {
                            let a0 = u16::from_le_bytes([entry[0], entry[1]]);
                            let a1 = u16::from_le_bytes([entry[2], entry[3]]);
                            let shape = (a0 >> 14) as usize;
                            if shape == 3 || (a0 & 0x100 == 0 && a0 & 0x200 != 0) {
                                continue;
                            }
                            enabled += 1;
                            let (mut w, mut h) = sizes[shape][(a1 >> 14) as usize];
                            if a0 & 0x300 == 0x300 {
                                w *= 2;
                                h *= 2;
                            }
                            let mut y = (a0 & 255) as i32;
                            if y >= 160 {
                                y -= 256;
                            }
                            // Canonical signed 9-bit X for diagnostics, independent of zoom.
                            let raw_x = (a1 & 511) as i32;
                            let x = if raw_x >= 256 { raw_x - 512 } else { raw_x };
                            if x < -16 || x >= 240 {
                                outside_x += 1;
                            }
                            if y < 160 && y + h > 0 {
                                vertical += 1;
                                if x < 240 && x + w > 0 {
                                    visible += 1;
                                }
                            }
                        }
                        let obj_nonzero = self.vram[0x10000..0x18000]
                            .iter()
                            .filter(|&&b| b != 0)
                            .count();
                        eprintln!(
                            "FFTA_OAM frame={} enabled={} visible_bbox_240x160={} outside_x={} vertical={} obj_vram_nonzero_bytes={} extra={}",
                            self.frames,
                            enabled,
                            visible,
                            outside_x,
                            vertical,
                            obj_nonzero,
                            extra()
                        );
                        for bg in 0..4 {
                            let word_at =
                                |off: usize| u16::from_le_bytes([self.io[off], self.io[off + 1]]);
                            eprintln!(
                                "FFTA_BG frame={} bg={} control=0x{:04X} hofs={} vofs={} dispcnt=0x{:04X}",
                                self.frames,
                                bg,
                                word_at(8 + bg * 2),
                                word_at(0x10 + bg * 4) & 511,
                                word_at(0x12 + bg * 4) & 511,
                                word_at(0)
                            );
                        }
                    }
                    if dispstat & 0x08 != 0 {
                        self.raise_irq(irq::VBLANK);
                    }
                    self.trigger_dma(1); // VBlank timing
                }
            }
        }
    }

    pub fn raise_irq(&mut self, bits: u16) {
        self.reg_if |= bits;
    }

    /// Evaluate the KEYCNT match condition (bits 0-9 select, bit 14
    /// IRQ enable, bit 15: 0 = any selected pressed, 1 = all selected
    /// pressed; KEYINPUT is active-low) and raise the keypad IRQ on a
    /// false -> true transition. Runs on KEYCNT writes and once per
    /// scanline — `keys` is written directly by frontends, so there is
    /// no write hook to piggyback on.
    fn check_keypad(&mut self) {
        let keycnt = u16::from_le_bytes([self.io[0x132], self.io[0x133]]);
        let sel = keycnt & 0x3FF;
        let pressed = !self.keys & 0x3FF;
        let cond = if keycnt & 0x8000 != 0 {
            sel != 0 && pressed & sel == sel
        } else {
            pressed & sel != 0
        };
        let met = keycnt & 0x4000 != 0 && cond;
        if met && !self.keypad_prev {
            self.raise_irq(irq::KEYPAD);
        }
        self.keypad_prev = met;
    }

    // ---- timers ----

    /// Clock at which timer `i` will next overflow (non-cascade only).
    fn timer_overflow_time(&self, i: usize) -> Option<u64> {
        let t = &self.timers[i];
        if !t.enabled() || t.cascade() {
            return None;
        }
        let remaining = (0x1_0000 - t.count as u64) << t.prescaler_shift();
        Some(t.last_sync + remaining)
    }

    fn advance_timers(&mut self) {
        for i in 0..4 {
            let t = self.timers[i];
            if !t.enabled() || t.cascade() {
                continue;
            }
            let shift = t.prescaler_shift();
            let elapsed = (self.clock.saturating_sub(t.last_sync)) >> shift;
            if elapsed == 0 {
                continue;
            }
            let total = t.count as u64 + elapsed;
            self.timers[i].last_sync += elapsed << shift;
            if total < 0x1_0000 {
                self.timers[i].count = total as u16;
            } else {
                let period = (0x1_0000 - t.reload as u64).max(1);
                let past = total - 0x1_0000;
                let overflows = 1 + past / period;
                self.timers[i].count = (t.reload as u64 + past % period) as u16;
                self.timer_overflowed(i, overflows);
            }
        }
    }

    fn timer_overflowed(&mut self, i: usize, times: u64) {
        if self.timers[i].irq_enabled() {
            self.raise_irq(irq::TIMER0 << i);
        }
        // Direct Sound: timers 0/1 clock the FIFOs selected in SOUNDCNT_H.
        if i < 2 {
            let t = &self.timers[i];
            let period = (((0x1_0000 - t.reload as u64).max(1)) << t.prescaler_shift()) as u32;
            let soundcnt_h = u16::from_le_bytes([self.io[0x82], self.io[0x83]]);
            for f in 0..2 {
                let timer_sel = (soundcnt_h >> (10 + f * 4)) & 1;
                if timer_sel as usize == i {
                    self.drain_fifo(f, times, period);
                }
            }
        }
        // Cascade into the next timer.
        if i < 3 && self.timers[i + 1].enabled() && self.timers[i + 1].cascade() {
            for _ in 0..times {
                let next = &mut self.timers[i + 1];
                let (count, wrapped) = next.count.overflowing_add(1);
                next.count = if wrapped { next.reload } else { count };
                if wrapped {
                    self.timer_overflowed(i + 1, 1);
                }
            }
        }
    }

    fn timer_read_count(&mut self, i: usize) -> u16 {
        self.advance_timers();
        self.timers[i].count
    }

    fn timer_write_control(&mut self, i: usize, value: u8) {
        self.advance_timers();
        let was_enabled = self.timers[i].enabled();
        self.timers[i].control = value;
        if !was_enabled && self.timers[i].enabled() {
            // Enabling reloads the counter.
            self.timers[i].count = self.timers[i].reload;
            self.timers[i].last_sync = self.clock;
        }
        self.recompute_next_event();
    }

    /// Consume samples from a FIFO; refill via sound DMA at the low mark.
    /// `period` is the driving timer's period in master cycles — the
    /// channel's sample spacing, recorded with each DAC event when the
    /// per-channel tap is on (an underrun holds the level, exactly like
    /// the hardware DAC, so the tap stays a uniform stream).
    fn drain_fifo(&mut self, f: usize, times: u64, period: u32) {
        for _ in 0..times {
            match self.fifo[f].pop_front() {
                Some(s) => self.fifo_sample[f] = s,
                None => {
                    self.fifo_underruns[f] += 1;
                    if f == 0
                        && self.fifo_underruns[0] % 200 == 1
                        && trace_sound_dma()
                    {
                        let d = &self.dma[1];
                        eprintln!(
                            "underrun#{} clock={} dma1: en={} timing={} repeat={} dst={:08x} src={:08x} cnt_h={:04x}",
                            self.fifo_underruns[0],
                            self.clock,
                            d.enabled(),
                            d.timing(),
                            d.repeat(),
                            d.cur_dst,
                            d.cur_src,
                            d.control
                        );
                    }
                }
            }
            if self.tap_channels && self.fifo_tap[f].len() < 0x1_0000 {
                self.fifo_tap[f].push((self.fifo_sample[f], period));
            }
            // The hardware raises the FIFO DMA request per overflow at
            // the low mark — refill inside the loop, or a batched timer
            // catch-up (large `times`) blows through the 32-byte FIFO
            // and manufactures underruns the hardware never had.
            if self.fifo[f].len() <= 16 {
                let fifo_addr = 0x0400_00A0 + 4 * f as u32;
                for ch in 1..=2usize {
                    if self.dma[ch].enabled()
                        && self.dma[ch].timing() == 3
                        && self.dma[ch].cur_dst & !3 == fifo_addr
                    {
                        self.fifo_refills[f] += 1;
                        if self.fifo_refills[f] % 64 == 1 && trace_sound_dma()
                        {
                            eprintln!(
                                "sdma f{f} ch{ch} refill#{} src={:08x} clock={}",
                                self.fifo_refills[f], self.dma[ch].cur_src, self.clock
                            );
                        }
                        self.run_dma(ch);
                    }
                }
            }
        }
    }

    /// Hardware FIFO depth is 32 bytes; a push to a full FIFO drops the
    /// incoming byte.
    fn fifo_push(&mut self, f: usize, value: u8) {
        self.fifo_pushes[f] += 1;
        if self.fifo[f].len() < 32 {
            self.fifo[f].push_back(value as i8);
        }
    }

    // ---- DMA ----

    /// Fire all enabled channels with the given start timing
    /// (0 = immediate, 1 = vblank, 2 = hblank, 3 = special).
    fn trigger_dma(&mut self, timing: u16) {
        for i in 0..4 {
            if self.dma[i].enabled() && self.dma[i].timing() == timing {
                self.run_dma(i);
            }
        }
    }

    fn run_dma(&mut self, i: usize) {
        let ch = self.dma[i];
        // Sound FIFO mode: always 4 words, 32-bit, fixed destination.
        let sound_mode = ch.timing() == 3 && (i == 1 || i == 2);
        let max: u32 = if i == 3 { 0x1_0000 } else { 0x4000 };
        let count = if sound_mode {
            4
        } else if ch.count == 0 {
            max
        } else {
            ch.count as u32
        };
        let word = ch.word() || sound_mode;
        let unit: u32 = if word { 4 } else { 2 };

        let dst_adj = if sound_mode { 2 } else { (ch.control >> 5) & 3 };
        let src_adj = (ch.control >> 7) & 3;

        let mut src = ch.cur_src & if word { !3 } else { !1 };
        let mut dst = ch.cur_dst & if word { !3 } else { !1 };
        let sprite_transfer = (src, dst);

        if let Some(log) = self.ffta_diagnostics.as_mut() {
            if (0x06000000..0x07000400).contains(&dst) {
                let context = log.dma_context[i];
                eprintln!(
                    "FFTA_DMA_CALLER frame={} channel={} src=0x{:08X} dst=0x{:08X} count={} unit={} timing={} arm_pc=0x{:08X} arm_lr=0x{:08X} arm_mode={}",
                    self.frames,
                    i,
                    src,
                    dst,
                    count,
                    unit,
                    ch.timing(),
                    context.pc,
                    context.lr,
                    if context.thumb { "Thumb" } else { "ARM" }
                );
            }
            log.dma_depth += 1;
        }

        if ffta_oam_log_enabled() && (dst >> 24 == 7 || (0x0600_0000..0x0601_8000).contains(&dst)) {
            eprintln!(
                "FFTA_DMA frame={} channel={} src=0x{:08X} dst=0x{:08X} count={} unit={} timing={} control=0x{:04X}",
                self.frames,
                i,
                src,
                dst,
                count,
                unit,
                ch.timing(),
                ch.control
            );
        }

        let mut done = 0u32;
        for _ in 0..count {
            if word {
                let v = self.read32(src);
                self.write32(dst, v);
            } else {
                let v = self.read16(src);
                self.write16(dst, v);
            }
            src = match src_adj {
                0 => src.wrapping_add(unit),
                1 => src.wrapping_sub(unit),
                _ => src, // 2 = fixed (3 is prohibited)
            };
            dst = match dst_adj {
                0 | 3 => dst.wrapping_add(unit),
                1 => dst.wrapping_sub(unit),
                _ => dst,
            };
            done += 1;
            // Hardware gives the sound FIFO channels priority: they
            // preempt a long lower-priority transfer mid-flight. Account
            // the clock chunkwise and service the timers so FIFO DMA can
            // interleave — otherwise a big graphics/overlay copy starves
            // the FIFO for its whole duration (audible ~1-2 ms dropouts).
            if !sound_mode && done % 16 == 0 {
                self.advance_clock::<false>(32);
            }
        }

        self.dma[i].cur_src = src;
        self.ffta_movement.observe_dma(
            sprite_transfer.0,
            sprite_transfer.1,
            count,
            unit,
            ch.control,
            &self.ewram,
            &self.vram,
        );
        self.ffta_stage.observe_dma(
            sprite_transfer.0,
            sprite_transfer.1,
            count,
            unit,
            ch.control,
            &self.ewram,
            &self.vram,
        );
        self.ffta_sprites.publish(
            sprite_transfer.0,
            sprite_transfer.1,
            (count * unit) as usize,
            extra() > 0 || extra_y() > 0,
            &self.oam,
        );
        if ffta_oam_log_enabled()
            && self.frames % 30 == 0
            && (0x07000000..0x07000400).contains(&sprite_transfer.1)
        {
            crate::ffta_sprites::tile_diagnostics(self);
        }
        if let Some(log) = self.ffta_diagnostics.as_mut() {
            log.dma_depth -= 1;
        }
        // Rough cycle cost: 2 cycles per unit transferred plus setup
        // (minus what the chunked accounting above already added).
        let accounted = if sound_mode {
            0
        } else {
            (count as u64 / 16) * 32
        };
        self.advance_clock::<false>(2 * count as u64 + 4 - accounted);

        // An EEPROM request is exactly one DMA; the end of the transfer
        // delimits the serial bitstream.
        if ch.cur_dst >> 24 == 0x0D {
            if let Some(e) = &mut self.eeprom {
                e.flush();
            }
        }

        if ch.repeat() && ch.timing() != 0 {
            // Repeat: count reloads; destination reloads in mode 3.
            if dst_adj == 3 {
                self.dma[i].cur_dst = ch.cur_dst;
            } else {
                self.dma[i].cur_dst = dst;
            }
        } else {
            self.dma[i].cur_dst = dst;
            self.dma[i].control &= !0x8000; // channel disables itself
            self.io[0xBB + i * 12] = (self.dma[i].control >> 8) as u8;
        }

        if ch.irq_enabled() {
            self.raise_irq(irq::DMA0 << i);
        }
    }

    fn dma_write_control_hi(&mut self, i: usize, value: u8) {
        let was_enabled = self.dma[i].enabled();
        self.dma[i].control = (self.dma[i].control & 0x00FF) | ((value as u16) << 8);
        if !was_enabled && self.dma[i].enabled() {
            if let Some(log) = self.ffta_diagnostics.as_mut() {
                log.dma_context[i] = log.context;
            }
            // Latch internal registers on enable.
            self.dma[i].cur_src = self.dma[i].src;
            self.dma[i].cur_dst = self.dma[i].dst;
            if self.dma[i].timing() == 0 {
                self.run_dma(i);
            }
        }
    }

    /// Route an I/O-register byte write to the owning peripheral.
    fn io_write8(&mut self, off: u32, value: u8) {
        // Diagnostic (RECOMP_TRACE_IOW): scroll/blend register write
        // timing, for scanline-effect divergence between dispatch paths.
        {
            static TRACE_IOW: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *TRACE_IOW.get_or_init(|| std::env::var_os("RECOMP_TRACE_IOW").is_some())
                && (off < 2
                    || (0x10..0x20).contains(&off)
                    || (0x40..0x4C).contains(&off)
                    || (0x50..0x56).contains(&off))
            {
                eprintln!(
                    "IOW f={} line={} lcyc={} off={:02x} val={:02x}",
                    self.frames,
                    self.scanline(),
                    self.clock.wrapping_sub(self.vline_start),
                    off,
                    value
                );
            }
        }
        match 0x0400_0000 + off {
            REG_IE => self.reg_ie = (self.reg_ie & 0xFF00) | value as u16,
            x if x == REG_IE + 1 => self.reg_ie = (self.reg_ie & 0x00FF) | ((value as u16) << 8),
            // IF: write-1-to-clear.
            REG_IF => self.reg_if &= !(value as u16),
            x if x == REG_IF + 1 => self.reg_if &= !((value as u16) << 8),
            REG_IME => self.ime = value & 1 != 0,
            HALTCNT => self.halted = true,
            // Timers: reload low/high, control.
            x if (0x0400_0100..0x0400_0110).contains(&x) => {
                self.io[off as usize] = value;
                let i = ((x - 0x0400_0100) / 4) as usize;
                match (x - 0x0400_0100) % 4 {
                    0 => self.timers[i].reload = (self.timers[i].reload & 0xFF00) | value as u16,
                    1 => {
                        self.timers[i].reload =
                            (self.timers[i].reload & 0x00FF) | ((value as u16) << 8)
                    }
                    2 => self.timer_write_control(i, value),
                    _ => {}
                }
            }
            // DMA registers. The raw store happens first: an immediate
            // transfer may clear the enable bit again from run_dma.
            x if (0x0400_00B0..0x0400_00E0).contains(&x) => {
                self.io[off as usize] = value;
                let rel = (x - 0x0400_00B0) as usize;
                let i = rel / 12;
                let field = rel % 12;
                match field {
                    0..=3 => {
                        let shift = field * 8;
                        self.dma[i].src =
                            (self.dma[i].src & !(0xFF << shift)) | ((value as u32) << shift);
                    }
                    4..=7 => {
                        let shift = (field - 4) * 8;
                        self.dma[i].dst =
                            (self.dma[i].dst & !(0xFF << shift)) | ((value as u32) << shift);
                    }
                    8 => self.dma[i].count = (self.dma[i].count & 0xFF00) | value as u16,
                    9 => self.dma[i].count = (self.dma[i].count & 0x00FF) | ((value as u16) << 8),
                    10 => self.dma[i].control = (self.dma[i].control & 0xFF00) | value as u16,
                    11 => self.dma_write_control_hi(i, value),
                    _ => unreachable!(),
                }
            }
            // PSG channel registers.
            x if (0x0400_0060..0x0400_0080).contains(&x) => {
                // SOUND3CNT_L bit 6 flip: the playing bank and the
                // window-visible bank trade places.
                if x == 0x0400_0070 && (self.io[0x70] ^ value) & 0x40 != 0 {
                    for i in 0..16 {
                        std::mem::swap(&mut self.wave_shadow[i], &mut self.io[0x90 + i]);
                    }
                }
                self.io[off as usize] = value;
                self.apu.write8(&self.io, off as usize, value);
            }
            // Direct Sound FIFO data ports.
            x if (0x0400_00A0..0x0400_00A8).contains(&x) => {
                let f = ((x - 0x0400_00A0) / 4) as usize;
                self.fifo_push(f, value);
            }
            // SOUNDCNT_H: FIFO reset bits (11/15) clear the queues.
            0x0400_0083 => {
                self.io[off as usize] = value;
                if value & 0x08 != 0 {
                    self.fifo[0].clear();
                }
                if value & 0x80 != 0 {
                    self.fifo[1].clear();
                }
            }
            // KEYCNT: re-evaluate the keypad IRQ condition.
            0x0400_0132 | 0x0400_0133 => {
                self.io[off as usize] = value;
                self.check_keypad();
            }
            // Affine BG reference points: request an accumulator reload.
            x if (0x0400_0028..0x0400_0030).contains(&x) => {
                self.io[off as usize] = value;
                self.aff_dirty[0] = true;
            }
            x if (0x0400_0038..0x0400_0040).contains(&x) => {
                self.io[off as usize] = value;
                self.aff_dirty[1] = true;
            }
            _ => self.io[off as usize] = value,
        }
    }

    fn io_read8(&mut self, off: u32) -> u8 {
        match 0x0400_0000 + off {
            DISPSTAT => self.dispstat_low(),
            VCOUNT => ((self.clock / CYCLES_PER_SCANLINE) % SCANLINES_PER_FRAME) as u8,
            KEYINPUT => self.keys as u8,
            x if x == KEYINPUT + 1 => (self.keys >> 8) as u8,
            REG_IE => self.reg_ie as u8,
            x if x == REG_IE + 1 => (self.reg_ie >> 8) as u8,
            REG_IF => self.reg_if as u8,
            x if x == REG_IF + 1 => (self.reg_if >> 8) as u8,
            REG_IME => self.ime as u8,
            // SOUNDCNT_X: bits 0-3 read back live PSG channel-active
            // flags; with the master enable off everything reads clear.
            0x0400_0084 => {
                let master = self.io[0x84] & 0x80;
                if master == 0 {
                    0
                } else {
                    master | self.apu.active()
                }
            }
            x if (0x0400_0100..0x0400_0110).contains(&x) && (x % 4 < 2) => {
                let i = ((x - 0x0400_0100) / 4) as usize;
                let count = self.timer_read_count(i);
                if x % 4 == 0 {
                    count as u8
                } else {
                    (count >> 8) as u8
                }
            }
            _ => self.io[off as usize],
        }
    }

    /// DISPSTAT bits 0-2 derived from the clock; bits 3+ from the backing
    /// store (IRQ enables, VCount setting).
    fn dispstat_low(&self) -> u8 {
        let line = (self.clock / CYCLES_PER_SCANLINE) % SCANLINES_PER_FRAME;
        let dot_cycle = self.clock % CYCLES_PER_SCANLINE;
        let stored = self.io[(DISPSTAT & 0x3FF) as usize];
        let mut value = stored & !0x07;
        // VBlank flag: lines 160..=226 (not 227).
        if (VISIBLE_SCANLINES..SCANLINES_PER_FRAME - 1).contains(&line) {
            value |= 1;
        }
        if dot_cycle >= HBLANK_FLAG_CYCLE {
            value |= 2;
        }
        let vcount_setting = self.io[(DISPSTAT & 0x3FF) as usize + 1];
        if line == vcount_setting as u64 {
            value |= 4;
        }
        value
    }

    /// VRAM's 96 KB maps into a 128 KB window as 64K + 32K with the 32K
    /// block appearing twice.
    fn vram_index(addr: u32) -> usize {
        let mut off = (addr & 0x1_FFFF) as usize;
        if off >= 0x1_8000 {
            off -= 0x8000;
        }
        off
    }

    fn rom_index(addr: u32) -> usize {
        (addr & 0x01FF_FFFF) as usize
    }

    /// Byte-composed read16, identical to the `Bus` trait default. The
    /// native fast paths below fall back to this wherever wide access
    /// isn't provably equivalent (I/O side effects, region edges, the
    /// VRAM fold, EEPROM, flash, open bus).
    fn rd16_bytes(&mut self, addr: u32) -> u16 {
        let lo = self.read8(addr) as u16;
        let hi = self.read8(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    /// Halfword-composed read32, identical to the `Bus` trait default
    /// (which composes two read16s — themselves byte-equivalent).
    fn rd32_halves(&mut self, addr: u32) -> u32 {
        let lo = self.read16(addr) as u32;
        let hi = self.read16(addr.wrapping_add(2)) as u32;
        lo | (hi << 16)
    }

    /// Reads past the cartridge's physical end return an address-derived
    /// 16-bit pattern (hardware reference: cartridge bus with nothing
    /// driving it).
    fn rom_open_bus(addr: u32) -> u8 {
        let half = (addr >> 1) & 0xFFFF;
        if addr & 1 == 0 {
            half as u8
        } else {
            (half >> 8) as u8
        }
    }
}

impl Bus for MemMap {
    fn decoded_base_palette(&self, source: u32) -> Option<crate::rom_data::PaletteRead> {
        self.rom_resources.as_ref()?.decoded_palette(source)
    }

    fn read8(&mut self, addr: u32) -> u8 {
        match addr >> 24 {
            0x00 => {
                if addr < 0x4000 {
                    if self.pc_in_bios {
                        // Real-BIOS execution: code running in the BIOS
                        // reads its own bytes.
                        self.bios[addr as usize]
                    } else {
                        // BIOS read protection: from outside, the guest
                        // sees the last-fetched BIOS opcode, byte-laned
                        // by address.
                        (self.bios_open >> ((addr & 3) * 8)) as u8
                    }
                } else {
                    // Open bus: the floating bus carries the current
                    // instruction prefetch, byte-laned by address.
                    (self.bios_open >> ((addr & 3) * 8)) as u8
                }
            }
            0x02 => self.ewram[(addr & 0x3_FFFF) as usize],
            0x03 => self.iwram[(addr & 0x7FFF) as usize],
            0x04 => {
                let off = addr & 0x00FF_FFFF;
                if off < 0x400 { self.io_read8(off) } else { 0 }
            }
            0x05 => self.palette[(addr & 0x3FF) as usize],
            0x06 => self.vram[Self::vram_index(addr)],
            0x07 => self.oam[(addr & 0x3FF) as usize],
            0x08..=0x0D => {
                if self.is_gpio(addr) {
                    return self.rtc.as_ref().unwrap().read(addr & 0x01FF_FFFF);
                }
                if self.is_eeprom_addr(addr) {
                    return if addr & 1 == 0 {
                        self.eeprom.as_mut().unwrap().read_bit() as u8
                    } else {
                        0
                    };
                }
                let idx = Self::rom_index(addr);
                if idx < self.rom.len() {
                    if Self::resource_page(idx) {
                        if let Some(bytes) = self.rom_resources.as_ref().and_then(|r| r.guest_read::<1>(idx, &self.rom)) {
                            return bytes[0];
                        }
                    }
                    self.rom[idx]
                } else {
                    Self::rom_open_bus(addr)
                }
            }
            0x0E | 0x0F => match &self.flash {
                Some(flash) => flash.read(addr),
                None => self.sram[(addr & 0xFFFF) as usize],
            },
            // Open bus (see region 0): the last prefetched word.
            _ => (self.bios_open >> ((addr & 3) * 8)) as u8,
        }
    }

    fn write8(&mut self, addr: u32, value: u8) {
        if let Some(log) = self.ffta_diagnostics.as_mut() {
            log.write(self.frames, addr, 1, value as u32);
        }
        match addr >> 24 {
            0x02 => self.ewram[(addr & 0x3_FFFF) as usize] = value,
            0x03 => self.iwram[(addr & 0x7FFF) as usize] = value,
            0x04 => {
                let off = addr & 0x00FF_FFFF;
                if off < 0x400 {
                    self.io_write8(off, value);
                }
            }
            0x05 => {
                // Byte writes duplicate into the aligned halfword.
                let base = (addr & 0x3FE) as usize;
                self.palette[base] = value;
                self.palette[base + 1] = value;
            }
            0x06 => {
                // Byte writes: duplicated in BG VRAM, ignored in OBJ VRAM.
                let idx = Self::vram_index(addr & !1);
                if idx < 0x1_0000 {
                    self.vram[idx] = value;
                    self.vram[idx + 1] = value;
                }
            }
            0x07 => {} // byte writes to OAM are ignored
            0x08..=0x0D => {
                // GPIO port (clock chip) is mapped into the ROM region;
                // wider writes decompose to these byte writes.
                if self.is_gpio(addr) {
                    self.rtc.as_mut().unwrap().write(addr & 0x01FF_FFFF, value);
                } else if addr >> 24 == 0x0D && self.is_eeprom_addr(addr) && addr & 1 == 0 {
                    self.eeprom.as_mut().unwrap().write_bit(value as u16);
                }
            }
            0x0E | 0x0F => match &mut self.flash {
                Some(flash) => flash.write(addr, value),
                None => self.sram[(addr & 0xFFFF) as usize] = value,
            },
            _ => {}
        }
    }

    fn read16(&mut self, addr: u32) -> u16 {
        match addr >> 24 {
            0x00 => {
                if addr < 0x4000 {
                    if self.pc_in_bios {
                        let off = (addr & !1) as usize;
                        u16::from_le_bytes([self.bios[off], self.bios[off + 1]])
                    } else {
                        // BIOS read protection (see read8).
                        (self.bios_open >> ((addr & 2) * 8)) as u16
                    }
                } else {
                    self.rd16_bytes(addr)
                }
            }
            0x02 => {
                let off = (addr & 0x3_FFFF) as usize;
                if off + 2 <= 0x4_0000 {
                    u16::from_le_bytes([self.ewram[off], self.ewram[off + 1]])
                } else {
                    self.rd16_bytes(addr)
                }
            }
            0x03 => {
                let off = (addr & 0x7FFF) as usize;
                if off + 2 <= 0x8000 {
                    u16::from_le_bytes([self.iwram[off], self.iwram[off + 1]])
                } else {
                    self.rd16_bytes(addr)
                }
            }
            0x05 => {
                let off = (addr & 0x3FF) as usize;
                if off + 2 <= 0x400 {
                    u16::from_le_bytes([self.palette[off], self.palette[off + 1]])
                } else {
                    self.rd16_bytes(addr)
                }
            }
            0x06 => {
                let i0 = Self::vram_index(addr);
                let i1 = Self::vram_index(addr.wrapping_add(1));
                if i1 == i0 + 1 {
                    u16::from_le_bytes([self.vram[i0], self.vram[i1]])
                } else {
                    self.rd16_bytes(addr)
                }
            }
            0x07 => {
                let off = (addr & 0x3FF) as usize;
                if off + 2 <= 0x400 {
                    u16::from_le_bytes([self.oam[off], self.oam[off + 1]])
                } else {
                    self.rd16_bytes(addr)
                }
            }
            0x08..=0x0D => {
                if self.is_gpio(addr) || self.is_eeprom_addr(addr) {
                    return self.rd16_bytes(addr);
                }
                let idx = Self::rom_index(addr);
                if idx + 2 <= self.rom.len() {
                    if Self::resource_page(idx) {
                        if let Some(bytes) = self.rom_resources.as_ref().and_then(|r| r.guest_read::<2>(idx, &self.rom)) {
                            return u16::from_le_bytes(bytes);
                        }
                    }
                    u16::from_le_bytes([self.rom[idx], self.rom[idx + 1]])
                } else {
                    self.rd16_bytes(addr)
                }
            }
            _ => self.rd16_bytes(addr),
        }
    }

    fn read32(&mut self, addr: u32) -> u32 {
        match addr >> 24 {
            0x00 => {
                if addr < 0x4000 {
                    if self.pc_in_bios {
                        let off = (addr & !3) as usize;
                        u32::from_le_bytes(self.bios[off..off + 4].try_into().unwrap())
                    } else {
                        // BIOS read protection (see read8).
                        self.bios_open
                    }
                } else {
                    self.rd32_halves(addr)
                }
            }
            0x02 => {
                let off = (addr & 0x3_FFFF) as usize;
                if off + 4 <= 0x4_0000 {
                    u32::from_le_bytes(self.ewram[off..off + 4].try_into().unwrap())
                } else {
                    self.rd32_halves(addr)
                }
            }
            0x03 => {
                let off = (addr & 0x7FFF) as usize;
                if off + 4 <= 0x8000 {
                    u32::from_le_bytes(self.iwram[off..off + 4].try_into().unwrap())
                } else {
                    self.rd32_halves(addr)
                }
            }
            0x05 => {
                let off = (addr & 0x3FF) as usize;
                if off + 4 <= 0x400 {
                    u32::from_le_bytes(self.palette[off..off + 4].try_into().unwrap())
                } else {
                    self.rd32_halves(addr)
                }
            }
            0x06 => {
                let i0 = Self::vram_index(addr);
                let i3 = Self::vram_index(addr.wrapping_add(3));
                if i3 == i0 + 3 {
                    u32::from_le_bytes(self.vram[i0..i0 + 4].try_into().unwrap())
                } else {
                    self.rd32_halves(addr)
                }
            }
            0x07 => {
                let off = (addr & 0x3FF) as usize;
                if off + 4 <= 0x400 {
                    u32::from_le_bytes(self.oam[off..off + 4].try_into().unwrap())
                } else {
                    self.rd32_halves(addr)
                }
            }
            0x08..=0x0D => {
                if self.is_gpio(addr) || self.is_eeprom_addr(addr) {
                    return self.rd32_halves(addr);
                }
                let idx = Self::rom_index(addr);
                if idx + 4 <= self.rom.len() {
                    if Self::resource_page(idx) {
                        if let Some(bytes) = self.rom_resources.as_ref().and_then(|r| r.guest_read::<4>(idx, &self.rom)) {
                            return u32::from_le_bytes(bytes);
                        }
                    }
                    u32::from_le_bytes(self.rom[idx..idx + 4].try_into().unwrap())
                } else {
                    self.rd32_halves(addr)
                }
            }
            _ => self.rd32_halves(addr),
        }
    }

    fn write16(&mut self, addr: u32, value: u16) {
        if let Some(log) = self.ffta_diagnostics.as_mut() {
            log.write(self.frames, addr, 2, value as u32);
        }
        // Bypass the byte-write quirks for naturally sized accesses.
        match addr >> 24 {
            0x02 => {
                let off = (addr & 0x3_FFFF) as usize;
                if off + 2 <= 0x4_0000 {
                    self.ewram[off..off + 2].copy_from_slice(&value.to_le_bytes());
                } else {
                    self.write8(addr, value as u8);
                    self.write8(addr.wrapping_add(1), (value >> 8) as u8);
                }
            }
            0x03 => {
                let off = (addr & 0x7FFF) as usize;
                if off + 2 <= 0x8000 {
                    self.iwram[off..off + 2].copy_from_slice(&value.to_le_bytes());
                } else {
                    self.write8(addr, value as u8);
                    self.write8(addr.wrapping_add(1), (value >> 8) as u8);
                }
            }
            0x05 => {
                let base = (addr & 0x3FE) as usize;
                self.palette[base..base + 2].copy_from_slice(&value.to_le_bytes());
            }
            0x06 => {
                let idx = Self::vram_index(addr & !1);
                self.vram[idx..idx + 2].copy_from_slice(&value.to_le_bytes());
            }
            0x07 => {
                let base = (addr & 0x3FE) as usize;
                self.oam[base..base + 2].copy_from_slice(&value.to_le_bytes());
            }
            _ => {
                self.write8(addr, value as u8);
                self.write8(addr.wrapping_add(1), (value >> 8) as u8);
            }
        }
    }

    fn write32(&mut self, addr: u32, value: u32) {
        if let Some(log) = self.ffta_diagnostics.as_mut() {
            log.write(self.frames, addr, 4, value);
        }
        let a = addr & !1;
        match a >> 24 {
            0x02 => {
                let off = (a & 0x3_FFFF) as usize;
                if off + 4 <= 0x4_0000 {
                    self.ewram[off..off + 4].copy_from_slice(&value.to_le_bytes());
                    return;
                }
            }
            0x03 => {
                let off = (a & 0x7FFF) as usize;
                if off + 4 <= 0x8000 {
                    self.iwram[off..off + 4].copy_from_slice(&value.to_le_bytes());
                    return;
                }
            }
            0x05 => {
                // Match the two-halfword path: each half re-masks to 0x3FE.
                let off = (a & 0x3FE) as usize;
                if ((a.wrapping_add(2)) & 0x3FE) as usize == off + 2 {
                    self.palette[off..off + 4].copy_from_slice(&value.to_le_bytes());
                    return;
                }
            }
            0x06 => {
                let i0 = Self::vram_index(a);
                if Self::vram_index(a.wrapping_add(2)) == i0 + 2 {
                    self.vram[i0..i0 + 4].copy_from_slice(&value.to_le_bytes());
                    return;
                }
            }
            0x07 => {
                let off = (a & 0x3FE) as usize;
                if ((a.wrapping_add(2)) & 0x3FE) as usize == off + 2 {
                    self.oam[off..off + 4].copy_from_slice(&value.to_le_bytes());
                    return;
                }
            }
            _ => {}
        }
        self.write16(a, value as u16);
        self.write16(a.wrapping_add(2), (value >> 16) as u16);
    }

    fn hle_halt(&mut self) {
        self.halted = true;
    }

    fn hle_intr_wait(&mut self, discard: bool, mask: u16) -> bool {
        // First execution: optionally discard stale flags, arm the wait.
        if !self.intr_wait_armed {
            if discard {
                let flags = self.read16(0x0300_7FF8);
                self.write16(0x0300_7FF8, flags & !mask);
            }
            self.intr_wait_armed = true;
        }
        let flags = self.read16(0x0300_7FF8);
        if flags & mask != 0 {
            // Diagnostic (RECOMP_TRACE_IOW): IntrWait satisfaction timing.
            {
                static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                if *TRACE.get_or_init(|| std::env::var_os("RECOMP_TRACE_IOW").is_some()) {
                    eprintln!(
                        "INTRWAIT-OK f={} line={} mask={mask:04x} flags={flags:04x}",
                        self.frames,
                        self.scanline()
                    );
                }
            }
            self.write16(0x0300_7FF8, flags & !mask);
            self.intr_wait_armed = false;
            return true;
        }
        // The BIOS loop forces IME on and halts until any enabled
        // interrupt is latched; the caller rewinds the SWI.
        self.ime = true;
        self.halted = true;
        false
    }

    fn note_unhandled_swi(&mut self, num: u32) -> bool {
        // No BIOS image loaded: record it for triage and treat the call
        // as a no-op rather than vectoring into zeroed memory.
        self.unhandled_swis |= 1u64 << (num & 63);
        true
    }

    fn note_swi_returned(&mut self) {
        self.bios_open = 0xE3A0_2004; // after SWI
    }

    fn note_fetch(&mut self, key: u32) {
        if !self.real_bios {
            return;
        }
        let pc = key & !1;
        let in_bios = pc < 0x4000;
        self.pc_in_bios = in_bios;
        if in_bios {
            // Hardware: the protection value is the last opcode the CPU
            // *fetched* from the BIOS — the pipeline prefetch, two
            // instructions ahead of the one executing (+8 ARM, +4
            // Thumb; the external conformance suite encodes the four
            // canonical seams as 0xDC+8 / 0x188+8 / 0x134+8 / 0x13C+8).
            // Tracking it per executed instruction reproduces them all
            // by construction.
            let ahead = if key & 1 != 0 { 4 } else { 8 };
            let off = ((pc.wrapping_add(ahead)) & !3) as usize;
            if off + 4 <= self.bios.len() {
                self.bios_open = u32::from_le_bytes(self.bios[off..off + 4].try_into().unwrap());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bios_region_reads_return_protection_value() {
        // Reads of 0x0..0x4000 from outside the BIOS return the
        // last-fetched BIOS opcode, byte-laned by address; the value
        // steps at the HLE seams (startup / after-SWI).
        let mut mem = MemMap::new(vec![]);
        assert_eq!(mem.read32(0x0), 0xE129_F000);
        assert_eq!(mem.read8(0xC3), 0xE1);
        assert_eq!(mem.read16(0x2), 0xE129);
        mem.note_swi_returned();
        assert_eq!(mem.read32(0x10), 0xE3A0_2004);
    }

    #[test]
    fn real_bios_reads_track_fetch_location() {
        let mut mem = MemMap::new(vec![]);
        let mut bios = vec![0u8; 0x4000];
        bios[0x100..0x104].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        // The protection value is the pipeline prefetch: executing at
        // 0x200 has fetched 0x208 (ARM, +8).
        bios[0x208..0x20C].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        mem.load_bios(&bios);

        // Fetching inside the BIOS: reads return real bytes.
        mem.note_fetch(0x200);
        assert_eq!(mem.read32(0x100), 0xDEAD_BEEF);
        assert_eq!(mem.read16(0x102), 0xDEAD);
        assert_eq!(mem.read8(0x100), 0xEF);

        // Fetching outside: protection value = last BIOS prefetch.
        mem.note_fetch(0x0800_0000);
        assert_eq!(mem.read32(0x100), 0x1234_5678);
        assert_eq!(mem.read8(0x0), 0x78);
    }

    fn timer_audio_fixture(period: u16) -> MemMap {
        let mut mem = MemMap::new(vec![]);
        mem.tap_channels = true;
        mem.write16(0x0400_0084, 0x80);
        mem.write16(0x0400_0082, 0x0304); // FIFO A, both sides, full volume.
        for sample in 1..=32u8 {
            mem.write8(0x0400_00A0, sample);
        }
        mem.write16(0x0400_0100, 0u16.wrapping_sub(period));
        mem.write16(0x0400_0102, 0xC0);
        mem
    }

    fn assert_same_timer_audio(mut expected: MemMap, mut actual: MemMap) {
        // Timer counters are lazy; synchronize both at the common final clock.
        expected.advance_timers();
        actual.advance_timers();
        assert_eq!(actual.clock, expected.clock);
        assert_eq!(actual.audio_buf, expected.audio_buf, "mixed samples");
        assert_eq!(actual.fifo_tap, expected.fifo_tap, "FIFO event stream");
        assert_eq!(postcard::to_allocvec(&actual).unwrap(),
                   postcard::to_allocvec(&expected).unwrap(), "complete bus state");
    }

    #[test]
    fn timer_audio_is_independent_of_tick_batch_size() {
        for period in [64, 127, 128, 255, 256, 257] {
            for batch in [2, 7, 32, 600] {
                let mut expected = timer_audio_fixture(period);
                let mut actual = timer_audio_fixture(period);
                for _ in 0..4096 { expected.tick(1); }
                let mut left = 4096;
                while left > 0 {
                    let n = left.min(batch);
                    actual.tick(n);
                    left -= n;
                }
                assert_same_timer_audio(expected, actual);
            }
        }
    }

    #[test]
    fn timer_audio_batches_preserve_two_timers_cascade_and_hblank_dma() {
        for control in [0xC0, 0xC1, 0xC2, 0xC3, 0xC4] {
            let fixture = || {
                let mut mem = timer_audio_fixture(127);
                mem.write16(0x0400_0082, 0x730C); // FIFO B on timer 1.
                for sample in 1..=32u8 { mem.write8(0x0400_00A4, sample * 3); }
                mem.write16(0x0400_0104, 0xFFFB);
                mem.write16(0x0400_0106, control);
                for i in 0..1024u32 { mem.write32(0x0200_0000 + i * 4, i); }
                mem.write32(0x0400_00D4, 0x0200_0000);
                mem.write32(0x0400_00D8, 0x0300_0000);
                mem.write16(0x0400_00DC, 8);
                mem.write16(0x0400_00DE, 0xA600); // Repeat HBlank, words.
                mem
            };
            let mut expected = fixture();
            let mut actual = fixture();
            for _ in 0..8192 { expected.tick(1); }
            actual.tick(8192);
            assert!(expected.clock > 8192, "HBlank DMA must steal cycles");
            assert!(!expected.fifo_tap[1].is_empty(), "timer 1 must drive FIFO B");
            assert_same_timer_audio(expected, actual);
        }
    }

    #[test]
    fn timer_audio_skip_matches_clocked_execution() {
        let mut expected = timer_audio_fixture(128);
        let mut actual = timer_audio_fixture(128);
        for _ in 0..512 { expected.tick(1); }
        while actual.clock < 512 { actual.skip_to_next_event(); }
        assert_same_timer_audio(expected, actual);
    }

    #[test]
    fn graphics_dma_preserves_audio_sample_timer_ties_and_cycle_cost() {
        let mut mem = timer_audio_fixture(128);
        for i in 0..300u32 { mem.write32(0x0200_0000 + i * 4, i); }
        mem.write32(0x0400_00D4, 0x0200_0000);
        mem.write32(0x0400_00D8, 0x0300_0000);
        mem.write16(0x0400_00DC, 300);
        mem.write16(0x0400_00DE, 0x8400);
        assert_eq!(mem.clock, 604); // 2 cycles/word + 4 setup.
        assert_eq!(mem.read32(0x0300_0000 + 299 * 4), 299);
        // Samples at 0,256,512 precede a timer overflow at the same instant.
        assert_eq!(mem.audio_buf, [0, 0, 64, 64, 192, 192]);
    }

    #[test]
    fn sound_dma_preserves_audio_across_tick_batches_and_stolen_cycles() {
        let fixture = || {
            let mut mem = timer_audio_fixture(128);
            for i in 0..1024u32 { mem.write8(0x0200_0000 + i, i as u8); }
            mem.write32(0x0400_00BC, 0x0200_0000);
            mem.write32(0x0400_00C0, 0x0400_00A0);
            mem.write16(0x0400_00C4, 4);
            mem.write16(0x0400_00C6, 0xB640); // Repeat, special timing, words, fixed dst.
            mem
        };
        let mut expected = fixture();
        let mut actual = fixture();
        for _ in 0..6000 { expected.tick(1); }
        actual.tick(6000);
        assert!(expected.fifo_refills[0] > 0);
        assert_eq!(expected.clock, 6000 + expected.fifo_refills[0] * 12);
        assert_same_timer_audio(expected, actual);
    }

    #[test]
    fn fifo_tap_records_samples_with_timer_period() {
        let mut mem = MemMap::new(vec![]);
        mem.tap_channels = true;
        // FIFO A on timer 0 (SOUNDCNT_H default), timer 0 period 256
        // cycles (reload 0xFF00, prescaler F/1).
        mem.write32(0x0400_00A0, 0x4030_2010); // four FIFO A samples
        mem.write16(0x0400_0100, 0xFF00); // reload
        mem.write16(0x0400_0102, 0x0080); // enable
        mem.tick(256 * 6 + 8);
        let tap = &mem.fifo_tap[0];
        assert_eq!(tap.len(), 6);
        assert!(tap.iter().all(|&(_, p)| p == 256));
        let samples: Vec<i8> = tap.iter().map(|&(s, _)| s).collect();
        // Four queued samples, then the DAC holds the last level.
        assert_eq!(samples, vec![0x10, 0x20, 0x30, 0x40, 0x40, 0x40]);
        // PSG-only tap fills on the sample grid alongside.
        assert_eq!(mem.psg_tap.len(), mem.audio_buf.len());
    }

    #[test]
    fn mirrors() {
        let mut mem = MemMap::new(vec![0xAB; 0x100]);
        mem.write32(0x0200_0000, 0xDEAD_BEEF);
        assert_eq!(mem.read32(0x0204_0000), 0xDEAD_BEEF); // EWRAM mirror
        assert_eq!(mem.read32(0x02FC_0000), 0xDEAD_BEEF);
        mem.write32(0x0300_0000, 0x1234_5678);
        assert_eq!(mem.read32(0x0300_8000), 0x1234_5678); // IWRAM mirror
        assert_eq!(mem.read32(0x03FF_8000), 0x1234_5678);
        // ROM mirrors across waitstate regions.
        assert_eq!(mem.read8(0x0800_0000), 0xAB);
        assert_eq!(mem.read8(0x0A00_0000), 0xAB);
        assert_eq!(mem.read8(0x0C00_0000), 0xAB);
    }

    #[test]
    fn vram_mirror_layout() {
        let mut mem = MemMap::new(vec![]);
        mem.write16(0x0601_0000, 0x1111);
        // 32K block is doubled: 0x18000 maps back to 0x10000.
        assert_eq!(mem.read16(0x0601_8000), 0x1111);
        // And the whole 128K window mirrors.
        assert_eq!(mem.read16(0x0603_0000), 0x1111);
    }

    #[test]
    fn byte_write_quirks() {
        let mut mem = MemMap::new(vec![]);
        mem.write8(0x0500_0001, 0x42); // palette: duplicated into halfword
        assert_eq!(mem.read16(0x0500_0000), 0x4242);
        mem.write8(0x0700_0000, 0x42); // OAM: ignored
        assert_eq!(mem.read16(0x0700_0000), 0);
        mem.write8(0x0600_0000, 0x37); // BG VRAM: duplicated
        assert_eq!(mem.read16(0x0600_0000), 0x3737);
        mem.write8(0x0601_0001, 0x99); // OBJ VRAM: ignored
        assert_eq!(mem.read16(0x0601_0000), 0);
    }

    #[test]
    fn rom_out_of_bounds_pattern() {
        let mut mem = MemMap::new(vec![0; 4]);
        assert_eq!(mem.read16(0x0800_0010), 0x0008);
        assert_eq!(mem.read16(0x0800_0012), 0x0009);
    }

    #[test]
    fn keyinput_defaults_released() {
        let mut mem = MemMap::new(vec![]);
        assert_eq!(mem.read16(0x0400_0130), 0x03FF);
    }

    #[test]
    fn if_write_one_to_clear() {
        let mut mem = MemMap::new(vec![]);
        mem.raise_irq(irq::VBLANK | irq::TIMER0);
        mem.write16(REG_IF, irq::VBLANK);
        assert_eq!(mem.reg_if, irq::TIMER0);
    }

    #[test]
    fn timer_overflow_raises_irq() {
        let mut mem = MemMap::new(vec![]);
        // Timer 0: reload 0xFFF0, prescaler 1, IRQ enabled, start.
        mem.write16(0x0400_0100, 0xFFF0);
        mem.write8(0x0400_0102, 0xC0);
        mem.tick(15);
        assert_eq!(mem.reg_if & irq::TIMER0, 0);
        mem.tick(2);
        assert_ne!(mem.reg_if & irq::TIMER0, 0);
        // Counter wrapped to reload and keeps counting.
        assert!(mem.read16(0x0400_0100) >= 0xFFF0);
    }

    #[test]
    fn timer_cascade() {
        let mut mem = MemMap::new(vec![]);
        // Timer 0 overflows every 16 cycles; timer 1 cascades with IRQ.
        mem.write16(0x0400_0100, 0xFFF0);
        mem.write16(0x0400_0104, 0xFFFF); // timer 1: overflow on first tick
        mem.write8(0x0400_0106, 0xC4); // enable + cascade + irq
        mem.write8(0x0400_0102, 0x80); // enable timer 0
        mem.tick(20);
        assert_ne!(mem.reg_if & (irq::TIMER0 << 1), 0);
    }

    #[test]
    fn immediate_dma_copies() {
        let mut mem = MemMap::new(vec![]);
        mem.write32(0x0200_0000, 0x1122_3344);
        mem.write32(0x0200_0004, 0x5566_7788);
        // DMA3: src EWRAM, dst IWRAM, 2 words, immediate.
        mem.write32(0x0400_00D4, 0x0200_0000);
        mem.write32(0x0400_00D8, 0x0300_0000);
        mem.write16(0x0400_00DC, 2);
        mem.write16(0x0400_00DE, 0x8400); // enable | 32-bit
        assert_eq!(mem.read32(0x0300_0000), 0x1122_3344);
        assert_eq!(mem.read32(0x0300_0004), 0x5566_7788);
        // Channel disabled itself.
        assert_eq!(mem.read8(0x0400_00DF) & 0x80, 0);
    }

    #[test]
    fn vblank_event_sets_flag_and_irq() {
        let mut mem = MemMap::new(vec![]);
        mem.write8(0x0400_0004, 0x08); // DISPSTAT: vblank IRQ enable
        mem.tick(CYCLES_PER_SCANLINE * VISIBLE_SCANLINES + 1);
        assert!(mem.frame_ready);
        assert_ne!(mem.reg_if & irq::VBLANK, 0);
        assert_eq!(mem.read8(0x0400_0004) & 1, 1); // vblank flag visible
    }

    #[test]
    fn wave_ram_bank_flip_swaps_window() {
        let mut mem = MemMap::new(vec![]);
        mem.write8(0x0400_0070, 0x80);
        for i in 0..16u32 {
            mem.write8(0x0400_0090 + i, 0xA0 | i as u8);
        }
        // Flip the playing bank: the window now shows the previously
        // hidden (zeroed) bank; the A pattern moved behind it.
        mem.write8(0x0400_0070, 0xC0);
        for i in 0..16u32 {
            assert_eq!(mem.read8(0x0400_0090 + i), 0);
            mem.write8(0x0400_0090 + i, 0xB0 | i as u8);
        }
        // Flip back: the A pattern reappears intact.
        mem.write8(0x0400_0070, 0x80);
        for i in 0..16u32 {
            assert_eq!(mem.read8(0x0400_0090 + i), 0xA0 | i as u8);
        }
    }

    #[test]
    fn wave_plays_hidden_bank_after_flip() {
        let mut mem = MemMap::new(vec![]);
        mem.write8(0x0400_0084, 0x80); // master enable
        mem.write8(0x0400_0080, 0x77); // PSG volume 7 both sides
        mem.write8(0x0400_0081, 0x44); // channel 3 to both sides
        mem.write8(0x0400_0082, 0x02); // PSG ratio 100%
        mem.write8(0x0400_0070, 0x80);
        for i in 0..16u32 {
            mem.write8(0x0400_0090 + i, 0xFF); // fill the idle bank
        }
        mem.write8(0x0400_0070, 0xC0); // flip: that bank now plays
        mem.write8(0x0400_0073, 0x20); // wave volume 100%
        mem.write8(0x0400_0074, 0xFF);
        mem.write8(0x0400_0075, 0x87); // trigger, freq 0x7FF
        mem.tick(256);
        // Constant nibble 15 → (15-8)*256 per side at full routing. If
        // the APU sampled the (zeroed) window instead, this would be
        // (0-8)*256.
        assert_eq!(*mem.audio_buf.last().unwrap(), 1792);
    }

    #[test]
    fn soundcnt_x_reports_active_channels() {
        let mut mem = MemMap::new(vec![]);
        mem.write8(0x0400_0084, 0x80);
        assert_eq!(mem.read8(0x0400_0084), 0x80);
        // Channel 1: length 1/256 s, full envelope, trigger + length.
        mem.write8(0x0400_0062, 0x3F);
        mem.write8(0x0400_0063, 0xF0);
        mem.write8(0x0400_0065, 0xC0);
        assert_eq!(mem.read8(0x0400_0084), 0x81);
        // Master off forces the flags clear.
        mem.write8(0x0400_0084, 0x00);
        assert_eq!(mem.read8(0x0400_0084), 0x00);
        mem.write8(0x0400_0084, 0x80);
        assert_eq!(mem.read8(0x0400_0084), 0x81);
        // One 64 Hz frame expires the length.
        mem.tick(262144 + 512);
        assert_eq!(mem.read8(0x0400_0084), 0x80);
    }

    #[test]
    fn fifo_depth_capped_at_32() {
        let mut mem = MemMap::new(vec![]);
        for i in 0..40u8 {
            mem.write8(0x0400_00A0, i);
        }
        assert_eq!(mem.fifo[0].len(), 32);
        // The first 32 bytes survive in order; overflow bytes dropped.
        let queued: Vec<i8> = mem.fifo[0].iter().copied().collect();
        assert_eq!(queued, (0..32).collect::<Vec<i8>>());
        assert_eq!(mem.fifo_pushes[0], 40);
    }

    #[test]
    fn keypad_irq_or_mode_edge() {
        let mut mem = MemMap::new(vec![]);
        mem.write16(0x0400_0132, 0x4001); // select A, IRQ enable, OR
        assert_eq!(mem.reg_if & irq::KEYPAD, 0);
        mem.keys &= !1; // press A (active-low)
        mem.tick(CYCLES_PER_SCANLINE + 1);
        assert_ne!(mem.reg_if & irq::KEYPAD, 0);
        // Held condition: no re-raise after acknowledge.
        mem.reg_if = 0;
        mem.tick(CYCLES_PER_SCANLINE);
        assert_eq!(mem.reg_if & irq::KEYPAD, 0);
        // Release and re-press: a fresh edge raises again.
        mem.keys |= 1;
        mem.tick(CYCLES_PER_SCANLINE);
        mem.keys &= !1;
        mem.tick(CYCLES_PER_SCANLINE);
        assert_ne!(mem.reg_if & irq::KEYPAD, 0);
    }

    #[test]
    fn keypad_irq_and_mode_needs_all_selected() {
        let mut mem = MemMap::new(vec![]);
        mem.write16(0x0400_0132, 0xC003); // A+B, IRQ enable, AND
        mem.keys &= !1; // only A
        mem.tick(CYCLES_PER_SCANLINE + 1);
        assert_eq!(mem.reg_if & irq::KEYPAD, 0);
        mem.keys &= !2; // A+B
        mem.tick(CYCLES_PER_SCANLINE);
        assert_ne!(mem.reg_if & irq::KEYPAD, 0);
    }

    #[test]
    fn unmapped_reads_return_prefetch_word() {
        let mut mem = MemMap::new(vec![]);
        // HLE: the seam-stepped protection value is the prefetch model.
        assert_eq!(mem.read32(0x0000_4000), 0xE129_F000);
        assert_eq!(mem.read32(0x0100_0000), 0xE129_F000);
        assert_eq!(mem.read32(0x1000_0000), 0xE129_F000);
        assert_eq!(mem.read16(0x0100_0002), 0xE129);
        assert_eq!(mem.read8(0x0100_0003), 0xE1);
        // Real BIOS: tracks the actual pipeline prefetch (+8 ARM).
        let mut bios = vec![0u8; 0x4000];
        bios[0x208..0x20C].copy_from_slice(&0xCAFE_F00Du32.to_le_bytes());
        mem.load_bios(&bios);
        mem.note_fetch(0x200);
        assert_eq!(mem.read32(0x0000_8000), 0xCAFE_F00D);
        assert_eq!(mem.read8(0x0100_0000), 0x0D);
    }
}
