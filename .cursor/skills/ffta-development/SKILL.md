---
name: ffta-development
description: Develop FFTARecomp runtime, UI, input, rendering and recompiler features using the project's existing architecture and compatibility constraints.
---

# FFTARecomp development

Read `AGENTS.md` and `docs/development/PROJECT_STATE.md` from the repository
root. Inspect current code and Git state; historical docs include old build
flags and paths, so manifests and implementations resolve disagreements.

Before choosing a work unit, especially after compaction or goal continuation,
read `docs/development/TASK_HANDOFF.md`. Reuse completed infrastructure and
fixtures; an open PERF parent is not permission to redo completed substeps.
Select a remaining criterion with a concrete deliverable, and apply the
repeat/reopen rules in `docs/development/MEMORY_WORKFLOW.md`. Preserve rejected
experiments as decisions until new evidence justifies reopening them.

Resolve the latest user request before following an older next-action queue.
For a workflow-only request, update the workflow and preserve pending runtime
work; do not resume that runtime work merely because a summary lists it next.

Before mutating files or starting expensive work after resumption, record the
selection checkpoint in TASK_HANDOFF as defined by MEMORY_WORKFLOW. Check the
selected ID's existing outcome; completed/rejected work requires a recorded
reopening reason. Validate an already implemented request instead of doing it
again. Check linked terminal results before trusting an old next-action line;
close a completed/rejected selection before choosing another unit.

Choose the narrowest relevant area:
- `crates/recomp/src/play.rs`: desktop event loop, pacing and presentation.
- `crates/menu/`: menu navigation, capture state and drawing.
- `crates/input-config/`: persisted settings and backend-independent input.
- `crates/screen/`: presentation, UI layers and GPU/software equivalence.
- `crates/gba-core/`: guest execution, hardware, timing and save bytes.
- `crates/recomp-core/`: analysis, generated C, compiler backends and loaders.

For settings, connect storage/defaults, menu controls and runtime behavior.
For gamepad changes, inspect XInput, DirectInput and Gilrs/Linux paths;
capture must not also confirm a menu item or execute a game action.
Preserve individual game bindings when introducing a combo; distinguish
simultaneous chords from sequential presses and account for release state.

Keep guest timing and host presentation separate. FPS INT counts completed
emulated frames; FPS EXT counts successful submissions, not monitor scanouts.
Read the relevant feature doc before changing an existing hook or UI layer.

Save states use `crates/gba-core/src/save_state.rs`, separate from cartridge
saves and native translation caches. Keep decoding transactional; include new
mutable machine/hardware state in serialization and bump the state format
version for incompatible layouts or execution semantics. Never serialize host
pointers or ROM/BIOS bytes. Test deterministic replay and failed-load isolation.
Keep `.ffstate` files out of commits and packages. Slot persistence and preview
headers live in `crates/recomp/src/save_state.rs`; the controller-only picker
lives in `crates/menu/src/save_states.rs`. Preserve nine per-ROM generic paths,
legacy loading, atomic preview/state writes and safe overwrite confirmation.
Keep state-operation errors and success feedback inside the menu. The Slots
owner keeps one session undo entry; replace it only after a successful save or
load and consume it only after a successful undo. Undo save restores exact file
and preview bytes without rewinding gameplay; undo load restores the pre-load
machine with the same frontend reset as load. Preserve recovery after failures
and refuse detected external slot changes instead of overwriting them.

Cartridge backups are distinct from state slots: see
`crates/recomp/src/save_backups.rs` and `docs/save_backups.md`. Back up before
startup can write the selected `.sav`; publish a new complete raw copy before
pruning to two. Restore must prepare a valid clean boot and replace the live
file atomically, so stale RAM cannot re-save the old progress. Preserve backup
integrity/order checks, safe confirmation, and no rotation during restoration.

Use targeted behavioral checks. Shared display globals require serial core
tests. Use the Windows validation skill when building or packaging Windows.
Finish with the project-memory skill, recording only observed results and
preserving unresolved limitations.

Portable storage is selected by `portable.txt` beside the executable; see
`docs/portable.md` and `input_config::storage`. Route new mutable app data
through that root in portable mode. Never fall back to AppData or persist an
absolute bundle path there; preserve explicit environment overrides. Test a
folder move with unrelated working directory and keep private Data out of release archives.
Native Linux bundles use `scripts/package_ffta_linux.sh` and the shared portable
packager. Preserve executable tar modes, bundle `tcc/libtcc.so` with its source
and notices, inspect `ldd`/glibc requirements and distinguish Xvfb checks from
physical-controller and Wayland checks.

For containerized Linux builds use `Dockerfile.linux` and
`scripts/build_linux_docker.sh`; keep private game data out of Docker context,
keep build caches separate from host `target/`, and export artifacts without
bind-mounting the checkout writable. Containerized compilation does not bundle
the host GUI/runtime libraries; record the base and observed libc requirements.
