# FFTA character and scene OBJ: runtime publication fix

Regional metadata correction (ROMDATA-04): the EUR main draw routines use the
direct ROM dimensions at0x14ced8, not the copied IWRAM table. The host decoder
now follows that original source in both legacy and decoded modes; battle and
visibility code retain mutable IWRAM dimensions. A48-case EUR/USA matrix checks
the actual original instructions, registers/cycles, writes and host attributes.
See [decoded metadata runtime](rom_data_runtime.md) for file-backed consumption,
the additional EUR copied alias, source provenance and remaining scene limits.

MEDIDO: native battle DMA submits OAM in at least two pieces:

```text
FFTA_DMA frame=13680 channel=0 src=0x03002DD8 dst=0x07000000 count=8 unit=4 timing=0 control=0x8400
FFTA_DMA frame=13680 channel=0 src=0x03000430 dst=0x07000020 count=56 unit=4 timing=0 control=0x8400
```

The first submission contains cursor/UI objects; character descriptors also
use the second submission. The old decoder accepted only 03000030/03000430,
and publication accepted only destination 07000000. Both restrictions
prevented character host lists from reaching the PPU.
MEDIDO: isolated BG2/BG3 images also contain static scene elements such as
trees and fences. Those are backgrounds and retain the existing BG patch;
the OBJ change cannot manufacture extra background tilemap contents.

## Own-code change

The decoder accepts the actual aligned EWRAM/IWRAM buffer passed to the
verified function. Its capacity must be at most 128 and fit in RAM.
It retains all input descriptors for the host, including those rejected
by the original coordinate or emission-capacity checks. No guest writes
or guest instruction/cycle changes were added.

Publication accepts aligned partial transfers inside hardware OAM.
Each segment validates the first six attribute bytes of every replacement
against the correct transferred destination, then shifts nominal slots by
destination offset / 8. Segment zero starts a new submission; subsequent
segments are merged. Unknown or mismatching overlapping transfers remove
the corresponding segment instead of rendering stale replacements.
DMA source data and the original guest OAM remain unchanged.

The PPU stably merges sorted host entries and current guest OAM in linear
time, without a Vec allocation or repeated full host scans per scanline.
Pending vectors retain capacity on frame rebuilds. Attribute comparisons
use stack values rather than allocating a Vec per comparison.
Original priority, affine OAM matrix reads, object windows and alpha rules
remain in the existing pixel renderer. Entries after nominal slot 128 are
also rendered; host X/Y are i32 rather than packed OAM coordinates.

The existing FFTA_PATCH_SPR=0/1 switch, required ROM SHA256 and complete
function-digest/short-byte checks remain unchanged. The copied RAM routine
is also compared against the user's ROM before hooking it.
No ROM bytes or generated game DLL are added for distribution.
No installer changes or commits are part of this task.

## Tests and literal results

MEDIDO: cargo test -p gba-core -p recomp-core -p recomp --lib --bins:
80 passed in gba-core, 6 in recomp, 3 in recomp-core; no failures.
The later RAM integration test is ignored by default and run explicitly.
The new partial-transfer test covers cursor/character merging, shifted
indices, X=900 retention and fail-closed overlapping/mismatching updates.
The 1536-pixel renderer test covers >200 objects, X=-600/700/800, a nominal
slot of 200, stable priority, affine transforms and semitransparency.

Both ROM-backed tests read only the user's local ROM, with absolute path:

```powershell
$env:FFTA_TEST_ROM='E:/AI/ARC/gba-recomp/Final Fantasy Tactics Advance (USA).gba'
cargo test -p gba-core ffta_decoder_matches_original_and_keeps_rejected_objects --lib -- --ignored --nocapture
cargo test -p gba-core ffta_ram_battle_decoder_matches_original --lib -- --ignored --nocapture
```

```text
FFTA_DECODER original_attributes=IDENTICAL cases=24 host_objects_per_case=200 original_limit=48
FFTA_RAM_DECODER original_attributes=IDENTICAL cases=24 host_objects_per_case=200 original_limit=48
```

The RAM cases vary bank, X/Y, flips, per-object priority, mosaic and original
capacity. The test copies the routine into test RAM from the user-owned ROM
at runtime; no game instruction bytes are embedded in the test.

