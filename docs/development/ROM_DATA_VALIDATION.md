# Decoded resource validation protocol

ROMDATA-05, selected before measurements on 21 October 2026. This first
tranche validates `graphics.obj-dimensions`; it does not close the remaining
resource matrix or mod phases. Existing implementation/results are in
[runtime consumption](../rom_data_runtime.md) and [TASK_HANDOFF](TASK_HANDOFF.md).

**Current user steering (21 October 2026): avoid performance measurements and
optimize later.** The time/RSS/storage thresholds and results below are retained
historical evidence. They do not authorize new benchmark/profile/startup/RSS
runs or regression investigations now. Functional per-frame state/audio/clock,
file provenance, native/save/lifecycle and packaging correctness remain in scope.

## Acceptance limits fixed before comparison

- Guest state at each compared frame, central/wide pixels, OAM, RNG/clock and
  deterministic audio must be exact. Final-state hashes alone do not claim a
  complete audio-stream comparison. Reuse passed boot/original-routine checks;
  add world map, observed battle/menu and relevant transition coverage.
- Five alternating release timing pairs per representative replay: candidate
  median mean frame work may regress at most 2%. Separate instrumented profiles
  use diagnostic thresholds of +5% p95 and +10% p99. Exceeding these limits
  requires investigation before normal enablement, not deletion of the result.
  A host-load/setup failure invalidates only the affected timing observation.
- After the ROM is selected/read, first preparation plus activation has a
  median budget of 250 ms and ready-dataset activation 100 ms. Measure seven
  first/cached observations, retain all values and maxima, and separate source
  I/O, preparation, activation and whole-process launch where observable.
- Owned first-family runtime resources stay below 1 KiB. Independently measure
  peak RSS: the candidate may add at most 2 MiB versus corrected legacy startup,
  allowing normal allocator/page granularity without hiding a ROM-sized copy.
  Report type/capacity accounting separately from process RSS.
- One valid generation's logical data/metadata is at most 32 KiB and allocated
  storage at most 128 KiB on the tested filesystem. Retained old/orphan generations
  are reported separately; do not delete user data to make a space result pass.

These are local regression gates, not native Windows, USB-drive, physical GPU
or universal storage guarantees. No speedup is presumed against in-memory ROM.

## Reference and inputs

The legacy reference must include only the independently verified EUR host
dimension-source correction on the prior preparation baseline, without the new
ROM routing/resource fields. Comparing only `--rom-data` on/off in the new
binary would omit always-present routing/layout overhead. Build the reference
and candidate sequentially from the same isolated source path with identical
Rust/toolchain/profile settings; protect root sources and canonical executables.
Retain an additional flag-off result if useful to attribute costs.

Use the already verified private worldmap snapshot/neutral input and observed
enemy replay/checkpoint under ignored `target/desktop-perf/`. Preserve hashes
of ROM, snapshots, recordings, translation and TinyCC loader before/after.
Do not recreate fixtures or reinterpret an introductory demo as an AI scene.
Each platform uses its corresponding native translation and loader.

Use the existing benchmark script, exact full-state/count verification and a
fixed zoom/FOV/warm-up. Verification with per-frame hashing precedes timing;
timing checks final identity without per-frame hashing. CPU affinity does not
reserve a core: inspect host load and the SMT sibling before/after each batch.
Do not accept timings collected while compilation or another heavy job runs.

Additional GUI checks cover decoded-data mode with Fast, pause/resume and the
necessary restoration/transition paths. Passed file-level Load/Undo/backup
resource-retention tests remain closed unless code/input changes or a new GUI
criterion requires another observation. Packaging must select distributable
inputs explicitly and exclude extracted data, saves, recordings and generated
ROM code. Native Windows hardware remains a distinct required gate.

## Results

The first native Linux comparison uses the same isolated source path, release
flags and TinyCC translation for corrected legacy and decoded-capable builds.
An initial candidate copy retained old source mtimes, so Cargo reused the
reference; identical binary hashes exposed this before any measurement. Source
mtimes were refreshed and the candidate rebuilt. Distinct hashes and runc help
were verified. Root source bytes and canonical binary were preserved.

Five alternating pairs pass the preselected mean/tail limits:

- World map,240frames/120warm-up: median2.153946 ->2.175660ms (+1.0081%);
  instrumented p95+1.7287%, p99+0.2292%.
