# Dialogue transitions and Monster Bank v22

Presentation-only. No guest RAM, OAM, VRAM, saves or game logic is patched.
The existing exact-profile/paired-terrain/dialogue-plane guards remain in place;
extra=0 and disabled layout still bypass extraction.

## Changes

Bubble attachment is classified from BG3 backing before its guest HOFS/VOFS
animation, rather than requiring opaque pixels at a visible viewport edge in
that particular frame. A verified dialogue may have only one of BG2/BG3 enabled
during a transition. If edge evidence is incomplete, the current portrait and
attachment metadata keep its side/vertical anchor. An unrelated scene clears
that metadata. Each frame captures fresh pixels: no previous portrait or bubble
image is retained. Existing scale/length/offset settings are applied throughout.

For a site menu with speech (including Monster Bank), the menu bounds are removed
from the bubble's geometry evidence. The menu and its cursor are captured on the
auxiliary canvas and retain the existing centered native layout/mouse transform.
Speech and its portrait stay on the main edge-attached canvas, using the same
customized layout as ordinary dialogue. Other Pub/Shop submenu layouts are not
reassigned by this split. Live timings still need user review across the game.

The cinematic UI blend path is now used for every active shader profile, including
uncertain scenes with global temperature. The previous lighting_mode < 2.5 check
could select exact native compositing, which rebuilt unshaded raw-world pixels
under translucent UI and pasted those over the already graded stage. Shader OFF
retains exact native compositing. Monster Bank remains an outdoor scene: no
night preset or manually authored light profile is forced or rewritten.

## Verification

Before screenshot target/monster-bank-v21-before.png captured the user window,
showing centered speech and a horizontal sky-color discontinuity. Screenshots
are local ignored diagnostic artifacts, not distributed source assets.

CPU tests simulate opening/closing offsets, one-plane phases, both attachment
sides and custom scale/length, check native OAM/framebuffer invariance, and verify
that clearing the current artwork leaves no stale portrait/backing pixels.
The existing bank fixture now checks separate speech/menu output in both regions,
including mouse cursor ownership, while retaining identical native framebuffer
and OAM. The complete gba-core suite passes 165 tests (5 intentionally ignored).
Menu/screen suites pass 33/60 tests (one GPU test skipped by default).
The GPU test was also run explicitly on RTX5050, including translucent UI over
a non-black already colored stage at two zooms, verifying that the graded
background is retained. Native shader-OFF reconstruction is checked separately.

NO MEDIDO: live user playback of the new binary, every dialogue's guest animation
implementation, and all service-specific transitions. The running old window is
left open for the user to change builds. Existing lighting files are untouched.
Windows binary: target/release/FFTARecomp-Dialogue-v22.exe.
