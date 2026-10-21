# Current development knowledge

This file stores durable facts and entry points. Chronological changes and their
validation belong in [CHANGELOG.md](../../CHANGELOG.md) and `docs/changes/`.
Read current source and manifests before relying on older build instructions.

Task selection after compaction or goal continuation starts with
[TASK_HANDOFF.md](TASK_HANDOFF.md): completed substeps, rejected experiments,
evidence and one next open work unit. An unchecked parent TODO does not reopen
completed children. See [repeat rules](MEMORY_WORKFLOW.md#resume-and-repeat-decisions)
before rebuilding, rerecording fixtures or rerunning an unchanged benchmark.

## Architecture and current scope

- The root Cargo workspace and `AndroidPort/` are separate trees. A root fix
  does not automatically update AndroidPort.
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
  [Inventory/contract](ROM_DATA_INVENTORY.md) documents other families' unknowns.
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
  are distinguished. Managed v1/e6-d5 extracts/verifies/prepares OBJ, BG metadata and base
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
  unknown. Next add bounded typed decoding before managed extraction/runtime.
  Exporting files alone is not completion; preserve guest behavior,
  native-cache/profile/save identity and measure costs against the memory baseline.

## Container builds

- `Dockerfile.linux` and `scripts/build_linux_docker.sh` build Linux amd64 using
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
  `scripts/build_docker.ps1` is the Windows-host entry point: default Windows,
  `-Platform linux` for Linux. Docker Desktop must use Linux containers.
  `scripts/build_windows_docker.sh` offers the Windows target from Bash.
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
  `scripts/package_ffta_windows.py` and Windows `build-all.ps1` create a portable
  ZIP by explicit file selection. Native Linux uses
  `scripts/package_ffta_linux.sh` and the shared
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
- [Save states](../save_states.md): pause menu > Save states, nine controller-
  operated slots with embedded previews and in-menu overwrite/error feedback.
  Storage: `SaveStates/<ROM SHA-256>/slot-1.ffstate` through `slot-9.ffstate`,
  relative to the selected cartridge save directory. A new session starts at
  slot 1; Left/Right wraps between all nine. Legacy raw states can be copied
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

## Native UI coordinate policy

Native UI GPU draws now use cached RG32Uint axis maps computed with the existing
CPU pixel-centre/floor/clip/clamp formulas. This avoids backend-dependent float
reassociation selecting adjacent texels at fractional world boundaries. Keys
include exact layout/clip/view bits, world dimensions and the device limit;
resource recreation invalidates maps. Conservative source bounds use the exact
transparent-pixel predicate and integer-map-derived scissors. Invisible layers
still claim shared world sources. No guest timing, snapshot or zoom/FOV change.

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

Follow [the memory workflow](MEMORY_WORKFLOW.md) and the local skills routed
from [AGENTS.md](../../AGENTS.md). Update this page when facts or decisions
change, rather than adding a second chronological log here.
