# FFTARecomp Android

This is a self-contained Android port. You can move or upload **this entire
folder** without copying anything from FFTARecomp - Copy or the PC project.
The README stays at the top of this folder so GitHub displays it.

- Preview APK: `APK/FFTARecomp-android-arm64-preview.apk`.
- Android UI / Android Studio project: `android/`.
- Required Rust engine and recompiler: `crates/`.
- Build and camera-test scripts: `scripts/`.
- Credits and licenses: `Licenses/`.
- No ROMs, saves, input recordings, SDK, NDK or caches are included.

<a id="compilar-desde-windows"></a>

## Build on Windows

You can also build with **Docker/Buildx and Python 3.9+**, without installing
an SDK, NDK, Java or Rust on the host. From this folder:

    python scripts/build_android_docker.py

This exports the ARM64 APK, SHA256 and test results to `dist/docker-android/`.
The recipe and script live inside this folder and continue to work when it is
copied separately. On Windows, Docker Desktop must use Linux containers.
Logs are saved in `target/build-logs/`; the first run downloads the tools.
The debug signing key is retained in the Docker cache; keys from previous
builds made outside Docker are not imported.

For a native build, you need Rust with `aarch64-linux-android`, Python 3.9 or
later, a JDK, Android SDK with NDK 28.2.13676358, and host GCC or Clang on PATH.

From this folder in PowerShell:

    $env:JAVA_HOME = 'C:\path\to\jdk'
    $env:GRADLE_USER_HOME = 'C:\Users\you\.gradle'
    python scripts/build_android.py --sdk D:\Android\SDK

Adjust these paths for your computer. The output is written to `APK/`.
The script downloads and verifies TinyCC, builds ARM64 libraries, includes
the required notices and source, builds the APK and runs the Kotlin tests.
The first build needs available dependencies or a network connection.

<a id="subir-manualmente-a-github"></a>

## Upload manually to GitHub

You can upload this entire folder. The APK is included for convenience:
if you do not want to upload binaries, remove only the `.apk` file in `APK/`
and `APK/SHA256.txt`. Building generates the APK again.
Files ignored by Git are still uploaded when dragged into the web interface:
before a later upload, remove generated `target/`, `build/`, `.gradle/`
and `local.properties` files. Keep `gradle/wrapper/`.

The PC project has not been moved or replaced. This is a self-contained copy
of the current Android sources, prepared to be moved independently.
Later changes in the PC project are not synchronized automatically.

<a id="estado-de-la-prueba-rg-rotate"></a>

## RG Rotate test status

Android 12, T618, 720x720. Camera sweeps at zoom 6 produced actual screen
samples around 55–60 FPS, with occasional drops; this is not a fixed 60 FPS.
Audio stability on this device still needs improvement: underruns were
recorded during the benchmark. Do not describe this test as perfect performance.

Pixel-by-pixel equivalence, route framebuffer/OAM hashes and the 600-frame
comparison without extra area passed. The Android integrated-controller test
passed using injected events with its real device identity; this does not
replace a manual test of the physical switches.

To repeat the route, use `scripts/ffta_android_camera_bench.py --help`.
It requires your local input recording and a copy of your save; neither is included.

<a id="optimizacion-en-curso"></a>

## Optimization in progress

New work continues only inside this folder. Status and evidence are in
[docs/PERFORMANCE.md](docs/PERFORMANCE.md). Performance and uninterrupted audio
are not claimed complete until measured on the target device.

Character fixes for vertical zoom and Normal fast-forward audio:
[docs/URGENT_FIXES.md](docs/URGENT_FIXES.md).

Desktop engine updates, UI independent of zoom and experimental touch
selection (off by default, Input > Touchscreen):
[docs/EXPERIMENTAL_TOUCH.md](docs/EXPERIMENTAL_TOUCH.md).

Dialogue-bubble optimization, RG Rotate measurements and tests:
[docs/DIALOGUE_PERFORMANCE.md](docs/DIALOGUE_PERFORMANCE.md).

Later engine optimization, core selection and complete measurements at
zoom 2, 3 and 6: [docs/FRAME_OPTIMIZATION.md](docs/FRAME_OPTIMIZATION.md).

Later dialogue optimization: row reuse and clipping by game windows,
with A/B tests: [docs/DIALOGUE_ROWS.md](docs/DIALOGUE_ROWS.md).

Investigation of occasional spikes and frame delivery to OpenGL:
[docs/FRAME_SPIKES.md](docs/FRAME_SPIKES.md).
