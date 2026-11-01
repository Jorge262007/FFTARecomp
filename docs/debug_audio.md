<a id="diagnostica-audio-circoscritta--dbg-14"></a>

# Bounded audio diagnostics — DBG-14

Collection is explicit and available in [isolated copies](debug_sessions.md).
It separates guest-generated buffers, the frontend producer's decision and the
callback buffer after DSP, gain and backend-format conversion. The latter is
what the application delivers to the backend callback: device buffering,
hardware XRUNs, speakers and physical listening remain unknown. It starts no
benchmark, native translation or new ROM/DS correctness check.

<a id="dal-menu"></a>

## From the menu

1. Open a debug copy with the updated runtime and pause it.
2. **Debug > Start audio capture** arms 60 frames, with mixed, PSG, HLE and
   both FIFOs. Collection does not rewind the machine or save snapshots.
3. Reproduce the short problem by resuming play or stepping frames.
4. Pause again and choose **Debug > Save audio report**.
5. Open `Data/config/audio/<run>/index.html` in the copy. Each stream offers
   an envelope, metadata, local WAV and raw data; the JSON is `audio.json`.

The report's fixed interface uses English, including stream truncation,
difference counts and unavailable previews. Captured payloads retain their
source content.

Pause and controlled steps normally discard frontend audio. Guest buffers can
therefore be nonzero while the callback is silent. The report shows
`discard-paused`, `discard-no-stream`, `discard-fast-silent`, `normal-queue` or
`rings` for each observed frame; it does not assume a generated buffer was
queued or heard. Queue sizes in callback blocks are observed after consumption,
not copies of queued samples.

<a id="stessa-operazione-da-cli"></a>

## The same operation from the CLI

Open the copy paused, then use another terminal:

```powershell
.\recomp.exe debug-session open 'target\audio-case' --paused
```

```powershell
.\recomp.exe debug-control audio-start 'target\audio-case' --frames 4 --streams mixed,hle
.\recomp.exe debug-control step 'target\audio-case' 1 --keys 3fe
.\recomp.exe debug-control step 'target\audio-case' 3 --keys 3ff
.\recomp.exe debug-control audio-report 'target\audio-case'
```

`--frames` accepts **1–180**; `--streams` accepts a selection without duplicates
from `mixed,psg,hle,fifo0,fifo1`. Menu and CLI share the collector and writer.
Status returns `audio_diagnostic` and the `audio_report` path. To capture
reproducible input, first start a [snapshot recording](debug_recordings.md),
stop it explicitly and close the copy before verification. Observed active-low
keys are also reported per frame. Audio collection alone does not certify replay.

Start and Save require a paused copy without a step/probe budget or active
replay. Unsaved collection is not replaced, and a separate replay cannot start
while it exists. Load/Undo/Restart freeze the audio prefix before normal reset;
the report may identify current state after that prefix. A publication error
retains frozen collection for another attempt. Published reports are not modified.

<a id="confrontare-buffer-già-conservati"></a>

## Comparing retained buffers

The offline command reads existing snapshots compatible with the same ROM/BIOS
and guest frame. It executes no frames, loads no dataset and compiles no native
code. Different clocks remain visible.

```powershell
.\recomp.exe debug-audio states --rom '.\ffta.gba' --left '.\left.ffstate' --right '.\right.ffstate' --dir '.\new-audio-comparison' --streams mixed,psg,hle,fifo0,fifo1 --start 81800 --count 256
```

For snapshots with a real BIOS, add `--bios '.\gba_bios.bin'`. The output directory
must be new. Start and count are interleaved-value or FIFO-event indexes; they
must be even and count is **2–262,144**. Comparison retains exact bytes, length
differences and the first 16 differing pairs, with absolute indexes and decoding.
It does not align delay, channels or phases between captures.

JSON outcome and exit code distinguish **0: equal**, **2: diverged**,
**3: incompatible**. Equal covers only selected streams/window, including empty
buffers; it certifies neither complete state nor perceived quality. Divergence
still publishes a report. Incompatible inputs produce a JSON result without
overwriting output.

The browser lets users select a stream and jump to the first difference in
sample previews. For FIFOs, it shows exact events in details; it does not convert
an event index into an inferred timestamp.

<a id="formati-limiti-e-identità"></a>

## Formats, limits and identity

- Mixed and PSG: little-endian i16 values, stereo at 65,536 Hz. HLE: little-endian
  IEEE f32 bits in the same order. FIFO: i8 sample followed by little-endian u32
  period, five bytes per event. `.bin` files retain exact values.
- WAVs are PCM16 previews with clipping and nonfinite values replaced by zero.
  FIFO preview uses isolated sample-and-hold at 65,536 Hz with `period/256`;
  filter, routing and initial phase remain unknown. Each preview lasts at most
  three seconds; that limit does not change the selected raw dump.
- Live capture: 30 host seconds, at most 180 guest frames and **2 MiB per stream**.
  Callback: three seconds or 524,288 normalized f32 values, at most 512 blocks
  and 64 resets. Omissions, truncation and stop reasons are explicit. The guest
  limit applies to frames after origin; buffers pending at startup are declared
  and may appear in the first handoff.
- The report retains exact frame/clock/state, ROM/runtime, actual BIOS/dataset,
  session manifest with seed/immutable-input hashes, build, input/AV configuration
  and observed keys. An in-progress recording path does not mean a complete/
  verified recording. Offline comparison identifies runtime, ROM, BIOS and
  snapshots; historical frontend settings and input absent from snapshots remain
  unknown.
- At most **eight reports** per copy, including incomplete attempts. JSON at most
  2 MiB; no automatic deletion or overwrite. Directories and all dumps remain
  private and Git-ignored. The [local homepage](report_home.md) finds them after
  regeneration.

The callback copies only values/counters into preallocated memory under its
existing mutex. It reads no clocks, allocates no buffers, writes no files/logs
and acquires no other locks. WAV/JSON/HTML are written by the main thread after
releasing the mutex. Published frame is a producer watermark, not a proven
origin for each callback sample. User/temporary mute, gain, enhanced/HLE mode,
requested/estimated Fast, residual queues, counters and resets explain the
buffer's context.

<a id="validazione"></a>

## Validation

Windows checks run under Wine/Xvfb in muted portable copies. The retained
divergence at frame 8514 and clock 2391464782 is localized to mixed 81918/81919:
−704 versus −576, also when selecting only 256 values around that point. This
reuses earlier snapshots without repeating replay or benchmark.

Live validation records A/release for four frames, compares complete endpoint
state and captures CLI selection, menu defaults and Fast/reset. Source peak is
0.16015625 while the muted callback is zero; this distinguishes generation from
application buffer without certifying hardware listening. Duplicate Start,
invalid parameters, eight-directory cap, active/unpaused/closed session refusals
preserve reports. Tests compare output/queues/counters bit for bit with/without
observer in f32/i16/u16 formats and existing audio transitions.

Detailed results and actual limits are in the
[DBG-14 record](changes/2026-10-29-add-bounded-audio-diagnostics.json).
Native Windows/Linux, physical controllers, other devices/scenes and physical
listening remain unverified by these checks. The presence of a report alone
establishes no ROM/DS/native acceptance or performance result.
