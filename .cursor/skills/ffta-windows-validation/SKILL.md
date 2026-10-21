---
name: ffta-windows-validation
description: Build, test, benchmark and package FFTARecomp for Windows, distinguishing native Windows results from Wine and simulated controller tests.
---

# Windows validation

Read `docs/development/PROJECT_STATE.md` and the feature's own documentation.
Use release builds for performance. Inspect installed targets, compiler and
libraries before assuming a toolchain exists; `target/` paths are local
artifacts, not repository prerequisites.

Read `docs/development/TASK_HANDOFF.md` before scheduling validation. Reuse
existing results for the same scope and inputs; compaction, a lost session
handle or an unchecked parent TODO does not invalidate them. Before repeating
an expensive build/test/benchmark, identify the changed input, uncovered
criterion, explicit user request or verified failure that requires it, following
`docs/development/MEMORY_WORKFLOW.md`. Check surviving process/artifact state
before restarting an interrupted observation. A new implementation still needs
the checks affected by that implementation.

A summary mentioning an unfinished build is not a new build instruction.
Reconcile it with the latest user request and actual output before acting; a
workflow-only request does not require rebuilding unchanged applications.

Before mutating files or starting expensive work after resumption, record the
selection checkpoint in TASK_HANDOFF as defined by MEMORY_WORKFLOW. Check the
selected ID's existing outcome; completed/rejected work requires a recorded
reopening reason. Validate an already implemented request instead of doing it
again. Check linked terminal results before trusting an old next-action line;
close a completed/rejected selection before choosing another unit.

- Native build: `cargo build --locked --release -p recomp -p ffta-play`.
- Cross build: select `--target x86_64-pc-windows-gnu` and a compatible linker.
- Compile affected Windows tests, then run their executables on Windows or
  through Wine. Report which environment was actually used. Run core tests
  with `--test-threads=1` because display globals are shared.
- A GUI change needs a relevant menu/HUD check as well as compilation.
  Gamepad tests should identify the backend and whether input was simulated.

Use an isolated save, configuration and Wine prefix for interactive tests.
Wine may initialize APPDATA itself; inspect the effective path rather than
assuming an environment override isolated the config. Keep test controller
DLLs and screenshots under ignored `target/`, outside deliverables.

For performance comparisons, use the same translation, input, warm-up and
settings; separate hash capture from timing. Reuse
`scripts/benchmark_windows_runtime.py`. Wine timing is not native Windows
timing; introductory replay results are not an AI-turn benchmark.

TinyCC caches are relocatable ELF objects even on Windows. Use its loader,
not the OS DLL loader. Cache revision rules live beside `TRANSLATION_REV`.

Package an explicitly selected runtime, launcher, gamedb, TinyCC library,
source and notices. Verify archive integrity and executable hashes; exclude
ROMs, saves, recordings, generated game code and test doubles. Document
commands, environment, evidence and limitations with the project-memory skill.

For dependency-contained builds use `Dockerfile.windows` and
`scripts/build_docker.ps1` (Windows host) or `scripts/build_windows_docker.sh`
(Bash). The builder is Linux/MinGW and its tests run through Wine; do not call
these native Windows validation. Verify PE resources and imported DLLs, build
TinyCC's helper with the host compiler and its library with the target compiler,
and keep Windows target caches separate from Linux target caches.
