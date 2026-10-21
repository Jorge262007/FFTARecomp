# Desktop performance measurement workflow

Start from the [performance TODO](PERFORMANCE_TODO.md). Keep the baseline binary,
its TinyCC loader and a single ROM translation. Use a release build and separate
verification from work timing. ROMs, save copies, recordings and result logs stay
under ignored local directories. The benchmark never writes the supplied save.

## Host load before timing

Check background CPU load and the pinned CPU's SMT sibling before accepting a
comparison. Affinity does not reserve a core; unrelated work can change cache,
boost and available execution resources without changing replay results. Retain
contaminated logs as correctness evidence and rerun timing after resolving the
interference. A passed deterministic comparison alone is not a performance gain.

In this Rust workspace, VS Code's automatic npm task discovery was observed
running `rg --files --hidden --no-ignore --follow` for package manifests for over
half an hour, using roughly ten CPU cores. Ignored build trees and Wine prefixes
can still be traversed by that search. No npm manifests were found in the source
tree. A local workspace `"npm.autoDetect": "off"` setting stops unnecessary npm task
discovery; it does not disable manual npm commands or change other projects.
Preserve existing editor settings. `.vscode/` stays ignored, and no global editor
configuration is required. Identify the specific stalled automatic scan before
cancelling it; leave unrelated interactive work alone. Check that it does not
restart before remeasuring. The observed background scan invalidates using the
earlier loaded-host battle timings as an uncontended code-performance baseline.

## Personal local fixtures

The optional root `saves.zip` and extracted `saves/` directory are ignored,
as are individual `.sav` and `.ffstate` files. Inspect archive paths before
copying selected entries into an isolated portable bundle under `target/`;
do not launch against the originals, since normal play persists cartridge
memory on exit. Keep the matching ROM and all fixture bytes local. The Docker
context uses an allowlist and excludes saves; portable archives also exclude
personal data.

The supplied slot has passed actual desktop loading and visual scene inspection;
the cartridge save has also reached the world map through Saved Game > Load.
After loading a slot, leave the Save states page, select Resume explicitly,
and verify that the game advances without the host pause menu. A successful
load message alone is insufficient for a gameplay measurement. Failed keyboard
automation on desktop Xwayland was discarded; isolated Xvfb input succeeded.
Xvfb/llvmpipe uses CPU rendering and shares host resources with emulation:
label those captures separately from Radeon/Wayland results. Current interactive
profiles include startup and navigation, so a scene screenshot does not make
the whole-session distribution a clean scene-only benchmark.

### Record a battle from a supplied snapshot

Input recording refuses host snapshot loads because its event timeline would
otherwise rewind. When the original cartridge save has no resumable battle,
load the supplied snapshot in an isolated, non-recording session and use the
game's own battle Save command to create a suspend save. Close cleanly and keep
a separate copy of that derived `.sav`. Start a new session with
`RECOMP_RECORD_INPUT` and resume the battle through Saved Game > Resume Battle.
This uses the existing recorder without weakening its load protection.

While recording, save a host snapshot before the target actions. The headless
runner's `--state ... --frames 0` reports its guest-frame origin. For replay
from that snapshot, retain subsequent recorded events and subtract that origin
from their frame numbers; carry the held key mask at the origin into frame zero
if needed. Keep the original recording and its `.input.sav` sidecar intact.
Save another snapshot after the observed enemy action, inspect the scene, and
check the replay's end frame and framebuffer hash against that endpoint before
timing. Do not classify the whole interval as AI computation: it also contains
player inputs, animations and idle frames. A matched image alone does not prove
complete machine or audio equality; retain the separate A/B state checks.

## Headless comparison

The historical script name now supports native Linux/macOS and Windows/Wine;
it detects the executable format and checks it against the host/runner. Existing
Windows commands remain supported. For example, on Linux from the repository root:

```bash
python3 scripts/benchmark_windows_runtime.py \
  --baseline target/perf-baseline/recomp \
  --candidate target/release/recomp \
  --rom /path/to/ffta.gba --library /path/to/translation.so \
  --input /path/to/battle.input --sav /path/to/save-copy.sav \
  --frames 6000 --warmup 120 --runs 5 \
  --scenario enemy-turn --out target/perf/enemy-turn
```

`--demo` instead of `--input` generates the existing introductory Start/A sequence;
it does not identify an AI workload. `--sav` preloads cartridge progress, so its
recording still navigates from boot. Alternatively, `--state /path/to/slot-1.ffstate`
resumes a desktop slot with embedded preview or a legacy raw snapshot through
the menu's transactional version/identity/checksum validator. Both runtimes must
support this flag. `--sav` and `--state` are mutually exclusive; a snapshot needs
an explicit `--input`, rather than the introductory `--demo`.

For snapshot runs, `--frames N` advances N additional guest frames, warm-up and
input events start at zero at the loaded point, and zoom schedules also use that
relative timeline. The guest's frame counter, clock, RNG and hardware deadlines
retain their original values. Without a snapshot the origin is boot frame zero,
which preserves existing scripts. `RECOMP_COST_FROM`, character diagnostics and
other legacy guest-frame diagnostics retain absolute frame numbers.
An idle input file is simply `gba-input v1` followed by `0 3ff` on the next line;
it exercises the loaded scene but does not prove enemy AI coverage.
Add `--zoom '0:0,1000:4'` to compare a fixed zoom schedule.

The runner never writes the supplied snapshot, and rejects `--out` pointing to
that same existing path or symlink. The comparison script protects snapshot
inputs from output collisions, records their file hashes, checks the loaded
machine identity and guest-frame origin on every run, and verifies exact frame
ranges relative to that origin. `summary.json` includes `start_frame` and
`initial_state` in addition to the requested replay length. Snapshot loading and
its identity hash precede timing; ordinary frame work remains unchanged.

For Windows under Wine add `--wine wine --wine-prefix target/perf/wine` and use
Windows executables with their TinyCC DLLs and a Windows-compatible translation.
Native Windows needs no runner. Results distinguish native Linux/Windows/macOS
from Windows via Wine. Use the same toolchain to build both binaries when isolating
small code changes; a previously packaged binary can also serve as a regression
baseline, but build-environment differences remain a confounding factor.

Each comparison:

1. Captures framebuffer/OAM hashes for every frame in two verification runs.
2. Checks the warm-up scene fingerprint, interval, dimensions and final image.
3. Alternates baseline/candidate timing order for five pairs by default.
4. Checks final results again on every timing run, without per-frame hashing.
5. Publishes `summary.json` with individual samples, median, platform, workload
   label and SHA-256 of the binaries, translation, ROM, input and supplied save.

`--verify-state` additionally requires both runtimes to support
`FFTA_BENCH_STATE=1`. It hashes the complete serialized machine snapshot after
the measured interval, including CPU, hardware and cartridge state. It is not a
per-frame proof of all memory transitions or a full-stream audio comparison.
An older executable without this diagnostic is rejected when the flag is set.

`--require-execution-counts` also compares native-block/fallback-step counts.
Leave it off when a deliberate translation change alters dispatch granularity;
counts are not a measure of CPU time or guest-state equality.

The script rejects incomplete/duplicate captures, mismatched warm-up scenes,
nonfinite timings, changed inputs and input/output path collisions. Processes
have a configurable timeout (`--timeout`, default 300 seconds per run).
Once execution starts, `summary.json` explicitly records running/failed status,
so an execution failure cannot leave an old successful result in place. Timing and verification logs remain
available for inspection; do not interpret a run as passed without checking status.

## Guest-cycle cost census

`RECOMP_COST_FROM=N` enables the existing per-PC guest-cycle histogram from
frame N in `runc`. It now observes the same dispatch loop as normal execution,
including sprite/battle mouse interpreter gates, BIOS/interrupt handling, audio
HLE hooks, strict no-interpreter mode and fallback-entry tracing. The default
runner uses a no-op observer specialization; it does not build a cost histogram.

`COST_DISPATCH` reports native blocks (`native`), missing translations (`missing`),
interrupt/sleep/IRQ-return steps (`system`) and intentionally interpreted sprite
or mouse steps (`hook`). The last two categories are separate from the existing
native/missing totals. `COST` retains its legacy PC aggregation, without adding
the Thumb mode bit, and reports guest cycles rather than host CPU time.
A hot guest PC is a triage lead, not proof of a host bottleneck. The census adds
per-dispatch accounting overhead; keep it out of timing comparisons. The A/B
script explicitly clears `RECOMP_COST_FROM` from child environments.

