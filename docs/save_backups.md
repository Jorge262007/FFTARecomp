# Automatic cartridge-save backups

For the desktop portable editions, settings are under `Data/config` and the
cartridge save is `Data/saves/ffta.sav`, with backups and state slots beside it.
See [portable storage and migration](portable.md). Installed-mode paths below
apply when `portable.txt` is absent.


Each desktop game launch protects the currently selected **cartridge `.sav`**
before loading it into the machine or allowing gameplay/autosave. The two most
recent startup copies are retained. The first launch with an existing save
creates one copy; the next creates the second. Each subsequent launch creates
a new complete copy before removing the oldest one. Unchanged launches can
produce identical copies. No copy is invented when the `.sav` does not yet exist.

These are separate from the nine `.ffstate` slots and their one-step undo.

## Restore using the controller

Open the pause menu and choose **Save backups**:

- **Copy 1 - latest** is the newest startup copy.
- **Copy 2 - previous** is the preceding startup copy.
- Empty, damaged or incompatible copies cannot be selected for restoration.

Use Up/Down and Confirm (A/South on the standard gamepad layout, Enter on the
keyboard). Restoration asks for confirmation inside the menu and initially
selects Cancel. Choose **Restore and restart** explicitly. Back/B/Escape cancels
without changing the current save.

Restoration atomically replaces the selected `.sav` and resets the emulated
machine to its startup state with that cartridge memory. The game remains
paused; return to the main menu and choose Resume to boot from the restored
save. Current session progress is replaced. A subsequent autosave or clean exit
uses the restored machine, so old RAM cannot immediately overwrite the recovery.
Window, bindings, video/audio settings and the native translation remain in use.

Restoring does **not** rotate the two backup files or launch another OS process.
You can try either copy in the same session. A later normal launch creates its
usual startup backup and rotates the set. Save-state slots stay on disk, but
the session's save-state undo entry is cleared by the game reset. Restoration
is refused while input recording is active, since its linear format cannot
represent the reset.

## Files, integrity and failure behavior

Backups are scoped to the selected save file, alongside it:

```text
<save-directory>/SaveBackups/<save-filename>/
  backup-<20-digit-sequence>-<SHA-256>.sav
  backup-<20-digit-sequence>-<SHA-256>.sav
```

Names encode order and integrity without an extra index file; the UI displays
the friendly Copy 1/Copy 2 labels. The copies are raw cartridge bytes and can
also be copied/renamed manually when recovering outside the application.
Different selected save filenames have separate folders. A missing remembered
or ROM-named primary file keeps its location when recognized backups exist,
so recovery does not silently switch to another `.sav` in the directory.
`FFTA_SAVE_PATH` and
the normal save-path resolution still decide which save is protected.

The runtime loads exactly the bytes captured at startup. The new copy is fully
written and synced before publication, then older recognized backup files are
removed. Failed backup creation aborts startup before gameplay can save over
unprotected data. Unrelated filenames are left alone. A crash between publication
and cleanup can leave an extra copy; successful startup cleanup returns to two.
A cleanup error is reported rather than silently deleting other files or running
without completing the requested protection.

Restoration checks the selected generation, content hash and cartridge-memory
size, and prepares a valid clean-boot machine before changing the live file or
machine. A missing/changed/damaged copy, bad boot snapshot or failed write leaves
the running machine and current `.sav` untouched. Feedback stays inside the menu.
Hashes detect backup-file damage; they do not validate game-specific save checksums
or prove that the original progress was valid before it was backed up.

The reader bounds cartridge saves to 128 KiB; FFTA's tested EUR Flash saves are
64 KiB. Existing backups and the primary file remain untouched if a larger source
cannot be protected. When filesystem problems prevent launching the game, the raw
backup files can still be recovered manually if readable.

No ROMs, personal saves or backup files belong in commits or releases. Existing
`.sav` ignore, packaging and audit rules also apply to this backup directory.

## Validation

Windows GNU release under Wine: **33 menu + 42 runtime tests passed**. Coverage
includes two-copy retention, exact raw-byte capture, per-save isolation, safe
confirmation with a pinned generation, main/video layout distinction, clean-boot
restore and subsequent normal save, damaged/missing copies, a blocked atomic
publication, recovery of a missing primary, and preserving old copies/unrelated
files after a failed capture. Five memory-tool tests, development skill validation
and whitespace checks also passed.

The controller-only simulated-XInput smoke exercised three starts, first/second
copy creation, default Cancel, restoring the older save, clean exit without stale
RAM overwriting it, and retention of exactly two copies on the third start.
Both remaining file hashes matched their encoded checksums. A separate final-build
smoke recovered a missing primary from existing copies and preserved it on exit.
Logs/screenshots are under ignored `target/save-backups-*` paths. These are Wine
and simulated-controller checks; native Windows and physical controllers remain
unverified. AndroidPort remains unchanged.
