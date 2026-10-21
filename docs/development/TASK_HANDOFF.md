# Task handoff

Updated: 2026-10-21. Latest user instructions take precedence. Follow
[resume/repeat decisions](MEMORY_WORKFLOW.md#resume-and-repeat-decisions).
An open parent never reopens completed children or rejected experiments.

## Current request checkpoint

- Latest request: full `rom_data_todo`, readable extracted files without space
  reduction; **updated user steering: do not measure performance regressions
  now, optimize later** (2026-10-21). This supersedes older queued benchmarks,
  startup/RSS campaigns and performance acceptance work. Preserve their evidence;
  continue functional migration/equivalence and correctness checks only.
- **ROMDATA-01-CONTRACT — complete**, inventory/contract in
  [ROM_DATA_INVENTORY.md](ROM_DATA_INVENTORY.md). Both exact local images checked:
  USA direct0x149510/copy0xa398dc and EUR0x14ced8 are identical48byte dimension
  tables. Existing sprite consumers inspected; other families and unknown
  regional/language catalogs explicitly listed. ROMDATA-01 parent stays partial
  for those unverified catalogs; no old runtime/performance work reopened.
- **ROMDATA-02-OBJ-DIMENSIONS — complete**. Bounded core typed decoder and
  source-authenticated schema1 manifest; explicit `rom-data extract|verify` CLI
  publishes complete temporary directories, reuses valid data and refuses
  corrupt/edited data. [Feature](../rom_data.md),
  [record](../changes/2026-10-21-extract-verified-obj-dimension-resources.json).
- Linux release3core/6CLI and Windows GNU/Wine3core/5CLI pass, including both
  exact local ROMs. Determinism, provenance/aliases, roundtrip, forged data,
  bounded/malformed inputs, missing files, relocation and4publishers covered.
  Windows compilation warning-free; first Wine lock setup failed inside the
  sandbox, same binaries pass with normal Wine runtime access. Native Windows,
  startup/runtime/lifecycle/performance remain separate. No normal gameplay
  path, state ABI, translation identity or canonical binary changed.
- All test processes10215/66112/44836 terminal; final Wine runner exited0.
  Evidence: ignored `target/rom-data/` (native-final-tests.log,
  windows-tests-build.jsonl/log, windows-tests.json, wine-results.json,
  wine-*.log, first setup failure, helper scripts and local input locators).
  Do not rebuild/reextract this closed offline unit to reconstruct context.
- **ROMDATA-02-CLI-PRODUCTION — complete**: native and Windows production CLI
  builds warning-free; actual help/extract/reuse/verify pass on both regional
  sources, native versus Wine output files byte-identical. Canonical binaries
  restored/verified by hash; main source mtime refreshed for the next Cargo build.
  Build handles45784/79661 and Wine smoke58438 terminal/passed. Reuse the
  source hashes, build JSON/logs, *-cli.json/log and private `extracted/` data
  under `target/rom-data/`; no rerun needed for these unchanged inputs.
- **ROMDATA-03-OBJ-STARTUP — complete for opt-in first-family preparation**.
  [Feature](../rom_data.md),
  [record](../changes/2026-10-21-prepare-versioned-rom-data-before-boot.json).
  `rom-data prepare` and `play --rom-data` share installed/portable storage:
  source/schema/extractor/decoder paths, immutable generations, OS file locks,
  atomic relative `current`, verified no-write reuse, visible corruption repair
  and progress before save discovery/backups/native compilation. Explicit
  FFTA_RESOURCES_DIR wins; no portable fallback to AppData. Root Data ignored.
- Linux14default +1private regional CLI and Wine12default +1regional pass;
  two Unix-only symlink cases and one ignored subprocess helper explain counts.
  Three forced child deaths prove partial/unpublished/committed handling and
  lock release;4preparers, cooperative cancellation, simulated full storage,
  failed publication, version/source separation and mod/save/cache isolation pass.
  Common extraction writer refactor revalidated only its affected CLI scope;
  unchanged core sources/results remain retained.
- Both production builds warning-free/canonical hashes restored. Actual native
  and Wine CLI pass managed reuse, corruption/missing-file recovery, regional
  switching, portable moves from unrelated cwd, overrides, installed roots and
  pre-save startup failure. Native and Wine interpreter GUI each pass create,
  close0, move, reuse identical generation and close0; language-screen captures
  inspected. No guest/data-consumer or native-equivalence claim yet.
- Native Xvfb setup initially lacked socket permission; original prepared GUI
  fixture reused after access correction. Windows CLI helper initially failed
  to normalize extended-length paths after successful USA creation; only helper
  corrected, existing USA dataset reused, remaining cases completed. No failed
  game run or production fix inferred from either setup failure.
- All handles4860/26330/43547/85711/56348/4043/44560/69677/70697/67204 terminal.
  Evidence ignored `target/rom-data-startup/`: frozen preimages, test/build logs,
  Wine results, *-preparation-cli.json/log, *-gui-results.json/log/captures,
  candidates, canonical hashes, validated-sources.json and helper scripts.
  Source mtime refreshed for Cargo after canonical restoration. No export/commit/
  push; AndroidPort unchanged. Physical power loss/native Windows unverified;
  old/orphan generations retained, directory fsync is Unix-only.
- Final Git inspection observes HEAD8472dd4 now contains the earlier completed
  offline extraction unit. Retained core/Cargo source hashes are unchanged;
  current uncommitted diff is the preparation unit above. No commit was made
  by this agent and no work was reverted on observing the updated HEAD.
- **ROMDATA-04-OBJ-CONSUMPTION — complete for opt-in first-family implementation
  and initial correctness/provenance gates.** [Runtime](../rom_data_runtime.md),
  [record](../changes/2026-10-21-consume-source-verified-obj-metadata.json).
  play/frames/runc --rom-data now bind immutable source-authenticated typed data.
  Four fixed source pages project CPU/DMA/native-API reads; host draw/visibility/
  battle readers use typed data while preserving changed RAM and out-of-family
  reads.432B session allocation; no hot-path I/O or instruction/timing bypass.
  Distinct source/file/dataset identities, base-only constructor, unchanged
  RtApi/native keys and state schema1. Serde-skipped Arc/counters survive
  transactional Load, Slots Undo and backup clean-boot restore.
- **ROMDATA-04-EUR-ALIAS — complete**: original01/02 provenance expanded by new
  evidence, not context recovery. Exact source scan finds two48byte copies per
  region: USA149510/a398dc, EUR14ced8/d976e8. Extractor2 records the additional
  EUR copied-code-area alias; schema/decoder remain1 and managed e1 is retained.
  The EUR host decoder incorrectly selected IWRAM for its main draws. Exact
  original literal ROM15ac ->0814ced8 and original-instruction matrix verify the
  corrected direct ROM source in both legacy/decoded modes. Copied routines keep
  RAM semantics. Actual battle initializer/alias-read trace remains05 coverage.
- Final release suites: native191core/95runtime and Wine191core/94runtime pass
  (7core/8runtime optional ignored). Private regional extractor and file/lifecycle
  checks pass on both targets.48-case actual EUR/USA draw matrix passes original
  register/CPSR/clock-at-every-step, RAM and host replacement attributes; each
  region records239decoded guest reads/288typed host reads. Synthetic poisoned
  ROM, all widths/mirrors/boundaries/native API, DMA and dynamic-RAM gates pass.
- Actual native600-frame legacy/decoded comparisons pass on Linux and Wine:
  every full state, central/wide image, OAM, dispatch and full mixed/PSG/FIFO/HLE
  audio (HLE by bits),3519typed host reads. Native blocks1350656Linux/261866Wine;
  each target uses its own existing TinyCC ELF cache (Wine filename.dll, not
  OS/GCC DLL). No cross-target timing or whole-scene coverage inferred.
  Boot bus reads0 are explicitly NOT guest-use proof: original width reads are
  conditional on clipping/flips, exercised by the stronger48-case matrix.
- Negative probes retained: native-pair-before-eur-alias.log and
  native-pair-final.log matched600states/audio/images but failed the unexercised
  boot guest-read gate; second reports3519dynamicRAM reads, exposing wrong host
  source. Fixed source and coverage established by actual disassembly/literal
  and matrix, not by calling a0-read scene guest use. Private instruction output
  in eur-draw-source-disassembly.txt; no game code/data committed.
- Production Linux/Windows builds warning-free; canonical hashes restored.
  Actual frames/runc120-frame legacy/decoded hashes match with decoded usage,
  protected ROM/library unchanged. Native and Wine interpreter GUI each prove
  positive typed-file use, create/close0/move/reuse same generation/close0;
  inspected captures. File/lifecycle tests also prove missing activation fails,
  loaded resources survive deleted files and legacy bytes/Load/Undo/backup reboot.
- Setup failures only: native GUI helper initially lacked its unrelated cwd;
  no game started, prepared folder reused after helper correction. Wine native
  harness initially lacked explicit TinyCC path; same binary/cache passes after
  specifying the existing library. Wine D-Bus background diagnostics did not
  prevent the successful600comparison/test exit. No production workaround.
- All processes13253/48498/32221/14224/93009/53423/45146/3944/72356/43043/16440/
 2295/30696/87589/50611/11325/13810/13803/75654/49859/17892/17777 terminal.
  Evidence ignored target/rom-data-runtime/: preimages/source hashes, test/build/
  matrix/lifecycle/native-pair logs, exact source occurrences, backend metadata,
  before-alias candidates, headless/GUI results/captures, canonical hashes and
  validated-sources.json. After final builds only new cfg(test) helper formatting
  changed; production code unchanged. Main mtime refreshed after restoration.
  No export, agent commit/push or AndroidPort update. Current work is based on
  HEAD94b81c5 (prior preparation observed committed externally).
- **ROMDATA-05-OBJ-VALIDATION — partial; ROMDATA-05-OBJ-SCENES-COSTS complete
  for the declared local EUR unit.** Latest request remains the full
  ROM_DATA_TODO. [Protocol/results](ROM_DATA_VALIDATION.md),
  [record](../changes/2026-10-21-validate-decoded-obj-scenes-and-costs.json).
  Limits fixed before measurements; source04 hashes preserved.13benchmark
  script tests, both synthetic tar/zip private-marker cases and warning-free
  native/Windows release test builds pass. Runner now has explicit per-variant
  --rom-data switches for verification/timing/profile; package includes help.
- Reference94b81c5 plus only the verified EUR source fix has no new resource
  routing/layout. Both variants built from the same isolated source path and
  flags. Initial copy2 preserved old mtimes and Cargo reused reference for the
  candidate (equal hashes/0.20s); caught before any measurements. Candidate-only
  rebuild after mtimes refreshed gives distinct hashes/help. Help helper first
  omitted stderr; capture corrected without a rebuild. Keep original logs.
- Five alternating native pairs, CPU15 with sampled sibling14 and no concurrent
  builds, pass every frame/OAM, loaded/final full state and dispatch. Worldmap240
  median2.153946->2.175660ms (+1.0081%); battle/menu/enemy3530 median5.833322
  ->5.819583ms (-0.2355%). All preselected mean/p95/p99 limits pass. Neutral
  local headless result, not a speedup/realtime/native Windows claim.
- New full-state/audio checks independently pass240/3530/8476frames on Linux and
  Windows/Wine using each platform's existing TinyCC ELF object. Every full
  guest state, central/wide pixels, OAM, dispatch and mixed/PSG/both FIFO/HLE
  sample matches its legacy-read instance; HLE compared by bits with nonempty
  streams after identical post-load observer setup. True-old-binary timing
  comparison above is separate. Actual EUR copied source read at5400, mask3;
  all guest RAM/clock/RNG states match through the boot transition. No DMA/PC
  attribution or USA initializer trace inferred. Source/fixture hashes retained.
- Seven first/cached helper observations: Linux median32.415695/21.018705ms,
  maxima34.267605/21.626677ms; Wine median109.222100/91.891400ms,
  maxima194.175200/183.130300ms. Both predefined median gates pass; cached Wine
  max exceeds100ms and remains recorded. Separate source I/O/decode/preparation/
  activation, every observation and whole-process wall retained. Owned432B;
  independent Linux seven-pair peak RSS deltas672KiB median/752KiB maxima pass.
  Generation1146logical/12288allocated bytes; managed root1164/32768bytes
  including selection/lock/ancestors. Fresh roots contain no old/orphan
  generations; previous datasets were not deleted. Wine Z uses host storage;
  no native Windows RSS/frame-time/filesystem claim.
- All handles39962/36367/92198/14156/11701/82904/14855/70162 terminal; repaired
  build validation also terminal. Wine background D-Bus diagnostics occur but
  both affected tests actually pass; no production workaround. Root production
  bytes/canonical hashes preserved (only new cfg(test) module registration);
  no export, agent commit/push or AndroidPort update. Existing04 work retained.
  Evidence ignored target/rom-data-validation/: reference-build.json/pre-repair
  logs, distinct help/hashes, native-{worldmap,enemy-menu}/summaries/logs,
  host-load samples, native/windows-scene-tests.json and full-state-audio logs,
  startup-{native,windows}/startup.toml, startup-results.json,
  windows-startup-results.json, RSS raw logs, package/benchmark test logs,
  source hashes and helper scripts. These are terminal results, not a rerun queue.
- No remaining deliverable/live process for this bounded scene/cost unit.
  The subsequently completed GUI outcome follows below. Closed timing/scene/
  matrix/lifecycle checks are not a rerun queue. USA initializer, actual updated
  archive and native Windows gates remain distinct before normal enablement;
  other catalogs/families/mods stay in the full active goal.

- **ROMDATA-05-OBJ-GUI — complete for local EUR declared scope.** Previous
  turn was progress; selected new GUI criterion without reopening completed
  headless timing/scene/matrix/file tests. [Record](../changes/2026-10-21-validate-decoded-resource-gui-lifecycle.json).
  Source-verified existing candidates, TinyCC objects, datasets and battle slot
  reused in new isolated config/save/cache roots. Linux native and Wine native/
  interpreter pass held/released/toggled Fast, pause/Return-resume preserving
  Fast,1xreturn, Slot Load/Undo/reload and WM close0. Captures inspected; Fast
  ratio4:1and1x~59.7. Positive decoded guest/host ROM/RAM use on all paths.
- Actual CPAL44.1kHz Linux/Wine48kHz callbacks execute all contexts with final
  volume0. Fast382/1634/1631callbacks; device errors, producer drops and
  over-period callbacks0. Transition priming/waiting and Linux playing holds
  retained: no steady zero-starvation/audibility/A-B callback speed claim.
  Native smaller virtual display clips the wide window right edge; FPS/menu
  and actual battle inspected. Complete-image parity is the closed headless gate.
- Separate actual Wine/native backup confirmation/restoration/restart passes:
  safe Cancel default, exact restored primary bytes while paused, Resume clean
  boot, backup order/content preserved, slot/source-save unchanged and resource
  ownership retained through reset.3stream constructions/0device errors.
  All original fixture/library/executable/dataset hashes and imported slot bytes
  protected. Four handles52685/14957/14446/26083terminal/pass, no live process.
  Evidence target/rom-data-gui/{verification-summary.json,run.py,run-backup.py},
  native/windows-native/windows-interp/windows-backup result/measurement JSON,
  play/Xvfb/Openbox logs and inspected PNGs. No runtime rebuild/code change,
  canonical/export update, agent commit/push or AndroidPort update. HEAD69cf601
  observed externally; preserved existing work, no agent commit.
- No missing deliverable for this bounded GUI unit. Subsequent actual archive
  results follow below. USA initializer/native Windows remain separate gates;
  all remaining catalogs/families/mods stay in the full active goal.

- **ROMDATA-05-OBJ-ARCHIVES — complete for declared container/host scope.**
  Full goal unchanged; GUI previous unit was progress. Required artifact work
  selected with checkpoint; static inspection found packager's required resource
  guides absent from Docker allowlist/COPY. Fixed both before actual builds.
  [Record](../changes/2026-10-21-validate-decoded-resource-portable-archives.json).
- Both build/test/export stages pass392Windows/Wine and395Linux tests,24optional
  ignored each; compiler output warning-free (cold apt optional man-page warnings
  retained). Explicit target/rom-data-archives outputs. Archive privacy/content/
  nested TinyCC source hashes, safe paths/types/modes, PE/resources/imports and
  Linux maximum GLIBC2.35/ldd checks pass. Windows DLL set onlylibtcc.dll.
- Actual extracted binaries, not previous host candidates, automatically find
  bundled TinyCC, create portable Data/resources without overrides, then pass
  battle Slot Load/Undo/reload/resume/WM close0 with inspected captures and
  positive decoded bus/host ROM/host RAM use. Public archive files and all
  source fixtures unchanged; test native caches/data inserted only after privacy
  inspection and never added to zip/tar. Both archive hashes retained in results.
- Guard exception: one prior dist/docker-linux/BUILD-INFO.txt hash changed;
  every prior archive and canonical runtime hash matches. Current metadata
  matches the identical current archive; cause unestablished, exact old bytes
  unavailable. Do not claim all metadata preserved, invent ASLR addresses or
  rerun successful builds. Both exports had passed before guard4102 exited1;
  build-results explicitly reconciles that exception. Scanner initially matched
  an empty TinyCC file to an empty prepare.lock hash and inspect.py shadowed
  stdlib traceback reporting; helper renamed/empty non-asset hash excluded,
  surviving extraction verified byte-exact, both inspections completed without
  another build/unpack. Not a game-data leak or production runtime failure.
- All handles4102/48361/6979terminal; no live work. Evidence ignored
  target/rom-data-archives/: frozen source/build-results.json, per-platform
  build logs/test counts, docker-* archives/BUILD-INFO, inspection JSON/PE dumps,
  verification-summary.json, actual extracted-root result/measurement/logs/PNGs,
  metadata difference/preimage-search evidence, original/corrected helpers.
  Source/implementation and canonical bytes preserved; outcome docs updated
  after frozen export inputs, no extra runtime rebuild. No agent commit/push,
  external publication or AndroidPort change.
- This artifact unit has no remaining deliverable. The subsequent independent
  USA initializer result follows below. Native Windows remains distinct before
  normal enablement; remaining catalogs/families and mods stay in the full goal.

- **ROMDATA-05-OBJ-USA-INITIALIZER — complete on Linux and Windows GNU/Wine.**
  [Record](../changes/2026-10-21-verify-original-usa-resource-initializer.json),
  [precise protocol/scope](ROM_DATA_VALIDATION.md#original-usa-initializer).
  Original USA/EUR entries/literals independently located; code remains ROM.
  New cfg(test) helper reaches the actual USA entry from clean boot/demo input,
  no save/forced PC. Both targets reach frame791/clock222325013, compare13full
  original-instruction states and observe24decoded reads at0814186c. Entire
  3648byte transfer and post-activation poisoned48byte raw alias probe pass.
  This is initializer parity, not full-USA scene/audio parity.
- Both test builds warning-free; native/Wine test logs terminal/pass3.44/3.53s.
  Evidence target/rom-data-initializer/: copy-pointer-xrefs.json, regional
  overlay-copy disassemblies, build logs/metadata and native/windows-result.json
  plus usa-initializer logs. Lost/truncated observation resolved by cheap reads,
  no successful test repeated. Source ROM/canonical binary hashes preserved.
  HEAD3a9313b includes earlier archive work committed externally; no agent commit.
  No production/ABI/cache/schema/archive/AndroidPort change or live process.
- User confirms **only Linux/Wine, no native Windows PC**. This does not waive
  the native Windows gate; normal enablement remains pending. No remaining
  deliverable for this initializer unit. Next select an unfinished resource
  family inventory under the full ROM_DATA_TODO; closed first-family tests,
  timings, GUI and archive checks are not a rerun queue.

- **ROMDATA-01-GRAPHICS-LZ77 / ROMDATA-02-LZ77-DECODER — complete for observed
  early producers and offline codec**, not a graphics-family migration.
  [Contract/results](../rom_data_graphics.md),
  [record](../changes/2026-10-21-decode-verified-boot-graphics-streams.json).
  Four independent1200frame EUR/USA Linux/Wine traces terminal/pass; each has
  4LZ77SWI11calls, matching actual output and trace bytes across targets.
  Bounded decoder verifies all8regional ranges on each target against retained
  guest outputs without boot replay.125/133/1481/2507source bytes decode to
  160/456/1996/2960; payload hashes equal across regions. Four synthetic tests
  per target pass including both BIOS11/12oracles, malformed/truncated/size/
  history, overlap/maxdistance4096 and exact consumption. Both builds warning-free.
- Original callers independently identify88byte descriptor rootsUSA569104 and
  EUR7662a8; field0c of record0resolves the first observed source. Count, other
  fields/selectors/raw alternatives, downstream packet semantics/sharing and
  regional/language completeness remain unknown. No active dataset/BIOS/cycle/
  startup/default/schema/cache/save change, speedup or native Windows claim.
- Evidence target/rom-data-graphics/verification-summary.json, native/windows
  result/decoder JSON, core test logs, calls.toml/destination dumps/verified
  source reports and original caller disassembly. All selected processes
  83624/78791/56413/92756/31383/55161/20309/9803/79899/75582/68474terminal;
  setup/compiler failures explicitly retained. Observer typo fixed before
  tracing. Stale native library binary dep-info tracked previous isolated
  measurement source; original fingerprint preserved and only affected lib
  fingerprint invalidated. Source touch alone had failed; no successful trace,
  prior measurement or reference source changed/repeated. Canonical ROM/runtime
  and original trace/output hashes preserved. No agent commit/push/export.
- No remaining deliverable/live process for these bounded units. Full goal
  continues; native Windows unavailable and not waived. **Next open unit:
  ROMDATA-01-BG-DESCRIPTORS**: verify count/bounds, relative fields, selectors and
  downstream packet meanings of the actual88byte catalog before designing a
  managed graphics resource. Select with checkpoint; reuse the exact traces,
  offline decoder and caller disassembly, not old OBJ benchmarks/GUI/archives.
- Initial selection checkpoint (historical): full goal and Linux/Wine-only availability
  unchanged. **ROMDATA-01-BG-DESCRIPTORS — selected / in progress**. Preceding
  producer/decoder units are closed with terminal evidence; no build/test repeat
  selected. Known: regional roots, stride88 and record0field0c pointer. Missing:
  independently bounded catalog, complete field semantics, selectors and packet
  consumers. Next inspect exact regional records and original loader routines
  with bounded local reads/disassembly; preserve unknowns rather than infer
  validity from a plausible pointer sweep. Existing uncommitted work retained.
- Automatic continuation checkpoint: previous turn **progress** (new USA
  initializer coverage, bounded LZ77 decoder and regional producer evidence),
  confirmed by terminal verification-summary.json and source hashes. Full
  ROM_DATA_TODO remains active. **ROMDATA-01-BG-DESCRIPTORS continues**, with
  no successful build/replay reopened. Missing deliverable remains catalog
  bounds and packet/field semantics; next read the original packet consumers
  and validate a typed packet interpretation against the saved outputs.
- Initial static inspection retained in target/rom-data-graphics/
  descriptor-candidate-structure.json: first relative payload offset0x37b0
  aligned with162records of88bytes, initially a count hypothesis.
  The later authenticated contiguous-prefix result below supersedes that
  hypothesis for this metadata block; other catalogs remain unverified.
  Record0fields04/08/10point to an outer11ffffff wrapper, with the actual
  observed LZ77 stream4bytes later; field0c points directly to LZ77. Several
  later fields mix optional relative pointers and absolute VRAM destinations,
  so treating all22words as pointers would be wrong. No new decoder/catalog
  implementation was selected from this observation alone. Subsequent packet
  and metadata outcomes follow below; this historical exploration is not a
  rerun queue. Full graphics descriptors/payload coverage remains partial.
- **ROMDATA-02-BG-PACKETS — complete for offline typed packet component.**
  [Contract/scope](../rom_data_graphics.md#typed-background-packets),
  [record](../changes/2026-10-21-decode-background-patch-packets.json).
  Halfword literal/external-dictionary syntax and compact paired-plane
  dictionary8/16bit indices, implicit/explicit pairs, ordered repeated writes,
  exact terminators and destination/cumulative-write/packet limits implemented.
  Three core tests per target and four EUR/USA Linux/Wine original-code cases
  pass; each case7invocations/87160instructions, all EWRAM at completion matches.
  Captured payloads reused without boot replay. Wide indexed plane matches
  unlimited and2048byte chunked original processing; narrow index also checked.
- Literal plane destination assumption was contradicted by the original
  isolated branch. Typed syntax retained; calculated-plane accessor refuses
  that interpretation. Recording-bus negative probe on both regions/targets
  confirms a write at region0 with plane RAM unchanged. This probe does not
  advance the hardware clock; no timing/full-CPU/audio/IRQ equivalence claimed.
  Initial8bit exploratory parse was also discarded after flag inspection showed
  captured16bit indices. Failed literal pre-guard log/builds retained under
  target/rom-data-bg-packets/before-literal-destination-evidence/.
- Evidence target/rom-data-bg-packets/verification-summary.json, native/windows
  results/build metadata/logs, core tests and original-packet logs. Source ROM,
  canonical runtimes and old capture/trace/output hashes unchanged. Final API
  doc comment only changes the consumed-field description; reconstructed tested
  preimage matches its recorded source hash. No executable change after tests.
  All processes57110/59294/51628/91395/69378/42448/80069/76845/70652/9101terminal;
  no live work or remaining deliverable for this bounded packet component.
  HEAD10af0e6 now contains the prior initializer/LZ77 unit committed externally;
  no agent commit/push, public archive or AndroidPort change.
- The subsequent metadata result closes the contiguous-prefix/selector
  criterion of **ROMDATA-01-BG-DESCRIPTORS**. The broader graphics family stays
  partial for linked formats/programs, other catalogs and language coverage.
- Continuation checkpoint: preceding turn **progress**, verified by the closed
  packet component's four terminal original-code results and protected hashes.
  Full goal and Linux/Wine-only availability unchanged. **ROMDATA-01-BG-DESCRIPTORS
  continues**: define a bounded authenticated catalog and verified field roles;
  next inspect existing original getters/loaders and the exact table boundary.
  No packet/boot/scene/performance rerun selected; existing uncommitted work
  preserved. The resulting structural/original-consumer evidence follows.

- **ROMDATA-01-BG-DESCRIPTOR-PREFIX / ROMDATA-02-BG-DESCRIPTOR-METADATA — complete
  for authenticated contiguous metadata contract/offline decoder.**
  [Contract](../rom_data_graphics.md#authenticated-descriptor-metadata),
  [record](../changes/2026-10-21-decode-authenticated-bg-descriptors.json).
  All162records/14256bytes structurally valid at independently read regional
  roots/literals; lowest relative payload is0x37b0, all links outside table and
  within image, transfer triples consistent/aligned. Next payload cannot extend
  the descriptor block. Exact source bytes/hash match across regions.
- Typed sources/optional transfers and variant pointer preserve all88bytes;
  54/55are conditional selectors, not dimensions,56alternate palette key,
  57retained opaque byte. Optional50is a relative source, not a boolean despite
  presence getters. Payload lengths/programs/game-level selector names are not
  inferred. Source-authenticated constructor rejects changed ROMs; bound record
  count512 and strict group/range validation, full metadata roundtrip/serde.
- Three core tests per target and four regional Linux/Wine original-selector
  cases terminal/pass; each2612calls/134629instructions checks all162IDs,
  eight environment values, direct/current forms, ff sentinels and memory-only
  post-authentication fallback probe, restoring its byte; all EWRAM read-only.
  Four deterministic106309-byte TOML outputs identical; builds warning-free.
  Source ROM, decoder/test sources and canonical runtime hashes preserved.
- Evidence ignored target/rom-data-bg-descriptors/: structural-boundary.json,
  build/executable metadata, core/original-selector logs, native/windows-results,
  four TOML/result directories and verification-summary.json. Exact root/loader
  reference disassembly and descriptor-prefix-boundary.json retained under
  target/rom-data-graphics/. Processes86422/7957/80795/66185terminal; Wine
  final runner also terminal/exit0. No successful closed unit repeated, live
  process, agent commit/push/export, public archive or AndroidPort change.
- No remaining deliverable for these bounded metadata units. Full goal stays
  active; native Windows unavailable/not waived. **Next open unit:
  ROMDATA-02-BG-MANAGED**: add authenticated descriptors to managed extraction/
  verification and versioned preparation with explicit per-file/total budgets,
  retaining existing OBJ files/generations; then04actual CPU/native/DMA reads.
  Current private TOML exceeds the OBJ-only16KiB bound and must not be silently
  admitted by relaxing every file cap. Graphics payloads/programs/other catalogs/
  languages remain distinct open migration work, not closed by this metadata.
- **ROMDATA-02-BG-MANAGED — complete for three-file extraction/verification/
  preparation and whole-set activation authentication.**
  [Contract/results](../rom_data_graphics.md#managed-descriptor-extraction-and-preparation),
  [record](../changes/2026-10-21-prepare-authenticated-bg-metadata-datasets.json).
  Schema1/extractor3/decoder2 ->v1/e3-d2; e1/e2 retained. Fixed manifest/OBJ/BG
  filenames,16KiB/16KiB/256KiB caps and272KiB total. All three files synced and
  source-validated before atomic publication/selection; no-write verified reuse,
  missing/corrupt BG repair retains old generations and simulated partial-BG
  failure preserves the prior commit. Runtime activation authenticates both
  metadata files and hashes all IDs/files into dataset identity, while current
  ownership/projections stay OBJ-only; BG prints prepared/runtime=rom.
- Affected release tests: native2core extraction+4core runtime+7file+10preparation,
  Wine2+4+6+9; two Unix symlinks explain counts. Two private regional/lifecycle
  tests per target pass on both ROMs: command handlers, actual old e2-d1import/
  preservation, rehashed edit/missing-file transactional failure, semantic-equal
  annotation changes complete identity while OBJ identity remains, source
  snapshots/Load/Undo/backup preserved. No old fixture regenerated. Concurrency,
  cancellation, forced process deaths, partial write, caps/total and relocation
  pass with the expanded set. Native default checks ran before only the new
  ignored-test module registration; production logic unchanged afterward.
- Warning-free actual production builds and18real CLI checks per target pass
  Linux/Wine. Help, both-region extract/reuse/verify, managed create/no-write
  reuse, BG corrupt/missing repair, root move from unrelated cwd and retained/
  refused e2-d1 pass. Cross-target files byte-identical per region;107861logical
  bytes total. Synthetic tar/zip marker cases exclude new BG names too. Canonical
  hashes restored to3db0749e.../f3e277db... and full protected source/ROM/legacy
  hashes match; main source mtime refreshed for next Cargo build. No archive
  rebuild, export, AndroidPort update or agent commit/push.
- Final formatter only reorders two cfg(test) module declarations; production
  functions/test bodies unchanged. Exact tested frontend source preimage/hash
  retained in tested-rom-data-frontend.rs and verification-summary.json records
  current source hashes plus this difference. Do not rebuild/replay for it.
- Evidence target/rom-data-bg-managed/verification-summary.json, core/file/prep/
  regional/lifecycle logs, build/executable metadata, native/windows results,
  actual CLI candidates/logs/three-file datasets/results, old-generation imports,
  canonical-before.json and marker log. All handles54019/17130/67181/4255/83722/
  20421/16730/40024/97842/44034terminal/pass; no live process or deliverable left
  for this bounded managed unit. Old OBJ scene/GUI/performance/archive outputs
  retain their own frozen revision scope, not repeated for context.
- **ROMDATA-04-BG-CONSUMPTION — complete for bounded metadata implementation
  and initial correctness/provenance gates.** Full TODO remains active; native
  Windows unavailable/not waived. Selection/resume checkpoints reconciled the
  terminal managed unit and missing retained-file/guest/native/lifecycle work;
  no old closed scene/benchmark was reopened. [Record](../changes/2026-10-21-consume-authenticated-bg-descriptor-metadata.json),
  [contract/evidence](../rom_data_graphics.md#direct-descriptor-consumption).
- Immutable owner now retains162typed records plus14256byte source projection.
  Fixed BG pages join the OBJ gate for CPU/DMA/native API reads, with separate
  BG source/file/dataset diagnostics, bus-operation/projected-byte counters and
  residual ROM payloads preserved. No instruction/decompression/timing bypass.
  Predeclared shared/OBJ<=1KiB inclArc and extraBG<=64KiB; measured440+28672=
  29112bytes each target. Schema/extractor/decoder1/3/2, RtApi/native keys and
  state schema1 unchanged. Allocator/RSS and scene/startup costs remain05.
- mem.rs common bus change justified new full serial core suites, not context
  reconstruction: Linux/Wine204passed/7ignored each. Frontend defaults98Linux/
  97Wine pass/18optionalignored. Three new core cases cover all widths/edges/
  mirrors/native APIs, CPU,16/32bit DMA and transactional snapshots.
- Four real-file regional cases reuse existing e3data, poison raw BG ROM only
  after source-authenticated binding and compile only original selector/helper
  code through production TinyCC. Each2608calls/134424original instructions,
  5836native dispatches/660event-boundary interpreter shells/3867native BG
  byte callbacks/3888total BG reads. Legacy/file interpreter and native match
  registers/CPSR/clock/EWRAM/IWRAM at boundaries/every interpreted step; complete
  final states match after restoring only the test poison. BG and OBJ counters
  stay distinct. Missing/corrupt activation preserves the live object/snapshot;
  deleted activated BG file, Load/Slots Undo and backup clean boot retain the
  owner/counters and continue file-backed reads.
- New EUR native1200-frame boot pairs pass every full state, central/wide image,
  OAM, dispatch and mixed/PSG/bothFIFO/HLE audio, HLE by bits; all streams
  nonempty,53BG reads each. Linux6479719/Wine4067051native blocks execute using
  existing target-specific TinyCC ELF objects. No cross-target dispatch/timing
  equivalence or broad BG scene coverage inferred.
- Actual production builds warning-free; updated help and interpreter demo/
  native idle/explicit-demo1200-frame CLI pairs match legacy hashes. Native
  final full-state hashes match too; demo paths53BG reads. runc idle naturally
  reads0BG; only new explicit-input pair added after the helper's overstrict
  gate. Existing generation/no-write reuse and source/library/canonical hashes
  verified. Canonical3db0749e.../f3e277db...restored, main mtime refreshed. Tests
  precede only final static HELP text; current production binaries check it.
- First component harness stalled because it omitted frame-ready dispatcher
  clearing; only harness corrected. Its negative evidence stays under
  before-frame-ready-harness. Initial CLI helper gate retained successful
  frames/idle runs; no production bug inferred. Wine background D-Bus messages
  coexist with successful test/process exits, no workaround.
- Evidence target/rom-data-bg-runtime/verification-summary.json, core/default/
  region/boot logs/results, bounded component objects, source/executable hashes,
  builds/candidates and CLI results/inputs. Handles82663/67480/73487/21731/
  28577/37661/54878/79226/10347/71131/16608/12744/5721/19237/99371terminal;
  no live process or missing deliverable for this04unit. Memory/format/CRLF
  checks pass; no archive/export, AndroidPort update or agent commit/push.
- **Next open unit: ROMDATA-05-BG-SCENES-COSTS.** Select with checkpoint before
  work: current BG bus/owner input differs from frozen OBJ/managed revisions,
  so new BG-relevant scene/lifecycle/GUI and startup/frame/RSS comparisons need
  predeclared limits and actual evidence. Reuse private e3files, source-equal
  snapshots/input/native caches and the warning-free managed CLI reference;
  do not recreate fixtures or rerun old unchanged results for context. Broader
  payload/program formats, other catalogs/languages, native Windows and mods
  remain open in the full goal. No normal enablement yet.

- Selection checkpoint: latest full ROM_DATA_TODO unchanged; previous goal turn
  is **progress**, verified by terminal BG-runtime summary/default/core/CLI
  results and current source hashes. **ROMDATA-05-BG-SCENES-COSTS selected**.
  Existing04implementation/provenance/1200boot is closed, no live process.
  Missing: current BG-relevant worldmap/battle/boot-transition full-state/audio,
  incremental and total memory-path frame/startup/RSS/disk evidence. Next reuse
  recorded snapshots/inputs/native caches and existing release binaries, add BG
  counters/optional existing-dataset support to the diagnostic, then run new
  changed-input checks. GUI is the subsequent distinct05unit, not silently
  declared complete. No old fixture recreation or completed gate repetition for
  context. Native Windows remains unavailable/not waived; full goal stays open.
  Limits before measurements: exact state/audio/images/counts; five alternating
  timing pairs +2% median mean/+5%p95/+10%p99; seven first/cached observations
  median250/100ms; shared/OBJ<=1KiB, BG<=64KiB, peakRSS delta<=2MiB; one e3
  generation <=128KiB logical/256KiB allocated. Old OBJ-only32KiB disk budget is
  not applied to the new106309byte typed BG file. Retain every value/failure.
  Frozen managed baseline and BG candidate were built at the same root path,
  toolchain/release flags; comparing baseline flag-off to candidate flag-on
  measures current decoded-set cost against its preceding ROM-memory path.

- Current05substeps: **BG recorded-scene parity complete** on Linux/Wine for
  worldmap240,battle/menu3530and boot-to-battle8476. Every full state/image/OAM/
  dispatch and mixed/PSG/bothFIFO/HLE stream matches (HLE by bits). Stable scenes
  BG0are reported honestly;8476requires and records53BG reads on each target.
  Handles50145/80051build,91872/92924scene runners terminal/pass. Reuse logs/
  protected-input manifests in target/rom-data-bg-validation; do not repeat
  these closed comparisons merely because the cost parent remains open.
- Startup/storage observations terminal: native first/cached medians51.943/
  40.225ms;Wine66.387/47.001ms pass250/100ms gates. Seven generations per
  target107861logical/118784allocated bytes each; one managed root107879/
  139264bytes. Owned29112B. Native seven-pair RSS median+2864KiB/max+2944KiB
  exceeds the predeclared2048KiB gate; preserve all fourteen original values.
  No assertion of memory acceptance or widening of the measured limit.
  Handles23654/98940terminal; Wine whole-command ready-dataset walls also kept.
- Native timing runner75169 terminal/pass: five pairs/worldmap+1.3600%mean,
  +1.7286%p95/+2.0514%p99; enemy-menu-0.6017%mean,+1.0092%p95/-2.0592%p99.
  All frame/OAM/final-state/dispatch checks and predeclared time limits pass.
  SMT14mean0.661%/0.578%,1s maxima1.98%/12%; no overlapping builds or heavy
  agent jobs. Cost-summary budget-exceeded solely for native RSS. Preserve it.
  No broad frame/scene replay is reopened for startup-parser investigation.
- **ROMDATA-05-BG-RSS-ATTRIBUTION selected within the open cost unit** for the
  concrete+2944KiB peak failure. Next isolate actual typed parser allocations
  and compare managed preparation-only versus active BG ownership with existing
  binaries. A direct cached-rlib probe rejected different build/runtime serde
  trait identities before any parse; isolated Cargo probe now compiles under
  target/rom-data-bg-validation/parse-heap-{probe,target}/. This is diagnostic,
  not a production dependency change. No timing observation overlaps this build.

- **ROMDATA-05-BG-SCENES-COSTS measurements terminal / acceptance partial**.
  Verification-summary.json confirms all six scenes, five-pair timing,
  startup/disk passes and failed RSS+2864/+2944KiB; no process remains. All
  sources/canonical/reference/fixtures protected. Memory/format/link checks pass.
  This closes the observation queue, not the RSS gate or full goal.
- RSS attribution terminal: prepared-only median184996/max185140KiB resembles
  active185004/185224KiB. Typed parser peak TOML0.8=1942055B; inline TOML=
  1952069B, TOML1.1=3381232B — both rejected as remedies, no dependency upgrade.
  Default pretty JSON149261Bfails disk budget; **one-space pretty JSON123663B/
  22528Bparser heap** preserves source-equal162records and fits the existing
  128KiB logical dataset budget with manifest/OBJ. Private probe is evidence,
  not production/RSS acceptance. Handles20794/78286/86024/35283terminal.
- Selection checkpoint: latest full TODO persists; previous scene/cost work is
  progress and terminal evidence above is verified. **ROMDATA-02-BG-JSON selected
  as the concrete RSS remedy**. Missing: versioned direct JSON encoding/parsing
  with strict fields/source validation, old e3retention, actual activation and
  runtime RSS acceptance. Next use existing cached serde_json with deterministic
  one-space pretty output; change only BG file/format, extractor4/decoder3,
  schema1/v1/e4-d3, preserve e3/e2/e1. Manifest/OBJ remain TOML, family ID and
  typed source bytes unchanged. No TOML upgrade or weakened size/memory gates.
  Existing scenes/frame timings are closed: startup-only parser/file change
  requires affected core/file/preparation/parser/lifecycle, actual CLI and new
  startup/RSS/disk measurements; no common bus/emitter/state logic changes to
  justify full scene/matrix/benchmark replays. GUI/archive/native Windows and
  other payload/catalog/mod work remain open.

- Latest user steering supersedes space-minimization work: full TODO still
  requested, but do not optimize/compress the extracted data to save space.
  Existing05measurements remain evidence, not a reason for further disk-budget
  tuning. **ROMDATA-02-BG-JSON remains in progress for direct typed parsing**;
  drop one-space pretty formatting/disk-gate assertions and use standard readable
  JSON. Finish only affected correctness/revision/file/lifecycle/CLI checks,
  then return to unmigrated payload/catalog work. No new benchmark/RSS batch
  solely to satisfy the former artificial space budget. Keep parser input caps
  as malformed-input bounds, distinct from storage optimization. Native Windows
  and real semantic/timing/state requirements remain separate.
  At steering arrival, native test build35551terminal/pass predates only new
  revision-retention test and forthcoming standard-pretty adjustment; no live
  build remains. Cargo.lock only adds serde_json to recomp's existing graph.

- **ROMDATA-02-BG-JSON — complete** for standard readable datasets, superseding
  the former one-space/space-target plan. [Record](../changes/2026-10-21-parse-readable-bg-json-datasets.json).
  Source contract1/4/3, bg-descriptors.json/bg-descriptors-json-v2, normal pretty
  JSON149261B/generation150818B; manifest/OBJ remain TOML. Prior e3/e2/e1 are
  retained. No data compression or further storage-budget tuning. File caps
  remain malformed-input guards; no current RSS claim from the isolated probe.
- Final affected checks32Linux/30Wine pass:2core+7runtime+8/7file+10/9prep
  defaults and five private cases per target (regional/e2, original Load/Undo/
  backup, actual old e3 retention and two regional JSON native/lifecycle probes).
  Each original/native case2608calls/134424instructions,3867native BG callbacks,
  3888reads. Strict parser/identity/source reconstruction/transactional failures
  and deterministic output covered. Warning-free production builds,18actual CLI
  operations/target and real interpreter/native1200-frame pairs pass (53BG reads,
  equal frame/native final-state hashes). Files byte-identical across targets,
  ready generations reused without writes. Tar/zip private-marker test passes.
- Evidence target/rom-data-bg-json/verification-summary.json, preimages/final
  source/build/test hashes, regional/revision/provenance/CLI/runtime results and
  package marker log. Canonical3db0749e.../f3e277db...restored and main mtime
  refreshed. Cargo.lock only adds recomp's already locked serde_json dependency.
  Handles35551/74133/54831/43169/23041/87032/92655/33184/37841/36432/91491/
  79632/56274/52968terminal; no live process or missing JSON deliverable.
  Memory/CRLF checks pass; no export/AndroidPort/agent commit/push.
- Full goal remains active with latest user steering: do not shrink/compress
  extracted-data files to save space; prioritize the remaining real resources.
  **ROMDATA-01-BG-PALETTES selected**. The prior parent graphics inventory is
  partial; no completed descriptor/LZ77/packet/runtime/scene criterion reopens.
  Missing: all162palette-source formats/extents/aliases across EUR/USA, decoded
  color semantics and original producer destination/usage before migration.
  Next cheaply classify headers at the already authenticated catalog's palette
  links from existing JSON files and exact ROMs; reuse the previously observed
  record0LZ77output rather than recreate its boot capture. Then define bounded
  typed palette data and actual runtime consumer with unchanged guest effects.
  GUI/archives/native Windows/other catalogs/mods remain separate, not waived by
  the storage preference. Do not restart any terminal timing/scene build above.

- Palette inventory now terminal for127unique sources per region/162records:
  125tag10LZ77and2tag0raw payloads; strict decoder produces11504u16words
  (23008B), all regional source/output hashes agree. All lengths64..256B in
  32Bsteps;476words have bit15set, so preserve fullu16 rather than normalize or
  infer alpha. Record0matches all four retained original160Bcaptures; no boot
  replay. Raw sources are record5and shared54/56, independently located.
- Exact original palette producers USA0801a42c/EUR0801b59c read the source
  header:10calls the existing LZ77 wrapper; otherwise skip4Bheader and CpuSet
  size/2halfwords. Environment lowbits0/3use this base catalog;1/2instead use
  separate alternate-palette program roots via descriptor byte56. Those remain
  unclassified, not silently included as decoded base palette colors.
  Next add/run a bounded original-producer oracle for every record/direct-current
  argument/base environment on Linux/Wine, comparing complete output/guards and
  EWRAM. Only cfg(test) registration changes production source's text; exact
  preceding tested frontend retained in target/rom-data-bg-json/tested-rom-data-
  frontend.rs. No production binary rebuild/new cost campaign needed.

- **ROMDATA-01-BG-PALETTES — complete for the declared base catalog**.
  [Record](../changes/2026-10-21-inventory-original-background-base-palettes.json),
  [contract](../rom_data_graphics.md#base-background-palette-catalog). Both exact
  images:162records/127unique sources (125LZ77+2raw),15612source bytes ->23008
  decoded bytes. All source/output hashes/aliases agree; record0matches four
  retained captures. Preserve fullu16words/bit15 (476unique/480record-indexed),
  no RGBA/alpha normalization. Raw record5/shared54-56 branches independently
  verified in original regional producer; alternative environment programs
  remain unclassified and outside this declared base catalog.
- Four final original-producer oracles pass: each1296calls/72792instructions,
  complete output/guards atIWRAM3860, unchanged EWRAM, SP restored. All4base
  environment bytes and direct/current ID arguments covered. No full-clock/
  native-replacement or migrated-palette-use claim. Warning-free builds and
  handles86503/47357/89522terminal. Evidence target/rom-data-bg-palettes/
  verification-summary.json, inventory/source/decoded hashes/private buffers,
  exact original disassembly and per-target/profile results. No live process
  or missing deliverable for this01unit; no old boot/scene fixture recreated.
- **Next open unit: ROMDATA-02-BG-BASE-PALETTE-DATA**. Select with checkpoint:
  implement a bounded typed model of full16-bit words grouped by16, explicit
  raw0/LZ7710source codecs and exact consumed extents; malformed size/bounds/
  unsupported tags rejected. Use all162base records (127aliases), not only0.
  Match the completed original oracle and retain its buffers; do not rerun boot
  captures or closed scenes just for context. Then extend managed extraction/
  actual runtime consumers without skipping guest writes/cycles. Source/profile/
  save/native identities must remain coherent. Environment1/2alternate programs,
  other catalogs/languages and mods remain open. Latest user priority stays:
  readable usable data and real migration, no compression/space-minimization
  or additional cost campaigns solely for earlier storage targets.

- Selection checkpoint: full TODO plus no extracted-data space minimization
  remains latest request. Previous turn is **progress**: JSON contract/runtime
  and palette01oracle terminal results verified cheaply above; no live process.
  **ROMDATA-02-BG-BASE-PALETTE-DATA selected / in progress**. Missing: bounded
  typed palette banks, malformed-source checks and complete authenticated
  four-file extraction/verification/preparation. Next implement fullu16banks,
  source0/10decoding and record-indexed catalog; integrate readable JSON as
  graphics.bg-base-palettes, extractor5/decoder4/schema1, retaining e4/e3/etc.
  Source limits are semantic/malformed-input guards, not storage optimization.
  Manifest cap expands specifically for162source provenance entries; no disk
  performance target. Add a borrowed complete-file view to keep validation/
  runtime identity atomic as families grow. Palette gameplay consumption is
  the following04step; do not claim file export alone as runtime migration.
  Reuse cached original palette buffers and four oracle reports, no boot or
  scene/benchmark rerun for context. ABI/state/native identity stays unchanged.

- **ROMDATA-02-BG-BASE-PALETTE-DATA — complete** for typed decoding and managed
  extraction/verification/preparation. [Record](../changes/2026-10-21-extract-authenticated-background-base-palettes.json),
  [contract](../rom_data_graphics.md#typed-palette-extraction-and-preparation).
  Full16-bit banks preserve allbits; raw0/LZ7710bounded sizes/extents reject
  malformed inputs. All162entries and source provenance/aliases are retained.
  Schema1/extractor5/decoder4 adds graphics.bg-base-palettes JSON, four fixed
  files, earlier e4/e3/e2/e1retained. Borrowed BaseFiles authenticates the whole
  parsed set and allIDs/hashes atomically. Palette gameplay still reports ROM.
- Final release defaults32Linux/30Wine plus7private cases each pass(39/37).
  Four original-producer controls now compare new typed model bytes, retaining
  1296calls/72792instructions and exact output/guards/SP/EWRAM each. All banks
  match retained buffers, full-set rehashed edits/missing activation transactional,
  palette whitespace changes full identity while Obj/BG identities/states stay,
  actual e4retention, partial writes and create/reuse/repair/move pass. Existing
  Load/Undo/backup/native BG provenance remains correct with the complete-set API.
- Production builds warning-free,22actual CLI operations per target pass regional
  extract/reuse/verify, all-family corrupt/missing repair, relocation and old e2
  retention/refusal. All4files match across targets; existing OBJ/BG bytes
  unchanged. Actual interpreter/native1200pairs match frame/native final-state
  hashes and53BG reads; palette diagnostics explicitly prepared/runtimeROM.
  Canonical3db0749e.../f3e277db...restored; main mtime refreshed. Synthetic tar/zip
  privacy markers pass. No compression/space tuning/new cost campaign or export.
- Evidence target/rom-data-bg-palette-data/verification-summary.json, source
  preimages/hashes, defaults/private/typed-oracle/provenance logs/results, actual
  four-file datasets/CLI/runtime results and old revision imports. File sizes:
  palette270075B,manifest26353USA/26359EUR,total446114/446120B. Input guards
  64KiBmanifest/16KiBOBJ/256KiBdesc/512KiBpal/768KiBtotal, not disk targets.
  Handles5597/46792/37763/53062/30348/79211/45316/30694/78893/57047/54720/
  70424/89500/37424/87785terminal; no live process or missing02deliverable.
- **ROMDATA-04-BG-BASE-PALETTE-CONSUMPTION — complete for the declared base
  catalog and initial correctness/provenance.**
  [Record](../changes/2026-10-21-consume-authenticated-background-base-palettes.json),
  [contract/evidence](../rom_data_graphics.md#direct-base-palette-consumption).
  Immutable typed banks now feed authenticated palette LZ77HLE; exact consumed
  source bytes beside banks supply CPU/DMA/native/guest-BIOS reads. Raw CpuSet
  stays unchanged. Existing SWI/control flow/modeled cycles and ordered writes,
  ROM/native/save identities, RtApi and state schema are preserved. Real BIOS
  byte views are tested, not execution with an actual BIOS image.
- Completeness required exact source metadata (unused flags/header/streams),
  validated against banks and authenticated rather than recompressed. Current
  schema1/extractor6/decoder5, palettev2JSON, v1/e6-d5 retains e5and earlier.
  Palette513639B, complete USA689678/EUR689684B; guards64KiBmanifest/16KiBOBJ/
  256KiBdescriptors/1MiBpalettes/1280KiBtotal are malformed-input limits only.
  Owned total123833B=448shared/OBJ+28672BG+94713palette on both64-bit targets;
  no new memory budget or storage optimization campaign.
- Terminal release210core each,101Linux/100Winefrontend defaults pass (7/22
  optional ignored). Core covers all widths/mirrors/edges/native API, DMA16/32,
  SWI11/12/rawCpuSet destinations/fallback, full state and ownership. Four
  original interpreter/native producer cases each pass1296calls/72792instr /
  4208native dispatches after poisoning all old source spans including headers.
  Each owner1272LZ77calls/231936decoded bytes/3544bus reads/5720source bytes.
  Load/Undo/backup, failed binding/deleted files and subsequent file-free use pass.
- New native EUR1200-frame pairs match every full state/image/OAM/dispatch/audio
  on Linux/Wine and consume one160Bdecoded palette plus53BGreads. Actual24CLI
  checks pertarget pass extract/reuse/verify/repair/move/e5refusal+retention;
  e4retention/regional models/source equality also pass. Current four files
  match across targets. Actual interpreter/native1200framepairs match hashes
  and native final state with positive decoded use, no dataset writes.
- All handles43420/67942/23109/10824/67817/95295/29692/17778/51099terminal;
  both production builds warning-free, canonical runtime/ROM/native cache/input
  hashes verified, mainmtime refreshed. Evidence ignored
  target/rom-data-bg-palette-runtime/verification-summary.json, core/default/
  private reports, producer outputs, regional/CLI datasets, runtime pairs,
  canonical backups/hashes and scripts. No export/agent commit/push/AndroidPort.
- **ROMDATA-05-BG-BASE-PALETTE-SCENES — complete for declared EURscenes.**
  [Record](../changes/2026-10-21-validate-decoded-base-palette-scenes.json),
  [acceptance/results](ROM_DATA_VALIDATION.md#base-palette-e6-scene-acceptance).
  New e6palette paths justify affected checks; all six Linux/Wine cases pass:
  worldmap240,battle/menu3530,boot-to-battle8476. Every full state/image/OAM/
  dispatch and mixed/PSG/bothFIFO/HLE-bit audio matches. Stable segments have
  palette/BG0; transition requires/records one160Bdecoded palette, one tag-byte
  source read and53BGreads. Zero-use stable scenes are not source-use proof.
- Only ignored diagnostic counters/required-use/progress changed. Both release
  test builds warning-free; handles10397/47587/24819/44936terminal. No repeated
  core/default suite or production rebuild needed for this test-only change.
  Protected e6files/source/snapshot/input/cartridge/native/TinyCC/testexe and
  canonical hashes retained; no fixture recreation/new extraction/export.
  Evidence ignored target/rom-data-bg-palette-validation/verification-summary.json,
  six logs, protected manifests, source hashes and build metadata. Memory/links/
  CRLF checks apply at handoff. HEAD8468b68observed externally committed earlier
  palette-data work; no agent commit/revert or code conflict.
- **ROMDATA-05-BG-BASE-PALETTE-COSTS — deferred by updated user instruction;
  selection closed, no measurement/optimization queue.** User now explicitly
  says to avoid measuring performance and optimize later. Existing first/cached
  startup/nativeRSS/Wine-launch and complete worldmap five-pair observations
  are retained under target/rom-data-bg-palette-validation/. Battle timing was
  stopped on request, not accepted/retried: runner3457exit143, only its exact
  runner/benchmark/runtime descendants terminated. Sandboxed /proc could not
  see the runner; scoped host-process access found/stopped it successfully.
- performance-deferred.json records the stop and excluded claims. Old raw
  numbers and RSS diagnostic failure remain historical evidence, not work to
  fix, finish, repeat or optimize now. No remaining performance process, no
  complete battle timing/profile claim, no performance gate blocks ongoing
  functional migrations under this user steering. Normal-enable/nativeWindows/
  other-family correctness questions remain separate, not waived by this note.
- Functional GUI/archive checks remain outstanding; old GUI telemetry helpers
  require removal of profiling/FPS/callback-period measurements before reuse.
  Recorded boot trace places the actual base palette call atframe1161; loading
  a later battle snapshot before that does not establish new palette use.
  Do not call zero-use GUI observations source-use proof or recreate fixtures
  merely to resume. Existing core/original/native/scene correctness stays closed.
- **ROMDATA-01-BG-ALTERNATE-PALETTE-PROGRAMS — complete for valid indices.**
  [Record](../changes/2026-10-21-inventory-alternate-background-palettes.json),
  [contract](../rom_data_graphics.md#alternate-background-palette-catalogs).
  Independent USA427a04/42cc80and EUR489bb0/48ee2ctables, consumers0800543c/
  080054a0. Environment2first/1second;157keys0..156each,314streams/region,
  BEu32size256, literal/copy/zero custom codec. Source39933B/output80384B;
  every regional source/output/link-table hash agrees. FFonrecords29/31/144/
  145/146is unavailable; original has no local guard, caller exclusion unknown.
- Warning-free test builds and four original Linux/Wine×EUR/USAoracles pass
  1256calls/2655156guest instructions each; actual consumer/arguments, complete
  output/guards, unchanged EWRAM/restoredSPchecked. Only cfg(test) source added;
  no production codec/dataset/native/state/canonical change or performance
  measurement. Handles62268/18524/85025terminal; native runner terminalinline.
  Evidence target/rom-data-bg-alternate-palettes/verification-summary.json,
  private disassembly/literal/stream inventories, decoded buffers, hypothesis
  decoder and four original reports. FFcases are deliberately not invoked.
- **Next open unit: ROMDATA-02-BG-ALTERNATE-PALETTE-CODEC**. Implement bounded
  typed custom decoder and catalog/source authentication for valid157keys×2,
  preserving fullu16values and FFmarkers. Validate exact consumption, malformed
  headers/tokens/references/truncation, overlap/zero/FFfill and length bounds
  against synthetic original-consumer probes; reuse retained314buffers and
  original oracles on Linux/Wine. Do not recreate old boot/scene fixtures or
  resume deferred performance work. Then managed extraction and actual runtime
  file consumption, remaining GUI/archive/caller-precondition/language/family
  coverage and mods stay in the full active goal. User has Linux/Wine only.

### Previous planning outcome (retained)

- Latest request: resume the TODO, prioritizing ROM extraction into decoded data
  files and actual runtime reads from those resources; mods follow afterwards.
  **FEAT-04-ROM-DATA-PLAN — complete / documentation only**, superseding the
  earlier FEAT-04-ROM-MODDING-PLAN draft selection.
- Delivered: [seven-phase TODO](ROM_DATA_TODO.md), linked from the desktop TODO
  and project state; [record](../changes/2026-10-21-plan-decoded-rom-data-runtime.json).
  Main delivery is extraction/decoding, startup reuse and verified direct
  consumption per resource family. Mod overrides and broader changes follow.
- Source inspection confirms the current ROM is loaded into `MemMap.rom` and
  read from memory. The plan therefore requires measured performance, provenance,
  native/profile/save identity and strict guest/audio/timing equivalence checks.
- Documentation checks pass: seven ordered unique phase IDs, implementation
  unchecked, local links resolve and whitespace clean. No runtime tests/builds,
  extraction, mod loader or speedup is claimed. Previous desktop fixes retained.
- No missing deliverable or live process for this planning request. All seven
  implementation phases remain pending. When implementation is selected, begin
  with ROMDATA-01: resource inventory and the first family to migrate end to end.
  The older tactical/performance queue remains unselected by this request.

## Previous desktop request outcomes

- Latest request: continue the desktop TODO; user confirms the reported INT/EXT
  difference occurs **only with Fast active**.
- **FEAT-02-FULLSCREEN — complete / retained for implemented desktop scope.**
  [Record](../changes/2026-10-21-persist-borderless-fullscreen-menu.json),
  [feature details](../fullscreen.md). Menu/F11/startup share borderless requests,
  default-off `video.fullscreen`, delayed state reconciliation/cancellation.
  Winit/WM retain normal/maximized placement; removed-monitor fallback is bounded.
  Fix real menu resize race: completed-canvas dimensions for GPU and software.
- Native28config/35menu/81runtime and Wine28/35/82 pass (4runtime ignored);
  final runtime suites repeated only after the resize correction. Both production
  builds warning-free. Actual Linux native/CPAL/Openbox and Wine interpreter GUI
  pass heldF11 once,640x360exact restoration, menu/F11 sync, restart fullscreen,
  maximize/unmaximize, VSyncOn, fullscreen minimize/restore, slots unchanged and
  clean exits. Inspected captures; max client1200x779 on both. CPU fallback GUI
  separately passes F11/menu/normal geometry, inspected captures and clean exit.
- Negative evidence retained: first upload640x360->1200x800/fallback panic under
  `resize-race-before/`; pointer-selected root rows under `pointer-harness/` and
  `wine-pointer-harness/` were setup failures. Passed F11/geometry checks reused;
  only remaining menu/restart cases followed helper correction. Empty EGL vendors
  panic inside wgpu initialization before usable window; valid-vendor/unavailable-
  driver setup then exercises actual CPU fallback. No production env switch.
- All processes terminal:30579/7770/52050/75415/49345/85211/26366/2311/75640/
 49486/4020/52453 and Wine36993. Evidence under ignored `target/fullscreen-menu/`.
  Canonical hashes verified/restored, source mtime refreshed. No export/commit/push.
  Hardware/native Windows/physical Alt-Tab/hotplug/DPI remain explicit limits;
  external WM state synchronization is limited to what winit reports.
- No selected unit or missing deliverable for Fullscreen. Under the broad active
  goal, next unfinished feature is **FEAT-03-TACTICAL-OVERLAY**, beginning with
  verified read-only data sources. Audio hardware/event and broader laws coverage
  remain separate; do not reopen any completed code unit to fill those limits.

- **FEAT-01-MUTE — complete / retained.** Audio menu/default-off `audio.mute`
  preserve volume, separate user and temporary mute, consume sources normally.
  Shared startup/live config now applies saved volume too;20ms gain time constant,
  exact zero below-100dB, no audio-only temporal rebuild. [Record](../changes/2026-10-21-persist-independent-user-audio-mute.json),
  [feature details](../audio_controls.md).
- Native27config/34menu/79runtime (4ignored), final gain-curve test and Windows
  GNU/Wine27/34/80 (4ignored) pass; both production builds warning-free. Callback
  tests cover Faithful/Enhanced/Fast/Normal ramp, settled silence, volume changes,
  reference queue/counter parity, temporary mute and discard.
- Actual Linux CPAL44.1kHz/native/Xvfb two-run GUI passes:20->30percent with MuteOn,
  EnhancedOn, restart retains values; unmute checked at Volume0. Inspected
  captures, Fast4x/normal return, unchanged slot and clean exits. Playing171/170
  and Fast108/108callbacks have0holds/drop/over-period; priming/pause excluded.
  No audible-quality/native Windows/controller-hardware claim.
- Processes37379/24412/4406/28130/17962 terminal. Evidence `target/audio-mute/`:
  source preimages/hashes, tests/builds/canonical hashes, Wine, two GUI logs/results/
  captures. Canonical binaries restored/verified, source mtime refreshed. No export,
  commit or push; Fullscreen is now completed above. Closed audio stays closed.

- **BUG-02-FPS — complete for the reported scope.** Inline Video row explains
  INT game / EXT presentations and differing Fast rates. No meter/pacing change.
  User clarification plus existing actual4x~129/32 and normal~59.7/59.7 establish
  expected behavior. [Record](../changes/2026-10-21-explain-fast-fps-counter-difference.json),
  [feature details](../fps_overlay.md). General hardware/VSync limits stay separate.
- Native targeted menu1 and existing FPS3 tests pass. Windows menu warning-free,
  Wine33menu+3FPS pass. Native/Wine640x480 render byte-equal, inspected/legible.
  Initial standalone-native menu build failed Wayland pkg-config setup; established
  combined runtime/menu features pass. No cadence matrix/replay/production rebuild.
- All processes terminal60293/70141; Wine command terminal too. Evidence under
  `target/fps-menu-help/`; reused actual audio captures. No missing deliverable,
  export, commit or push. Remaining TODO features can be selected under full goal;
  neither FPS diagnosis nor the Normal unit below is pending runtime work.

**BUG-03-NORMAL-BOUNDS — complete / retained.**
- [Record](../changes/2026-10-21-bound-delayed-normal-audio.json),
  [feature details](../fast_forward_audio.md). Fixed512KiB/2second queue plus
 2620byte fade tail; overflow keeps latest250ms with10ms stereo fade. Faithful1x
  even with Enhanced configured; no temporary file or callback I/O/allocation.
  Menu delay label, legacy config and existing timeline resets retained.
- Native78runtime/33menu and Wine79runtime/33menu pass (4optional runtime ignored),
  production builds warning-free. Actual Linux CPAL44.1kHz/native/Xvfb4x Normal
  gate9400:11400 passes365/376callbacks, max130287/130783pairs<131072,
 10resyncs each, zero holds/over-period/producer drops, Enhanced sources unrouted.
  Persisted menu choice, inspected captures, slot bytes unchanged, no spool,
  normal return~59.7INT/EXT and clean exit0. Volume0, not audible-quality proof.
- First GUI harness paused at root due to a second Escape, last guest9305<9400
  and all counts0. Retained as setup failure; only unmeasured gates repeated
  after single-Escape correction, no fixture recreation/rebuild.
- All processes terminal (GUI98315 included). Evidence under ignored
  `target/normal-audio-bounds/`: source preimages/hashes, build/test/Wine logs,
  canonical/candidate hashes, GUI result/captures and first failed setup.
  Canonical binaries restored/verified, source mtime refreshed. Memory and
  whitespace checks pass. No export/commit/push; audible/event/native Windows
  hardware checks remain open. Never reopen bounds due to the open parent.

Previous completed consumption request:
- Latest request: fix the second verified Fast audio cause, nominal4x consumption
  exceeding actual guest production. **BUG-03-EFFECTIVE-RATE — complete / retained.**
- Common host production/device-frame clock now drives faithful/PSG/HLE/FIFO;
 200ms estimate,250ms queue correction,60ms reserve,100ms slew and0.05x..request+2%
  limits. Fractional sinc cutoff bank is built once per stream. Priming/rebuffer
  silence is explicit and counted;1x/Normal/Silent/guest timing stay separate.
  Previous transition-reset and warning-cleanup outcomes remain retained.
- Evidence: [record](../changes/2026-10-21-follow-effective-fast-audio-production.json),
  [feature details](../fast_forward_audio.md). Baseline regression536641steady
  holds; corrected callback has zero holds and live synthetic output. Clock72case
  matrix, rate/stall/reset, spectral and rebuffer telemetry tests pass. Native
 74runtime/4ignored plus final2clock checks; Windows/Wine75runtime/4ignored pass.
  Native/Windows production builds pass without warnings, canonical binaries
  restored by hashes. Source mtime refreshed for correct next Cargo regeneration.
- Actual Linux CPAL44.1kHz/native/Xvfb Fast4x steady9400:11400gate:357Faithful/
 359Enhanced callbacks, no holds/PSG-HLE empties/waiting/rebuffer/drops/over-period;
  mean consumption2.2032/2.1906 tracks production2.2030/2.1906, max estimated source
  queue107.3/108.0ms. Slot bytes unchanged, Vulkan/no fallback, captures inspected,
  clean closes. Volume0: no audible-quality or hardware latency claim.
- Earlier compilation-overlap GUI remains variable-load evidence:3rebuffers,
 13171waiting frames,0source holds/empties, speed0.519..2.245/maxqueue177.0ms.
  Only the failed steady criterion repeated after builds. Faithful ambiguous1x
  post-pause image resolved by short Return-resume check:130.0/32.5Fast then
 59.7/59.7normal. Helper failed obsolete launcher assertion after verified exit0;
  completed evidence retained; do not repeat those runs or the old baseline.
- No missing deliverable/live process for the selected cause. Local artifacts:
  `target/fast-audio-rate/` (sources/preimages, validated-sources.json, old/candidate
  hashes, native/Wine logs, GUI results/captures and first negative run under
  gui-under-build-load). Build/test/GUI handles51951/72493/97293/98575/10060/85875/
 43855/56671/2008/14367/43406 are terminal. No commit/push/export update.
- BUG-03 parent remains partial for audible/event/device and native Windows
  validation. Normal backlog bounds and the Fast-only FPS diagnosis are completed
  above. Do not
  reopen the completed consumption fix merely because the parent is unchecked.

Previous completed build-warning request:
- Latest request: resume the assessment/fix of supplied Docker build warnings.
- **BUILD-01-WARNING-CLEANUP — complete.** Scope test/platform helpers to their
  callers, remove the unused FIFO wrapper and unused Linux Gilrs binding, retain
  pure Linux snapshot tests on Windows. Windows-only RTC and hardware-reader
  warnings found during validation are also resolved. No global lint suppression.
  Both Dockerfiles select noninteractive apt configuration for that command only.
- Evidence: [record](../changes/2026-10-21-clean-desktop-build-warnings.json),
  [warning explanations](../docker.md#build-warnings). Native production build and
 184core/26input/69runtime tests pass without Rust warnings (5core/4runtime ignored);
  final targeted4hostclock/4snapshot checks pass. Final Windows production/tests
  compile warning-free; Wine184core/26input/70runtime pass with5/4optional ignored.
- Existing canonical native/Windows runtime and launcher restored by SHA-256;
  portable archives unchanged. No full Docker rebuild: pinned base is not local.
  Local Debconf selects noninteractive. Debian slim apt-utils, combined-bundle
  rehash and omitted-manpage notices are documented; they may remain visible.
  The supplied log ends before final tests/export, despite successful archive
  creation; no full user-build success is inferred from that partial log.
- No missing deliverable or live process for this unit. Local evidence under
  `target/build-warning-cleanup/`: validation/windows-validation.json,
  wine-results.json, build/test logs and scripts, first-pass warning evidence.
  Handles72956/69213/5375/57934/80220/9896/34509 are terminal. Final memory and
  CRLF-aware whitespace checks pass. No commit, push or package update.
- This warning-only unit did not resume audio/performance work. Its former next
  selection is superseded by the completed audio-consumption result above.

Previous audio-transition request and retained baseline evidence:
- Latest request: continue resolving the desktop TODO. User clarified the audio
  report as **Fast audio at4x**, suspecting stale queued output.
- Selected **BUG-03-TRANSITION-RESET — complete / retained**. Queues already
  cleared on Fast transitions, but callback-owned DAC/filter history survived.
  The new host epoch invalidates that history without rebuilding tables; pause
  discards output while preserving the Fast latch, and device loss also discards
  Normal backlog. Load/Undo retains stream recreation. Guest state is unchanged.
- Evidence: [record](../changes/2026-10-21-reset-fast-audio-consumer-history.json),
  [behavior and limits](../fast_forward_audio.md). Before-fix synthetic residual
  peak0.19975911; after-fix20source/speed cases and28fresh-callback comparisons
  pass. Native68runtime/4ignored plus the added transition test pass; final
  Windows/Wine70runtime/4ignored pass. Native candidate builds; canonical binary
  restored by hash. Portable exports and AndroidPort unchanged.
- Actual Linux CPAL44.1kHz/Xvfb/llvmpipe idle-battle checks pass Faithful/Enhanced
  hold/release/toggle/pause/resume/WM-close, inspected captures and unchanged slot
  bytes. Volume0: callback/lifecycle evidence, not audible-quality validation.
  Old launcher helper assertion failed only after Faithful exited0; result was
  retained, helper fixed, only previously unrun Enhanced executed. Initial ALSA
  sysroot and Wine sandbox setup failures are documented, not runtime failures.
- Historical diagnosis: sustained nominal4x consumption exceeded local guest
  production: captures INT121.7/128.2 and EXT30.4/32.1;497/499Fast
  callbacks all hold, ~47%PSG/HLE requests empty, zero producer drops/over-period
  callbacks. Enhanced configured targets are affected too. Normal-playing
  intervals207/205callbacks have zero holds. This is separate from stale history
  and does not establish the user's hardware latency or an audio-event offset.
  It motivated the completed correction above; do not select it as pending work.
- No process live. Local artifacts: `target/fast-audio-reset/` (regression/build/
  native/Wine logs, candidate and restored-binary hashes, isolated GUI inputs,
  captures, `analysis.json`). Test/build handles24498/45705/6631/23587/24241/
  49418/96366/30121 are terminal. No commit, push or package update.

Previous requested first fix and history cleanup remain closed:
- **BUG-01-LAWS — implemented / verified for the observed layout.** Frame-based
  laws recognition keeps the table, HELP and indicators in the centered modal.
  Native183core/67runtime, Wine183core,72EUR reconstructions,24synthetic GPU/CPU
  cases and inspected Linux/Windows-Wine frontend checks pass. Wider coverage
  stays explicit in the [feature doc](../ffta_laws_layout.md) and TODO.
  [Record](../changes/2026-10-21-fix-battle-laws-modal.json); local `target/laws-fix/`.
- **BUG-01-IMAGE-HISTORY — complete / remote verified.** Image-bearing trees
  c890cf0->6eaecd5 and e262699->e459c65 rewritten with all other tree bytes and
  metadata preserved. Exact-lease push and remote tip verified; no branch/tag
  history contains the image. Local recovery checkpoints are not branch history.
- Pending performance work below remains unselected; no completed or rejected
  performance unit was reopened for this bug-fix request.

## Pending performance context (not selected by the current request)

- Previous performance request: continue executing the performance TODO. Prior progress:
  **PERF-07-TIMER-AUDIO-ORDER — complete/retained/delivered**; see evidence below.
- The18-ROM-seed trial remains rejected. Corrected event ordering makes its first
  211frames equal, but CPU/clock differ at212 and final IWRAM/HLE differ. No
  coverage timing/acceptance; its continuous-audio pair case did not run.
- Selected ID: **PERF-07-NATIVE-FRAME-BOUNDARY — in progress**. Previous
  interrupted goal turn made progress: confirmed delayed IRQ in existing snapshots,
  reproduced it with original synthetic code, implemented a guarded prototype.
- Done: source preimages preserved; entry cycle-budget guard and post-store/shell
  recheck, unchanged RtApi layout/save schema, cache revision7. Core180/5ignored,
  runtime66/4ignored, recomp-core3/4ignored and explicit11-case TinyCC regression
  pass. Isolated runtime18081, guarded pair32781 and pair helper66188 terminal/
  passed; checked the interrupted helper handle, no process restarted.
- Final hardened correctness is proven:181core/66runtime/3recompiler tests,
  explicit11-case TinyCC frame/IRQ/DMA regression and matching Windows/Wine
  suites. Pair22246 and CLI47161 terminal/passed: all3530states match at every
  frame, retained and continuous queues; full mixed/PSG/HLE/FIFO streams equal.
  Actual CLI matches all frame/OAM and final state; legacy images also equal.
- Prototype-v1 evidence remains closed and separate. Hardened protocol uses
  len0/budget-in-addr after the positive-length content-guard regression exposed
  an ambiguity; all final sources/libraries are under `native-frame-boundary/hardened/`.
- Direct-query correctness and timing stages are complete, unshipped. Triplets
 55899:legacy5.976894ms/guarded6.242606ms/extra6.056551ms; reference sources/binaries
 are preserved under `native-frame-boundary/hardened/`.
- Selected phase: **PERF-07-NATIVE-EVENT-ALLOWANCE — in progress**. Carry a
 conservative allowance across chained blocks, refresh when insufficient and
 invalidate/recheck after stores/shells. Query/yield-bit protocol uses cache8,
 no state or RtApi layout change. New artifacts under `allowance/`.
- Allowance correctness is complete:181core/66runtime/3recompiler tests,
  explicit11-case native/TinyCC and Windows/Wine suites. Pair7045/CLI27840 are
  terminal/passed: every3530-frame full state, complete audio (HLE compared by
  bits), images and dispatch counts match the direct-query reference. Old
  legacy images match too. Sources/binaries/protected inputs remain frozen.
- Allowance timing78865 terminal/passed: medians legacy5.850195/direct5.997167/
  allowance5.972798/extra5.797168ms. Hints win all five comparisons vs allowance
  (-2.9405% median); overall-vs-legacy gain is small (-0.9064%), not a broad
  speedup claim. Worldmap22807 passes240frame/full-state equality. Separate
  tails86913 complete: no over-budget samples, not repeatable tail-gain proof.
- Selected phase: **PERF-07-EXACT-ROM-HINTS — integration in progress**. Add the
  already-proven18 immutable entries only for the exact Europe hash, using one
  union helper in cache identity and compilation. Preserve user/database labels,
  keep USA/unknown images unchanged, and make hint growth change the digest.
- Next: run metadata/cache tests, build a new isolated runtime and prove default
  translation equals the validated explicit-hint library; then final replay,
  GUI and export gates. New integration artifacts under `integrated/`.
  Source preimages retained; earlier allowance/hardened/reference results remain
  closed evidence. Canonical native/dist still contain timer/audio correction.
- Evidence:[record](../changes/2026-10-21-preserve-native-event-boundaries.json).
  No process live before starting integration validation.

## Latest completed unit and current exports

**PERF-07-TIMER-AUDIO-ORDER — complete.** Event timestamps precede batch endpoint;
DMA steals extend CPU work, DMA clock service avoids video reentry, skip uses
same path. Five ROM-free regressions plus179native/Wine core and66runtime checks
pass (5/4optional ignored). Optional3000frame full-state/frame/OAM/audio observer
regression passes. Actual native/exported CLI preserve3530old frame/OAM/counts
and exactly match corrected final state. Old/corrected baseline final CPU/clock
match, only mixed audio/HLE/timer bookkeeping changes intentionally.
Five alternating pairs:5.756460->5.823067ms (+1.157%,0.066607ms), not a speedup.
Native and actual exported Linux GUI old-slot/zoom/Undo/reload/WM-close pass;
Vulkan/no fallback, inspected battle/Undo captures, cold cache rebuilt28859blocks.
No serialization/ABI/seed change; version1 slots remain valid. Broader native
Windows/physical hardware and the original rare AI stall remain unverified.

Docker349Windows/Wine and350Linux tests pass,15optional ignored per target.
Archive/privacy/docs/frozen-source/PE/imports/GLIBC2.35 gates pass. Local archives
published only after gates; previous versions preserved. Exact validated native
runtime/launcher installed into target/release (do not assume the former native
baseline is still there). Old native reference remains `timer-audio-order/recomp-baseline`.
No commit, push or public release.

Evidence:[record](../changes/2026-10-21-order-timer-audio-events.json).
Ignored artifacts: `target/desktop-perf/timer-audio-order/` (preimage/tests/native
builds/CLI/state components/five pairs/GUI), `timer-audio-order-exports/`
(frozen manifest,staged and inspected archives,compatibility,published hashes,
previous-exports), `timer-audio-order-export-gui/` (actual cold-cache bundle/logs/
captures). Missing local artifacts don't reopen completed work.

Current local exports:
- Windows `dist/docker-windows/FFTARecomp-windows-portable.zip`, SHA256
  `be4cc2fb96c5f379001c9944f4c6c927f2947d4de0fafa67fb3fd1bfae074f68`.
- Linux `dist/docker-linux/FFTARecomp-linux-portable.tar.gz`, SHA256
  `1d7cf99ef8b8734f00687f17e2c515ce1b9738cb3ba8e3c4d534a047b89024a3`.
- BUILD-INFO beside each archive; previous accepted archives under
  `target/desktop-perf/timer-audio-order-exports/previous-exports/`.

**PERF-10-PRESENTATION-CADENCE — previously completed, retained.** Observer,
clock/audio/VSync matrix, silent fallback, default-off/benchmark and lifecycle
checks already exist; no repeat required. It observes API presents, not scanout.
[Record](../changes/2026-10-21-observe-successful-presentation-cadence.json).
Artifacts:`presentation-cadence{,-gui,-no-audio,-exports,-export-gui}/`.

## Previous completed CPU triage and rejected compact IDs — reuse evidence

Current host sample: `target/desktop-perf/post-ppu-host-triage/` has isolated
symbols, source/input hashes, perf.data/self/children/annotate/triage and state/
count proof. Profiling build11285 and sample12711 terminal. Leaf samples don't
split missing/intentional fallbacks; no AI-stall/native Windows speed claim.

Compact-ID trial: native174core/5ignored,WindowsGNU/Wine174/5 and GUI/headless
all3530frame/OAM/state/count equality pass. Five headless pairs-1.34795%; single
instrumented core run slower. GUI five CPU-profile-on pairs-0.45386%,one slower;
five profile-off pairs-0.77360%,one slower. Max observations worse in both GUI
sets. Reject: no sufficiently robust useful GUI gain established, no causal
universal-regression claim. Exact baseline source and native runtime restored
by bytes/hashes; touched source mtime to force Cargo to invalidate stale trial
artifacts on the next build. Windows test artifact is still experimental in
ignored target/, not a canonical delivered binary. No additional tests/builds
needed merely to re-prove the unchanged baseline. No packages changed.

Evidence:[record](../changes/2026-10-21-evaluate-compact-ui-layer-ids.json) and
[workflow](PERFORMANCE_WORKFLOW.md#rejected-compact-native-ui-layer-ids).
Artifacts:`compact-ui-layer-id/` (baseline and evaluated candidate source/bin,
tests/builds/measurement-inputs/evaluation/decision.json), companion
`compact-ui-layer-id-gui-evaluation/` (verification,timing-summary and default-
timing-summary), under target/desktop-perf. Historical measurement manifests
name candidate files that have since been restored at root; decision.json and
preserved evaluated-candidate files distinguish them. Reuse outputs.
Terminal:core86946,native45178,headless54859,GUIverify47650,Windows98255/Wine30695,
GUIprofile49351/default33173. Worldmap/zoom/lifecycle/export gates intentionally
not run after rejection; do not treat these as pending work for that variant.

## Previous completed backend observation — reuse evidence

**PERF-09-BACKEND-OBSERVABILITY — completed for local PipeWire.** One continuous
observer; PID on Client, stream Node client.id, active links to physical sink,
start/end object.serial/state verified. Default piped pw-top buffered tables:
first GUI77632 passes but phase timing is invalid (paused rows same read time).
Preserved that negative result; only corrected backend collection repeated with
`stdbuf -oL`,59031 terminal/pass. Cadence <=1.001s gaps, running states/nonzero
rate/quantum. Idle10samples/~9s,paused3/~2s,resumed6/~5s: stream/sink ERR delta0.
Sink alreadyERR1 at first idle sample; creating-node zeros are not a running
baseline and no time/cause of that existing count is established. Shared graph
errors aren't dedicated hardware XRUNs or proof of game causality.

The unchanged actual Linux portable hash matches canonical export. App gate
8596:9196 complete235playing callbacks,0faithful holds/drops/over-period/device
errors. Backend phases are wall intervals, not exactly that guest-frame gate.
Idle/resumed screenshots inspected, CPAL/Vulkan/no fallback/exit0; slot unchanged.
No runtime/system audio change or rebuild. Native Windows/slow AI-scene/backend
hardware guarantees remain unverified. Do not repeat diagnostics to reconstruct
context or turn the result into a universal no-XRUN claim.

Evidence:[record](../changes/2026-10-21-characterize-pipewire-backend-errors.json).
Local/ignored `target/desktop-perf/audio-backend-observability/` is the buffered
trial; `audio-backend-observability-line/` contains accepted top.jsonl/phases,
active/end dumps/PIDs/analyze.py/result.json/source binary hashes, GUI logs and
idle/resumed captures. Read-only initial inventory also retained. pw-dump needs
approved host access here; sandbox cannot connect. All processes cleaned up.

Current local exports are unchanged from the enhanced-source unit below; no
public release or redundant rebuild was performed.

## Previous enhanced-source unit — exports superseded, reuse evidence

**PERF-09-ENHANCED-SOURCE-EVENTS — completed.** Fixed per-context PSG/HLE stereo
pair and FIFO event requests/empty counts, plus configured-target totals. Warm/
unrouted consumers remain in raw totals. Target is configuration, not audibility,
voice activity, crossfade weight or a defect. No queue/FP/DAC/crossfade policy,
guest/save/cache ABI change. No new callback lock/IO/log/allocation; counters
compile out of the default DSP loop, one dispatch per callback selects variant.

Native63runtime/4ignored pass: exact sample/queue/counter/interpolator parity,
partial stereo pairs, DAC holds, routing/HLE/backlog/mute, bounded/gated totals.
Five synthetic observer off/on pairs:40666.215/40696.723ns per512-frame callback
(+0.0750%), not hardware/game/default-before-after timing. Idle Polly battle
window8596:9196 after restored8476:236callbacks each mode, no empty source reads.
Enhanced targets PSG/HLE; FIFO stays warm, faithful has no enhanced target.
Captures inspected, CPAL/Vulkan, no fallback. Driver initially shadowed keyboard
helper with source-loop variable after faithful passed; preserved first result,
fixed only harness and measured only previously unmeasured enhanced. No runtime
failure or repeated faithful run.

Docker341Wine/342Linux pass,15optionalignored each. Source/doc/archive/privacy/
resources/DLL-set/GLIBC2.35 pass. Actual Linux archive cold-cache load/zoom/Undo/
reload/resume/WM-close passes, inspected battle/FPS/Undo, Vulkan/no fallback/exit0.
Source counts reconcile. Lifecycle playing has2741PSG/HLE empty-pair requests,
660perFIFO and2741faithful holds in one callback; enhanced target counts0 in
faithful mode. Keep separate from idle window, no audible-defect inference.
Default-off has no diagnostic rows/ignores invalid gate; callback-free benchmark
ignores gate, matches240reference frames and preserves private Data. Local dist
updated only after checks; previous archives preserved, no public release.

Evidence:[record](../changes/2026-10-21-observe-enhanced-audio-source-requests.json).
Artifacts under `target/desktop-perf/`: `audio-source-events/` (source preimages,
focused diff/tests/overhead/build/measurement hashes), `audio-source-events-gui/`
(result/captures/original harness-error and resume logs), `audio-source-events-exports/` (frozen manifest/inspection/compatibility/published-artifacts/old exports),
`audio-source-events-export-gui/` (actual archive result/captures/profile/default/
benchmark logs). Terminal sessions:67091/92019/61539 tests,47581 native build,
48191 initial GUI,14067 enhanced completion,24533/31303 Docker,57172 archiveGUI.

Previous local exports (superseded; reuse evidence):
- Windows `dist/docker-windows/FFTARecomp-windows-portable.zip`, SHA256
  `8cc8466020c332876fec5c644facf9a7d4dfe85f50c4ff1435ece354f0f47b93`.
- Linux `dist/docker-linux/FFTARecomp-linux-portable.tar.gz`, SHA256
  `d348913aa3d0b22396fe33befc6f934efac09662f3b2cf7278ae3f89170d2fbc`.
- BUILD-INFO beside archives; previous versions under
  `target/desktop-perf/audio-source-events-exports/previous-exports/`.

## Closed units — reuse evidence

- **PERF-09-INTERACTIVE-GATING — completed.** One producer-frame traversal,
  load/Undo closure, context separation, idle-pair and archive/default/benchmark
  checks done. [Record](../changes/2026-10-21-gate-realtime-audio-by-guest-frame.json).
  Artifacts:`audio-scene-gate{,-gui,-exports,-export-gui}/`. Old exports superseded.
- **PERF-08-UI-WORLD-ONLY-PIXELS — retained.** Default-write world-only pixels
  without sort/blend;174native/Wine core checks,3530frame/state/count parity,
  headless-4.20%,GUI-2.67%,worldmap/zoom6/defaultGUI and final exports passed.
  [Record](../changes/2026-10-21-evaluate-world-only-ui-pixels.json).
  Artifacts:`ui-world-only-pixels{,-gui-evaluation,-gui,-exports,-export-gui}/`.
- **PERF-08-UI-PANEL-ROW-CLASSIFICATION — retained.** Lazy per-pixel bounds and
  row rules preserve missing-bounds/banner precedence;173native/Wine core checks,
  full replay/zoom6 parity,headless-2.69%,GUI-2.26%,worldmap/defaultGUI/exports pass.
  [Record](../changes/2026-10-21-evaluate-ui-panel-row-classification.json).
  Artifacts:`ui-panel-row{,-gui-evaluation,-gui,-exports,-export-gui}/`.

- **PERF-07-FALLBACK-CANDIDATES — triage complete.** New enemy census preserves
 3530frames/state/counts;22.63M native/79.14M missing/55.91M intentional hooks
 after warmup. One pre-panel-cache optimized perf sample3928/zero lost identifies
 capture_dialogue_part31.52%self. Guest counts aren't host time, callchains don't
 fully split missing/intentional work. No lookup/translation change accepted.
 Profile binary isolated, original native restored. [Record](../changes/2026-10-21-characterize-enemy-fallback-host-cost.json).
 Artifacts:`enemy-fallback-census/`; do not rerun the completed census/sample.

- **PERF-04-DEVICE-LOSS — controlled invalidation fixed/complete.** Actual
  destroy+new UI panicked on invalid texture. Owned Device callbacks now latch
  first error in Arc/OnceLock; return fallback and suppress subsequent UI/VSync.
  Scoped errors stay local; no per-frame allocation/wait/fault API. Native58/46,
  explicit four-case native/Wine tests, normal GUI and3530frame Radeon Wayland
  verification pass. Current exports333Wine/334Linux,15ignored; actual Linux GUI
  cold-cache and archive gates pass. [Record](../changes/2026-10-21-handle-owned-gpu-device-failures.json).
  Artifacts:`device-loss/`, `device-loss-{gui,wayland,exports,export-gui}/`.
  Does not prove all asynchronous hardware resets/native Windows faults.

- **PERF-04-MONITOR-TRANSITIONS — complete for virtual X11/Wine outputs.**
  --monitors diagnostic requires two actual enumerated handles/current_monitor,
  A/B/A/B/A, resize/VSync and teardown. Native/Wine unequal1280x720/1280x960
  runs20Presented/0Skipped, actual640x360/640x480 changes and independent positions.
  Actual Linux portable battle+Undo/reload/close passes after explicit evdev US
  keyboard setup; initial intro-only flow is labeled unverified, not a battle pass.
  No physical GPU/DPI/color/Wayland/native Windows claim. [Record](../changes/2026-10-21-validate-virtual-monitor-transitions.json).
  Artifacts:`monitor-transitions/`, `monitor-transitions-{wine,game}/`.

- **PERF-04-MINIMIZE-RESTORE — complete for native Linux X11/Wine Vulkan.**
  Original no-ROM default mode:3observed WM cycles, three resize/VSync tests,
  clean teardown. Native27Presented/0Skipped; Wine18Presented/9Skipped. xprop
  independently confirms3Iconic/4Normal. Actual Linux portable battle then
  Undo/reload/WM-close passes. First Wine D3D12 device creation failed and remains
  negative evidence; successful retry only disables that prefix's D3D12 DLLs.
  [Record](../changes/2026-10-21-validate-window-minimize-restore.json).
  Artifacts:`minimize-restore/`, `minimize-restore-{game,wine}/`.

- **PERF-04-SKIPPED-UPLOADS — completed.** Track pending Queue writes across
  UI updates, flush transfer-only on skips, avoid frontend zero-size uploads/
  software surfaces, retain benchmark skip errors. No guest/audio change.
  Native58/46 tests,1200zero-size batches/492MB logical bytes with stable RSS,
  GUI and actual archive GUI pass. Current exports333Wine/334Linux,14ignored;
  privacy/doc/source/PE/GLIBC2.35 checks pass. Initial pre-guard builds superseded.
  [Record](../changes/2026-10-21-flush-skipped-presentation-uploads.json).
  Artifacts:`skipped-uploads/`, `skipped-uploads-{gui,exports,export-gui}/`.

- **PERF-03-FRACTIONAL-UI-TEXELS — retained/completed.** Cached CPU-defined
  RG32Uint sampling maps and conservative bounds/scissors preserve original
  zoom/state. Radeon32-case resize/zoom/DPI and all fractional comparisons match
  CPU; replay3530/state/counts, worldmap, GUI and Wayland pass. Table-only trial
  +4.1681% rejected; bounds candidate five pairs -5.5309% (8.229995->7.774799ms),
  lower p95/p99. Native unpaced llvmpipe/CPU-profile scope, not AI/native Windows.
  Previous Docker332Wine/333Linux,14optionalignored and actual archive GUI passed.
  [Record](../changes/2026-10-21-align-native-ui-sampling.json).
  Artifacts: `ui-coordinates-{evaluation,exports,gui,wayland,export-gui}/`.

- **PERF-05-SHARED-WORLD-UPLOADS — retained/completed.** Same-call immutable
  source extent/dimensions share upload; divergent sources split before writes.
  Five pairs -0.3929% work, -40.37% UI payload, exact independent-GPU equality.
  Previous exports329 Wine/330 Linux and GUI/Wayland pass. Fractional CPU gap
  subsequently closed above. [Record](../changes/2026-10-21-evaluate-shared-world-uploads.json).

- **PERF-02-AUDIO-CALLBACK-DIAGNOSTICS — done for faithful/current scope.**
  Bounded context-separated observer and callback parity/overhead tests exist.
  Final Linux p99 total 0.486576 ms, mutex 0.000972 ms; no over-period callbacks
  or producer drops; transition-inclusive holds are not isolated AI underruns.
  [Record](../changes/2026-10-21-profile-realtime-audio-callbacks.json).
- **PERF-05-NATIVE-UI-INPUT-CACHE — retained; PERF-06 complete.** Full input
  keys skip packing; five pairs -1.41%; GUI/style/restore/exports checked.
  [Record](../changes/2026-10-21-evaluate-ui-packing-input-cache.json).
- **PERF-05-LUT-FPS / PERF-06-FIFO-PACING — done.** Revisions/LUT content
  comparison, FIFO capacity and startup pacing retained.
  [Record](../changes/2026-10-20-reuse-desktop-uploads-and-audio-buffers.json).
- **PERF-02-GPU-EXECUTION-TIMESTAMPS — done for implementation/current coverage.**
  Three bounded asynchronous readbacks, no wait; physical Radeon/GUI checks pass.
  [Record](../changes/2026-10-20-measure-deferred-gpu-render-pass-time.json).
- **PERF-02-GUI-CONTROLLED-OVERHEAD — done for unpaced workload.** Benchmark
  command, 3530-frame verification and five off/CPU/GPU triplets exist; reuse.
  [Record](../changes/2026-10-20-benchmark-bounded-gui-replays.json).
- **PERF-02-COARSE-PROFILE / PERF-02-PPU-STAGES — done for native Linux.**
  CPU/frontend and detailed PPU attribution already implemented and calibrated.
  [Record](../changes/2026-10-20-attribute-cpu-scanline-stages.json).
- **PERF-01-REPLAY-TOOLS — done.** Native/Wine snapshot/input/state tools exist.
  [Record](../changes/2026-10-20-resume-headless-benchmarks-from-snapshots.json).
- **PERF-01-WORLDMAP — done.** Visually classified fixture and five-pair baseline.
  [Record](../changes/2026-10-20-evaluate-ui-front-layer-selection.json).
- **PERF-01-ENEMY-REPLAY / PERF-01-ENEMY-BASELINE — done.** Observed enemy action,
  3530-frame recording, five baseline pairs and stage profiles are complete.
  [Record](../changes/2026-10-20-record-observed-enemy-activation-replay.json).
- **PERF-08-UI-ROW-INVARIANTS — retained (-2.176% at that boundary).**
  [Record](../changes/2026-10-20-cache-ui-window-row-invariants.json).
- **PERF-08-UI-PREPARED-BG-ROWS — retained (-4.144% at that boundary).**
  [Record](../changes/2026-10-20-share-native-ui-background-rows.json).
- **PERF-08-UI-EMPTY-ROWS — retained (-21.503% at that boundary).**
  [Record](../changes/2026-10-20-skip-empty-ui-source-rows.json).
- **PERF-01-HIGH-ZOOM-WORKLOAD — characterized; no parallel policy justified.**
  [Record](../changes/2026-10-20-characterize-high-zoom-replay.json).
- **PERF-03-RESTORE-VIEW-STRIDE — fixed.** Native paused preview/Undo never
  interprets old scratch stride; cache clearing and current exports pass.
  [Record](../changes/2026-10-20-restore-native-preview-across-zoom.json).
- **PERF-03-CURRENT-DESKTOP-BUILDS — done for stated container/Wine scope.**
  Old archives are superseded by current validated exports above.
  [Record](../changes/2026-10-20-validate-optimized-desktop-exports.json).
- **PERF-04-RECOVERY-IMPLEMENTATION — done; wider lifecycle matrix open.**
  [Record](../changes/2026-10-20-recover-transient-desktop-surfaces.json).
- **PERF-04-XVFB-GL-INITIAL-SURFACE — fixed/current coverage done.** Balanced
  configure scopes/fallback, valid resize/VSync and native close checked.
  [Record](../changes/2026-10-21-handle-surface-configuration-errors.json).
- **DOC-README-FEATURES — done.** Seven concise feature lines, layout preserved.
  [Record](../changes/2026-10-20-summarize-desktop-features-in-readme.json).
- **WORKFLOW-RESUME / WORKFLOW-RESUME-CHECKPOINT — done.** Latest request and
  selected ID/outcome must be reconciled before acting on old summaries.
  [Record](../changes/2026-10-21-require-resume-selection-checkpoint.json).

## Rejected units — keep the decisions

- **PERF-07-IMMUTABLE-ROM-SEEDS — rejected.** Existing18-seed translation
  fails full state, initially audio and after event fix CPU/clock at212. Source
  library pair retained; no timing or acceptance. Follow the new boundary
  criterion rather than repeating this trial unchanged.
  [Original trial](../changes/2026-10-21-evaluate-immutable-rom-entry-seeds.json),
  [first diagnosis](../changes/2026-10-21-isolate-rom-coverage-audio-divergence.json),
  [corrected event ordering](../changes/2026-10-21-order-timer-audio-events.json).

- **PERF-10-POST-WAIT-COMPOSE — rejected/restored; no validation pending.**
  Five alternating realtime pairs show worse p99 in four of five; exact accepted
  source/runtime restored, portable exports unchanged. Do not rerun the trial
  because its former next action survived in an older handoff or summary.
  [Record](../changes/2026-10-21-evaluate-composition-before-deadline.json).
  Artifacts: `target/desktop-perf/pre-wait-compose{,-gui}/`; jobs terminal.

- **PERF-08-COMPACT-UI-LAYER-ID — rejected/restored.** Sub-percent GUI gain
  inconsistent across pairs, contrary maxima; exact baseline source/bin restored.
  [Record](../changes/2026-10-21-evaluate-compact-ui-layer-ids.json).

- **PERF-05-NATIVE-UI-REVISIONS — rejected and restored.** Output-byte double
  buffers reduced payload but worsened total work +0.131%; packing-input cache
  is a different accepted design. Do not reactivate output-byte comparison.
  [Record](../changes/2026-10-21-evaluate-native-ui-upload-revisions.json).
- **PERF-06-ROW-BOUNDS — rejected for production.** Synthetic benefit did not
  survive end-to-end replay; experiment remains test-only.
  [Workflow](PERFORMANCE_WORKFLOW.md#panel-bound-scan-diagnostic).
- **PERF-08-FRONT-TWO — rejected, standard sorting restored.** No sufficient
  overall benefit; an open PERF-08 parent does not reopen this experiment.
  [Record](../changes/2026-10-20-evaluate-ui-front-layer-selection.json).

## Artifact locators and remaining scope

All paths below are local/ignored; missing artifacts do not reverse closed tasks.
Read tracked results first and restore only what the next new criterion needs.

- Current completed unit artifacts are listed above. Earlier shared-world
  verification/timing/world-map/resource manifests and GUI/export evidence remain
  in `target/desktop-perf/shared-world-{baseline,evaluation,gui,wayland,export-gui,exports}/`.
  Do not rerun these finished checks simply because they are no longer at the top.
- Common fixtures: `ai-turn-replay/{start.ffstate,input.input}` (3530 frames),
  `user-fixture/worldmap.ffstate`, `translation.so` under target/desktop-perf/.
  Never commit/package fixture bytes or regenerate their recording to resume.
- Reference: `gui-controlled/{verification.json,off-verify.log}`. Its FNV frame/
  OAM transcript differs in format from headless SHA logs; compare like with like.
- Earlier evidence: `audio-callback-{gui,export-gui,exports}/`,
  `ui-input-cache-evaluation/`, `surface-config-exports/`, `gui-controlled/`.
- Use `surface-config-vsync-gui/` for actual ON/OFF. Earlier valid-gui Return-only
  captures both show OFF. Old high-zoom-gui Undo capture shows a fixed defect;
  use restore/current-export captures as passing evidence.
- Native Windows GUI/device/GPU, physical controllers, full fault/lifecycle/
  monitor matrix, interactive scene/warm-up gating, enhanced starvation/backend
  XRUNs and original reported slow-turn attribution remain separate criteria.
  Callback-free unpaced runs do not establish realtime overhead. See
  [PERFORMANCE_TODO.md](PERFORMANCE_TODO.md) for the full remaining objective.