For a local ROM/native-translation fixture, the optional test
`native_cost_observer_matches_game_frames_audio_and_full_state` runs the same
3000-frame demo with and without dispatch observation and scoped PPU timing.
It compares a full-stream digest
of native/wide framebuffer, OAM, mixed/PSG/FIFO/HLE audio, final serialized machine
state and dispatch totals; it also checks that attributed cycles sum to the
actual guest-clock advance. Set `FFTA_STATE_TEST_ROM`, `FFTA_STATE_TEST_NATIVE`
and, when required, `FFTA_TCC_LIBRARY`, then run:

```sh
cargo test --locked --release -p recomp native_cost_observer_matches_game_frames_audio_and_full_state -- --ignored --nocapture --test-threads=1
```

No ROM, translation or diagnostic result is packaged or committed. This fixture
covers the exercised introductory sequence; it does not establish AI coverage
or every hardware/audio-engine/hook path.

## Frame-time diagnostics and their overhead

Add `--profile` when both executables support `FFTA_BENCH_FRAME_TIMES=1`. This
adds two separate instrumented runs after ordinary timing. Reports include p50,
p95, p99, maximum and counts above the GBA frame budget (16.742706 ms) and twice
that budget. The nearest-rank percentile convention is used. Samples measure
CPU wall time inside the frame execution path, including CPU PPU work; they
exclude the window, device callback, frame verification and report generation.

The collector reserves memory before timing, retains at most 360,000 frames and
reports omitted samples. The comparison rejects truncated diagnostic runs.
The report's overhead estimate compares a single instrumented run against the
uninstrumented median; noise can make this estimate negative. It is not a new
optimization result. Per-frame logging and final-state serialization stay outside
ordinary timing; sample capture is disabled unless explicitly enabled.

## CPU PPU timing

Add `--ppu-profile` to request separate instrumented runs with CPU PPU stages;
it implies `--profile` and requires both executables to support this diagnostic.
Ordinary verification/timing runs explicitly disable it, even if the parent
shell has `FFTA_BENCH_PPU=1`. For manual `runc` runs that environment variable
enables PPU capture without requiring the benchmark script.

The report has one sample per completed guest frame after headless warm-up:

- `hardware_scanlines`: wall time inside the 160 hardware scanline calls,
  including UI extraction and forced-blank handling.
- `zoom_rows`: wall time inside extra host zoom-row rendering when available;
  zero when no such work occurred. A parallel path includes the caller's wait
  for its workers, not the sum of worker CPU times.
- `core_other`: captured core interval minus those two PPU totals, including
  CPU execution, other hardware work and instrumentation overhead.

`FFTA_PPU_FRAME` rows provide p50/p95/p99/max, total/mean and budget counts;
`FFTA_PPU_COUNTS` checks frames, scanline/zoom calls and invalid interval
totals. The script rejects missing/duplicate stages, truncated captures,
nonfinite/inconsistent totals, missing scanlines and unexpected instrumentation
in normal timing runs. It stores the stages under each profile's `ppu` field
in `summary.json`. Compare overhead against uninstrumented timing separately;
do not sum percentiles from different stages as if they describe the same frame.

Capture is scoped and thread-local in `gba_core::ppu_profile`, outside machine
state and save serialization. Timers read the host clock only while enabled;
disabled PPU entry points only check whether a capture is active. Completed-frame
capture is bounded like the existing frame profiler. Incomplete guarded frames
are discarded. Nested/error/unwinding scopes restore their prior capture;
worker threads do not inherit it. No GBA cycles/events or GPU timestamps change.

For current-source desktop builds, reuse the Docker scripts with a fresh local
export directory before replacing existing dist archives. Check test gates,
archive CRC/member paths, required executables/loaders, executable modes and
private-data exclusions. Empty Data directories are the expected portable
scaffold; personal files and translation caches are not payload. Verify PE
resources/imports and Linux symbol requirements, and retain BUILD-INFO hashes.
A Windows/Wine replay can use the existing Windows translation with unchanged
ABI to check frames/state/counts against the old export and validated Linux
results. Such a correctness run is not a native Windows performance comparison.
Test the newly exported Linux package itself, including cold-cache startup and
restore, before updating canonical local archives; preserve the old outputs.

`FFTA_PROFILE=1` also includes these rows in desktop exit reports, with the
interactive profiler's first-18000-frame limit and omitted-sample counts.
It still includes startup and has no GUI scene/warm-up selector. Actual GPU
execution timestamps and the originally reported slow enemy turn remain
separate work.

### Optional scanline stage attribution

Set `FFTA_PPU_STAGES=1` in addition to `--ppu-profile` for headless comparisons,
or alongside `FFTA_PROFILE=1` for desktop reports. It is resolved at profiler
creation; it does nothing without the enclosing PPU capture. Ordinary timing
and verification runs retain disabled capture. Coarse-only profiling still
works without this flag and does not read the additional stage clocks.

`FFTA_PPU_STAGE` reports bounded per-frame aggregates for disjoint intervals:

- `backgrounds`: BG rendering in the hardware and relocated wide passes.
- `objects`: OBJ rendering, plus wide-pass OBJ scratch reset.
- `composition`: hardware/wide composition.
- `ui_capture`: native panel/backing extraction, including its internal BG/OBJ
  preparation. Those inner operations are not counted again in other stages.
- `ui_setup`: line-zero dialogue/UI detection and preparation.
- `scanline_other`: scanline total minus these five intervals, including affine
  stepping, blank handling, scratch setup and timer/bookkeeping overhead.

These six totals partition `hardware_scanlines`; do not add them to that parent
or sum their percentiles. Extra zoom-row work remains a separate coarse total.
The comparison script validates complete, finite stage reports and their total,
then stores them as `ppu.scanline_stages`. Old baseline executables can omit
these optional rows. Check their presence in the candidate report before claiming
stage coverage. Compare its instrumented time with its own uninstrumented median
to estimate diagnostic overhead, keeping single-capture noise explicit. This
does not measure per-tile cost or GPU execution and changes no guest timing.

### Native UI background row reuse

UI extraction prepares native BG2/BG3 once within a scanline and shares those
immutable planes across primary/extra and normal/backing passes. Backing-only
text filtering uses a private copy. Stats still uses its original zero BG mask
and transparent input planes; portrait capture flags are resolved per pass.
Anchor accumulation and destination writes remain part of each extraction.
The data is not retained across scanlines or frames: VRAM/palette/register
changes and load/Undo need no new cache invalidation or save-state format.
This reuse belongs to `ui_capture`, not the separate world `backgrounds` stage.

UI rows with transparent native text/backing and no portrait pixels bypass the
world-candidate sort/blend loop. World-only candidates cannot become UI pixels.
The shortcut runs after backing allocation, anchor accumulation and portrait
extraction/flag cleanup, and fills the selected destination row with the exact
`UiPixel::default()` value so old frame/load contents cannot remain visible.
The previous complete loop remains a test-only const-generic reference;
nonempty rows retain it unchanged. This skips host UI work, not guest frames,
hardware scanlines, interrupts or decisions.

## Interactive CPU phase profile

On Linux launch the actual runtime or portable wrapper with:

```bash
FFTA_PROFILE=1 ./FFTARecomp
```

On Windows PowerShell:

```powershell
$env:FFTA_PROFILE = '1'
.\FFTARecomp.exe
Remove-Item Env:FFTA_PROFILE
```

Play through the target scene and close normally. The runtime prints
`FFTA_PLAY_PROFILE` summaries at shutdown. The Linux wrapper stores them in
`play.log`; when launching `recomp play ...` directly, redirect stdout/stderr to
an ignored local log. Windows wrapper success output is captured internally and
is not persisted, so use the runtime directly on Windows to retain these reports.

`FFTA_UI_UPLOAD` also reports layer-update calls and payload bytes scheduled by
those calls (textures plus uniforms), across the gameplay session. It is a
logical upload counter, not a measurement of PCIe traffic or VRAM use. Unchanged
FPS pixels and identical LUTs can now be reused. Game/world uploads retain their
normal update path; payload counters continue after timing samples reach their
retention limit.

