# Note 10 optimization and name-entry regression ? 2026-10-28

Device: SM-N970U1, Snapdragon 855, Android 12. Updates use `adb install -r`;
no uninstall or data clearing. ROM and the original `files/ffta.sav` are preserved.
Only AndroidPort is modified.

## Identical 6000-frame European intro replay, maximum zoom

Square reference viewport: 512x512. Exactly 3556 dialogue-active samples in each run.
These measure native simulation/JNI work, not delivered display FPS.

| Revision | Dialogue mean | Dialogue p95 |
| --- | ---: | ---: |
| Desktop-port prototype | 23.2367 ms | 27.2780 ms |
| Live-row reuse and masked-row elimination | 20.3044 ms | 22.0147 ms |
| Shared immutable menu cursor scan | 16.2428 ms | 17.7828 ms |

Mean decreases 30.1%. All 12 original framebuffer/UI/OAM checkpoint hashes match.
The new opt-in counters report setup falling from 4.0389 to 1.1600 ms between
the two optimized runs. Original prototype counters were placeholders and must
not be treated as a zero-cost raster/zoom measurement.

Work removed: decode live native UI rows only once; skip window-masked UI rows
without UI objects; reuse empty/masked plain-dialogue world rows conservatively;
share one frame-local cursor index between the original menu readers. Every
candidate is revalidated. No guest memory/input, animation timing, camera range,
world content or simulation frames are skipped. Extended stats/service/attack
panels keep the full path. Reference controls remain available through
`--reference-dialogue` and `--reference-frame` for the corresponding changes.

## Real device aspect and discovered keyboard bug

The old benchmark deliberately forced a square. `--device-aspect` now measures
676x320 at zoom 6, filling the 2280x1080 landscape display. The app preference
was 16:10 and was changed to its existing Full screen mode on this device.

The longer 13100-frame replay reached name entry and stayed there, so its last
segment is NOT a valid battle/camera measurement. It exposed the shared-atlas
name keyboard being misclassified as dialogue/attack UI. The script now labels
last samples as replay samples and separately detects actual HOFS/VOFS changes.

Name entry now uses its verified original task/ROM signatures and one centered
native UI canvas. It includes the keys, tabs, portrait, name frame and confirmation.
Priority-zero/one UI OBJ are captured; field actors stay in the world. Battle HUD
classification cannot remove the selected tab. Touch coordinates use the same
canvas. A regression checks a glyph and the priority-one name frame and verifies
that the dedicated layout ends when the original task ends. Device screenshots
were compared with the original LCD framebuffer extracted from the checked state.

Debug-only snapshots are restricted to sessions using benchmark.sav. Benchmark
QuickSave/manual slots use files/benchmark-saves, never files/Saves. No ROM, save
or replay is included in this document or redistributable sources.

## Reproduce

`python AndroidPort/scripts/benchmark_dialogue.py note10-test --adb C:/platform-tools/adb.exe --serial R58M78B8F0M --frames 6000 --zoom 6`

Add `--device-aspect` for the actual phone ratio. Requires the user's isolated
benchmark.input and benchmark.sav in app files. Local reports and screenshots:
AndroidPort/target/note10-optimization and target/dialogue-note10-*.

No claim of fixed 60 FPS at maximum zoom, or exhaustive battle performance.

Delivered APK SHA256: 459fa50a3df6d3f20b82902a47bb40a1b8b57902ef83d3456600f9feb0bcf098

## Delivery verification

- Core regression: 267 passed, 7 ignored; no failures.
- Android native tests: 66 passed, 2 ignored; no failures.
- Android Kotlin unit tests: 24 passed.
- Name glyph and priority-one frame regression passed.
- Physical Note 10: complete keyboard, selected tab, name frame and Yes/No
  confirmation inspected. A subsequent isolated 3000-frame input replay selected
  Yes and continued the original introduction beyond name entry.
- Original personal cartridge save compared byte-for-byte and unchanged.
- Final APK/native library provenance checked through merge, strip and zip packaging.
