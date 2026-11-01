<a id="personajes-y-fast-forward-normal"></a>

# Characters and Normal fast-forward audio

<a id="personajes"></a>

## Characters

Identified cause: visibility check 030062EC, already used by the PC fix,
extended X but still rejected Y outside 0..159.
Android also needs the zoom's extra_y vertical margin.
Only this check is extended to -extra_y .. 159+extra_y, retaining ROM SHA256,
function-hash and ROM/RAM match checks.
The result is unchanged with extra_x=0 and extra_y=0.
FFTA_PATCH_ZOOM_SPR=0 allows comparison in the host runner.
Original producers, attributes, priority and OAM limits are preserved.

MEASURED in the Android engine with host TinyCC, replay from frame 0:
frames 11900..13100, 1201 total, 512x512 output, zoom 6.
Before: 646 frames with fewer than 8 characters; minimum 2 published and drawn.
After: 8 published and 8 drawn in all 1201 frames; zero duplicates.
Previously missing owners included positions Y=-48 and Y=240.

The metric uses the PC-measured roster at 020223A4, with 0x90-byte records:
identifier at +64 hex, body at +5C and auxiliary at +60.
It groups all body pieces by owner and counts IDs 0..7 only once.
It excludes auxiliaries, cursor, effects and ID 8. Drawing requires composed
body pixels. Results are rejected if the roster signature does not match.
Pixel capture is enabled only near the end of the benchmark, not in normal
play. Extra rows are rendered serially during this metric to preserve
counters; those timings cannot measure normal performance.

Local evidence CSVs, outside the portable sources:
../target/android-validation/characters-before.csv and characters-after.csv.
The complete list of previous failures is in characters-before-failures.csv.

<a id="audio-normal-con-fast-forward"></a>

## Normal audio during fast-forward

Identified cause: Android consumed a single 16.74 ms audio block after each
simulation batch. PC consumes it from an independent output thread.
When a batch took longer than that block, Android emptied its AudioTrack.

PCM at x1 and its disk queue are retained, but an independent audio thread
consumes it at 48000 Hz without taking the emulator mutex. Game batches do
not determine how many ms of audio are delivered. This is used only during
fast-forward with Normal; accelerated audio, Silent and normal-speed play
retain their paths.

MEASURED: complete-batch draining, sample order and stereo-pair test.
NOT MEASURED: subjective audio listening; user feedback is required.
The debug --audio-check test runs 600 startup frames followed by 120 x4
batches. benchmark-normal-audio.csv records played pairs and underruns.

<a id="comprobaciones"></a>

## Checks

MEASURED: 600 real-ROM frames with extra=0, identical framebuffer/OAM.
MEASURED: 90 gba-core tests, 9 native Android tests and 12 Kotlin tests passed.
APK: ../APK/FFTARecomp-android-arm64-preview.apk, version 0.16.
No commits, PC/installer changes or access to original saves in this iteration.

<a id="confirmacion-en-rg-rotate"></a>

## RG Rotate confirmation

MEASURED ARM64: full benchmark from frame 0, final 1201 frames
11900..13100, 8 distinct IDs published and drawn in every frame; zero failures
and zero duplicates. CSV: ../target/android-validation/urgent-device/benchmark-characters.csv.

MEASURED Normal audio: 120 batches with configured x4 multiplier,
4.159 s test, 197760 played pairs and zero independent-thread underruns.
Output configured at 48000 Hz; total time includes startup. This does not
claim the T618 emulates at effective x4 or that subjective listening has
been validated. CSV: ../target/android-validation/urgent-device-audio/benchmark-normal-audio.csv.

The character test enables counters; do not use its timings to assess
performance. Normal play retains parallel zoom rendering.

<a id="fragmento-fijo-en-el-cielo"></a>

## Fixed fragment in the sky

MEASURED: in the tutorial, 113 leftover OAM entries (slots 15..127) use
attributes 00A8,00F8,0000 and do not correspond to host-published pieces.
The original GBA screen does not show them. Upward vertical zoom exposes
their tile 0 at X=248,Y=-88; all entries overlap there.
The fragment was observed on the device screen before the fix.

The PPU skips only this exact pattern when it lacks host coordinates,
with the FFTA patch enabled and extra_y>0. The FFTA gate already requires
the verified ROM. OAM, VRAM, ROM and character coordinates are unchanged.
A legitimate host piece with the same attributes is still drawn.
MEASURED: dedicated test passed; 91 gba-core tests passed.
The input OAM list is in ../target/android-validation/sky-before.oam.csv.

MEASURED visually on RG Rotate: the fragment is absent from the new capture.
In its fixed rectangle (x=540..551,y=124..135 on the 720x720 display),
107 pixels differed from the sky on the same row before, and 0 after.
The captures have different cameras; this check compares only the fixed
fragment with adjacent sky, not the complete framebuffer.
Local captures: ../target/android-validation/sky-game.png and sky-after-device.png.
MEASURED: host replay retains 8 published/drawn characters for 1201 frames;
600 frames without extra area retain identical framebuffer and OAM.
