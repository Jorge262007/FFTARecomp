# Auto Save — desktop full-machine state

The original save/load card selector now shows three uniformly reduced cards. A green **Auto Save** card is on the left; File No.1 and File No.2 retain the original save-game logic. Mouse hit areas match the new card rectangles; keyboard and controller navigate all three. Auto Save loads immediately and is read-only in this selector. It displays NO SAVE until the first state has been written.

Every five seconds of active gameplay, the frontend captures a complete guest-machine state at a frame boundary. Title/load/save selectors and pause do not replace it. Encoding occurs on the game thread; disk writes run on a bounded background worker. The previous state is copied to .bak before atomic replacement; pending work is drained on normal exit. No cartridge .sav file is replaced by the autosave writer.

Location: next to the resolved .sav, with the ROM's twelve-character SHA256 prefix. For E:\AI\ARC\ffta.sav and EUR:
`E:\AI\ARC\ffta.8fab1e390cb9.autosave.state`
The backup is `ffta.8fab1e390cb9.autosave.state.bak`.

Version 1 stores CPU registers and banked registers, all mutable guest memories, IRQ/IntrWait state, timers, DMA latches, video clock/phase, affine accumulators, backup-media protocol/data, RTC protocol, audio FIFO/DAC/PSG state and native framebuffer. It excludes the ROM, compiled code and host pointers. SHA256 checks bind it to the exact ROM and BIOS and validate the payload; corrupted/incompatible data is rejected before modifying the current machine. These states are not emulator .sav files and are not transferable between ROM regions. Normal .sav compatibility is unchanged.

On restore, native decoded-block memoization and host UI/sprite/surface caches are invalidated; audio shadow mixing is rearmed from the guest state, and input is released. The normal game renderer rebuilds the view. Current video/input preferences remain active. State restore rewinds the guest backup medium as part of the complete machine, as conventional save states do.

Validation:
- Core deterministic continuation: serialize/restore, then compare subsequent full states across five frames with active timer/audio. Pass.
- Corruption, truncation and other-ROM rejection with no partial mutation. Pass.
- Atomic replacement/previous-state backup and exact three-card hitboxes. Pass.
- Native EUR replay: ordinary run and restore at frame600 both finish frame1200 with hash 6fb3ed275933006e and identical 4,067,051 native blocks / 1,972,835 fallback steps.
- Three-card appearance checked using actual captured selector RAM/VRAM/OAM: target/autosave-selector-proof.png.

User gameplay validation remains necessary for restoring a late-game battle, dialogue and naming input. Future format changes require a new state version; do not interpret an incompatible version as SRAM. No new third-party dependency was added. The unfinished F12 scene-skip route was removed from the frontend.

Additional validation: automatic timer test passes (no write before five seconds, no selector overwrite, full state written after threshold, normal .sav untouched). Primary state loading now automatically falls back to .state.bak when the primary is missing/corrupted. The final host autosave test suite has three passing tests. Keyboard selection also accepts remapped GBA left/right/A bindings.

## Recreated cards and saved preview

The three cards are drawn independently at native glyph resolution (no downscaled text). Bounds are [6,48,78,148], [84,48,156,148], [162,48,234,148]. All backings are semitransparent; Auto Save is green, the original slots blue. Original header information is read from the save-task manager rather than copying/resampling its rendered text.

Auto Save metadata and image are reconstructed from the same checksummed machine state, including after application restart. Live gameplay metadata was verified against EUR0813FC28: player name at EWRAM1F1C, level at89, hours at2FBC (low ten bits), minutes at2FBE. Original file headers store name at1A, level at2E, packed hours/minutes at30, clan at52, clan level at74 and missions at76. Recorded headers match Marche Lv50, 32:01 / 32:30; captured gameplay RAM reads Marche Lv50, 32:45. No clock estimate based on host uptime is used.

