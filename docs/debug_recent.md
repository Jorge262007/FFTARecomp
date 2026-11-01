<a id="registro-degli-eventi-recenti--dbg-19"></a>

# Recent event log — DBG-19

DBG2-08 connects host events to the [shared schema](debug_events.md): identity,
host ordering, correlation and coverage map shared by UI and recent events.
Original `sequence`/`host_ms` fields remain local to the collector.

In the paused [isolated copy](debug_sessions.md), choose **Debug > Start recent
events**. The log retains a rolling window; older events are evicted while the
game continues. **Freeze / save recent events** freezes and saves context in
Data/config/recent/&lt;run&gt;/index.html and recent.json. The report's fixed
interface, counts and empty-state explanations use English; observed payloads
retain their source text.

By default, a shared menu/CLI command error or a fatal error observed by the
frontend automatically freezes and saves the log. The report contains the
message and preceding context; it sends nothing. No new snapshots or continuous
whole-machine dumps are created.

## CLI

Open the copy with the runtime containing the tool; older copies retain their
own authenticated executable:

~~~powershell
.\recomp.exe debug-session open 'Data/debug-sessions/case' --paused
~~~

From a second terminal:

~~~powershell
.\recomp.exe debug-control recent-start 'Data/debug-sessions/case' --events 1024 --reads on --start 00000000 --end 02000000 --on-error on
.\recomp.exe debug-control step 'Data/debug-sessions/case' 10
.\recomp.exe debug-control recent-report 'Data/debug-sessions/case'
~~~

**recent-freeze SESSION** freezes without writing; it also works during free
running at a boundary handled by the frontend. Explicit Save requires pause
and no pending steps/probes/replays. It does not interrupt a guest frame midway.
**status.recent** exposes the window, counters, stop and any write error;
**recent_report** identifies the latest HTML. Status queries do not fill the log.

**--events** accepts **1–1,024**; **--reads on|off** selects instrumented reads;
**--start** inclusive and **--end** exclusive are hexadecimal **physical ROM
offsets**, within 02000000. They are not PC addresses. **--on-error off** retains
errors as events and keeps rolling; the default is on.

A new Start requires the preceding window to be saved. Duplicate Start is an
observed error and may therefore freeze/save the old window when on-error is
enabled. Timeline restore freezes with reason state-restored; origin, frozen
boundary and current state remain distinct. A separate replay requires Save
first. Normal closure without Save loses the memory buffer. An observed exit
error attempts Save; a process killed without callbacks cannot export its memory.

<a id="leggere-il-contesto"></a>

## Reading context

Events retain sequence, monotonic host time, guest frame and clock:

- **guest-input**: active-low latch actually used before the frame. The report
  also retains the last observed input; frames report keys even after the
  change event was evicted. Raw device polls belong to the
  [UI inspector](debug_ui.md).
- **recognized-ui-transition**: readonly frontend-signature change with recognized
  selector, bounded targets, dialogue/panel signals and name input. Includes
  selection changes. Coverage is partial; a disabled or unmatched recognizer
  remains unknown, not proof that a scene is absent. No global scene/unit IDs
  are invented.
- **rom-read-batch**: up to **16 selected unique keys per frame** from the existing
  collector, with actual Typed, Raw or Unclassified service, reader, address/
  offset/width and interpreter scope when available. Keys are aggregated and
  sorted, not the temporal order of individual reads. Fetches excluded;
  HLE/DMA scope may be inherited.
- **frame**: input, boundaries/completion, selected backend and observed
  native/missing/system/hook counts. Interpreter backend performs no native
  lookup; only native/missing prove table selection. Native-block internals
  and loaded-image identity remain out of scope.
- **control-request/result**, **error**, **freeze**: shared commands, result,
  bounded message, PC/keys and stop reason. Unconnected error channels or a
  crash without callbacks remain unobserved.

Reads require a rom-data-read-audit build. In an ordinary build,
feature-unavailable is distinct from disabled, observer collision and zero
requests in the window/range. When another collector already owns the observer,
the game continues with explicit unavailability. Groups lost to caps are counted;
they do not prove global non-use of resources or functions.

