# Nine desktop save-state slots

For the desktop portable editions, settings are under `Data/config` and the
cartridge save is `Data/saves/ffta.sav`, with backups and state slots beside it.
See [portable storage and migration](portable.md). Installed-mode paths below
apply when `portable.txt` is absent.


Open the pause menu and choose **Save states**. Everything happens inside the
menu, including feedback and overwrite confirmation; no file picker is needed.

- **Left/Right** chooses slot **1–9**, wrapping at either end. Confirm on the
  Slot row also advances to the next slot.
- **Up/Down** selects Slot, Save state, Load state, Undo last operation or Back.
- **Confirm** (A/South on the standard gamepad layout, Enter on keyboard)
  saves or loads the chosen slot. Saving an occupied slot opens an in-menu
  replacement confirmation, initially selecting Cancel. Choose Replace state
  explicitly to overwrite it. Empty slots cannot be loaded.
- **Back** (B/East, or Escape) returns to the previous page; from the overwrite
  confirmation it returns to the slot picker. Loading leaves the game paused.
  Return to the main menu and choose Resume when ready.

The selected slot shows Empty, Saved or Unreadable, plus a screenshot when
available. Success and errors appear in the same panel and need no OS dialog.
The menu-opening combination remains configurable under Input > Open menu.
Slots remember the selected number while this game session is running; a new
session starts at slot 1.

## Undo the last save or load

Choose **Undo last operation** in the same controller-operated menu. When an
undo is available the row identifies it, for example **Undo save (slot 9)** or
**Undo load (slot 2)**. It targets the last successful operation across all
slots, independently of the currently selected slot.

- Undoing a **save** restores that slot's exact previous file and screenshot.
  If the slot was empty before saving, it becomes empty again. Current gameplay
  is unaffected.
- Undoing a **load** restores the complete machine captured immediately before
  loading, including cartridge memory. It also discards gameplay since that
  load. The slot file stays unchanged; the usual audio/input/display reset is
  applied and the game remains paused.

There is **one level of undo for the current game session**. A new successful
save or load replaces the undo entry. Menu closing/reopening and ordinary
play do not clear it, while restarting/switching the game does. Undo consumes
its entry; there is no redo or older history. Cancelling a confirmation or a
failed operation keeps the previous undo available. With no entry, the menu
shows Nothing to undo.

Undo snapshots are held in RAM, not in extra backup files. Undoing a save first
checks that its current file still matches the version written by this session;
if a changed or missing file is detected, undo refuses to replace/delete it and
keeps the recovery entry. An unsuccessful undo leaves it available for retry.
Undoing a load is blocked during input recording, just like an ordinary load.
Cartridge `.sav` persistence after undoing a load follows the normal autosave
and exit behavior described below.

## Automatic files and previews

The folder is relative to the **currently selected cartridge `.sav` file**:

```text
<cartridge-save-directory>/SaveStates/<full-ROM-SHA-256>/
  slot-1.ffstate
  slot-2.ffstate
  ...
  slot-9.ffstate
```

The folder is created on the first save. Names never depend on the ROM filename;
identical ROM content in the same save directory shares slots, while a different
ROM/region has its own subfolder. Changing the cartridge save directory also
changes where its slots are stored. There are exactly nine numbered slots per
ROM in that folder; there is no automatic rotation or deletion of older slots.

The screenshot captures the current visible game view, with zoom/letterboxing
and native UI, excluding the pause menu and FPS HUD. It is stored as a 240×160
RGBA preview **inside the same `.ffstate` file**, rather than as a separate PNG.
The preview and machine snapshot are committed together with a temporary file
and atomic replacement. An unexpected occupied slot is protected against an
unconfirmed write, even if it appeared after the menu read its status.

Files persist across restarts. The slot picker reads only their preview prefix;
it does not decode all nine complete machines just to show a thumbnail.

## Existing checkpoints and compatibility

Original raw `.ffstate` checkpoints remain loadable. To import one, copy it
into the ROM's slot folder as `slot-1.ffstate` (or another unused number 1–9).
An older file displays Saved - no preview; loading works as before. Saving into
that slot replaces it with the new preview-bearing format after confirmation.
Existing files elsewhere are not moved or deleted automatically.

The slot envelope is `FFTA-SL1`: fixed-size screenshot, screenshot checksum and
length-delimited original machine snapshot. The inner state remains format
version 1, bound to exact ROM/BIOS SHA-256 hashes and protected by its checksum.
Bad/truncated previews, damaged state data, wrong ROM/BIOS and incompatible
versions are rejected before changing the live machine. A valid thumbnail alone
does not establish that its machine data can be loaded.