- Observed battle/menu/enemy replay,3530frames/120warm-up: median5.833322
  ->5.819583ms (-0.2355%); instrumented p95-3.7654%, p99-4.1991%.

Every frame's framebuffer/OAM, loaded identity, final full state and native/
fallback counts match; candidate counters show720/16988 guest resource reads
and2640/314108 typed host reads respectively. Source/fixture/library hashes
are unchanged. CPU15 affinity, no concurrent compilation; sibling CPU14 mean
busy0.745%/0.189%, one-second maxima2.94%/7.00%. These local headless results
are neutral within small variation, not a claimed speedup, realtime audio or
native Windows result. Profiles are separate instrumented observations.

Private evidence: `target/rom-data-validation/` reference-build.json, repaired
build log/help/hashes, native-{worldmap,enemy-menu}/summary.json and individual
logs, runner logs and per-second host-load observations. Full per-frame
state/audio and startup/storage/RSS results follow below. GUI
The later GUI/archive results below close those local gates; native Windows
remains open; the subsequent independent USA initializer result follows below;
the comparison above alone does not enable the default.

### Native Linux complete-state and startup observations

The complete-state/audio harness runs two instances of the current runtime
(legacy reads versus attached decoded data); the separate corrected-old-binary
benchmark above checks every image/OAM and final state and includes layout/
routing costs. These are complementary comparisons, not interchangeable ones.
New32:9/zoom0 checks pass every full state, central/wide image, OAM, dispatch
and mixed/PSG/both FIFO/HLE output for worldmap240frames, recorded battle/menu/
enemy3530frames and clean boot with the protected suspend save through8476frames.
HLE output is nonempty and compared by bits after identical observer setup.
The boot replay first reads the primary source at frame541/clock151880961 and
the copied alias at frame5400/clock1516754627. Its mask3 is actual alias-read
coverage during the transition, with all guest RAM/clock/RNG states matching;
no DMA/routine-PC attribution is inferred from this per-frame observer.
The8476-frame comparison produces18599928mixed/PSG samples,2236876per FIFO
and18578590HLE samples per machine. Protected input/cache/loader hashes match.

Seven first/cached observations through the production helpers pass:

- First median32.415695ms/max34.267605ms after source read: source decoder
  median6.881894ms, preparation11.125100ms, activation13.941984ms.
- Cached median21.018705ms/max21.626677ms: source decoder6.912662ms,
  preparation0.195377ms, activation13.886811ms.
- Source-file read medians1.478829/1.292860ms are separate. These local reads
  have the host page cache available, not a physical cold-storage guarantee.
- Owned resources432B. Independent seven-pair whole-process peak RSS medians
 182652/183324KiB (legacy/candidate), maxima182772/183524KiB: median delta672KiB,
  maxima delta752KiB, both below2MiB. Each wrapper has exactly one child to avoid
  cumulative child `ru_maxrss`. Whole-process median wall224.294/245.942ms is
  separate from the after-ROM-read helper budget; it includes the native loader.
- One generation:1146logical file bytes,12288allocated bytes including its
  directory. A complete one-generation managed root, including selection/lock
  and ancestor directories:1164logical bytes/32768allocated bytes, four files.
  No old/orphan generations exist in these new measurement roots; none of the
  previously retained datasets were deleted. The seven-root totals and all
  observations remain in startup-results.json, not presented as one dataset.

These costs pass the stated native Linux gates. GUI/audio callback behavior
and native Windows remain separate criteria; actual callback lifecycle is
covered below.

### Windows GNU through Wine

The same three new differential checks pass240/3530/8476frames independently
against that platform's legacy-read instance and existing TinyCC ELF object.
Every full state/image/OAM/dispatch/audio comparison passes; mixed/PSG/FIFO/HLE
sample totals match the native Linux observations, although this does not claim
cross-platform machine/dispatch identity. The copied alias is actually read at
frame5400/clock1516754627, mask3; primary first read541/clock151880965.
The8476-frame pair executes47977490native blocks and91433515fallback steps.
Existing Wine D-Bus background diagnostics also occur here; both affected
checks terminate with an actual passing result, without a production workaround.

Seven first/cached observations pass their predefined **median** limits:

- First median109.222100ms/max194.175200ms after source read; component medians
  source decoder25.476500ms, preparation28.404500ms, activation56.181100ms.
