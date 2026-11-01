# Android frontend/save fix ? 2026-10-28

Preview 0.28 fixes a full-height versus native-height buffer mismatch. With nonzero vertical zoom padding, the renderer passed only width*160 source pixels to the custom save painter, which indexes (y+extra_y)*width. Large vertical padding caused an out-of-bounds panic when opening Save Game from the title.

`presentation_frame` now normalizes the raw image to render_width*render_height before host save painting. Only the original 160 rows are copied into the center when the PPU did not render expanded terrain rows. Save painting also sizes its output defensively.

Frontend padding uses only verified background resources: the repeating save/menu backdrop, clean title parchment patches, or language-selector margin rows. Foreground titles, flags, options and sprites are not tiled vertically. Existing terrain rows remain authoritative in gameplay.

Validation: 68 Android native tests passed, 2 ignored. Regression covers full-height composition with maximum phone zoom, source placement and save painting with both complete and legacy short inputs. The language-padding test passed and checks exclusion of the choice rows. APK assembled for arm64-v8a. No device was available via ADB for physical visual verification.
