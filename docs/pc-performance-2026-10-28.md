# PC maximum-zoom performance pass ? 2026-10-28

The rendering hot path still executes on the CPU before the existing GPU presenter scales/composites it. A faster GPU does not remove repeated PPU tile decoding and UI classification. At 4x all four guest frames are still simulated and rasterized; this change introduces no frame skipping and preserves the temporal response history.

## Changes

- Share the verified native BG2/BG3 portion of the current scanline with UI capture instead of decoding it again. No cache crosses guest VRAM writes.
- Skip UI work only when the owning hardware window masks the entire row and no UI OBJ needs capture.
- Reuse a composed world row where the UI is provably absent. Action, status, name-entry, notices, shop/service, battle HUD and shader surface capture keep the full path.
- Build one immutable frame-local index of menu cursors and feed the existing validated menu readers. No negative result/address cache persists across guest execution.
- Split the additional zoom rows into four independent tasks on hosts with sufficient CPU parallelism; retain the two-task path on smaller hosts. Native scanlines, guest CPU, IRQ, DMA, input and audio timing are unchanged.
- Opt-in PPU profiling counters expose setup/capture/raster/zoom costs. No new dependency was added.

## Reproducible render benchmark

`crates/gba-core/examples/pc_render_benchmark.rs` loads a private copied state, varies BG0/BG1 horizontal scroll in a straight line, renders 96 complete frames per resolution and times the final 80. Both tests use maximum 2x view expansion (320 render rows), separate native UI and parallel zoom. The raw state is read from a copy of QuickSave; no user save is written.

Repeated before/after measurements on the same host:

| Render size | Before mean | After mean | Reduction | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 568 x 320 (16:9) | 5.242 ms | 3.391 ms | 35.3% | 5.953 ms | 4.252 ms |
| 1136 x 320 (32:9) | 6.974 ms | 4.748 ms | 31.9% | 8.458 ms | 5.449 ms |

Rolling hashes over every sampled wide framebuffer, native framebuffer and main/extra UI canvas are unchanged: `586d13c95f0ff32d` and `06b122b410cd3fb9` respectively. These are PPU measurements, not whole-game FPS or a guarantee of 60 FPS at 4x. CPU execution, audio, host UI preparation, GPU presentation and display waiting are outside this benchmark.

Evidence is under `target/pc-performance/` (ignored build/test data). The baseline executable is retained there for repeat comparisons. Core tests include pixel-exact serial/parallel and native-row reuse checks; no rendered intermediate frame is dropped.

## Validation

Core suite with parallel-ppu: 271 passed, 7 ignored. Frontend executable suite: 78 passed, 1 ignored. Frontend verification targets the recomp binary; the all-targets command also finds a pre-existing missing `world_rect` initializer in the unrelated `dialogue_layout` example.
