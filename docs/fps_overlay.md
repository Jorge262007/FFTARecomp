# Desktop FPS overlay

For the desktop portable editions, settings are under `Data/config` and the
cartridge save is `Data/saves/ffta.sav`, with backups and state slots beside it.
See [portable storage and migration](portable.md). Installed-mode paths below
apply when `portable.txt` is absent.


Open the pause menu with Esc, then **Video > FPS counter**. Choose **Off**,
**Internal**, **External**, or **Both**, using Left/Right or the dropdown.
The choice persists as `video.fps` in `av.cfg`; existing configs default to Off.
On Windows this file is `%APPDATA%\FFTARecomp\av.cfg`.
The row explains `INT: GAME / EXT: PRESENTS` and that Fast can show different
rates, so both measures can be understood before choosing the overlay.

The small top-left panel shows:

- **FPS INT**: completed emulated GBA frames per second of wall time. Normal
  speed is about 59.7; fast forward can produce several internal frames per
  presentation.
- **FPS EXT**: successful game-frame presentations per second. It counts host
  submissions, not the monitor's refresh rate or physical scanouts. Failed
  presentations and pause-menu redraws do not contribute.

Values use a one-second measurement window and initially show `--.-`.
Pausing/resuming or changing the selected mode resets the window. The HUD is
hidden while the pause menu is open. It stays the same size when changing
game zoom or aspect ratio and follows window DPI for readability.

The reported difference was confirmed by the user to occur only with Fast
active. This is expected: at requested 4x the loop completes four guest frames
per host presentation. Local inspected captures show 128.9 INT / 32.2 EXT with
Normal audio and 130.0 / 32.5 with Fast audio; after disabling Fast they return
to approximately 59.7 / 59.7. A host that cannot reach the requested speed can
show fewer than 239 INT while retaining this ratio. Neither counter claims
physical display refresh. There is no reason to make these numbers equal or
change pacing for this reported case. Normal-speed anomalies on another setup
would require their own evidence; native Windows hardware remains unverified.

The GPU uses a final native UI layer above the dialogue layers; the software
presenter samples the same panel after composing the game UI. The panel uses
its own high-contrast palette, independently of game brightness and screen
simulation. Text is rasterized once per measurement window. Off creates no
HUD textures or per-frame text rendering. Guest memory, timing and save data
are untouched.

## Validation

The Fast-only clarification and inline menu help were checked without
repeating the completed presentation matrix. The existing native and Wine
3-test meter suites pass (Fast ratio, pause/skipped frames, clipping and
revision behavior), as does the native FPS dropdown test. Windows menu builds
without warnings and all 33 menu tests pass under Wine; its actual 640x480
render is byte-identical to the inspected native preview. The first menu-only
native build lacked Wayland pkg-config data; using the established combined
runtime/menu feature selection succeeded. No pacing source was changed.
Local evidence: `target/fps-menu-help/`; reused runtime screenshots under
`target/fast-audio-rate/` and `target/normal-audio-bounds/`.

Windows x86_64 GNU release builds were executed via Wine:

- 27 runtime tests, including independent internal/external counts during
  fast forward, reset after pause, HUD clipping and packed-layer sampling.
- 25 menu tests, including FPS dropdown commit/cycling and persistence.
- 22 input/config tests, including all four modes, case normalization,
  legacy configs and invalid values.
- GPU/CPU native-layer equivalence on AMD Radeon 780M / Vulkan, including
  the HUD's one-pixel backdrop texture.
- An interactive Windows/Wine smoke run selected Both through the actual
  menu, resumed gameplay and verified the saved `video.fps = both` setting.
  Captures show INT/EXT at 59.7/59.7 normally and distinct rates while holding
  fast forward. The game exited cleanly using an isolated test save.

Logs and visual previews are under ignored `target/windows-fps-*.log`,
`target/fps-*-preview.*` and `target/fps-ui-smoke/`.

## Upload reuse