The phases are:

- `core_including_ppu`: native/interpreter frame execution with PPU inside it.
- `ui_prepare`: packing the host UI layers.
- `temporal_push` and `temporal_compose`: CPU temporal history and color composition.
- `audio_handoff`: producer-ring transfer and any mutex wait in that function.
- `autosave`: periodic save check/hash/write, sampled only every 300 frames.
- `present_including_acquire_upload_submit`: CPU presentation call, including
  potential swapchain waits. This is **not GPU execution time**.
- `ui_layout_upload`: UI layout preparation and per-layer uploads.
- `gpu_cpu_setup_upload`: surface resize/resource setup, base-frame upload and
  uniform preparation. It excludes the UI uploads above.
- `gpu_cpu_acquire`: acquire the swapchain frame, including any existing
  reconfiguration/retry after a lost or outdated surface.
- `gpu_cpu_encode`, `gpu_cpu_submit`, `gpu_cpu_present`: command encoding,
  queue submission and the host present call measured separately. Queue upload
  transfers execute asynchronously; these are all CPU wall times, not GPU work.
- `pacing_wait`: sleeps explicitly used for the gameplay clock/ring pacing.
- `tick_after_input_including_work_and_wait`: inclusive gameplay tick after input
  handling; may include multiple guest frames or only a ring-full wait.

Counts differ between phases. The presentation subphases overlap the total
presentation time; tick samples overlap the other phases. Do not add inclusive
totals to their subphases. Stages not completed after an error have no sample.
Paused-menu rendering, input polling, GPU execution, the audio consumer callback
and final shutdown save are not measured by these phase timers. The separate
callback profile below covers the audio consumer.
The first 18,000 invocations of each phase are retained, with omission counts;
record short focused sessions or inspect omitted counts before interpreting them.
Interactive capture currently includes startup/resource-creation samples; unlike
headless comparison it does not yet filter a warm-up or label a scene interval.
A single initial pipeline/texture-creation spike does not establish persistent
stutter. Warm-up/scene selection and controlled GUI overhead comparisons remain
part of PERF-02.

No phase timers or sample allocations are enabled by default.

The developer `--stats` title shows native blocks and fallback steps as separate
counts, replacing the misleading mixed-unit percentage. FPS INT/EXT remain the
existing user-facing rate counters. An exhausted emulation guard now fails before
pushing incomplete frames into temporal history or incrementing the frontend's
completed-frame bookkeeping; error exits still release desktop resources.

## Audio callback and ring diagnostics

`FFTA_PROFILE=1` also enables a separate host audio observer for normal play.
No callback profiler is allocated by default or for `play --benchmark`, which
discards audio and cannot measure realtime device behavior. It adds no new
callback mutex, logging, file IO or growing buffers; it observes under the
existing ring mutex. Existing audio error/backlog handling remains unchanged.

At normal shutdown, after dropping the stream, reports include:

- `FFTA_AUDIO_DEVICE`: callback constructions, last configured rate/channels
  and device-error callbacks. A construction can precede a failed stream build;
  zero measured callbacks is not evidence of successful playback.
- `FFTA_AUDIO_COUNTS context=playing|priming|paused|fast_forward`: callback and
  device-frame counts, `faithful_source_holds`, callbacks containing such holds,
  `callback_over_period`, producer dropped items, mixed-ring fill range, retained
  and omitted timing samples. Producer dropped items combine stereo pairs in
  mixed/PSG/HLE rings and individual FIFO events; this is not a sample-pair sum.
- `FFTA_AUDIO_PROFILE` for `lock_wait`, `render`, inclusive `total` and
  `start_interval`: median, p95, p99, maximum and mean wall time in ms.

`lock_wait` starts before acquiring the existing audio mutex. `render` starts
when it is acquired and includes the existing DSP/backlog work. `total` is their
sum; diagnostic bookkeeping and callback return/unlock follow the final clock
read and are excluded. `start_interval` is arrival spacing, not CPU time or a
backend XRUN measurement; the first callback after context changes or stream
reconstruction has no interval. `callback_over_period` compares inclusive work
with that callback's device-frame count divided by its rate, rather than a fixed
GBA frame budget. It does not prove a missed device deadline.

Contexts follow the runtime's existing pause, primed and fast-forward flags,
with pause taking precedence. Cold start and re-prime are separate from playing;
resuming an already-primed session can still have refill holds in playing.
These totals survive legacy alarm-counter resets, load/Undo and device restart,
but do not mix intentionally drained paused callbacks into playing totals.
`faithful_source_holds` counts the existing faithful/backlog underflow path,
which holds its last pair. It does not count enhanced PSG/HLE/FIFO starvation,
prove an audible defect during crossfade, or replace driver-level XRUN telemetry.

Source requests are reported separately as `FFTA_AUDIO_SOURCE` for each context
and `psg|hle|fifo_a|fifo_b`. `requests` counts resampler pair requests for PSG/HLE
and DAC event requests for FIFO; `empty_requests` counts those requests lacking a
complete pair/event. Partial stereo pairs retain the previous DSP behavior.
`configured_target_requests`, `configured_target_empty` and
`configured_target_callbacks` select the callback's configured enhanced target:
PSG when enhanced is selected without normal-speed backlog, HLE when its target
is on and any DS route is enabled, otherwise the routed FIFO channel. Warm,
unrouted and inactive consumers remain in raw totals. All counts follow the
existing frame gate/context; totals continue after timing sample caps.

These are requests, not output samples, durations, audible defects or XRUNs.
Configured target does not encode volume/mute, live crossfade coefficients,
voice activity or filter history; an old path can still contribute during a
crossfade. Empty inactive FIFO/HLE consumers are expected. No source sample is
skipped to collect telemetry. The unobserved const specialization removes
per-request counter work; one callback dispatch selects the stream's variant.
No new callback mutex, logging, IO or allocation is added. Source-loop counter
work is included in render time; final aggregation follows the last clock.

The pinned CPAL0.16.0 ALSA backend recovers some `EPIPE`/XRun paths internally
(`output_stream_worker`, `poll_descriptors_and_prepare_buffer`, `process_output`)
and does not notify the app for every underrun. Device errors or arrival spacing
cannot establish zero backend XRUNs. Use independent backend/server telemetry
when available; this change does not fork CPAL or alter its recovery policy.

On this Linux host, a read-only PipeWire capture now covers the native portable
in the idle Polly battle and pause/resume. Use a **single**
`stdbuf -oL pw-top -b -n 120` process while logging monotonic read times and phase
markers. Default piped output buffers multiple tables: the first trial had
three paused rows arrive together and is excluded from phase timing. For short
phases, validate sample cadence before interpreting any delta; a line-buffered
reader is required here. Do not clear/restart the observer inside the interval.

Use `pw-dump -N` after loading the scene: match the runtime PID to its PipeWire
Client, follow Node `client.id`, then active output-node/input-node links to the
sink. Do not assume PID lives directly on the stream Node. Verify stream and
sink `object.serial` at the beginning and end to exclude ID reuse. Keep raw
metadata and all server/device identifiers in ignored local artifacts. Shared
sink graph errors cannot be uniquely attributed to the game. Per the installed
pw-top manual, follower ERR tracks missed processing completion and driver ERR
tracks graph-cycle errors, which can also include hardware scheduling misses;
it is not a dedicated ALSA hardware-XRUN counter.

The accepted capture uses stable identities, running states, nonzero rate/
quantum and maximum sample gaps below1.001s. Between first/last phase samples:
idle10samples/~9s, paused3/~2s, resumed6/~5s, both stream and sink ERR deltas0.
The stream is44100Hz and physical sink48000Hz. Sink ERR is already1 at the first
idle sample; initial creating-node zero rows are not a valid running baseline
and cannot establish when/cause of that existing error. The test does not prove
zero hardware XRUNs or original AI-stall resolution. App frame window8596:9196
is complete with235playing callbacks, no faithful holds/drops/over-period and
no device error notifications; backend phases are wall intervals, not exactly
that guest-frame window. Scene/CPAL/Vulkan and clean exit are verified separately.
No runtime, system audio configuration or archive rebuild was needed.


