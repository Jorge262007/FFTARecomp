<a id="codice-label-e-fallback--dbg-15"></a>

# Code, labels and fallback — DBG-15

In a paused isolated copy, **Debug > Start code trace** arms collection for one
frame and at most 512 events. Step through or replay the short sequence, pause
again and choose **Save code report**. Open `Data/config/code/<run>/index.html`:
the sequence connects PC/mode, current ROM or RAM backing, disassembly at entry,
declared labels and actual runtime choice. `code.json` contains the same data.
No snapshots are created and no performance measurements are enabled.
The HTML interface uses English; captured labels and diagnostic payloads retain
their source text.

<a id="cli-equivalente"></a>

## Equivalent CLI

Open the [isolated session](debug_sessions.md) with the runtime containing the
tool. Older copies keep their own authenticated executable.

```powershell
.\recomp.exe debug-session open 'target\code-case' --paused
```

From a second terminal:

```powershell
.\recomp.exe debug-control code-start 'target\code-case' --frames 1 --events 128 --start 08000000 --end 0a000000 --mode thumb --kind all
.\recomp.exe debug-control step 'target\code-case' 1
.\recomp.exe debug-control code-report 'target\code-case'
```

Start and Save require pause, without probes, pending steps or separate replays.
`status.code_trace` shows filters, counts and stop reason; `code_report` identifies
the latest HTML. An unsaved collection prevents a second Start or a separate
replay. Load/Undo/Restart freeze the prefix. The report's current state may
therefore belong to the restored timeline, identified separately. Closing
without Save loses the in-memory collection.

Parameters: **1–60 frames**, **1–512 events**, 30 host seconds; `--start` inclusive
and `--end` exclusive are hexadecimal addresses. Mode `arm`, `thumb` or `all`;
kind `all`, `native`, `missing`, `system`, `hook` or `interpreter`. Defaults match
the menu. Capture filters exclude events before dumping; browser search/filter
acts only on the retained prefix.

<a id="leggere-il-passaggio"></a>

## Reading the sequence

- `native`: the table selected a compiled entry for the exact PC/mode. The
  report identifies the key without inventing the block body or length. It does
  not observe individual instructions inside the native function.
- `missing`: no compiled entry for that key; the runner performs the normal
  interpreter step. ARM and Thumb are different keys.
- `system`: Halt/wakeup, pending IRQ or HLE IRQ-return step. It may execute no
  instruction at the displayed PC; disassembly is omitted.
- `hook`: the runner selected the interpreter for the normal battle-mouse or
  sprite hook before looking up a block. This is not a missing table entry.
- `interpreter`: interpreter backend selected, without native lookup. A static
  label does not change that choice.

Events precede dispatch/step; the report retains the endpoint at the observed
boundary and limit reason. Bytes are read from immutable backing or current RAM,
with physical offsets and mirror distinctions, without simulating fetches or
reading I/O protocols. Disassembly covers one ARM word or Thumb halfword at
entry, without reconstructing a function or merging Thumb BL. RAM code may
change over time; the dump retains only the sampled point.

<a id="label-per-la-rom-scelta"></a>

## Labels for the selected ROM

Place an explicit copy of your [labels](labels.md) file in the session root as
`debug-labels.toml`. Text v1 and TOML v2 are recognized by content. Start reads
at most 1 MiB, authenticates the ROM and pins the hash of the same parsed bytes.
A different ROM refuses Start with a readable error and CLI response; neither
the source file nor the normal accumulator is changed. Without the file, labels
are reported as unselected.

Names and ranges are **static declarations**. Correlation with an observed PC
does not prove function extent, code completeness or a new ROM resource. The
report shows at most 64 static names and four matches per event, with
128-character prefixes and explicit omissions. ROM, IWRAM and reserved entries
remain separate counts. No extracted-code percentage is calculated from entry
or dispatch counts.

<a id="identità-e-conservazione"></a>

## Identity and retention

The report pins runtime/build, ROM/BIOS, seed and other session-manifest pins,
initial/current state, input/AV and actual dataset when present. For native code
it retains the loader's path, table size and hash of the file read at trace
startup. That read **does not retroactively authenticate the code image already
loaded in memory**; the report alone does not prove absence of external file
changes between load and Start. Host pointers and native-function lengths remain
unknown.

At most eight directories, including incomplete attempts, and 2 MiB JSON. No
automatic overwrite or deletion. A publication error retains the trace for
another Save. Data, code bytes and paths are private and Git-ignored. The
[homepage](report_home.md) finds them after regeneration.

Observation is normally disabled. The native runner with an empty observer
retains its usual path; active collection adds a readonly read before dispatch.
The observed interpreter loop preserves the same `Machine::step` sequence,
without new guest fields or state-format/ABI changes. Limits stop collection
without interrupting a normal frame midway. Events after stopping are unknown,
not zero.

Actual checks and limits are in the
[DBG-15 record](changes/2026-10-29-add-bounded-code-dispatch-diagnostics.json).
Comparing an observer with the same native runtime neither reopens nor certifies
interpreter/native parity or earlier ROM/DS acceptance.

<a id="verifiche-del-29-ottobre-2026"></a>

## October 29, 2026 validation

Windows feature/default builds locked and offline, without warnings. **74 Wine
tests**: 73 ordinary (collector, controller, menu, frontend and wrapper cost) and
one optional check with explicit local ROM/seed/library. The synthetic test
reconstructs six compiled and six missing entries with identical state. The
existing-library check compares one observed/unobserved frame without a writer
or new snapshot: complete state/clock and 6,547 blocks / 14,991 steps are identical.
The bounded prefix retains 41 native, 470 missing and one IRQ; events 17/18:
`08145c10 Thumb` → `0801bde8 Thumb`, with explicit fallback reason. This checks
the new observer and does not certify that library against the interpreter.

The default GUI copy checks menu/CLI Start/Save, IWRAM/ARM filter with authentic
label and 16 events, default 512-event prefix and a no-match region with 106,554
dispatches explicitly excluded. A two-frame A/release recording reaches the
exact full endpoint after Stop and closure; the copy exits with an inactive
server. Different-ROM label, invalid parameters, duplicate Start, eight-directory
cap, free running and closed-session refusals pass. The six actually selected
inputs/runtimes retain hashes/mtimes; no normal executable is replaced.

Chrome desktop and narrow layouts check PC/label search, dispatch filters,
details, empty prefix and absence of page overflow. The homepage includes
41 indexes, four code reports. Screenshots inspected; payloads and evidence stay
in `target/debug-tools-code/`. Native Windows/Linux, physical controllers,
other scenes and native GUI capture were not verified by these checks.
