<a id="menu-testo-e-input--dbg-13"></a>

# Menu, text and input — DBG-13

DBG2-08 connects host events to the [shared schema](debug_events.md): identity,
host ordering, correlation and coverage map shared by UI and recent events.
Original `sequence`/`host_ms` fields remain local to the collector.

In the [isolated copy](debug_sessions.md), pause and use **Debug > Start UI /
input trace**, reproduce the case, then **Save UI / input report**. The CLI
shares the same actions:

```powershell
.\recomp.exe debug-control ui-start 'target\case'
<a id="usare-tastieragamepad-o-i-passi-dbg-10-per-riprodurre-il-caso"></a>

# Use keyboard/gamepad or DBG-10 steps to reproduce the case.
.\recomp.exe debug-control ui-report 'target\case'
```

`status.ui_trace` reports active collection, count, stop reason and relative
path of the latest report. Open `Data/config/ui/<run>/index.html`. The page works
offline, with menu PNG, focus, text metrics and a filterable sequence. Its fixed
interface and status labels use English; observed text and payloads retain
their source content. The report stays private and appears on the
[local homepage](report_home.md) after regenerating the list.

`debug-control status` also exposes `status.host_menu`: current page, row,
binding and dropdown. Read this focus before automating navigation: a connected
controller may open the Gamepad tab directly. An open dropdown consumes arrows
until selection/closure; those arrows do not move the underlying panel's rows.

<a id="quale-passaggio-ha-consumato-il-comando"></a>

## Which stage consumed the command

The sequence distinguishes:

- `keyboard`: press/release received by Winit, physical code and autorepeat;
- `controller-poll`: sampled XInput tokens before the capture guard;
- `mapped-poll`: held keys, active-low GBA mask, host navigation,
  `rising_confirm`, binding capture, shortcuts and suppression mask;
- `host-menu-dispatch`: before/after focus, input and actual returned action;
- `menu-consumed-by-remapping`, `menu-debounce-refused`, `shortcut-consumed`:
  explicit reason that command does not continue along the same path;
- `bindings-changed` / `pad-bindings-changed`: configuration after remapping;
- `guest-input-latch` and `guest-frame`: mask delivered to the guest and actual
  completed frame, with controlled-step/host source and recorder state;
- `control-request` and `host-action`: CLI/shortcut requests and frontend actions.

Each event has a sequence number, relative **host** time, **guest** frame and
clock. Duplicate down-states are deduplicated per poll; physical keyboard repeats
remain explicit. Absence of an edge is not a second dispatch. A remapped mask
is not automatically guest input: the paused menu executes no frames, and an
F7 step uses the neutral mask declared by DBG-10.

Collection does not change existing input or filters. Presses are observed at
current points without a new injection channel. DBG2-09 adds Winit button/wheel
tokens and semantic menu changes; raw Gilrs/DirectInput transitions remain
outside the hooks, while the selected backend, aggregate remapped result and
effective changes are observable. Poll timing does not establish hardware latency.

<a id="testo-e-rettangoli"></a>

## Text and rectangles

The report redraws the host menu into a temporary buffer using the existing
font/pixel path. A thread-local observer collects at most 512 calls: text
(256-character prefix), font/size, origin, maximum width, ascent/descent,
full/prefix advance and outline bounding box of the actual selected glyphs.
Measurement stops at 4,096 characters and reports the limit.

`width_stopped` identifies the original `break` when the next advance exceeds
available width. `canvas_clipped` identifies outlines outside the canvas:
these are separate checks. The bounding box covers outlines, not nonzero-alpha
pixels. The page overlays the box and available area on the PNG; it does not
change fonts, wrapping, ellipses or menu geometry. Host focus reports page, row,
binding being captured, panel and selected rectangle with current scroll.
An open dropdown includes option, index, scroll and geometry from the same
helper used for drawing and hit testing.

For guest UI, only selectors already recognized by existing helpers,
dialogue/panel flags, buffer boxes and frontend layout/clipping are declared.
Other menus, focus and guest glyph metrics remain **unknown**. These measurements
do not prove ROM provenance, consumed typed bytes or framebuffer winner.