The preview is a 63x42 (exact 3:2) area-averaged image of the saved native framebuffer. Only the image is downsampled; all labels, borders and numeric fields retain native one-pixel glyphs. Metadata is published to the UI only after the background state write succeeds. Core state format remains version1; existing states gain a preview without migration.

Validation: five host autosave tests passed (including decoded metadata, native preview colors/aspect, checksum rejection and persistence across restart); core state regression also rerun. New design visually checked with actual captured save-selector data and a recorded gameplay state. The frontend's normal mouse/keyboard/controller routes are preserved.

Compact cards now use a native 3x5 font: title, player name, level, 63x42 screenshot, and time at the bottom. Clan/Clear details are omitted. Normal saves gain previews when a new cartridge header is observed, using the last active gameplay framebuffer before the save menus. Preview sidecar is .previews beside the .sav; entries are keyed to the original header (including save generation/checksum), so old or externally replaced saves are not assigned unrelated screenshots. Existing manual saves have no picture until saved again. These images do not alter cartridge saves.

Refinement: save/load selector identity now follows the shared EUR task callback through tasks4/11 rather than matching transient BG control words. Original slot coordinates are translated for each phase. Text has a one-pixel black outline; all preview areas have a recessed, bevelled frame. Previews now sample original pixel colours without area averaging. Automatic full-state capture includes the world map and internal menus after gameplay starts; title/language/boot screens remain excluded.

## Save selector redesign (supersedes the earlier compact-card presentation)
Cards now render through the existing menu Canvas at physical window resolution, with scalable embedded fonts, themed blue/green panels, gold selection borders and inset screenshot frames. The guest's extracted UI layers, grid and temporal filtering do not composite over this interface. Pointer coordinates use its own centered 720x480 logical layout. The rendered canvas is cached until selection, resolution, phase or slot headers change.

New previews preserve every pixel of the native 240x160 framebuffer, and are sampled once at their final displayed size. Manual sidecar version2 stores full frames. Version1 images are migrated with nearest sampling (their lost detail can only be recovered by making a new save). Existing full machine states already contain the full framebuffer and automatically acquire sharper previews.

Recorded EUR replay identifies task4 as the separate Resume Battle / Load choice, task11 as the actual file cards and task255 as the file-exit transition; callback0813DD75 owns the latter two. The host replaces only file cards and their exit, leaving Resume Battle / Load functional. Live world-map callback08031601 also uses task11, so the task number alone must never determine eligibility.

Auto Save now requires verified terrain BG identity (battle/interiors), or the verified map BG pair AA03/AC02 with its live world callback. Save/load screens, Party/Clan selectors, management panels, title, blank frames and host settings are excluded. This supersedes the previous policy which allowed internal management menus.

## Classic presentation restored
The full-window save redesign was removed. Save cards again occupy the original GBA selector over its own patterned background, with outlined 5x7 text and translucent blue/green backing. Full-size preview storage and gameplay-only Auto Save eligibility are retained. The host suppresses extracted original card layers while drawing the three-card presentation.

Manual cards now call the unmodified EUR shared confirmation handler0813DD74 with the verified selection byte at manager+225D and the original A-edge at03000002, on a temporary CPU/stack. The handler validates the original per-slot flags, schedules state22/next12 and returns; normal guest execution performs the complete original loading process. The normal CPU registers and input edges are preserved. Hover does not simulate guest navigation. This avoids a second on-screen file selection.

Native replay validation: at frame2600, direct File1 confirmation schedules state22/next12 and reaches live map callback08031601 with Marche Lv50,32:01. File2 independently reaches the map with Marche Lv50,32:30. No controller/key pulses were present after frame2304 in either test. Cartridge file was read only by replay. Logs target/manual-native-slot1.log and target/manual-native-slot2.log.

## Cartridge detail and direct Saved Game entry
The classic screen now keeps the original tiled backdrop and uses a compact 130x19 parchment caption. Cards have clipped corners, an inner light/dark bevel, label separators, small corner studs, grip grooves and inset preview rims. Metadata remains outlined native pixel text.

