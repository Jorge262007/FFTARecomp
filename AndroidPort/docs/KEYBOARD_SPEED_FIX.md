# Name keyboard and speed update ? 2026-10-28

Android preview 0.29 uses rates 1, 1.5, 2, 3 and 4. Old saved speed 8 migrates to 4. A half-frame accumulator schedules 1.5x as alternating one/two guest frames; the audio resampler uses the constant requested rate, so those batches do not reset its phase or Normal-audio backlog. Integration test confirms 90 guest frames in 60 host ticks and rejection of speed 8.

The keyboard has its own semantic NameEntry canvas and host transition. Measured native BG2/BG3 scroll establishes entry/open/exit. The name-frame OBJ allocation keeps ownership during temporary cursor disappearance; partial native exit pixels never replace the cached full canvas. Keyboard OBJ remain excluded from widened world rendering even outside the native bounds. Host input is disabled during entry/exit and uses the rendered transform when open.

Letters, categories, delete and START are queued so later taps do not replace pending input. START's hit region includes its actual label and OK indicator. Android hides gameplay touch controls while naming to keep the letters accessible, with a Back button for deletion/cancel. Physical controls remain available.

Keyboard core and transition changes are shared with Windows. Its active executable was locked by the running game, so the newly linked binary was copied to target/release/recomp-keyboard-preview.exe; target/release/recomp.exe was not replaced.

## Validation

- 16 semantic input tests passed on each core.
- 12 desktop UI-motion tests passed.
- 70 Android native tests passed, 2 ignored; the additional fractional-speed integration test passed separately.
- Fractional audio test verifies resampling phase and persistent Normal-audio queue ownership.
- Private EUR ROM/state replay preserved keyboard ownership through frames 0..169 (confirmation, cancellation and exit), then released it on frame 171. No user saves were written.
- Android Kotlin compilation, unit tests and APK assembly succeeded.
- No physical Android device was connected through ADB; visual/touch acceptance on device remains pending.

APK: AndroidPort/APK/FFTARecomp-android-arm64-preview.apk. SHA256 is in the adjacent SHA256.txt.
