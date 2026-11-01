---
name: ffta-windows-validation
description: Build, test, benchmark and package FFTARecomp for Windows, distinguishing native Windows results from Wine and simulated controller tests.
---

# Windows validation

Read the short TASK_HANDOFF including its final **Final handover** section, Git
status, and only relevant PROJECT_STATE/feature sections. Never preload the
archive, full CHANGELOG/PROJECT_STATE or unrelated large TODOs.
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
Archive accepted/closed outcomes with evidence and limits; partial tasks keep
their original IDs and missing gates in the 10–20 KiB operational handoff.
Keep the final handover last and use MEMORY_WORKFLOW's shared-lock/preimage
publication protocol. Pause/interruption does not select new validation or ROM
work. Inspect job handles, output and ownership before restarting; never stop
another agent's jobs. Sandbox ps alone does not prove host inactivity.

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

Keep validation output bounded using
[local artifact retention](../../../docs/development/LOCAL_ARTIFACTS.md).
After the managed/native unit passes and its preservation checks and results
are recorded, run `python3 scripts/local_artifacts.py complete target/<unit> --apply`.
The helper retires disposable interrupted/moved fixture copies, retaining the
accepted offline dataset, source preimages and result/log evidence. Do not run
it on a failed/live unit. Retire superseded unit-local executables explicitly,
keeping the canonical runtime and current helpers. Guard actual inputs of the
new check instead of rehashing every historical mutable output; cleanup receipts
record availability and do not invalidate completed proofs.

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
`scripts/windows/build_docker.ps1` (Windows host) or `scripts/linux/build_windows_docker.sh`
(Bash). The builder is Linux/MinGW and its tests run through Wine; do not call
these native Windows validation. Verify PE resources and imported DLLs, build
TinyCC's helper with the host compiler and its library with the target compiler,
and keep Windows target caches separate from Linux target caches.
