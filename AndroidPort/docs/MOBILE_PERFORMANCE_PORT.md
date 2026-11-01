# Mobile performance port ? 2026-10-28

Android preview 0.31 was installed with `adb install -r` on the connected SM-N970U1 / Android 12. The earlier installed build was 0.26. The delivered APK includes all intervening keyboard, save-screen, fractional speed, frontend padding and dialogue-exit fixes.

Native BG/UI row reuse, masked-row elimination, shared menu indexing and conservative one-pass world composition already existed in Android. The missing desktop change was the independent four-part zoom render. It is now ported, retaining Android affinity and selecting 2 or 4 workers from allowed CPU capacity. Workers use only CPUs within 70% of the fastest allowed CPU's capacity. The Note 10 exposes four such cores; two-big-core devices retain two workers. Unavailable capacity metadata falls back to two. No governor or thermal settings are changed.

The test override is debug-only frontend tooling and is set before thread-pool creation. Physical thread listing confirmed FFTA-zoom-0..3. Serial/parallel pixel tests and Gradle unit tests passed. Native ARM64 library and APK rebuilt successfully.

## Physical checks

Isolated 600-frame replays at 676x320 (device aspect, maximum zoom) retained the exact same final UI, OAM and framebuffer hashes with 2 and 4 workers. Normal SRAM SHA256 remained 71189f7fb6aed638640078fba3a35fda6c39c8962e74dcc75935aac948da9063. Existing benchmark state/input were backed up and restored. No normal ROM or user SRAM was changed. The app was returned to normal mode after testing.

Warm runs varied: 2 workers 15.705 ms overall / 1.916 ms expanded rows; final 4-worker run 15.493 ms overall / 1.889 ms expanded rows. Earlier samples were slower during native-cache warmup. This does not establish a significant overall FPS improvement, nor does it transfer the desktop 35% claim to Android. Camera changes were zero in the sampled snapshot, so this was not a moving-camera battle benchmark. Evidence is under AndroidPort/target/note10-workers and note10-workers-warm.

APK: AndroidPort/APK/FFTARecomp-android-arm64-preview.apk. SHA256 is recorded alongside it. No new dependency or license was introduced.
