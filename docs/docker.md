# Desktop portable builds with Docker

For the ARM64 Android APK, use
`python3 scripts/android/build_android_docker.py`. It defaults to the maintained
`AndroidPort/` tree, includes SDK/NDK/Java/Rust inside Docker and exports the APK,
checksum and Kotlin unit-test results. See [Android Docker builds](android_docker.md)
for the standalone-port entry point and explicit root-workspace selection.

## Build from Windows / PowerShell

Install and start [Docker Desktop](https://docs.docker.com/desktop/setup/install/windows-install/)
using **Linux containers**. From the checkout in PowerShell:

```powershell
.\scripts\windows\build_docker.ps1
```

The default produces the Windows x86_64 portable ZIP and `BUILD-INFO.txt` in
`dist\docker-windows\`. No host Rust, Visual Studio, MSYS2, GCC or Python
installation is needed. The builder is a Linux container that cross-compiles
`x86_64-pc-windows-gnu` using MinGW; it is not a Windows container. The exported
`.exe` files run on Windows normally, without Docker or Wine.

The same entry point can build Linux from Windows:

```powershell
.\scripts\windows\build_docker.ps1 -Platform linux
.\scripts\windows\build_docker.ps1 -Platform windows -OutputDir 'D:\FFTA builds'
```

The script targets Windows PowerShell 5.1+ and PowerShell 7. If local policy
blocks the script, run it in a single process with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\windows\build_docker.ps1
```

This does not change the user's persistent execution policy. Both platforms
use Docker Buildx's local exporter and the same private-data exclusions.
Build context resolution is relative to the script, not the calling directory;
paths containing spaces are passed as individual native-command arguments.

## Build from Linux / Bash

From the repository root:

```bash
bash scripts/linux/build_linux_docker.sh
```

To build the Windows package from a Linux host instead:

```bash
bash scripts/linux/build_windows_docker.sh
```

The host only needs a working Docker Engine/Desktop with Buildx. Rust, GCC,
Python, pkg-config, ALSA and udev development packages are installed in the
build container. Network access is needed on the first build for the base image,
Debian packages and Cargo dependencies. The script explicitly builds
`linux/amd64`; other host architectures need Docker's amd64 emulation.

The Linux, Windows and coverage Dockerfiles use BuildKit's bundled Dockerfile
frontend, avoiding a separate `docker/dockerfile:1` download. If an older copy
stops at `resolve image config for docker-image://docker.io/docker/dockerfile:1`,
update the Dockerfile used by the launcher. An anonymous-token DNS error at that
step occurs before runtime compilation and does not mean registry credentials
are missing. Uncached base images and dependencies still need working Docker
network/DNS. The [Docker syntax reference](https://docs.docker.com/reference/dockerfile/#syntax)
describes the bundled frontend default; an explicit `BUILDKIT_SYNTAX` override
can select a different frontend.

Output:

- `dist/docker-linux/FFTARecomp-linux-portable.tar.gz`
- `dist/docker-linux/BUILD-INFO.txt` (compiler, OS/libc, linked libraries and hash)

To choose a different export directory, pass it as the sole argument:

```bash
bash scripts/linux/build_linux_docker.sh /tmp/ffta-release
```

Buildx output paths must not contain commas or newlines. The script can be
called from outside the checkout; it resolves the build context from its own
location. The build copies source into the container; it never writes into a
bind-mounted checkout or creates root-owned source/target files on the host.
BuildKit manages Cargo registry/git and target caches. Only the final archive
and build information are exported, using Docker's
[local artifact exporter](https://docs.docker.com/build/building/export/).

On a Docker build-layer cache miss, the Linux and Windows recipes refresh the
copied crate files before Cargo uses the shared target cache. This matters when
switching between source snapshots with older timestamps: Cargo can otherwise
reuse a binary produced from different source content. Unchanged Docker layers
remain cached. The refresh happens inside the image, without touching the host
checkout. The coverage recipe already refreshes its Git archive at extraction.

The Bash Linux builder prints the path of a new log under
`target/build-logs/linux-*.log` on every invocation and saves both normal output
and errors there. When a build fails in an interactive terminal, it waits for
Enter before exiting, so a terminal opened by a file manager keeps the error
visible. Noninteractive runs return the original failure code without waiting;
failed builds do not report a successful export.

## Extract decoded ROM data

Extraction has dedicated entry points that require only Docker on the host:

```bash
bash scripts/linux/extract_rom_data_docker.sh
```

```powershell
.\scripts\windows\extract_rom_data_docker.ps1
```

The ROM argument is optional; both scripts default to
`rom/1194 - Final Fantasy Tactics Advance (E)(Surplus).gba` relative to the
repository root. Override it with a first ROM argument on Bash or `-RomPath` on
PowerShell. Custom relative ROM paths also start at the repository root;
absolute ROM paths are accepted. The output argument is optional; the default is
`<repository>/target/rom-data/<ROM filename stem>/`. Windows uses Docker
Desktop's Linux containers. Both hosts build the same `linux/amd64` `extract`
target from `Dockerfile.linux` into the local `ffta-rom-data-extractor:local`
image, then invoke the source-authenticated offline extractor. Rust, Python and
ALSA/udev build/runtime dependencies remain in that image; there is no host
toolchain, Wine or desktop GUI requirement. The targeted build compiles only
the CLI and does not run portable packaging or its broad test suites. The
existing `build`/`export` release targets retain their test gates.

BuildKit reuses the existing native Cargo caches. The selected ROM never enters
the build context or image; it is bind-mounted as one read-only file at runtime.
The writable mount is the output parent, allowing complete dataset publication
by atomic sibling rename. Linux forwards the caller's UID/GID to avoid root-owned
outputs. Extraction runs with `--network none` and the container is removed
after exit; the build still needs network access for uncached dependencies.
No checkout or Docker socket is mounted into the extraction container.

Both scripts resolve their own build context, accept paths with spaces and
print the host dataset directory. Explicit output paths are relative to the
calling directory; ROM/output paths containing commas or newlines are
rejected because Docker mount arguments use CSV. These bind mounts require a
local daemon with access to the host paths (including Docker Desktop file
sharing). Existing valid datasets are verified/reused; corrupt or incompatible
datasets are preserved and reported as errors. Choose a fresh output directory
to regenerate them. See [decoded files and coverage](rom_data.md).

The extraction target was built and run on Linux for EUR: seven files match the
retained Wine extraction byte-for-byte, reuse changes no hashes/mtimes, and
outputs belong to the host user. Five Bash argument/error tests pass. Six
PowerShell 7.4 cases with a simulated Docker command run inside its Linux
container; native Windows PowerShell 5.1/Docker Desktop remain unverified.

## ROM coverage atlas

The coverage generator also runs entirely through Docker, including Python3.11,
Cargo and `nm`. With Docker Desktop using Linux containers on Windows:

```powershell
.\scripts\windows\rom_coverage_docker.ps1
```

On Linux:

```bash
bash scripts/linux/rom_coverage_docker.sh
```

Both use `Dockerfile.coverage` to generate
`docs/development/rom-coverage/index.html` and `coverage.json`. Git and Docker
are required on the host. A temporary context contains runtime sources archived
from a fixed HEAD commit plus the current public coverage tools/evidence; local
unfinished extraction changes and private inputs stay outside the image. The
normal executable and host Cargo output are untouched. The container compiles
the CLI/helper, reads their source revision and prepares/reuses the matching
`target/rom-data/coverage/<ROM-SHA>/e<extractor>-d<decoder>/` cache. No host Python,
Rust or retained ELF is required. Source mtimes are refreshed inside Docker so
warm Cargo caches cannot mask a changed archived source tree.

The same archive includes committed `scripts/`: Rust embeds public tool scripts
and HTML templates through `include_bytes!`, so they must exist before Cargo
runs. These embedded assets come from the same fixed commit as the runtime.
Current coverage scripts/evidence are overlaid only after compilation; local
edits to embedded tools cannot silently enter the committed executable.
Runtime fixes still present only in the working tree are consequently absent
from this build until committed, including compiler-warning fixes.

The 30 October correction passed eight launcher tests, including committed
embedded-asset bytes despite local edits. The real context captured from the
Bash launcher built both coverage executables successfully in Docker; offline
coverage help and the CLI's 97-operation readonly registry also passed. That
build used commit `75fb1cfe` and retained its two known warnings; the local warning
fix was deliberately excluded. No ROM, dataset or report generation was used.
PowerShell archive roots were checked against Bash; native Windows remains
unverified.

Both coverage launchers use this same Dockerfile and the bundled frontend
described above. Uncached Rust base images, Debian packages and Cargo
dependencies still require working network/DNS during the build.

The map follows newly committed extraction generations automatically. Ordinary
use evidence is inherited only for identical decoded files and source maps;
new/changed families are visible with review pending. The generated HTML shows
its real generation time, dataset generation, source commit and separate evidence
review date. Logs include count deltas or explicit unchanged coverage feedback;
both HTML/JSON are rewritten on every successful run. PowerShell `-OpenReport`
uses a timestamped file URL to request the new page. Failure logs remain under
`target/build-logs/rom-coverage-*.log`, and validation failures preserve the report.

ROM/evidence and explicit datasets/ELF inputs are read-only; only the private
cache and report output are writable. Generation runs offline and Linux
preserves host UID/GID. Override `-RomPath/-DatasetDir/-TranslationPath/-OutputDir/-EvidencePath`
or `--rom/--dataset/--translation/--output/--evidence`. An explicit dataset from
another revision is prominently labeled historical; omit it for current data.
All relative paths start at the repository root. The older direct
`Dockerfile.linux --target coverage` target uses the working tree instead of
these launchers’ committed-source isolation.
See [inputs, interpretation and commands](development/ROM_COVERAGE.md#rigenerazione).

Coverage launchers also accept explicit DBG-06 audit reports: repeat `--audit FILE`
on Bash or pass `-AuditPath @('a.json','b.json')` on PowerShell. `--compare FILE` /
`-ComparePath FILE` selects an older same-ROM coverage.json. These files are
read-only runtime mounts, excluded from the image and build context. No gameplay
replay is performed; no host Python is needed. Without audit arguments the new
report has no scene inventory. See [scene coverage](development/ROM_COVERAGE.md#scene-letture-e-confronto-dbg-07).

## Environment and compatibility

`Dockerfile.windows` copies the pinned Rust 1.94.0 toolchain from the same
image used by the Linux builder into a digest-pinned Debian Trixie build stage.
This reuses the compiler download while providing newer MinGW/Wine packages
for modern Windows APIs. It installs the
Windows Rust standard library and cross-compiles the
wrapper/runtime. TinyCC uses native GCC for its build-time helper and MinGW
for `libtcc.dll`. Resource build scripts check the target OS so Windows icons
are embedded during cross-compilation as well as native compilation.
Input/config, menu and runtime tests execute through 64-bit Wine **inside the
container**, with a disposable prefix. They are not native Windows test runs.
Windows target caches are separate from Linux target caches; Cargo downloads
are shared. See Rust's [Windows GNU target support](https://doc.rust-lang.org/nightly/rustc/platform-support/windows-gnu.html).

`Dockerfile.linux` selects Rust 1.94.0 on Debian Bookworm and installs its native
build dependencies. `Cargo.lock` is enforced for builds and tests. This avoids
linking releases against the developer host's Ubuntu 26.04 libc. The base image is
pinned by digest, including the Rust version and distribution. Debian package repositories can still
receive updates, so this is not a claim of bit-for-bit reproducible builds.
Check the exported build information and the actual ELF requirements when
changing the base image. The earlier direct Ubuntu build remains a separate
artifact at `dist/FFTARecomp-linux-portable.tar.gz`.

The Linux build compiles the portable runtime and bundled TinyCC, runs the
input/config, menu, core, screen and runtime test suites serially (optional
local-ROM and GPU readback tests remain ignored),
and packages only after a successful build; failed tests prevent export.
`.dockerignore` allows only desktop source, assets, docs, licenses and packaging
inputs. ROMs, saves, savestates, recordings, personal `Data`, build outputs and
agent settings are excluded. Do not add a ROM to the build context or image.
TinyCC source and notices accompany the exported native library.

## Run the exported game

On Windows extract `dist/docker-windows/FFTARecomp-windows-portable.zip`,
then launch `FFTARecomp.exe` from the extracted folder. Keep the whole folder
and choose your own ROM on first launch. For Linux:


```bash
mkdir -p dist/docker-linux/game
tar -xzf dist/docker-linux/FFTARecomp-linux-portable.tar.gz -C dist/docker-linux/game
./dist/docker-linux/game/FFTARecomp-portable/FFTARecomp
```

This container is for **compiling and packaging**. The exported game runs on
the host desktop: its graphics stack, libc, ALSA and udev runtime libraries are
still required. Docker does not embed these libraries in the portable archive.
There is no GUI container, display/device passthrough, privileged mode or Docker
socket mount inside the build. Save data stays under the extracted folder's
`Data`, as described in [portable storage](portable.md).

## Cache maintenance

Subsequent builds reuse downloaded crates and compiled dependencies through
[BuildKit cache mounts](https://docs.docker.com/build/cache/optimize/).
The container's `target` cache is separate from the checkout's `target/`.
Manage Docker cache using your normal Docker workflow; the build script does
not prune images, containers, caches or unrelated projects.

## Build warnings

The Debian package-install step explicitly uses `DEBIAN_FRONTEND=noninteractive`
in both Dockerfiles. This avoids attempting Dialog/Readline in a build without
a terminal; the setting applies only to that install command.

Minimal Debian images can still print package-maintenance notices:

- `debconf: delaying package configuration, since apt-utils is not installed`:
  optional package preconfiguration is unavailable; package installation still
  performs its configuration.
- `rehash: warning: skipping ca-certificates.crt`: that file is the combined
  certificate bundle, while rehash expects individual certificates. This line
  alone does not mean certificate installation or HTTPS validation failed.
- `update-alternatives: ... /usr/share/man/... doesn't exist`: the slim image
  omits those manual pages; the notice concerns their documentation links.

Keep installation errors visible. Installing extra documentation packages or
removing the certificate bundle merely to silence these messages is unnecessary.

Project Rust helpers are compiled only for the platforms/tests that call them:
DirectInput normalization on Windows and tests, reference PPU/audio wrappers in
tests, and Gilrs binding helpers on the non-Linux path. Linux's device snapshot
reader/table stay Linux-only while pure snapshot tests remain cross-platform.
The RTC fallback constant is retained off Windows and in tests; Windows reads
local time through GetLocalTime. The unused private FIFO wrapper has been
removed. This cleanup preserves the runtime paths and leaves Rust warnings
enabled.

The supplied Linux log reaches `Finished release` and archive creation, then
ends during test compilation; it does not contain the final test/export result.
After the cleanup, native release build and affected core/input/runtime tests
were checked separately; see the [validation record](changes/2026-10-21-clean-desktop-build-warnings.json)
for actual platform coverage and limits.

## Validation

The container build/export completed with Rust 1.94.0 on Debian Bookworm
(glibc 2.36). 26 input/config and 41 runtime tests passed inside the container;
two optional ROM-fixture runtime cases remained ignored. ELF inspection found
requirements up to glibc 2.35 for the runtime, 2.34 for the wrapper and 2.34 for
TinyCC. This removes the direct Ubuntu build's 2.43 requirement; compatibility
with older Ubuntu/Steam Deck desktops has not been established by a GUI test.

The exported archive ran natively on the Ubuntu 26.04 host. An isolated
Xvfb/Mesa llvmpipe keyboard smoke saved slot 1 with its preview, closed normally,
moved the entire folder, relaunched and loaded the same slot. The translation
cache bytes/mtime were reused, startup backup copies numbered one then two,
and per-user config/cache/data files were unchanged. The ROM was preplaced in
the test folder. Physical controllers and native Wayland interaction were not
tested. No ROM or test data was copied into the container or exported archive.

The current Linux and Windows Docker recipes also gate export on the core and
screen suites, using serial tests because display configuration is process-global.
The first expanded-suite run passed 307 tests on Linux (164 core, 26 input/config,
33 menu, 44 runtime, 40 screen) and 306 under Wine (164 core, 26 input/config,
33 menu, 45 runtime, 38 screen). Eight optional tests were ignored per target.
The ignored panel-bound microbenchmark is a developer diagnostic, not a CI timing
assertion. GPU readback and local-ROM tests need explicit external fixtures and
remain separate checks; container success does not establish native GPU behavior.

### Linux export interruption and retained errors

The 2026-10-22 retained failing build completed compilation and archive creation,
then stopped at `per_family_input_bounds_and_complete_set_limit`. That older
six-file fixture now fits the current aggregate input guard, so its rejection
expectation was stale. The test now accepts it and checks the actual aggregate
boundary and one byte beyond without allocating a full-size dataset. Production
input guards and release test gates are unchanged.

One corrected Linux Docker export passes 461 tests with 67 optional cases ignored,
including the previously failing case. The portable archive and matching
`BUILD-INFO.txt` are exported successfully. Three focused wrapper checks cover
output/log preservation, failure status, paths with spaces/unrelated working
directory, and an interactive terminal that waits for Enter on failure.
This verifies the builder; no new gameplay/GUI or USA validation is claimed.
See the [change record](changes/2026-10-22-fix-linux-builder-limit-test-and-retain-errors.json).

### Windows container validation

The Windows cross-build completed with Rust 1.94.0, MinGW GCC 14 (win32 thread
model), mingw-w64 12 headers and Wine 10 on Debian Trixie. Inside the container,
26 input/config, 33 menu and 42 runtime tests passed; two optional local-ROM
runtime cases remained ignored. PE inspection confirmed x86_64 binaries with
icon, icon-group and version resources in both executables. Imported DLLs did
not include external GCC, libstdc++ or winpthread runtime libraries. ZIP CRC,
portable marker, TinyCC source/notices and private-data exclusion checks passed.

On the Ubuntu host the exported wrapper ran through Wine 10 with simulated
XInput. It saved a preview slot, closed normally, moved the folder, relaunched
and loaded the slot. Cache bytes/mtime were reused, backup copies counted one
then two, and installed AppData files remained unchanged. The ROM was supplied
only to this isolated host test, never to Docker or the release archive.
The shared Linux Docker workflow also passed its 67 tests after the build-script
and TinyCC helper changes.

The PowerShell entry point was parsed and exercised using PowerShell 7.4.6 on
Linux with a simulated Docker CLI: default Windows and mixed-case Linux target,
checkout/export paths containing spaces, unrelated working directory, invalid
arguments, missing Docker and nonzero exit propagation. Native Windows/Docker
Desktop, Windows PowerShell 5.1 and physical controllers were not available
for validation; do not treat Wine or the PowerShell mock as native Windows
coverage.

## Local generated files

Follow [local artifact retention](development/LOCAL_ARTIFACTS.md) after successful
managed/native validation: retire interrupted/moved test copies with
`python3 scripts/local_artifacts.py complete target/<unit> --apply`, preserving
accepted offline datasets and result/log evidence. The coverage launchers also
retain only the current and newest other automatic cache generation after
successful publication; explicit datasets remain untouched. This bounds local
test leftovers without changing production resource retention.

`target/` also holds local validation builds, Wine test prefixes and repeated
extraction/lifecycle fixtures. Completed tests can leave `interrupted`,
`offline dataset`, `moved resources` and CLI `resources` directories; those names
alone do not mean that an extraction is still running. Check
[the task handoff](development/TASK_HANDOFF.md) and current dataset references
before removing obsolete copies. Keep pending validation inputs, ROMs, saves,
recordings, current runtimes and the latest verified portable archives.

Local cleanup does not change the runtime's immutable-generation retention
policy. Logs, result summaries and tracked change records retain completed
validation evidence after old generated copies are removed; missing local
artifacts do not reopen completed tests. See the
[resume/repeat workflow](development/MEMORY_WORKFLOW.md#resume-and-repeat-decisions).

## Decoded-resource archive gate

Both container context/COPY lists include docs/rom_data.md and
rom_data_runtime.md alongside the existing selected portable guides. The
packager requires them; adding a required document to packaging also requires
updating the Docker allowlist and COPY inputs. Keep the explicit privacy
exclusions for game data and runtime storage.

ROMDATA-05 candidate exports pass395Linux and392Windows/Wine tests,24optional
ignored each, archive privacy/identity and PE/ELF requirements (Linux GLIBC2.35
maximum). Their actual extracted runtimes automatically find bundled TinyCC,
create portable decoded resources and pass GUI Slot Load/Undo/reload/close.
The resource route remains opt-in; USA initializer/native Windows gates are
separate. A prior Linux BUILD-INFO metadata hash variation, with unchanged
archive/canonical bytes, is retained rather than claimed preserved.
[Detailed scope and evidence](development/ROM_DATA_VALIDATION.md#actual-portable-archives).


## Offline artifact manager

[DBG-18](debug_artifacts.md) provides reviewed preview/apply and logged Bash/
PowerShell launchers. Dockerfile.debug-tools derives from the existing public
coverage image and adds Git for tracked-file guards; it performs no Cargo or
game-data build/copy. Initial image preparation needs Debian package access;
tool invocations use network none. The selected checkout bind is writable for
leases and explicitly reviewed cleanup; dataset reader mounts stay readonly
with a separate cache coordination root where needed. Host Python and Docker
plans use different absolute namespaces: use one launcher for preview/apply and
regenerate after moving folders. Native Windows/PowerShell remains unverified.


## Public debug tools in portable packages

Both desktop build sources now copy the explicit public tool/doc dependency
set required by the portable packager. Docker context allowlists include those
files only; no session/recording/result payloads are added. The package manifest
verifies staged bytes and preserves empty Data. This source/COPY closure is
checked by public packaging tests; the DBG-22 validation does not rebuild the
unrelated full Rust Docker images.

## Build method verification (30 October 2026)

Current native desktop entry points are `scripts/linux/package_ffta_linux.sh`
and `build-all.ps1 -SkipLinux -SkipAndroid` on Windows. Both use
`scripts/build_desktop.py` to follow Cargo's actual executable paths rather than
assuming `target/release`. The helper checks the Windows/Linux x86_64 headers,
stages TinyCC beside those binaries and passes that directory to the portable
packager. Rust, the selected target's linker, Python3.11+ and a host C compiler
are native-build prerequisites; Linux additionally needs pkg-config and the
ALSA/udev development headers. Docker supplies its own build dependencies.

The audit passed 21 distinct tooling tests: five synthetic Cargo/TinyCC routing
cases, eleven portable-package checks and five Docker-wrapper cases. They cover
custom target directories, target subdirectories, Unicode/spaces, PE/ELF x86_64
selection, compiler failures, old-package preservation, public docs/notices,
private-file exclusion and unsafe CSV output paths. The Docker contexts contain
the native helper and the Atlas/extended-save guides required by the packager.

A tiny standalone Rust fixture in a cached Docker image reproduced an old
binary after replacing its source with an older timestamp. Running the recipes'
crate-file refresh caused Cargo to produce the new value. No FFTA code or ROM
was used. Actual Buildx stage parsing passed for Linux, Windows and coverage;
the earlier full coverage-image compilation/startup proof remains valid.

The Linux CI workflow now uploads the portable archive actually produced in
`dist/`, refuses missing artifacts, uses locked serial tests and selects
Python3.11 through [setup-python](https://github.com/actions/setup-python).
Its YAML and producer/upload path agreement were checked locally; the hosted
workflow itself was not run.

PowerShell was reviewed statically because a local interpreter was unavailable.
Native Windows/PowerShell, the complete new native/package and Docker export
matrix, Windows-to-Linux installer execution and Android SDK/NDK builds remain
to be verified. Full application/GUI suites and benchmarks were not repeated.
`build-all.ps1` retains Linux/Android steps when their skip switches are absent;
`AndroidPort/` is a separate tree. The generic upstream
`package-release.sh`/Unix TinyCC helpers package the toolkit/launcher and are
separate from the current FFTA portable entry points above.

Offline homepage/artifact/campaign launchers reuse already available images
without network. The [common guide](debug_tools.md) lists prerequisites and
Windows/Wine/Linux scopes. Current-package verification builds an archive with
the existing offline-tools image, checkout mounted readonly and a separate
output mount; it executes no game or Cargo build.

Host distribution builds should remap compiler source paths before packaging:
Rust --remap-path-prefix for checkout/home/cache roots, and TinyCC/build.py
maps its source and adapter roots with -ffile-prefix-map. The shared packager
accepts repeatable --private-prefix PREFIX to reject embedded UTF-8/UTF-16LE
private paths before replacing the previous archive. It never rewrites binaries;
the manifest records only the number of prefix checks. DBG-22 uses this guard
with its isolated remapped Windows binaries.
