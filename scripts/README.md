# Project scripts

Platform-specific entry points live in these folders:

- `linux/`: Linux Docker build/extraction entry points and the native Linux
  packager. `build_windows_docker.sh` builds the Windows package from a Linux host.
- `windows/`: PowerShell entry points, the Windows portable-packager wrapper,
  MSYS2 TinyCC builder and Windows-to-Linux cross-build helper with its C linker.
- `android/`: Android APK builders and Android-device camera helper. Run
  `python3 scripts/android/build_android_docker.py` for the maintained AndroidPort
  with Docker/Buildx and Python only; `--project-dir .` selects the root tree.
  The direct builder supports Windows/Linux SDK/NDK toolchains.
- `unix/`: TinyCC builder shared by Linux and macOS.
- `tests/`: shared Python tooling tests.

Cross-platform extraction, packaging, release, analysis and project-memory
tools stay directly in `scripts/`. For example,
`benchmark_windows_runtime.py` supports native and Wine runtimes despite its
historical filename; `package-release.sh` runs on all three CI host platforms.
The private debug-assets exporter (`export_debug_assets.py`, with Bash and
PowerShell wrappers) creates a separate, local Windows portable ZIP with the
selected debug collection. Its input selection, privacy and prerequisites are
documented in [private debug asset export](../docs/private_debug_assets.md).

The root `build-all.ps1` remains the Windows all-platform build entry point.
Use `-SkipLinux -SkipAndroid` for a Windows desktop package only; omitting only
Android retains the Windows-to-Linux installer step. Native desktop wrappers
share `build_desktop.py`, which follows Cargo's reported executable paths and
stages TinyCC in that same directory before packaging.
It launches the Linux cross-build helper with the current PowerShell executable,
so PowerShell 7 does not try to find the Windows PowerShell `powershell.exe`
inside its own installation directory.
`AndroidPort/` has its own scripts and is managed separately.
Its Docker recipe/driver remain self-contained when that directory is copied.
See [Android Docker builds](../docs/android_docker.md) for APK output, logs,
toolchain versions and the distinction between the two source trees.

The runtime's [session diagnosis](../docs/debug_diagnostics.md) is available as
`recomp diagnose` and Debug > Copy diagnosis; it needs no Python installation.
[Isolated debug sessions](../docs/debug_sessions.md) use `recomp debug-session`
and Debug > Start debug session to work on portable copies of the selected case.
[The save catalog](../docs/debug_catalog.md) uses `recomp debug-catalog list/open/import`
and Debug > Debug catalog for shared filters, integrity checks, previews and
opening a DS checkpoint in an isolated copy. These runtime commands need no Python.
[Snapshot input recordings](../docs/debug_recordings.md) use Debug > Start/Stop
input recording and `recomp debug-record start/verify`: capture relative input
from a private snapshot and compare a bounded interpreter replay's full endpoint.
[Replay profiles](../docs/debug_replays.md) use `recomp debug-replay create/run/cancel`
and Debug > Replay case. The same isolated worker reports progress and distinct
success/divergence/incompatible/timeout/interruption outcomes, without performance
measurement or game-save writes; an existing native library is optional.
[ROM read audits](../docs/debug_rom_audit.md) use `debug-rom-audit create/run/cancel`
and Debug > Audit replay, with an explicit `rom-data-read-audit` worker feature.
They add bounded events, frame/address/reader filters, actual typed/raw service
and physical-byte unions without re-extraction or automatic coverage promotion.

The [completed debug-tools checklist](../docs/development/DEBUG_TOOLS_TODO.md)
records the DBG01–22 scope. The new [integrated-debug TODO](../docs/development/DEBUG_TOOLS_V2_TODO.md)
plans in-menu content/script selection, event and command consoles, and a cheat
debug menu. Implementation starts with the readonly
[operation contract](../docs/debug_operations.md), available as
`recomp debug-operations list/show [--json]`. It inventories existing commands,
parameters and remaining menu gaps; it does not execute the listed operations.
The readonly [content catalog](../docs/debug_content.md), `debug-content scan/check`,
indexes only declared app roots and explicitly added folders, with stable IDs,
hashes, availability, declared compatibility and bounded refresh; no content is
executed. The [integrated content selector](../docs/debug_selector.md),
Debug > Content selector, adds explicit search/filter/select/refresh/import
controls over the same readonly catalog. Typed tool inputs and parameter drafts
are implemented by DBG2-04/05. Scripts and tools adds explicit declared adapters
for homepage/campaign report/artifact preview (selected Python3.11+), readonly
command recipes and a separate guest catalog/inspector link. It stages exact
public source hashes on Start, retains bounded logs and supports cancellation;
PowerShell/Bash/Docker wrappers remain separate explicit offline entry points.
DBG2-07 saves guarded relative/internal and explicit/external choices plus
selector views/history; a moved portable copy restores compatible pins and
explains missing/changed ones without launching content.
DBG2 development remains distinct from DS scene acceptance.