- Cached median91.891400ms/max183.130300ms; source decoder25.546000ms,
  preparation10.462300ms, activation56.038600ms. The maximum exceeds100ms;
  the preselected100ms budget is for the median, not every launch. Retain this
  variability rather than rerunning until it disappears.
- Separate source-file read medians37.396000/36.691600ms, owned resources432B.
  Generation files1146logical/12288allocated bytes; complete managed root
 1164logical/32768allocated bytes on Wine's Z drive backed by the Linux host
  filesystem. No native Windows disk, peak RSS or frame-time claim follows.

Evidence: windows-{worldmap,enemy-menu,battle-entry}-full-state-audio.log,
windows-scene-tests.json (protected input hashes), startup-windows/startup.toml
and windows-startup-results.json under the same ignored validation directory.
Native and Wine test builds are warning-free. The new tests are opt-in; closed
production suites and existing boot/matrix/lifecycle gates were not repeated.

### Package selection

The portable whitelist now includes `docs/rom_data.md` and
`docs/rom_data_runtime.md`. A synthetic tar.gz/zip test plants private markers
at both repository/runtime roots in Data/resources, other resources, native
cache, generated code and ROM/save/state/recording neighbors. Both archive
formats include the selected runtime/help bytes and exclude every marker.
This validates selection without copying private game data; it is not an
actual new release export or archive GUI check.

## Reproducing the optional private checks

`rom_data::validation_tests::decoded_base_matches_recorded_scene_at_every_frame`
uses `FFTA_STATE_TEST_ROM`, `FFTA_STATE_TEST_NATIVE`,
`FFTA_STATE_TEST_INPUT` and an explicit `FFTA_STATE_TEST_FRAMES`. Select either
`FFTA_STATE_TEST_STATE` (existing desktop/raw snapshot),
`FFTA_STATE_TEST_SAVE` (read-only cartridge save), or neither for clean boot.
The same transactional loader/input/native loop is used for both machines.
`FFTA_TCC_LIBRARY` selects the corresponding TinyCC loader; the Windows cache
is an ELF object despite its DLL filename. Set widescreen/zoom explicitly.
Run this ignored release test with `--exact --ignored --nocapture
--test-threads=1`. The existing600-frame boot test remains a separate closed
check and need not be repeated to run this one.

The recorded-scene check compares the untouched loaded states first, then arms
both audio shadows/taps identically after loading and clears accumulated host
output equally. Every subsequent state includes guest CPU/clock/RNG/memory;
central/wide images, OAM, dispatch and mixed/PSG/FIFO/HLE output are also compared
explicitly before draining. HLE samples compare floating-point bits, and every
stream must contain actual samples. Source-mask changes log the first completed
frame/clock. `FFTA_STATE_TEST_REQUIRE_SOURCE_MASK=3` additionally requires reads
from both primary and copied source ranges; a missing range is a coverage
failure, not proof of initializer execution.

`rom_data::validation_tests::measure_decoded_startup_costs` takes the private
ROM and a **new** `FFTA_STATE_TEST_COST_ROOT`. It saves all14observations in
`startup.toml`, retaining seven independent generation roots and their cached
reuse. Source-file reading is timed separately; machine construction (common
to both paths) precedes the source decoder/preparation/activation clock.
Explicit-root lookup and progress-console formatting are outside those helper
timings; independent whole-process startup/RSS observations include normal
startup behavior. Report every value/max and evaluate the preselected medians;
do not delete earlier outputs or rerun an exceeded budget without a reason.

## Actual GUI and callback lifecycle

ROMDATA-05-OBJ-GUI is complete for local EUR Linux/Wine coverage. Source-verified
production candidates and previously validated immutable datasets were reused;
no runtime rebuild, extraction, benchmark or recording was needed. New isolated
config/save/cache roots import the protected frame8476battle slot read-only.