In the parameter form, **Recent** with `--reads on` is refused during validation
if the runtime lacks `rom-data-read-audit`; `--reads off` remains valid in both
builds. The form test checks these cases separately without expecting an ordinary
build to have optional hooks. Focused results are in the
[test correction record](changes/2026-10-30-correct-recent-form-feature-expectations.json).

<a id="limiti-identità-e-conservazione"></a>

## Limits, identity and retention

The buffer retains at most **1,024 events / 256 KiB of serialized events**,
with **8 KiB per event**. An oversized event is dropped and counted; it does not
evict valid context. UI signature is limited to 2 KiB, origin to 128 KiB; error
messages retain at most 1,024 characters with explicit truncation. Even with
one slot, the error that froze the window remains.

Unlike prefix collectors, there is no duration limit: the log keeps rolling
beyond 30 seconds. First/last sequence, host ms and frame/clock define the actual
window. Attempt/eviction/drop and read totals belong to the whole collection,
not just retained events. After Freeze, the suffix is unobserved, not zero.

Start and Save identify complete state by hash only at explicit boundaries,
plus ROM/BIOS, runtime/build, session manifest/pins, actual dataset, input and AV.
No new guest property, ABI or performance measurement. The diagnostic report
does not certify new scenes, resources or native parity.

At most **eight directories / 2 MiB JSON per report**, including incomplete
attempts; no automatic overwrite or cleanup. Failed Save retains the frozen
buffer and reason. Saving an already published window returns the same path.
Paths/metadata are private and Git-ignored; review before sharing. The
[homepage](report_home.md) finds them after regeneration. The
[artifact manager](debug_artifacts.md) preserves reports and shows their limits.

Actual checks and platform limits are in the
[DBG-19 record](changes/2026-10-29-add-recent-event-ring.json).

On October 30, `unused_mut` warnings were removed from output-directory checks
while preserving symlink and Windows reparse-point refusal. Changed blocks
compile without warnings for Linux and Windows; this is a focused compiler
check, not new GUI/report validation. See the
[fix record](changes/2026-10-30-remove-report-link-flag-warnings.json).

<a id="verifica-del-29-ottobre-2026"></a>

## October 29, 2026 validation

Windows GNU release builds locked/offline, with/without audit, without warnings.
**81 distinct Wine tests passed**: 79 feature tests (one million rolling events,
both caps, oversize, one-slot error, failed Save, actual services/collision and
dispatch plus menu/control/frontend), one real observer and one default
unavailable-feature test. Two initial tests searched for the batch at the first
position occupied by input; only tests were corrected. Attempts/results remain
retained. The 11 Python tests affected by the limit summary passed.

The new observer uses the retained DS20 seed: the same two A/release frames
with/without observation have identical complete state at every boundary up to
frame 22221/clock6241706243. It does not repeat earlier ROM/native acceptance.
Feature GUI uses 25 CLI invocations: after 26 frames, an eight-event window has
60 evictions and retains input/UI signature/frame/reads plus controlled error
**Start a code trace first**. Automatic Save does not change guest state. Menu
Start/Save remains active beyond 31 seconds paused; Freeze during free running,
reads off, idempotency and eight-directory cap checked. Default GUI uses five
invocations and publishes feature-unavailable without inventing zero reads.

The default runtime had already exited with 0 when the old closure helper
referenced an undefined log variable: ready inactive, lease released, response/
report/integrity checked by subsequent reads without relaunch. Offline Chrome
desktop/narrow checks five views, filters/search/reset, error and unavailability;
screenshots inspected. Homepage had 67 indexes/5 recent reports at that check,
without reaccepting other reports.

The final verifier authenticates 19 source overlays, two candidate runtimes and
five HTML/JSON pairs with byte/sequence accounting. Six normal inputs/runtimes
retain hash/mtime; helpers closed. Only five exited browser profiles retired;
reports, inputs, source/preimages, attempts, logs and candidates retained.
No disposable managed/native fixture: the ROM completion helper does not apply.
Local evidence: **target/debug-tools-recent/**. Native Windows, native Linux GUI,
physical controllers and actual native observation were not verified. No normal
runtime replacement, performance measurement, publishing or Android/Godot change.