Compatible builds can share files with matching ROM/BIOS and formats. Changes
to incompatible machine layouts or execution semantics require incrementing
`VERSION` in `crates/gba-core/src/save_state.rs`; incompatible preview-envelope
changes need a new slot magic/version. This UI/storage change does not alter the
guest/native ABI or require a translation-cache revision bump.

The timer/audio event-order correction keeps the existing state schema. Old
version1 slots were verified through load, Undo and reload; subsequent audio
uses the corrected scheduler and need not match an older executable's output.

## Cartridge saves and restored state

Normal `.sav` files also have [two automatic startup backups](save_backups.md),
restorable through Save backups. They are independent of these nine slots.

`.ffstate` files are separate from cartridge `.sav` saves. They cannot be loaded
directly on a GBA or another emulator. For transfer, load the slot, resume, save
using the game's own command and transfer the resulting compatible `.sav`.

Loading restores cartridge memory too. Normal autosave or clean exit later
writes it to the **currently selected `.sav`**. Keep a separate copy of that
file if you want to preserve progress from both timelines.

The snapshot includes CPU registers/banks, RAM, IO, VRAM, OAM, cartridge backup
bytes and protocol state, timers, DMA, interrupts/sleep, video timing,
framebuffers, sound FIFOs, PSG and enhanced audio engines. Host sprite,
movement, stage and dialogue rendering state is included. RTC protocol and
offset are restored; a live clock still follows host time.

ROM/BIOS bytes, native code, OS resources, diagnostic logs and pending mouse
requests are not stored. Loading cancels pointer requests and resets decode
caches, audio queues/resamplers, FPS sampling and temporal display history.
Window/settings/bindings/zoom remain session settings. While paused after a
load or Undo load, the restored native 240×160 image is centered with black
margins. The saved wide scratch image may have a different stride from the
current zoom, so it is not used as a preview. The first completed game frame
rebuilds the extended view and separate UI layers; preview generation does not
advance or modify the restored machine.

Loading while input
recording is active is refused inside the menu, since its linear recording
format cannot represent a rewind.

State files contain game memory and screenshots. Git ignores `.ffstate`; source
packaging and release auditing exclude it. No personal checkpoint ships in a
release archive.

## Validation

Slot tests cover all nine generic paths, ROM isolation, persistence into a fresh
machine, no-clobber versus confirmed atomic replacement, matching previews,
legacy loading, missing/damaged files and failed-load isolation. Menu tests
cover wraparound, empty-slot feedback, controller actions, safe confirmation
and preview/mouse geometry from 240×160 to 3840×2160.

The underlying machine format was previously checked with 163 serial Linux core
and 163 Windows/Wine core tests. Its optional native replay compared 120 frame
hashes and complete final state after rewind and in a fresh machine, including
enhanced audio taps. This slot change does not modify that core format.

For this slot change, Windows GNU release tests under Wine passed: **29 menu +
31 runtime tests**. The interactive smoke used simulated XInput only, including
menu opening, selecting slot 9, saving, preview display, cancelling and confirming
replacement, loading, empty-slot feedback, wrapping to slot 1 and loading after
restart. Both game processes resumed and exited cleanly. Only slot 1 and slot 9
were created in the isolated test save folder; preview checksums and declared
state lengths matched. Native Windows and physical controllers remain untested.

Five project-memory tests and the updated development skill validator passed.
Logs and screenshots live under ignored `target/state-slots-*` paths. The root
workspace is updated; the separate AndroidPort tree is unchanged.

### Undo validation

Windows GNU release tests through Wine passed: **30 menu + 36 runtime tests**,
plus the optional native replay. Undo tests cover restoring empty/occupied slots,
exact original file/preview and legacy bytes, complete pre-load machine recovery
after further play, last-successful-operation ordering across slots and operation
kinds, no redo, failed-operation retention and refusing detected external file
changes. The native test also checked 120 subsequent framebuffer hashes and the
complete final machine after undoing a slot load, with enhanced audio taps on.

The controller-only simulated-XInput smoke created slot 9, undid its creation,
repeated undo safely, replaced the slot after advancing gameplay, restored its
original file hash, loaded and undid that load, then resumed. A new process
retained the slot but showed Nothing to undo. Both game processes exited cleanly.
Screenshots and logs are under ignored `target/state-undo-*` paths. These remain
Wine/simulated-controller checks, not native Windows or physical-pad validation.

## Diagnostic headless replay

`recomp runc <rom> --state <slot.ffstate> --frames N --input <replay.input>`
resumes the slot without writing it. It shares the desktop slot validator and
also accepts legacy raw snapshots. Do not combine it with `--sav`. Frame count
and input/zoom schedules are relative to the loaded point; guest clocks remain
unchanged. Native code must already be built for the matching ROM/BIOS. See the
[performance workflow](development/PERFORMANCE_WORKFLOW.md) for controlled
comparisons and local-only fixtures.