Linux with native guest translation, and Windows/Wine with native translation
and interpreter, each pass actual Slot Load, held Fast/release, toggle, pause/
Return-resume keeping Fast, return to1x, Undo load, reload, resumed battle and
clean WM close. Captures show about4:1 INT/EXT in Fast and about59.7/59.7at1x.
Native dispatch uses each platform's existing hash-protected TinyCC ELF object.
Positive decoded usage (guest/typed host ROM/typed host RAM):14640/27061/222040
Linux;15744/29102/238784Wine native;14874/27473/225589Wine interpreter. No
regeneration or dynamic-RAM fallback occurs; original resource and slot hashes
remain unchanged. These counters are cumulative session use, not per-phase
trace or instruction timing. Native Linux's smaller virtual screen clips the
wide window right edge; inspected FPS/menu/battle establish these controls,
while complete-image equality is the already closed headless criterion.

Actual callbacks run at44.1kHz Linux/48kHz Wine with final volume0. Playing,
paused and Fast contexts all have callbacks; Fast382/1634/1631respectively.
All sessions report0device errors, callbacks over period and producer drops.
Fast inclusive callback p99 is0.836176/0.265100/0.256200ms. The whole-session
profiles include transitions: Linux playing has17callbacks with faithful holds,
and Fast waiting device frames11290/7680/7200are retained, with0rebuffers and
maximum estimated source queues156.616/122.350/114.466ms. This is actual
callback/lifecycle observation, not steady-play zero-starvation, an audio
quality judgment or an A/B callback-performance claim.

A separate Wine/native GUI backup check observes Cancel selected by default,
explicit Restore and restart, success feedback, exact startup save bytes while
paused, Resume into clean boot and clean close. Backup bytes/order do not
change, slot/source save hashes stay protected, and decoded resource usage
survives the reset. Three stream constructions report0device errors. Closed
file-level Load/Undo/backup retention tests are reused, not rerun.

Private evidence: `target/rom-data-gui/verification-summary.json`, per-mode
result.json, measurement-inputs.json, play logs, inspected PNG captures and
helpers. Every process52685/14957/14446/26083is terminal. The observed controls
close this bounded GUI gate. Actual archive evidence follows below; USA
initializer is covered below; native Windows and other families/mods stay open.

## Actual portable archives

ROMDATA-05-OBJ-ARCHIVES is complete for the declared container/host scope.
The packager required rom_data.md/rom_data_runtime.md, but the Docker context
allowlist and COPY lists omitted them. Both recipes now include those explicit
files. Frozen input hashes identify the actual worktree rather than assuming a
commit contains all uncommitted implementation files.

Both real builds/test/exports pass:392Windows GNU/Wine and395Linux tests,
24optional ignored per platform. No compiler warnings occur; cold Windows apt
setup reports only absent optional man-page links. Exported zip/tar files pass
integrity, safe path/member checks, executable modes and public source/document
hash checks. No Data files, private game/resource/save/recording/cache/code
members or matching known private content hashes occur, including inside the
TinyCC source zip. Windows has only the intended libtcc DLL, PE resources and
no unbundled GCC/C++/pthread runtime imports. Linux maximum required GLIBC is
2.35and host ldd resolves its libraries.

The actual extracted executables automatically find the bundled TinyCC, create
portable Data/resources with no resource override, load the protected battle
slot, Undo/reload/resume and close0. Windows guest/host ROM/host RAM resource
reads3084/5875/46774; Linux3072/5870/46592. Captures are inspected. All original
fixtures and public archive-member hashes match; private test caches/data are
added only after archive inspection, while zip/tar bytes remain untouched.
This validates these artifact bytes, not the old host candidate by substitution.

The post-build preservation guard finds one prior BUILD-INFO.txt hash changed
outside the selected export path. All prior archive bytes and both canonical
runtime hashes are unchanged; current Linux metadata matches the identical
archive. The cause is unestablished and an exact old preimage is unavailable;
retain this exception rather than inventing old ldd ASLR addresses. Both build/
export stages had already passed, so they are not rerun. The first scanner also
mistook a harmless empty TinyCC source file for the empty prepare.lock payload;
excluding that non-asset identity and byte-checking the existing extraction
completes the failed inspection without another build/unpack.

Evidence: `target/rom-data-archives/` build-results.json/source hashes,
windows/linux-build.log and test-counts.json, both docker-* archives/BUILD-INFO,
platform-inspection.json/PE dumps, extracted runtime result/measurement JSON,
logs/PNG captures and verification-summary.json. Windows archive SHA-256
88ea78fc5c03fe5dc3c073086b2f18d7b7106ed08cf2d5bb705a3c2d5f3b5b94;
Linux08726f7fcea11f0ed4bfd967b70eefdee49693fb09a44a307d0ddd968f44904c.
Handles4102/48361/6979are terminal. Native Windows still gates normal enablement,
and the full remaining resource matrix/mods stay open.

