# PC performance — 2026-10-27

Scope: general runtime and renderer performance, without shader work. PC mouse-facing selection is disabled; its calibrated core implementation remains available for Android.

Changes:
- Request the high-performance GPU on Windows, matching Linux.
- Enable the existing persistent two-worker zoom renderer on desktop. `FFTA_PARALLEL_ZOOM=0` disables it for comparisons. Headless widescreen tests use the same setting.
- Remove repeated process-environment lookups from the native block dispatch loop and audio DMA tracing.
- Avoid compositing margins that the world pass immediately overwrites; reject off-row sprites earlier.
- Reuse exact auxiliary UI packing results and limit transparent UI composition to conservative occupied spans.
- Replace per-pixel sorting with an equivalent two-target blend merge; exhaustive priority-order test passes.

Measurements (shader disabled):
- Deterministic 1,200-frame native replay: median 3.785 s before, 2.785 s after (26.4% less elapsed time, three alternating samples each). Final framebuffer hash and execution counts match. This is a startup replay, not a battle FPS claim.
- Captured battle renderer at maximum supported 16:9 zoom out, 568x320: median 3.490 ms before, 2.537 ms after (27.3% less renderer time). Fixed captured scene; not a complete moving-camera benchmark.
- All 18 renderer comparisons preserve framebuffer and UI hashes. Includes battle, abilities and prison at base, maximum zoom and a larger diagnostic stress size.
- Window smoke test selected NVIDIA GeForce RTX 5050 through Vulkan. No shader was enabled. This test was startup only.

Validation logs are under target/pc-optimization-*.log. Core suite: 254 passed, 7 ignored; runtime suite: 46 passed, 1 ignored; exhaustive UI-priority test and audio FIFO tests also pass. No resolution reduction or detail removal was used.

Run:
```powershell
cd "E:\AI\ARC\FFTARecomp"
.\target\release\FFTARecomp-PCOptimized.exe play "E:\AI\ARC\Final Fantasy Tactics Advance (Europe) (En,Fr,De,Es,It).gba"
```

Remaining validation: sustained maximum-zoom battle camera movement on the target PC; renderer snapshot timings do not measure input, presentation, or frame pacing.
`nReplay caveat: pc-maxzoom-battle.log reached 568x320 zoom settings, but visual inspection of its final frame shows name entry, not a battle. Its 13.019 ms/frame result is excluded from battle claims. Use the captured battle renderer measurement above until a verified moving-camera battle replay is available.

## Fixed-resolution battle HUD

The earlier per-sprite zoom compensation has been replaced with a native 480x160 transparent HUD canvas, presented after world zoom. L/R legends, all three Start: To Battle pieces (atlas 0x31c/0x320/0x324), the walking miniature and its circular base are extracted together from the live host publication/guest OAM. Start is centered between the shoulder legends at native row 16. Reserved world-actor atlas entries remain in the world pass. The new canvas is enabled by the desktop frontend only.

Validated against the open game's captured RAM, VRAM, OAM and live host sprite publication: target/hud-native-proof.png. Two regression tests verify identical HUD pixels for all seven zoom levels, inclusion of both preview components, title placement, removal outside the terrain scene, and exclusion of the world actor atlas. Shader code is unchanged.

## Complete battle HUD transitions

Battle HUD now uses four independent native canvases: shoulder legends/directional arrows, walking preview plus translucent oval, Start banner, and Dispatch Mode. Preview decoding supports varying palette banks and both 4bpp and 8bpp allocations; field actors remain excluded by priority/allocation. Host sprite positions are retained while group positions are canonicalized to prevent the native slide from competing with host motion. Each canvas uses translation-only entry/exit with cached complete pixels; changing troop while open replaces content without restarting entry. Deployment confirmation clears retained HUD canvases immediately.

Validation: five core HUD tests and two transition tests passed. `cargo check -p recomp` and release compilation passed. Arthur and the full visual transitions still require user verification in gameplay; palette/format coverage was tested synthetically. Executable: target/release/FFTARecomp-BattleHUD.exe.

Dispatch Mode anchor correction: verified live guest/host positions (179,128), (211,128), (227,128). Restore native origin (179,128), with bounds ending at Y=144 and sixteen native pixels of bottom clearance. Preserve translation-only host animation. Five HUD tests pass, including explicit origin and bottom-margin assertions.
