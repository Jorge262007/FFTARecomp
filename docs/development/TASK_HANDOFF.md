# Task handoff

## Operational status

Updated 2026-11-01. Latest request: continue the whole-project English-first
goal. Selected `ENGLISH-FIRST-PROJECT`. The intervening Android request was
already implemented and verified read-only; it did not advance this goal.
Existing policy, guide translations and compatible memory-parser edits are
partial work to validate and continue. No accepted ROM/runtime build resumed.

Read this entire file and its final handover, then only the relevant sections
linked below. Do not automatically load the archive, full CHANGELOG,
PROJECT_STATE or large TODOs. Accepted evidence remains valid even when local
artifacts have been retired. Rules: [MEMORY_WORKFLOW](MEMORY_WORKFLOW.md#resume-and-repeat-decisions).

Execution state and acceptance outcome are separate: `active`, `suspended`,
`interrupted`, `failed`, `waiting`; open outcomes are `partial`,
`integrated_unverified`, `not_started`. Completed/rejected tasks move to the
archive after acceptance checks; an open parent never reopens closed children.
Here `suspended` means outside this session's selection, without authorizing
termination of another owner's process or changing another thread's goal state.

## Running or suspended tasks

### ENGLISH-FIRST-PROJECT

- State: active
- Outcome: partial
- Verified: English-first policy/rule and Android/menu/Docker/private-assets guide translations are in the working tree; parser accepts English and legacy memory fields, with 11 focused tests passing. Initial inventory found 53 document candidates and text in 18 Rust/7 Python/8 HTML/JSON/TSV files; it is heuristic, not a completeness proof. Existing Docker scripts/APK hashes were verified without rebuilding.
- Missing: translate the remaining maintained documentation across both trees, default UI/console/report/script/comment text, and complete an audited inventory. Preserve optional translations and immutable historical evidence. No scope reduction to the initial guides.
- Resume: verify the English handoff retains task IDs/states/evidence/limits and unchanged archive hashes; continue Android technical guides and remaining documentation/runtime surfaces in reviewable groups, recording acceptance and missing criteria.

### ROMDATA-03-04-E105-SENARY-COMPACT-OBJ

- State: active
- Outcome: not_started
- Verified: source and 756-byte sparse model PASS under Wine, two spans and gap2; accepted e105 dataset has 95 resources.
- Missing: publication, loader authentication and runtime composition preserving gaps, inputs and previous resources.
- Resume: integration and tests written; isolated Windows build recorded as active in session 51589, logs in `target/rom-data-compact-obj-senary-runtime/`. Check its current handle/output/owner before acting; if PASS, use the prepared Wine test. Fallback81 verified statically, gap preserved.

### ROM-DATA-FULL-EUR

- State: suspended
- Outcome: partial
- Verified: e105-d104, 95 resources/96 files; CLI create/reuse/startup and 98 protected files PASS in `target/rom-data-compact-obj-quinary-cli/result.json`, read during the audit. Neutral 24755→24756 with new counters zero does not prove ordinary use. Do not repeat accepted source/model/runtime/CLI checks or closed negative attempts.
- Missing: unmigrated resources/consumers, missing ordinary-use evidence, ABI/global extent/indirect calls and mod phases ROMDATA-06/07; Linux/USA/native Windows/new32:9 remain distinct. EUR, reduced Wine scope, no performance/space measurements; originals and personal saves unchanged.
- Resume: [ROM_DATA_TODO](ROM_DATA_TODO.md), only the selected criterion's section (ROMDATA-01/02/03/04/05/06/07); [inventory](ROM_DATA_INVENTORY.md). Do not read the whole TODO. The following children retain their IDs and limits, with no new attempts selected here:
  - `ROMDATA-04-PARAMETER-RECORDS27-ORDINARY`: source/model/runtime/CLI accepted; 59 records observed, full domain unproven; positive normal use and complete paired evidence missing. Eleven negative captures closed; archive legacy lines 17624..17672 and `docs/rom_data_gameplay.md`, ordinary sparse parameter record reachability section.
  - `ROMDATA-04-LOCALIZED-TABLE-GRAPHICS-ORDINARY`: 40 banks/8 tables source/model/runtime/CLI accepted; positive normal trigger and paired evidence missing; negative attempts remain closed. `docs/rom_data_ui_graphics.md`, ordinary table extension reachability section; archive 17844..17864.
  - `ROMDATA-05-DYNAMIC-RAW-ORDINARY-REACHABILITY` and `ROMDATA-01-LOCALIZED-RAW-DYNAMIC-DOMAIN`: conditional fragments/paths accepted; global domain and ordinary invocation unproven. Unit320 negative closed; later state2/state4/selector-witnesses diagnosis closed without a scene. Archive 59..75 and 789..866; no blind navigation.
  - `ROMDATA-01-OBJ-REMAINING-DRAW-ROOTS`, `ROMDATA-01-OBJ-REMAINING-CONSTRUCTOR-SOURCES`, `ROMDATA-01-OBJ-GRAPHICS-PAYLOADS`: authenticated increments accepted; global inventory/remaining consumers open. Do not repeat the broad experiment with 61 truncations. Archive 2853..2874, then only the selected family's record.
  - `ROMDATA-05-OBJ-VALIDATION`: local equivalence/costs, GUI, archive and USA initializer have later accepted outcomes; native Windows gates, further catalogs/scenes and normal enablement remain. Archive 3945..4087. Old artificial RSS/space constraints and `ROMDATA-05-BG-SCENES-COSTS` do not authorize further measurements: superseded by EUR/no-metrics steering.
  - Compact OBJ e101..e105: conditional prefixes/model/runtime/CLI accepted; ordinary use still open. Record222/candidate-table and selector33 have only static conditions and bounded negatives. Archive 11..65; do not infer a scene from its name.

## Incomplete tasks and open criteria

DBG2 audit of 2026-11-01: all eleven parents 13..23 remain unchecked;
8 have partial/integrated work, 3 lack the requested implementation.
The checks below predate this session and were inspected without rerunning.
Original criteria and audit notes: [DEBUG_TOOLS_V2_TODO](DEBUG_TOOLS_V2_TODO.md#p2--console-di-comandi-e-ricette-per-lassistente).
DBG2-01..12 remain complete within their scope. Read historical details only
for a specific question: archive legacy lines 179..744 or the named record below.

### DBG2-13

- State: suspended
- Outcome: partial
- Verified: typed parser, catalog/profile ID selection, cancel/deadline, retry, discovery and shared replay have Windows GNU/Wine evidence. `2026-10-31-prepare-debug-content-id-selection.json` and `2026-10-31-connect-shared-replay-commands.json` in docs/changes; profile-result and replay-panel/gui-result confirm selection/panel, without personal replay.
- Missing: full menu/CLI/console parity for validation, actions and results across all requested operations, beyond the selected subset.
- Resume: compare the contract and handlers only for the missing group in debug_command.rs/debug_control.rs/play.rs; do not repeat accepted profile selection, cancel or replay.

### DBG2-14

- State: suspended
- Outcome: integrated_unverified
- Verified: Unicode editor, history/selection, contextual completion, simulated XInput keyboard, focus/hide, isolated clipboard and multiline paste without submission accepted. `docs/changes/2026-10-31-begin-debug-command-console.json`, add-debug-command-context-completion and add-debug-command-controller-keyboard; clipboard/gui-result read and positive.
- Missing: real IME composition and relevant layouts; composition/capability guards unverified in GUI. Native Windows/Linux/physical controller unverified, distinct from accepted Wine evidence.
- Resume: select only the missing input criterion; editor/panel/completion and focus in play.rs. Do not recreate the clipboard check.

### DBG2-15

- State: suspended
- Outcome: partial
- Verified: JSON outcome/target/progress, ownership, cancel/deadline, idempotency, text retry, discovery and replay/panel checked in the October 31 records show-debug-command-job-context, classify-debug-command-local-errors, add-debug-command-retry and connect-shared-replay-commands. terminal-result confirms pending null; replay JSON copy matches.
- Missing: complete assistant procedure and full console API parity; cancel/deadline while replay actually executes guest frames (so far a synthetic slow child), slow live refresh and complete presentation in a narrow window.
- Resume: only the missing flow; do not reopen closed retry, wrong-ID, synthetic deadline or selection steps. A client timeout means unconfirmed delivery/outcome, not a stuck process.

### DBG2-16

- State: waiting
- Outcome: not_started
- Verified: DBG2-06 host runner/observational recipes and supervised replay exist; these do not constitute console batches with checkpoints.
- Missing: saved typed command sequence, pins, assertions/waits/caps, cancellation and resumption without repeating successful steps.
- Resume: depends on DBG2-15; no new development selected during the audit.

### DBG2-17

- State: suspended
- Outcome: partial
- Verified: readonly index with 341 entries, empty/explicit import, simulated menu/controller, metadata/provenance/master, EUR/USA hash context, local channel and guide/package exclusion. October 31 records index-local-cheat-catalog-readonly, browse-local-cheat-catalog-in-menu, describe-cheat-target-and-public-guide and expose-cheat-snapshot-local-control. menu-gui2 confirms 341 synthetic entries/import/exact state.
- Missing: runtime/verification index by ROM/device/decoder; summary/menu_entries still report decoder unsupported and no handler despite verified source-bound inspection/one-shot. Connect per-CH capabilities; seven static plans do not prove behavior/device acceptance.
- Resume: [cheats.md](../cheats.md#readonly-index-and-cli--dbg2-17-in-progress), debug_cheats.rs and contract; import/browsing are complete, not to be reimplemented.

### DBG2-18

- State: suspended
- Outcome: partial
- Verified: CodeBreaker subset with synthetic fixtures, explicit plaintext inspection, explicit seed, source/master/CRC; seven EUR priorities compared statically with mGBA. October 31 records inspect-assumed-plaintext-codebreaker, inspect-explicit-seeded-codebreaker and inspect-source-bound-codebreaker-masters.
- Missing: connect executable CH capabilities consistently with the index still fixed at unsupported (each CH must report what it can execute). The subset is verified; other opcodes/formats may remain rejected. A decoded master is not an executed hook; this limit is distinct from decoding.
- Resume: [cheats.md](../cheats.md#assumed-plaintext-codebreaker-inspection--dbg2-18-in-progress), debug_cheat_decoder.rs; do not duplicate the verified decoder.

### DBG2-19

- State: suspended
- Outcome: partial
- Verified: source/state-bound preview and manual one-shot in the bus of a paused interpreter copy, atomic refusals, retry without duplication; October 31 record apply-guarded-cheat-once. This is not periodic enforcement.
- Missing: distinct arm/maintain/disarm operations, deterministic timing, conflicts/overlaps and supported memory/backend coverage; I/O writes, native/ROM and mirrors unsupported.
- Resume: [cheats.md](../cheats.md#manual-one-shot-in-an-isolated-session--dbg2-19-in-progress); do not repeat accepted one-shot validation.

### DBG2-20

- State: waiting
- Outcome: partial
- Verified: readonly search/details/filters and prepare/apply-once commands exist; menu remains readonly, without implicit activation.
- Missing: values/ranges, groups/presets, toggles, disable all, active indicator, conflict checks and full controller flow. Depends on DBG2-19 and DBG2-15.
- Resume: only after the required dependency, without duplicating the catalog view or equating CLI one-shot with a complete Cheats page.

### DBG2-21

- State: waiting
- Outcome: partial
- Verified: CH-0321/0323/0324/0325/0331/0334/0336 decoded statically; CH-0334 changes two RAM values and reopening the seed restores exact state. `target/debug-tools-v2-cheat-eur-resource/result.json` read: true for two writes/restoration/unchanged originals, no private payload.
- Missing: at least one UI access and one resource modifier with gameplay effect, isolated activation/deactivation/restoration; combined CH-0321 is not simple HP.
- Resume: depends on DBG2-19/20; do not equate static RAM or a fresh seed with a reversible toggle or UI evidence.

### DBG2-22

- State: waiting
- Outcome: not_started
- Verified: generic catalog/decoder does not prove advanced-group effects.
- Missing: evidence for each selected CH affecting abilities/AP/rules/transitions, with before/after and persistent effects; Clear Mission note uncertain.
- Resume: after DBG2-21, with an explicit small group; no development or activation selected here.

### DBG2-23

- State: waiting
- Outcome: not_started
- Verified: play.rs refuses new recordings/saves after one-shot pending provenance; debug_recording.rs/debug_replay.rs do not implement the requested cheat contract.
- Missing: set/hash/parameters/order/frame/decoder in checkpoints/reports, Load/Undo/reset/ROM semantics, state-clock replay and incompatible-set refusal.
- Resume: after DBG2-19/21; refusal guard is protection, not completed provenance/replay support.

### DBG2-24

- State: waiting
- Outcome: not_started
- Verified: preceding foundations accepted; no recorded closure of parent 24, outside audit 13..23.
- Missing: named triggers; dependencies DBG2-05/15 and DBG-08/11.
- Resume: only criterion DBG2-24 in TODO section P4 upon a relevant request; no development selected now.

### DBG2-25

- State: waiting
- Outcome: not_started
- Verified: preceding foundations accepted; no recorded closure of parent 25, outside audit 13..23.
- Missing: observed values/differences pinned on screen; dependencies DBG2-03/15 and DBG-11/17.
- Resume: only criterion DBG2-25 in TODO section P4 upon a relevant request; no development selected now.

### DBG2-26

- State: waiting
- Outcome: not_started
- Verified: preceding foundations accepted; no recorded closure of parent 26, outside audit 13..23.
- Missing: timeline and exportable recipe; dependencies DBG2-11/16/23 and DBG-09/16.
- Resume: only criterion DBG2-26 in TODO section P4 upon a relevant request; no development selected now.

### DBG2-27

- State: waiting
- Outcome: not_started
- Verified: preceding foundations accepted; no recorded closure of parent 27, outside audit 13..23.
- Missing: targeted integration/regression matrix; dependencies on earlier criteria and distinct platforms/input paths.
- Resume: only criterion DBG2-27 in TODO section P4 upon a relevant request; no development selected now.

### DBG2-28

- State: waiting
- Outcome: not_started
- Verified: preceding foundations accepted; no recorded closure of parent 28, outside audit 13..23.
- Missing: guide, distribution and version compatibility; depends on DBG2-27 for distributed features.
- Resume: only criterion DBG2-28 in TODO section P4 upon a relevant request; no development selected now.

### PERFORMANCE-TODO

- State: suspended
- Outcome: partial
- Verified: infrastructure/replay/baselines and accepted PERF-03/05/06/08 optimizations; timer/audio, hardened frame boundaries, event allowance and selected timing checks have terminal results. Compact-ID/18-ROM-seeds/post-wait/row-bounds experiments remain rejected.
- Missing: rare original lag, real attribution/realtime overhead, hardware/native Windows/Wayland, physical lifecycle and PERF-01/02/04/07/09/10 criteria; PERF-11/12/13 conditional. `PERF-07-EXACT-ROM-HINTS` retains the old integration ID: metadata/cache, default/explicit equivalence, replay/GUI/export were not closed at that checkpoint. None of its execution selected now.
- Resume: only upon a performance request; [PERFORMANCE_TODO](PERFORMANCE_TODO.md#stato-dellesecuzione) for summary, archive legacy lines 4998..5049 and record 2026-10-21-preserve-native-event-boundaries for integration. Do not repeat accepted hardened/allowance/timing checks.

### DEBUG-SAVES-EXPANSION

- State: suspended
- Outcome: partial
- Verified: accepted imports/copies/boot/scenes remain accepted; old inventory of 6 valid/9 partial/11 uncovered is not the current global state. Later increments and user DS-20 closed; a GUI interruption does not prove normal-close.
- Missing: only scene subcases still uncovered/partial in the current catalog; IN05 File2, IN19 EUR wrapper and IN18 conversion remain unaccepted leads to reconcile before acting.
- Resume: only upon a relevant request; docs/development/debug saves.md and local catalog, record 2026-10-23-audit-imported-debug-saves, archive 2551..2573 and 2643..2736. Do not recreate fixtures/imports or explore scenes now.

### LOCAL-ARTIFACT-HYGIENE

- State: interrupted
- Outcome: partial
- Verified: October 26 applied.json records applied cleanup; later sweeps have separate closed receipts. No deletion in this reorganization.
- Missing: original task's final validation/memory closure unattested; later cleanups do not automatically prove all its criteria.
- Resume: only upon a cleanup request, check task receipts/limits before any documentation-only closure; archive 2486..2496 and `target/local-artifact-cleanup-2026-10-26/applied.json`. No new sweep to reconstruct context.

## Blockers, dependencies and shared resources

- Async work: the collaboration inventory observed on 2026-11-01 showed only `/root`; ps visibility is limited to the sandbox namespace. This session has not started, terminated or signaled a build/Wine/runtime job. Host/other-thread state is unobservable: do not assert global inactivity. Historical results prove their children's closure, not today's host state. Before resuming, check handle/process, output and owner; no response or a lost handle does not establish failure.
- Shared resources: checkout, handoff/archive/CHANGELOG, canonical targets, Wine prefixes, display, datasets and fixtures. Do not stop others' processes, restore source/binaries without ownership or clean live/failed tasks. Historical Wine retention audit: deleting a prefix requires coordination/host visibility, not sandbox ps. Disk nearly full in an old sweep; current availability not measured here.
- Memory writes: one coordinated writer; proposals with both preimage hashes and common lock through scripts/task_handoff.py update. If changed, reread and merge before publication. No background writer; no lock authorizes interference with other jobs. Details in MEMORY_WORKFLOW.
- Complete history: [TASK_COMPLETED_ARCHIVE](TASK_COMPLETED_ARCHIVE.md#legacy-handoff-snapshot), a provenance snapshot, not an operational queue. Old partial descriptions remain to prevent information loss: open tasks remain here, not promoted to closed. `TASK_HANDOFF_MIGRATION.json` records sections, hashes and terminal-outcome index. Search an ID and read only the necessary range.
- Godot: planned assessment complete, implementation deferred, not a current dependency. AndroidPort is separate; originals/ROM/BIOS/private catalogs/saves/recordings stay out of commits/packages.

## Final handover

- Task: ENGLISH-FIRST-PROJECT, active whole-project English-first goal.
- Last verified result: existing policy and four translated guides inspected; English/legacy handoff parser passes 11 tests; Docker implementation and accepted APK hashes match.
- Remaining work: remaining maintained documentation and default program/tooling text, project-wide audit and memory publication.
- Errors or blockers: none; preserve optional translations, immutable archive bytes and unrelated task ownership.
- Next action: verify handoff migration preservation, then translate remaining Android technical guides; do not resume ROM work or repeat accepted builds.
- References: `docs/development/ENGLISH_FIRST.md`, `scripts/task_handoff.py`, `docs/menu_localization.md`, `docs/android_docker.md`, `AndroidPort/README.md`.
