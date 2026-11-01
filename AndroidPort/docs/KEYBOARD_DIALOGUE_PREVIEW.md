# Keyboard and dialogue preview ? 2026-10-28

Android preview 0.27 and the Windows `target/release/recomp.exe` share:

- Native name-entry keyboard on a centered UI canvas, independent of world zoom.
- Name glyphs, tabs, portrait and name frame captured together, excluding deployment HUD extraction.
- Pointer coordinates transformed through the rendered UI canvas on Windows and Android.
- Keyboard page tabs require a click; hovering cannot switch pages.
- Android name-entry taps work with experimental stage clicks disabled. Touch controls still consume their own gestures.
- Dialogue exit retains the complete last canvas and its geometry while the original window shrinks; exit takes 220 ms. Entry remains 150 ms.
- Verified speech-window phase detection also applies outside service interiors; constructor and BG-map validation remain required.

## Verification

- Desktop core suite: 266 passed, 7 ignored before the added input test; targeted naming tests: 2 passed.
- Desktop UI motion: 11 passed, including complete-canvas exit through shrinking guest bounds.
- Android native: 67 passed, 2 ignored; targeted naming tests: 2 passed.
- Generic dialogue exit owner detection: passed on desktop.
- Android Gradle unit tests and debug APK build succeeded.
- No physical Android device was available through ADB during this change. Visual and physical touch review remains necessary.

APK: `AndroidPort/APK/FFTARecomp-android-arm64-preview.apk` (SHA256 in the adjacent SHA256.txt).