Timing storage is bounded independently: 9000 playing callbacks and 3000 each
for priming, pause and fast-forward. Counts continue after each cap; timing
percentiles describe retained samples only. Keep focused recordings and check
omitted counts.

For a focused realtime scene, set `FFTA_AUDIO_PROFILE_FRAMES=START:END` together
with `FFTA_PROFILE=1`. START is inclusive and END exclusive, using the machine's
absolute guest frame counter, not wall seconds or frames since launch. Load a
known state before START and reserve warm-up before that frame. The frontend
publishes completed guest frames under the audio-ring mutex; callbacks are
attributed to the last published frame, not to individual samples' guest origin.
The window does not change playback, input, pacing, DSP, queues or auto-close the
app. CPU/GPU play profiles remain whole-session; this switch gates audio only.
Malformed ranges reject profile-enabled normal play; the switch is ignored when
profiling is off and in the callback-free GUI benchmark.

Capture is one traversal: reaching END closes it permanently. A backwards frame
publication or load/Undo while inside the interval closes it with
`interrupted=true`, even for a restore at the same or a later frame. Loading
before entry is allowed. A closed completed window remains complete after later
load/Undo. Start a new process for another interval. `FFTA_AUDIO_WINDOW` reports
bounds, last published frame, closure/interruption, excluded callbacks and drops;
these excluded events do not consume sample allowances or enter percentiles.
Context and device changes reset arrival history as before. Device-error counts
and callback construction metadata remain whole-session, not gated XRUN data.
Check nonzero playing callbacks, zero omitted samples, `closed=true` and
`interrupted=false` before treating a capture as complete.

A 600-frame idle-battle window8596:9196 after restoring frame8476 was measured
with both faithful and enhanced selected output on native CPAL/Xvfb/llvmpipe.
The captures retain236/235 playing callbacks, no source-faithful holds, producer
drops or callbacks over their own period. Total p99 is0.548/0.665ms; mutex p99
0.000912/0.000711ms. Screenshots confirm the same idle Polly action menu. This
single pair does not justify an SPSC rewrite or establish mode performance,
enhanced-source shortage/XRUN coverage, an AI turn or original-stall resolution.

The actual renderer is exercised without CPAL devices for sample/queue/counter
parity with capture on/off in f32/i16/u16, mono/stereo/six-channel layouts,
44.1/48 kHz, short input, starvation, HLE/enhanced switches and fast-forward.
An optional release test `tests::callback_profile_overhead` measures five
alternating observer on/off pairs on the same synthetic empty-ring callback;
it is a diagnostic overhead check, not a game speedup or hardware latency test.

## Successful host presentation cadence

Normal play with `FFTA_PROFILE=1` also records bounded end-to-end intervals
between successful host presentations, reusing the FPS meter's existing
`Instant`. It does not change pacing, guest timing, DSP, the FPS HUD, or add a
clock read/allocation when diagnostics are off. It is disabled in the unpaced
GUI benchmark, where intervals are not realtime cadence.

`FFTA_PRESENT_PROFILE_FRAMES=START:END` optionally selects one traversal of
completed guest frames, inclusive START/exclusive END. This is independent of
`FFTA_AUDIO_PROFILE_FRAMES`; CPU/GPU phase reports remain whole-session. Load a
known snapshot before START and reserve warm-up. The shared pure range helper
preserves the audio gate's existing behavior. A load/Undo or backwards frame
inside the window interrupts it permanently; a completed window never reopens.

Intervals require normal active play, after audio priming or during silent
wall-clock fallback. Paused/priming/fast-forward/zero-size periods are excluded.
Pause, restore, zoom, resize/DPI, audio recovery and configuration boundaries
start new anchors, preventing false spikes across those boundaries. Surface
skips/software-present failures retain the previous successful anchor, so the
next success includes the missing presentation. A GPU/software change starts a
new anchor; nonzero renderer changes invalidate a single-backend interpretation.

At shutdown, `FFTA_PRESENT_WINDOW` describes the range and closure;
`FFTA_PRESENT_COUNTS` reports successful presentations, anchors, surface skips,
excluded attempts, renderer changes, repeated guest frames, multi-frame guest
advances and their total. `FFTA_PRESENT_INTERVAL` has retained/omitted samples,
median/p95/p99/max/mean milliseconds. Presentations equal anchors plus retained
and omitted intervals. Guest-advance counts include omitted intervals too and
exclude anchors/boundaries. Timing storage is capped at18000 intervals, counts
continue afterwards, and reporting/sorting occurs after rendering ends.

These are host API-success timestamps, not GPU completion or display scanout.
The literal16.742706ms budget counters are not underrun alarms: tiny positive
jitter can exceed that threshold even at a correct average rate, and audio
batching deliberately has longer intervals. Validate nonzero samples, closure,
no interruption/omission and renderer scope before comparing captures.

The native CPAL/Xvfb/llvmpipe fixture window8596:9196 now covers clock/audio with
both requested VSync settings. Clock:600presentations/599intervals, exactly one
guest advance each, mean16.7445/16.7427ms and p99~17.96ms. Audio:237presentations/
236intervals, all with multiple guest advances, mean42.6843/42.6692ms and p99
52.77/52.31ms. No surface skips or renderer changes. This observes the existing
batch-consumption behavior; Linux/Windows already default to clock pacing, so
it is not a new scheduling fix. Virtual VSync requests do not prove physical
60/high-Hz behavior or original enemy-turn lag resolution.

## Optional GPU render-pass timestamps

Run the desktop runtime with both switches (without `FFTA_PROFILE=1`, GPU
profiling stays off):

```bash
FFTA_PROFILE=1 FFTA_GPU_TIMESTAMPS=1 ./FFTARecomp
```

On Windows, set both environment variables to `1` and run `recomp.exe play ...`
directly to preserve stdout. Remove the variables after the diagnostic session.
The presenter requests only `TIMESTAMP_QUERY` if advertised by the adapter;
unsupported features leave ordinary rendering and CPU profiling available. If
optional-feature device creation fails, it retries once without that feature.
The startup log reports timestamp availability.

`FFTA_GPU_PROFILE phase=render_pass` reports median, p95, p99, max, mean and
budget exceedances for the pass drawing the base image and active UI/HUD. It
includes the pass clear and its draw work, excluding uploads, acquisition,
query resolution/readback, queue wait before the pass and compositor/scanout.
Use it alongside CPU/PPU reports; these asynchronous intervals are not additive.
Timestamp conversion uses the queue's timestamp period. Reversed/overflowing
intervals or an invalid period are rejected rather than reported as durations.
Zero-length samples can occur at the device's timestamp resolution.

Three reusable slots each hold one timestamp pair and two 16-byte buffers.
Each presentation polls with `PollType::Poll` and consumes only completed
mappings from earlier submissions. A busy ring skips the sample and continues
rendering. There is no blocking poll, synchronous receive or shutdown wait for
GPU timing. Paused frames are not sampled, but earlier gameplay results may be
drained while paused. No samples are tied to the current CPU frame or scene.
The first 18,000 submitted samples are retained; new submissions stop at that
limit. Mapping/poll failures stop diagnostic capture without disabling rendering.

`FFTA_GPU_COUNTS` reports submitted, completed, mapping failures, invalid
intervals, poll errors, busy/limit skips and the current pending count. Pending
samples at exit are intentionally not awaited. `gpu_cpu_timestamp_readback`
measures CPU polling/decoding overhead within the inclusive presentation time;
encode/submit phases also include diagnostic resolve/copy/map scheduling work.
Measure total enabled overhead with controlled GUI runs before drawing timing
conclusions. Unsupported adapters produce no GPU duration series. With GPU
profiling disabled, no query/readback resources, optional features or polls are
introduced. These diagnostics do not change guest execution or save formats.

The explicit screen test `deferred_readback_bounds_work_and_preserves_rendered_pixels`
checks a real device's asynchronous completion, busy ring, submission cap and
identical RGBA pixels with/without timestamps. It is ignored by ordinary suites
because it requires an available timestamp-capable adapter. The ordinary test
covers optional feature selection and timestamp validation/conversion.

## Bounded GUI workload benchmark

