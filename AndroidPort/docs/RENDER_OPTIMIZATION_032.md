# Android renderer optimization 0.32 ? 2026-10-28

Installed on Note 10 SM-N970U1 / Android 12 with install -r; no uninstall or data reset.

Changes are confined to the Android renderer. Brightness/darkness composition now finds only the winning layer instead of searching for an unused second layer. Real alpha and semi-transparent OBJ keep the full compositor. Per-window effects and target bits are preserved. Finite terrain and viewport-local effects skip contiguous transparent margin intervals rather than testing every empty pixel; the native viewport boundary remains a policy boundary.

Validation: 272 core tests passed, 7 ignored; Gradle unit tests passed. Existing randomized reference-compositor checks cover windows, priorities and blending; parallel-row and dialogue regressions passed. No frames skipped, no reduced resolution, no audio changes, no new dependencies.

## Physical measurement

Two warmed baseline 0.31 trials followed by two final 0.32 trials, 600 frames each, first 120 discarded, 676x320 maximum zoom:

| Mean of two trials | 0.31 | 0.32 |
|---|---:|---:|
| Native frame ms | 15.710 | 15.610 |
| Raster ms | 6.699 | 6.325 |
| Expanded rows ms | 2.012 | 2.235 |

Raster cost fell 5.6%. Overall frame timing varied enough that this does not establish a meaningful overall FPS increase. This is an isolated snapshot replay, not a moving-camera battle benchmark (camera changes=0). Broader stage testing remains necessary. Earlier cold/warming trials are also retained, not mixed into this comparison.

All final framebuffer/UI/OAM hashes match the baseline: bd552d56572745e9 / 3130e5f2da6dd9b8 / bf5f204179b9f531. Normal SRAM SHA256 unchanged: 71189f7fb6aed638640078fba3a35fda6c39c8962e74dcc75935aac948da9063. Diagnostic inputs/states restored; app returned to normal mode.

Evidence: AndroidPort/target/note10-blend-before-warm and note10-blend-final. Reusable harness: scripts/benchmark_android_render.py --label LABEL --trials 2 (requires connected test device and reference.state fixture). Artifact: APK/FFTARecomp-android-arm64-preview.apk, versionCode32; checksum in APK/SHA256.txt.
