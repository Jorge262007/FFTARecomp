# Desktop portable builds with Docker

## Build from Windows / PowerShell

Install and start [Docker Desktop](https://docs.docker.com/desktop/setup/install/windows-install/)
using **Linux containers**. From the checkout in PowerShell:

```powershell
.\scripts\build_docker.ps1
```

The default produces the Windows x86_64 portable ZIP and `BUILD-INFO.txt` in
`dist\docker-windows\`. No host Rust, Visual Studio, MSYS2, GCC or Python
installation is needed. The builder is a Linux container that cross-compiles
`x86_64-pc-windows-gnu` using MinGW; it is not a Windows container. The exported
`.exe` files run on Windows normally, without Docker or Wine.

The same entry point can build Linux from Windows:

```powershell
.\scripts\build_docker.ps1 -Platform linux
.\scripts\build_docker.ps1 -Platform windows -OutputDir 'D:\FFTA builds'
```

The script targets Windows PowerShell 5.1+ and PowerShell 7. If local policy
blocks the script, run it in a single process with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build_docker.ps1
```

This does not change the user's persistent execution policy. Both platforms
use Docker Buildx's local exporter and the same private-data exclusions.
Build context resolution is relative to the script, not the calling directory;
paths containing spaces are passed as individual native-command arguments.

## Build from Linux / Bash

From the repository root:

```bash
bash scripts/build_linux_docker.sh
```

To build the Windows package from a Linux host instead:

```bash
bash scripts/build_windows_docker.sh
```

The host only needs a working Docker Engine/Desktop with Buildx. Rust, GCC,
Python, pkg-config, ALSA and udev development packages are installed in the
build container. Network access is needed on the first build for the base image,
Debian packages and Cargo dependencies. The script explicitly builds
`linux/amd64`; other host architectures need Docker's amd64 emulation.

Output:

- `dist/docker-linux/FFTARecomp-linux-portable.tar.gz`
- `dist/docker-linux/BUILD-INFO.txt` (compiler, OS/libc, linked libraries and hash)

To choose a different export directory, pass it as the sole argument:

```bash
bash scripts/build_linux_docker.sh /tmp/ffta-release
```

Buildx output paths must not contain commas or newlines. The script can be
called from outside the checkout; it resolves the build context from its own
location. The build copies source into the container; it never writes into a
bind-mounted checkout or creates root-owned source/target files on the host.
BuildKit manages Cargo registry/git and target caches. Only the final archive
and build information are exported, using Docker's
[local artifact exporter](https://docs.docker.com/build/building/export/).

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