<a id="limiti-registrazione-e-conservazione"></a>

## Limits, recording and retention

Explicit collection, normally disabled. It stops at **30 seconds, 1,024 events
or 600 frames**, retaining reason and prefix. Start/Save require a paused copy
without an active step/search/replay budget. Collection can observe steps, play
and a [DBG-04 recording](debug_recordings.md), which must be checked with its
own command: a host trace alone is not a complete replay.

A timeline restore closes the prefix with `state-restored`; the report may then
show current post-restore state without attributing it to the preceding events'
endpoint. After an automatic stop, save the report before starting a new trace.
Events are not written continuously to disk.

Maximum **eight directories** per copy, including failed captures. The report
limits canvas to 1600×1000 and JSON to 2 MiB. Complete state is checked before/
after redraw; ROM/runtime and origin/current are identified. `ui.json` is
published last; no overwrite, pruning or additional snapshot from the collector.
Errors retain the trace and any resources for diagnosis. Status's latest path
still identifies a previous capture when a new command fails: always read the
request outcome and identity.

<a id="risultati-verificati-il-28-ottobre-2026"></a>

## Results verified on October 28, 2026

Locked/offline Windows builds with/without audit, both without warnings;
**69 distinct Wine tests**: 2 collector, 7 control, 12 frontend, 47 initial menu
and one new dropdown check. After revisions, only the 2 affected report tests
were repeated (twice): 73 total executions. The font observer leaves every
pixel identical; removing only hooks recovers the original input-mapping,
frame-runner and canvas bodies, plus the dropdown helper.

Three cases make the stages readable:

- Keyboard with Enter remapped to A: 14 autorepeat events produce one
  `host-menu-dispatch`, opening the Extended saves dropdown. Subsequent polls
  show held confirm and `rising_confirm=false`. F7 delivers neutral `3ff`;
  subsequent steps deliver `3fe/3ff` as requested. This prefix is accepted
  observation; its first recording, interrupted by the test, remains incomplete
  and was not certified as replay.
- Separate copy with completed recording: two CLI A/release frames, explicitly
  stopped and followed by clean closure, pass DBG-04's full verifier. Endpoint
  SHA256 `dd841ef6dba845974625e7e5ae5839f86069000066f876a32d0a86bf6fcb0b81`.
- **Simulated** XInput, South held for 1,200 ms in the A binding: one North→South
  update, one keyboard confirmation to open capture, no second confirmation or
  guest frame. Focus/capture/mapping and raw tokens distinguish binding from
  navigation. Existing-trace, capacity, active-execution and closed-copy refusals
  pass.

The report also measures the long diagnostic title: **545.89px** advance,
**472px** available, **64/75 characters** selected, `width_stopped=true` and
`canvas_clipped=false`. The cause is the font-call limit. The Extended saves
dropdown has bbox `[266,202,320,76]` on an 852×480 canvas; the current option
and arrows concern that popup before the underlying menu.

Six reports are retained. Chrome desktop 1400px/mobile 620px checks focus,
text/bounding-box selection, images, event filtering and absence of overflow;
screenshots inspected. The refreshed homepage recognizes all six UI reports
among 29 indexes. All 202 selected originals and both normal runtimes remain
unchanged.

Test attempts also revealed navigation errors in the harness: arrows in a
still-open dropdown, checking while the session held its lease, Right after
automatic Gamepad selection and mistaking the Open menu row for A. They are
retained with logs/preimages; accepted prefixes were not repeated. Two copies
exit cleanly; two older diagnostic copies were interrupted by test cleanup and
have no exit/replay acceptance. The final check uses page, labels and geometry
returned by `status`.

Native Windows/Linux, physical controllers, other scenes/native backends,
guest glyphs and raw Gilrs/DirectInput/mouse transitions are not certified.
The PNG is a diagnostic CPU redraw of the menu over a preview, not a monitor/GPU
capture. No new ROM/DS/native acceptance or performance measurement. Local
evidence: `target/debug-tools-ui/`; temporary Chrome profiles were removed,
retaining errors and inputs. [Record](changes/2026-10-28-add-bounded-menu-text-input-diagnostics.json).