MEDIDO: native SPR=0 versus SPR=1, extra=0, 600 boot frames:

```text
frames: 600 hash: 727e9157e9bcb0a6
frames: 600 hash: 727e9157e9bcb0a6
```

Both final framebuffer SHA256:
b53a744554887531fb57c7bb2a3860b67b6a53b5502172f964c5d9c746de1ad3.
Both OAM SHA256:
d38556ef73dbe61e6ee9f9712f1f5d7a595f8238df1cab1a9f373eae32379747.
SPR=1 executed 833 verified hook entries. These are final hashes after
600 frames, not an assertion that a per-frame hash stream was collected.

## Native battle A/B in 32:9

MEDIDO: same input recording and common warm-up, frames 13680..14999.
The control and patched runs have equal final central framebuffer and OAM
SHA256, and equal starting EWRAM/IWRAM/IO/OAM scene hashes.

```text
FFTA_HOST publications=2638 objects=44
FFTA_PIPELINE decoded=10384 last_failed_buffer=None no_input=2 mismatches=0 active=true
FFTA_HOST_TILES frame=15000 objects=44 original_rejected_or_capped=3 all_tiles_zero=0 out_of_obj_vram=0 obj_vram_nonzero_bytes=15899 status=CONTENTS_ONLY
```

The two initial no-input transfers occur at the common warm-up boundary;
there is no pending host list before the first hooked frame rebuild.
NO MEDIDO: absence of a transient on every possible enable/disable boundary.

Captured extrema of this traversal, not proved global map limits:

| Capture | Frame | Terrain HOFS/VOFS | Changed central pixels | Changed margin pixels |
|---|---|---|---|---|
| Left | 14007 | 152 / 36 | 0 | 184 |
| Right | 14473 | 216 / 64 | 0 | 171 |
| Top | 13992 | 168 / 8 | 0 | 154 |
| Bottom | 14428 | 152 / 87 | 0 | 169 |

MEDIDO: visual A/B restores character pixels at image X=404..411 in the
right margin of all four images. It does not prove left-margin recovery
when this recording places no rejected character there.
Evidence: target/ffta-movement/sprites-control-*.png and
sprites-combined-*.png. Corresponding PPMs are unmodified native output.

MEDIDO: headless native elapsed time over the same 1320-frame interval:
SPR=0 7.321428 ms/frame; SPR=1 7.374606 ms/frame.
This run has a populated host list. NO MEDIDO: desktop GPU presentation,
LOG+SPR interaction or statistical confidence from multiple repeats.

MEDIDO: the same native traversal at 1536 pixels completed at
11.262409 ms/frame with 44 host objects, 2638 publications and zero
attribute mismatches. All four images are in sprites-1536-*.png in the
capture directory. The 9-bit OAM X limit does not constrain the host i32
coordinates; it still constrains unhooked hardware OAM entries.
The OAM_LOG tile diagnostic now also samples partial OAM destinations,
so a new user capture can observe the combined character list rather
than only the first cursor submission.

MEDIDO only at the final frame: all three recovered/capped objects have
nonzero tile data and valid OBJ VRAM references; 15899 of 32768 OBJ bytes
are nonzero. Nonzero data is not proof that every tile contains correct
art. NO MEDIDO: tile availability throughout every battle, camera-driven
tile eviction, upstream object-list culling, or full-map object enumeration.
These limitations prevent claiming that all conceivable clipping is solved.

## Run the patched product path

```powershell
$env:FFTA_WIDESCREEN='32:9'
$env:FFTA_PATCH_BG='1'
$env:FFTA_BG_EDGE='transparent'
$env:FFTA_PATCH_SPR='1'
$env:FFTA_OAM_LOG='0'
.\target\release\recomp.exe play 'Final Fantasy Tactics Advance (USA).gba'
```

Use SPR=0 for A/B. Clear FFTA_WIDE_DEBUG or use a fresh PowerShell: nospr
intentionally hides margin sprites. Numeric WIDESCREEN=648 selects the
1536-pixel maximum width (648 extra pixels on each side of 240).