The developer command `recomp play <rom> --benchmark <config.toml>` uses the
existing GUI's guest-frame, temporal/LUT, layer upload and GPU presentation
functions, one completed guest frame and successful presentation per cycle.
It exits automatically at the requested replay endpoint. A skipped/failed GPU
presentation or early window close fails the run rather than silently changing
its scope. Normal interactive play does not enter this path.

Example config, with paths resolved relative to the TOML file:

```toml
state = "start.ffstate"
input = "input.input"
frames = 3530
warmup = 120
scene = "enemy-replay"
zoom = 0
verify = false
```

`frames` is the number of additional frames after the snapshot; input event
frames are relative to it. Use the existing `gba-input v1` recording format,
including semantic mouse events. Require `0 <= warmup < frames <= 360000`;
unknown settings, wrong types and zoom beyond the selected aspect's limit fail.
For profiling-overhead comparisons retain at most 18,000 measured frames, since
CPU series and timestamp submissions have independent capture caps.
`scene` labels the entire run and accepts 1–64 ASCII letters/digits/underscores/
hyphens. The default warm-up is 120, default zoom 0 and default scene `replay`.

The benchmark ignores physical gameplay/menu input, keeps Close available,
forces the FPS HUD off, discards audio output without a callback and omits
clock/ring pacing. Cartridge startup backups, periodic save and exit save are
skipped. State/input/config are read; the normal translation cache may be built
if missing. Use an isolated portable copy and keep private fixtures under
ignored `target/`. This is an **unpaced GUI work benchmark**, not a realtime
audio, compositor/display cadence or interactive-input benchmark.

`FFTA_GUI_BENCH_START` identifies the initial snapshot hash, origin frame,
scene/config and execution mode. `FFTA_GUI_BENCH` reports the exact endpoint,
measured sample count, mean/p50/p95/p99/max cycle work time, dispatch totals and
full final machine hash. Each retained cycle covers replay input, guest/PPU,
UI/temporal preparation, audio-tap discard and presentation; it excludes event
pump time, setup/teardown and final hashing. CPU diagnostics and timestamp
submission start only after the configured warm-up, so asynchronous GPU results
cannot originate from its excluded frames. The first measured timestamp may
still include first-use query-driver cost; inspect maxima as well as medians.

Run separate verification configs with `verify=true`: `FFTA_GUI_VERIFY` emits
native framebuffer/OAM hashes for every additional frame, including warm-up,
outside the cycle timer. Compare these and final state/dispatch counts before
using timing-only runs. Verification I/O disturbs queue timing; do not use those
runs as overhead measurements. Compare identical configs/binary/translation,
settings, adapter and driver with both profiling variables absent, CPU profiling
only, and CPU+GPU profiling. Alternate run order and keep results beyond noise
separate from calibration. These controls do not measure the original slow turn
or default-code overhead against an older binary lacking this benchmark mode.

## Remaining work

Collect an identified slow enemy-turn replay and additional zoom/menu/animation
workloads. Optional detailed PPU attribution and deferred GPU render-pass
queries now exist; expand representative device/backend coverage and measure
GUI overhead before treating them as a complete diagnosis. CPU acquisition,
upload and submit intervals remain distinct from GPU execution.
Reuse the callback observer for targeted realtime scene attribution; extend
enhanced-path/backend underrun coverage, presentation cadence and profile
overhead in controlled native GUI runs. The current profiler supplies CPU wall
times and comparison guards, not a completed diagnosis of the reported AI lag.

## Panel-bound scan diagnostic

A test-only candidate derives each nonempty row's first and last visible pixel
instead of accumulating every visible pixel. It retains the current
`pixel_bounds` inset coordinates, exclusive right/bottom bounds and support for
a partial final row. Production still uses the full scan: the integrated
candidate passed equivalence but was slower in two 3000-frame introductory
comparisons, including one with CPU affinity. No content cache, guest state,
serialization or translation ABI changes are involved.

Run the reference-equivalence test and optional release microbenchmark serially:

```sh
cargo test --locked --release -p gba-core row_endpoint_bounds -- --test-threads=1
cargo test --locked --release -p gba-core benchmark_panel_bounds -- --ignored --nocapture --test-threads=1
```

The ignored diagnostic compares the previous full scan with the test-only scan for
empty, dense, sparse and rectangular-panel 480x160 inputs. Each of five
alternating pairs measures 2000 calls using `black_box`. This measures one
function on synthetic data; it does not establish whole-game or enemy-turn gains.
Run without other CPU workloads and report both improvements and regressions.

On Linux, a follow-up A/B run can inherit a single allowed CPU using
`taskset -c CPU python3 scripts/benchmark_windows_runtime.py ...` (replace
`CPU` with an allowed CPU number). Record that constraint, keep workload/frame
count/settings identical between comparisons, and do not compare a short pinned
replay directly against a longer unrestricted replay. Affinity can reduce
migration noise but does not eliminate frequency, thermal or background-load
variation, and it does not establish performance on other machines.

## Rejected native UI output-comparison cache

The 2026-10-21 producer-side output cache preserved image/state/dispatch parity
and cut logical UI payload from 2011423488 to 530412288 bytes over 3410 updates.
It still performed all packing, then compared a second complete buffer. Five
alternating 3530-frame GUI pairs (warm-up 120, CPU profile on, llvmpipe Vulkan)
showed median run means 8.328947 ms baseline versus 8.339867 ms candidate
(+0.131109%; paired changes -0.008598% to +0.840389%). UI preparation rose from
0.156532 to 0.179629 ms while upload fell from 0.129143 to 0.103490 ms. Combined
UI work barely changed and the whole cycle did not improve usefully.

The candidate was removed and the exact validated baseline restored; published
archives were left intact. Do not retry the same output-comparison/double-buffer
scheme merely because PERF-05 is open. A materially different experiment can
try avoiding packing itself when every source/packing argument is unchanged,
with explicit invalidation and measured equality/overhead. The byte reduction
alone is not evidence of a gameplay, hardware-GPU or power improvement.

## World uploads shared within a presentation

The native UI presenter shares a world texture only when frames in the current
`set_ui_layers` call borrow the same slice (address and extent) and dimensions.
The immutable borrow defines that call's generation; keys are never persisted
as proof of unchanged content. Every unique world is uploaded again next call.
A formerly shared slot receives a separate texture before it can overwrite an
earlier active layer with different data. Independent LUT/UI/layout bindings
remain per layer. Stats' private background and the FPS backdrop keep their own
sources; dimension changes rebind textures without discarding valid UI/LUT data.

The explicit `shared_world_uploads_preserve_pixels_when_sources_split_merge_and_resize`
GPU test checks exact upload bytes and output against independent per-layer
uploads across 33 source graphs/dimension states, including in-place mutation,
absence/reactivation/reordering and recreation. Layer pixels, LUTs and layouts
are distinct. All layers now also require exact CPU sampling equality. The former fractional
CPU/WGSL gap is handled by the integer coordinate maps described below; the
shared-world implementation itself did not cause it.

For the existing enemy-activation replay, all 3530 guest image/OAM hashes, final
state and dispatch counts match, private input/config/save files stay unchanged,
and scheduled payload falls 530412288 -> 316299008 bytes. Five alternating
same-toolchain native GUI A/B pairs (CPU profiling on, no GPU timestamps/audio
callback/pacing or per-frame verification logging) give median run means
8.251393 -> 8.218974 ms (-0.3929%); all paired means improve 0.336-0.773%.
Median p95/p99 are 10.068110/10.571226 -> 10.015975/10.458356 ms on llvmpipe
Vulkan. The separate verification run overlapped compilation and its timings
are excluded. This is not a realtime, native Windows or isolated AI speedup.

## Deterministic native UI sampling

GPU UI texel selection now consumes CPU-defined integer axis maps, preserving
`Frame::sample`'s existing pixel-centre/clipping/floor/clamp formulas. UI and world
indices are explicit little-endian RG32Uint entries. The map's complete key uses
all rect/clip/view float bits, world dimensions and device extent limit; content
and palette changes do not invalidate geometry. Two rows of at most the device
2D texture limit bound storage. Physical origins and sampling extent are device
bounded; empty/padded UI entries discard. Maps survive unchanged frames and
rebuild on resource creation. No epsilon or zoom/FOV policy is introduced.