## Original USA initializer

ROMDATA-05-OBJ-USA-INITIALIZER is complete on Linux and Windows GNU/Wine.
Independent pointer searches and original disassembly identify USA entry
0808f74c, sourcea38adc..a3991c, destination03005f28 and CpuSet wrapper0814186c.
EUR entry080924e8 has independently read literals ford968e8..d97728; no regional
offset arithmetic is used as evidence. The3648byte blobs contain code and the
48byte dimension alias, at relativee00; only that metadata is redirected.

The optional test boots the exact private USA image without a save or forced
PC/registers, follows standard demo input and reaches the actual entry at
frame791/clock222325013. A legacy instance loads this real caller checkpoint.
All13original instructions compare full serialized guest states; the actual
return is08023f76 and all3648IWRAM bytes match the original source. The CpuSet
instruction performs24decoded halfword reads from the alias. A second instance
authenticates the same files first, loads the same checkpoint and poisons only
the48old raw-ROM alias bytes. It still copies the complete blob correctly in13
steps with24decoded reads. The poison is removed only after guest return to
compare snapshot ROM identity and complete state against legacy. Edited base
data remain rejected by production activation; this is a test-only probe.

Both targets compile warning-free and pass the exact new ignored test (3.44s
Linux,3.53s Wine), with identical trace fields and protected ROM/source/canonical
binary hashes. The earlier complete suites/scenes/archives are not repeated
for this cfg(test)-only addition. No production runtime or public archive changes.
This is initializer-instruction parity, not whole-USA boot/scene/audio parity.
The user confirms native Windows is unavailable; Linux/Wine do not waive it.

Reproduce with the existing private-test build/run setup and test name
`rom_data::validation_tests::usa_copied_obj_initializer_from_clean_boot_uses_decoded_source`,
`FFTA_STATE_TEST_ROM` pointing to the exact USA image, `FFTA_WIDESCREEN=0` and
`--exact --ignored --test-threads=1 --nocapture`. The default12000frame limit
can be set with `FFTA_STATE_TEST_FRAMES` (1..30000); failure to reach the actual
entry fails the test. No private ROM/instructions are included in test source.
Evidence: ignored `target/rom-data-initializer/` copy-pointer-xrefs.json,
regional overlay-copy disassemblies, both build logs/executable metadata,
native/windows-result.json and terminal usa-initializer logs. No live process
or remaining deliverable for this bounded gate; other families remain open.

## BG metadata scene and cost protocol

ROMDATA-05-BG-SCENES-COSTS is selected after the bounded04consumption unit.
Its changed fixed bus pages/owner make new comparisons necessary; historical
OBJ-only results remain closed at their original revision. Reuse existing e3
files, recorded scenes/save/input and platform-specific TinyCC objects. No
fixture is recreated merely to recover context.

Before any new measurement, retain exact full-state/image/OAM/clock/RNG/audio
and dispatch equality, five alternating timing pairs with+2%median mean,
+5%p95/+10%p99diagnostic gates, seven first/cached observations with250/100ms
median limits, shared/OBJ<=1KiB and extraBG<=64KiB, plus peakRSS delta<=2MiB.
The new three-file generation has a128KiB logical/256KiB allocated disk bound;
the old32KiB OBJ-only bound predates the106309byte BG file and is retained
only for its original scope. Report all observations/maxima and old/orphan
storage separately; preserve prior generations and private inputs.

Use the frozen warning-free managed CLI reference and04BG candidate compiled
at the same root source path/toolchain/release flags. Reference flag-off versus
candidate flag-on measures the full current decoded set against the preceding
ROM-memory path, including the added fixed gate/owner overhead. Source/hash
metadata must establish the comparison; on/off in the new binary alone would
omit its always-present changes. A separate managed flag-on result is needed
only if attributing the BG increment requires it. Original no-resource-layout
OBJ measurements remain separate; no exact whole-history additive speedup is
inferred. New source builds are not needed for already frozen production code.

