# Desktop features port: Android preview 0.26

Work is isolated in `AndroidPort/`. The desktop source remains unchanged.

## Included

- Current desktop EUR core/UI fixes, semantic four-panel composition, and four independent battle HUD layers.
- Host-owned entry/exit transitions and uniform panel scaling independent of camera zoom.
- Checked full-machine saves in the app private `files/Saves` directory: QuickSave every five seconds of eligible gameplay, Save1, Save2 and additional carousel slots.
- Existing cartridge saves are read first. New manual saves use the latest clean map snapshot; an existing original is archived only after its occupied slots have migrated. Save-state format matches desktop, but requires the same ROM/BIOS and compatible core version.
- Blue add-card, two-pose arrows, save-only red trash icon and confirmation. Deleted snapshots move to `Saves/Deleted`.
- Direct save-card taps work even with experimental gameplay touch selection off. Touch controls retain their own gestures.
- Full-resolution preview textures, not previews enlarged from their small card thumbnail.
- Persistent video settings: aspect ratio, zoom, fullscreen/system-bars mode, screen model, brightness, panel darkness, LCD response, pixel grid, scaling and VSync.
- Original desktop color LUT generation. Display effects are a GPU post-process over both world and game UI; default Raw/Nearest/Off bypasses the additional framebuffer pass.

## Platform differences

Android fullscreen means immersive mode; window mode shows system bars. Android controls the actual app window. VSync requests the EGL swap interval; device compositor policy still applies. LCD response and scaling are GLES implementations, not the desktop WGPU implementation. This preview does not include the experimental cinematic lighting shader.

## Validation

Native unit tests, including save checksums, backup recovery, deletion, carousel, eligibility and UI transitions, run without a device. Audio tests share a process-global reader, so use `cargo test -p gba-android --lib -- --test-threads=1`.

Physical-device validation is pending: legacy save migration, save/load during battle, map manual saves, touch and gamepad navigation, live video changes, and maximum zoom-out performance with dialogue. Do not clear app data or uninstall to update this preview.
