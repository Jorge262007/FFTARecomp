# RG Rotate Android performance 0.33 ? 2026-10-29

Device: RG Rotate / T618 / Android 12, 720x720. Updated from installed 0.25 to reference 0.32, then tested and delivered 0.33 via install -r. No uninstall, data clear, governor or thermal changes. No new dependency or license.

## Changes

- Reject sprite rows before expensive detached-UI ownership classification. The integrated legacy UI still uses its relocated source row. Canonical stats coordinates are retained.
- Calculate a conservative sprite Y envelope once per immutable VBlank worker snapshot. Rows outside it need no OAM classification. Both guest OAM and published offscreen host actors contribute, including affine double-size and canonical portrait/stats coordinates. Replaced/hidden entries may widen coverage, never narrow it. Native scanlines still read live state; no cross-frame cache is introduced.
- Debug benchmark accepts a requested snapshot frame, allowing current replays to generate exact isolated battle fixtures. The reusable harness now accepts serial, dimensions, frames, replay and state, handles absent diagnostic files, validates binary transfers, restores diagnostics and returns to normal mode.

## Real camera test

The old 13100-frame replay stopped at name entry with the current keyboard. It was retained as diagnostic evidence, not counted as battle performance. A new isolated replay exits the keyboard and tutorial instructions, captures a battle state, then sweeps straight toward stage edges. Neither ROM nor state/replay assets are included in source.

Two reference runs and two final runs use the same state, keys and camera trajectory: 1200 frames each, initial 120 discarded, maximum zoom 6, 512x512 render. Each run includes 363 samples where camera coordinates actually change (the harness's adjacent-trimmed counter shows 362).

| CPU mean ms | Reference 0.32 | Final 0.33 |
|---|---:|---:|
| All sampled frames | 19.003 | 17.720 |
| Camera moving | 19.211 | 17.725 |
| Native raster | 5.192 | 5.050 |
| Expanded zoom rows | 3.074 | 2.054 |

Actual moving-camera frame cost fell 7.7%; expanded-row cost fell 33.2%. These are native CPU timings, not measured presentation FPS or a promise of 60 FPS on all stages. Early single-optimization results are separate from the final comparison.

All UI/framebuffer/OAM checkpoint hashes and every recorded camera/key trajectory entry match. Core tests: 273 passed, 7 ignored; randomized early-cull equivalence also checks fully empty rows outside the envelope. Gradle unit tests/build passed. No resolution reduction, frame skipping, simulation or audio changes.

Normal SRAM SHA256 remained d46cc3912a530fb3044e17f7c3f208eef6206038b5a394024fdb92eaaa956210. Main ROM untouched. Evidence: target/rotate32-battle-camera, target/rotate33-battle-final, target/rotate33-comparison.json. Intermediate zoom checks: target/rotate33-zoom2 and rotate33-zoom3.

Delivered APK: APK/FFTARecomp-android-arm64-preview.apk, versionCode33 (0.33-sprite-culling), checksum beside it. Desktop sources were not changed.

Intermediate zoom, same 1200-frame sweep, one run per level: zoom 3 (364x364) 15.448 ms native frame mean; zoom 2 (324x324) 15.990 ms. Run-to-run variation remains; the smaller run was not faster overall, and these single runs do not establish stable 60 FPS. Original diagnostic input was restored and the newly created benchmark.state removed; normal app relaunched.