CPU affinity is not exclusivity: sample the selected CPU and SMT sibling,
run timing after builds/verification have terminated, and retain host-load
failures. Startup source-I/O/decode/preparation/activation and whole-process
wall are separate. GUI/Fast/pause/Load/Undo/backup, archives, native Windows and
payload/catalog/mod work remain distinct gates.

The existing ignored diagnostic
`rom_data::validation_tests::decoded_base_matches_recorded_scene_at_every_frame`
accepts `FFTA_STATE_TEST_DATASET` to reuse a source-authenticated directory.
It emits `ROM_DATA_BG_SCENE` with bus operations/projected bytes. Set
`FFTA_STATE_TEST_REQUIRE_BG=1` only for a replay expected to read BG metadata;
a stable world-map segment legitimately has no such reads. All previous
state/image/OAM/dispatch/full-audio comparisons remain exact. With no explicit
dataset, the helper retains its private temporary extraction path.

### BG metadata e3 results and RSS failure

All new recorded-scene comparisons pass independently on Linux and Wine:
worldmap240,battle/menu3530and boot-to-battle8476frames. Every full state,
central/wide image,OAM,dispatch and mixed/PSG/both FIFO/HLE stream matches;
HLE compares bits with nonempty streams. Stable segments read BG0; the full
transition requires and records53BG reads each. No missing use is inferred
from the stable scene. These checks are closed for the frozen04runtime.

Five alternating native timing pairs against the frozen preceding ROM-memory
path pass all declared limits:worldmap2.153014->2.182296ms(+1.3600%),observed
battle/menu6.011131->5.974965ms(-0.6017%). Separate p95/p99deltas are+1.7286/
+2.0514%and+1.0092/-2.0592%. CPU15's SMT14mean busy0.661/0.578%,one-second
maxima1.98/12%; no agent build/heavy verification overlaps timing. This is a
neutral local headless result. No realtime/native Windows speedup is claimed.

Seven first/cached observations give medians51.943/40.225ms Linux and66.387/
47.001ms Wine, maxima58.944/41.429and72.946/49.630ms, passing250/100ms gates.
Owned memory29112B and one generation107861logical/118784allocated bytes
pass their predeclared bounds. Its managed root is107879logical/139264allocated
bytes including selection/lock/ancestors. No old/orphan generation exists in
these new measurement roots; prior user generations were retained. Wine uses
host-backed Z storage, not a native Windows filesystem measurement. Whole
ready-dataset command medians264.836->314.892ms Linux and364.999->415.250ms
Wine warm-prefix include common ROM/loader work and are separate observations.

**Native RSS fails the2048KiB gate:** seven pairs have median+2864KiB and
maximum+2944KiB. Raw values and failed status remain retained; normal enablement
is withheld. Prepared-only e3 reference observations peak similarly (median
184996/max185140KiB) to the active owner (185004/185224KiB). This attributes
most excess to preparation/parsing; distributions do not establish a precise
per-owner RSS increment. Owned type accounting is not a substitute for RSS.

An isolated custom allocator measures the actual typed162-record parse and
checks its complete source projection digest. Existing TOML0.8requests1942055B
of peak temporary heap; inline-table TOML1952069B and cached TOML1.1requests
3381232B. Both alternatives are rejected as memory remedies. Direct cached
rlib setup initially mixed build/runtime serde traits and failed before any
parse; a separate Cargo graph fixes only that diagnostic setup. No production
parser dependency changes. JSON directly deserializes into the same typed data
with22528Bpeak; default pretty output149261Bexceeds the128KiB logical gate.
One-space pretty JSON123663Bfits it with the manifest/OBJ files and retains the
same low parser peak. This is a concrete next encoding remedy, not an accepted
runtime/RSS result or a reason to widen the measured limit.

Evidence:target/rom-data-bg-validation/verification-summary.json, frozen-inputs,
scene logs/protected inputs, native-{worldmap,enemy-menu} summaries/all raw runs,
host-load samples, startup TOMLs/results, RSS/launch logs, cost-summary and the
isolated parser/prepared-only attribution. Scene/time/startup/disk observations
are terminal; the RSS gate stays open pending a versioned bounded-format change
and actual runtime validation. No production rebuild, canonical update, export,
AndroidPort change, agent commit/push or old fixture recreation occurred.

### Updated user preference and JSON encoding