The original shader could select adjacent texels because [WGSL permits
reassociation/fusion](https://www.w3.org/TR/WGSL/#reassociation-and-fusion).
A new patterned fixture using actual sync_view formulas at 852x480 observed
43098 differences at zoom6; an exact-x2 variant still differed (86474), so
clamping zoom was rejected as a workaround. Both cases now match CPU exactly.
The expanded GPU test reuses resources across 32 combinations of zoom, resize,
853x481 odd dimensions/125% UI scaling and 1704x480 letterboxing, with strict
CPU equality. Native/dyadic/fractional source-sharing cases are strict too.

A table-only prototype passed correctness but five unpaced llvmpipe A/B pairs
showed +4.1681% total work (8.220145 -> 8.562769 ms), so it was not accepted.
The current candidate also caches exact transparent-pixel bounds on UI uploads
and derives an integer scissor from the maps when bounds/geometry change. Only
`eva=0, evb=16` is discarded; palette darkness is not transparency. Active source
claims remain unchanged even with no draw, preventing invisible slots from
breaking world texture sharing. Scissors clamp to the actual target size.
The final bounds candidate completed five fresh pairs: median run means
8.229995 -> 7.774799 ms (-5.5309%), all pairs improve 5.36-6.08%. Median p95/p99
10.003106/10.432033 -> 9.351037/9.847532 ms; logical payload remains 316299008
measured bytes because maps stay cached during that interval. All 3530 guest
frame/OAM hashes, final state/counts and private files match. This is unpaced
llvmpipe work, not native Windows or AI-turn timing. Do not substitute the
rejected table-only measurements for these final-implementation results.

## Skipped presentation upload lifecycle

`PERF-04-SKIPPED-UPLOADS` closes the unsubmitted staging gap: wgpu 26 copies
`Queue::write_buffer`/`write_texture` data into staging immediately and starts
transfers at the next submit. The old skip paths retained these writes across
repeated timeout/zero-size calls. PendingUploads survives multiple/absent UI
updates; skipped calls submit transfers once without draw/present/timestamp
reservation or a blocking wait. Normal submission consumes the same flag.
PlayApp skips all presentation preparation for a zero-sized window, including
software-surface creation. Guest/audio progression is unchanged.

The optional no-ROM lifecycle diagnostic uses the actual Presenter:

```sh
cargo run --locked --release -p recomp --example presenter_skip_uploads
```

It needs a working desktop/GPU or Xvfb. It schedules 1200 UI batches against
zero-sized presentation inputs, including hidden/reactivated layers, changed
world/LUT data, preserved pixel revisions, normal recovery and clean teardown.
Linux prints process RSS/high-water observations; other platforms omit memory
figures. This is not a real compositor minimize or forced-timeout test and does
not claim memory budgets for every driver. The native llvmpipe Vulkan run
scheduled 492179776 logical bytes while RSS rose 130672 -> 133232 KiB and was
unchanged between batches 600 and 1200. Normal rendering resumed successfully.
The unit transition test covers prolonged skips, single consumption, an empty
batch and subsequent normal/skipped submissions. SurfaceRecovery's existing
simulated timeouts/retry budget tests remain authoritative for acquisition.

## Observed window-manager minimize/restore

`PERF-04-MINIMIZE-RESTORE` adds a separate no-ROM, event-loop-driven diagnostic:

```sh
cargo run --locked --release -p recomp --example presenter_window_lifecycle
```

Run it with a real window manager on X11 or Windows. Bare Xvfb has no WM to
minimize the window; the check fails rather than treating a zero-size call as
minimization. Wayland's pinned winit API cannot report minimized state or request
unminimize, so this diagnostic cannot prove that backend's lifecycle. It reports
unavailable state as an error. A close request before completion also fails.

The diagnostic observes three minimized/restored cycles with bounded deadlines,
tries presentation while minimized (Presented or Skipped is permitted), requires
successful presentation after every restore, resizes to 640x360/960x540/852x480
and checks VSync ON/OFF presentation at each size. It releases the Presenter
before the window in exiting and prints PASS only after run_app returns and all
cycles/teardown completed. It uses no ROM, save, fault-injection option, new
runtime behavior or blocking GPU poll.

Native Linux Xvfb/Openbox/llvmpipe Vulkan passed all three cycles, all resize/
VSync steps and clean teardown (27 Presented, zero Skipped). Independent xprop
observed three Iconic and four Normal WM states. Openbox and missing libraries
were extracted into ignored target storage; no system package installation.
The unchanged canonical Linux portable also passed three actual Iconic/Normal
cycles while running the isolated battle, then Undo/reload/resume/WM-close.
The restored battle/HUD capture was inspected. These results validate actual
X11 WM lifecycle, separately from earlier explicit zero-size API/staging tests;
they do not establish native Windows, Wayland minimize/restore, monitor switching
or forced device loss. Shipping binaries/archives are unchanged by this example.

The same example cross-compiled with the existing MinGW13 toolchain and passed
under Wine10/Openbox with llvmpipe Vulkan: 18 Presented/9 Skipped, three observed
Iconic/four Normal states. Wine exposes zero-size minimized windows here, unlike
the nonzero X11 native case. Wine's first default D3D12 adapter failed device
creation; that attempt remains failed evidence. The successful retry disabled
only the test prefix's D3D12 DLLs (`WINEDLLOVERRIDES=d3d12,d3d12core=;winemenubuilder.exe=d`),
leaving production backend selection and portable packages untouched. This is
Wine validation, not a native Windows device result. Ignored result/transcript
locators: `target/desktop-perf/minimize-restore/`, `minimize-restore-game/`,
`minimize-restore-wine/`.

## Virtual monitor transitions

The optional window diagnostic has a separate mode for two separately positioned
monitors (the selected pair must each be at least 1000x600):

```sh
cargo run --locked --release -p recomp --example presenter_window_lifecycle -- --monitors
```

It requires two distinct enumerated monitor handles. It requests five placements
in order A/B/A/B/A, verifies `current_monitor` before drawing, resizes to half the
monitor dimensions (within bounded test sizes), checks visible VSync ON/OFF
presentation and releases the Presenter before the window. It fails on missing
monitors, unsupported moves or an incomplete sequence, rather than counting
window-coordinate requests as successful monitor changes. Default minimize mode
is unchanged and does not run again in monitor mode.

The isolated capability probe found that Xvfb resets RandR monitor definitions
between clients unless `-noreset` is supplied. With that corrected it exposes
two RandR 1.5 areas, but pinned winit enumerates CRTC outputs, so those areas
alone cannot prove this test. A local Xorg/dummy server with AutoAddDevices,
AutoEnableDevices and AutoAddGPU disabled provided two distinct active outputs.
No installed desktop configuration was changed or system package installed.

Native Openbox/llvmpipe Vulkan passed A/B/A/B/A with independent xwininfo position
observations. A same-size 1280x720 pair passed, then unequal outputs 1280x720
and 1280x960 passed with actual 640x360/640x480 surface-size transitions. Each
run reports 20 Presented/zero Skipped, four monitor transitions and clean exit.
This is virtual same-GPU/scale-1 coverage, not physical display, GPU migration,
color calibration, mixed-DPI, hotplug or Wayland/native Windows coverage.
Runtime shaders, backend choices, guest timing and archives remain unchanged.
Results and probe transcripts: `target/desktop-perf/monitor-transitions/`;
the completed same-size run is preserved under `same-size/`.

Windows GNU/Wine10/Vulkan also passed the unequal pair: distinct DISPLAY1/2
handles, independent A/B/A/B/A window positions, actual 640x360/640x480 changes,
20 Presented/zero Skipped and clean exit. It reused the established test-only
D3D12-disabled prefix setup; no backend policy was changed in production.
The actual unchanged Linux portable passed battle load/zoom and four window
moves between the unequal outputs, followed by Undo/reload/resume/WM-close.
Battle/HUD on output B, Undo and reloaded-battle captures were inspected;
positions, exit status and canonical runtime/archive hashes were verified.

The first game script moved the window but stayed in the intro: Xorg dummy's
synthetic keyboard needed explicit evdev US initialization. The initial capture
was not accepted as battle/load/Undo coverage. A focused menu/slot probe verified
`setxkbmap -rules evdev -model pc105 -layout us` before the corrected scenario.
Artifacts: `monitor-transitions-wine/` and `monitor-transitions-game/`; the latter
keeps the rejected first scenario under `input-unverified/`. Runtime/core/shader
suites and portable exports remain valid unchanged; rebuilding them cannot add
coverage of these virtual versus physical monitor distinctions.

## Owned-device loss regression

A private optional `screen` test uses `Device::destroy` against an actual window/
surface. Run alone with an X11 or Windows desktop and GPU/software-surface support:

```sh
cargo test --locked --release -p screen -p recomp \
  present::device_loss_tests::destroyed_device_returns_error_then_software_can_present \
  -- --ignored --exact --nocapture --test-threads=1
```

The test warms presentation, destroys the device and covers plain/cached UI,
new UI creation and framebuffer growth. It requires a controlled error and
successful softbuffer presentation after releasing the GPU presenter, with
surface/context before window teardown. It also checks scoped validation errors
do not poison the Presenter, and that latched errors suppress UI writes, remain
stable and reject VSync. It is ignored by default and uses dev-only winit/
softbuffer dependencies already present in the desktop workspace.

Baseline plain/cached cases returned controlled errors, but new UI after actual
destruction panicked on an invalid native-ui texture view. The fix installs
callbacks on the Device owned by Presenter and stores only its first fatal
uncaptured error/loss in Arc/OnceLock. Later operations return that message to
the existing app software fallback. Local error scopes are preserved; there is
no per-frame allocation, GPU polling/wait, callback log or public fault knob.
Native Xvfb/llvmpipe Vulkan passes all four cases and scoped-error isolation.
Device::destroy is deterministic injection; it does not prove every asynchronous
hardware reset race or real driver fault. Detailed evidence lives under
`target/desktop-perf/device-loss/`.

## Enemy dispatch/host-cost triage

`PERF-07-FALLBACK-CANDIDATES` reuses the observed enemy replay (guest8476->12006,
3530 frames, warm-up120). The existing dispatch census was for the intro; one
new scene census used cost_from8596 and fallback tracing, with all3530 framebuffer/
OAM hashes, final state and execution counts matching the existing headless
reference. Warm-up-excluded dispatches:22626079 native,79140200 missing,6820
system,55906060 intentional hooks. The full replay reports3804 distinct missing
entry keys (3729ROM/75IWRAM). Entries, instruction steps, native blocks and guest
cycles are different units; none is a host CPU percentage.

The shipped release is stripped. For host sampling, only the final recomp crate
was rebuilt with the normal release optimizations plus `-C strip=none -C debuginfo=1`
using cargo rustc. The unstripped executable was isolated under ignored target
storage and the original native executable restored by hash. No release profile,
compiler dependency, translation or package was changed.

One perf cpu-clock:u sample at199Hz with8KiB DWARF stacks was enabled via control
FD/ack after the8596 scene marker. Census, per-frame hash I/O and optional phase
profiling stayed off in this sample.3928 samples, zero lost; final full state and
counts match the same reference. Flat leaf/self attribution identifies
capture_dialogue_part31.52%, render_sprites_extent8.63%, compose_into7.76%,
exec::step_inner6.21%, render_text_bg_hardware5.14%, Machine::step4.56%.
These are sample estimates from one headless native workload, including inlined
work in those symbols; they are not wall times or inclusive call-tree totals.
Call-chain reconstruction does not separate all missing/intentional interpreter
work, so this is insufficient to justify changing direct lookup or translation
semantics. No AI/native Windows speedup is claimed.

That triage selected repeated panel-membership work inside the BG loop of
capture_dialogue_part. The resulting cache and its acceptance evidence are
recorded below; this selection is complete, not a new task to restart. It is
separate from the per-row window register cache and rejected front-two selectors,
and preserves lazy bounds, WIN/blend/portrait semantics and the CPU reference. GPU/full-PPU rewrites do not
follow merely from the large sample share. Evidence: ignored
`target/desktop-perf/enemy-fallback-census/{summary,sample-result,triage}.json`,
census.log, self.txt, children.txt and perf.data. Original profiling/recording
checks remain complete when these local artifacts are unavailable.

## Cached panel membership in native UI extraction

`PERF-08-UI-PANEL-ROW-CLASSIFICATION` keeps a per-call snapshot of banner vertical
membership and panel area rules. Each pixel evaluates that rule only on its
first eligible BG, reusing the boolean for later BG candidates. Missing bounds
stay lazy: an overriding banner or a row without eligible BG never unwraps them.
There is no cross-scanline/frame cache, allocation, sort/blend change or guest/
translation ABI change. The original policy remains an uncached test specialization.

Native serial and Windows/Wine core suites pass173 tests (5 optional ignored).
New old-policy comparisons cover action/town/shop flags, None/Stats/Actions/Notice,
banners, rectangles, range boundaries, primary/extra and absent lazy bounds.
Existing full panel/backing state comparisons use the uncached policy. All3530
replay framebuffer/OAM hashes, final state/counts match at normal and zoom0:6.

Five uninstrumented headless pairs (CPU15,120 warm-up) improve in every pair:
median6.001925->5.840709ms(-2.6861%). Separate instrumented p95/p99/max improve
7.081627/7.251019/8.735002->6.864737/6.994522/7.344674ms; no over-budget frames.
Five complete unpaced GUI pairs on native Linux/llvmpipe Vulkan with CPU profiling
on improve2.108-2.641% each, median7.767086->7.591386ms(-2.2621%). Median GUI
p95/p99/max9.397574/9.895277/11.371842->9.136884/9.641691/11.062622ms. Payload
316299008bytes unchanged; no audio callback, GPU timestamp or per-frame hash I/O
in timing. These are work measurements on the available replay, not native
Windows timing or identification of the originally reported rare stall.

Worldmap240-frame parity and two diagnostic1200-frame pairs pass with comparable
means and unchanged private data. The first setup missed copied empty.input and
failed before advancing the guest; only that scene was rerun after copying the
existing fixture. GUI five-pair results were reused. Normal GUI profiling-off
load/zoom/Undo/reload/resume/WM close passes with inspected battle/HUD and no
fallback. Artifacts under `target/desktop-perf/ui-panel-row/`,
`ui-panel-row-gui-evaluation/` and `ui-panel-row-gui/`.

Final exports pass335 Windows/Wine and336 Linux tests (15 optional ignored each),
archive source/doc/privacy/integrity, PE resource/DLL-set and GLIBC2.35 checks.
Actual Linux portable cold-cache default GUI passes load/zoom/Undo/reload/resume/
WM close on Vulkan without fallback; battle/HUD and Undo captures inspected.
Current local dist archives were replaced after validation with prior versions
preserved under ui-panel-row-exports/previous-exports; no public release.


## Rejected compact native UI layer IDs

After the retained panel-row/world-only pixel changes, a fresh optimized host
sample on the existing enemy replay contains3798userCPU samples, zero lost,
with exact final state/counts and restored normal-runtime/source/fixture hashes.
Current self costs are capture_dialogue_part31.20%,render_sprites_extent9.19%,
compose_into8.85%,step_inner7.64%. This replaces the older sample only for current
hotspot selection; it is not a comparative speed measurement or attribution of
missing versus intentional interpreter work. Do not resample it for context.

The proposed local-ID narrowing (usize->u8, domain0..6) preserves174native/Wine
core tests and3530frame/OAM/full-state/count equivalence. Five headless pairs
improve5.756819->5.679220ms(-1.348%), but a separate instrumented run is contrary
(5.755066->5.833724ms). Five CPU-profile-on GUI pairs give only-0.454% with one
slower pair; five diagnostic-off GUI pairs give-0.774%, also one slower pair,
paired range+0.147% to-1.433%. GUI median maximum increases11.518->15.911ms with
profiling and12.101->16.227ms without it. These runs do not prove a causal
universal regression, but did not demonstrate a sufficiently robust useful
whole-GUI benefit to justify changing the validated implementation.

The candidate is rejected; source and native runtime were restored exactly by
bytes/hashes, and source mtime refreshed so Cargo cannot reuse an experimental
artifact merely because restored source timestamps predate its build. Trial
source/binary/measurement manifests remain ignored under
`target/desktop-perf/compact-ui-layer-id/`; GUI evidence is in the companion
`compact-ui-layer-id-gui-evaluation/`. No further worldmap/zoom/lifecycle/export
gates are needed for this rejected variant. Existing archives remain canonical.
Reopening requires distinct evidence; an unchecked PPU parent is not a reason.

## Rejected immutable ROM seed coverage trial

`PERF-07-IMMUTABLE-ROM-SEEDS` reused the existing enemy fallback census top
entries rather than collecting it again. Eighteen ROM-only Thumb labels were
fed through existing `build --ram --labels` in an isolated portable root, with
identical runtime, boot profiling and TinyCC. No battle RAM was translated and
no intentional interpreter gate changed. The isolated baseline library hashes
exactly like the existing reference; candidate coverage grows28859->29196blocks.

Both3530-frame verification runs have equal framebuffer/OAM captures, initial
snapshot and warm-up scene. Full final serialized-state hashes differ, so the
candidate is rejected for production and no timing pairs ran. Changed dispatch
counts are expected with more coverage and are neither a speed nor correctness
proof. Which state component differs remains unknown; do not claim visible
gameplay/audio damage or fix it by excluding state fields from comparison.

Current runtime, original library, fixtures, game database and packages remain
unchanged. Local ignored `target/desktop-perf/immutable-rom-seeds/` preserves
build/evaluation scripts, manifests, both libraries and equivalence logs.
Jobs74952/4376 are terminal. Read those results before any new investigation;
this rejected trial has no pending performance validation. A separate first
state-divergence investigation is required before further coverage acceptance.

### First audio-state divergence isolated

A separate ignored standalone copy of the unchanged runtime loaded both existing
coverage libraries and compared full snapshots at every frame until divergence.
Frames1..37 match exactly; at relative38/guest8514 both clocks equal2391464782
and CPU state matches, while two mixed samples and HLE verifier fields differ.
The helper continues to3530frames, reproducing both prior final-state hashes
exactly, so this is the same failure rather than an unrelated probe setup.
Production source, runtime, translation and private inputs remain unchanged.

A ROM-free fixture enables FIFO A and a128-cycle timer, then compares tick600
with600 single-cycle ticks. Both end at clock600 with the same DAC value but
produce different mixed samples and timer bookkeeping. MemMap::tick pumps at
the scheduled event before advance_timers observes the final batch clock; this
is a concrete batch-sensitivity reproduction, not yet the complete causal
proof or accepted fix for the battle. Event/DMA/skip ordering and total cycles
must be checked before changing this shared hardware path. Do not discard audio
or verifier fields merely to make the coverage comparison pass.

`target/desktop-perf/rom-seed-state-probe/` contains the standalone copied source,
source hashes, first/final raw snapshots, bounded field diff, verified-result.json,
ROM-free timer fixture/log and helper binaries. No production source was edited.
Initial build lacked pkg-config configuration; the corrected build reused the
existing Linux dependency sysroot. Build26038, diagnosis24808 and timer1662 are
terminal. Use these artifacts for the next event-order investigation; don't
rebuild the old coverage libraries or rerun the completed3530-frame diagnosis.

### Timer/audio event chronology correction

The shared bus clock now services each scheduled event at its timestamp before
advancing to the end of a tick batch. Audio grid samples at the same instant are
mixed before the timer changes the DAC, matching single-cycle execution. DMA
steals extend the caller's endpoint; they do not replace requested CPU cycles.
DMA's own clock advances service timers/audio without reentering video events
and accidentally restarting the active transfer. Halt/IntrWait skips use tick.
No new machine fields, save layout, native ABI or translation seeds are added.

Five ROM-free regressions cover24batch/period combinations, both timers and all
prescalers/cascade with HBlank DMA, FIFO sample-grid ties, sound refills/stolen
cycles and skipped-event execution. The initial batching regression fails on
the old implementation; corrected native/Wine core suites each pass179tests
(5optional ignored), and native runtime66pass (4ignored). Actual corrected CLI
matches all3530old framebuffer/OAM hashes, initial scene and dispatch counts;
its complete final state matches the corrected isolated baseline. Comparing
old/corrected baseline final snapshots finds equal CPU/clock and changes in
only mixed audio, HLE and lazy timer state. Old-state equality is intentionally
not claimed for an audio correctness fix; state gates remain strict per build.

The18-seed coverage candidate remains rejected. With the correction it is equal
for211frames, then differs in CPU/clock at212, with final IWRAM/HLE differences.
The pending continuous-audio pair case did not run after this failure. Do not
accept coverage from this prefix or benchmark the failed extra-seed candidate.
This remaining native block/frame-boundary issue is separate from the corrected
ROM-free event ordering and still requires investigation.

Local `target/desktop-perf/timer-audio-order/` holds source preimage, failing/
passing tests, corrected CLI/state comparisons, isolated binaries, GUI captures
and timing inputs/results. Five alternating pairs measure median mean work
5.756460->5.823067ms (+1.157%,0.066607ms), with strict expected full-state checks
per build; this is a correctness cost, not a speedup. Docker349Windows/Wine and
350Linux tests pass (15optional ignored per target), with archive/privacy/docs/
source/PE/import/GLIBC2.35 gates. The actual Linux export also matches all3530
old frame/OAM/counts and the corrected native final state, then passes cold-cache
GUI old-slot/zoom/Undo/reload/close; captures inspected. Local dist and validated
native outputs were updated only after these gates, prior versions preserved.
No source/schema/ABI change beyond the event-order correction is implied. Native Linux
GUI with an old slot passes load/zoom/Undo/reload/close on Xvfb/Vulkan; images
were inspected. Wine is compatibility evidence, not native Windows timing.

## Native event-boundary candidate under acceptance

PERF-07-NATIVE-FRAME-BOUNDARY decoded the existing frame212 snapshots: IRQ was
pending/unmasked while native code continued to its loop back-edge. Original
synthetic code reproduced8late cycles and changed IRQ-stack bytes. A conservative
entry guard fixed the divergence; full3530-frame state and continuous audio
checks passed on both ROM coverages, as did native/Wine synthetic regressions.
The direct-query reference is preserved under ignored
`target/desktop-perf/native-frame-boundary/hardened/`. Five rotating triplets
measured medians5.976894ms legacy,6.242606ms direct guard,6.056551ms extra coverage.
Those results motivated a measured allowance refinement, not a repeat for context.

The current allowance candidate reserves each block's conservative cycle sum
from a local counter and refreshes it only when insufficient. Its invariant is
that remaining allowance never exceeds cycles to the next hardware event.
Stores/shells invalidate that counter and recheck before continuing because
DMA, halt or IRQ changes can invalidate the entry bound. Near an event it uses
one interpreter instruction, retaining exact frame/IRQ boundaries. The counter
is local to a native invocation and requires no machine-state serialization.

The scheduling query is a reserved zero-length EVENT_GUARD_TAG operation with
no address input. It returns available cycles, with EVENT_YIELD in the high bit
for immediate dispatch. Positive-length operations remain content hashes;
a retained failing test caught the earlier prototype ambiguity before release.
RtApi layout and save schema stay unchanged; cache revision8 regenerates old
code. Prior unshipped revision7 artifacts are historical reference only.

Final allowance native181core/66runtime/3recompiler suites and explicit11-case
TinyCC frame/IRQ/DMA tests pass, including actual native batching, fewer
scheduling queries than loop iterations, read-only query and content-hash
separation. Matching Windows GNU/Wine suites pass. Full state matches at every
3530frame on both coverages, retaining queues and draining them continuously;
7,746,586mixed/PSG/HLE samples per stream and931,920events perFIFO agree. HLE
floats are compared by bits. Actual CLI images, full state and dispatch counts
match the direct-query reference, and legacy raw frame/OAM images match too.

Ignored `native-frame-boundary/allowance/` contains frozen sources, matched
libraries, tests, pair/CLI manifests and the active five rotating timing groups
(legacy/direct/allowance/extra,CPU15,120frame warmup,3530frames,strict expected
state/count checks per variant). Source/performance acceptance, GUI/export and
any automatic ROM hint expansion remain pending. Canonical native binaries and
dist packages still contain the delivered timer/audio correction.
