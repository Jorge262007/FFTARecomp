# Battle laws modal

The desktop renderer keeps the observed laws table and its Select/HELP panel
in one centered native UI layer. Text, frame, selector and continuation marker
move together; the remaining character HUD keeps its own placement. Bubble
Size uses the existing centered modal layout and viewport clipping. Length
does not stretch the table or its text.

## Recognition and ownership

The measured EUR scene uses BG2=5C08/BG3=4A09 with zero BG2 scrolling and no
enabled hardware windows. Those registers are shared with action menus. The
old Notice recognizer required WIN0; without an edge HUD, this laws scene
could bypass UI extraction, while a remaining HUD could enable the action
layout and its X=160 fallback split.

`laws_bounds` in `crates/gba-core/src/ppu.rs` checks the table's BG3 top/bottom
frame, both row ends and column divider. It does not inspect translated labels.
The observed table occupies [8,72,216,128]. Its selector extends left to X=2,
so the captured modal bounds start at X=0. When all four HELP corners are
present at [16,16,224,64], the combined bounds are [0,16,224,128]. Otherwise
the table-only bounds are [0,72,216,128]. Existing action/terrain/exact-ROM
eligibility still applies; this is not a generic rectangular-window detector.

The existing `Notice` kind supplies whole-panel extraction, centering, no
side-panel BG clamp/Length expansion, and modal mouse protection. A separate
`DialogueState::laws_layout` host flag admits only the measured 8x16 selector
(tile 02DC, palette/priority zero) and HELP continuation marker to that layer.
The flag resets before each frame and uses `serde(skip)`: it adds no snapshot
bytes, guest writes, translation ABI or cache revision. An old snapshot can
recompute it on the next rendered frame. No new serialized enum variant exists.

## Validation, 2026-10-21

- Native release core suite: 183 passed, 5 optional ignored; runtime: 67 passed,
  4 optional ignored. The subsequently added snapshot-byte regression and laws
  layout regression both pass in final targeted native and Windows/Wine runs.
- Windows GNU core suite under Wine: 183 passed, 5 optional ignored. This is
  Wine compatibility evidence, not a native Windows check.
- Synthetic laws regression: table/HELP, HUD present/absent, widths 284/568/1136;
  exact native recomposition and unchanged framebuffer, OAM and VRAM. Disabling
  the new recognizer makes this regression fail. Incomplete frames clear the
  subtype rather than retaining stale laws ownership.
- Private EUR Italian/English scenes reproduced with the interpreter. A
  72-case offline matrix checks list/HELP, widths 240/284/568, zoom 1/2, Size
  0.5/1/2 and Length 1/2: all native reconstructions have zero differing pixels.
- Explicit `centered_laws_modal_matches_cpu_across_sizes` screen test: 24
  synthetic modal cases on llvmpipe Vulkan, zero GPU/CPU differences. Persistent
  layers cover both bounds, 3:2/16:9/ultrawide and odd window sizes, three Sizes,
  transparent gaps and blending over a changing-color world source.
- Actual Linux frontend: isolated Xvfb/llvmpipe Vulkan, 600-frame interpreter
  GUI replay, inspected complete laws/HELP and clean completion.
- Actual Windows executable: isolated Wine/Xvfb/llvmpipe Vulkan normal-play
  slot load and inspected laws/HELP. WM close exits successfully and preserves
  the slot bytes. Native and Windows release builds succeed.

Private evidence is under ignored `target/laws-fix/`: source probe, logs,
snapshot matrix, GPU comparison, and before/after/frontend captures. No ROM,
snapshot or screenshot is distributed in the repository or packages.

## Limits and separate observations

Native Windows, USA ROMs, other table layouts/law counts, the exact tutorial
state from the supplied screenshot, zoom above 2 and a laws-specific load/Undo
cycle remain unverified. Existing Notice/action/statistics and save-state tests
pass; those do not replace the missing manual cases. Large Size values retain
the centered layout's normal viewport clipping.

The first Windows bounded GUI benchmark displayed the panel correctly but
reported `benchmark interrupted at 601/600 frames`; it is failed benchmark
evidence, not a clean-completion result. Normal-play validation is separate.
An early GUI harness used an invalid input-file header, and another remained
on the pause menu; those attempts do not prove gameplay navigation. Keep their
limits distinct from the final inspected normal-play captures. No pacing or
benchmark change was made for this rendering fix, and portable exports remain
unchanged.