FPS pixels carry a process-unique revision changed when the text is rendered
or reset. A layout/DPI change still updates uniforms, without reuploading the
unchanged text texture. Native game UI also retains the complete source and
backing inputs plus the exact packing mode, length and anchor parameters.
Identical inputs reuse packed pixels and their revision, skipping both packing
and texture transfer. Any changed input conservatively repacks; clear/load/Undo
invalidates the host cache. No pointer identity or collision-prone hash is used.
Unversioned UI still uploads unconditionally; dynamic world textures keep their
existing update path. Each GPU layer compares
its LUT contents against a retained copy, so in-place color changes are detected
without relying on allocation addresses. Recreated layers begin with empty caches.

Layers using the exact same immutable world slice and dimensions in one
presentation share its GPU texture and upload. Different backgrounds, including
Stats' backing and the FPS backdrop, remain distinct. Each unique world is
refreshed on every call; there is no cross-frame address-based content cache.
Shared handles split before different-source writes, and changing dimensions
rebinds the world while retaining valid UI/LUT textures.

For an unchanged 480x160 UI with a one-pixel world, scheduled update payload
with a 320x160 clip falls from 443,458 bytes to 66 bytes (world pixel plus uniforms); the first update
and subsequent text changes still upload their required data. This is a transfer
payload reduction, not an FPS or AI-turn speedup measurement. CPU fallback reads
the same current frame data directly. Opt-in `FFTA_PROFILE=1` diagnostics report scheduled UI payload bytes at shutdown.

The extended GPU/CPU equivalence test checks repeated frames, in-place LUT
mutation, changed UI revisions, unversioned UI, changed world data and recreated
resources at three world sizes including 1x1. On AMD Radeon 780M / RADV it passed
all pixel comparisons and exact scheduled-byte assertions. Meter tests cover
unchanged revisions between measurements, reset, DPI/layout changes and distinct
meter instances.

### Temporary surface failures

A GPU surface timeout skips that host presentation and retries on the next
normally paced frame. It does not disable GPU rendering or increment EXT.
Pending texture/uniform writes are submitted even when a frame is skipped,
so repeated timeouts cannot accumulate unsubmitted staging allocations. These
transfer-only submissions do not draw, increment EXT or request GPU timestamps.
Zero-sized desktop windows bypass new UI/framebuffer uploads and software-surface
creation; emulation/audio pacing remains independent.
Lost/outdated surfaces receive one reconfiguration/retry per frame; three
consecutive failed recoveries activate software fallback. Successful acquisition
resets that budget. Memory exhaustion and unknown errors fall back immediately.
Zero-sized windows skip GPU acquisition. Guest frame accounting stays separate.

Surface configuration itself can also fail, even after capabilities were
reported. Startup, resize, VSync changes and acquisition recovery now capture
wgpu validation/internal/memory errors and return them to the runtime, which
switches to the existing software renderer. There is no panic, tight retry
loop or replacement of the global error handler for these configuration errors.
All scoped errors are drained before returning. Normal frames with unchanged
size/mode do not perform this check. Surfaces/contexts still release before the
window and display connection, including a failed GPU initialization.

Native packing-cache reference cases cover in-place changes, blending, backing,
mode/style/anchor changes and clear/reactivation. On the existing 3530-frame
battle replay, five alternating native Linux A/B pairs reduced median mean
host work from 8.340 to 8.222 ms (-1.41%) on llvmpipe Vulkan; every verification
frame/OAM hash, full final state and dispatch count matched. Logical UI payload
fell about 74%. Two world-map diagnostic pairs showed comparable work times;
these checks do not establish native Windows or AI-turn speedups.

With `FFTA_PROFILE=1`, normal play also reports bounded audio callback lock/work
times and faithful-source holds at shutdown, separated by playing, priming,
pause and fast-forward. This diagnostic does not change the FPS counter or
pacing. Enhanced-path starvation and backend XRUNs are not counted by it.

