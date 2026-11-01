# Current development knowledge

This file stores durable facts and entry points. Chronological changes and their
validation belong in [CHANGELOG.md](../../CHANGELOG.md) and `docs/changes/`.
Read current source and manifests before relying on older build instructions.

Task selection starts with the bounded [TASK_HANDOFF.md](TASK_HANDOFF.md) and its
final **Final handover** section, Git state and only task-relevant evidence/sections
of this document. Do not load full PROJECT_STATE/CHANGELOG/TODOs or history at
startup. [TASK_COMPLETED_ARCHIVE.md](TASK_COMPLETED_ARCHIVE.md) preserves the
original history exactly; consult only a named ID/section when needed. Open
parents do not reopen accepted children. The [memory workflow](MEMORY_WORKFLOW.md#resume-and-repeat-decisions)
defines explicit task states, pause/async handling and locked preimage-checked
publication; `scripts/task_handoff.py check` enforces structure and preservation.

## Architecture and current scope

- The [English-first policy](ENGLISH_FIRST.md) applies to maintained documentation,
  comments and default product/tooling text in both source trees. Optional locale
  catalogs retain their languages; English remains the default/fallback. The
  operational handoff now publishes English headings/fields/states, with legacy
  Italian parsing retained for lossless migration. Its immutable historical
  snapshot remains byte-identical. Whole-project translation is in progress;
  the [audit ledger](ENGLISH_FIRST_AUDIT.json) records reviewed files and pending
  scope without treating heuristic searches as proof of completeness.

- The readonly [cheat catalog](../cheats.md) has an explicit plaintext or
  caller-supplied seeded inspection for a bounded CodeBreaker subset. A
  source-bound plan can be manually applied once in a paused isolated interpreter
  after a state-bound preview and source/ROM/checksum recheck. Explicit source/master inspection decodes master metadata and
  can compare its checksum against supplied ROM bytes; seven priority EUR
  entries have static source/CRC and mGBA comparison evidence; CH-0334 also has
  a bounded isolated RAM before/after and fresh-seed restoration proof. Device
  behavior, native hook handling and gameplay effects remain unverified.
  Maintained cheats, conflict handling and provenance-aware replay remain open.

- The explicitly requested [private debug-assets export](../private_debug_assets.md)
  builds a separate ignored Windows portable ZIP from the user's selected local
  catalog/saves/snapshots, verified source manifest and completed recordings.
  EUR working slots are installed in its own Data directory; USA material is
  retained as pre-existing and unverified. ROM/BIOS and unrelated target trees
  are excluded. Ordinary release packages remain private-data-free.

- [Local artifact retention](LOCAL_ARTIFACTS.md#bounded-retention-policy) treats
  old datasets and isolated Wine prefixes as reviewable generated artifacts,
  not permanent inputs. Prefix cleanup must not follow Wine's links into host
  folders and requires host-visible or coordinated evidence that no user is
  active; a sandbox-only process listing is insufficient.

- The desktop [Clan Atlas menu](../menu_clan_atlas.md) uses the existing native
  Canvas/tiny-skia fonts with a permanent left sidebar (eight Game/eight Debug
  sections), fixed shell and internal list/detail scrolling. Legacy handlers,
  confirmations, DS/content selectors and 9/99 slots remain in use; the readonly
  operation and 28-task references do not execute CLI strings or future tools.
  DBG2-01..12 retain their current completed status; DBG2-13/14 are in progress.
  `ff.percent` adds100..800% in20-point steps while preserving legacy
  `ff.speed`2/4/8. The rational frame batch clock preserves each complete guest
  frame/clock; fast audio uses fractional playback ratios. Menu-only shoulder
  chords preserve preferences and yield to binding capture/text editing.
  Debug UI is hidden by default (av.cfg debug=false). Up Up Down Down Left Left
  Right Right in the open menu reveals it and persists debug=true; direct config
  editing works on restart. Debug-only save options and planned controls are
  hidden too, while stored preferences and separate CLI/session guards remain.
  Capture/editors/dropdowns and pointer navigation cannot complete the sequence.
  Directions navigate; Confirm opens a pending value, Confirm applies and Cancel
  discards (AV, slots, catalog filters, host choices and input-device tabs).
  Left at the content edge reaches the sidebar; Right enters a section.
  The sidebar cursor has opaque brass fill/dark text/marker distinct from the
  active page. Hints and the menu binding follow the last intentional keyboard,
  mouse or controller activity, independently of the persisted gameplay profile;
  connected/unchanged held pads do not steal keyboard hints. Xbox labels follow
  the current shared controller-label policy. Keyboard hints show actual menu
  navigation rather than advertise nonexistent shoulder-key equivalents.
  Video exposes persisted menu.transparency0..1 (default0.1; explicit old values
  retained), affecting background fills while text, borders and previews retain their foreground.
  System persists menu.language by stable code (English default, Italian included).
  One JSON per language in crates/input-config/languages supplies code, native
  name and messages; builds discover/register/embed all files automatically.
  scripts/menu_language.py create/check supports new languages without Rust edits,
  with English fallback for missing keys and build-time metadata/template checks.
  Each menu owns its locale; the generated catalog translates native UI copy before
  layout while config tokens, bindings, imported content and diagnostic payloads
  retain their identity. ROM language is independent; see [menu localization](../menu_localization.md).
  Complete selected values are available through readonly Details/F1/R3;
  capture, editors, dropdowns and busy host owners keep priority. Native font
  scaling follows HD/4K, with readable nested catalog/selector/tool controls.
  Logical Fast Forward ON at100% still applies the selected FF audio mode.
  See the programmatic [migration inventory](MENU_ATLAS_INVENTORY.json).

- The [local report homepage](../report_home.md) is generated as ignored root
  `index.html`, indexing existing coverage/comparison/graphics/other HTML indices
  with search, filters, relative links and file modification dates. Reusable
  generator/template and Windows/Linux launchers are versioned; Docker launchers
  reuse the existing coverage image with readonly checkout/network disabled,
  without extraction or runtime builds. Regenerate after report changes; browser
  reload alone cannot discover new local files. Catalog presence/mtime does not
  establish report validity or new scene acceptance. The template, generated group
  labels, search feedback and date presentation use English. Existing report
  titles and paths remain verbatim. Code/UI/audio/recent-event report templates
  likewise use English fixed UI; source-generated metadata is audited separately.

- The [local cheat reference catalog](../cheats.md) lives in ignored root
  `cheats/`, with a Markdown index and provenance-preserving JSON export for
  future work. Third-party source collections and their derivatives stay local
  while redistribution permissions are unverified. Regions/device formats and
  source-specific master codes remain distinct; static preservation checks do
  not establish runtime compatibility. The readonly `debug-cheats` CLI builds a
  bounded in-memory index with ordered entries and source metadata, disabled
  states and explicit unknown/unsupported/unverified compatibility. A separate
  readonly Cheat view reuses the content selector for explicit import, search
  and details; opening an entry never decodes or applies it. Exact known EUR/USA
  ROM hashes now provide source-declared region comparisons without treating
  them as code verification; device version stays unknown. The public portable
  whitelist includes the guide, excludes catalog payloads and ignores imported
  `cheats/` folders. The isolated local channel and command console can select
  a catalog by ID and asynchronously refresh a pinned readonly snapshot, then
  query bounded pages and UTF-8 detail chunks while another job is pending.
  Source verification is at refresh, not continuously. Cheat execution and
  verification remain unfinished.

- The [integrated debug follow-up](DEBUG_TOOLS_V2_TODO.md) is in implementation.
  DBG2-01 adds the readonly [operation contract](../debug_operations.md):
  `debug-operations list/show [--json]` initially inventories65operations/242
  parameters (67/250 after DBG2-02), current/planned menu routes, prerequisites/effects/results/caps,
  fixed/live menu bindings and remaining gaps. Stable IDs identify menu UI-trace
  actions and local-control pending/results without replacing existing handlers.
  DBG2-02 adds the readonly [content catalog](../debug_content.md), CLI
  debug-content scan/check and reusable bounded Index: declared app/portable or
  explicit roots,16content kinds, stable relative IDs, hashes/duplicate counts,
  availability and declared-only compatibility. 8roots/512entries/8192nodes/
  depth12/64MiBhash/8MiBmetadata, explicit truncation and stale/missing/type checks;
  listing never executes or replaces action preflight. Canonical parent/component
  guards also reject real Unix aliases hidden by Wine attributes. Wine synthetic
  9content+3registry tests/16CLI cases and real alias refusal pass without guest
  data/frames; nativeWindows/Linux runtime/controller checks remain unverified.
  DBG2-03 adds the reusable [internal selector](../debug_selector.md),
  explicit refresh/file/folder actions, stable guarded selection, Unicode search
  and controller keyboard, filters/favorites/recent/details and readonly snapshot
  preview. Wine GUI and simulated XInput pass; native platforms/physical pads
  remain unverified. Native-dialog return events are drained for400ms and two
  neutral polls after an observed synthetic Escape/resume regression; corrected
  GUI keeps full state/frame0/clock0 exact. DBG2-04 removes all3legacy
  frontend picker fallbacks; missing inputs open typed internal lists without
  automatic execution.12host input roles retain hashed/context-bound references
  and action rechecks; selected labels feed code tracing. Actual Wine GUI
  exercises missing/present/changed/incompatible routes and explicit dialog
  cancellation; runtime/library ABI remains worker preflight. Preparation of
  profiles/parameters is implemented by DBG2-05 via12tool drafts over shared
  CLI parsers, bounded scalar presets, readonly verification and unique pinned
  profile preparation with explicit execution. Exact menu/CLI parameter/result
  equivalence and controller punctuation are verified under Wine. Active UI
  preparation reads its own immutable source pins under its existing runtime
  lease; other active sources still refuse. DBG2-06 adds Scripts and tools with
  17typed input roles, declared homepage/campaign-report/artifact-preview adapters,
  explicit Python3.11+ selection, source/dependency hashes and cancellable capped
  process logs. Readonly status/inspect recipes and a separate bounded guest
  catalog link to the gameplay inspector without executing guest data. Shared
  operation registry now has97operations/329parameters. DBG2-07 persists relative internal/explicit external choice pins and scoped
  selector/history state with bounded metadata, async readonly restoration,
  missing/changed explanations and guarded atomic saves. Wine reopens the same
  seed after a Unicode/spaced portable move from unrelated cwd with exact
  state/frame0; restoring favorites/recent never starts a tool. DBG2-08 adds
  the [common event schema](../debug_events.md): shared host order/session/IDs,
  request/job context and one frontend fanout into UI/recent. Collector-local
  aliases stay compatible; supported/partial/missing producers and guest sorted
  aggregates are explicit. DBG2-09 adds bounded semantic menu property diffs,
  stable item/field IDs, actual dropdown/editor/binding/feedback values and
  input/handler causes; queued mouse context and existing handler counts stay
  intact. Unchanged hover emits no changes. Raw Winit button/wheel/IME hooks
  complement mapped input; raw Gilrs/DirectInput remain absent. Broader runtime
  and tool lifecycle hooks now publish bounded correlated DBG2-10 events, with
  actual coverage and missing instrumentation declared; future cheat hooks remain
  reserved. Wine validates current Clan Atlas recording/step integration. DBG2-11
  adds the optional [live event console](../debug_events.md#finestra-eventi-live--dbg2-11):
  bounded independent rolling window, view-only freeze/filter, visible losses,
  explicit copy/export and configurable poll coalescing; supported source and
  guest-frame aggregation limits remain explicit. Wine GUI/input simulation passes;
  DBG2-12 limits producer work to32events per frontend tick with explicit drops,
  bounds retained/search data and shares immutable frozen entries; its Wine
  synthetic load and off/on state/clock comparison pass. Restored snapshots with
  channel taps disabled use faithful host audio/pacing without rewriting state;
  short Wine queue observations pass, native/audible coverage remains open.
  DBG2-13/14 work now has a partial typed command parser, local-control adapter
  and initial command panel; its Wine synthetic step, focus-return and simulated
  XInput keyboard checks pass. Quote-aware option/ID completion and cycling
  have selected Wine GUI/model coverage. An isolated Wine clipboard check covers
  copy/cut, Unicode multiline paste, rejection on submit and copied result JSON;
  actual IME composition remains unverified. Wine also verifies CLI and console
  selection of synthetic replay profiles by ID without starting a replay;
  physical-controller checks and full equivalence remain open. See
  [the command operations](../debug_operations.md).
  A bounded Wine search also verifies targeted cancellation and a client/server
  deadline; terminal local-control replies no longer list the completed job as
  pending. Same-run explicit CLI and textual-console retries use bounded
  identity tracking; Wine confirms a console retry does not repeat a step.
  Mailbox inputs are atomically claimed before reading, so consuming an older
  request cannot remove a later publication; transient claim failures retry.
  Explicit authenticated content refresh and generation-checked paginated
  listing now allow Wine-tested ID selection without navigating the menu.
  The command panel shows verified session/ROM identity and throttled pending
  job parameters and progress; Wine verifies a search at minimum window size.
  Local parse, compatibility and availability failures now have distinct
  structured outcomes; transport failures do not claim delivery was prevented.
  The selected replay profile can be run in four modes through the shared
  command channel with worker progress, terminal result and request-ID cancel;
  Wine synthetic profiles verify routing and retry, while a deliberately slow
  synthetic child verifies supervisor cancellation and deadline handling. A
  separate Wine panel submission verifies replay and copied terminal JSON.
  Broader DBG2-15 result presentation and assistant procedures remain open.
  The broader checklist specifies built-in selection of existing
  inputs/profiles/scripts with external file dialogs only from explicit buttons,
  editable tool parameters, optional live event and textual command consoles,
  shared menu/CLI/local-control operations and an isolated cheat menu grounded in
  the private CH catalog. EUR candidates remain unverified; decoding, mutation
  semantics and replay provenance are prerequisites. DBG01-22 remain closed;
  DBG2 implementation does not reopen unrelated runtime work.

- The desktop debug-tools checklist DBG-01…DBG-22 is complete within its declared
  scope. Start with the [common portable guide](../debug_tools.md): recording,
  reproducible bug context, two selected runtimes, uncovered ROM reads and
  reviewed cleanup. Bundles include an explicit public script/template/doc
  closure and per-payload manifest with empty Data; prefix-rejection checks and
  compiler path remapping prevent builder-home leakage in the selected DBG22
  archive. Runtime/PS/Bash/offline launchers retain logs. Wine integration,
  Linux-host offline tools and public PS7.4 mocks are verified separately;
  native Windows/PS5.1, recent native Linux runtime and physical hardware remain
  unverified. Preserved divergences do not become parity acceptance. Further
  work requires a concrete missing criterion or regression, not reopening the
  completed checklist. See the guide and individual records for exact limits.

- [Shared debugging tools](DEBUG_TOOLS_TODO.md) is the completed checklist with
  stable DBG-01..DBG-22 IDs, separate from DS scene cases. It inventories existing
  tooling before prioritizing shared menu/CLI diagnostics, isolated sessions,
  snapshot/input replay, bounded ROM audits, coverage linkage and first-divergence
  reports. Each item has dependencies and acceptance criteria. DBG-01 is now
  implemented as [session diagnosis](../debug_diagnostics.md): Debug > Copy
  diagnosis and read-only `recomp diagnose` share a versioned JSON collector,
  build/file identities, effective storage paths and actionable preflight errors.
  Report refresh is bounded/atomic; snapshot checks use a disposable machine,
  while the menu observes live bytes/frame/clock without modifying guest state.
  Wine validation covers the CLI and menu/clipboard; native Windows and Linux
  remain unverified. DBG-02 adds [isolated portable sessions](../debug_sessions.md)
  via Debug > Start/Close debug session and `debug-session create/open/inspect`.
  A separate copied runtime owns its ROM/seed/config/save/dataset/cache, relative
  immutable input pins and exclusive lease; inherited project write overrides
  are scrubbed. The normal window stays paused with unchanged progress while
  the copy runs; menu return, load/undo/restart/backups and relocation pass Wine
  checks. Default creation stops at eight retained sessions without deleting them.
  DBG-03 adds the [shared save catalog](../debug_catalog.md): Debug > Debug
  catalog and `debug-catalog list/open/import` share region/status/kind/DS
  filters, recorded provenance/scope, file checksums and snapshot previews.
  Open creates a DBG-02 copy with a portable selected-entry catalog; import
  writes an exact checkpoint into a free isolated slot with Undo and explicit
  Load. DS/IN/GEN/variant identifiers stay distinct and recorded DS statuses
  are never promoted by checksum validation. DBG-04 adds
  [snapshot input recordings](../debug_recordings.md): menu Start/Stop and
  `debug-record start` capture exact initial state, explicit held keys at relative
  frame0, ordered input and a complete endpoint in a private session. Load/Undo
  load finish the capture and refuse the first rewind request. Incomplete or
  changed captures cannot pass `debug-record verify`; its bounded interpreter
  replay compares full serialized state/clock without a cartridge writer.
  Legacy RECOMP_RECORD_INPUT stays separate; its focused verifier stays interpreter
  only. DBG-05 adds [portable replay profiles](../debug_replays.md), CLI
  `debug-replay create/run/cancel` and menu Replay case/profile selection/cancel.
  Selected runtime, recording runtime, ROM/BIOS/dataset/input/endpoint identities
  stay distinct. A supervised worker uses no cartridge/slot/backup writer;
  progress/result JSON and exit codes separate pass, divergence, incompatible,
  timeout, interruption and worker failure. Replay never enables performance
  measurement or compiles a native translation. An explicit existing library is
  supported, but the old local EUR library shows a same-clock full-state
  divergence on DS20; its parity is not accepted or silently repaired here.
  DBG-06 adds [bounded ROM read audits](../debug_rom_audit.md), CLI
  `debug-rom-audit create/run/cancel` and menu Audit replay. Optional feature
  hooks retain old Capture/SiteCapture keys; the new thread-bound observer
  caps unique events and records actual typed/raw service plus interpreter
  scope. Presets, physical address filters and relative frame windows feed
  byte unions and identity-matched known-plan comparison, with explicit
  truncation, partial windows and uncovered hooks. Manifest reads share the
  existing 16 MiB project limit. Raw/typed DS20 replay preserves full state/clock;
  the two-frame inventory is12,416B with11,495B actually served typed using e76.
  Plan overlap and outside-plan reads do not establish new resources or DS
  acceptance. DBG-07 links those retained reports to the [ROM atlas](ROM_COVERAGE.md)
  with explicit readonly Docker inputs, per-audit selection, physical unions,
  resource/case/evidence drilldown and dated same-ROM generation comparisons.
  Positive-family extents, actual observed/Typed bytes and collector gaps remain
  distinct; imports never promote new DS acceptance.
  DBG-08 adds [trigger captures](../debug_captures.md), CLI debug-capture
  create/run/cancel and menu Capture on trigger, reusing isolated replay workers.
  Feature-gated first-hit observation covers authenticated resource source
  reads, verified conditional interpreter ROM instruction entry, or completed
  relative-frame events. It stops at the next safe completed-frame boundary,
  with explicit frame/timeout limits and distinct captured/not-reached outcomes.
  Bundles preserve executed origin/input buffers, identities, first-hit versus
  boundary metadata and a variant-only catalog; at most eight per profile,
  without overwriting earlier points or promoting DS status. Capture profiles
  are interpreter-only and separate from simultaneous audit inventories. Machine
  state/save/native ABI remain unchanged. Wine CLI/GUI/reload and raw DS20 checks
  pass; native platforms/controllers/typed dataset capture remain unverified.
  DBG-09 adds [first-difference comparisons](../debug_comparisons.md), CLI
  `debug-compare create/run/cancel` and menu Compare first difference. One worker
  steps two independent Machines from identical snapshot/input: interpreter/raw
  reference and interpreter/native candidate with optional authenticated dataset.
  Every completed frame compares full state, clock, central/wide buffers, OAM
  and exact audio bits/events. Readonly diagnostics name actual serialized CPU/
  Bus fields without changing save/native ABI. The first different boundary
  retains paired snapshots/images/audio and the prior matching point; matching
  or interrupted prefixes retain only metadata. Frames are bounded to 3,600 and
  reports to eight per profile. This is prefix evidence, separate from exact
  endpoint verification and instruction-level cause. The old native cache shows
  clock/r15 differences at frame1, with images/OAM/audio equal; its parity stays
  unaccepted. Wine CLI/GUI and offline browser checks pass without replacing
  normal runtimes; native platforms/controllers remain unverified.
  DBG-10 adds [controlled frame stepping](../debug_control.md) in isolated copies:
  F6 pause/resume, consumed F7/F8 one/ten-frame shortcuts, matching menu actions
  and acknowledged local `debug-control status/pause/step/resume` commands.
  `debug-session open DIR --paused` starts at the seed before any guest frame.
  Steps require pause, use neutral or explicit CLI keys and complete1..600frames
  before stopping; frame/clock/reason/remaining and readonly state hash are visible.
  Paused steps discard frontend audio, transitions invalidate DSP/queues and reset
  host pacing. Starting a separate replay is refused while a frame budget remains.
  Wine GUI/CLI proves exact menu/keyboard/CLI states and22/80frame recorded endpoints
  against continuous interpreter execution; native platforms/controllers/native
  stepping and physical listening remain unverified. No normal runtime replaced.
  DBG-11 adds [bounded interpreter probes and readonly inspection](../debug_probes.md):
  debug-control break/watch/inspect and menu Inspect PC / registers. PC/mode stops
  before an eligible interpreter step; read/write/value watch stops after the
  whole step including HLE/DMA/tick, excluding instruction fetches. Watch uses
  the audit feature; native searches are refused rather than claiming block-
  internal instruction granularity. Search is limited to5Msteps/600frames, neutral
  input and paused copies without recording/replay. Immutable buffer windows
  refuse IO/backup protocols and report registers, mirrors/physical ROM offsets,
  exact-buffer authenticated labels and source-plan owners, separate from typed
  consumption. No Machine/save/native ABI change. Local response outbox survives
  transient Windows replacement refusal without repeating the guest command.
  Wine tests/CLI/GUI verify PC and two IRQ accesses, readonly state preservation,
  default feature boundaries and interruption; native platforms/controllers and
  native search remain unverified or unsupported.
  DBG-12 adds the [offline graphics inspector](../debug_graphics.md), menu
  Capture graphics report and `debug-control graphics SESSION [--compare-last]`
  in idle paused isolated copies. At most eight private reports retain raw
  buffers, central/wide images, frontend CPU composition with/without UI and
  frozen BG/OBJ/OAM/palette/tile/window projections, without new snapshots or
  guest writes. Pixel attribution proves declared VRAM/palette backing only;
  final framebuffer winner, scanline history and ROM transfer source stay unknown.
  Previous same-session reports are hash-authenticated before raw byte deltas
  and paired images; different view/settings do not imply pixel alignment.
  DS20 frame22220 demonstrates wide-canvas padding versus actual UI bounds.
  Wine tests/CLI/GUI plus desktop/mobile browser checks preserve complete state,
  original inputs and normal runtimes; native platforms/controllers/native
  capture, other scenes and final GPU effects remain unverified.
  DBG-13 adds [menu/text/input diagnostics](../debug_ui.md), explicit Start/Save
  actions and `debug-control ui-start/ui-report`. Readonly `status.host_menu`
  identifies actual page/row/capture/dropdown geometry and option, avoiding
  assumptions about controller-selected tabs. A30s/1024event/600frame observer
  distinguishes physical keyboard/XInput samples, mapping, host dispatch,
  remapping/shortcut consumption and actual guest-frame latches. Disposable
  host-menu redraw records the existing font's advance/outline/width stop and
  canvas clip with identical observed/unobserved pixels; guest text stays unknown.
  At most8private reports, no collector snapshots or guest mutation; report
  presence is not replay/scene acceptance. Wine keyboard/autorepeat/simulated
  XInput and a closed2frame recorded CLI endpoint pass, with browser views and
  exact source-body preservation. Native platforms/physical pads and raw
  Gilrs/DirectInput/mouse stages remain unverified or uncovered. Homepage groups
  these reports separately; failed harness prefixes remain local diagnostics.
  DBG-14 adds [bounded audio diagnostics](../debug_audio.md): isolated Start/Save
  menu actions and debug-control audio-start/audio-report observe selected guest
  mixed/PSG/HLE/FIFO buffers before handoff and actual post-DSP/gain/native-format
  callback buffers. Producer decisions, mute/Fast, queue sizes and bounded reset
  contexts remain separate from physical delivery/listening, which is unknown.
  Preallocated callback storage performs no new IO/clock/allocation/lock; private
  WAV/raw/HTML/JSON output occurs outside its mutex. Limits30s/180frames/2MiB per
  source,3s/524288callback values/512blocks/64resets/eight reports. Live reports
  identify ROM/BIOS/runtime/build, seed/session pins, effective dataset/input/AV
  and actual keys; no new snapshot. Offline debug-audio states compares exact
  selected buffers in existing same-frame snapshots without guest execution;
  exit0equal/2diverged/3incompatible, local lossy previews and raw exact bits.
  A retained mixed difference at frame8514 is localized without replay; Wine
  menu/CLI and a complete4frame recorded endpoint pass. Native platforms and
  hardware listening remain unverified; no new ROM/DS/native/performance claim.
  DBG-15 adds [bounded code/label/dispatch diagnostics](../debug_code.md),
  menu Start code trace / Save code report and debug-control code-start/code-report.
  Pre-dispatch PC/mode, current readonly ROM/RAM bytes and entry disassembly join
  strict same-ROM declared labels and actual Native/Missing/System/Hook choices;
  interpreter-selected execution remains distinct from native lookup failure.
  No-op specialization preserves normal/cost dispatch and no guest/save/native
  ABI changes. Filters cover address/mode/kind, limits30s/60frames/512events,
  eight private reports/2MiBJSON, no new snapshot or automatic overwrite/pruning.
  Native table keys prove selection only, not block extents or extracted-code
  percentages; file identity read at trace start is not loaded-memory identity.
  A retained native DS20 prefix shows real native-to-missing transitions with
  exact observed/unobserved state/count parity using the same old library,
  separate from interpreter/native correctness. Wine menu/CLI/foreign-label/
  recorded2frame endpoint and offline browser checks pass; native platforms,
  physical controllers and live native GUI capture remain unverified.
  DBG-16 adds [local issue reports](../debug_issues.md), menu Create issue report,
  standalone debug-report create and paused debug-control issue-report. Readonly
  profile/pin/input preflight and retained result/comparison metadata produce
  HTML/JSON, reviewable summary and private attachment links without replay,
  payload copies or sending. Eight folders/2MiBJSON with writer lock and no
  automatic overwrite/pruning; incompatible preflight still publishes a report.
  Summary separates recorded frames, selected window, completion and first
  difference/components. Relative PowerShell/POSIX launchers use the pinned
  runtime/profile after a whole-folder move; browser copies the current command
  and opens local folders. Shareable-candidate metadata still needs review of
  symptom/case; old result freshness/settings and loaded native compatibility
  are not certified by report presence. Wine moved-folder SH replay reaches an
  exact2frame endpoint; GUI report leaves guest state/clock intact and browser
  clipboard/folder/error/comparison checks pass. Native Windows/PowerShell runtime
  and Linux-native execution remain unverified; earlier acceptance stays closed.
  DBG-17 adds [bounded task/script/unit inspection](../debug_gameplay.md), menu
  Start task / script trace / Save gameplay report and gameplay-start/report.
  Exact EUR catalogs authenticate descriptor loadsites, primary/aux cursor/
  dispatch/advance points and narrow unit graphics/sprite fields. Per-field
  register/RAM/ROM backing provenance, signed cursors and sentinel/unknown values
  remain separate from projection links and typed service. Addresses identify
  observations, never persistent unit/allocator identities or global domain caps.
  Shared readonly code/gameplay entry collectors preserve the ordinary step loop;
  native interiors remain unobserved. Limits30s/60frames/512events/1MiB event
  bytes/eight2MiBJSON folders, no new snapshot or automatic pruning/overwrite.
  Same-interpreter observer parity plus Wine menu/CLI coexistence preserves
  complete state;70observed events reconstruct a two-frame task countdown.
  That window has no script events, without implying global non-use. Browser
  sequence/context/resource filtering passes; native platforms/live native
  gameplay and physical controllers remain unverified.
  DBG-18 adds [bounded session/artifact retention](../debug_artifacts.md):
  preview/apply/keep-selection with HTML/JSON byte estimates and per-operation
  receipts. Preserve all session inputs/proofs/errors/pinned runtimes/datasets;
  select only generated native libraries and existing explicit completed ROM
  disposable copies. New session manifest attests directory-v1 coordination;
  atomic directory leases span creation/runtime/cleanup and coverage cache
  generation/read/prune across Python/Wine/container namespaces. Active,
  interrupted, legacy/unattested, failed, linked/shared or unexpected scopes
  remain protected; nested activity protects the entire ROM unit. No automatic
  stale-lease reclamation. Explicit historical cache reads coordinate via
  detected layout or --cache-lease; Docker keeps dataset mounts readonly with
  a separate coordination-root mount. Cache pruning retains current generation
  and receipts, stopping at64receipts. Manager scopes/entries/leaf-file counts/
  JSON and existing capture limits remain bounded. Root/staged Docker Python
  imports are complete; an optional Git-enabled offline-tools image and logged
  Bash/PowerShell launchers are provided without a new Cargo/ROM build.
  Synthetic Python lifecycle/error/interruption/concurrency, affected Wine
  session/replay tests and real paused Windows/Python lease contention pass;
  Docker keep/apply/stale-plan/readonly-reader and browser checks pass.
  Native Windows/PowerShell, Linux-native GUI and physical controllers remain
  unverified. Original runtime/input/proof preservation is independently checked.
  DBG-19 adds [recent-event rolling context](../debug_recent.md): opt-in shared
  menu/CLI Start, Freeze/Save and observed-error autosave. Keep actual input,
  partial readonly UI/signature changes,16selected ROM read keys/frame and actual
  dispatch/error metadata within1.024events/256KiB (8KiB/event). Continue rolling
  past prefix time limits; evictions/drops/lifetime counters and effective window
  remain explicit. No continuous snapshots or guest state/ABI changes. Reads
  unavailable/disabled/collision differ from zero; sorted aggregates are not
  within-frame chronology, native internals remain unknown. Start/Save hash
  full state at explicit boundaries only. Eight private2MiBJSON folders;
  save failure retains frozen context, repeated Save returns published path.
  Wine synthetic million-event/controlled-error, actual menu/CLI after31s,
  two-frame observer full-state parity and default unavailable/browser checks
  pass. Native Windows/Linux GUI and physical controllers remain unverified.
  DBG-20 completes the existing [local file control channel](../debug_control.md)
  with separate status/cancel query lock/mailbox/result, leaving operational
  completion independent. Cancel targets the exact pending request; deadline
  and20..120000msclient timeout distinguish unacknowledged waits from frontend
  budget stop. Check expiry between existing frame/search ticks, never interrupt
  guest execution mid-call. Ready/results identify manifest hash/label/run/request;
  wrong sessions/IDs refuse. Shared Pause boundary handles cancellation, no
  networking service or normal-game channel. Structured outcomes cover local
  errors, cancelled/timeout/closed; closed status can remain unknown/null.
  Legacy requests without optional identity/deadline remain decodable, while
  current CLI requires new capabilities. Wine protocol/frontend and actual
  pending-step query/cancel/deadline/readonly capture pass; native platforms
  remain unverified. Clock changes affect server civil-time deadline; no hard
  interruption guarantee for blocked/synchronous operations.
  DBG-21 adds [targeted campaign selection/reporting](../debug_campaigns.md):
  explicit case/DS/family/transition filters, pinned profile/result/audit metadata,
  positive/negative/incompatible/not-executed and reused evidence. No default
  sweep or payload opening; zero selected-window reads never implies global
  feature non-use. One-case explicit dispatch authenticates runtime/profile,
  records intent and execs existing supervised replay; old declared/undeclared
  result requires reopening reason.32cases/eightoutputs/2MiBreport, strict bounded
  JSON, coordinated attested readers and legacy exclusion unknown. Logged host/
  Docker-report-only entry points, homepage and limits integrate without Rust
  changes/builds.23Python checks, public fake dispatch, actual readonly reuse of
  two retained DS20 proofs, Docker/browser pass; no new guest frames or old
  acceptance replay. Windows/PowerShell/real profile dispatch remain unverified
  in this unit; existing replay supervisor evidence is reused.
  EUR extraction retains priority,
  Godot stays deferred and performance measurement stays separate.

- [Godot frontend assessment](GODOT_FRONTEND_TODO.md) is planning only,
  explicitly deferred until ROM extraction is finished and the user selects
  development. No prototype remains. Machine is reusable as a library, but
  full native dispatch and application lifecycle are still private binary
  modules; a shared runtime boundary is a prerequisite for native frontend
  parity. Do not infer implementation approval from this deferred TODO.

- [EUR audio source contracts](../rom_data_audio.md) identify the original
  M4A driver,3player configurations,640song-directory entries and455unique
  headers with explicit dummy-header/WaveData alias. A graph over1610track
  roots identifies390bank/program pairs,429tones,347keymap B and277PCM waves;
  a focused Wine source check confirms program/map selection and276interpolating
  endpoints plus one fixed-frequency endpoint. Their736-range/1941759Bsource
  plan now has a shared bounded producer parser and strict lossless typed
  tone/keymap/signedPCM/CGBmodel. One focused Wine decoder/independent hashes/
  alias-dependency check and2neworiginal CGBcopy checkpoints pass. Managed
  publication plus guest and MP2Kshadow-sampler consumption now pass in e42-d41:
  37files/36resources/all35prior decoded/e41retained, actual directory alias
  dependency. Two reduced Wine checks pass26guards/lifecycle/all1941759B/
  25original-interpreter-native fullstate-clock checkpoints/736edges-SWI-DMA;
  both actual MP2Khook/tick callsites read typed headers/samples for all277waves,
  nonzero bit-exactshadow audio and completehoststate-clock equality. Internal
  checkpoints remain partial. ProductionCLIcreate/reuse-no-write39managedfiles/
  oldcache and one400ordinary Winefullstate-image-audio pair pass516956guestB
  and3093208hostB(1056header/3092152PCM); CGBordinary0. DefaultCLIshadowoff is
  explicit; differential arms shadow after load. No newperformance measurement.
  No local640song or128tone guard is inferred. The strict15,236Bsource model
  and one focused Wine original-reader oracle pass640entries/nine targeted
  inputs/one raw lookup/six synthetic variants; unused bytes stay explicit.
  EUR e41-d40adds the typed audio directory:36files/35resources/457new source
  spans, all earlier decoded files/e40retained, USAe8-d7. Two focused Wine
  publication/authentication/lifecycle and44original-interpreter-native start
  paths pass with raw15236Baltered/fullstate-clock equality. ProductionCLI
  create/reuse/oldcache and one400-frame ordinary Wine fullstate-image-audio
  pair pass179newsourceB across directory/header/track fields (no player reads).
  Sequence source model now derives105253nonoverlapping instructions from
  1610roots/805ranges177047B, with canonical branch/pattern IDs and explicit/
  inherited opcode/typed operands.49clockB/36commandwords144B/CD8-9dispatch8B
  add3spans:808sources177248B. One focused Wine source/model check passes all
  independent fields/hashes/27guards and404new original-reader checkpoints;
  no fullsequencer/native/clock/ordinary claim for that source check. e43-d42now
  publishes sequences with actual directory+tonebank dependencies:38files37
  resources/all36prior/e42retained. Two reduced Wine checks pass25guards/
  lifecycle/all177248B/allinstruction-table masks and35original-interpreter-
  native fullstate-clock checkpoints,808edges-BIOS-SWI-DMA/generic-host cross-
  owner/rawfallback/liveRAM runningoverride. ProductionCLIcreate/reuse-no-write
  40managedfiles/oldcache and one400ordinary Winefullstate-image-audio pair pass
  819newB(366opcode/262operand/191clock),185source instruction records including
  lookahead;0command/extended/generic-host. No hostsequencer/wholeABI claim.
  Seven further driver tables370b20..370d04/484Bnow have a strict typed source
  model and oneWineoracle:180PCMkeys/12u32bases/12samplecounts/132CGBkeys/
  12i16bases/60noise/16volume codes.1138neworiginal probes pass unsignedmulhi/
  signedASR/truncation-clamps/sample0-13-15adjacent aliases/publiczero skip/live
  volume aliases; all484B read at observed original loadsites. Eight additional
  targeted gain returns correct the initial cap15inference: balanced liveinputs
  canyield16/23/31, while one-sided routing clamps15. No ordinaryreachability/
  global16-domain claim. e44-d43now publishes/consumes seven tables:39files/
  38resources/all37prior/e43retained, actualsequence boundarydependency.
  Two reduced Wine checks pass19guards/lifecycle/all484B/424entry bits and
  28original-interpreter-native complete-state/clock paths including mixed
  Thumb/ARM/aliases/BIOS-SWI-DMA/guest-host boundaries. No fullinit/mixerABI;
  productionCLIcreate/reuse-no-write41files/oldcache and one400ordinary Wine
  fullstate-clock-image-audio pair pass756aggregate byte reads/67entries from
  PCM/CGB key-base tables.0samplecount/noise/volume/generic-host ordinaryreads;
  their targeted scope remains distinct. Queued370eb0/b94690are nowreclassified:
  copied-IWRAM memoryhelpers (executable support), and palette-effect dispatch.
  Fivefixed constructors/state1/unmaskedu16index prove a17word68Bprojection with
  12nullslots; no globalmax16guard/fulltableextent. Static7literal/5producer/hash/
  all38priorowner checks pass. Strict17slot68Bpalette model/oneWinecheck nowpass
  independent fields/hash/JSON/12guards/31neworiginal prefixes, all68Bobserved;
  source-only scope distinct. e45-d44now publishes40files/39resources/all38prior/
  e44retained. Two reduced Wine checks pass12guards/lifecycle/all68B/17slotmask/
  10original-native complete-state/clock prefixes/livekind-null-raw aliases/
  BIOS-SWI-DMA/boundaries. ActualproductionCLIcreate/reuse-no-write42files/
  oldcache and one normal English-confirm1000Winepair pass everyfullstate-clock/
  image-OAM-dispatch-audio with5944aggregate byte reads/kind1only(mask2), other
  callbacks/nulls targeted-only. Neutralboot240closed0use/no fullpair. No whole
  callback/allocatorABI. Paletteprogram inventory nowroots185programs/1000commands
  from285literalexpressions/3countedlists/2stackcopies:16135candidateprogramB+
  88producerBdisjointfrom39e45owners.20indexedproducer projections now follow
  explicit task-ID/message/constant/count-loop evidence:97cells388B add45programs,
  total230programs/1135commands/18221programB+476producerB=18697B/165ranges, disjoint
  all39owners.185earlier records unchanged;39bcc4only2pointers, mixedtail excluded.
  These are no globalRAM guard/fullphysicaltable claim. Strict lossless typedmodel
  nowreconstructs165ranges/18697B and oneWine sourcecheck observes all18221programB/
  18681allB (16reservedlistBpreserved/unread),97cells/8rows/2stacktables,6copyboundary/
  3count-period/1direction probes and16shape-JSON-source guards; warningfreeWindows.
  Source/model childclosed; no fullcallback/native/stateclock/ordinary claim.
  e46-d45nowpublishes41files40resources/all39prior-e45retained and consumes typed
  header-command-parameter-producer fields. ReducedWine passes12guards/lifecycle/
  all18697B/230programs25tables and24original-native complete-state-clock prefixes/
  rawoutside/BIOS-SWI-DMA; testchaining endpoint corrected, emitterunchanged.
  ActualCLIcreate/reuse-no-write43files/oldcache passes. English-confirm1000reads0
  newB/closednegative/no fullpair;10protected worldmap-bank snapshots have no
  palette-program task. Actual EUR Montblanc Richiamo/Pecorelle CLI and one
  1000-frame Wine fullstate-clock-classic240x160-image-OAM-dispatch-audio pair
  nowpass170sourceB/4programs (32header/90command/48parameter),0producer. The
  generic test guard accepts typed bus or host consumers; selected palette-use
  assertions remain strict. Other226programs/tables targeted-only/new32:9 pending.
  Neighbor080d77c4is nowidentified as blendcontrol via originalDMAshadow→
  BLDCNT/BLDALPHA/BLDY mapping. All248direct constructor calls are classified;
  CFG-checked constants/conditional/indexed/inline/RAMtemplate producers yield
  160programs/345commands/2340B plus22pointercells88B=2428B/158ranges, disjoint
  all40e46owners. False code-root080e2ec8rejected; actualmutableRAMtemplate from
  39a5f0has a live duration override. Independent static reconstruction/hash/
  44consumer opcodes pass. Strictlossless160program/6table source model now
  authenticates110opcodes/all2428B158ranges. Warning-freeWindows/oneWineoracle
  passes160headers345commands22cells/5instantchains/template-RAMoverride/packed
  masks31-mode0/6duration/3delay/index65535raw;17shape3source2JSON2identity guards.
  2376Bobserved/52nonalpha timed second bytes preserved-hashchecked only for
  these initial modes. No fulltaskABI claim. EUR e47-d46nowpublishes/consumes
  42files41resources/all40prior-e46retained/USAe8-d7 unchanged;158bindings2428B
  serve typed header/command/producer fields. Reduced Wine passes12guards/
  lifecycle/all2428B/160program6tables and20original-native fullstate-clock
  prefixes/[34header70command16producer]compiledB/live2→1mode/rawcommand65535-
  index3/previouspaletteindex5/new2-old2/SWI-DMA. Source-set coherence separately
  passes onefocused Wineguard; production backend/source proofs unchanged.
  ActualCLIcreate/reuse-no-write44managedfiles/oldcache and one1000Wine ordinary
  EURPecorelle fullstate-clock-classic240x160-image-OAM-dispatch-audio pair now
  pass18aggregateB(10header8command)/1program/0producer; other159programs/tables
  targeted-only. NewLinux-USA-nativeWindows/new32:9 scene staypending. [Blend consumption](../rom_data_graphics.md#managed-blend-programs-and-typed-guest-reads).
  OtherLUTs/unselected bank/sample/sequence work
  and newLinux-USA-nativeWindows stay open.

- [UI choice list source contract](../rom_data_ui.md) now bounds thirteen EUR
  ranges/42eight-byte entries/336B: twelve lists from16direct8BBF0callers plus
  one six-entry template for two liveRAMfilters. All18literal occurrences/
  93original instruction pins/50prior owners checked statically. RawID/X/Y/
  opaque/navigation bytes retain exact provenance, high-bit coordinate reads
  and signed full-word copy count versus low8live count; no host target/count
  cap. Filtered0200F250lists remain mutable; base-copy ordering preserves down1
  with zero selected slots. Strict lossless public decoder/shared source plan
  and one Wine source-reader oracle now pass17guards/all336original-loadsiteB/
  18copy prefixes/44getters/43idle/64livefilters/2unmasked raw-coordinate
  prefixes with703hash-mtime guards; warning-free Windows build. No fullwidget/
  native/fullstate-clock claim for that source oracle. Managed e56-d55now
  publishes52files51resources/all50prior decoded unchanged/e55retained, noasset
  dependency/fullsetcoherence. Two reduced Wine checks pass17guards/lifecycle/
  all336typedB/13lists and29original-interpreter-native complete-state-clock
  paths/compiled[92ID183coordinate92opaque368nav]aggregateB/widths-mirrors-
  BIOS-SWI-DMA/mixedoldOBJ-raw-new/706hash-mtime guards. No wholewidget ABI;
  actualCLIcreate/reuse52generation54managedfiles no-write/oldcache and one
  ordinaryParty320fullstate-clock-image-OAM-dispatch-all-audio pair nowpass
  32aggregateUIB=[4ID8coordinate4opaque16nav]/1list/769hash-mtime guards.
  Pecorelle1000/Moogle600are terminalnegative/no pairs; other12lists have
  ordinary coverage pending and retain targeted checks. Existing snapshot/input reused, no new fixture/
  navigation or oldproof repeat. USAe8-d7unchanged; new Linux/USA/native Windows checks pending.

- [Task descriptor source contract](../rom_data_tasks.md) covers341primary/
  20auxiliary/51script records,3296B/3spans disjoint41e47owners. Original main and
  secondary constructors use different ID formulas. Independent graphics root
  39a490and directory kind0/ID413reference bound the final descriptor39a488;
  nullableThumb/contextsize/rawmetadata remain distinct. Strict lossless model/
  one reduced Wine original-reader oracle pass all3296Breconstructed/3156Bobserved,
  aliases/rawindices/23guards;140suffixmetadataBpreserved-unread intheseprobes.
  No global live selector cap or complete task ABI is inferred. e48-d47nowpublishes
  43files42resources/all41prior decoded/e47retained, USAe8-d7. Two reduced Wine
  checks pass13guards/lifecycle/all3296typedB/412records3catalogs and29original-
  interpreter-native fullstate-clock prefixes/[44callback22context9metadata]API B,
  raw/aliases/oldblendowner/BIOS-SWI-DMA; no wholeABI claim. ProductionCLIcreate/
  reuse-no-write45managedfiles/oldcache andone1000Wine ordinaryPecorelle fullstate-
  clock-classic240x160-image-OAM-dispatch-allaudio pair pass28aggregateB/4records/
  2catalogs (16callback8context4metadata). Other408records/thirdcatalog targeted-
  only; new32:9/Linux-USA-nativeWindows pending. No fixture/navigation/payloadedit.

- [Effect/event source model](../rom_data_events.md) covers75event records,
  51selectors and28evenThumbsame-state dispatchwords:1516B/4spans disjoint42e48
  owners. Independent successor roots andevent compare74bound projections; no
  global live index guard or source count fromtaskcatalog size.70assetlinks use
  engine.task-descriptors. Strictmodel/oneWineoracle pass1106original-loadsiteB,
  410Bpreserved-unread/27guards/defaults/rawindices/wrap;92halfword8word fingerprints.
  Evententry124e1creachability unknown; no wholeinterpreter/ordinaryclaim.
  e49-d48nowpublishes44files43resources/all42prior decoded/e48retained, actual
  taskdescriptor dependency/USAe8-d7. Two reduced Wine checks pass13guards/
  lifecycle/all1516typedB and29original-interpreter-native complete-state-clock
  prefixes/[24event6selector24dispatch]API B, raw/defaults/evenstate/previousfont11B/
  samepagepalette4B/BIOS-SWI-DMA. ActualCLIcreate/reuse-no-write46managedfiles/
  oldcache passes. Pecorelle1000/schoolcoldboot3000/suspendboot-to-battle8476
  locators readzero newbytes, closednegative/no event-requiredfullpairs. New normal
  EUR progression reaches a genuine39420bedroom/book snapshot; one1800-frame
  Wine locator and fullstate-clock-image-OAM-dispatch-all-audio pair on actual
  e52generation pass5aggregateB=1selector+4dispatch/one selector/one table and
  470hash-mtime guards. Ordinaryeventrows0/other50selectors/other table remain
  targeted-only; entry124e1creachability/wholeABI/new32:9 unclaimed. Linux-USA-
  nativeWindows pending, no product rebuild/performance measurement.

- The [EUR script command model](../rom_data_scripts.md) covers 115 six-byte
  commands/690 bytes, independently bounded by a distinct byte lookup. Strict
  source authentication and one corrected Wine original-reader oracle pass all
  690 bytes, signed cursors, u16 advances, relative targets, Thumb BX and loop
  return branches. A test-only ADDS expectation was corrected; production/model
  code needed no correction. Managed e50-d49 nowpublishes45files44resources,
  all43prior decoded/e49 retained/USAe8-d7 unchanged, noassetdependency. Two new
  reduced Wine checks pass13guards/lifecycle/all690B and26original-interpreter-
  native complete-state-clock prefixes/[14advance28callback]API B, raw115/255/
  signedcursor/wrap/relative-target/BL-BX/returnloops/adjacentoldowners/rawgap.
  ActualCLIcreate/reuse-no-write47managedfiles/oldcache andone8476-frame Wine
  EURsuspendboot-to-battle fullstate-clock-image-OAM-dispatch-audio pair pass
  42B=14advance28callback/7commands. Other108commands targeted-only; new32:9/Linux-
  USA-nativeWindows pending. No whole script interpreter ABI/new opcode cap.

- The [EUR relative script archive source contract](../rom_data_scripts.md#relative-script-archive-and-event-key-source-contract)
  identifies115464B/two independently bounded ranges:234event keys,62topbanks/
  233slots,207physical command segments/35048stored instructions, explicit
  repeated/header/fill target views and45FFfillspans.493links use49existingeffect
  selectors; command/effect assets are actual dependencies. Static exact-byte/
  address/fingerprint/disjointness checks pass. Strict wholearchive model andone
  Wine source oracle nowpass42333original-loadsiteB/73131Bpreserved-unread/40guards,
  including header/fill views andrawIDs/q0/255. Managed e51-d50 now publishes
  46files/45resources/all44prior decoded/e50 retained, USAe8-d7 unchanged. Typed
  index/opcode/parameter/key/fill reads preserve prior owners/raw boundaries.
  Two reduced Wine checks pass13guards/lifecycle/all115464B and28original/native
  complete-state-clock prefixes. Actual CLI create/reuse-no-write48managedfiles/
  oldcache passes; retained8476-frame EUR locator reads27archiveB across one
  bank/program/key, zero fill. One Wine8476-frame pair passes every full state/
  clock, image/OAM/dispatch and mixed/PSG/FIFO/bitwise-HLE sample; other records
  and fills remain targeted-only.
  Five linked localized text archives now have a [source model and typed runtime](../rom_data_text.md#managed-script-text-and-direct-typed-reads):
  1803744B/five ranges,385banks/41010programs,63top entries/26live variants per
  language and147glyphs already in the complete font resource. Strict source
  model/Wine oracle passes all fields/hashes/45guards and837source-vs-typed
  expansion cases. e52-d51now publishes47files46resources/all45prior decoded/
  e51retained/USAe8-d7, with actual script-key/font dependencies. Shared text
  literal mapping and two new Wine checks pass13guards/lifecycle/allbytes and
  35original/interpreter/compiled full-state-clock prefixes; live variants,
  exact slack, mixed previous-owner/raw reads and original ABI remain. Actual
  CLI create/reuse-no-write49files/oldcache passes. Retained8476battle locator
  reads0new text; a distinct school3000locator reads289B across one language/
  bank/two programs. One Wine3000-frame pair passes every full state/clock/
  image/OAM/dispatch and mixed/PSG/FIFO/bitwise-HLE sample; other records/variants/
  slack retain targeted coverage. New Linux/USA/native Windows/new32:9
  checks remain pending, without new guest caps or performance measurements.

- The [EUR script-coordinate source/model](../rom_data_scripts.md#european-script-coordinate-paths-source-contract)
  covers3056B/210paths/856XYpairs/42alignment spans, bounded by independent roots
  D68FD4/D69BC4.68script producer links use56paths; command44low8/ignored byte3
  differs from command45u16. Live map height and raw guest arithmetic remain.
  One warning-free Windows build/single Wine oracle pass24guards/allsource fields/
  hashes,212original copies/all210paths/two changed live map cells and26raw/
  producer prefixes.2972sourceB observed/84padding Bpreserved-unread; no native/
  fullstate-clock/wholemovement-task/ordinary claim for that source oracle.
  Managed e53-d52now publishes48files47resources/all46prior decoded/e52retained/
  USAe8-d7. Shared typed index/count/XY/alignment owner preserves tripleD68text/
  key/coordinate and rawD69BC4, original height/control/save/native ABI. Reduced
  Wine managed14guards/lifecycle/all3056B and24original-interpreter-compiled
  fullstate-clock cases/four full copies/allfour classes pass; new-test array/
  observer fixes retained, successful proofs reused. ProductionCLIcreate/reuse-
  no-write50files/oldcache and one6000-frame genuine EUR pair pass every full
  state/clock/image/OAM/dispatch/audio with28B=4index14count10XY/one path,
  0alignment/581guards. Other209paths/42alignment targeted-only; new32:9/Linux-
  USA/nativeWindows pending, no performance/space measurement or new fixture.

- The [EUR auxiliary script archive contract](../rom_data_scripts.md#european-auxiliary-script-archive-source-contract)
  identifies a separate u16-relative archive atD69BC4 and17-row command directory
  at370F94:4200B/8banks442slots/133physical programs913instructions.30slot links
  enter interior instructions; root walks alone omit69instructions. All489
  relative targets are instruction boundaries; structural908/913coverage is
  not actual reachability. The4098Barchive projection endsD6ABC6 after its last
  linked terminator; graphicsbaseD6B4C8 is only a ceiling. FinalFF2/archive and
  zero2/directory gap bytes remain unowned. Static reconstruction/hashes/723
  halfword4word/all47e53owner disjointness pass; no guest cap/host interpreter/
  primary-archive dependency inferred. Strict typed directory/archive models
  now preserve interior instruction references/all physical storage; one final
  Wine source filter passes40guards/442lookup/913dispatch+RAMopcode3/80advance/
  489relative/10wrap/15raw/11wrapper probes.2887Bobserved/1313Bpreserved-unread;
  raw LDRH expectations corrected to aligned32-bit rotation, first failure and
  narrower successful retry retained; model/core unchanged. No predicate/whole
  interpreter/native/fullstate-clock/ordinary claim for that source oracle.
  Managed e54-d53now publishes both directory/archive resources:50files49resources/
  all47prior decoded/e53retained/USAe8-d7. Actual own-directory dependency and
  one shared typed4200Bowner preserve old370audio/D69coordinate owners/rawtails,
  liveguest/LDRH/save-nativeABI. Two new exact Wine filters firstPASS26guards/
  lifecycle/all4200B and44original-interpreter-compiled fullstate-clock prefixes/
  allsix classes[6,65,18,19,15,2]B/edges-widths-mirrors-BIOS-SWI-DMA. Whole predicate/
  interpreter/callback ABI and ordinary unclaimed for that targeted scope.
  ActualproductionCLIcreate/reuse-no-write52managedfiles/oldcache/all50generation
  equality and one6000-frame genuine EUR fullstate-clock-image-OAM-dispatch-audio
  pair nowpass with96directoryB=32advance64callback/6commands and82archiveB=
  16index20opcode46parameter/2banks2programs/0alignment;640guards. Other11commands/
  6banks131programs/5alignment spans targeted-only. NewLinux-USA-nativeWindows/
  32:9pending, no performance/space measurement or new gameplay fixture/navigation.

- The [normal EUR unit initial-pose source model](../rom_data_unit_obj.md)
  follows existing metadata52/14 producers through394834 and original
  22174/22254/221D4/00B14/00800. Its121IDs/477frames/sixOAMlists31objects/
  333raw4bpp requests135168B become4224indexed tiles, preserving opaque fields
  and original live scheduling/DMA.415spans146358B are disjoint from51e56owners.
  Source model/original Wine oracle is closed. EUR e57-d56 now publishes and
  consumes typed fields/decoded indices with metadata52/14dependencies:
  53files52resources/all51prior decoded/resource records/e56retained/USAe8-d7.
  Two new Wine filters pass17guards/lifecycle/all146358B/30original-interpreter-
  compiled complete-state-clock paths, rawID/animation/liveRAMframe and original
  DMA. All415edges/mirrors/BIOSbus/SWI-DMA/rawholes/oldOBJsamepage compose;
  source/model proofs remain closed. Actual productionCLIcreate/reuse-no-write
  55managedfiles/all53accepted generationfiles/oldcache and one320-frame existing
  Party-unit fullstate-clock-image-OAM-dispatch-allaudio pair now pass. New10962
  aggregateB include3520pixelB/2sequences; other119ordinary sequences targeted-only.
  Otherposes/parts/IDs/palettes and unresolved65535job0/1values
  are not covered. No guest cap/wholeunit-allocator/physical catalog follows.
  New Linux/USA/native Windows/new32:9 pending; no performance/space campaign.
- The [EUR equipment OBJ source model](../rom_data_unit_obj.md#equipment-poses-source-model)
  follows original item-property9/action overrides and part0/1 loops, separate
  from actor-body sprites.748physical cells/86null/258sequences3439frames map
  to593spans110666B, disjoint all53e58resource sources. Only operation1 words
  become OAM/pixel references; commands0/6/8 retain opaque words. One reduced
  Wine source oracle passes1496constructor-prefix aliases/86null+258sequence
  resets/sixcommand cases/280scheduler-DMA uploads/all37440pixelB/10guards.
  EUR e59-d58 now publishes55files54resources with actual item dependency,
  all53prior decoded/resource records/e58 retained/USAe8-d7. Two new Wine filters
  pass17guards/lifecycle/all110666typedB/26original-interpreter-compiled complete
  state-clock paths, commands0/6/8/nulls/liveRAMframe, all593edges/mirrors/BIOS/
  SWI-DMA/new-raw-oldowner composition. Source/model acceptance stays closed.
  ActualCLI create/reuse-no-write55generation/57managed files passes from
  unrelatedcwd/oldcache. Sheep1000/Fight700 equipment-negative locators are
  terminal/no fullpairs. One newly confirmed Polly→Hildegard copiedEURFight500
  ordinary fullpair now passes all states/clocks/central-wide/OAM/dispatch/audio,
  equipment4477aggregateB/1408pixelB/two sequences exactlyequal actuale61CLI.
  Explicit positiveuse guard prevents vacuous scene acceptance; other256
  equipment sequences retain targeted coverage. General body/alternate/script
  pairings remain open. User explicitly authorizes local
  ordinary-pad EURcheckpoint derivation from a copy on25October, preserving
  originals/cartridge saves and private outputs; previous review block cleared. No bounds or extents from adjacency.
  New Linux/USA/nativeWindows/new32:9 pending; no performance campaign.
- The [normal-mode EUR battle pose source model](../rom_data_unit_obj.md#normal-mode-battle-poses-source-projection-and-typed-model)
  extends the121normal producer IDs plus42 through29body/8extra-part bases:
  10980logical keys/8374cells/5293nulls/2581sequences17894frames,150OBJlists and
  3216indexed4bpp requests. Exact initial/equipment shared views retain146358/
  6984B prior ownership; new6021spans1455474B are disjoint other54owners. One
  Windows build/one reduced Wine sourcefilter firstPASS12guards/16474constructor
  prefixes/5293null+2451newsequence resets/100commands/2858scheduler-DMA/all
  1063200newpixelB/wholeVRAM. Prior original-reader acceptance is not repeated.
  The unrestricted alternate cross-product is rejected: sprite247/animation104
  reaches directory394834, so a frame-count filter or guessed bank size would
  misclassify data. Source/model child closed; EUR e60-d59now publishes/consumes
  typed sources with four actual metadata52/14/initial/equipment dependencies:
  56files55resources/all54old records/e59retained/USAe8unchanged. Two reduced Wine
  filters firstPASS17guards/lifecycle/all1455474newtypedB/2451newsequences and32
  original/interpreter/compiled complete-state-clock paths, parts0-1-2/null/raw
  IDs/liveRAMframe/commands0-2-3-6-7-8/two uploads+twoDMA/all6APIcategories. All
  6021edges/mirrors/realBIOSbus/SWI-DMA pass; mixedwindow2new1036raw316initial
  248equipment8oldOBJ B preserves exact charges; separate2new+2editedraw API
  case passes. ActualCLIcreate/no-write reuse56generation/58managedfiles andone
  existingF09Wait3530fullstate-clock-image-OAM-dispatch-allaudio pair now pass
  842236aggregateB/815360pixelB/fivesequences, exactlymatchingCLIusage. Other2446
  newsequence ordinary uses retain targeted coverage. Alternate/script/live-
  stream pairing and newLinux/USA/nativeWindows/32:9 remain pending; no metrics.
- The [alternate EUR battle source contract](../rom_data_unit_obj.md#alternate-battle-selectors-and-explicit-shared-target-views)
  now retains all65ID/5850logical queries as genuine nulls/counted sequences/
  explicit shared-index-root targets.20sprite247targets have independently
  typed initial-pose directory/index witnesses; they are not thousands-frame
  sequences. One4Bdirectory/index self-overlap is fully witnessed, no cap/pruning/
  guessed bank extent.706proper sequences2771frames/30OBJlists108objects/
  1203pixel requests give3321new spans410086B after three earlier-owner shares;
  all55e60owner source/overlap checks pass. The tagged source model and one new
  original Wine oracle now pass15guards/all5850bindings/10544constructorprefixes
  including40shared-target aliases/4428null+606proper resets/five newcommand
  cases/1079scheduler-DMA/all342112newpixelB/wholeVRAM. No fakeframes or oldoriginal
  replay. EUR e61-d60now publishes/consumes the3321new spans with five actual
  dependencies;57files56resources/all55prior records and e60retained/USAe8unchanged.
  Two reduced Wine filters firstPASS20managed guards/lifecycle/all410086typedB/
  606new sequences and34original/interpreter/compiled fullstate-clock paths,
  properpart0/realnullparts1-2/shared-root prefixes/commands2-3-8/two uploads+DMA/
  all3321edges/BIOS-SWI-DMA/mixedraw-oldowner composition. StrictNull emptystruct
  rejects extra fields; valid source JSON stays identical. Source/model/managed/
  native/actualCLI closed:57generation59managed no-write reuse matches accepted
  e61and oneframeoldcache attachment passes (zero newalternate use). Read-only81
  existing EURsnapshot checks find no current newalternate-task replay lead;
  positiveordinary/reachability remain open, no newnavigation/checkpoint.
  Linux/USA/nativeWindows/32:9 pending. No performance/space measures/gameplay checkpoints.
- [Script49 actor selection](../rom_data_unit_obj.md#script-animation-selectors-require-live-actor-bindings)
  is now classified as live lookup, not an immediate sprite operand. Decimal49
  (0x31)/126CC0has167instructions:151low selectors scan live units,16high selectors
  use registry/fallback, then actor lookup matches unit pointers/current sprite
  +34. Group×4low16/facing signed+1F/body flag0and samebase extra parts are original.
  Nearby31can reuse a unit without changing its job field;32only repositions.
  Static script×all-sprite projection remains unjustified. Existing scripts/
  67instruction/nine literal/eighteen BL pins pass without guest/build work;
  one new test-only Wine6000functional census now links six49sources/selector16/
  group2to sprite46/animation8/parts0-1-2(eighteen queries). Body is already owned;
  two unowned null cells37C760/37C764add8Bbut no new sequence/pixels. Original
  missing-entry steps/native constructor are preserved; first all-native
  preflight failed before frames, corrected build/actual census pass. Stable
  null-cell source contract and other161instructions remain open, no fixedsprite
  promise for selector16. e61/USAe8unchanged; Linux-USA-nativeWindows-new32:9
  pending; no metrics.
- [Conditional normal script-special source pairs](../rom_data_unit_obj.md#conditional-normal-script-poses-for-special-unit-selectors)
  now join151low-selector49instructions to exact logical metadata14normal fields,
  under same-unit current normal assignment/override20clear/facing0..3.20producer
  pairs/14logicalIDs→18sprite-base pairs/108queries76nulls/32sequences241frames;
  no arbitrary cross-product, pruned query, role/extent/mixed-item conflict.
  All56e61owners share4780initial+30586normalB; new33spans6436B include60cells53
  nulls/sevensequences46op1frames/17pixel requests5248B. Sourcecontract/27halfword/
  one literal/three BL pins closed. Typed model/151refs20witnesses and single new
  sourceWine oracle pass18guards/120constructor aliases/53null7proper resets/
  17realDMAuploads/all5248newpixelB; overlapping initial/normal dependency views
  independently validated. Managed/runtime now EUR e62-d61/58files57resources,
  actual metadata14/scriptarchive/initial/normal dependencies/all56prior records/
  retained e61/USAe8. Two new Winefilters firstPASS29guards/lifecycle/all6436typedB
  and23original-interpreter-compiled fullstate-clock paths/33edges/mirrors/BIOS/
  SWI-DMA/mixed2new4normal and2new2raw; original tasks/commands remain guest work.
  Newdirectory/OBJ bytes are zero, sharedowners retained. ActualproductionCLI
  create/reuse58generation60managed/all58matchaccepted passes, canonical exact
  restored; oneframeoldcacheF09attachment haszero newusage/smokeonly. Newactual
  Snow6000locator observes48typednullindexB/no newpixels; RIVERstart618zero newuse.
  Bothclosed/no fullpair/repeat;81new-sequence snapshot reads find no tasklead.
  Ordinarynewsequence/pixel/equivalence and alternate/override/high16/current-state
  cases pending;
  newLinux/nativeWindows/USA/32:9 remain pending, no performance work.
- [Conditional alternate script-special poses](../rom_data_unit_obj.md#conditional-alternate-script-special-source-and-typed-model)
  now have an exact151reference/20logical-witness source projection and strict
  offline model:84keys68null/14sequences64frames,20new spans816B over three
  prior owners. One focused Wine source oracle passes19guards/108newcell
  constructor aliases/52null+2proper resets/two actualDMAuploads512pixelB with
  wholeVRAM equality. EUR e63-d62now publishes/consumes59files58resources with
  five actual dependencies/all57prior records+data/e62retained/USAe8unchanged.
  Two new Winefilters pass31guards/lifecycle/all816typedB and24original-
  interpreter-compiled complete-state-clock paths/20edges/BIOS-SWI-DMA/
  mixed2new4alternate+2new2raw; oldowners unchanged, no task/save/nativeABI
  rewrite. ActualproductionCLIcreate/reuse59generation61managed/all59accepted
  files/no-write andoneframeoldcacheattachment pass(zero newuse/smokeonly).
  Read-only81snapshot search for exactly two new sequences finds no matching
  enabled/dormant task; locator closed/no guest/fullpair/retry. Ordinarynewpixel/
  equivalence remains pending; high16/42-230/currentactor and newLinux/
  nativeWindows/32:9 remain separate. No performance/storage measurement.
- [Forced42/230script pose sources](../rom_data_unit_obj.md#conditional-forced42-and230script-pose-source-model)
  now project167exact refs/13groups through original flag20forced current
  normal42/alternate230assignments under explicit retained actor conditions.
  26mode-base/334conditional witnesses/156keys126nulls/28sequences140frames;
  new40spans1050B over oldnormal/alternate views. Strict offline model and one
  new Winefilter pass19guards/192newcell aliases/90null+6proper resets/bothnew
  operation2argument copies/two actualDMAuploads256pixelB/wholeVRAM equality.
  EUR e64-d63managed/runtime now passes60files59resources/threeactual deps/
  all58prior data+records/e63retained/USAe8unchanged. Two newWinefilters pass
  31guards/lifecycle/all1050typedB and27original-interpreter-compiled fullstate-
  clock paths/bothnewop2/newOBJ/40edges/BIOS-SWI-DMA/mixedoldowners/raw.
  ActualproductionCLIcreate/reuse60generation62managed/all60accepted/no-write/
  oneframeoldcacheattach pass(zero newuse/smokeonly); ordinarynewpixel/equivalence
  remain pending, no task/save/nativeABI rewrite. New read-only81snapshot scan
  for allsixnewoverride sequences finds no enabled/dormant task; locatorclosed,
  no guest/build/fullpair/retry.
  This does not close highregistry/liveactor identity; no metrics/newfixtures,
  newLinux/USA/nativeWindows/32:9 pending.
- [Registry-knownspecial script source projection](../rom_data_unit_obj.md#registry-selectors-and-known-special-unit-source-projection)
  authenticates16high refs/seven pairs/93knownmetadata14rows under same-unit
  retainednormal-or-alternateassignment/override20clear, before targets; no fixed
  registry identity.1302witnesses/2220keys1908cells/373842sourceB share358566B
  across six priorowners; new272spans15276B include598cells567nulls/21seq96op1
  frames/37pixelrequests10880B. All59e64owners/41halfword4literal8BLpins pass,
  no root-target/role/extent/mixed-item conflict. Strictlossless model and one
  focused Wineoracle pass26guards/1196newcell facingaliases/567null+21proper
  resets/37actualDMAuploads/all10880newpixelB/wholeVRAM; six previous typed
  views validated independently, physical aliases coalesced exactly. Corrected
  frontend build warning-free. EUR e65-d64nowpublishes61files60resources with
  actual metadata14/archive+sixpreviousview dependencies. Two focusedWinechecks
  firstPASS33attach+10dependency guards/lifecycle/all15276typedB21seq and25
  original-interpreter-compiled fullstate-clock paths/272edges/BIOS-SWI-DMA/
  mixedold-raw-new. Six previous owners retain sharedbytes. ActualproductionCLI
  create/reuse61generation63managed/all61accepted/no-write/oneframeoldcache
  attach firstPASS(allnewusage0/smokeonly); ordinarynewpixel/equivalence
  pending. New read-only81snapshot/21newsequence locator finds no enabled or
  dormant task, closed/no guest-build-fixture-fullpair or unchangedretry.
  USAe8unchanged; genericjob/outofcatalog/redirect/currentchanged
  source cases remain separate, no metrics/newfixtures or newotherplatform tests.
- [Registry generic direct-job source projection](../rom_data_unit_obj.md#registry-selectors-and-generic-direct-job-source-projection)
  authenticates high16refs/sevenpairs, nonnullunit+4=0/1/currentjob+5/alternate+7/
  retainedsameunit originalassignment/override20clear before targets. All116rows
  classified:110normal100alternate concrete/2normal12alternateFFFF/4routeFF;
  2940witnesses6720refs,83halfword6literal-dispatch11BLpins. All60e65owners checked
  before targets.107sprites535pairs3210cells2420null/777properseq5093frames and12
  explicit initialindexroot48..59views. Full796992B shares697168B withsixowners;
  new659spans99824B include1386cells1260null/126seq588op1/268pixelrequests82016B.
  No mixeditem/role/extentconflict; initialjoint110count corrected beforetargets,
  no hidden sentinel/redirect pruning or u16FFFFfallback. Strictlosslessmodel
  and firstfocusedWineoracle pass34guards/2772newcell aliases/1260null126proper
  resets/268DMA82016pixelB/wholeVRAM/all12initialroot views; sixpriorviews checked
  independently. EUR e66-d65nowpublishes62files61resources withactual metadata52/
  archive+sixpreviousview dependencies. TwofocusedWinefilters firstPASS36attach+
  10dependencyguards/lifecycle/all99824typedB126newseq and25original-interpreter-
  compiled fullstate-clock paths/659edges/BIOS-SWI-DMA/mixedold-raw-new. Six
  previous owners retain sharedbytes. ActualproductionCLIcreate/reuse62generation
  64managed/all62accepted/no-write/oneframeoldcacheattach firstPASS(allnewusage0/
  smokeonly); ordinarynewpixel/equivalence pending. New read-only81snapshot/
  126newsequence locator finds0enabled/dormant bodytasks without currentpointer
  filtering, closed/no guest-build-fixture-fullpair or unchangedretry. USAe8
  unchanged; no previous acceptance replay,
  fixture/navigation/metrics or newotherplatform tests; globalregistryidentity/
  outofcatalog/nonreturningredirect/currentchanged remain separate. The
  [known alternate-job redirect contract](../rom_data_unit_obj.md#registry-generic-jobs-redirected-to-a-known-alternate-job)
  closes currentjobs80/81/82/85routeFF to concrete known alternate rows:
  11760conditional witnesses26880refs map exactly four-to-one onto all2940direct
  witnesses. All116rows/928effective classifications retained;98halfword6literal
  11BLpins and all61current resource/source hashes pass before targets. Same
  3210cells/777sequences/full796992B/sevenowners, zero newbytes/resources/revision;
  existing metadata52ande66Wine proofs reused without build/guest replay.
  The [known FFFFreader contract](../rom_data_unit_obj.md#known-generic-job-ffff-results-and-raw-bus-reads)
  classifies196direct+784redirected witnesses/2240refs: actorproperty083B4C12=DC86,
  directory083D4830=14AE3F1E;60facingqueries/30words use unmapped14bus, with live
  bios_open/CPUrotation rather than a proved ROMsequence/null result.71readerpins/
  threecodeblocks/twoliterals pass. Both ROMreads total6Bcurrently unowned/raw;
  their migration and ordinaryreachability remainpending, no cap/pointerfix.
- [Actor property words](../rom_data_unit_obj.md#actor-property-words-from-known-sprite-assignments)
  have a separate source contract:221C4reads394C14+2*sprite, unsignedu16 through
  actor9AD0E/r8/constructor r3into allocation/task+16.398knownnormal/alternate
  job-special-override witnesses derive182words before targets;98halfword15BL/
  fourcodeblocks/oneliteral andall61currentowner hashes pass. Fourunownedranges/
  364B/sourceSHAad328c8e.. retain139words16/43words24 andrawholes, no248extent/livecap
  or flag/VRAM-unit interpretation. Strictlosslessmodel andfirstWineoracle now
  pass20guards/all182originalgetters364B/3rawfullwordcases/fournewtaskprefixes/
  tworeuseprefixes/fourisolatedsetters/wholeRAMoutside declaredscratch-destinations.
  Existingtask keeps its own+16word; newtask receives newproperty. Windowsfrontend
  firstbuildwarning-free/82build83Wineguards11compiledinputs exact. No fullallocator/
  native-clock/ordinary claim for02. EUR e67-d66nowpublishes63files62resources with
  actualmetadata52/14dependencies/all61priorfiles-records/e66retained. Twofirst
  exactWinefilters pass17attach+4fullsetdependencyguards/lifecycle/364typedB/
  182recordmask and16original-interpreter-compiled fullstate-clock paths with
  all364rawB altered. Fourrangeedges/widths-mirrors/realBIOSbus/SWI-DMA/mixed
  new-old-raw pass;84build86Wineguards20compiledinputs exact. ActualproductionCLI
  create/reuse63generation65managed/all63accepted/no-write/oneframeoldcacheattach
  firstPASS(allnewusage0/smokeonly),154activeCLIguards exact. One newordinary
  F09Wait3530CLI/fullpair nowfirstPASS10reads/20aggregateB/twowords; everyframe
  fullstate-clock-central-wide-OAM-dispatch-allaudio equal, remaining180words
  targetedonly.155activeguards/21compiledinputs exact; optionalnewpositiveguard,
  oldnormal-scenechild remainsclosed/no otherproof replay. No fullallocator or
  new32:9claim. Ordinarydeclaredsceneclosed,
  USAe8unchanged. No oldproof replay/gameplayfixture/navigation/performance measurement;
  FFFFraw6B/globalidentity/changedactor separate.
- The [twenty-slot job lookup source](../rom_data_gameplay.md#twenty-slot-job-lookup-source-contract)
  now has a bounded EUR contract:20u8jobrefs/20B at399750, twenty distinct existing
  metadata52rows, disjoint64currentowners. Original12CA98live20slot selection,
  preservedr6 across CFF70clear andsixlookupwrappers prove the conditional domain
  before data inspection;268halfwords/9literals/8BLs authenticated. No whole
  physicaltable/globalinputcap/gameplayreachability or sprite target is inferred.
  Sourcechild closed. Strictlossless20entrymodel andone firstWinefilter nowpass
  independent20B/12guards/123originalpaths (21bare/81fullproperty/21nameIDprefix),
  threehighinputaliases/all20originalLDRBsiteB/callee-fullRAMoutside20Bstack/
  VRAM-OAM-palette; firstWindowsfrontendbuild warning-free. Onlytwo moduledecls,
  The model phase left e70-d69/65files64resources/USA8-7 unchanged. Managed
  e71-d70now publishes66files65resources/all64oldrecords-files/e70retained, actual
  metadata52dependency/immutable20Bowner/page399/20bit mask. Two Winefilters after externalENOSPC/tmp recovery/
  one warning-freeWindowscore/frontend build pass18attachguards/3dependencyguards/
  unavailable lifecycle/all20rawB altered and16original-interpreter-native full
  state-clock paths, including12CA98mode3slot19→clear/nameIDprefix/rawoutside/
  boundaries-SWI-DMA. No globalguestcap introduced. ActualproductionCLI nowpasses
  create/reuse-no-write/oldcache-F09oneframe:66generation68managed65resources/
  all66accepted, canonicalhash-mtime exactrestored; privateTMPbacking avoids03
  actualENOSPC withoutnewcleanup/buildretry. DeclaredBank120ordinary nowpasses
  firstnewCLIpositive andoneWinefullpair120state-clock/capturedcentral-wide/OAM/
  dispatch/allaudio,2jobB/1slot equalCLI; other19ordinaryslots targeted-only.
  Only5lineoptionalnewguard/report, firstWindowsfrontendbuild warning-free;
  rawCLI284x160Bankbackground has no UIpanels, no composedGUI/fullABI claim.
  Source/model/runtime/CLI/declaredordinary children closed; no fixture/navigation/
  measurements or oldmetadata52/FFFF/A4proof replay.

- The [bounded native ROM API residue test](../rom_data_runtime.md#bounded-native-rom-api-residue-inventory)
  wraps a test-only BlockTable/read8-16-32 API, leaving production dispatch/API
  unchanged. First EUR Bank12 Wine filter/one warning-freeWindows frontend build
  preserve every fullstate-clock/buffer/OAM/dispatch/audio comparison and73actual
  input guards.3166context reads/3683imageB include676unownedB/79observedranges;
  these are source leads, not asset boundaries/completeness. Interpreter/HLE/DMA/
  native constants/other scenes remain outside this observer. Its five highwords
  are now staticallyclassified: two bounded4entry DMAregister directories and
  one ThumbIWRAM branch pointer viaBXr2;392halfwords/47pins/5literals/4BL exact.
  Lower656B now have static original Thumb witnesses:158literalwords/184linear
  context paths (507unique halfwords) and6words from5guarded switchtables
  (82prefix halfwords). No exact dynamic instruction/mode attribution or all-use
  claim. All676stilloutside owners; originalROM/code support dependency explicit.
  No RAMbody/ABI/asset or whole-game completeness inference.
  Samefrozenobserver nowpasses one WineF0912filter/fullstate-clock/buffers/OAM/
  dispatch/audio, no rebuild or oldscene acceptance replay.13671contextreads/
  13961imageB include604unownedB,584sharedwithBank+20newB/fivePCrelative static
  witnesses.76actualinputguards exact. No newasset/revision/completeness claim.
  No fixture or performance campaign; e71/USA8-7 unchanged, platforms pending.

- The [optional bus/host ROM audit](../rom_data_runtime.md#optional-bus-and-host-rom-read-audit)
  Cargo feature is absent by default. Scoped TLS collects successful bus and
  selected audio/sprite/table/keyboard source requests, suppressing interpreter
  instruction fetch only; no Machine/API/save schema fields. One WineF0912filter
  passes fetch/width/mirror/lifetime probes and every fullstate-clock/buffer/OAM/
  dispatch/audio comparison after a warning-free build retry for a test import.
  23061unique requests/19984imageB include1146unownedB,542beyond prior nativeAPI
  sample;90new highbytes are source leads only.76actualinput+onecomparison guards
  exact, oldacceptances not replayed. Its90newhighB nowhave sourcecontracts:
  two ordinaryindirectOBJlists64B/10objects,10parameterB and16IWRAMcontrolB.
  One newproducerargumentWineframe plus295windowhalfwords/53pins/7literals/
  reusedABIhashes identifiesactualreaders. No assumedwhole table/cap/allocator/
  commandproducer ABI. Two-list isolatedtypedmodel nowpasses one Winefilter:
  64B/10objects/14guards/sixoriginalROM-RAMreader paths, wholeoutput/matrix/callee/
  RAMguards exact. One warning-freeWindowsfrontendbuild;75selectedguards exact,
  oldCatalog74/e71unchanged during that isolated model milestone. Managed/runtime
  integration now passes EUR e72-d71:76lists/185objects1262B,66files65resources,
  126direct refs/all74oldlists/all64other records-files/dimension dependency/e71
  retained. One warning-free Windows frontend build and two first Wine filters
  pass12attachguards/publication/lifecycle, six original/interpreter/native
  fullstate-clock paths with64rawB altered, sixSWI-DMA/sixhost checks and mixed
  old-new/raw-new API reads.77selected guards/compiled code exact; required
  artifact completion retires only disposable copies. Actual productionCLI now
  firstPASS create/no-write reuse66generation68managed/all66accepted plus existing
  F09oneframe/cache/input16:9; bothnew sources guest-host positive.143CLIguards/
  implementation/canonicalhash-mtime exact, one warning-freeproductionbuild.
  Declaredordinary firstF09frame8476→8477 now passes one Winefullpair withboth
  newguest-host sources required: fullstate-clock-central/wide-OAM-dispatch-all
  audio, actualsamples[2196,2196,264,264,2120], exactCLIaggregate147B/12hostreads.
  Five optionalvalidator lines only; firstfrontendbuild warning-free,75guards
  exact. Broaderordinary remains separate. The10parameterB now have an isolated
  observed_task_parameters model:2phase1/2u8thresholds/fourindices4..7signed16
  corrections,27guards/18originalprefix cases in one firstWinefilter after a
  warning-freeWindowsfrontendbuild. Oldcounter compare/phasewrap/lowbitalias/
  signedsubtraction/RAM-callee guards exact;79selectedinputs exact. Phase0/
  correctionpriorowner/outsidequeries stayoutside model, no globalguest cap or
  units inferred. Model milestone left productione72unchanged. Managed/runtime
  nowfirstPASS e73-d72/gameplay.observed-task-parameters:67files66resources/all65
  oldfiles-records/e72/76OBJlists retained, one10B/twospan file withindependent
  fields/nodeps. Two Wine filters pass13attach/3fullset guards/lifecycle and24
  original/interpreter/native fullstate-clock prefixes with10rawB altered,
  outsidequeries/edges-mirrors-BIOS/sixSWI-DMA/mixednew-old-raw.26compiledB/
  316instructions24dispatch arecoverageonly;74guards/compiledinputs exact.
  Firstcorebuildcompiled; two frontendtest-call missingargs fixed, incremental
  buildwarning-free. Requiredartifactcompletion retiresonlydisposable copies.
  ActualparameterCLI nowpasses firstproductionbuild/canonicalrestore, prepare/
  no-write reuse67generation69managed/all67accepted andoldcacheattachment. One
  user-interruptedprepare archived/retriedsamebuild; firstframe1thresholdB but
  correction0 isnegative. New12frameCLI reaches20reads28B/12threshold16correction/
  2phases4rows;145guards/code/canonical/userAtlas exact. Ordinary12framefullstate
  nowpasses one Winefullpair8476→8488/all6rows: everyfullstate-clock/central-wide/
  OAM/dispatch/allaudio, samples[26334,26334,3168,3168,26258], exactCLI20reads28B.
  Five optionalvalidator lines only, firstWindowsfrontendbuild warning-free/
  76guards/code/originals/userAtlas exact. Allbounded parameterchildren closed;
  broaderordinary separate. The lower452B are now source-classified by an
  opt-in filtered execution-site capture:113words/114reads,101literal words404B
  from102executed Thumb LDRs and12switch words48B. Original opcode/address/width
  checks distinguish actual operands from inherited HLE/DMA scope; cached and
  uncached probes retain old address-only keys. One new Wine source-only filter
  observes all selected words in a single current F09 decoded12frame arm;
  no old equivalence replay.85guards/source/defaultcfg preimages/Atlas exact.
  These bytes remain code support, no new resource/revision/asset extent.
  Pointed394FBC now has a separate conditional constructor-animation source/model:
  four8B threshold/list-pointer pairs32B plus four14B/two-object lists56B/eight
  objects. The original u8counter increment/store/reload precedes unsigned-word
  compare;255wraps0, newphase>3 clearsactivebit. Oldphase has no prelookup cap;
  model0..3 only. One firstWindowsbuild/oneWinefilter passes13guards/15counter/
  4constructor/4new-list copied-reader cases,94inputs/source/preimages exact.
  The isolated milestone left e73 unchanged. Managed/runtime now passes e74-d73:
  68files67resources,32B table withOBJdependency and4lists56B merged into existing
  owner,80lists193objects1318B/all76oldlists/65otherrecords-files retained. First
  Wine passes10attach/2halfset/publication/lifecycle/13fullstate-clock prefixes/
  native36typedB/all4phases/4outside/708buscases, then testsetupROMidentity fails
  beforeDMA. Restore-before-load correction is testonly; remainingfilter passes
  6SWI-DMA/4hostpublications/twoobjects each without replaying completed checks.
  Production byteidentical across observations;169guards/source/Atlas exact.
  ActualCLI nowpasses firstproductionbuild/canonicalrestore and3Winecommands:
  prepare/no-write-reuse/zero-frame startup,68generation70managed/all67runtime
  resource hashes exact/oldcacheloaded;145guards/source content exact. No guest
  execution claim. Ordinarypositive discovery remains partial: one testbuild/
  fourWine normal-key bounds from retained12006 copy (no3530replay) allnewtable0,
  although movement selection/confirmation completed. Passive observer and7line
  optional validatorgate compile; fullpair notrun/no positive fixture. Original
  effect flag setter9D054 and6enable-disablecallers have a source-linked
  follow-up:21boundedwindows bindMove state27→pathcount→28/29 andenable99F18,
  plusseparate3A/exchange paths. Readonlynormalgrid fields justifyonlyA-confirm;
  one reused-Wineobserver160bound isnegative table0 (fifthclosedbound), notfullpair.
  OneexistingCLI64frame readonlydump provesnormalMove flag20set12186–12230/
  45postframes whileboth394/3A4slots remaininactive/counter-phase0/table0. No new
  build;587guards496frozeninputs exact/helper0paths, negativefixtures retained.
  Ordinaryflagreachable isclosed. The subsequent source-only slotgate child
  authenticates8windows/owner-callback flow andcell predicates: FD2, cell02,
  surfaceheight>actorheight/animation36!=40 plusflags/free slot. Reused64RAM
  captures haveflags02absent fromall256conditionalcells, stable512Bhash; no
  eligiblecell inthisunchangedmapstate, so furtherMoveguessing there isexcluded.
  87guards/helper0paths, no newbuild/gameplay/test. Parentordinary stillopen;
  newreadonly81normalEURsnapshot criterion nowclosesNEGATIVE:2mapprojections/
  alldomainsvalid/zero02cells/88guards, oldalternate-sequence scan notreplayed.
  Normalterrainloader alreadyclosedOct24, so oldBGownership isreused. New
  typedsemanticquery113auxiliarypacketprograms/162links finds25programs/44IDs
  seeding02; no newresource/decoder or test. Targeted1DE98directBL/literal census
  empty (notdeadcodeproof), one1DD68caller reads02.24guards4windows exact/helper0.
  Conditionalmapselection source child nowbinds253C8state10/variable7/setter/
  loader totypedopcode12(advance2/callback126831).204instructions56default/
  148literal;19literalinstructions select10eligible rawmapargs.37guards9windows
  exact/no newbuild-game-test. TWOretainednormal examples active35/197 default
  records match99/85andremainineligible; not81scan replay or eventkey=indexjoin.
  Active-index producer is now source-linked:9CF0 writes02002170/172;9D04 selects
  event-row primary/alternate, and127388 scans rows255..1 by event key. The normal
  63944 path joins selected70B record713B3C+45 to12B event row71C760. A new query
  finds44 direct-primary/default-script/cell02 rows in the caller's256-value
  domain; not archive extent or ordinary availability.42guards23windows exact,
  no build/game/test. An eligible quest actually available from a normal
  checkpoint was then confirmed by two reused-observer Wine
  routes:620 to retain the missing pub menu,300 normal4Down+A to White Flowers.
  Live14-entry list has eligible IDs58/72; visible fifth row/detail selector
  confirms58→event58/primary315/maparg69/descriptor68.130guards496frozen sources/
  23windows exact/helper0paths; no build. Normal mission58 acceptance is then
  proved by300gil payment7709→7409 and Fee:Paid detail.27bounded Wine navigation
  checkpoints/4840frames reuse309ba31c/e77,337guards496sources17exploratory windows
  exact, no build/fullpair. QuitMission menu misread was corrected without
  cancelling; actualLeave reachesworld. Named cursor endpoints Cyril/LutiaPass/
  Nubswood/Sprohm, no Giza battle yet. Stop blind cursor trials; inspect original
  world-location/placed-coordinate join. This join is subsequently closed:
  corrected literal03002810, source375F8/385CC coordinate/placement semantics,
  Giza region7/node20/source395E24→world112,296. One warning-free497-source Windows
  test build/one neutral Wine frame2232reads2276sites323scoped pass; rawRAM matches
  retained parser.152guards32windows/helper0paths, no newnavigation. Retained21
  cursor128,287 is nearby; shortLeft thenactual label beforeA. New30-node20B ROM
  family first2coordinatebytes classified. The subsequent600B source projection
  now authenticates four ordered path/neighbor links and10opaqueB per record:
  300semantic/300opaqueB,90links/45reverse path pairs.4DAD0neighbor-first lookup
  differs from46EB0nonzero-path adjacency; node0valid, rawfallback outside30.
  25windows44pins42guards/69owner49746span disjointness pass, no build/game/test.
  Typed world_nodes model now reconstructs600B/one source plan with30ordered
  nodes/four links/opaque bytes and separate shape/source checks. One warning-
  free core-only Windows build/oneWinefilter pass23guards/566original-reader
  calls/all300semanticB/16bytealiases/24outsidecalls;300opaqueB preserved/unread.
  24actualguards215frozen coresources/helper0paths exact, no gameplay/frontend
  build. Subsequent e78-d77 managed/runtime now publishes71files70resources,
  all69prior exact, USAe8-d7 unchanged; one600B typedbinding/7statsfields andraw/
  RAM fallback, no guestalgorithm/ABI change. OneWinefilter passes12attach/
  2fullset guards,15native fullstate-clock prefixes/32compiledsourceB/124instr,
  all600busB/30nodes/widths-mirrors-edges/priorowner/LoadUndo/DMA3. Two staleoldcore
  builds rejected; exactoldfingerprint moved afterlibtouchfailed, thirdbuild
  warningfree/fresh=false/frozen2cf1d882.. from502stablefiles.200actualguards/
  502integration+215modelsource hashes pass; disposableinterruptedcopy retired,
  acceptedoffline dataset kept. Actual productionCLI now passes create/no-write
  reuse/normal16 at19049→19065:71files exactruntime/70IDs/73managedpaths unchanged,
  old28859blocks cache loaded/402coordinateB22nodes.169guards502sources/helper0,
  warningfree e0491cdc.. andcanonicalhashmtime restored. Private absolutecounter
  matcher fixedoffline after3childexit0/no rerun. Path-neighbor-opaque0 in16bound
  remainsclosed. Newsourcecaller10windows/27guards plus2retainednormalstates
  justifylatertravel: current17/selected20(dest19)→current19/progressFF;
  conditional3B012→4DB1C/4DB80,47034progressgate/4DAD0. OneCLI360andoneWinefullpair
  nowpass6841B=4820coordinate864path1157neighbor/29nodes, all360states-clock/
  classic-central-wide/OAM/dispatch/5audio, exactCLIcounts. Five-line optional
  guard/oldvalidatorbyteexact; one warningfree502source build dabb1ac6..,
  103guards/old169CLIguards/source27 exact/helper0. OrdinaryworldnodeCASEclosed,
  opaque0targeted-only; othernodes/maps remainopen. Road397018nowhasconditional
  sourcecontract:45selected8Bheaders/360B plus2188signedXYoffsetpairs/4376B,
  46spans4736B/4601semantic135opaque, disjoint70owners49747spans.4DD0Clowu8
  direction/index/step, reverseorder/no signnegation/out-of-rangezero;4DD70
  orientation/4DD84minnode anchor. Fiveonepixel-short endpoints preserved,
  notnormalized.6windows60pins17guards/helper0/no build-game-test for source.
  Standalone world_roads model now reconstructs46spans4736B, with strictJSON,
  coherent partition and separate authenticity; preserves fullu8 counts/opaque/
  signed samples and rawoutside45. One warningfree core-onlyWindows build/
  oneWinefilter pass29guards/2537originalcalls/all4601semanticB,135opaqueB
  preserved-unread, reverse/zerooutput-loadskip/u8aliases/order helpers exact.
  32preservationguards/504buildfiles(218coreinputs) exact/helper0, no gameplay;
  Standalone-model revision was e78-d77. Now e79-d78/index70 publishes
  world-roads.json:72files71resources/all70prior exact; USA8-7/schema1 unchanged.
  Immutable46span typed count/pointer/sample/opaque owner composes oldnodeleft/
  rawright/indices45..255. Two warningfree506source builds/accepted52be2981..;
  firstWine publication12attach2fullsetPASS, nativecase3FAIL missing396page gate.
  Routingfix+remainingWine passes12native fullstate-clock poisoned4736B,
  all4736busB/4classes45paths/widths/mirrors/LoadUndo/DMA3. 221guards506sources/
  504closedmodelpreimages exact/helperonlyinterruptedcopy; no game/oldproof
  replay. RoadactualCLI nowpasses create/reuse/normal360:72files exactruntime/
  71IDs/74managedpaths no-write, 28859oldcacheblocks; 2976reads/5067B=
  885count/2788pointer/1394sample/0opaque/40paths, positive3classes.
  Warningfree506source productionbuild 1a86d341../canonicalhashmtime restored;
  180guards/helper0. One ordinaryWine360fullpair nowpasses allstates-clock/
  classic-central-wide/OAM/dispatch/5audio, all7roadcounts exactCLI, 3classes
  positive/40paths(anyfield, not40payloads)/opaque0. One5lineoptionalguard/
  oldvalidatorbyteexact, warningfree506source build 4c453f07../100guards plus
  180closedCLI guards exact/helper0. RoadordinaryCASEclosed; all45payloads/maps/
  opaqueordinary/globalextent/fullABI remainunproved.
  The subsequent normal Giza route closes battle entry: primary315/map69/
  descriptor68, live512Bgrid equals packet56seed with20flag02cells. MarshFD2 at
  (6,5)normally moves to(5,7), triggering all4phases/4lists guest-host after75
  frames:22reads/88tableB. The new11-frame before-use suffix now passes actual
  CLI and one ordinary fullpair22713→22724: allstates/clock/pixels/OAM/dispatch/
  fiveaudio, all4lists guest-host and exactCLIcounts.91guards496sources/helper0,
  no build; ordinary constructor case closed. Sixteen prior discovery captures/
  3875frames/221guards remain evidence, not16equivalence acceptances. No manual
  guest edits/cart writes; earlier negative bounds remain closed. Declared40Bencounterheader physicalcatalog nowproved:442headers1..442,
  first70C8F4/rootbase70C8CC/nextindependent28Bfamily710E04,17680B.27cachedroot
  loaders/40windows54pins68guards;33semantic7opaqueB/header. All442template
  backlinks contiguous2850*48B6EA048..70B6A8; nestedpayload schema separate.
  Index0prefix ends in header442coordinatepayloads, notvalidheader/rawfallback.
  Header29pointer70CD58alreadyconstructor-alternates-owned:2newspans17676B,
  dependency/sharedcoherence/skip4oldbytes required. Standaloneencounter_headers
  model nowlossless442typedheaders/full17680B/shared2span17676B; strictJSON/shape
  separate fromsourceauth/constructorpointercoherence, byte_at skipsold4B,
  lowu16alias/Noneoutside retained. One core-onlyWindows build5855af92.. and
  oneWinefilter pass29guards/7523originalprobes(7072one-loadprefixes/451fullgate),
  all14586semanticB incl4shared/3094opaque preserved-unread, registers/RAM/VRAM/
  OAM/palette checked.33guards508frozenbuildfiles221coreinputs/helper0 exact;
  no game/frontendbuild for model. Now e80-d79/index71 publishesencounter-headers:
  73files72resources/all71prior exact, constructor-alternatesdependency/coherence;
  USA8-7/schema1 unchanged. Immutable2span17676B owner skipsold4B, sourcepage70C..710,
  8classes/442headermask/11stats. One warningfree510source build1c4135f5../oneWine
  filter pass14attach3fullset/26native fullstate-clock poisoned17680B,
  31compiledinstructions/37newB+4oldsharedB; all17676new+4oldB/8classes442headers/
  widths/mirrors/mixedborders/LoadUndo/DMA3. 206guards510hashes508modelpreimages
  exact/helperonlyinterruptedcopy. ActualCLI nowpasses prepare/reuse/startup700:
  73files exactruntime/72IDhash/75managedpaths no-write, 28859oldcacheblocks;
  17headerreads/41B, 6semanticclassespositive/1headers/0opaqueB.
  Warningfree510source productionbuild 9d0a772d../canonicalhashmtime restored;
  184guards/helper0. One Wine700ordinaryfullpair nowpasses allstates-clock/
  classic-central-wide/OAM/dispatch/5audio, all11usagecounts exactCLI/sixclasses
  positive/oneheader; placementparameter0/opaque0ordinary unobserved. One5line
  optionalguard/oldvalidatorbyteexact, warningfree510source build43fcc614..;
  103ordinaryguards+184closedCLIguards/510sourcehasheseach exact/helper0.
  OrdinarybattleentryCASEclosed. Nestedunittemplates nowhave sourcecontract:
  2850*48B136800B6EA048..70B6A8,442cachedheadergroups/8windows5new3reuse/39pins/
  4BLtargets/41guards. Full48projection/equipment14250refs/metadata14-job owners
  checked; two newfragments136608B skipoldconstructor192B/records225..228/header29,
  fullsharedcoherence required. Dynamicbitset14+index/8 has no20B/160bitguestcap;
  tail2E..2F opaque/fullu8-u16 retained. No build/game/test/oldproof replay.
  Strictencounter_unit_templates model nowlossless2850records/full136800B:
  headerpartition/complete48sharedcoherence/item-metadata14-job reference checks
  separatefromshape/sourceauth; byte_at skipsold192B, opaque2/fullfields retained.
  One core-onlyWindows builde0ae0d5c.. from512files/224coreinputs +oneWinefilter
  passes38guards/2100selectedoriginalprefixes(1020LDR/1080bit,120outside20B),
  distinctreads1531new/102old/4rawoutside; notallrecordssemanticcoverage/fullABI.
  AllRAM/VRAM/OAM/palette/ROM exact; 34modelguards512hashes+41sourceguards/helper0.
  No game/frontendbuild/oldacceptance replay. Now e81-d80/index72 publishesunit
  templates:74files73resources/all72prior exact/fiveownerdependencies; immutable
  twofragment136608B/shared192skip/sourcepages6EA..70B/globalOR/9classes andrecord-
  headerstats. One warningfree514source build3d4b2af2../oneWine filter14attach7fullset/
  27native fullstates-clock with136800rawBpoisoned, 25compilednewB/8oldB;
  all136608new+192oldB/2846newrecords441headers/mixedwidths-mirrors-edges/LoadUndo/
  DMAword34202-CpuSet68404halfword-FastSet34208word allPASS. 215guards514hashes/
  512modelpreimages exact/helperonlyinterruptedcopy. Actualunit-templateCLI now
  passes create/reuse/normal700:74files exactruntime/73IDhash/76paths no-write,
  28859oldcacheblocks/186reads211B/5records1headers,
  8positiveclasses/1unobserved. One warningfree514source productionbuild
  afeaef25../canonicalhashmtime restored/184guards/helper0. One Wine700ordinary
  unit-template fullpair nowpasses allstates-clock/classic-central-wide/OAM/
  dispatch/5audio, all13counts exactCLI/eightclassespositive/5records1header/
  opaque0unobserved. One5lineoptionalguard/oldvalidatorbyteexact, warningfree514
  source build33c61082../104ordinaryguards+184closedCLIguards514hasheseach/helper0.
  Unit-templateordinaryCASEclosed. Coordinates nowhave sourcecontract:691lists/
  2342pairs/4684B70B6A8..70C8F4, allheaderpointer-count extents exactlypartitioned,
  noholes/alias/priorowner against73resources49797spans. Roles placement0=3272B/
  random680B/placement2=88B/default644B;323headerswithpayload119allnull. Rawu8XY
  retain signedsearch/unsignedreads; RNG02868nonnegative15bits/remainder146314/
  u16mask original, no10/16cap/fallback invented.12windows3new9reuse/41pins38guards,
  no build/runtime/game/fixture/oldproof replay forsourcecontract. Strictcoordinate
  model nowlossless691lists2342u8XYpairs/all4684B/691spans, shape/headercoherence/
  sourceauth separate/no nullfallback/observedvaluecaps. FirstWindowscorebuild
  failstestBusimport, fixedtestonly/second516copy accepted2fe3e236.. warningfree/
  227coreinputs. OneWinefilter30guards/8044actualsigned-unsignedloadprefixes/all4684B,
  12isolatedhighbit/102originalrandomprefixes(all34lists*3seeds), RNG/index/mask/
  address/destination/callee-SP/outerRAM-VRAM-OAM-palette-ROM exact.40modelguards/
  516accepted+516failedpreimagehashes/38sourceguards/helper0, notfullABI/nativeclock.
  Now e82-d81/index73 publishescoordinates:75files74resources/all73prior exact,
  headerdependency/691spans4684B, immutable4roles+XY/list-headermasks, local/global
  pages70B..70C andprior-templateleft/header-right composition/index0payload.
  Three warningfree518source builds/accepted0f85fbf7..; firstWinepub-attachPASS,
  twoharness-onlynativefailures fixedwithBLpair/twofamilystopguards, finalremaining
  filter14attach3fullsetcached/22native
  fullstates-clock poisoned4684rawB(18loads/index0gate/3randomprefixes),
  21compilednewB/2templateB/13headerB;
  all4684B/4rolesXY/691lists323headers/all691boundaries/LoadUndo/DMA3 PASS.
  253guards518hashes516modelpreimages exact/helperonlyinterruptedcopy;
  originalguest/ABI unchanged. CoordinateactualCLI nowpasses create/reuse/700:
  75files exactruntime/74IDhash/77paths no-write/28859oldnativeblocks,
  20reads20B/1lists1headers/1positiveroles/X-Ypositive.
  One warningfree518source productionbuildb467d4ad../canonicalhashmtime restored/
  186guards/helper0. One Wine700coordinate ordinary fullpair nowpasses every
  fullstate-clock/classic-central-wide/OAM/5audio; all10counts exactCLI20reads20B/
  placement0=20/XY10each/1list1header, other3rolesunobserved. Optional5lineguard/
  oldvalidatorbyteexact, warningfree518source buildde67e015../104ordinaryguards+
  186closedCLIguards518hasheseach/helper0. CoordinateordinaryCASEclosed;
  all691lists323payloadheaders/all2850records/all442/fullABI remainopen;
  Following linkedmetadata28 nowhas declaredsourcecontract347x28/9716B710E04..
  7133F8,17roots/28B arithmetic/nextindependent4Bfamily2readers; physicalcountnot
  guestcap. D07A4low16index/low8selector, scalarfields/inlineview+12/fullflags+
  21bitqueries/default0; directu16+22/bytes24..26, opaque3/15/27 preserved.
  All347recordsroundtrip/74owners50488spansdisjoint/20windows14new6reuse/
  69pins/122guards/helper0. Standalone metadata28model nowlossless347rows9716B/
  unsignedfields/fullflags/opaque, fixedshape vs sourceauth separate; low16/low8
  lookup/scalar-vs-sourceaddress/defaultnodata/outsideowner retained. One warningfree
  corebuild358dba2c../520acceptedcopysources andoneWinefilter27guards/14517original
  prefixes(12088fullgetter1388tail1041inline)/8675actualsemanticB/1041opaqueroundtrip,
  9isolatedvariants342prefixes/RAM-callee-stack-VRAM-OAM-palette-ROM exact;
  36modelguards520hash+122sourceguards/helper0. Managede83-d82/index74 nowadds
  linked-metadata28.json/1span9716B/nodeps:76files75resources/all74prior exact,
  immutable5byteclasses/347recordmask/8counters/local-globalpages710..713,
  priorheaderleft/rawsuccessorright composition. One warningfree522sourcebuild
  040133f5../oneWinefilter14attach2fullset/23nativefullstates-clock with9716rawB
  poisonedAFTERLoad(17loads6fullD07A4getters/twostop-safeanalyses),31compilednewB/
  1priorheaderB/all9716busB5classes347records/all347ends/LoadUndo/DMA3fullstate-clock.
  216actualguards522hash520modelpreimages/helperonlyinterruptedcopy;
  guestlogic/save-native-CAPI ABI unchanged. Actualmetadata28CLI registration
  nowpasses create/reuse/startup:76files exactruntime/75IDhash/78paths no-write/
  28859cacheblocks. One warningfree522source productionbuild0acb352e../3label-only
  overlay inversebyteexact/521othersources exact/canonicalhashmtime restored/
  220guards/helper0. Fourordinarylocators Giza700/Jobs540/Abilityset700/
  BattleTechRinfo800 haveall8counts0; observedmenus/normalinputderivation, not
  consumption/fullpair. Those negatives stay closed. A later normal dry-ground
  Steal menu now closes positive ordinary use:42frames22669→22711,15reads20servedB/
  halfword10-byte10/fiverecords; inline-flag-opaqueordinary0. ActualCLI0acb352e..
  and one Wine fullpair match all8counts/everyfullstate-clock/classic-central-wide/
  OAM/5audio. One warningfree523source testbuild5a6c81be../optional5lineguard exact
  oldvalidatorpreimage/test-only normalobserver; sevencaptures1182frames oncopies,
  no manualguestedit. 177guards523hash+220closedCLIguards522hash exact/helper0.
  Source-model-runtime-CLI/ordinaryCASE closed; executed effects/all347/fullABI open;
  Following four-byte family now has its own source contract:7133F8..713744,
  844B/211 physical slots, six roots/u8*4 and independent successor8B stride.
  Four unsigned fields;1041 parent references/194 distinct slots/zero-index3/
  outside0, no211 cap or global null/selector cap. Zero slots0/1/209/210 remain
  distinct, trailing reserve/padding undecided. All844B exact/75owners50489spans
  disjoint/13windows6new7reuse45pins112guards/helper0; static inspection only.
  Standalone records4 typed model now keeps all211x4/844B with strict shape/order/
  parser vs exact base auth; u8 encoded addresses retain outsideowner211..255,
  no null/cap. One warningfree corebuilda7270580../525frozen233coreinputs andone
  Winefilter20guards/1567original calls-prefixes (1055field probes/all844B,
  512full global-instance setters/all256indices/low8aliases/outsideaddresses),
  2isolatedhighbitvariants10prefixes/RAM-callee-SP-LR-IWRAM-VRAM-OAM-palette-ROM
  exact.125guards525hash+112sourceguards/helper0. Schema1/e83-d82 unchanged;
  Managed e84-d83/index75 now77files76resources/all75prior exact/one844Bspan;
  immutable four-field owner/211slotmask/sevencounters/page713/local-global,
  prior metadata28/raw successor composition. ThreeWindowsbuilds firstfailed
  borrow+stale menu cache/secondwarningfree-unused/thirdwarningfree33144a8c..
  current551source baseline; pre-Wine LDR-alignment expectation fix only.
  OneWinefilter publication14attach2fullset/21nativefullstates-clock/all844rawB
  poisonedafterLoad:41compiledinstr/21new6priorB/fourstop-safe groups. All844busB/
  fourfields211slots/all211ends/LoadUndo sameArc/DMA3fullstates-clock pass.
  237guards551accepted+two551diagnostics/525modelpreimages/helperonlyinterrupted.
  ActualCLI registration nowPASS: one warningfree551source productionbuild
  bcf632fa../no overlays/canonicalhashmtime restored; threeWinechildren all0,
  all77runtime files/76IDhashes/79managed paths no-write/28859nativecacheblocks.
  New normalStealArmor A8hold12/120f22711→22831 reachesTARGETscreen/all7counts0;
  closednegative, no fullpair or endpointffstate.187guards551CLI+551runtimehash
  exact/helper0. Positive following-turn case nowCLOSED:40frames24755→24795,
  223reads/servedB/fieldcounts64-132-11-16/twoslots, all7capture-CLI-fullpair counts
  exact. OneWinepair every40fullstates-clock/classic-wide/OAM/5audio passes.
  Onewarningfree552source hosttestbuilda5b2fc84../normalobserver+optional5lineguard/
  inversevalidator exact/coreunchanged;10normalcaptures2084f oncopies, no guestedit.
  210uniqueguards552hash+187closedCLIguards551hash/helper0; allfourfields observed,
  notall211/fullABI/attack-success/global. Following8Bfamily now has its own
  [source contract](../rom_data_gameplay.md#european-conditional-eight-byte-record-source-contract):
  713744..713B3C/127x8/1016B, zero-gate skips, distinct first/second flag and
  counter predicates, live encoded keys and original output pointers. Fullu32
  filter/capacity retained; no entry capacity0 guard, two callers pass64.
  76enabled/51zero slots,105live references63keys/onecounteroperand2; allbytes
  exact/disjoint76owners50490spans/8windows6new2reuse93pins96guards/helper0.
  Standalone conditional_records8 model nowstrict/lossless127rows1016B/onespan,
  baseauth/8windows separatefromshape; select retainsfullu32context/capacity,
  originalpointers/livecallmultiplicity/oppositepredicates, no clear-helper model.
  Onewarningfree corebuildc6f5406d../554frozen239coreinputs andoneWinefilter pass
  24guards/16base+93synthetic original D56CE..D57A4loops/realflag-countercallees;
  612actualbaseB/134syntheticB/all8lastslot vs1016B roundtrip. EWRAMexactexcept
  expectedoutputs/IWRAMexcept4Breturnlinkscratch/stack-r11r12/VRAM-OAM-palette-ROM
  exact;120guards554hash+96sourceguards/helper0. Entryclearhelper/epilogue/fullABI
  excluded. Managed runtime now e85-d84/index76/conditional-records8.json with
  one1016Bspan/nodeps/78files77owners/all76prior exact; immutable6fieldclasses/
  127slotmask/9counters/page713 composes priorrecords4/rawsuccessor.17distinctnative
  fullstate-clock cases/all1016busB/all127ends/failedattach/LoadUndo/DMA3 pass.
  Native path631servedB includes2existing interp_one fallback reads; testobserver
  counts thatcallback too, preserving original strict totals/state/clock.
  No runtime/CAPI/guest fix or ABIchange; [combined evidence and limits](../rom_data_gameplay.md#managed-conditional-records-and-typed-runtime-reads).
  ActualCLI nowregistered: one warningfree568-source productionbuild72d44985../
  nooverlays/canonicalhashmtime restore; create-reuse-neutral1 all0/78runtimefiles
  exact/77IDhash/80managedpaths nowrite/old28859cache. Oneprivateindex75->76assert
  correctedaftercreatedchild0, no prepare/buildrepeat;189guards568CLI+568runtime
  hashes/helper0. Startup24755→24756 hasall9counts0. OrdinaryRumorsentry nowPASS:
  42f15181→15223/468servedB/sixclasses127-76-152-76-34-3/127physicalheads inclzeros;
  all9capture-CLI-fullpaircounts exact/every42fullstates-clock/classic-wide/OAM/
  5audio. One569source testbuild63584945../normalobserver+optional5lineguard,
  inversehub-validator exact/core-runtime unchanged;2normalcaptures254f oncopies,
  no forcedguestedit/old620or397route replay.143guards569hash+568CLI/3roothashes/
  80nowrite/helper0; no selectedrumor/outcome/allbranches/fullABI claim.
  Declared8Bchain closed. The following [mission-record source contract](../rom_data_gameplay.md#european-mission-record-source-contract)
  now establishes713B3C..71C73C/512x70/35840B from two independent1..511 scans,
  getter-index0 and separately rooted successor byte arrays.65selectors retain
  unsigned packed/LE/scaled/mapped values and7addressselectors/6inlineoffsets,
  original aliases and residual bits; no guestindexcap512 or300 inferred.
  Byte+45HEX69decimal retains the closed event join;512numeric references/
  173values/fouraliasgroups, zero caller-specific. Allbytes reconstruct exactly,
  disjoint77owners50491spans/19windows9new10cached/157pins110guards. Source-only:
  no build/game/fixture/oldproof replay. Strictmission_records70 model nowpasses
  full35840Broundtrip/strictshape vs exactROM+19windowauth; query retainslow16/8,
  computedaddressesoutsideowner/defaultSome0/unownedscalarNone diagnostic.
  One571source242coreinput Windowsbuild663e8b87../oneWinefilter21guards438base+
  260synthetic originalgetter/helper calls pass results/actualbyte-readorder,
  EWRAM/VRAM/OAM/palette/IWRAM exactincludingreturnlinks/SP-R4..R12/finalROM.
  Actualbase324B/synthetic54B distinctreads are separatefromall35840roundtrip;
  139guards571hash569preimages110sourceguards/helper0. e85-d84/78files unchanged;
  Managedmission runtime nowe86-d85/index77/79files78owners/all77prior exact,
  one35840span/10classes13stats/8maskwords/pages713..71c. Two573source Windows
  builds:733firstpubPASSthenfirstattachFAILoldshared/OBJ1024cap; userno-space-
  limits removeslegacyOBJ/BGallocationcaps+threeassertions, fullauthretained.
  Remaining-only455bdb1c Wine reusesSAME79dataset, passes14attach2fullset/
  41nativefullstate-clock(16load25getter)/all35kbus512ends/LoadUndo/DMA3; no pub
  replay. Native52servednewB5priorB/allcallbacksattributed,168entries69dispatch
  coverageonly, notwholecatalog native.860guards573accepted+573diagnostic/571
  model/eightcurrenthashes/78priorfiles exact; helperonlyinterruptedcopy.
  ActualCLI nowPASS: onewarningfree573source production738b7f20../nooverlays/
  canonicalhashmtime restore; Winecreate-reuse-neutral1 all0,79files78IDhash/
  81nowritepaths/28859oldcache. Privategeneratorindextoken fixedBEFOREWine, no
  sourcecopy/build/actualprepare replay. Startup24755→24756 all13counts0, no
  action/navigation/newcheckpoint/ordinaryacceptance.195guards573CLI+573runtime/
  helper0. OrdinaryGizaPlainsinfo nowPASS33f15017→15050, fiveSERVEDpackedB/one
  record/other9classes0; all13capture-CLI-fullpaircounts exact/every33fullstates-
  clock/classic-wide/OAM/5audio. One574source testbuild3d21e89c../passiveobserver+
  optional5lineguard/inversehubs/core-runtimeunchanged. One81fnormalDown3/A12
  capture oncopies; postcapturehelpermissingsteps repairedregistrationONLY/no
  replay. Cachedseedclassic/scale authenticated; newfullwide284x160/bitrep match
  FFSTATE andclassicGizaMISSIONinfo page1/3Rank1 inspected, no missionoutcome.
  133guards574hash+573CLI/3current/81nowrite/helper0; no allfields/all512 claim.
  Declared70Bchainclosed. The next [selection-byte source contract](../rom_data_gameplay.md#european-selection-byte-table-source-contract)
  now establishesC73C..C740/fourcountthresholds andC740..C75E/thirtyordinalweights,
  34B/twospans/allbytesroundtrip/nooverlap78owners50492spans. Sixwindows2new4cached/
  95pins96guards; firstfive-liveheader low3count0..5 guard<=3, live rawu8 status
  02002CC3+12*(low8ordinal-1), two30ordinalscans/numericzero/wrappingsignedrank
  mechanics preserved. Numerichelperinternals/RNG/fullABI unproven; gap2B remains
  opaque, eventC760 needs owncontract. Sourcecontractclosed; [typedmodel/stageproof](../rom_data_gameplay.md#typed-selection-byte-tables-and-original-stage-proof)
  nowPASS34B/twospans/one576source245coreinput buildb40503b8/oneWine18guards
  45base24synthetic prefixes+actual377BC. Independent results/livequery/source
  loadorder/fullRAM/SP-R8..R12/finalROM exact; all34base/32syntheticB observed
  distinctfrom34Broundtrip.123guards/574preimages/96sourceguards PASS/helper0.
  Fixed4/30u8/shape-source separate; full32wrappingsignedrank/nozero shortcut.
  No numerichelper/fullABI claim. [Managedruntime](../rom_data_gameplay.md#managed-selection-byte-tables-and-typed-runtime-reads)
  nowe87-d86/index78/80files79resources/78prior exact/twospans4-30B/no deps,
  fixedowner page71C/6stats/4+30positionbits/prior70B-rawgap composition. One
  warningfree578source246core e5b45668 build/oneWinepublication14attach2fullset/
  20nativefullstate-clock/all34bus/LoadUndo/DMA3 PASS;76servednewB5prior/all
  callbacksattributed/fallbacksource0, notdistinctunion.243guards578hash576model
  sevenimplementation/79prior exact; helperonlyinterrupted. [ActualCLI](../rom_data_gameplay.md#actual-selection-byte-cli-registration)
  nowPASS one578source production22af4132/canonicalhashmtime restore, create-reuse-
  neutral1 all0/80payloadexact79IDhash82nowritepaths/cache28859exact/192guards.
  Guest24755→24756 all6counts0; startupscopeonly. [Ordinarytravel33](../rom_data_gameplay.md#ordinary-selection-threshold-travel-equivalence)
  nowPASS one579source8f3bee97 observer+5lineguard/inversehub/core-runtime exact.
  Eightcallerwindows354pins/fourseedreadonlygates, normalGiza19049copy/A12 one
  129fcapture first97/19147→suffix33/19145→19178; discoverySTOP. SAMECLI one
  positive33/SAMEtestfullpair every33states-clock/classic-wide/OAM/5audio, all
  sixcounts exact1bus1SERVEDthresholdB1position/weights0/notall4or30.140guards
  579hash578CLI/threecurrent/82managed/oldcache-ROM-seed-canonical exact/helper0.
  Chainclosed/unordinario. [Event12sourcecontract](../rom_data_gameplay.md#european-event-record-source-contract)
  nowPASS71C760..71D360/256x12/3072B/onespan/allbytesroundtrip/nooverlap79owners
  50494spans; explicit255..1stride12/liveu8zero/nextindependent27root, notglobal
  indexcap.71D1EC=base+225*12/31physical SAMEowner, low8arguncapped/highoutside;
  selection1..30maps226..255. Fullu16keycallback highestmatch255..1/excludes0,
  conditionalprimary02/alternate04 andrawparam06-byte07-flags08/opaque0A2B retained.
  Fifteenwindows10new5cached/67pins112guards/80acceptedfiles exact/helper0,
  staticonly. [Strictmodel/originalprefixWine](../rom_data_gameplay.md#typed-event-records-and-original-prefix-proof)
  nowPASS581frozen248core a4b88a69/oneWine20guards51base37synthetic prefixes.
  PhysicalownerNone vs originalu16view/low8aliasoutside, full16keyhighest255..1
  row0excluded; source3072roundtrip/302base268syntheticBobserved/actualLDRorder/
  RAMindexstore-prefixpushes/SP-R8..R12/finalROM exact.133guards581hash/three
  current579preimages112source/helper0. [Managedruntime](../rom_data_gameplay.md#managed-event-records-and-typed-runtime-reads)
  nowe88-d87/index79/81files80resources/79prior exact/3072B/onespan/nodeps,
  SAMEalias225owner/8classes11stats/4maskwords256records/pages71c-71d. One
  warningfree583source249core9225a3bc build/oneWinepublication14attach2fullset
  22nativefullstate-clock/all3072bus8classes256records-borders/LoadUndo/DMA3
  PASS.296SERVEDnewB3prior/allAPIattributed including2newinterp_onefallback,
  notdistinctunion/allnative;51entry30dispatch coverageonly.253guards583hash
  581model/sevencurrent/80prior exact/helperonlyinterrupted. [ActualCLI](../rom_data_gameplay.md#actual-event-record-cli-registration)
  nowPASS one583source productionb9a60501/canonicalhashmtime restore; Winecreate-
  reuse-neutral1 threeexit0/81payloadexact80IDhash83nowrite/cache28859/194guards.
  Guest24755→24756 all11counts0/compatibilityonly. [Ordinarymissionentry34](../rom_data_gameplay.md#ordinary-event-record-mission-entry-equivalence)
  nowPASS one584source0b6581cb observer+5lineguard/core-runtimeunchanged/inverse
  hubs exact. GuardedWhiteFlowersGiza19409copy/A12 one34fcapture firstrel2/19412,
  suffix34 same19409→19443; discoverySTOP. SAMECLI child0/34f collectorOLDID
  TypeError fixedOFFLINEregistration/noCLIreplay; SAMEtestonefullpair34 every
  states-clock/classic-wide/OAM/5audio PASS all11counts4bus6SERVEDB/key2-primary2-
  alternate2/onephysicalrecord/other5classes0/notall8-256-alias/outcome.126guards
  584hash583CLI/threecurrent/83managed80IDhash/cache-ROM-seed-canonical exact/
  helper0, authenticatedwide284x160bitrep/classicmapentry inspected. Chainclosed/
  oneordinary;27Bnext wholecatalogextent/fullABI/liveeffects/indirect/Linux-USA-nativeWindows pending.
  [Observed27Bsource](../rom_data_gameplay.md#observed-european-parameter-records-with-27-byte-stride)
  now authenticates59 event-selected indices0..50/115..122,1593B/two runs, all256
  eventlinks/roundtrip/nooverlap80owners50495spans/27windows70pins125guards.
  Packed01..06, u16LE07-0A-12, signed0C-15, separate0E0F-1011 operands andopaque
  bytes retained. Rawliveu8 index uncapped;128-by-adjacent-root fit rejected,
  holes/highindices rawunowned. [Strictsparsemodel/Wine](../rom_data_gameplay.md#sparse-parameter-record-model-and-original-prefix-wine-proof)
  nowPASS586source251core49c72f02/onefilter25guards73base48synthetic12prefixclasses,
  results/readorder/RAM/SP-R8..R12/finalROM exact.95base20synthetic distinctownedB
  observed vs1593Broundtrip/739instructionscoverageonly.143guards584preimages/
  125source/inversehub exact/helper0. [Managedsparseowner](../rom_data_gameplay.md#managed-sparse-parameter-records-and-typed-runtime-reads)
  nowe89-d88/index80/82files81resources/prior80exact/two1593Btotalspans/nodeps,
  12classes15stats/59physicalslots/prior event+rawholes composition. First588
  warningfree51e19build publication14attach2fullset/17native PASS thenoracle
  priorbytecount2vs4failure; test-only corrected278d6d23/remaining16native+
  all1593bus59borders/LoadUndo/DMA3 PASS SAME82dataset13objects. No publication/
  attach/first17 replay.33totalfullstate-clock cases37SERVEDnewB5prior; no
  distinctunion/alldirectnativeclaim/fallback0onlyremaining16.291guards588accepted
  +588diagnostic252core586model/7current81prior exact/helperinterruptedonly.
  accepted-selection.json authoritative; originalselection diagnostic.
  [ActualCLI](../rom_data_gameplay.md#actual-sparse-parameter-record-cli-registration)
  nowPASS one588source productionbd99ac5b/canonicalhashmtime restore; oneWine
  create-reuse-neutral1 threeexit0/82payloadexact81IDhash84managednowrite/cache28859.
  Guest24755→24756 all15counts0/compatibilityonly;197guards588CLI+588runtime/
  ROM-seed-cache-loader-canonical exact/helper0.
  [Ordinaryreachabilitypartial](../rom_data_gameplay.md#ordinary-sparse-parameter-record-reachability-remains-partial)
  now has589source8f0c02b7 passive15stats/5lineguard/inversepreimages/coreunchanged.
  Eleven authorized normal captures now have all15counts0, including SprohmInfo,
  source-guided Nubswood navigation/travel and CyrilPub entry; no positive suffix/
  CLI/fullpair. Six additional selected bank gates are not liveness proof.
  One existingCLI neutral render inspection exit0 saves/adopts no state; readonly
  saved classic/wide/nativeUI projections show no readable Pub menu. Stop blind
  Pub/world navigation, no RNG resampling or inferred mission IDs.9sourcewindows/
  additional80B initializer and accepted scriptjoinopcode80mode2→fiveprograms/
  keys/events216..220/param116..120 retained; WhiteFlowers58param0/program163.
  Indirect/mode0override0 remainunproved.233guards589+588sourcehash/3current/
  84managed exact/diagnostics and initial receipts retained. Reuse SAMEobserver/
  CLI only for a new concrete normal source-specific trigger. Independent source
  contracts may progress; ordinarypositive/fullpair/fullcatalog remain pending.
  [Adjacent38Bsource](../rom_data_gameplay.md#observed-european-world-records-with-38-byte-stride)
  declares31observed rows/1178B at71E0E0..71E57A, all512mission packed indices0..30,
  same interiorrow1/loop7..30 and nextindependentu32pointerroot71E57C/gap2Bunowned.
  D347Clow8 vsD4C4Cfullu32R2 uncapped; fullraw fields/31thresholds/opaque00-06
  retained. Static12windows10new2cached/58pins107guards/81owners50497spans/nooverlap
  pass, no build-Wine-input/oldproof replay in source unit.
  [Strictmodel/Wine](../rom_data_gameplay.md#world-record-model-and-original-prefix-wine-proof)
  nowPASS one590source/254core ed4d65c8 warningfree build/oneWinefilter19guards/
  41base19synthetic original prefixes.1178roundtrip/23base6syntheticdistinctB,
  actualresults-loadorder/fullRAM+expecteddestinations/SP-R8..R12/finalROM exact;
  353instructions coverageonly, no helper-RNG/fullABI/nativeclock/ordinaryclaim.
  26unit107sourceguards/588preimage/3current/inversehub/helper0 exact. Runtime/
  [Managedruntime](../rom_data_gameplay.md#managed-world-records-and-typed-runtime-reads)
  nowPASS e90-d89/index81/83files82resources/prior81exact/one1178Bspan. Immutable
  page71E composes sparse27/rawgaps/nextroot;5classes8stats/one31mask/noABIcap.
  One592build271dceba warningfree/collectorlabel offline adopted SAMEexe; oneWine
  publication-interruption/14attach2fullset/27native fullstate-clock10families,
  23SERVEDnewB5prior/fallback[0, 0],all1178B31records/borders/LoadUndo/
  DMA3 pass. 269guards592frozen/255core590model/7current82prior exact;
  helperonlyinterrupted, currentobserver preserved/excludedfrozen only.
  [ActualCLI](../rom_data_gameplay.md#actual-world-record-cli-registration) nowPASS
  one592source production6d95f705/canonicalhashmtime restore; oneWine creation-
  reuse-neutral1/threeexit0/83fileexact82IDhash85managednowrite/cache28859exact.
  Guest24755→24756 all8counts0/registrationonly;201guards592CLI+592runtime/
  ROM-originalseed-cache-TinyCC-canonical exact/helper0.
  [Ordinary45](../rom_data_gameplay.md#ordinary-world-record-counter-expiration-equivalence)
  nowPASS one593sourceee939288 observer8stats+5lineguard/core-runtimeunchanged.
  Initial360IN13travel negative/counter32→30; fourreadonlybankgates selectIN08
  livecounter13=1/normalcursor96 thenonepositiveA12capture189f3116→3305,
  first157/3274→rolling45suffix3260→3305/discoverySTOP. SAME6d95CLI onepositive45/
  SAMEtestonefullpair every45states-clock/classic-wide/OAM/5audio exact; all8stats
  2bus2SERVEDB(1threshold1byte01)/onephysicalrecord/other3classes0/notall5-31.
  165guards593+592source/3current/inverse/85managed/cache-ROM-originals exact/
  helper0, authenticatedclassicworldtravel/no arrival-missionoutcomeclaim.
  Chainclosed/oneordinary; no oldproof/negativebound/route replay ormetrics.

- The [EUR indexed graphics archive source contract](../rom_data_indexed_graphics.md)
  declares four five-slot language pointer groups/20archives649programs,24disjoint
  source spans74907B/278336decodedB. Original05228/0537C uses BE offsets and a
  fixed external dictionary, distinct054A0;4137 stays opaque. Raw languageu8
  selectors and low16start-count semantics are uncapped by host ownership.
  Exact source/output reconstruction and22authenticatedwindows/101pins/121guards
  pass without a runtime build/run;82acceptedowners/83files stay unchanged.
  Other16nearbygroups, dictionary tails/gaps/context3710D4/globalextent remain
  unclassified. Strictmode0width5model nowpasses one595frozen/257core Windows
  buildd78a74d9 warningfree/oneWinefilter37guards/20wholearchivecalls+9start-count
  boundaries+9synthetic+28rawlanguageprefixes. Source/output/status/globals/
  SP-R4..R11/RAM outside68B originalstack scratch/VRAM-OAM-palette/finalROM exact;
  26unit121sourceguards/593preimages/3current/inversehub preserved.
  Runtime nowPASS e91-d90/index82/84files83resources/all82prior exact/24spans,
  immutablepacket-dictionarysource views/7classes12stats/20bank649program4group
  masks/original decode-upload work retained. First597buildfailsBus testimport
  only; corrected597/258core0732b3b9 warningfree/productionunchanged,ONEWine
  publication-interruption/14attach2fullset/35fullstate-clock nativecases
  (28load+1oneprogramwrapper+6rawselectors)/3stop-safe families;272SERVEDnewB4
  priorworld/fallback[1,0]/232instructions59dispatch coverageonly. All74907busB/
  all24edges/LoadUndo/DMA3mixed64B pass,282guards597accepted597diagnostic/595model/
  7current83prior exact/helperonlyinterrupted. Accepted-selection.json is
  authoritative; firstselection remainscompileFAILdiagnostic. ActualCLI nowPASS
  one597sourceproductiond8767645 warningfree/canonicalfinallyrestore, oneWine
  create-reuse-neutral1/threeexit0/all84runtimefiles83IDhash86managednowrite/
  cache28859exact. Guest24755→24756 all12counts0/registrationonly;201guards
  597CLI+597runtime/84accepted originals preserved. OneordinaryMENU nowPASS
  one598source6d40a6a5 warningfree/passive12stats+5lineguard/core-runtimeunchanged.
  Authenticated453E2→1967Ccontext3→program2/oneStart12capture41f4519→4560/first
  9guest4529/discoverySTOP; SAMEd876CLIonepositive41 andSAMEtestonefullpair all41
  complete states-clock/classic-wide/OAM/fiveaudio exact.274reads277SERVEDB
  selector4-header6-index4-token67-literal17-dictionaryheader4-dictionary175/
  onearchive-program-group/all7classespositive/nonall20-649-4.144guards598frozen
  597CLI/threecurrent86managed/originals exact; authenticatedwide284x160/classic
  MENU Party-Clan-AreaList-System/MoonDay9. Otherordinary/mode3-widerwidth/fullABI-
  liveuploads and deferred platforms remain pending; no oldsource-model-native-
  CLIprepare/old320Down-A/world38daily45/negativeproof/metrics replay.
  [Next table-selected custom graphics](../rom_data_ui_graphics.md#additional-european-table-selected-source-contract)
  sourcecontract nowPASS eight rawu8language groups71E590/5A4/5B8/5CC/644/680/
  6BC/6D0→40unowned054A0banks,48spans53667sourceB/114880decodedB. Allpackets/
  source/output reconstructed,14windows10new4cached/48pins/111guards/nooverlap
  83priorowners50522spans/84files unchanged. Exactslot0..4physicalownership
  isseparatefromuncappedrawu8address, nofallback/skipinvented; originalscratch/
  livetask/helperwork retained,fulltransferABI pending. Known71E5E0tablealias
  remain separate; the seven raw groups now have the bounded source contract below.
  Global extent/tails/indirect references remain unclassified. Strictnew
  ui_graphics::table_selected model nowPASS one600frozen260core cb46ccb1 build
  warningfree/oneWine23guards40neworiginaldecodercalls56rawselectorprefixes.
 53667sourceB/48spans114880decodedB/cursor-produced-returned-size/SP-R4..R11/
  fullEWRAM/IWRAMoutside40B originalstack/VRAM-OAM-palette/finalROM exact.
  26unit111sourceguards/598preimages/3current/inverseUIhub preserved,1335470
  instructions coverageonly. Offline23B2Econtrollabel corrected0150words vs
  02A0halfwords/same1344B, no source/modelinputs orproofrerun. Managedruntime
  nowPASS e92-d91/same84files83IDs/UIindex15 56banks10selectors106bindings/
  allother82records-files-old16banks-twoinline-sharedOBJ exact.80newruntime
  bindings byteunion48canonicalsource53667B exact;6newfamilycounters old6UIstats
  unchanged/originaldecode-uploadwork retained. One601frozen260core d4a73eb2
  warningfree/ONEWinepublication-interruption14attach2fullset22fullstate-clock
  cases/3stop-safe families,499SERVEDnewB5priorindexed/fallback[3,0]/356visited
  instructions71dispatch coverageonly. Allnewsource53667B24328literal160selector
  40banks8groups/all48edges/LoadUndo/DMA3 pass/267guards601frozen600model/
  sevencurrent84prior exact. ActualCLI nowPASSone601production31c0f298 warningfree
  canonicalfinallyrestore/oneWinecreate-reuse-neutral1threeexit0/all84runtime
  files83IDhash86managednowrite/cache28859exact. Guest24755→24756 sixnewtable
  counters0/registrationonly;202guards601CLI+601runtime/84accepted originalsexact.
  Positiveordinary/fullpair/fulltransferABI/deferredplatforms pending; noold40
  model-16UI-20indexed-native22-35-41ordinary-CLIproof replay/game/fixture/
  navigation/metrics.
  Newtableordinary staysPARTIAL:twoSAME31c0CLI boundednormalprobes64targetconfirm
  23851→23915 and160Comboattempt23611→23771/all6newcounts0,83IDhash86managed
  nowrite/cache28859exact. Sourceinitializerlinknotreinitialization/liveness;
  PPMseparateHUDomissionpreventsclaimCombo reached. No observer-guardbuild/
  fullpair/endpointstate/longerretry/RNGresampling/oldroute recreation; blind
  discovery stoppedpendingconcreteconstructorcontext, diagnosticsretained.
  [Raw localized transfer sources](../rom_data_ui_raw_transfers.md) now declare
  seven groups/five slots/35 fixed views,42 spans/38220B, authenticated against
  all13 root literals and193 pins/32 windows with130 preservation guards.
  Six bank prefixes are224/160/768/1600/2048/2432B; the seventh owns only
  +0200..0380/384B. Original CpuSet/queued DMA halfword units are checked by
  fused BL targets; no length from pointer spacing. Four dynamic seventh-group
  views and their uncapped index metadata/lower prefix/tails remain unowned.
  Strict ui_raw_transfers model now passes exact source/LEword reconstruction,
  shape versus source authentication,91 raw selector prefixes/60 isolated
  original helper copies/25 original queue-executor DMA copies/21 guards.
  Complete memory guards/SP/callee/final ROM match; stack scratch36B for the
  queued executor includes its16B wait frame. Accepted603-source/262-core-input
  Windows build35bdc5fd is warning-free. Three builds/two Wine attempts include
  two test-only oracle corrections (stack extent, adjacent owned boundaries);
  model/source unchanged.44 unit/130 source guards,601 baseline source hashes
  and three current paths/inverse hub exact.6635 instructions coverage only.
  Managed runtime now e93-d92/index83 graphics.ui-raw-transfers/85 files84 IDs,
  all83 prior records/payloads exact. Typed raw words/pointers project42 spans/
  38220B with six separate counters; guest copy queues and transfer work remain.
  Two605-source/263-core Windows builds, accepted4b219adc warning-free; one
  Wine filter passes publication/interruption/14attach2fullset/23 complete
  state-clock native paths across four stop-safe families (12loads, CpuSet,
  queue executor, nine selectors), all bytes/42edges/sparse-prefix/prior-successor
  ownership/LoadslotUndo/four64B mixed DMA16-DMA32-CpuSet-FastSet cases.
  Native939 served new bytes/three prior indexed, fallback768/0;115 instructions
  and25 dispatches coverage only.309 preservation guards/605frozen603model/
  seven current paths84prior files/inverse hub exact. Pre-Wine test-only fix
  distinguishes prior indexed archive441608 from raw holes; product unchanged.
  Actual CLI now passes one605runtime-identical productionffda6c59 build,
  warning-free/canonical hashmtime finally restored; one Wine orchestration
  three child0/create-reuse-neutral1:85 runtime files84 IDhash87 managed paths
  unchanged/native cache28859 exact. Guest24755→24756/all six raw counts0,
  registration only.203 guards605 CLI+605runtime/85 accepted originals exact.
  One EUR/Wine 1200-frame clean boot now positively consumes the raw transfer
  resource (16 selector/5472 payload bytes served; four groups), with a matching
  per-frame full-state/clock/video/OAM/audio pair using the same native cache.
  [Scope and evidence](../rom_data_ui_raw_transfers.md#ordinary-european-boot-equivalence).
  Other ordinary paths, dynamic domain/full caller ABI and deferred platform
  checks remain pending; no performance measurement.
  Bounded [dynamic-view source discovery](../rom_data_ui_raw_transfers.md#bounded-dynamic-view-producer-discovery)
  identifies explicit selectors 7/8/9 and 1304 non-overlapping candidate bytes;
  the [bounded typed source model](../rom_data_ui_raw_transfers.md#dynamic-fragment-source-model)
  passes a focused EUR/Wine source check. The independent e94-d93 managed
  resource serves those seven spans with verified byte/boundary and one DMA
  state/clock check on Wine. Production CLI creation, reuse and loading of all
  85 resource hashes now pass on Wine for one neutral frame. The original
  dynamic routine and DMA also match complete state/clock across raw/typed
  interpreters and translated code for 30 isolated EUR/Wine combinations.
  A source-proved guest gate at 0200203d and a bounded negative Unit320
  observation narrow ordinary reachability, without establishing a positive
  case. Live-byte producers leave the global index domain open.
  The European UI resource e95-d94 now also serves the five-pointer clan table
  alias without duplicating its banks. Wine verifies publication and 42 original
  interpreter selector prefixes with full state/clock. Six original translated
  prefixes also match raw interpreter state/clock in 24 selected cases with
  typed callback reads. The production CLI creates and reuses e95-d94 and
  loads all 85 resources for a Wine neutral frame. One separate Bank entry
  shows two typed alias reads; a 120-frame same-input pair matches complete
  state/clock, video and audio. Other paths, Linux and USA remain pending.
  EUR e96-d95 additionally owns the conditional 74-row lookup at `6e6dbc`;
  outside-projection selectors preserve their original reads. Strict managed
  activation, selected translated reader prefixes and production CLI
  create/reuse/startup pass on Wine. An ordinary 540-frame Ability scene uses
  both fields and matches raw state/clock, video and audio each frame; other
  scenes and Linux/USA remain pending. See [lookup evidence](../rom_data_gameplay.md#ordinary-ability-scene-differential).
  EUR e97-d96 adds the adjacent conditional packed lookup for indices 1–33;
  Wine confirms managed publication, translated prefixes and production CLI,
  including preservation of the existing metadata14 alias. Ordinary use remains to verify;
  see [packed lookup evidence](../rom_data_gameplay.md#managed-conditional-packed-lookup).
  EUR e98-d97 also projects 23 conditional descriptor pointers and 684 records;
  the excluded bank-0 pointer and bank-22 gap stay raw. Wine validates managed
  publication, original translated prefixes, production CLI and a 540-frame
  Ability scene using pointers and fields 0/2. Later bounded Battle Tech and
  Steal scenes also cover kind, byte 7 and field 4 with Wine full-frame pairs;
  these do not establish all-record or all-scene coverage. See
  [descriptor evidence](../rom_data_gameplay.md#ordinary-battle-tech-descriptor-information).
  EUR e99-d98 adds descriptor pointer 0 and conditional bank-0 rows 1–15;
  row 0 remains raw. Wine validates managed publication, adjacent-owner
  composition and production CLI. Wine Item-info and battle Item-menu pairs
  cover its pointer and all represented field types in bounded scenes. See
  [bank-0 evidence](../rom_data_gameplay.md#ordinary-item-info-access-to-descriptor-bank-0).
  EUR e100-d99 adds 35 property-selected u32 masks; managed activation,
  adjacent-owner composition and an original native prefix pass on Wine.
  Production CLI create/reuse/startup and a bounded headgear scene differential
  also pass on Wine, with positive typed mask reads. See
  [mask evidence](../rom_data_gameplay.md#conditional-property-selected-mask-table).






  No forcedID/RAM/navigationguess. No furtherblindnavigation/manualflag/oldnegative replay or fullPASSclaim. Phase0 at371064 is now typed in e75-d74:
  3conditional unsigned thresholds16/8/8 +4old corrections,11B/two spans3/8,
  same68files67resources/all66other e74 records-files/old10B retained. No global
  phase cap; runtimephase/correction coverage partitions3/4bits. One first
  Windowsfrontendbuild/oneWinefilter passes4neworiginal/7attach/6native-state-clock/
  3DMA and rawedges; helperchanges visibilityonly. Firstproductionbuild/canonical
  restore +one actualCLI28 auto-prepare pass all68files/all3phases4corrections,
  36reads44B/28threshold16correction. Private logmatcher repaired offline, child0
  neverrerun. Samefrontend one NEW28frame fullpair8476→8504 passes everyfullstate/
  clock/central-wide/OAM/dispatch/allaudio; samples[61446,61446,7392,7392,61370],
  counts exactCLI.215guards/implementation/5lineguardpreimage exact, helper0paths.
  Constructorordinary is now closed by the Giza case above. Independent371034cycle projection
  now has a source/model contract:48signed-i8 increments, old-u8 unchecked lookup
  before stored-u8 increment/remainder48,255→0, signed add/sub taskhalfword+8.
  Exact reader/helper/literals/full48ownership authenticated; no axis/lifetime
  inference or global cap. One first warning-freeWindowsbuild/oneWinefilter
  passes23guards/58originalprefixes/8helperboundaries/all48rows/RAM-callee guards,
  77actualinputs-frozen/source exact. Isolated model leaves e75 unchanged;
  Managed/runtime now passes e76-d75/gameplay.task-cycle-deltas:69files68resources,
  one48B span/all67prior resources-files exact, typed/prior/raw composition.
  FirstWindowsfrontend warningfree/oneWinefilter passes9attach2fullsetguards/
  newfileinterruption/8complete-state-clock native prefixes with48rawB altered/
  5explicitcompiledB/all48rows/mixed2+2owner read/3crossingDMA-SWI/Load-Undo and
  unavailablefilelifecycle.154actualguards/compiledimplementation exact; required
  cleanup retires only interruptedcopy. ActualCLI nowpasses firstproductionbuild/
  canonicalhashmtime exactrestore/3Wine prepare-reuse-runc0 commands,69generation/
  71managed files/all69accepted/all68runtimeIDhashes/oldcacheload. Managedpathset/
  hashmtime no writes onreuse/startup,226guards/source exact, helper0paths. Zero
  execution/cycleusage in thatstartup provesregistration only. Ordinary cycle
  nowpasses one source-backed newneutralBank48bound3660→3708: exactnormalstatecopy,
  actualCLI48reads48B/all48rows/current71managedpaths-hashmtime unchanged. Source
  initializer18D6C/default03 andgeneric015E48/03-07 plusreadonlyBankflags03candidate
  selectscene; setter18E5C preservesold04/08 withnew01, argument03alone insufficient.
  11/3/89rawBLcandidates notwholecallgraph/each-taskliveness. FirstWindowsfrontend
  warningfree/5lineoptionalpositiveguard/oldloopbyteexact. OneWinefullpair passes
  everyfullstate-clock/central-wide/OAM/dispatch/allaudio, samples
  [105336,105336,12672,12672,105260], counts48/48/48exactCLI.97guards/compiledsource/
  preimage exact, helper0paths. No navigation/manualRAM-task-ROMflag/cartridge
  writes, productionrebuild or oldBank12/F09/sourcefilter acceptance repeated.
  Declared cyclechildren closed; broader scenes/all-use/fullmigration remainopen.
  [Ordinary cycle](../rom_data_gameplay.md#ordinary-european-task-cycle-scene).
  [Cycle contract](../rom_data_gameplay.md#conditional-european-task-cycle-delta-source-and-model).
  [Phase-zero extension](../rom_data_gameplay.md#conditional-phase-zero-threshold-extension).
  [Constructor animation](../rom_data_obj.md#conditional-constructor-obj-animation-source-and-model).
  [Source proof](../rom_data_runtime.md#lower-bus-and-host-residue-source-contracts).
  USA8-7 unchanged; Linux/nativeWindows/new32:9 pending, no newfixture or
  measurements. Other threads/scenes/readers/typed HLE bypasses remain separate.

- The [Bank bus/host census](../rom_data_runtime.md#bank-bus-and-selected-host-rom-inventory)
  now covers interpreter and selected host access beyond the old native-onlyAPI:
  optional explicit all-site capture keeps old word filters/address keys/default
  builds unchanged. One first warning-freeWindows feature build/one Wine filter
  passes newmode probes and one currentBank12decoded arm, projection exact and
  retainedold dispatch/audio counts exact.7430reads/7532sites/497scoped7035None,
  7083imageB/1434unowned/all3683oldnativeB included.992prior-code+320executedliteral+
  32dispatchB=1344codesupport;90dataB remain source/model work at4114FC/4165A0,
  u16task readers2C5E0/2C60C distinct from establishedu8animation producers. No
  root-cardinality/asset-extent/global127cap inference. Cycle21–32reads from
  interpreterLDRB18E2A explainoldNativeAPIgap.95guards/compiledsource/default
  preimage/Atlas exact, helper0paths. No oldfullpair/sourceclassification replay,
  newfixture/navigation/productionbuild/revision/ABI or measurements. e76/USA8-7
  and deferredplatforms unchanged; censusclosed. Newu16taskOBJ source/model now
  authenticates4producerprefixes/50lists140OBJ/7seq49frames/65spans1378B, all68old
  owners disjoint. Sevenunreferencedlists/3zeroseqheaders retained, no invented
  hold/alluse/globalcap/archive claim. Sharedlosslessfields/strictJSON/source
  projection/u16queries preserveoriginaltiming width. Firstcorebuildwarningfree/
  Wine model23guards pass then test-onlyctorr0return expectation fails; product
  unchanged, correctedtest/secondwarningfreecorebuild/remainingonlyWine pass
  ctor23/update93/producer6/RAM-callee/aliases/bounds. Oldmodelstage notrerun or
  fullfirstfilterPASS claimed.88guards/199coresources exact, helper0paths; coredev
  serde_json avoidsparallelfrontend compilation at that model milestone. The
  subsequent e77-d76 managed/runtime child nowpasses across staged Winechecks:
  70files69resources/all68prior files-records exact,65spans1378B typed through
  guest/native/bus/host,27originalstate-clock prefixes/202compiledB,217mixedreads/
  DMA3/missingfiles Load-slotUndo ownerretention and50host lists140OBJ/95reads.
  Shared immutable projection preserves oldbody/pagegate andguestu16logic/ABI.
  First interruptedfilter retains closedstage receipts, not claimed fullPASS;
  remaining-onlyhost filter completes theunit. One concurrent-sourceguard build
  rejection and one test-only privatefieldcompilefailure are retained; corrected
  stable snapshot build/Winehost pass.165guards/496snapshot/currentROMinputs exact,
  helperretires onlyoneexpected interruptedcopy. ActualCLI child nowpasses one
  warningfreeproduction snapshot build/threeWinecommands:create/reuse/runc0,
  all70generationfiles69runtimeIDs exact/72managedpaths unchanged/all9newfields0.
  Canonicalexehashmtime restored;154guards496copiedsources exact/helper0paths.
  One ordinaryBank12positive child nowpasses on exact existingnormalfixture
  copies: CLI292reads346B/all6classes20host, one5lineopt-intestguard/oldvalidator
  byteexact, firstwarningfree snapshotWindowsbuild/oneWinefullpair all12states/
  clock/images/OAM/dispatch/all5audio, all9countersexactCLI.94guards496snapshot
  sources/currentROMinputs exact/helper0paths. No allbank/list/fullgame claim;
  constructorordinary separatepartial. USAe8-d7 and deferredplatforms unchanged.
  ConcurrentDBGwork staysseparate.
  [U16bank contract](../rom_data_obj.md#european-obj-banks-with-u16-task-counters).

- The [following live-slot helpers](../rom_data_gameplay.md#live-state-helpers-following-the-slot-job-lookup)
  D05E8/D0618test/count nonzero firstwords in20RAMslots at02002E78, whileD0650
  increments livebyte02001F68onlywhenold<=98 (largervaluesunchanged). Exactthree
  bodies/68halfwords/30criticalpins/6literals authenticateRAMstate, no newimmutable
  asset/root/resource/revision or global20inputcap. Raw3/2/1caller candidates are
  not whole-game/liveness proof; partialD0670separate. Sourcechildclosedstatic-only,
  e71/USA8-7 unchanged, no newbuild/game/test/fixture/navigation/measurements.

- The [known FFFF numeric reader fields](../rom_data_unit_obj.md#typed-model-for-known-ffff-reader-fields)
  now have a strict lossless model: u16property/u32encoded directory/query65535,
  two spans6B and980conditionalmetadata52/archive49witnesses2240refs. Existing
  property/byte-alias/valid-sequence models remain unchanged; no asset target,
  pointer fix or index cap is inferred. FirstWindowsfrontendbuild warning-free/
  frozen83ffcd75.. and one firstWinefilter pass13guards/3numericroundtrips/
  independent980records+6B/twooriginalreaders28instructions/callee-fullRAMguards.
  156build157Wineguards35compiledinputs exact, all63current owners disjoint.
  Modelclosed. [Managed publication/typed bus](../rom_data_unit_obj.md#managed-ffff-reader-fields-and-typed-bus-consumption)
  now passes e70-d69/65files64resources/all63oldrecords-files/e69retained, actual
  metadata52/archive dependencies. Two first Wine filters/one warning-freeWindows
  core/frontend build pass20attachguards/4dependencyguards/lifecycle/unavailable
  directory/all6Bpoisoned and14original-interpreter-native fullstate-clock paths,
  liveBIOSunmapped14/rotate16/zeroearlyreturn/boundaries-SWI-DMA. Only two numeric
  fields are owned; no valid sprite target is inferred. ActualproductionCLI
  create/reuse-no-write/oldcache-oneframe now passes65generation67managed/
  64resources/all65accepted; canonicalhash-mtime and originalF09/cache exact.
  Ordinarypositive/reachability/equivalence remain pending. Newreadonly81snapshot
  locator finds159potentialFFFFcontexts/61snapshots (allkind0/job0/alt0),157null
  bodypointers/twooutsideEWRAM/no livebodytask lead. It infers neither actorvalidity
  nor globalunreachability; no guest/build/fixture replay. USAe8-d7 unchanged,
  unmapped0x14 live bus semantics retained. Closed71pin/60query/A4/oldoracles/scenes notreplayed; no
  fixture/navigation/performance-space work. Otherplatforms remain pending.
- The [additional property getter callers](../rom_data_unit_obj.md#additional-property-getter-callers-and-the-ui-byte-input-window)
  have a source contract for all five retained direct calls. Two additional u16
  paths update the task property while retaining allocation; their known concrete
  assignments alias the existing182words. Two UI callers use a byte input to
  select a descriptor. Their exact256-query width window shares182property words
  and two palette-offset words, leaving72newwords/144B in four disjoint ranges.
  All62current owners authenticate; no256catalog extent, livegettercap or universal
  ordinary reachability is inferred. The new lossless72wordmodel and firstWine
  filter pass14guards/512UIprefixes/all512readB/eightskips/fouru16comparisons/
  fourbytewriters/sixadditionalsetterprefixes/wholeRAMoutside destinations plus
  VRAM/OAM/palette guards.109build110Wineguards/13compiledinputs exact; no fullUI/
  allocator/native-clock/ordinary claim for02. EUR e68-d67 now publishes64files/
  63resources/all62oldrecords63files/e67retained withactualpropertyword+palette
  dependencies. Newimmutableowner servesonly72words144B/fourspans/ordinalbitmap;
  184sharedwords keeppriorowners. FirsttwoWine pass17attach+fourdependencyguards/
  lifecycle/all144typedB72records plus original-interpreter-compiled fullstate-
  clock getter/UIprefix/fullword/liveRAM/mixed paths. Fourrangeedges/width-mirrors/
  realBIOSbus/SWI-DMA pass; API2new2property/2new2palette/2new2raw, CpuSet2new4property/
  FastSet28new4property/DMA4new2palette. ActualproductionCLI firstprepare/reuse
  nowpasses64generation66managedfiles/63resources/twodeps/all64accepted/no-write;
  authenticF09oneframeoldcache attachesanduses1read2aggregateB/oneword, smokeonly/
  nofullsceneequivalence.182activeguards/23compiledinputs exact; canonicalrestored.
  NewordinaryF09Wait3530fullpair nowfirstPASS 476reads/952aggregateB/4words;
  everyframe fullstate-clock-central-wide-OAM-dispatch-allaudio equal, allfiveaudio
  streams nonzero. Other68words targetedreaderonly; 183guards/24compiledinputs
  exact. Optionalfive-line positiveguard only, no duplicateCLI3530 or oldscene/
  source/model/runtime replay. Declaredordinarychild closed; broadercoverage,
  fullUI/allocator andotherplatforms separate. USAe8-d7/FFFFraw6B unchanged. No fixture/navigation/performance measurement or old
  acceptance replay. Unknown u16, changed actors and computed
  indirect calls remain open.
- The [property-selected static UI OBJ lists](../rom_data_obj.md#managed-ui-property-list-extension-and-typed-consumption)
  now extend the existing catalog in EUR e69-d68:126refs/74lists175objects1198B.
  Four conditional A4/B2 roots flow through1F6Cto1500; source proof is closed.
  A4adds one14B/two-object list; B2retains its existing typed owner. Same64files/
  63resources/format/dimension dependency/runtime handlers; all73oldtypedlists,
  62otherfiles/records ande68 retained. Two first Wine filters pass model/source
  roundtrip/eight negative guards/publication/repair/move/lifecycle plus two
  original-interpreter-compiled draws/fullstate-clock/709instructions/4dispatches/
  36compiledB/twohostpublication checks. Bothboundaries/widths/mirrors/realBIOSbus/
  mixedAPI2new2B2/DMA-CpuSet14new2B2/FastSet4prior14new14B2 pass. CorrectedWindows
  build warning-free; first failure was test-only constant qualification.
  77build79Wineguards29compiledinputs exact. ActualproductionCLI firstprepare/
  no-write reuse64generation66managed/all64accepted/63resources/74sources and
  authenticF09oneframeoldcacheattach pass; 148activeCLIguards/29inputs exact.
  Declaredordinary05nowclosed: firstDispatch1840actualCLI A4guest-hostpositive/
  15evaluationB; firstexactWinepair passes all1840fullstate-clock-central-wide-
  OAM-dispatch-allaudio, all5streams actualsamples. Five-lineoptionalA4gateonly,
  firstWindowsfrontendbuild warning-free/frozen427845f2../152guards29inputs exact.
  New25halfword/sixswitchword join6375C/D2D44→1092→063230/02AF14→animation24→A4
  explainsordinaryroute. F09/Party/bank andearlyDispatch1400negative stayclosed;
  1400missed1600input, original1840positiveevaluationproof justifiedcorrectedbound.
  Sourcecheckpoint/input hashes-mtimes exact/authorizedcopyonly/no cartedit.
  No oldsource/model/CFG/scene replay, performance/storage or fullUI/globalclaim.
  USAe8-d7/Linux/nativeWindows/new32:9 and broader indirect sources remain open.
- The [EUR Color-Mode unit/UI palette model](../rom_data_color_palettes.md)
  follows five original CF49C..CF5DC helpers with70direct callers,40constant
  supplemental calls/four slice callers and51metadata14special projections.
  Its30spans5750B=2873u16colors+fourselectors are disjoint all52e57owners.
  Strict lossless model and corrected reduced Wine selector/copy-prefix oracle
  pass all5750actual-read bytes:165pointer cases/99copies/30staged readers/
  54special copies/12slice prefixes/eightguards. Live low2modes/low8arguments,
  companion mode1alias, source gaps and guest RAM remain intact. The first
  oracle failed from a destination-register seed assumption, corrected using
  original MOV/LDR decoding; no model difference. EUR e58-d57 now publishes and
  consumes typed colors/selectors with metadata14dependency:54files53resources/
  all52prior decoded/resource records/e57retained/USAe8-d7. Two new Wine filters
  pass17guards/lifecycle/all5750B/28original-interpreter-compiled complete-state-
  clock paths, low8index/mode3/liveRAMpalette/30edges/mirrors/BIOS-SWI-DMA and
  mixed2new+20raw+4oldUIB/exact2/4charges. Source/model/oldproofs stay closed;
  actualCLIcreate/no-write reuse56managed/all54generationfiles and one positive
  ordinaryLutia1700 fullstate-clock-image-OAM-dispatch-allaudio pair now pass.
  New1102aggregatecolorB=[512unit512companion78supplemental]/fourfragments;
  Party320/boot240negative locators stay closed/no fullpairs. Other26ordinary
  fragments/othermodes/shared-special-selector/wholeallocator-scheduler and
  newLinux-USA-nativeWindows/32:9 remain pending; no performance/space campaign.
- The [normal EUR terrain sources](../rom_data_maps.md) already belong to the
  162 BG descriptors and352packet programs: original1B688/state6→1AE88 routes
  all486field04/08/10links to existing typed owners. Field10initializes the
  normal16×16height/flag grid02007CB0; live context02007F10+4/+8/+9 and mutable
  flags remain guest state. Static link/source/49owner checks pass without a
  new decoder, revision or guest run. Other context layouts/ten-byte1DFB4caller
  remain unknown. Separate raster palette bank4872B0..489BB0now has a verified
  static32record/324link/33range/10496Bcontract, with160u16colors per record.
  Original161halfword copy crosses into the next header or the existing
  alternate-palette owner, then wraps the extra destination word. Preserve
  this read/timing boundary. Strict lossless model/one Wine original-reader
  oracle now pass all10496B/324links/15guards and36complete322Bcopies including
  the neighboring013Cword before wrap; unsigned/signed/raw/sentinel/deferred/
  synthetic bit15/range/queuedDMA cases pass with component RAM/SP/callee checks.
  Managed e55-d54now publishes51files50resources,
  all49prior decoded/e54retained, actualBGdependency and one immutable owner.
  Two reduced Wine filters pass17guards/lifecycle/all10496B and28original-
  interpreter-native fullstate-clock paths, compiled[15index46header24color]B,
  original copies/wrap/queuedDMA/mixedoldowner/rawfallback/BIOSbus-SWI-DMA.
  ActualCLIcreate/reuse-no-write53managedfiles/all51generation equality/oldcache
  and one1500frame genuine coldboot Winefullstate-clock-image-OAM-dispatch-audio
  pair nowpass50880aggregateB=[2index322header50556color],701guards. Programmask2
  includes neighbor-header touches, not two full palettes/distinctB. Other
  contexts/whole camera-rasterABI/newLinux-USA-nativeWindows/new32:9 remain
  unclaimed/pending, USAe8-d7unchanged.

- The root Cargo workspace and `AndroidPort/` are separate trees. A root fix
  does not automatically update AndroidPort.
  Android Docker builds use `scripts/android/build_android_docker.py` (default
  maintained AndroidPort; `--project-dir .` explicitly selects root). The port
  keeps its Dockerfile/driver/exporter self-contained. Filtered public contexts,
  separate Cargo/Gradle/signing caches and APK/hash/test export avoid writing
  build intermediates into either checkout; see [Android Docker](../android_docker.md).
- `crates/recomp/src/play.rs` connects the desktop event loop, input, menu and
  presentation. `crates/menu` owns UI state and rendering; `crates/input-config`
  owns persisted bindings and combo capture/latching. `crates/screen` presents
  the framebuffer and UI layers.
- `crates/gba-core` implements the emulated machine. `crates/recomp-core` emits
  native code; TinyCC compiles it. Runtime execution can still use interpreter
  fallbacks. Inspect cache revision/ABI requirements when changing emission or
  guest semantics; a host-only overlay does not itself change the guest ABI.
- Desktop packaging covers Windows and Linux. The release profile uses thin
  LTO and 16 codegen units; the Cargo manifest is authoritative.
- The desktop loads the full ROM into `MemMap.rom` at startup; ordinary guest
  ROM reads use that in-memory buffer. The [ROM data TODO](ROM_DATA_TODO.md)
  orders extraction/decoding, startup reuse and direct runtime consumption first,
  followed by mod overrides and broader changes. The explicit
  [rom-data CLI](../rom_data.md) now extracts/verifies typed OBJ dimensions for
  exact EUR/USA images, with a source-authenticated manifest and bounded parsing.
  `scripts/extract_rom_data.py` runs the existing offline extractor via Cargo or
  a selected executable/Wine and lists readable outputs under ignored `target/`.
  `scripts/linux/extract_rom_data_docker.sh` and
  `scripts/windows/extract_rom_data_docker.ps1` instead build/run
  Dockerfile.linux's shared `extract` target, requiring only Docker on either
  host (Docker Desktop Linux containers on Windows); ROMs are mounted read-only
  at run time and outputs remain host files, with host UID/GID on Linux.
  Docker extraction defaults to the EUR filename under root `rom/`; no ROM
  argument is required. Custom relative ROM arguments also resolve from the
  repository root, while absolute overrides remain available.
  The Linux Docker build wrapper saves every run's stdout/stderr under
  target/build-logs/ and waits for Enter on interactive failure, retaining
  the original exit status for noninteractive automation.
  [Inventory/contract](ROM_DATA_INVENTORY.md) documents other families' unknowns.
  The [overall ROM coverage atlas](ROM_COVERAGE.md) follows the latest committed
  EUR extractor generation with metadata-only HTML/JSON and separate candidate
  code/data/runtime/ordinary-use/sharing/unknown layers. Docker launchers stage
  a Git HEAD archive plus current public tools into Dockerfile.coverage, excluding
  uncommitted extraction work and private inputs. The archive also includes
  committed scripts/templates required by Rust include_bytes! before Cargo;
  current coverage tools are overlaid only after compilation. Embedded assets
  therefore share the runtime commit, independent of worktree edits.
  The cache key comes from the
  built extractor source. Coverage uses BuildKit's bundled Dockerfile frontend
  without a separate Docker Hub syntax-image fetch; uncached build dependencies
  still need network/DNS. Dataset identity remains
  independent of historical evidence. New/changed
  resources remain pending ordinary-proof review; exact decoded AND source-map
  hashes preserve reviewed proofs. Migrated source-only spans are subtracted
  from that layer. HTML shows generation time, commit, dataset/evidence revisions;
  logs give deltas or unchanged coverage feedback. Every success rewrites outputs;
  PowerShell -OpenReport uses a versioned URL. Host requires Git/Docker and EUR ROM,
  no Python/Rust/old DLL. Runtime source counters do not establish distinct bytes
  read; speculative code/data conflicts remain explicit. Direct Python and the
  old working-tree Docker target remain developer alternatives. DBG07 imports
  explicit bounded DBG06 metadata reports (--audit / -AuditPath) into schema3,
  recomputing physical byte unions against the selected authenticated manifest.
  Per-audit checkboxes, resource/scene/evidence drilldown, actual Typed/Raw read
  layers and cross-case sharing stay separate from positive-family extents and
  speculative code. Truncated/partial reports and collector gaps remain lower
  bounds, never new DS acceptance. Same-ROM --compare / -ComparePath and default
  prior output expose dates/identities plus added/removed intervals and changed
  audit selection. Inputs are read-only Docker mounts outside the image/context.
  Retained e76/DS20 audit validation needs no gameplay or extraction replay. Native Windows/
  PowerShell5.1/Docker Desktop are unverified.
  EUR `v1/e14-d13` adds [static OBJ lists](../rom_data_obj.md):107direct literal
  roots reference66lists/160objects. Source-authenticated JSON fields now feed
  original CPU/DMA/native and host descriptor readers without changing guest
  instructions/clock or save/native ABI; all13files/12resources are verified
  before activation. Focused Wine original/native/managed/lifecycle and one
 397-frame ordinary transition/actual CLI checks pass; e13and prior decoded
  file bytes remain unchanged. Other indirect lists/animation/sprite pixels
  are not claimed migrated, and new Linux/USA verification remains pending.
  EUR e15-d14also adds ten indexed OBJ/animation bank pairs:167lists/95sequences/
 184frames with exact relative indices and duration words. Typed CPU/DMA/native/
  host views preserve original task counters/scheduler work, save/native ABI and
  e14file bytes. Reduced Wine original constructor/body, authentication/native/
  lifecycle checks and one positive160-frame ordinary menu/actual CLI pair pass;
  the three empty direct-root domains are now classified below; indirect/live
  scheduling remains unverified. The retained short battle sample is state/audio-equal
  but uses0new animation bytes, so it is not an ordinary-use acceptance.
  EUR e16-d15extends the static catalog to123references/73lists/
 173objects/1184sourceB, retaining all original66records, e15generations and
  every other decoded file. Seven new lists reuse the typed model/source paths;
  per-source guest/host flags keep new-list coverage distinct. Three focused
  Wine integration checks and one positive240-frame language-screen full-state/
  audio/actual CLI pair pass. Remaining switch/indirect roots stay open.
  EUR e17-d16adds an eleventh OBJ bank with five proved nonrepeat zero-count
  holds: explicit list selectors/duration-source aliases preserve header zero
  and reuse typed next-header/index owners without overlaps. Earlier ten-bank
  records and all other decoded files/e16generations remain unchanged; current
  totals168lists/286objects/100sequences/184frames/290ranges. Focused Wine
  original/native/host/authentication/lifecycle checks and a positive240-frame
  Party full-state/audio/actual CLI pair pass. The three earlier empty direct-root domains are classified below; other
  constructor-fed roots and new Linux/USA checks remain open.
  EUR e18-d17adds five language-selected OBJ/sequence pairs and two
  typed pointer tables, retaining all eleven old bank records/other files/e17.
  The indexed catalog now has16pairs/317lists/105sequences/328frames/456ranges;
  original selector loads, task work and duration256/u8counter wrap are preserved
  through typed guest/native/host views. Reduced Wine original, authentication/
  native/lifecycle checks and one positive400-frame worldmap“Engage!”full-state/
  audio/CLI pair pass. New localized counters distinguish this family. Linked
  compressed OBJ graphics/palettes remain open, along with new Linux/USA
  verification. The three older empty records B4BD20/B4BD34/B4FA10 have a
  bounded direct-root exclusion: twelve calls select0; eighteen widget mode
  producers select0..3 and two updates2/3. Thirty root literals/47instruction
  pins/50owners checked statically; no proved hold producer, so no new model/
  resource/revision. Six possible selector bytes stay raw; durations8/16/4
  alias existing sequence indices. Rawmode7/65539 can select6/7; no guestcap
  or global dead-code claim. Reopen only for a concrete producer/ordinary trace
  with established scheduling semantics; indirect/live mutation remains
  unverified. [Caller domains](../rom_data_obj.md#earlier-empty-sequence-caller-domains).
  EUR e19-d18adds fifteenth file obj-graphics.json/resource
  graphics.obj-custom-banks:39raw custom word banks/six five-entry selector
  tables,45disjoint ranges/62674sourceB/131872decoded B. Exact typed literals
  and pointers serve original CPU/DMA/native reads, retaining all decompression,
  scratch transforms, partial uploads and live RAM tails; PPU reads live VRAM.
  Previous13decoded files/e18generations stay unchanged. Reduced Wine original/
  managed/native/lifecycle and one positive400-frame Engage full-state/audio
  scene/actual CLI create/no-write reuse/old-cache replay pass; an audited DMA-fixture address is corrected by a narrow positive
  typed20Bcheck. Save/native ABI unchanged; new Linux/USA remain pending.
  EUR e20-d19adds sixteenth ui-palettes.json/resource
  graphics.ui-raw-palettes:21raw BG/OBJ UI shadow banks/23direct-derived-branch
  references/2656B/1328full-u16colors. Original176Bcode initializer and two live
  Color-Mode selectors, memcpy/fades/publication and palette RAM stay active.
  Typed CPU/DMA/native sources preserve save/native ABI and all14earlier decoded
  files/e19generations. Reduced Wine original/native/lifecycle and one positive
 240-frame boot full-state/image/audio/actual CLI pair pass; new Linux/USA
  remain pending.
  EUR e21-d20adds seventeenth ui-graphics.json/resource
  graphics.ui-custom-banks:nine raw custom banks(eightBG/oneOBJ)/ten original
  literal decode/upload paths,57038encoded B/84160decoded B. Typed literals
  preserve original r0returned-length/count work, live scratch and544Bpartial
  BG upload; all15earlier decoded files/e20generations and save/native ABI
  remain unchanged. Focused Wine original/native/lifecycle and a positive
 320-frame Clan full-state/image/audio/actual CLI pair pass; first Party→Unit
  locator is coverage-negative and preserved. Other indirect palette/graphics
  sources, unverified indirect/live empty selection and wider TODO/new Linux/USA stay open.
  EUR e22-d21extends UI graphics to16banks/two language selector groups
  and two explicit shared OBJ links,26ranges/71474sourceB/119168decodedB.
  Seven new banks and ten asset-pointer literals add17ranges/14436B; code jump
  tables, live language, English-vs-skip fallback and5056OBJ/2688BG split copies
  remain original. All9oldUIrecords/15other decoded files/e21generations stay
  unchanged, still17files/16resources. Focused Wine source/native/authentication/
  lifecycle checks pass every new bank and both selectors with full state/clock
  equality; save/native ABI unchanged. One positive1200-frame English intro
  full-state/image/audio/actual CLI Wine pair passes with4selectorB/2347new-sourceB/
 1124new-literalB. Other ordinary languages/new Linux/USA/native Windows stay open.
  EUR e23-d22adds [static item records](../rom_data_gameplay.md),
  eighteenth items.json/resource gameplay.item-records:376records/32bytes/12032B.
  Exact typed halfword/raw/signed/tail fields serve original CPU/DMA/native
  property readers. Live02002030bonuses, inventory, signed/clamp logic and
  secondary table remain original; save/native ABI and all16previous decoded
  files/e22generations unchanged. Reduced Wine source14371+240isolated-negative
  calls and authentication/lifecycle/native86calls with altered source/full
  states/clocks pass. One positive320-frame Unit full-state/image/audio/CLI Wine
  pair consumes40typed signed-field bytes; prefix/tail scenes not claimed. Source signed fields are all nonnegative; the synthetic
  cases are isolated and never authenticated/published. Other gameplay catalogs/
  semantics and new Linux/USA/native Windows remain unverified.
  Current EUR e24-d23adds item-links.json/resource gameplay.item-links:
 225records/20bytes/4500Bwith count/opaque/nine-pair storage and parsed item-owner
  dependency.19files/18resources authenticate together; all17old decoded files/
  e23remain unchanged. Typed CPU/DMA/native reads retain live unit query/bit7/
  FFbyte3dpublication and original work/clock/save/native ABI. Reduced Wine
  managed/lifecycle/native62paths with altered raw source match complete states/
  clocks; all bytes/BIOS/SWI/DMA and a2old+2new typed-byte boundary word pass.
  One positive560-frame headgear inventory full-state/image/audio/actual CLI Wine
  pair consumes574new bytes(287count/205selector/82value); the first slots-only
  locator is negative and retained. Source2244+8original checks stay closed. New Linux/USA/native Windows/wider
  gameplay remain pending. [Contract](../rom_data_gameplay.md#managed-and-direct-item-link-consumption).
  EUR e25-d24 adds `linked-metadata16.json` / `gameplay.linked-metadata16`:
  128 records of 16 bytes, 2048 source bytes, 20 files / 19 resources. All 18
  prior decoded files and e24 generations remain unchanged. The standalone
  source oracle passes 2581 original cases plus 19 isolated synthetic edges;
  the subsequent two Wine integration checks pass 42 sampled native paths with
  every new raw byte altered and equal complete states/clocks. Original live
  list/RNG work, typed crossing ownership and save/native ABI remain intact.
  One positive 520-frame Clan Mission Item full-state/image/audio/actual CLI
  pair passes with 6 typed display bytes. Broader field names, alias-zero caller
  preconditions and other ordinary fields remain unproved; new Linux/USA/native
  Windows checks remain pending. See the [contract](../rom_data_gameplay.md#managed-and-direct-metadata16-consumption).
  EUR e26-d25 adds `linked-metadata52.json` / `gameplay.linked-metadata52`:
  116 records / 6032 source bytes, 21 files / 20 resources, retaining all 19
  earlier decoded files and e25 generations. Actual parsed normal item-link
  selectors validate against the new owner; FF retains its separate contract.
  Two focused Wine checks pass 75 sampled native paths with new raw bytes
  altered / equal complete states and clocks, transactional authentication,
  lifecycle, all-byte bus/SWI/DMA and adjacent typed ownership. Original getter/
  alternate/redirect/live-record work and save/native ABI remain. Closed source
  6369+294 calls / bounded-loop proofs are retained. One positive 540-frame
  Change Jobs full-state/image/audio/actual CLI pair passes with 664 new bytes
  (74 header / 335 routing / 255 packed); ordinary tail/opaque is not claimed.
  New Linux/USA/native Windows checks remain pending. See the
  [contract](../rom_data_gameplay.md#managed-and-direct-metadata52-consumption).
  EUR e27-d26 adds `linked-metadata4.json` / `gameplay.linked-metadata4`:
  20 two-pair records / 80 source bytes, 22 files / 21 resources, retaining
  all 20 earlier decoded files and e26 generations. Parsed metadata52 indices
  and new pair selectors validate together. Two focused Wine managed/native/
  lifecycle checks pass 56 prefixes with raw bytes altered / equal complete
  states/clocks and adjacent typed ownership. Original count/contiguous walk/
  live qualification/signed comparison/tally and save/native ABI remain.
  One positive 540-frame Change Jobs full-state/image/audio/actual CLI pair
  reads 32 selector / 6 value bytes. Closed source320+24 proof stays valid;
  new Linux/USA/native Windows checks remain pending. See the
  [contract](../rom_data_gameplay.md#managed-and-direct-metadata4-consumption).
  EUR e28-d27 adds linked-metadata2.json/gameplay.linked-metadata2,23files/
  22resources, all21prior decoded files/e27retained. Metadata2 bounds 44 two-byte key/value records / 88 bytes at
  6e6c44..6e6c9c; last row43 is real. Original forward u8 indexing reads44,
  inverse u8 search scans43 and returns first stored key / miss0. One focused
  Wine348real+13synthetic source oracle and parsed metadata52 owners pass;
  semantic value names/global caller bounds remain unproved. Two focused Wine
  managed/lifecycle/native checks pass21paths with88rawB altered/equal complete
  states-clocks, typed boundaries/SWI/DMA. One1840Pub mission full-state/image/
  audio/actualCLI pair reads4value/0keyB; inversekey use is targeted-only.
  Source348+13proof stays closed, new Linux/USA/native Windows stay pending. [Contract](../rom_data_gameplay.md#two-byte-forward-and-inverse-source-contract).
  EUR e29-d28adds default-constructor-template.json/resource
  gameplay.default-constructor-template,24files/23resources/all22prior decoded
  files/e28retained. `default48::Catalog` owns48B at6e6c9c..6e6ccc. Original constructor
  mode1/2gates copying, IDs20..23select four separate sources, common signed-ID/
  selector/argument/ORflag patches preserve all remaining fields. One focused
  Wine271real+30synthetic prefix oracle passes after original IWRAM helper
  initialization; allocation/live-unit tails, alternate192B/ROMpointer and next
  240Bremain unowned. Two focused Wine managed/native/lifecycle checks pass16paths
  with48rawB altered/fullstates-clocks equal, real IWRAMhelper and8alternate/
  rejected paths0newreads. One397Pub full-state/image/audio/actualCLI pair reads
  96newBacross all structural categories; source271+30closed/not repeated.
  New Linux/USA/native Windows remain pending. [Contract](../rom_data_gameplay.md#default-constructor-template-source-contract).
  EUR e30-d29adds constructor-alternate-templates.json/resource
  gameplay.constructor-alternate-templates,25files/24resources/all23prior decoded
  files/e29retained. `constructor_alternates::Catalog` owns4records/192Bat6eca78..6ecb38
  and their4Bbase pointer70cd58,196Btotal. Typed IDs/offsets/structural fields and
  first5equipment halfwords perrecord validate against parsed376item owners.
  One focused Wine24real+8synthetic original-copy/live-patch prefix oracle passes,
  with allocation/live-unit tails unclaimed. Two focused Wine managed/native/
  lifecycle checks pass14paths/196rawBaltered/equal fullstates-clocks/pointer+
  all4recordsmask15/five zero-new-use paths. One3600Lutia deployment full-state/
  image/audio/actualCLI pair reads45B(pointer4/record20fields41), mask1; other
  records/full-copy behavior targeted-only. NewLinux/USA/nativeWindows pending.
  [Contract](../rom_data_gameplay.md#typed-constructor-alternates-source-proof).
  EUR e31-d30 adds linked-metadata14.json/gameplay.linked-metadata14,
  93 records / 1,302 bytes at 6e6f6c..6e7482, logical IDs2..94 and biased
  address6e6f50. All 26 files / 25 resources and parsed metadata52 byte02 owner
  authenticate together; all 24 earlier decoded files/e30 remain unchanged.
  Byte01 has no proven semantic owner. Source504+13 and two focused Wine
  managed/native/lifecycle checks pass; 37 paths with raw bytes altered match
  full states/clocks, normal IDs0/1 use no new bytes. Original live books,
  palette copies, overrides, clock and save/native ABI remain. One397Pub
  full-state/image/audio/actualCLI pair reads27B/two records/all field groups.
  Global bounds/palette payloads/allocation tails and newLinux/USA/nativeWindows
  remain pending. [Contract](../rom_data_gameplay.md#managed-and-direct-fourteen-byte-special-unit-metadata).
  EUR e32-d31 adds linked-metadata12.json/gameplay.linked-metadata12,27files/
  26resources/all25prior decoded files/e31 retained;108rows/1296B across two
  indexed banks6e7484..6e7994,48two-bit fields per row. Original full32row/
  signed-wrapping slot/u8mode/clock/save/nativeABI remain. No semantic owner
  dependency invented. Source1640real/10raw-wrap/384synthetic plus two focused
  Wine managed/native/lifecycle checks pass49paths/raw1296Baltered/fullstate-
  clock equality/bothbanks positive/three outside0newreads.
  One positive2200-Echo Grass confirmation full-state/image/audio/actualCLI pair passes.
  Global bounds/next global bank/otherfamilies and newLinux/USA/nativeWindows
  remain pending. [Contract](../rom_data_gameplay.md#managed-and-direct-twelve-byte-packed-pair-banks).
  EUR e33-d32 adds global-pair-flags.json/gameplay.global-pair-flags,
  28files/27resources/all26earlier decoded files/e32 retained. Minimum44pairs/
  11Bat6e7994..6e799f, next3Bunowned. Source102+10+91 and two focused Wine
  managed/native/lifecycle checks pass28paths/raw11Baltered/fullstate-clock
  equality, prior/new typed crossing and partial SWI/DMA tails. Original
  ignoredr0/slotwrap/u8mode/live-mask/earlyexit/clock/save/nativeABI remain.
  ActualCLIcreate/no-write-reuse/oldcache/cwd pass;2200EchoGrass is0globalB
  despite44oldmetadata12B. Positive ordinary global use/fullpair and highmode/
  newLinux/USA/nativeWindows remain pending; original descriptorhword4/live
  mask gates guide new scene selection. [Contract](../rom_data_gameplay.md#managed-and-direct-global-packed-pair-flags).
  EUR e34-d33 adds evaluation-parameters.json/gameplay.evaluation-parameters,
  29files/28resources/all27prior decoded files/e33retained.7terms/2bonuses/
  5class thresholds own38B at6e79a2..6e79b4and6e79c6..6e79da;14Bscoregap/
  4Brow0unowned. Source132+52 and two focused Wine managed/native/lifecycle
  checks pass27prefixes/raw38Baltered/fullstate-clock equality/all5thresholds/
  class0no-read/SWI-DMA/rawgap boundaries. Original live producers/unsigned-u8/
  signed32/wrap/bonuses/RNG/clock/save/nativeABI remain. One1840Pubmission
  full-state/image/audio/actualCLI pair reads15scoreB; a later400frame ordinary
  Yes/fee acceptance CLI/Winefullpair reads1threshold/class1. Classes2..5
  remain targeted-only. NewLinux/USA/nativeWindows/wholefunctiontails/
  semanticlabels pending. [Contract](../rom_data_gameplay.md#managed-and-direct-evaluation-parameters).
  A static ten-byte progression/price contract bounds100records/1000B at
  6e79da..6e7dc2 from normal index0..99and three literal/indexed/default paths.
  Eightu8deltas retain original live level/progress/flag work; byte08discount
  uses the original parsed-Items price chain. Byte09unused/12Btailunowned;
  candidate101rows rejected. Strict lossless source model and one Wine original
  oracle pass362real/68synthetic publisher/delta/live-price cases using actual
  parsed Items, exact u8wrap/signed-unsigned minimum and full EWRAM/stack/callee
  guards. Managed/runtime/ordinaryuse and newLinux/USA/nativeWindows pending;
  e34unchanged. [Contract](../rom_data_gameplay.md#next-ten-byte-progression-and-price-records).
  EUR e35-d34 adds progression-rewards.json/gameplay.progression-rewards,
  30files/29resources/all28prior decoded files/e34retained;100records/1000B
  direct typed delta/discount/opaque bytes. Two focused Wine managed/native/
  lifecycle checks and27original/interpreter/native paths with1000rawB altered
  pass fullstate-clock equality; compiled17delta/4discount/0opaque, outside/
  publisher/ungated paths0newreads, allbytes/boundaries/mirrors/BIOS/SWI/DMA
  and old-evaluation/new-progression crossing pass. Source362+68closed;
  ActualCLI prepare/reuse/no-write/oldcache and one600Shop fullstate/image/
  audio pair pass7discountB/record4; later e36dispatch below covers8deltaB/row0.
  NewLinux/USA/nativeWindows/whole progression caller remain pending.
  [Contract](../rom_data_gameplay.md#managed-and-direct-progression-rewards).
  Next static progression-gates source owns64x12/768B6e7dce..6e80cefrom
  original0487c8stride/loop, with independent next root exactly at its end.
  Byte00unused, halfword01returned/byte03output/eight live unsigned minima
  preserve physical-index u64received-bit ordering; row63returns0but still
  marks bit63. Strict source model/one Wine oracle pass80real+25synthetic
  full0487c8cases, all64rows/eightliveunsignedminima/bit31-32-63/zero-success/
  fullRAM/SP/callee guards;704used bytes and768Broundtrip. Whole downstream
  receipt/FK/managed/runtime/ordinaryuse pending, e35unchanged. [Contract](../rom_data_gameplay.md#next-progression-gates-and-returned-receipts).
  EUR e36-d35adds progression-gates.json/gameplay.progression-gates,
  31files/30resources/all29prior decodedfiles/e35retained;768typedB/two pages/
  requirements-return-output-unused/64physicalrecordmask. Two reduced Wine
  managed/native/lifecycle checks pass19original/interpreter/native paths
  with all768rawB altered/fullstate-clock equality, zero-valued success and
  fullmask0read preserved;bus/BIOS/SWIpagecross/DMA/gap/coexistence pass.
  Source80+25closed. ProductionCLIcreate/reuse-no-write/oldcache passes; a
  new normal10dayMadAlchemist dispatch yields209requirements/8deltaB, one
  1000Winefullstate-image-audio pair passes. Return/outputordinary remain
  targeted-only; the separate400frame Yes/fee prefix now closes thresholdclass1
  ordinary use. Downstreamreceipt-FK/newLinux/USA/nativeWindows pending. [Contract](../rom_data_gameplay.md#managed-and-direct-progression-gates).
  Next progression selection contract owns1024B6e80de..6e84de (64x16,
  physicalindices1..64), row0/16B and next2B unowned. Hard unsigned minima,
  ANY of8active live thresholds, original flag/reset/count helpers and physical
  order preserve conditional selector/zero-terminal/no-store-exhaustion behavior.
  Source-only strict model and one Wine fullD2390/reset-count-flag oracle pass
  138real+80synthetic cases,831original fieldB/1024Broundtrip/SP-callee-fullRAM;
  row52zero-terminal versus exhausted no-store preserved. Managed/runtime/
  ordinaryuse subsequently verified below. [Contract](../rom_data_gameplay.md#progression-selection-gates-source-contract).
  EUR e37-d36adds progression-selection-gates.json/gameplay.progression-selection-gates,
  32files/31resources/all30prior decodedfiles/e36retained/USAe8-d7. Two reduced
  Wine managed/native/lifecycle checks pass18fullD2390paths/raw1024Baltered/
  fullstate-clock equality, all6field counters/64recordmask/16Brow0+2Bgapraw/
  typedbus-BIOS/SWI/DMA/oldgate-newselectioncoexistence. Original reset/count/
  ANY/flag/terminalstore/clock/save-nativeABI remain. Source138+80closed;
  ActualCLIcreate/reuse-no-write/oldcache passes; first400Yes-fee locator0newB
  has no fullpair. Existing1000normal dispatchcompletion CLI/fullpair passes
  277newB/all6fields/first26records, every complete state-clock-image-audio equal.
  NewLinux-USA-nativeWindows/all64ordinary/semanticdisplay pending.
  [Contract](../rom_data_gameplay.md#managed-and-direct-progression-selection-gates).
  Aggregate-flag-keys source contract owns300u16slots+ffff/602B6e9a5c..6e9cb6,
  independent next root at exactend. Strict source model and one Wine full
  D22FCglobal/13FB1Ccontext oracle pass32real+26synthetic cases/all602B,
  key+2ffu16wrap/duplicates/earlysentinel/contextisolation/flag36ORlinked017f/
  read-only fullRAM-SP-callee guards. Managed/runtime/ordinary proofs follow
  below; newLinux-USA-nativeWindows pending. [Contract](../rom_data_gameplay.md#aggregate-flag-keys-source-contract).
  EUR e38-d37adds aggregate-flag-keys.json/gameplay.aggregate-flag-keys,
  33files/32resources/all31prior decodedfiles/e37retained/USAe8-d7. Two reduced
  Wine managed/native/lifecycle checks pass16fullglobal-contextcounter paths/
  raw602Baltered/fullstate-clock equality/all602B-300entrymask/terminal-SWI-DMA/
  raw-boundary-BIOS guards. Original live predicates/context/earlystop/wrap/
  count/clock/save-nativeABI remain. Source32+26closed. ActualCLIcreate/reuse-no-write/
  oldcache and one1000Wine dispatch fullstate-image-audio pair pass1204sourceB
  (1200key/4terminal)/all300slotmask; includesrereads, no bothentrypointsinference.
  NewLinux-USA-nativeWindows/wholeotherUIreaders pending. [Contract](../rom_data_gameplay.md#managed-and-direct-aggregate-flag-keys).
  Source-only trade-parameters model owns five96B records/physical IDs2..6,
  480B at6e85bc..6e879c, from explicit selector/biased price/independent next roots.
  64opaqueB/u16markup/30discountB preserve count0alias of markup highbyte41.
  Original separate70B-source D1F30property36, zero skip, live flags/count30RAMrows,
  unsigned divide/wrap arithmetic remain guest code. One Wine source oracle
  passes499real+160edited-source+15separate injected arithmetic operands,
  all160price-fieldB/320opaqueBroundtrip-only/fullRAM-SP-callee/source guards.
  EUR e38-d37 unchanged; managed/runtime/ordinary/newLinux-USA-nativeWindows
  pending. Adjacent6e9cb6halfword lookup remains partial: producer index bounds
  unproved, no106-entry model inferred. [Contract](../rom_data_gameplay.md#trade-parameters-source-contract).
  EUR e39-d38publishes trade-parameters.json/gameplay.trade-parameters,
  34files/33resources/all32prior decodedfiles/e38retained/USAe8-d7. Two reduced
  Wine checks pass28original/interpreter/native paths/raw480Baltered/fullstate-
  clock equality, all480B/mask31/field-boundaries/priorowner2-raw222-new2/SWI-DMA/
  lifecycle/source guards. Original live/getter/price/count/clock/ABI preserved;
  opaque price reads unclaimed. ActualCLIcreate/reuse-no-write/oldcache and one
  400Wine feeacceptance fullstate-image-audio pair pass14sourceB(markup-low7/
  shared417),mask1physicalID2, includesrereads. Other ordinary fields/records/
  newLinux-USA-nativeWindows pending.
  [Contract](../rom_data_gameplay.md#managed-and-direct-trade-parameters).
  Next packed item-effect parameter source is statically bounded:14×7B/98B
  6e879c..6e87fe, original itemIDs362..375/subtract362 and independent next root.
  Nine5-bit coefficients/8-bit cap/3opaque bits; CFE28low8property/index and
  CFDA8fullu16cap index differ. Strict source model/one Wine original oracle pass
  540real+4explicit raw-cap aliases+166synthetic cases/all98B/fullEWRAM-SP-callee/
  itemowner/source guards; no14371itemoracle repeat. Managed/runtime/ordinary/
  newLinux-USA-nativeWindows pending, e39unchanged.
  [Contract](../rom_data_gameplay.md#item-effect-parameter-source-model-and-original-oracle).
  EUR e40-d39publishes item-effect-parameters.json/gameplay.item-effect-parameters,
  35files/34resources/all33prior/e39retained/USAe8-d7; explicit parsed item-owner
  dependency. Two Wine checks pass50original-interpreter-native paths/raw98B
  altered/fullstate-clock equality/all98B-mask16383/edges-BIOS/SWI-DMA/two-owner
  trade crossing/publication-retention-lifecycle. Original getter/indexwidths/
  raw-cap aliases/live current/quantity/clamp/clock/ABI preserved. ActualCLIcreate/
  reuse-no-write/oldcache and one400Wine EUR GoblinPlay fullstate-image-audio
  pair pass3sourceB(coefficient1/shared051/shared061),mask1record0. Normal guest
  input from retained EUR IN13reaches actualCyrilBank/GoblinLv4/Play; replay is
  mixed native/interpreted. Other14items/20monsters/cap>99UI/newLinux-USA-native
  Windows pending.
  [Contract](../rom_data_gameplay.md#managed-and-direct-item-effect-parameters).
  `rom-data prepare` and opt-in `play --rom-data` share installed/portable
  preparation: versioned immutable generations, source-verified reuse, OS locks,
  atomic relative selection and corruption recovery before save writes. Explicit
  `FFTA_RESOURCES_DIR` overrides the root. No old generation is recursively
  deleted; Linux and Windows/Wine process-interruption/relocation/startup checks
  pass. [Direct OBJ metadata consumption](../rom_data_runtime.md) is now opt-in
  in play/frames/runc: fixed source projections serve CPU/DMA/native reads,
  typed host readers preserve changed RAM and nonserialized Arc resources
  survive Load/Undo/backup clean boot. Extractor2 adds EUR's copied alias;
  the verified EUR host draw source is ROM14ced8, not IWRAM. Base data remain
  source-equal, preserving save/native identities. New EUR worldmap/battle/menu/
  boot-transition full-state/audio checks pass independently on Linux and Wine;
  the copied alias is actually read at frame5400. Local frame/startup/memory/disk
  costs pass the predefined [validation protocol](ROM_DATA_VALIDATION.md), with
  neutral frame results and retained Wine startup variability. Actual Linux/
  Wine native/interpreter Fast/pause/Load/Undo and backup clean-boot GUI checks
  now pass with real muted callbacks and positive decoded use. Actual container
  archives pass395Linux/392Wine tests, privacy/PE/ELF checks and extracted-runtime
  GUI consumption with automatic bundled TinyCC and new portable preparation.
  Both Docker recipes/context include the packager's resource guides. An
  independent USA clean boot reaches the original copied-table initializer:
  all13instructions match full legacy states and24reads consume decoded data,
  including a poisoned-raw-source provenance probe, on Linux and Wine. Native
  Windows remains a gate before normal enablement; the user currently has only
  Linux/Wine. Other families/mods remain open.
  [Graphics groundwork](../rom_data_graphics.md) adds bounded offline LZ77
  decoding, four independently observed source ranges per region and exact
  comparisons to captured guest destinations on Linux/Wine. Original callers
  identify88byte descriptor roots atUSA569104/EUR7662a8. Typed halfword and
  indexed-pair packet decoders now match original regional writers, including
  chunk continuation, on Linux/Wine. Literal pair syntax has a separate
  destination guard after an original-code negative probe. A source-authenticated
  typed model now verifies the contiguous162-record/14256-byte descriptor block,
  exact source/TOML roundtrip and original selectors for every record on Linux/
  Wine. Links, transfer triples, optional variant pointer and opaque tail bytes
  are distinguished. Initial managed v1/e6-d5 extracted/verified/prepared OBJ, BG metadata and base
  palette banks with exact consumed source metadata:64KiB manifest/16KiB OBJ/
  256KiB descriptors/1MiB palettes/1280KiB total input guards, atomic four-file publication/reuse/repair and prior-version retention. Activation
  authenticates all files and hashes every ID/file into dataset identity;
  direct BG source reads now use its retained typed metadata/projection through
  fixed CPU/DMA/native bus paths with separate diagnostics/counters. Owned
  memory448shared/OBJ+28672BG+94713palettebytes stays within predeclared1KiB/64KiB limits.
  Linux/Wine204core tests,98/97frontend defaults, regional original/native
  poisoned-ROM provenance and Load/Undo/backup pass. New1200-frame EUR native
  boots match every full state/image/OAM/dispatch/audio with53BG reads each.
  Current BG worldmap/battle/boot-transition full-state/image/audio parity now
  passes Linux/Wine; five native timing pairs and startup/disk gates pass.
  Historical e3 TOML peak RSS exceeds the declared2MiB diagnostic bound,
  mostly preparation/parsing rather than the29KiBowner. Current e4 JSON uses
  direct typed parsing and standard readable formatting; actual regional file/
  native/lifecycle/CLI checks pass and old e3data remain intact. The user removes
  extracted-data space minimization as a priority, so no packing/indentation
  tuning or extra budget-only campaign continues. Updated user steering also
  defers performance concerns and explicitly asks to avoid measuring them;
  do not resume old benchmark/profile/startup/RSS queues. Preserve historical
  results, continue functional migration/equivalence and optimize later.
  A later explicit request on 22 October authorizes one current Linux-export EUR
  headless performance sample with ROM-data off/on; this scoped exception does
  not resume the paused implementation goal or broader profiling/RSS/optimization
  queues. See the current [validation protocol](ROM_DATA_VALIDATION.md).
  Updated regional priority is EUR; new USA verification remains pending.
  Retain completed USA evidence without reopening or extending its campaigns.
  Updated platform priority is reduced Wine-only testing; mark new Linux checks
  pending and retain the Linux evidence already completed before this steering.
  Current whole-process JSON RSS is not inferred from the parser prototype. Remaining BG GUI/archive,
  associated payload/program formats, other catalogs and language coverage.

  The next content family has a complete declared base-palette inventory:
  all162descriptor links resolve127unique sources per region (125LZ77/2raw),
  with equivalent source/output hashes. Original regional producers match all
  decoded buffers on Linux/Wine in1296calls/72792instructions each, including
  direct/current IDs and both base environment modes. Fullu16palette words
  retain bit15; alternate environment programs remain unclassified. A bounded
  typed palette model and readable four-file preparation now pass39Linux/37Wine
  affected checks and22actual CLI checks per target, including original producer
  comparison, prior e4retention, full-set identity and transactional failures.
  Palette04consumption now uses decoded banks in the existing LZ77HLE and
  exact file-derived CPU/DMA/native/guest-BIOS source views. Schema1/extractor6/
  decoder5 retains e5; all forms stay source-equal. Linux/Wine210core and
  101/100frontend defaults pass. Four original/native regional producer cases
  match1296calls/72792instructions with all source spans poisoned; full state
  and Load/Undo/backup/file-deletion behavior pass. New EURnative1200-frame
  pairs match every state/image/OAM/audio and consume one160Bpalette. Actual
  current CLI verifies retention/repair/reuse and positive interpreter/native
  use. New e6EURworldmap240/battle3530/transition8476frame checks now match
  every full state/image/OAM/dispatch/audio on Linux/Wine; stable segments use
  no new palette, transition consumes one160Bpalette. Further GUI/costs, actual
  BIOS execution, native Windows,
  remaining GUI/alternate palette migration and other families stay separate
  gates. Alternate palettes now have two independently located157-key catalogs
  per region,314custom LZ/RLEstreams producing256Beach. Source/output/link-table
  hashes agree EUR/USA; four original producer oracles match1256calls each.
  Five descriptor FFkeys are unavailable and caller preconditions remain
  unknown. Bounded typed alternate decoding now matches all retained sources/
  outputs and1120synthetic original calls per regional/target case, including
  every token/03..0Fcontrol, alignments and source cursor. A malformed original
  overrun is rejected offline. Fullu16banks, relative links, keyFFand exact
  source metadata are retained. Managed alternate extraction/preparation now adds a fifth fixed JSON file
  under e7-d6, with all316provenance ranges, complete dataset identity and
  transactional activation authentication. e6is retained; standard readable
  JSON is used without space tuning. Actual regional CLI, repair/reuse/move,
  Load/Undo/backup/restart and failed activation checks pass on Linux/Wine.
  Alternate guest consumption now retains immutable banks/links and resolves
  custom-codec literals from decoded words, controls from exact metadata. Shared
  codec maps preserve every original instruction/write/cycle through existing
  CPU/DMA/native bus APIs. Four regional/target poisoned-source producer cases
  match1256calls/final full states each, with positive decoded/native bank reads;
  Load/Undo/backup/restart retain the owner after file deletion. Native216core/
 104runtime and Wine216/103pass; separate first-frame CLI/cache checks are
  activation-only with0alternate reads. Original EUR Options now identifies LCD A=0, LCD B=1(second alternate catalog),
  TV=2(first), committed only through Save changes Yes. Four direct callers per
  region and the setter are inventoried; local descriptor-FF guards were not
  found. Worldmap/Party mode changes report0alternate reads, while the new
  Cyril-pub BG62transition proves positive link/literal use for LCD B/TV. Four
  native397-frame Linux/Wine pairs match every full state/image/OAM/dispatch/
  audio sample; actual GUIs present both modes and close0with positive use.
  Field04/08/10packet catalogs now have352disjoint sources per region,
  raw/LZ77wrappers, at most2-node descriptor inheritance and exact dictionary
  read extents. All source/body/alias/parent hashes agree EUR/USA. Four original
  component loader cases on Linux/Wine match every162×3complete target buffer,
  guards and SP. Dynamic paired fill and ordered patches remain explicit;
  Typed packet files now retain352programs/486links, ordered writes and exact
  source/body metadata with strict graph/source validation. Eight core packet
  tests, JSON guards and full retained EUR body checks pass on Linux/Wine;
  standard readable JSON is identical. New USA validation is pending by user
  instruction. Managed e8-d7now publishes the sixth packet JSON, authenticates
  all352source ranges/file identity and retains e7. Four focused Wine cases plus
  seven actual Windows CLI operations pass create/verify/reuse/repair/move,
  transactional activation and old-cache dispatch. Canonical/protected inputs
  unchanged, builds warning-free; Linux/USA new checks pending. Runtime packet
  reads now consume the immutable typed e8model for EUR: original control bytes,
  typed raw/LZliterals and decoded HLEbodies, with merged palette/packet edges
  and the verified895page. USA stays authenticated/verified-only. Three reduced
  Wine checks pass, including nine original/native calls with all352old source
  spans poisoned, full-state/clock parity and compiled typed use, plus
  Load/Undo/backup/restart with unavailable temporary file copies. One397frame
  EUR LCD B pub transition matches every state/image/dispatch/audio stream and
  consumes2640typed packet bytes. Dataset/native/save/cache ABI unchanged; no
  performance/space work. New Linux/USA/GUI/archive checks pending. EUR field00
  now has a bounded typed offline catalog:162links/50disjoint sources,
 23tag20/27tag22,621923encoded/835232decoded bytes. The staged producer0801b688
  uses the alternate palettes' custom LZ/RLE080054a0; shared parsing preserves
  independent palette guards. All50full outputs/cursors/guards/SP and two
  representative phase2/3DMA layouts match the original on Wine (54calls).
  Nine focused Wine checks pass including JSON/source-authentication guards.
  Managed EUR extraction/preparation now adds the seventh fixed graphics JSON
  under1/e9-d8, with50source ranges, full-set authentication/identity, atomic
  publication/reuse/repair/move and e8retention. Four reduced Wine checks and
  seven actual CLI operations pass; prior decoded files remain byte-identical.
  At that managed boundary first-frame old-cache activation used0resource bytes.
  EUR graphics consumption now retains typed words, exact headers/controls and
  custom-codec literal maps under fixed769..895bus pages. Three reduced Wine
  tests pass, including four original/native calls with all50raw sources
  poisoned and full state/clock parity (29023compiled typedB), and retained owner
  across Load/Undo/backup/restart with temporary copied files unavailable.
  One397frame Cyril-pub EUR transition matches every full state/image/OAM/audio
  and consumes10334decoded graphics literalB. e9-d8/save/native ABI unchanged;
  USA's six-file e8-d7contract is preserved, new Linux/USA checks remain pending.
  EUR transfer/variant inventory now has97active triple links across five slots,
 42typed controls/34shared raw tile banks (968/199296B) and all162variant pointers
  absent in this declared block. Bounded offline source-authenticated models
  retain frame durations/tile offsets/aliases/words. Five reduced Wine checks
  pass, including four original tick/animation/split-DMA cases with full VRAM
  equality after each chunk (634calls/80766instructions); no new recording.
  Managed transfer publication now adds an eighth fixed JSON under1/e10-d9with
  all76source ranges and complete-set authentication/identity. Four reduced
  Wine checks and three actual CLI prepare/reuse/old-cache activation commands
  pass, preserving e9and six prior decoded files. Transfer runtime now retains
  typed controls/tiles through fixed778..897CPU/DMA/native source views, with
  separate counters and original mutable countdowns/flags/copy progress/clock.
  Three reduced Wine checks pass: source widths/edges/last page/SWI/DMA/load,
  two poisoned-source original/native forward/reverse cases (60calls/66compiled
  controlB/9984native-DMA tileB) and unavailable-copy Load/Undo/backup/restart.
  One397frame EUR pub transition matches every state/image/OAM/dispatch/audio,
  uses1203controlB/21504tileB; one actual CLI with no-write e10reuse/old cache
  matches the valid retained legacy row. e10-d9/save/native ABI unchanged.
  USA retains six-file e8-d7;
  new Linux/USA checks pending. Broader regional/gameplay/catalog/language coverage,
  global FFpreconditions, archives and nativeWindows stay open.
  Exporting files alone is not completion; preserve guest behavior,
  native-cache/profile/save identity; cost measurements are deferred by the user.
  EUR text/font inventory now independently locates original08016648's five
 107-entry short-string tables. Bounded offline typed model retains535sources
  (478compact/57extended), exact controls and65referenced packed4bpp glyphs/
  widths; full provenance670ranges. Two reduced Wine checks match every original
  pointer/expanded string/guard, all65glyph slots and variable/fixed metrics
  (1331calls/89446instructions). See [text/font contract](../rom_data_text.md).
  Managed EUR1/e11-d10now adds ninth short-texts.json/eight resources, all670
  provenance ranges and whole-set identity; retains e10/seven decoded files and
  USAe8-d7. Three reduced Wine checks cover publication/reuse/atomic failure/
  repair/move/source/guard/transactional activation; actual CLI prepare/no-write
  reuse/one-frame old-cache activation passes at that managed-only boundary.
  Text runtime now retains typed indices/codes/glyph words/metrics through fixed
  EUR source pages, separate counters and unchanged original work/clock. Three
  reduced Wine checks pass: synthetic widths/mirrors/edges/SWI/DMA/load, five
  original/native reader chains with all670raw sources altered (20calls/full
  states/clock equal;20table/124string/636glyph/6metric compiled API bytes), and
  unavailable-copy Load/Undo/backup/restart ownership. Fixed mode skips metrics
  exactly as original. No schema/save/native ABI change. Manifest guard is1MiB.
  Other catalogs,
  full font/control/Unicode contracts and new Linux/USA verification remain open.
  The broad EUR offline text model now covers all nine pointer selectors
  (16100strings/45tables) and29relative selectors (145archives/17035programs)
  in the declared reader block; all33581source ranges disjoint,128glyphs.
  Headers, compact/extended commands/operands, exact custom-LZ representations
  and decoded trailing bytes remain explicit; dynamic context stays guest RAM.
  Two reduced Wine checks match independent full static inventories plus
 80new pointer boundaries/145roots/300archive outputs/63new font slots
  (968calls/867138instructions). Oracle explicitly supplies one-based active
  text context1via original nullable fallback, not ordinary gameplay coverage.
  Managed EUR1/e12-d11now adds tenth text-catalogs.json/nine resources with all
 33581source ranges and whole-set authentication/identity; e11/eight earlier
  decoded files retained, USAe8unchanged. Three reduced Wine checks cover
  deterministic publication/reuse, atomic-write failure/repair and actual EUR
  guards/identity/corruption/move/transactional activation. Actual CLI prepare/
  no-write reuse/old-native-cache one-frame activation passes at managed-only
  boundary. Broad runtime now serves typed pointers/strings/relative indices/
  headers/program units/font through fixed EUR pages; compressed literals come
  from decoded units/padding. Three reduced Wine checks pass: synthetic source/
  encoded-literal alteration/edges/SWI/DMA/load, fifteen original/native chains
  with all33581raw sources altered (90calls/full states/clock equal, positive
  compiled use in all8kinds), unavailable-copy Load/Undo/backup/restart.
  Explicit nullable context1fixture, no ordinary scene claim. Earlier overlapping
  subset stays authenticated/verified-only, no duplicate read owner. Manifest
  guard16MiB/catalog+dataset512MiBaccept readable data, not packing targets.
  New Linux/USA pending, no performance/space campaign.
  Full text catalog now also passes one retained EUR/Wine397-frame LCD B/pub
  transition with every state/clock/image/OAM/dispatch/audio equal and positive
 52pointer/185string/2index/4header/27decoded-program/8064glyph/93metric bytes.
  This is a raw-program scene; compressed use has separate native04proof.
  Warning-free production candidate/canonical restored; one new decoded actual
  CLI with no-write e12reuse/old cache matches the valid retained legacy row
  without repeating legacy. Broader scenes/GUI/archives/full font/readers and
  new Linux/USA/nativeWindows remain open. No new recording or performance work.

  EUR now prepares and directly consumes the complete3175-slot packed font/
  metrics and three name-keyboard programs/indices/navigation widths under
 1/e13-d12(12files/11resources, seven additional provenance ranges). Earlier e12
  files remain retained and unchanged, USA stays e8-d7. The complete font owner
  replaces overlapping subset reads; original keyboard page construction,
  rendering work and guest clock/save/native ABI remain unchanged. Reduced Wine
  source/original/managed/native altered-source/full-state/lifecycle checks and
  one397-frame ordinary pub transition/actual CLI pass. Keyboard ordinary scene,
  other catalogs/control semantics/Unicode, GUI/archives and new Linux/USA/
  nativeWindows verification remain pending. The latest request resumes EUR
  functional TODO work with reduced Wine tests and again excludes all new
  performance/space campaigns; the earlier Linux sample is closed history.

## Container builds

Native desktop packaging uses `scripts/build_desktop.py` through the Linux
wrapper or Windows `build-all.ps1 -SkipLinux -SkipAndroid`: Cargo JSON artifact
paths select the exact runtime/TinyCC directory, including custom target dirs.
Only Windows/Linux x86_64 executables are accepted. The all-platform Windows
script keeps its Linux/Android routes unless explicitly skipped. The Linux CI
uploads the actual `dist/FFTARecomp-linux-portable.tar.gz` and refuses empty
artifact output. Bundles and Docker contexts include Atlas/extended-save docs.

The Linux, Windows and coverage Dockerfiles use BuildKit's bundled frontend,
avoiding a separate Docker Hub `docker/dockerfile:1` fetch before compilation.
Uncached base images and build dependencies still require working network/DNS.
Desktop Docker stages refresh copied crate-file timestamps before using Cargo's
shared target cache, preventing stale reuse across older source snapshots;
coverage refreshes its archived sources. The host checkout is unaffected.

- `Dockerfile.linux` and `scripts/linux/build_linux_docker.sh` build Linux amd64 using
  Rust 1.94.0 / Debian Bookworm with ALSA/udev development dependencies inside
  the container. BuildKit caches Cargo/target separately from the host checkout.
  The scratch export stage writes only the portable tar.gz and BUILD-INFO.txt
  to `dist/docker-linux/`; runtime GUI/system libraries remain host dependencies.
  `.dockerignore` allows desktop build inputs and excludes game/personal data.
  Container export is gated by serial input/config, menu, core, runtime and
  screen suites; optional local-ROM/GPU cases remain separate. The exported
  runtime requires glibc 2.35 and previously passed a native Ubuntu Xvfb
  save/load/relocation check.
  Current optimized-renderer exports also pass an isolated cold-cache Xvfb
  load/Undo/reload/close check. Canonical dist exports include the retained
  renderer changes; validation records identify their checks and limits.
  Older-distro and physical-controller compatibility remain unverified.
  See [Docker workflow](../docker.md). Keep the direct Ubuntu-built archive
  distinct, and establish actual libc requirements for each build environment.

- `Dockerfile.windows` cross-compiles Windows GNU x86_64 with MinGW from the
  pinned Linux Rust base, builds TinyCC with distinct native/target compilers,
  and runs input/config, menu and runtime tests under container Wine.
  `scripts/windows/build_docker.ps1` is the Windows-host entry point: default Windows,
  `-Platform linux` for Linux. Docker Desktop must use Linux containers.
  `scripts/linux/build_windows_docker.sh` offers the Windows target from Bash.
  Export is `dist/docker-windows/`; it contains no game or personal data.
  Resource build scripts branch on Cargo's target OS rather than build-host cfg.
  The Windows builder reuses the pinned Rust image and copies its toolchain
  into Debian Trixie for MinGW 14 / Wine 10. Windows/Wine tests gate exports
  on serial core and screen suites as well as input/config, menu and runtime.
  Earlier packaged PE resource/import checks and a simulated-XInput portable
  save/load/move smoke also passed. PowerShell 7.4.6
  Linux mock-CLI validation passed; native Windows/Desktop/PowerShell 5.1 remain
  unverified. The current exported Windows runtime also matches the prior
  Windows and validated Linux replay on framebuffer/OAM, final machine state
  and counts under Wine. This is compatibility evidence, not native timing.

## Desktop behavior

- [Fullscreen](../fullscreen.md): Video > Fullscreen (F11) persists default-off
  `video.fullscreen`, sharing startup/menu/shortcut requests and asynchronous
  reported-state reconciliation. Winit/WM restore normal placement/maximization;
  missing-monitor fallback keeps a usable window. Existing resize/cursor/presenter
  paths remain; no guest/audio/aspect/zoom/history change. Menu GPU/software
  presentation uses the completed canvas dimensions across concurrent resize,
  avoiding upload overflow and a fallback stride panic. Native28config/35menu/
 81runtime and Wine28/35/82 pass; physical/native Windows coverage stays open.

- [Audio controls](../audio_controls.md): Audio > Mute persists default-off
  `audio.mute` separately from Volume and temporary Fast Silent/fallback mute.
  Startup/live edits share audio configuration; source consumption continues
  while output gain slews to silence, retaining volume and timing. Timeline
  resets preserve user mute. Audio-only edits do not reset video history.
  Native27config/34menu/79runtime and Wine27/34/80 pass; hardware listening and
  native Windows remain unverified.

- Fast audio transitions invalidate callback DSP history as well as queues:
  host epoch resets DAC/sinc/FIFO/DC/crossfade state under the existing mutex,
  retaining filter tables and ramping new output through the volume slew.
  Pause discards pending output while preserving the Fast latch; device loss
  also discards Normal's backlog. Load/Undo retains stream recreation.
  Fast consumption now follows produced PCM pairs against device sample-clock
  progress with a common smoothed clock for faithful/PSG/HLE/FIFO, a60ms reserve
  after each callback and fractional sinc cutoff tables. Normal1x retains its
  DSP; explicit waiting/rebuffer telemetry prevents silence hiding starvation.
  Native74/Wine75runtime tests and72clock cases pass. Actual local4x CPU-limited
  battle gates pass357/359Faithful/Enhanced callbacks with no source holds,
  PSG/HLE empties, waiting/rebuffers or drops; consumption tracks~2.2x production
  and max estimated source queue stays~108ms. Under changing compilation load,
  recovery can still require silence; this is reported, not continuous playback
  proof during arbitrary stalls. Backend-buffered audio cannot be retracted;
  native Windows/hardware/event latency and audible quality remain open.
  Normal delayed1x now uses a preallocated512KiB/2second queue, replacing old
  audio with the latest250ms and a10ms stereo fade on overflow; no temporary
  file or callback I/O. Its menu states the delay, legacy config remains valid,
  and Enhanced configuration still uses faithful PCM for Normal. Native78/
  Wine79runtime and33menu checks pass; real Linux4x Normal callbacks stay
  below131072pairs through10resyncs per mode with no holds/over-period calls.
  The queue budget excludes backend buffering. See [Fast audio](../fast_forward_audio.md).

- The observed battle laws table and its HELP window use the existing centered
  Notice layer, including the selector/continuation sprites, independently of
  the action-menu split and any remaining HUD. Recognition checks the frame,
  rows and divider inside the existing ROM/terrain/action gate, not translated
  words. The host subtype resets per frame and is skipped in snapshots. Native
  and Windows/Wine suites, 72 private EUR reconstruction cases, 24 synthetic
  GPU/CPU modal cases and inspected frontend checks pass. Other layouts/USA,
  native Windows and exact original tutorial coverage remain open. See
  [laws layout](../ffta_laws_layout.md).

- Native UI capture skips per-pixel sorting/blending when no admitted UI BG or
  portrait can reach the texture, retaining a default destination write to clear
  stale pixels. Real UI keeps the full blend/world-target path. No persistent
  cache or state/translation ABI change. Native/Wine174 core checks and full
  replay/zoom6 equivalence pass; five pairs reduce headless work4.20% and complete
  native unpaced GUI work2.67% relative to panel-row classification. Worldmap and
  default GUI load/Undo/reload/close pass. Current local Docker exports include
  that stage:336Windows/Wine and337Linux tests passed (15optional ignored each),
  archive/privacy/DLL/GLIBC2.35 and actual cold-cache Linux portable GUI verified.
  Previous archives are preserved. This is not original rare-stall or native
  Windows performance evidence.

- Release desktop audio/presentation resources in `PlayApp::exiting`, before
  `EventLoop::run_app` consumes/drops the display connection. Release surfaces
  and their contexts before the window, including error exits. Holding them
  until PlayApp's later field destruction caused a reproducible Wayland/Mesa
  EGL SIGSEGV even when Vulkan rendered the game. Native Ubuntu 26.04 GNOME /
  AMD RADV close-event regression checks pass; see [portable notes](../portable.md).

- [Desktop portable editions](../portable.md): `portable.txt` beside the actual
  executable opts into shared `input_config::storage::portable_root()` resolution.
  Settings/labels use `Data/config`, native translations `Data/cache`, cartridge
  progress `Data/saves/ffta.sav`; backups and slots remain beside that save.
  No AppData ROM/settings/save-pointer fallback in this mode. The wrapper copies
  a first-launch selected ROM to `ffta.gba`. Explicit environment path overrides
  still apply. Move the entire folder while closed; paths are resolved afresh.
  `scripts/windows/package_ffta_windows.py` and Windows `build-all.ps1` create a portable
  ZIP by explicit file selection. Native Linux uses
  `scripts/linux/package_ffta_linux.sh` and the shared
  `scripts/package_ffta_portable.py --platform linux` for a tar.gz with executable
  modes preserved and `tcc/libtcc.so`. The current archive targets the Ubuntu
  26.04 x86_64 host; broad older-distro/Steam Deck compatibility is unverified.
  Existing installed behavior needs no marker;
  migration is an explicit copy, never a silent move of personal data.
- Developer `play --benchmark <config.toml>` restores a bounded snapshot/input
  replay with a scene label and warm-up exclusion. It reuses guest/UI/temporal/
  GPU presentation functions while disabling physical input, FPS HUD, audio
  callbacks and pacing, and skips cartridge discovery/backup/autosave/exit
  writes. Verify images/OAM and full state separately from timing. This is an
  unpaced GUI work benchmark; realtime input/audio/compositor performance and
  older-binary default-path overhead need distinct evidence. See the
  [measurement workflow](PERFORMANCE_WORKFLOW.md).
- With `FFTA_PROFILE=1`, normal-play audio callbacks have bounded wall-time
  observation under the existing mutex and phase-separated faithful holds/
  producer drops. No new callback lock, logs or allocation; clock reads stay
  off by default. Capture survives legacy counter resets and reports after
  stream teardown. The unpaced GUI benchmark does not run the callback.
  Optional `FFTA_AUDIO_PROFILE_FRAMES=START:END` now selects one traversal of
  producer-published guest frames. Load/Undo inside capture interrupts it;
  default-off and callback-free benchmarks ignore the window. Native/Wine
  boundary/restore/audio parity and actual archive GUI checks pass. Idle battle
  window measurements show sub-microsecond mutex p99, with no faithful holds or
  drops; lifecycle transitions can still produce holds. Enhanced source demand
  telemetry is now available as raw PSG/HLE/FIFO requests/empty reads plus
  configured-target totals under the same gate. Warm/unrouted consumers are
  separate, with no per-request counters in the default const specialization.
  Output/queue/interpolator parity tests pass, including Windows/Wine. Idle
  battle captures show no empty source reads; lifecycle transitions can still
  produce holds/empty reads. Current Docker exports pass341Wine/342Linux tests
  (15optional ignored each), archive compatibility/privacy and actual Linux
  cold-cache lifecycle/default-off/benchmark checks. Configured target is not
  audibility or live crossfade weight; CPAL0.16.0 ALSA can recover XRUNs without
  app errors. Independent local PipeWire observation now matches PID/client/
  stream/link/sink and verifies stable serials/sample cadence: no ERR increments
  in measured idle/pause/resume intervals, but sink baseline already1 and graph
  errors are not dedicated hardware XRUNs. No audio-policy change is justified
  by these idle-scene results; Windows/hardware/slow-scene attribution remains
  open. See the workflow.
- Successful host presentation intervals are available in normal play under
  `FFTA_PROFILE=1`, optionally scoped by `FFTA_PRESENT_PROFILE_FRAMES=START:END`.
  They reuse the FPS timestamp; no new clock/allocation off or benchmark capture.
  Lifecycle/configuration/backend boundaries create anchors; surface skips stay
  in success-to-success gaps. The pure range helper is shared with audio without
  changing its behavior. Native/Wine tests, local clock/audio/requested-VSync
  matrix, silent fallback and actual portable lifecycle/off/benchmark checks pass.
  Clock is already Linux/Windows default; local means~16.74ms versus audio batch
  cadence~42.67ms. This is host API cadence, not scanout or native Windows/physical
  high-Hz proof. Current Docker exports pass344Wine/345Linux tests (15optional
  ignored each), archive compatibility/privacy and cold-cache Linux GUI gates.
- GPU render-pass profiling is opt-in via `FFTA_PROFILE=1` plus
  `FFTA_GPU_TIMESTAMPS=1`. Request only supported timestamp queries, retry device
  creation without optional features if needed, and use three reusable readbacks
  with nonblocking polling and a 18000-submission cap. Results are asynchronous
  gameplay-pass durations, separate from CPU queue/acquire/present and scanout.
  Paused frames do not submit queries; shutdown reports pending results without
  waiting. Mapping/poll failures stop diagnostics, not normal rendering. See
  [measurement workflow](PERFORMANCE_WORKFLOW.md). Default rendering requests
  no timestamp features or resources.
- Surface acquisition timeouts skip a host presentation while retaining the GPU.
  Lost/Outdated reconfigure and retry once per call; three failed recovery calls
  trigger software fallback. Successful acquisition resets the budget; memory
  exhaustion/unknown errors fall back immediately. Zero-sized surfaces skip
  acquisition, and skipped frames do not increment FPS EXT. Event-loop pacing
  remains responsible for retries; no tight retry loop or per-frame error log.
  Pending Queue writes are tracked across UI updates and flushed once on skips
  with transfer-only submissions. This prevents unsubmitted staging growth while
  retaining revision caches; these submits do not count as EXT or GPU pass samples.
  Zero-sized frontend windows avoid GPU/UI preparation and software surfaces;
  benchmarks still fail on skipped presentations. A no-ROM lifecycle diagnostic
  lives in `crates/recomp/examples/presenter_skip_uploads.rs`. Its actual
  llvmpipe run completed 1200 batches/492 MB logical transfers with stable RSS
  between batches 600 and 1200, then resumed normal rendering. Current exports
  pass 333 Windows/Wine and 334 Linux tests (14 optional ignored each), archive
  compatibility/privacy checks and cold-cache Linux portable GUI load/Undo/
  reload/close. Observed X11 minimize/restore is covered separately below;
  Wayland/native Windows, monitor/device-loss cases remain open.
- `crates/recomp/examples/presenter_window_lifecycle.rs` exercises real WM
  minimize/restore, resize and VSync without ROM data. It uses the event loop,
  bounded phase deadlines and observed minimized state, then releases Presenter
  before window. Native X11 Openbox/llvmpipe passes three cycles, as does the
  unchanged Linux portable in battle. The example also passes under Wine10/
  Vulkan after disabling only Wine's failing D3D12 DLLs in an isolated prefix.
  Native X11 minimized sizes stayed nonzero (27 Presented); Wine exposed zero
  sizes (18 Presented/9 Skipped). Do not equate minimized state with zero size
  or assume every hidden window skips acquire. EXT still counts successful host
  presentations, not visible scanout. Pinned winit cannot observe/restore
  minimized state on Wayland; this diagnostic fails on unsupported state rather
  than treating it as a pass. Native Windows/Wayland minimize, monitor changes
  and runtime device loss are not covered by these results.
  The separate `--monitors` mode now covers two distinct virtual CRTC outputs
  and observed A/B/A/B/A transitions, monitor-relative resize and VSync checks.
  Xorg/dummy 1280x720/1280x960 with Openbox/llvmpipe Vulkan passes20 presentations
  on native Linux and Windows/Wine; independent positions confirm the moves.
  The unchanged Linux portable passes the same moves in battle plus Undo/reload/
  close, inspected captures and canonical hash checks. Xvfb RandR monitor areas
  alone do not establish this: pinned winit enumerates CRTC outputs. The dummy
  keyboard needs explicit evdev US setup for synthetic menu input. Physical
  screens, GPU/DPI/color transitions, hotplug and Wayland remain separate work.
- Presenter owns its Device and retains the first uncaptured GPU error or
  device-loss callback in Arc/OnceLock. A confirmed destruction regression could
  panic when new UI/framebuffer textures were created; callbacks now return the
  failure to the existing software fallback. Further UI/VSync work is rejected
  on that presenter. Local configuration scopes stay recoverable and do not
  poison this latch; normal checks do no allocation, GPU poll/wait or callback
  logging. The private ignored `device_loss_tests.rs` covers actual destroy with
  plain/cached/new UI/resize and software presentation. Native and Wine/Vulkan
  pass, including scope isolation and stable first-error behavior. It does not
  prove all spontaneous/asynchronous hardware-fault cases.
  Updated Docker exports pass333 Windows/Wine and334 Linux tests (15 optional
  ignored each), with this regression also explicitly run on native/Wine Vulkan.
  Archive source/doc/privacy/DLL/GLIBC checks and actual cold-cache Linux portable
  load/Undo/reload/close pass without unexpected fallback. Normal physical Radeon/
  Wayland replay preserves3530frame hashes, state/counts and clean close.
- Surface configuration is guarded separately from acquisition recovery.
  `configure_surface` in `crates/screen/src/present.rs` captures native wgpu
  validation/internal/out-of-memory errors with balanced device error scopes.
  Startup/resize/VSync/recovery failures return to existing software fallback;
  the VSync setter returns a Result and its menu caller discards a failed GPU
  presenter. Scope inspection uses the pinned native backend's ready futures,
  without a new GPU poll/readback wait. Normal unchanged frames skip it.
- A native UI output-comparison cache was rejected after five GUI A/B pairs.
  It cut logical upload bytes about 74% but added preparation work and showed no
  useful whole-cycle gain (+0.131109% median work time). Source/native binary
  were restored exactly, and existing FPS/LUT caching is retained. Do not
  reactivate that double-buffer comparison design for context recovery. A
  follow-up must eliminate packing work or have distinct measured evidence;
  see the [evaluation workflow](PERFORMANCE_WORKFLOW.md#rejected-native-ui-output-comparison-cache).
- Native UI packing now caches complete source/backing inputs and resolved
  mode/length/anchor parameters using safe exact field words. Identical inputs
  skip pure packing and GPU texture transfer; changes conservatively repack.
  Clear/load/Undo invalidates host snapshots/revisions, and FPS/native producers
  share process-unique tokens. World/background pixels, LUT and layout remain
  dynamic. The five-pair native unpaced replay improves median mean host work
  1.41%; this does not establish an AI-turn or native Windows speedup.
- Native UI worlds share a texture/upload only for the same immutable slice
  extent and dimensions within one presentation. Every unique world uploads
  again on the next call; changed source groups split handles before writes.
  Stats/FPS sources remain independent, layout/LUT/UI state stays per layer,
  and dimension changes rebind the world without discarding valid UI/LUT data.
  Five native unpaced pairs show 40.37% lower scheduled UI payload and 0.39%
  lower median mean work; no AI/native Windows speedup is established.
- The row-endpoint panel-bound scan is a test-only experiment. It preserves
  the full scan's results and reduces synthetic panel cost, but production
  integration was withheld after slower 3000-frame introductory A/B replays.
  Keep the current full scan until representative end-to-end benefit is shown;
  see the optional diagnostic in the performance measurement workflow.
- GPU UI layers retain LUT contents and skip identical uploads; FPS pixels use
  process-unique render revisions. Native UI uses complete packing-input keys;
  unversioned UI and world data still update normally. Resource recreation clears caches, and color comparison handles
  in-place changes. FIFO audio taps drain without discarding producer capacity;
  `FFTA_FRAME_PACING` is resolved at startup. See [FPS notes](../fps_overlay.md)
  and the performance TODO for remaining optimization work.
- [FPS overlay](../fps_overlay.md): Video > FPS counter; off/internal/external/
  both. Inline row help explains game/present counts and expected Fast divergence;
  the user confirms the report occurs only with Fast, consistent with the4:1
  live captures at4x and~59.7/59.7 after disabling it. No pacing change.
  INT measures completed guest frames per wall second; EXT measures host
  presentations, not display scanout. Configuration uses `video.fps` in
  `%APPDATA%\FFTARecomp\av.cfg` in installed Windows mode.
- [Menu shortcuts](../menu_shortcuts.md): configurable single controls/chords
  under Input > Keyboard, Mouse & Keyboard, or Gamepad > Open menu. Defaults
  Escape and Start+Back/Select. Escape remains a recovery shortcut; Guide/Mode
  works where exposed. Persistence uses `key.menu` and `pad.menu` in
  `%APPDATA%\FFTARecomp\input.cfg` in installed mode.
- Save resolution lives in `crates/recomp/src/save_path.rs`. `FFTA_SAVE_PATH`
  overrides the selected location. Windows remembers the shared save location
  under `%LOCALAPPDATA%\FFTARecomp\shared-save-path.txt`. Save files are raw
  cartridge `.sav` data (64 KiB for the tested EUR game), not emulator save
  states. Portability requires matching game edition/region and a tool/device
  that accepts that save format; no new conversion feature was implemented.

- Restored paused displays use the authoritative native 240×160 framebuffer
  centered with cleared margins. The wide scratch buffer has no serialized
  stride and cannot be interpreted with the current frontend zoom. Separate
  UI caches stay hidden until the next completed guest frame rebuilds them;
  preview generation does not advance the machine or change its snapshot format.
- [Extended saves](../extended_saves.md): Debug option defaults off and expands
  snapshot slots from 9 to 99 without changing paths/formats or deleting hidden
  files. Debug > Game saves selects 99 standard cartridge files: original slot 1,
  `-slot-02.sav` through `-slot-99.sav`. Copy protects confirmed replacements;
  switching validates/prepares boot, backs up target, flushes old source and
  updates active path before frontend reset. Empty targets use erased memory.
  Portable selection persists only a number; explicit FFTA_SAVE_PATH wins.
- [Save states](../save_states.md): pause menu > Save states, nine default controller-
  operated slots with embedded previews and in-menu overwrite/error feedback.
  Storage: `SaveStates/<ROM SHA-256>/slot-1.ffstate` through `slot-9.ffstate`,
  relative to the selected cartridge save directory. A new session starts at
  slot 1; Left/Right wraps between the nine or 99 enabled slots. Legacy raw states can be copied
  into an unused slot and loaded without an image until resaved.
  Files use `.ffstate`, carry ROM/BIOS identity and a versioned machine snapshot,
  and are separate from cartridge `.sav` files. Loading also rewinds cartridge
  memory, which normal autosave/exit later persists. Native translations and host
  resources are not stored; input recording must stop before loading.
  Undo last operation is one RAM-only session entry across all slots: undo save
  restores the previous file/preview (or an empty slot), undo load restores the
  pre-load machine. Failed operations preserve the entry; successful undo
  consumes it without redo. A detected externally changed/missing slot prevents
  undo save. Load and undo load both use the shared frontend restore reset.

- [Restart ROM](../restart_rom.md) in the pause menu flushes current cartridge
  memory and transactionally rebuilds the startup machine in the same window.
  It resets frontend timelines and slot Undo while preserving slot files and
  startup backups. Failed writes and active recording keep the paused session.
  Clean boot retains the current immutable decoded-resource owner without file
  I/O, shared with backup recovery.
  Deployment laws now recognize the distinct WIN0 compact table and WIN1 HELP
  from user slot3, keeping both columns/selector intact and HUD separate.

- [Cartridge save backups](../save_backups.md): capture the selected `.sav` at
  desktop startup, retain two raw copies under `SaveBackups/<save-filename>/`,
  and restore via the controller-operated Save backups menu. Sequence/hash
  filenames identify order and integrity. Restore prepares a clean boot from
  the session's startup snapshot before atomic file replacement, resets the
  machine and frontend, and clears save-state undo. It preserves both backup
  copies and the nine state slots. Backup failure aborts before gameplay writes.
  Missing remembered/default primary files keep their recovery location when
  recognized backup copies are present, rather than selecting an unrelated save.

## Validation knowledge and remaining uncertainty

- [Debug save cases](debug%20saves.md) uses stable DS-01 through DS-26 IDs and
  distinguishes observed coverage from candidate snapshots and missing scenes.
  The ignored `saves/debug-cases/` collection follows the same slot numbers;
  its local catalog preserves source hashes and flags partial cases explicitly.
  Expanded imported sources and ordinary derived checkpoints live under ignored
  `debug_saves/ready/`, split by USA/EUR. IN source numbers belong to standard
  save selection; DS case numbers belong to snapshot selection. Its local
  catalog distinguishes wrapper region hints, verified runtime region, payload
  duplicates and untested internal game File No.2 entries. Original containers,
  converted SRAM, screenshots and snapshots remain outside Git/packages.
  Additional user snapshots retain their original bytes under descriptive
  ignored names; the local catalog records relocation, ROM/BIOS and proof scope.
  DS20 now has a verified EUR failed-dispatch return at slot20, with a nearer
  precursor at EUR65.
  Derived standards use GEN IDs:30/31 for map progress,33 for an ordinary
  Save Now suspended battle verified through Resume Battle. Retained masters
  live outside the working family in ignored `debug_saves/generated/`.
  NEW-32 is factory-empty memory for naming checkpoints: choose that standard
  slot before loading DS23, since snapshots also restore/flush cartridge memory.
  Snapshot slots51–64 are explicit variants, separate from stable DS IDs.
  Clan-threshold prizes and the originally slow enemy turn still require their
  own evidence; related mission rewards or a normal turn do not close them.
- [Desktop performance TODO](PERFORMANCE_TODO.md) tracks task completion separately
  from partial progress. Its first P0 tranche adds native Linux/Windows/Wine
  comparisons, optional final serialized-state verification, bounded headless
  frame-time capture and opt-in interactive CPU phase profiling, including
  separate CPU-side presenter setup/upload/acquire/encode/submit/present times. See the
  [measurement workflow](PERFORMANCE_WORKFLOW.md). These tools do not establish
  an AI-turn speedup; CPU submit timers alone do not measure GPU execution.
  Optional render-pass queries now provide a separate GPU series. The frontend fails before advancing
  temporal history on an exhausted frame guard; stats display separate native
  block/fallback-step counts rather than their mixed-unit percentage.
  Detailed PPU attribution and an observed enemy-activation replay are complete;
  reuse them through TASK_HANDOFF. Remaining P0 work includes the originally
  reported slow-turn scene, controlled realtime GUI overhead and broader
  hardware/native Windows coverage. Frame-window gating, raw enhanced source
  counters, local PipeWire ERR observation and presentation cadence are already
  implemented. They do not alone establish audible underruns or every hardware
  XRUN. The bounded
  unpaced replay mode now provides controlled work-overhead comparisons. Preserve completed recovery
  and allocation/upload substeps rather than restarting them. GPU LUT/temporal processing is conditional on
  measured cost; preserve per-guest-frame history and the CPU reference path.
  Guest AI/GPU or full-PPU rewrites are not the first-line approach.
- The observed enemy replay has a dispatch census distinct from introductory
  observer validation. Missing entries/guest cycles do not isolate host time.
  A pre-panel-cache native perf sample placed 31.52% of leaf samples in
  capture_dialogue_part and guided panel-membership caching below. Preserve
  direct lookup/interpreter hooks without specific evidence; do not reopen
  completed window caches or rejected selectors from this historical sample.
  Profiling binaries/logs are isolated; see the performance workflow.
- Extra immutable ROM seeds can preserve every image while changing audio state.
  The recorded18-seed coverage trial is rejected: first full-state difference
  at relative38/guest8514 affects mixed audio and HLE verification, with equal
  CPU/clock; the diagnostic reproduces both original final hashes. A ROM-free
  timer fixture also exposes mixed-output dependence on tick batch size. Event
  chronology now has a core correction with single-cycle/DMA/skip regressions:
  scheduled events are serviced before finishing a tick batch, and DMA steals
  extend CPU work without video reentry. Native/Wine179core and66native runtime
  checks pass;3530old frame/OAM/counts and native load/Undo/reload/close pass.
  Corrected extra-seed coverage still diverges in CPU/clock at212 and remains
  rejected. Timing/export gates for the core correction are tracked in the
  handoff; retain strict per-build full-state/audio checks. No schema/ABI change.
  Five pairs measure+0.066607ms (+1.157%) mean-work cost for correctness, not a
  speedup. Current Docker exports pass349Windows/Wine and350Linux tests,15optional
  ignored each; archive/compatibility and actual Linux cold-cache GUI/replay
  gates pass. Prior archives preserved; canonical native builds also updated.
- CPU PPU timing uses scoped thread-local capture outside machine/save state.
  `FFTA_PROFILE=1` includes scanline/zoom/core-remainder rows on desktop;
  `--ppu-profile` enables separate headless diagnostic runs after warm-up.
  Disabled entry points check capture state without reading a clock. Capture
  is bounded, incomplete frames are discarded and parallel zoom measures the
  caller's wall time including worker waits. No guest or translation ABI change.
- Native UI extraction caches window registers/vertical membership, modal flag
  and native origin only within one `capture_dialogue_part` call. WIN0 overrides
  WIN1, then OBJ window and outside; horizontal/vertical wrapping is unchanged.
  Values are reread on each call, so DMA/scanline changes and load/Undo need no
  persistent cache invalidation. No serialized guest state or ABI changes.
- Native panel area/banner rules are snapshotted per capture call and lazily
  evaluated once per eligible pixel across BG candidates. Missing bounds remain
  unevaluated under overriding banners or absent BGs. The uncached test policy
  and full state comparisons preserve clipping/blending/sort semantics. Native/
  Wine173core tests pass;3530frame/state parity includes zoom6. Five pairs show
  2.69% less core work and2.26% less complete unpaced GUI work on native Linux;
  worldmap/default GUI checks pass. No guest/cache ABI or Windows-speed claim.
  Final Docker335Wine/336Linux tests,15optionalignored, archive compatibility/
  privacy and cold-cache actual Linux portable GUI pass. Local dist is updated;
  prior archives preserved. Subsequent world-only pixel work is also retained
  and verified (see Desktop behavior); it is not a next candidate to repeat.
  PERF-03 correctness controls and PERF-08 CPU PPU reductions meet their TODO
  criteria. Remaining host cost or an open parent does not reopen closed trials.
- Native BG2/BG3 UI planes are prepared once per hardware scanline and borrowed
  by primary/extra and normal/backing extraction. Backing text filtering uses
  a private copy; Stats' zero mask uses transparent planes. Scratch lifetime
  ends at the scanline, so no cross-frame/write/restore invalidation is needed.
- Native UI rows with transparent text/backing and no portrait candidates clear
  their destination and bypass world-candidate sorting/blending. Allocation,
  anchor updates and capture-flag cleanup precede the check. Nonempty rows keep
  the full pixel loop; that loop is also the test-only const-generic reference.
  No hardware scanline, CPU step, guest cycle or interrupt is skipped.
- `FFTA_PPU_STAGES=1`, alongside the existing PPU profile, adds optional disjoint
  per-frame BG/OBJ/composition/UI capture/setup/residual totals. The flag is read
  at profiler creation; detailed clocks stay off otherwise. Nested UI preparation
  belongs to capture, and the totals partition scanline time. The comparison
  parser accepts older baselines without these optional rows and validates any
  supplied stage report. See the performance workflow for scope and overhead.
- Headless `runc --state` and the comparison script can resume desktop slots
  or legacy raw snapshots through the menu validator. Snapshots and cartridge
  `--sav` inputs are mutually exclusive. Replay length, input, zoom and warm-up
  use a relative origin while guest frame/clock values remain intact; existing
  no-snapshot replays retain boot origin zero. Comparisons check loaded machine
  identity and exact absolute frame ranges, and keep loading outside timing.
  Inputs and personal snapshots stay local and read-only. See the performance
  workflow for the diagnostic CLI and its remaining AI-scene coverage limits.
  To record from a supplied battle snapshot, use an isolated in-game suspend
  save, then resume during a new recording session; checkpoint saves are allowed
  while host loads remain prohibited. Rebase subsequent events to the checkpoint
  origin and visually verify the enemy action and the replay endpoint.
- `runc` guest-cycle profiling (`RECOMP_COST_FROM`) observes the shared native
  dispatch loop, preserving normal HLE/interpreter gates and strict fallback
  behavior. `COST_DISPATCH` separates native, missing, system and intentional
  hook dispatches. Its per-PC charged cycles are guest time, not host timings;
  keep the census disabled for timing comparisons. See the performance workflow.
- [Windows runtime performance](../windows_runtime_performance.md) documents
  the tested replay and matching framebuffer/OAM results. Timing improvements
  were measured under Wine, with environment lookup costs specific to that
  setup; they do not establish native Windows gains or AI-turn performance.
- Windows GNU tests and interactive FPS/menu checks ran through Wine. Menu
  gamepad smoke checks used a simulated XInput controller. Native Windows and
  physical controller coverage remain to be obtained.
- Check host load and the pinned CPU's SMT sibling before timing. VS Code npm
  discovery can follow ignored build/Wine test trees and saturate the host; the
  root Rust source tree has no npm manifest. See the performance workflow for
  the observed confounder and local workspace-only `npm.autoDetect` workaround.
  `.vscode/` remains ignored; no global editor settings are required.
- Some gba-core tests share display globals. Use serial execution for that
  suite; distinguish a baseline parallel failure from a regression.
- Optional TinyCC observer checks need its ELF relocatable object loader even
  for Windows. Record the backend and checks actually used.
- Keep test saves/config separate. Wine may replace APPDATA supplied by the
  caller; verify the effective location within the isolated prefix.
- Local `target/` and `dist/` files are ignored build artifacts, not portable
  prerequisites. Discover installed toolchains and rebuild when absent. Never
  include ROMs, saves, recordings or test controller DLLs in release archives.
  [Local artifact retention](LOCAL_ARTIFACTS.md) requires closing successful
  managed/native units with `scripts/local_artifacts.py complete`: retire only
  disposable interrupted/moved fixture copies after preservation/results, retain
  current required offline data and proof logs, and review superseded datasets
  and old test binaries under the bounded retention policy.
  Refuse links/private/tracked content. Historical mutable-output maps are not
  perpetual live-input requirements; cleanup receipts describe availability,
  without reopening completed proofs. Coverage Docker launchers keep two owned
  automatic cache generations after successful publication; explicit datasets
  remain untouched and read-only direct generation defaults to no eviction.

## Native UI coordinate policy

Native UI GPU draws now use cached RG32Uint axis maps computed with the existing
CPU pixel-centre/floor/clip/clamp formulas. This avoids backend-dependent float
reassociation selecting adjacent texels at fractional world boundaries. Keys
include exact layout/clip/view bits, world dimensions and the device limit;
resource recreation invalidates maps. Conservative source bounds use the exact
transparent-pixel predicate and integer-map-derived scissors. Invisible layers
still claim shared world sources. No guest timing, snapshot or zoom/FOV change.

Game UI layers also receive the current Grid parameters and share
`crates/screen/src/grid_filter.wgsl` with the world pass. Grid phase and
small-scale fade follow the UI's own layout/Size, not world zoom; the original
coordinate maps still define centre samples and transparent coverage. FPS
overlays explicitly opt out. Software fallback and the pause canvas remain
grid-free. See [dialogue layout](../dialogue_layout.md).

The explicit Radeon tests match CPU for fractional crops and 32 persistent-resource
zoom/resize/DPI states. The original table-only prototype regressed 4.17% and
was rejected; bounded rasterization plus the maps improves the five-pair native
unpaced replay median mean by 5.53%. World-map correctness, native GUI
load/Undo/reload/close and physical Radeon Wayland replay pass. Final exports pass 332 Windows/Wine and 333 Linux tests (14 optional ignored
each), archive privacy/integrity and PE/GLIBC compatibility checks. The actual
Linux archive passes cold-cache GUI load/Undo/reload/resume/close with inspected
captures. Evidence is tracked in the [record](../changes/2026-10-21-align-native-ui-sampling.json).
This is not an AI-turn or native Windows performance claim.

## Maintaining this knowledge

The [debugging model workflow](DEBUGGING_WORKFLOW.md), routed by AGENTS and
`ffta-debugging`, keeps GPT-6.1 Terra as the client-selected primary agent and
consults GPT-6.1 Sol for stalled diagnosis before Terra resumes implementation
and validation. Both require Extra High (`xhigh`) reasoning: Terra is set in
the client/CLI and Sol by an explicit delegation override. Model/effort
overrides require supported tools; the skill does not switch the primary
settings or resume paused work. Consultation evidence
and the return action belong to the selected task's handoff entry.

Platform-specific root-workspace scripts are grouped under `scripts/linux/`,
`scripts/windows/`, `scripts/android/` and `scripts/unix/`; shared tools remain
directly under `scripts/`, with tests under `scripts/tests/`. See the
[script directory guide](../../scripts/README.md). AndroidPort keeps its separate
script tree.

Follow [the memory workflow](MEMORY_WORKFLOW.md) and the local skills routed
from [AGENTS.md](../../AGENTS.md). Update this page when facts or decisions
change, rather than adding a second chronological log here.
