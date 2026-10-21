# Windows runtime performance, 2026-10-20

This change targets the desktop runtime, including Windows x86_64. The
separate `AndroidPort` tree is unchanged. Game rules, AI decisions, guest
instruction timing, rendering and the native-code ABI are unchanged.

## Changes

- Read `RECOMP_TRACE_IWRAM` once per emulated frame instead of once per
  native dispatch. Resolve the fallback tracing flag once per frame too.
- Reject unrelated instruction addresses before consulting sprite diagnostic
  configuration, both in native dispatch and in interpreter observations.
- Cache `RECOMP_TRACE_SDMA` like the other startup trace settings, and check
  the logging cadence before checking that flag. FIFO contents, DMA requests
  and audio samples follow the existing execution path.
- Update the optional native-observer integration test to retain and use
  TinyCC's loader for its relocatable output. The old test assumed every
  generated artifact was an OS-loadable DLL, which is no longer true.

No translation-cache revision bump is needed: generated game code, runtime
ABI and emulated state transitions have not changed.

## Validation

Windows GNU release binaries were cross-compiled from Linux and executed
under Wine. These are Windows-code-path checks, not measurements on a native
Windows installation or measurements of displayed FPS.

- `gba-core`: 159 tests passed, four require external fixtures and are ignored.
- Desktop runtime: 25 tests passed, one optional test ignored.
- Recompiler: three ordinary tests passed. Both optional TinyCC tests passed
  (callback ABI/large stack/cache round trip, compile-error cleanup).
- The optional native-observer test also passed with `FFTA_OAM_LOG=1`, checking
  CPU registers, CPSR, RAM, IO, VRAM, OAM and guest cycles against the interpreter.
- Linux core tests: 159 passed before and after the changes, using one test
  thread. The baseline parallel suite fails because tests share global display
  configuration; this patch does not change that test setup.

Local validation logs and artifacts are under ignored `target/windows-perf/`
and `target/windows-*-tests.log`. The runtime update is packaged under ignored
`dist/FFTARecomp-windows-performance.zip`; it contains no ROM, save, input
recording or generated game code.

## Measured result

European ROM (`8fab1e390cb9d5e695bfdb86e81246892e1aa8c0cb4014db8235ff76538ea632`),
Windows GNU release executables via Wine, TinyCC translation shared by both
runtimes, 16:9, 6,000 frames, first 120 excluded from timing:

- Original runtime: 16.029209, 16.433341, 16.043990 ms/frame.
- Optimized runtime: 4.568973, 4.092949, 3.948501 ms/frame.
- Median: 16.043990 to 4.092949 ms/frame, a 74.49% reduction in measured
  headless emulation time on this host. This is not a native Windows speedup
  claim; Wine's environment-query cost may amplify this particular gain.
- Separate captures match framebuffer and OAM hashes on all 6,000 frames.
  Every capture and timing run also agrees on final frame hash
  `de6e64380b3d2bab`, 35,519,344 native dispatches and 96,744,712 fallback steps.
  The fallback count excludes intentionally interpreted sprite/mouse hooks,
  as in the existing runtime; it is not a complete interpreter-work metric.

The tested and packaged `recomp.exe` SHA256 is
`3c4d8a3a6743400af9e346eb6ca5e8e707fecba0acb7e61bdf8751118777461d`.
The baseline executable SHA256 is
`3d949607c7d47888cb2f38f05cc8812dba08b1fc46f46d221416cf5b1916bccb`.
The package SHA256 is
`22e3a1f4c5918801f4aef1a60e733fbec6bb7ee1bef70cabd66dc5468a0c4413`.

## Reproduce a runtime comparison

Keep both runtime executables. Build the game's translation once and pass
the same library to both, so compiler output and coverage cannot change
between the measurements. Run from the repository root on Windows:

```powershell
.\baseline\recomp.exe build "C:\Games\ffta.gba" --ram --gamedb gamedb.sqlite
python scripts/benchmark_windows_runtime.py --baseline baseline/recomp.exe --candidate optimized/recomp.exe --rom "C:\Games\ffta.gba" --library out/ffta.dll --demo --frames 6000 --runs 3 --out target/windows-perf/comparison
```

On Linux, add `--wine wine64 --wine-prefix target/windows-perf/wine` and use
the Windows builds. Each executable needs the bundled `tcc/libtcc.dll`
beside it, or `FFTA_TCC_LIBRARY` pointing to that library.

`--demo` generates the same deterministic Start/A taps as the runtime's
verification command. To reproduce a specific enemy turn, use a recording
with `--input file.input` and optionally a read-only save copy with `--sav`.
The script separates per-frame framebuffer/OAM verification from timing,
alternates timing-run order, rejects incomplete or divergent runs, and writes
the individual logs plus `summary.json`. For a decoded-resource comparison,
`--baseline-rom-data` and `--candidate-rom-data` enable that runtime option
independently in verification, timing and profile runs, including when both
variants use the same executable path. The summary records both switches.
Use a corrected legacy binary without resource routing as the reference when
measuring the complete migration cost; see the
[resource validation protocol](development/ROM_DATA_VALIDATION.md).
Its default 120-frame warm-up uses
the existing `FFTA_BENCH_FROM` mode, which restores sprite expansion at the
start of the measured interval.

The demo reaches the introductory snow scene on the supplied European ROM;
it is not a targeted AI-decision replay. A native Windows run of a recorded
enemy turn is still needed to quantify the improvement in that situation.