The user now explicitly asks to prioritize the full migration without reducing
or compressing the space used by extracted ROM data. Earlier measurements and
failed thresholds above remain honest historical evidence; they do not trigger
further disk packing or indentation tuning. Current e4-d3 adopts standard
readable JSON149261B for BG,150818B per generation, with direct typed parsing.
The one-space prototype is not adopted. Per-file limits remain malformed-input
bounds. No current whole-process RSS result is inferred from the isolated heap
probe, and no new budget-only benchmark campaign is required by this steering.
Functional source/state/timing/native/save equivalence and native Windows
requirements remain separate from that storage preference.

### Base-palette e6 scene acceptance

ROMDATA-05-BG-BASE-PALETTE-SCENES selects new changed-input checks after the
completed e6base-palette runtime unit. The predeclared criterion is exact
full state/image/OAM/native-dispatch and all deterministic audio streams at
every frame, with nonempty mixed/PSG/bothFIFO/HLE streams. Reuse existing
worldmap240, battle/menu3530and boot-to-battle8476fixtures, exact source/cache
hashes and current source-authenticated e6files; no recording is recreated.
The ignored diagnostic now emits `ROM_DATA_PALETTE_SCENE` source bus/byte and
decoded-call/output counters. `FFTA_STATE_TEST_REQUIRE_PALETTE=1` requires
positive decoded-bank use only for the transition. Stable scenes with zero
palette calls provide parity coverage, not new source-use proof. These new
inputs justify scene comparisons; prior e3results remain their frozen outcomes.
Timing/startup/RSS/storage/GUI/archive and native Windows are separate criteria.
All six comparisons pass independently on Linux and Windows GNU/Wine:
240worldmap,3530battle/menu and8476boot-to-battle frames each. Every full
state, central/wide image, OAM, dispatch and mixed/PSG/bothFIFO/HLE-bit stream
matches. Stable worldmap/battle segments record zero palette/BGsource reads;
the transition consumes one160Bdecoded palette (one source tag byte) and53BG
reads. This is the observed scene coverage; all162palette records have separate
original/native provenance coverage in04.

Nonempty audio totals per scene (mixed,PSG,FIFO0,FIFO1,HLE) are identical across
targets:526680/526680/63360/63360/526604;7746586/7746586/931920/931920/7746510;
18599928/18599928/2236876/2236876/18578590. Native dispatch counts differ across
the existing platform caches; each target compares only to its own legacy path.
Source, dataset, snapshot/input, TinyCC/native-cache, test executable and
canonical runtime hashes remain protected. Existing e6files are reused without
new capture/extraction. Only the ignored diagnostic changes; no production
rebuild or repeated core/default suites are needed for this test-only addition.

Evidence:target/rom-data-bg-palette-validation/verification-summary.json,
six complete-state/audio logs, protected manifests, source hashes and warning-free
test-build metadata. All handles10397/47587/24819/44936terminal. No timing,
GUI/archive/native Windows/default-enablement result follows from this unit.

### Base-palette e6 cost selection

ROMDATA-05-BG-BASE-PALETTE-COSTS reuses existing warning-free e5production
reference (ROM-memory path) and e6decoded candidate, compiled from the same
root/profile/toolchain. Their recorded hashes are verified before measurement;
no new production build or fixture is needed. Current new owner/source routing/
HLE changes justify new cost observations. Retain seven first/cached startup
and native RSS pairs, plus warm-prefix Wine whole-command wall, then five
alternating native timing pairs with per-frame verification and separate
profiles on existing worldmap/battle replays. No competing build/heavy agent
job during timing; observe CPU15and SMT14load.

Existing time limits remain250/100ms median first/cached preparation+activation,
+2%mean/+5%p95/+10%p99frame work. Keep the historical+2MiB RSS diagnostic and
all observations/failures. No disk-size budget or new palette owner cap is
imposed, and no compression/format/packing campaign follows from storage size.
Full-scene parity above is closed; cost timing does not repeat per-frame audio.
User subsequently directs the agent to avoid performance measurements. This
selection is closed/deferred: existing native/Wine startup and native RSS,
whole Wine launch and worldmap timing observations remain local evidence;
battle timing was stopped with task-specific process termination (runner
exit143), with raw completed output retained. No complete battle timing/profile
or overall performance acceptance is claimed. Do not finish or rerun these
measurements until the user asks. See performance-deferred.json in the private
output root. Continue functional migration with completed scene parity above.
