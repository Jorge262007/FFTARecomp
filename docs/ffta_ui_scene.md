# Host UI scene and reference catalogue

The EUR desktop UI is composed after the world/shader, in physical screen space.
World zoom is not an input to panel layout. The native GBA framebuffer and OAM
remain the compatibility reference; the separated canvases affect only presentation.

## Architecture

- gba-core/src/ppu.rs captures verified BG2/BG3/OBJ UI. BG2/BG3 are decoded once
  per scanline for all panel/backing passes. VRAM changes between scanlines remain
  visible. Empty action-backing OBJ passes and pixels owned by another panel skip
  unnecessary composition.
- gba-core/src/ffta_ui.rs supplies semantic roles, full native bounds and edge,
  corner, center or modal anchors. play.rs builds the scene once per emulated
  frame; rendering, mouse hit conversion and reference inspection consume it.
- Complete unscrolled backing geometry and original panel-owner bounds determine
  layout. A partial entrance, long label or blinking cursor does not redefine
  that geometry. Original animation content/windowing is retained in the canvas.
- Size uses one uniform scale. Oversized panels reduce their effective scale to
  fit the viewport, including Length. Length extends backing and translates
  foreground rigidly; the translation is limited before it discards glyphs or
  portraits at the native canvas boundary.
- PackedLayer compares source pixels, backing and style exactly. Unchanged panels
  reuse their packed bytes and geometry. No hash is used to decide pixel reuse.
- ui_layer.rs keeps the original raw-world blend when the shader is disabled.
  With cinematic blending enabled, UI blends over the already lit scene and no
  raw world texture upload/copy is required. Opaque panels also skip that upload.

These changes cover already classified panels. They do not magically recognize
all game UI, decode dialogue strings, or reflow arbitrary replacement text.

## Optional CPU diagnostics

PowerShell 5.1:

    $env:FFTA_UI_TIMING='1'
    .\target\release\recomp.exe play 'E:\AI\ARC\Final Fantasy Tactics Advance (Europe) (En,Fr,De,Es,It).gba'

FFTA_UI_CPU reports emulator+PPU and host preparation mean/max over 120 frames.
FFTA_UI_PRESENT reports CPU presentation including driver/vsync wait. It is NOT
a GPU timestamp measurement. Diagnostics are off by default.

## Optional references for future mods

    $env:FFTA_UI_REFS='E:\AI\ARC\FFTARecomp\target\ui-refs.jsonl'

This appends a JSON record when reference content changes: semantic panel role,
native bounds, BG register/map/character addresses, unscrolled BG tile positions,
tile content hashes, and verified UI OAM references. All addresses are decimal GBA
addresses; tile coordinates are source-map coordinates, not physical mouse pixels.
Different regions/languages/assets must not be matched solely by one tile number.

BG2 candidates include headers/borders as well as glyphs. The format explicitly
sets decoded_text=false and rom_string_address=null. A VRAM tile address is NOT
a ROM string pointer. No OCR, string-table guess or universal text-editing claim
is made. Decoding source dialogue tables and reinserting translated text remain
separate work. This is a reference interface, not a finished mod manager.

The catalogue contains metadata/hashes, not ROM bytes, saves or sprite bitmaps.
Enabling it adds inspection and disk-writing cost; leave it unset for performance.

## Validation

Release CPU tests cover source changes, cache invalidation, disappearing panels,
oversized/long panels, stable animation geometry, existing native composition,
and source references. The ignored GPU test compares actual GPU readback with
CPU expected pixels, with shader on/off and world texture resizing. The EUR
ffta_ui_zero_extra example compares framebuffer and OAM for 600 frames without
loading or writing any save.

Packing benchmark: synthetic unchanged 480x160 panel at Length 150%, 1000 iterations.
Measured 0.3886 ms uncached vs 0.1223 ms cached. This is host packing only, not a
gameplay FPS improvement or a GPU benchmark. Synthetic complete-dialogue renderer
measurements varied between runs; no significant overall speedup was demonstrated.
Real-scene visual review and end-to-end performance measurements remain necessary.


## Visual review follow-up

Four presentation slots: main content, root/secondary panel, Clan Funds, and
short action help/caption. Service capture keeps its native base scroll; verified
action/attack capsules use the unscrolled source path. See ffta_ui_review.md for
the checklist and limits of the effect-plane classification.
