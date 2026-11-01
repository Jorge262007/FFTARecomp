<a id="estado-de-rendimiento-android"></a>

# Android performance status

Working tree: AndroidPort. The PC project and installer were left unchanged
in this recorded iteration. No commits were made. FFTARecomp - Copy was not used.

<a id="prueba-reproducible"></a>

## Reproducible test

RG Rotate, Android 12, ARM64, T618 SoC, 720x720 display.
Full replay from frame 0 through 13100; zoom 6, 512x512 output.
The battle-1 startup is retained, with straight sweeps added along all
four edges. Movement is measured only when HOFS/VOFS changes:
610 moving-camera frames starting at frame 9700.
benchmark.sav is a private copy; the original save is not modified.

PowerShell example from this directory, adjusting local paths:

    python scripts/ffta_android_camera_bench.py --adb C:/platform-tools/adb.exe --serial ROTATE01523130 --input E:/AI/ARC/gba-recomp/ffta-batalla-1.input --save E:/AI/ARC/ffta-respaldo.sav --out target/camera-test
    python scripts/ffta_android_bench_summary.py target/camera-test

To compare runs, pass two directories to the summary script. Check the four
framebuffer/OAM hashes and the complete trajectory. CPU timings are not
presentation FPS. Do not compare runs with simpleperf enabled against
measurements without a profiler. Keep cold and warm caches separate.

<a id="cambios-de-esta-iteracion"></a>

## Changes in this iteration

- Reuse PPU buffers for each zoom thread; clear only the active width,
  preserving per-row clearing and transparency.
- Return from pump_audio before extracting the mixers when no sample is
  due yet. The audio clock is unchanged.
- Record audio underruns per frame in the benchmark, including camera movement.
- Add the Gradle -PfftaProfileSymbols option to retain native symbols when
  the Rust library is built with profile.release.strip=false.

A three-frame audio prebuffer was tried. It showed no improvement and was
removed; it is not part of the delivered APK.

<a id="validacion"></a>

## Validation

MEASURED: 90 gba-core tests passed, 0 failed, 3 ignored.
MEASURED: real-ROM test with host TinyCC and 600 frames without extra area:
framebuffer and OAM identical to the original interpreter on every frame.
MEASURED: Kotlin unit tests and Android assembly succeeded.
NOT MEASURED in this iteration: independent count of all 8 characters
per frame. Producers, ranges and the character list were not changed.

<a id="limites-conocidos"></a>

## Known limits

Performance has not reached a verified fixed 60 FPS. The screen capture
measured in the preceding iteration was around 55–60 FPS, with occasional drops.
Audio underruns remain; they must not be described as resolved.
The new APK was not measured on Snapdragon 855 in this iteration.
Game logic is not changed to accelerate rendering.

Test records remain outside this portable directory:
../target/android-validation/. Sources include no ROMs, saves or replays.