Previews are no longer restricted by the native 63x42 card raster: the composed native UI is copied unchanged to physical display pixels, then each preview is sampled directly from the full 240x160 stored capture into its actual output rectangle. No font/vector/full-window redesign is used. GPU and software paths both display this result without guest UI overlays.

Saved Game's shared task4 is routed through its original Load branch exactly once (selected byte manager+225D=1). The original handler schedules task19 then the file selector. The custom cards remain visible across that entry. Back from the selector routes through the original cancel branch and returns to the title, preventing a reopening loop. All manual file loads still use the verified native confirmation path.

Validation: eleven autosave tests pass, including physical-output preview sampling from adjacent original pixels. Native replay with no user input after Saved Game opens the custom file selector automatically; direct File1 load reaches the world map at32:01. Separate Back replay returns to title callback081404E1 (entry state19, cancel state18). Proof image target/save-cartridges-proof.png; logs target/route-cartridges.log and target/route-cartridges-back.log.

## System > Save and entry-frame ownership
The custom cards now own all visual phases of both original EUR callbacks:0813DD75 (load) and0813D049 (save), including initialization and fade/exit. Entry routing is also processed immediately after emulation and before presentation, closing the one-frame gap that exposed the old menu.

System > Save uses its own native state8 handler, not the state11 load handler. Selecting File1/File2 invokes the corresponding original save confirmation and normal write path. Auto Save is read-only in this context so clicking it cannot accidentally restore an old state. The caption changes to Pick slot to save. State17 preserves the game's original overwrite Notice, including its language-dependent bounds and existing keyboard/gamepad/mouse confirmation. No guest save bytes are reconstructed by the host.

Cartouche rims now have continuous clipped-corner outlines, coherent bevels, a footer divider and a slightly more opaque body to reduce competing backdrop patterns. Full-resolution preview composition is retained.

Native System > Save replay: frame5900 state8, direct File1 event at6000 enters state17 overwrite confirmation; selecting Yes advances header generation26 to30 and returns to state8. The replay uses the SRAM in memory and does not write the source .sav. Log target/system-save-direct.log.

## New manual state slots (replaces native cartridge writes)
Runtime now stores Saves/QuickSave, Saves/Save1 and Saves/Save2 in the game directory (repository root during development). FFTA_SAVES_DIR overrides the directory. Normal frontend cartridge flushes on timer, exit and Switch ROM were removed; the original .sav is imported/read only.

Save1/2 persist full machine states from the unobstructed world map. A map capture refreshes each second and immediately before Start is supplied to the guest. Verification requires world callback08031601, the AA03/AC02 BG pair, world task state0 at EWRAM1C3C4 and an actively owned map cursor derived from IWRAM2810. BG identity alone was insufficient: it can remain behind System. This rule is independent of mouse mode.

Opening System > Save retains the themed cards but clicks write the cached map state asynchronously without invoking0813D049's save branch or cartridge overwrite prompt. Successful write feedback appears only after fsync/rename completes. File loading restores the state and reinitializes host audio/input/UI caches just like QuickSave. Auto Save stays read-only in the manual Save selector.

Each mission picks a candidate illustration after a varying10?20 seconds in its terrain, excluding a visible dialogue. It is independent of the hardware framebuffer in the state. It remains available on the map and is replaced by the next mission's selected midmission frame; it is bundled with new QuickSave/manual files. Without a mission image yet, map screenshots are used. Restoring Save1/2 always uses the map hardware state, regardless of the illustration.

Bundle format FFTASV02: state length, image pixel count, SHA256 of state+RGB555 picture, then the unchanged full-machine state and optional240x160 picture. Bounds/checksums and inner state are checked before restoration. Per-file .tmp and .bak names are independent; corrupted primaries fall back to the last complete backup. Existing automatic states are copied/migrated into the new QuickSave when no newer QuickSave exists.