On the established 3530-frame replay, scoped world reuse reduced scheduled UI
payload from 530412288 to 316299008 bytes (about 40%). Five alternating native
Linux pairs gave median mean work 8.251 -> 8.219 ms (-0.39%), with slightly lower
median p95/p99. This is a small unpaced llvmpipe improvement, not an AI-turn
speedup claim. GPU readbacks match independent uploads across split/merge,
mutation, absence, resizing and resource recreation with distinct layer colors
and layouts. Fractional crops now use the same CPU-defined integer texel maps on both
paths, including UI glyph selection and world sampling.

## Native sampling consistency

Native UI sampling uses the existing software pixel-centre policy. A two-row
RG32Uint texture holds UI/world indices for each physical axis; exact layout,
clip, world-view and dimension keys reuse it until geometry changes. This avoids
backend floating-point reassociation at texel boundaries. The map is bounded by
device texture limits and is rebuilt with GPU resources. Palette/text changes
keep their existing independent revision handling.

Rasterization is clipped to a conservative rectangle derived from visible UI
pixels and those same integer maps. The shader's exact discard predicate
(`eva=0, evb=16`) defines transparency; black and partially blended pixels remain
visible. Source bounds are rescanned only when UI pixels upload. Empty draws
retain their source ownership, preserving shared-world updates. Scissors are
clamped to the actual render target after resize. This changes no guest state,
zoom factor, color blending or save format.

The sampling-map/bounds implementation passed five alternating replay pairs:
median mean host work 8.230 -> 7.775 ms (-5.53%) on unpaced llvmpipe Vulkan,
with lower median p95/p99 and identical guest state/counts. The first table-only
prototype was rejected for +4.17% work; cached visible bounds recover that cost.
This is not a measurement of native Windows or isolated AI-turn performance.

The presenter retains the first uncaptured error or loss callback from its own
GPU device. Failed UI/texture creation returns that error to software fallback
instead of panicking. Later UI writes and VSync changes are rejected on that
failed presenter. Scoped configuration errors remain local. These failure checks
add no per-frame allocation, logging or GPU wait, and do not change INT/EXT.

Native panel extraction snapshots banner/bounds rules once per scanline capture
and classifies each pixel only on its first eligible background. This preserves
window/blend/portrait behavior and lazy bounds handling without a cross-frame
cache. In the measured native Linux replay, five pairs reduced median core work
by 2.69% and complete unpaced GUI work by 2.26%; these are work-time measurements, not
an increase in the GBA frame rate or proof of the original rare AI slowdown.

Within nonempty capture rows, pixels without an admitted UI background or
portrait now bypass world-only sorting/blending. Their destination is still
cleared; real UI keeps its full ordering and blend targets. Five native Linux
pairs preserve all replay images/state/counts while reducing median core work
4.20% and complete unpaced GUI work 2.67% relative to the panel-row cache above.
These measurements do not establish native Windows or original AI-stall gains.

Developer audio profiling can select one realtime guest-frame interval with
`FFTA_PROFILE=1 FFTA_AUDIO_PROFILE_FRAMES=START:END`. It excludes warm-up outside
the interval, retains separate pause/priming/fast-forward contexts and closes on
rewind or load during capture. This does not change audio or the FPS counter;
see the performance workflow for frame attribution, report checks and limits.

With audio profiling enabled, `FFTA_AUDIO_SOURCE` separates raw PSG/HLE/FIFO
requests and empty reads from configured-target requests. Consumers kept warm
can legitimately read empty sources; these counters do not report audible
underruns or driver XRUNs. Normal playback keeps source counting compiled out.

Optional developer profiling also reports intervals between successful host
presentations with `FFTA_PROFILE=1`; `FFTA_PRESENT_PROFILE_FRAMES=START:END`
selects a guest-frame window. This uses the existing FPS timestamp and leaves
HUD behavior unchanged. It measures host API cadence, not monitor scanout;
see the performance workflow for lifecycle exclusions and report interpretation.
