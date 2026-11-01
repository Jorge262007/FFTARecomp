<a id="compilare-android-con-docker"></a>

# Build Android with Docker

The host needs **Docker with Buildx** and **Python 3.9+**. On Windows,
Docker Desktop must use Linux containers. Rust, Java, the SDK and the NDK
do not need to be installed on the host; no ROM is needed to build the APK.

From the repository root:

```sh
python3 scripts/android/build_android_docker.py
```

In PowerShell, the same command can use `python` instead of `python3`.
The default source tree is the maintained **`AndroidPort/`**. The output is:

```text
dist/docker-android/FFTARecomp-android-arm64-preview.apk
dist/docker-android/SHA256.txt
dist/docker-android/BUILD-INFO.txt
dist/docker-android/BUILD-SOURCE.json
dist/docker-android/tests/TEST-*.xml
```

The APK is **ARM64 and debug-signed**, with the Rust and TinyCC libraries.
Rust code is compiled in release mode. The build runs `:app:assembleDebug`
and `:app:testDebugUnitTest`; export requires both ARM64 libraries, the
expected TinyCC source/notices and the Kotlin test results. The exported
APK checksum is checked again on the host.

You can select an output directory explicitly, including a path with spaces:

```sh
python3 scripts/android/build_android_docker.py --output "dist/Android APK"
```

<a id="alberi-separati"></a>

## Separate source trees

`AndroidPort/` and the main workspace have different sources and versions.
The script does not synchronize them or replace APKs already in `APK/`
or `Installer/Android/`. To build the main workspace's Android app explicitly,
run this from the repository root:

```sh
python3 scripts/android/build_android_docker.py --project-dir . --output dist/docker-android-root
```

The port remains self-contained. After copying only `AndroidPort/`, you can
run the following from that folder:

```sh
python3 scripts/build_android_docker.py
```

The default output is then `dist/docker-android/` inside the port.
`--dry-run` checks the inputs and shows the command without calling Docker.

<a id="toolchain-cache-e-firma"></a>

## Toolchain, caches and signing

`AndroidPort/Dockerfile.android` uses digest-pinned Rust 1.94.0 and
Eclipse Temurin Java 25 images, Android SDK 35, build-tools 35.0.0,
NDK 28.2.13676358 and the project's Gradle 9.5.0 wrapper. The native target is
`aarch64-linux-android`, with minimum API 24. The Android command-line tools
and TinyCC source are verified by SHA256; the wrapper verifies its Gradle
download. Tool versions and source/APK hashes are recorded in the exported files.

The first build downloads images and dependencies. BuildKit caches retain
Cargo, Gradle, compilation outputs and the **debug key generated inside Docker**.
The port and main workspace use separate caches. This key supports updates
between successive APKs produced by the same builder/cache; keys from earlier
native builds are not imported. Removing the signing cache changes the key
used by subsequent builds. Personal signing keys are not read.

The base images are pinned, but package repositories and downloadable
dependencies remain external services: this is not a promise of byte-identical
APKs across different machines or caches. The build uses the actual selected
sources, including local edits, rather than an older archived commit.

The launcher prepares a temporary context containing only allowed public
build inputs. It excludes ROMs, saves, recordings, generated binaries,
`local.properties`, caches, hidden files and symbolic links. It does not
bind-mount the checkout writable into the container. SDK and native build
intermediates remain inside Docker; only final artifacts are exported.

<a id="log-e-verifiche"></a>

## Logs and validation

The main launcher retains builder stdout/stderr in
`target/build-logs/android-*.log`, including on Docker failure, and returns
the error code. The self-contained entry point uses the port's
`target/build-logs/`. Selected source paths and hashes remain in the log.
If Docker fails, the public context is retained beside the log in an
`android-…-context` directory, so the exact inputs can be inspected.
Temporary contexts from successful builds are removed.

Launcher checks cover context filtering, tree selection, paths with spaces,
errors and checksums. The existing builders' adapters separately check the
Windows/Linux NDK conventions without changing game execution. Compilation
and Kotlin tests do not establish physical-device, controller, audio or
performance results.

Validation on 2026-11-01: both complete Docker builds passed; **24 Kotlin tests**
passed for AndroidPort and **12** for the main workspace, with no errors or
skipped cases. The **7 launcher/adapter/export tests** passed, including the
port copied outside the repository. APK hashes, ARM64 libraries, the TinyCC
source archive and JUnit results were verified. Docker Desktop/Windows and
execution on a physical device remain separate checks.

Toolchain references: [Android command-line tools](https://developer.android.com/studio#command-line-tools-only)
and [Java/Gradle compatibility](https://docs.gradle.org/current/userguide/compatibility.html).