Legacy File1/File2 remain loadable until each corresponding new slot exists. Loading a legacy slot queues automatic conversion at the first clean map. Original EUR header tags0/1 at offset10 identify which slots need preservation. The .sav is renamed to .sav.legacy-backup only after all present normal slots have valid new files. Backups are never deleted, and path discovery recognizes the archived logical path. User-requested manual overwrite of a slot explicitly replaces that slot; other original slots remain accessible until converted.

Native replay: Save1 requested from System at frame5900, written as a681815-byte bundle. Restoring at6400 resumes the verified clean map at frame3701, callback08031601. Image target/state-slot-restored-map.png verifies no menu is present. Tests use target/state-slot-native-clean-proof; original E:/AI/ARC/ffta.sav remains unchanged. Unit tests cover migration, independent image/state, corruption recovery and exact pre-menu capture.

This implementation is in the desktop frontend. Android's existing app has not been rebuilt or migrated by this change; its eventual storage must use filesDir, not evictable cacheDir.

Final validation:19 autosave tests plus2 save-path tests pass (21 total), including the midmission picture's exclusion of opening frames and retention across the world-map transition. Runtime save files and legacy backups are ignored by Git. Windows binary target/release/FFTARecomp-StateSaves.exe.

## Save-and-return and precomputed map entry
Manual saving now uses save_and_return: enqueue the already prepared state, wait for successful durable publication, then restore that exact slot through the standard host state-load cleanup. This returns immediately to its unobstructed world map instead of remaining in the file selector. Failure leaves the player in the selector with an error; background legacy conversion uses save_manual and never triggers this return.

The map state is now captured once per transition into the clean map, not once per second and not when Enter opens a menu. Re-entering the world after a shop, mission or menu refreshes it. The Save click does no new state serialization. The independent mission screenshot and card publication remain atomic with the slot write.

Validation:21 autosave tests pass, including a real worker write followed by automatic return, failure/import non-return, and cache identity across elapsed map time and Enter versus replacement on the next map entry.

## Additional slots and carousel
Manual slots are now discovered dynamically from SaveN files (and valid backups) in Saves. A sparse ordered map avoids allocating up to the highest numbered file. Save1/2 retain legacy migration behavior; Save3 and later use exactly the same checksummed state format and save-and-return path.

System > Save shows manual cards and a blue New Save cartridge with a yellow circular plus button at the end. Activating it writes the next unused number above the highest discovered slot. After saving it returns to the map; the next visit presents another New Save card. The load selector lists QuickSave and every manual slot, without a creation tile.

Three cards remain visible at once. Left/right keyboard/gamepad navigation keeps the selected card visible; clicking a side arrow shifts the carousel immediately by one card. Only sides with off-screen cards draw/hit-test an arrow. Metadata and full-resolution preview positions map to the visible card rather than the underlying file number.

Validation:23 autosave tests pass, including dynamic Save3/Save6 discovery after restart, restoring the exact state from an additional slot, sparse numbering, plus-card context, carousel navigation, hitboxes and arrow endpoints. Visual proofs target/save-carousel-page0.png and target/save-carousel-page1.png.

## Animated arrows and save-only removal
Carousel arrows now alternate two authored pixel poses every18 emulated frames, with a dark outline and gold/pale highlight. Visibility and hit testing still depend on off-screen cards.

Only System > Save draws red trash sprites on manual cartridges; Load, QuickSave and New Save have no removal control. A trash click opens an icon-only modal: cross cancels, red bin confirms. Cancellation is selected by default; mouse input is consumed by the modal, and keyboard/gamepad navigation/confirmation cannot activate the cards behind it.

Confirmed removal moves primary and backup files into Saves/Deleted with unique timestamped names. A SaveN.deleted marker prevents a removed legacy slot from reappearing through the original .sav. Reusing that slot clears its marker after successful writing. The original cartridge save is never directly modified by removal.

Validation:25 autosave tests pass, including save-only hitboxes, default cancellation, recoverable primary+backup removal, restart behavior and subsequent recreation of a deleted slot.