From the repository root, extract readable ROM data with Docker:

```bash
bash scripts/linux/extract_rom_data_docker.sh
```

```powershell
.\scripts\windows\extract_rom_data_docker.ps1
```

For portable builds:

```bash
bash scripts/linux/build_linux_docker.sh
bash scripts/linux/build_windows_docker.sh
```

```powershell
.\scripts\windows\build_docker.ps1
```

Both extraction entry points use
`rom/1194 - Final Fantasy Tactics Advance (E)(Surplus).gba` relative to the
repository root by default. Override it with a first ROM argument on Bash or
`-RomPath` on PowerShell; absolute ROM paths are accepted and relative ROM paths
always start at the repository root, even when launched from another directory.

Scripts resolve their repository from their own location. Explicit output
paths remain relative to the calling directory. See
[Docker workflows](../docs/docker.md) and [extraction options](../docs/rom_data.md).
The EUR extractor also publishes `word-task-obj-animations.json` in e77-d76:
four source-authenticated banks for u16 task counters, with readable OBJ lists,
relative indices, frame selectors and durations. See the [managed runtime
contract](../docs/rom_data_obj.md#managed-runtime-for-the-u16-task-obj-banks)
for validation, actual-CLI evidence and the source-backed ordinary Bank check. USA remains e8-d7.
EUR e78-d77 additionally publishes `world-nodes.json`: typed coordinates,
ordered path/neighbor links and losslessly preserved opaque fields. The
[world-node runtime contract](../docs/rom_data_maps.md#managed-world-nodes-and-typed-guest-reads)
records the reduced Wine integration checks and actual CLI create/reuse/normal
coordinate use. The subsequent ordinary travel360check passes coordinates,
paths and neighbors with complete state/clock/image/audio equivalence. Other
nodes/maps, opaque ordinary use and new Linux/USA checks remain pending.


The Linux Docker builder saves each run's output and errors under
`target/build-logs/linux-*.log`. On failure it waits for Enter in an interactive
terminal; noninteractive automation receives the original exit code immediately.

After recording successful managed/native validation, retire disposable test
copies with `python3 scripts/local_artifacts.py complete target/<unit> --apply`.
Preview without `--apply`; retain the accepted offline dataset and proof logs.
See [local artifact retention](../docs/development/LOCAL_ARTIFACTS.md) for safety
checks, optional superseded-binary retirement and coverage cache limits.


EUR e79-d78 extraction additionally publishes `world-roads.json`
(`gameplay.world-roads`), covering45conditional8-byte headers and2,188signed
XY samples through46source spans. Schema1 and USAe8-d7 stay unchanged. Runtime
preserves guest movement and bytewise old-owner/raw fallback; the reduced Wine
integration proof is recorded in [managed world roads](../docs/rom_data_maps.md#managed-world-roads-and-typed-guest-reads).
Production CLI and ordinary-scene proof remain next; Linux/USA tests pending.


Actual e79-d78CLI creation/reuse/startup and one Giza360ordinary road pair now
pass on Wine, with72files/71resources/74managedpaths unchanged and full states/
clock/images/OAM/dispatch/fiveaudio. All seven road counts match CLI, including
positive count/pointer/sample bytes. See [the scoped result](../docs/rom_data_maps.md#ordinary-sampled-road-travel-equivalence).
Linux/USA/nativeWindows and broader coverage remain pending; no measurements.

ROM coverage launchers accept repeatable DBG-06 audits (`--audit FILE` on Bash,
`-AuditPath @('a.json','b.json')` on PowerShell) and a previous snapshot
(`--compare FILE` / `-ComparePath FILE`). They import only local metadata through
read-only mounts, without replay. The map distinguishes observed unions,
Typed/Raw service and entire positive families, with explicit filters and limits.
See [instructions and examples](../docs/development/ROM_COVERAGE.md#scene-letture-e-confronto-dbg-07).


EUR e80-d79 extraction publishes `encounter-headers.json`
(`gameplay.encounter-headers`) with a constructor-alternate dependency. Its two
fragments migrate17,676bytes while retaining existing ownership of the shared
four-byte pointer. The reduced Wine integration result is documented under
[managed encounter headers](../docs/rom_data_maps.md#managed-encounter-headers-and-typed-guest-reads).
Actual CLI/ordinary coverage and Linux/USA/nativeWindows remain pending; no
performance measurement is included.

Runtime command `debug-capture create/run/cancel` and Debug > Capture on trigger
produce points derived from isolated replays: first resource read, interpreter
entry at a verified ROM PC, or N completed frames. They require
`rom-data-read-audit`, recorded input and an explicit limit; the result is a
private bundle/catalog or `not-reached` without a snapshot. See
[usage, provenance and limits](../docs/debug_captures.md).


EUR e81-d80 extraction adds `encounter-unit-templates.json`
(`gameplay.encounter-unit-templates`) with five explicit header/constructor/item/
metadata14/job dependencies. Two source fragments skip the192 bytes already
owned by constructor alternates; strict coherence and typed reads preserve
original live initialization. [Contract and checks](../docs/rom_data_gameplay.md#managed-encounter-unit-templates-and-typed-guest-reads).

`recomp debug-compare create/run/cancel` and Debug > Compare first difference
compare every frame from the same snapshot/input: interpreter/raw as reference,
and interpreter/native with optional dataset as candidate. The first differing
boundary produces a private HTML/JSON report with exact CPU/Bus fields,
side-by-side images and streams; an equal prefix retains only metadata.
The audit feature is not required. See [usage, scope and checks](../docs/debug_comparisons.md).

`recomp debug-control status/pause/step/resume SESSION` controls an isolated
copy's window. `step SESSION N [--keys HEX]` advances exactly 1–600 frames
and remains paused; the menu and F6/F7/F8 share the same actions.
`debug-session open SESSION --paused` prevents frames before the first request.
See [DBG-10: procedure, audio and limits](../docs/debug_control.md).


EUR e82-d81 extraction adds `encounter-coordinates.json`
(`gameplay.encounter-coordinates`) with header pointer/count/role coherence.
Its691 spans retain raw XY bytes and original signed/unsigned placement/RNG
logic. [Managed resource and checks](../docs/rom_data_maps.md#managed-encounter-coordinates-and-typed-guest-reads).

`recomp debug-control inspect SESSION ADDRESS LENGTH` reads buffers and registers
without performing I/O accesses. `break SESSION PC arm|thumb` and `watch SESSION
read|write START END` search for a point in the interpreter with step/frame limits;
watch requires `rom-data-read-audit`. See [DBG-11 and granularity](../docs/debug_probes.md).

`recomp debug-control graphics SESSION [--compare-last]` captures a
[DBG-12 graphics report](../docs/debug_graphics.md) from the paused copy, with
private PNG/JSON/PPU buffers, readonly projections and comparison to the latest
report. It requires no audit and creates no new snapshot.

`report_home.py` generates root `index.html` with all local HTML indexes,
search and filters. `windows/report_home.ps1 -OpenReport` and
`bash scripts/linux/report_home.sh` use the existing coverage Docker image,
with networking disabled and readonly checkout; no extraction or runtime.
Only the generated catalog is ignored. [Usage and checks](../docs/report_home.md).

`recomp debug-control ui-start SESSION` / `ui-report SESSION` and Debug entries
Start UI / input trace / Save UI / input report collect bounded observations of
focus, fonts, input/remapping and host/guest dispatch in the isolated copy.
`status.host_menu` exposes actual focus before automating navigation.
Private reports are recognized by the homepage. [DBG-13](../docs/debug_ui.md).

`recomp debug-control audio-start SESSION [--frames 1..180] [--streams LIST]` /
`audio-report SESSION` and Start audio capture / Save audio report collect guest
buffers and callbacks with mute/Fast/reset context in the isolated copy.
`debug-audio states --rom ROM --left STATE --right STATE --dir NEW` compares
retained buffers without running the game; exits 0 equal / 2 diverged / 3 incompatible.
Private WAV/raw/reports are indexed by the homepage. [DBG-14](../docs/debug_audio.md).

`recomp debug-control code-start SESSION` / `code-report SESSION` observe a
PC/mode prefix, the current instruction, static labels and
native/missing/system/hook/interpreter dispatch in the isolated copy. Filters
and limits are explicit; no new snapshot or code percentage inferred from counts.
[DBG-15](../docs/debug_code.md).

`recomp debug-report create PROFILE/replay.json --symptom TEXT` and the
Create issue report menu entry gather context, input/identity and retained
evidence in local HTML/JSON with relative launchers, without starting the case
or sending data. `debug-control issue-report SESSION` is the equivalent for an
open window. [DBG-16 and privacy](../docs/debug_issues.md).

`recomp debug-control gameplay-start SESSION [--frames N --events N --family FAMILY --context HEX]` /
`gameplay-report SESSION` and Start task / script trace / Save gameplay report
observe verified task/script/unit fields with RAM/ROM/register provenance,
explicit filters and limits. Resource links and unknown fields remain distinct
from persistent actor identities. [DBG-17](../docs/debug_gameplay.md).

`debug_artifacts.py preview --session DIR --keep DIR --unit target/rom-data-NAME --out NEW`
produces an HTML/JSON plan; `apply PLAN --receipt NEW.json` verifies it under
coordination and records each removal. Inputs/reports/errors and retained
sessions are preserved. Launchers `windows/debug_artifacts.ps1` and
`linux/debug_artifacts.sh [--docker]` retain logs. Docker uses the derived
`Dockerfile.debug-tools` with Git, without a runtime build. [DBG-18](../docs/debug_artifacts.md).

**recomp debug-control recent-start SESSION** / **recent-freeze SESSION** /
**recent-report SESSION** and Start recent events / Freeze / save recent events
retain a rolling 1,024-event/256-KiB window. An observed-error trigger saves
useful context; read availability, evictions and limits remain explicit.
No new snapshot. [DBG-19](../docs/debug_recent.md).

**recomp debug-control status SESSION** also works while a command waits.
**cancel SESSION --request ID** cancels that budget at the next frontend return;
**--timeout-ms N** (20–120000) adds bounded waiting and a server deadline.
JSON distinguishes unconfirmed timeout, server state and manifest/run/ID identity.
No network service. [DBG-20](../docs/debug_control.md#richieste-cancellazione-e-scadenza--dbg-20).

**debug_campaign.py report MANIFEST --case ID --out NEW** selects DS/families/
transitions and reuses pinned JSON evidence without automatic replay.
Positive/negative/incompatible/not-run outcomes and zero reads in the observed
window remain explicit. **run-case MANIFEST ID --reason TEXT --out NEW** delegates
one profile to the existing replay; previous evidence requires --reopen-reason.
Launchers **windows/debug_campaign.ps1** and **linux/debug_campaign.sh [--docker]**
retain logs; Docker provides report/help only. [DBG-21](../docs/debug_campaigns.md).


The [shared debug guide](../docs/debug_tools.md) covers recording, bug reports,
runtime comparison, uncovered ROM reads and retention from a moved portable
bundle. **windows/debug_runtime.ps1** and **linux/debug_runtime.sh** preserve
command logs in Data/config/debug-logs and select the adjacent runtime.
Offline launcher logs remain in target/build-logs; home launchers accept
-Python / --python, retain the old page on failure and hide absent bundle docs.
PowerShell artifact/campaign -Docker comes first; --out and other tool flags
are forwarded literally. Public checks: **tests/test_package_ffta_portable.py**
and **tests/test_debug_windows_launchers.ps1** (Linux PowerShell container,
public mocked applications only; see its prerequisites).

The portable packager selects public docs/tools/notices explicitly and writes
PACKAGE-MANIFEST.json; no existing Data, ROM/save/input/dump/cache/test files are
copied. Shell execute permissions survive the Linux tar. Both Docker source
stages include the same module/template/doc closure.

Host distribution builds should remap compiler source paths before packaging:
Rust --remap-path-prefix for checkout/home/cache roots, and TinyCC/build.py
maps its source and adapter roots with -ffile-prefix-map. The shared packager
accepts repeatable --private-prefix PREFIX to reject embedded UTF-8/UTF-16LE
private paths before replacing the previous archive. It never rewrites binaries;
the manifest records only the number of prefix checks. DBG-22 uses this guard
with its isolated remapped Windows binaries.
