<a id="atlante-del-clan--menu-desktop-nativo"></a>

# Clan Atlas — native desktop menu

The menu uses the Rust canvas, tiny-skia and the already embedded Cinzel/Inter
fonts. The mineral-blue and brass frame, original geometric crest, left sidebar
and footers are independent of content scrolling. The canvas's technical
minimum is 480×360 physical pixels; below that size, it asks the user to enlarge
the window, with Escape available for recovery.

The sidebar has exactly eight sections per tab. Debug is initially hidden and
appears only after the unlock described below. **Game** contains Overview,
Saves, Fast Forward, Text windows, Audio, Video, Controls and System. **Debug**
contains Overview, Sessions and cases, Replays and comparisons, Execution and
memory, Traces and inspectors, Consoles and scripts, Cheats, Reports and archive.
Switching tabs preserves the section, selection and content scroll. Long
settings use two columns when at least 750 pixels of content width are available;
Debug lists have adjacent details, overlaid internally when content is narrower
than 470 pixels. The sidebar stays on the left. Fonts scale with the canvas from
HD to 4K. The DS catalog, content selector, virtual keyboard and tool parameters
use the same palette and rearrange controls within the available body. Previews
preserve original pixels; a missing preview is stated explicitly.

Overview offers Return to game, quick slot, Save/Load/Undo, volume and Fast
Forward. The selected slot persists when reopening the menu; selecting it does
not save. An occupied file requires the existing confirmation, initially on
Cancel. An empty file cannot be loaded. A state's date is its mtime;
**Last .sav modification** separately shows the actual cartridge save's mtime.
Dates are explicitly UTC. Missing files or unreadable dates are shown without
inventing dates.

Saves groups Save states, Extended saves (9/99 slots), numbered cartridge saves
and backups. Existing forms and handlers preserve previews, confirmations and
Undo. System groups language, restart, ROM selection and exit. Bindings and the
three input profiles remain in Controls.

The menu starts in **English**. **Game > System > Language** offers **English**
and **Italiano**, applied immediately on confirmation and persisted as
`menu.language = en|it` in `av.cfg`. This guide uses the English labels;
the Italian catalog provides their optional translations. Older configurations
without a language use English; ROM language remains independent. Details and
instructions for adding a single automatically registered JSON file:
[menu language](menu_localization.md).

**Game > Video > Menu transparency** adjusts backgrounds from 0% (opaque)
to 100% (transparent), in 10% steps. The initial value is **10%**. Text, borders
and previews remain opaque. The preference is saved as
`menu.transparency = 0.0..1.0` in `av.cfg`, including portable mode.
An older file without this key uses 10%; an explicitly saved value, including
0%, preserves the user's choice. The game visible beneath the menu stays paused;
every redraw starts from the frozen frame, without accumulating transparency
from the preceding draw.

<a id="sblocco-permanente-di-debug"></a>

## Permanently unlocking Debug

With the menu open, press the arrows or D-pad in separate presses:

**↑ ↑ ↓ ↓ ← ← → →**

The last direction reveals Debug and saves **`debug = true`** in `av.cfg`
(`Data/config/av.cfg` in portable mode). This is needed only once: closing the
menu, changing ROM or restarting preserves the unlock in that configuration.
Alternatively, add `debug = true` directly to the file and restart.
The initial value is `false`; manually setting it back to `false` hides the
screens again on restart without deleting Debug data or preferences.

Before unlocking, the Debug tab, planned Events/CMD controls, Debug references
and **Extended saves / Game saves** options in Saves are absent. Normal slots,
backups and game controls remain available. Navigation continues normally during
the sequence; an incorrect direction restarts from the still-valid prefix.
Closing/reopening, Confirm/Cancel, focus changes, Details, clicks and wheel input
interrupt the sequence. Binding capture, editors and dropdowns do not contribute.
The isolated copy's F6/F7/F8 commands also require the unlocked flag. CLI commands
and isolated-copy ownership checks retain their contracts: this is a menu
visibility preference.

<a id="comandi-e-fast-forward"></a>

## Controls and Fast Forward

Tab switches between sidebar and content; in existing selectors/editors, Tab
retains local navigation and Ctrl+Tab moves to the sidebar. Gamepad X/West changes
focus. D-pad/arrows move the cursor without changing options: **OK/Confirm** opens
the choice, directions highlight a value, a second **OK** applies it and **Cancel**
discards it. This also applies to sliders, slots, filters and Debug-form choices.
OK uses the device's shared command (for example South on a gamepad), rather than
only Enter. Clicking a row opens its choice; clicking an option confirms it,
while right-click cancels. Left from the content edge reaches the sidebar;
Right on a section returns to its content. The sidebar cursor has a solid brass
background, dark text and a `›` indicator, distinct from the active section alone;
it remains recognizable even at 100% transparency. In two-column layouts,
left/right first reaches the adjacent control. In Controls, Up from the first
binding reaches device tabs: directions highlight, OK selects. Shift+Tab or
Y/North switches the Debug view between tools and V2 references; two clickable
tabs are also available. In references, ↑/↓ scrolls the list and ←/→ scrolls
details; the wheel acts on the pointed area. Escape returns to the calling
context or resumes from Overview. Holding Escape cancels only once: repeat
events cannot close a dropdown and then resume the game.

**Details**, F1 or R3 opens a read-only view with full values, bindings, dates,
messages and metadata for the selected context. Text scrolls internally; returning
preserves selection and scroll. Values are a snapshot taken when opened. Binding
capture, editors, dropdowns and busy host operations have priority: Details is
disabled in those cases. Utilities defer to a configured menu-binding key when
it belongs to the already supported capture vocabulary (for example Tab).

With the menu open, L1/R1 changes volume by 10 points; L1+R1 toggles Mute without
changing volume. L2/R2 changes speed by 20 points; L2+R2 toggles Fast Forward
without changing its preference. An 80 ms window gives priority to nearly
simultaneous presses; a partial release does not repeat the chord. Binding
capture and text/choice editing in host forms remain exclusive. These shoulder
buttons remain dedicated shortcuts. Global controls do not consume buttons when
the menu is closed. Hints follow **the last input used**: keyboard or mouse shows
Arrows, Enter, Esc, Tab and F1; gamepad shows D-pad, A/B, X, RS CLICK and the
project's existing Xbox shoulder buttons. Debug tabs show Shift+Tab or Y;
Details shows F1 or RS CLICK. Connecting a pad or an already detected held press
does not supersede a later keyboard command. Labels do not change the profile
or game bindings. With a keyboard, adjust volume and Fast Forward by selecting
the entry and pressing Enter; hints do not advertise nonexistent keyboard
shortcuts. The left footer shows the current binding of the device in use,
including a newly captured binding. Escape always remains a recovery path.

Effective speed is 100–800%, in 20% steps. A rational accumulator produces whole
frames per tick: for example, 120% produces 1,1,1,1,2 frames over five ticks.
Each frame retains its own GBA clock and operations; the duration of an emulated
frame is unchanged. Disabling Fast Forward restores 100% while retaining the
preferred speed. The momentary binding still inverts the session latch.

`av.cfg` accepts `ff.percent = 120`. Legacy `ff.speed = 2 | 4 | 8` retains its
200/400/800% meaning and round-trip format. New percentages are saved with
`ff.percent`. Audio `silent`, `fast` and `normal` retain their existing semantics:
temporary silence, playback at the selected speed (including 1.2×/1.4×), or a
bounded backlog at normal speed. Even at 100%, the toggle retains an ON state
distinct from OFF: ON applies Fast Forward audio mode while executing one frame
per tick.

<a id="debug-e-inventario"></a>

## Debug and inventory

Real tools retain selectors, parameters, presets, preparation, validation and
isolated-copy protections. **Menu / CLI reference** reads the same 97-operation
registry as `recomp debug-operations`: ID, path, handler, command, parameters,
prerequisites, effects and limits. It is always read-only and does not interpret
displayed commands. CLI-only capabilities retain that label. Existing tool pages
remain the executable entry points; references have no fake Run buttons.

The V2 view contains exactly DBG2-01…28 and reflects its source checklist.
At the original menu-migration checkpoint, 01…09 were complete and 10…28 planned.
Events and CMD in the header were explicitly planned, without a simulated live
backend. Cheats retained a description of missing dependencies. Consult the
[current task handoff](development/TASK_HANDOFF.md) for later accepted work and
remaining criteria; this migration baseline does not reopen completed steps.

The [previous inventory](development/MENU_ATLAS_BASELINE.json) preserves contracts
before migration; the [new map](development/MENU_ATLAS_INVENTORY.json) maps all
190 row occurrences, 54 previous actions and 97 operations to current paths.
It includes subforms, dropdowns and confirmations; ToggleFastForward is the only
new frontend action. Operation IDs, parameters, safety contracts and CLI handlers
are unchanged.

Check assets and the map after a change with:

```sh
python3 scripts/menu_atlas_tasks.py --check
python3 scripts/menu_atlas_inventory.py --check
```

<a id="verifica-della-migrazione"></a>

## Migration validation

Windows GNU/Wine baseline: 29 input-config tests and 72 menu tests passed before
changes, on commit `945966c`; the specification cited earlier commit `999d0a6`.
The map therefore includes the already present DBG2-01…09 backends.

Windows GNU release validation performed **only through Wine** covered:

- 31 input-config tests and 86 menu tests; no pre-existing test removed.
- A separate explicit export test: 45 native views, nine contexts at 1280×720,
  1024×768, 854×480, 640×480 and 480×640. Declared synthetic graphics fixtures,
  distinct from real game data.
- 11 focused frontend tests: complete state and guest clock equivalent to
  sequential execution at 100/120/140/200/400/800%, fractional audio and history
  invalidation, normal audio queue, simulated XInput capture and operation registry.
- EUR GUI with mouse, keyboard and simulated XInput: read-only references,
  dropdown cancelled without writes, near-simultaneous chords and partial release,
  quick slot, overwrite initially on Cancel, Undo, 99 slots, cartridge/backup forms,
  remapping and dynamic bindings. Retained operations use previous handlers:
  migration does not indiscriminately rerun every CLI tool.
- OpenGL and Vulkan presentation through llvmpipe and actual GDI software fallback;
  held Escape does not resume the game. Separate 100% toggle test with
  silent/fast/normal audio and return to OFF, without duration or speed measurements.
- Details through F1/R3/mouse, Tab-binding priority, exclusive capture, cancellable
  editors and native selector, parameter and explicitly synthetic empty-catalog
  screens. Complete guest state/clock and original ROM/seed/.sav remain unchanged
  in navigation-only checks.
- Actual Win32 resize through SetWindowPos, with GetClientRect verified before
  captures: Overview at the five sizes above; selector, parameters and catalog
  at 854×480, 640×480 and 480×640. Initial images obtained by resizing only the X
  surface are diagnostic, not evidence of adaptive layout.

Detailed results are in the
[change record](changes/2026-10-30-implement-native-clan-atlas-menu.json).
Logs, authenticated executables and screenshots remain local in
`target/menu-clan-atlas/`; they are not distributable fixtures.
These original migration checks did not validate Linux, native Windows,
physical controllers or the USA ROM. No performance measurement belongs to this
migration. AndroidPort is a separate tree and was not changed by it.

<a id="file-e-avvio"></a>

## Files and startup

`clan_atlas.rs` computes geometry/navigation shared with hit testing;
`atlas_theme.rs` draws the frame and rows. `save_forms.rs` retains save forms,
`information.rs` full text and `shortcuts.rs` chord arbitration. Existing modules
handle catalog, content and parameters. `input-config::fast_forward` owns
percentages/configuration/accumulator; `recomp::play` applies actions, clock,
audio and actual metadata. The complete changed-file list is in the record's
`files` field; the before/after map is in the inventory linked above.

With the Rust Windows GNU target and MinGW linker configured:

```sh
cargo build --locked --release --target x86_64-pc-windows-gnu -p recomp -p ffta-play
```

Start `target/x86_64-pc-windows-gnu/release/ffta-play.exe` in the project's
prepared Windows/Wine environment, select your own EUR ROM and open the menu
with Escape or the configured combination (default Start+Back). For an isolated
copy, follow [Debug tools](debug_tools.md); do not replace an authenticated
session's executable to try a different build.

<a id="trasparenza-e-modifica-dopo-ok"></a>

## Transparency and editing after OK

Focused Windows GNU/Wine validation of the October 30 change included
32 input-config and 92 menu tests passing (one optional export ignored), plus
focused selector-refresh, frontend XInput-capture and tab-focus-to-mouse-click
cases. Previous tests were adapted to explicit confirmation while preserving
overwrite, Undo, profile and exclusive-capture checks.

The EUR GUI with simulated XInput verified sidebar/content navigation using
only the D-pad, volume and transparency unchanged before OK, cancellation,
backgrounds at 0/50/100%, Code parameter choices and reopening at persisted 50%.
Images were inspected. Complete state, frame, clock and guest input stayed
unchanged; original ROM and save hashes were preserved. A final isolated fix
for focus after clicking a tab was checked with only the affected Wine case,
without repeating accepted GUI checks.

Local evidence: `target/menu-transparency-navigation/` (build/test logs,
`gui2/accepted-prefix.json`, `gui3/acceptance.json`, images and runtime pins).
Two harness errors involving the pointer and label prefixes were retained;
successful prefixes were not repeated. No benchmark. Linux, native Windows,
physical controllers and USA were **unverified by this validation**.

<a id="default-e-suggerimenti-contestuali-31-ottobre-2026"></a>

## Defaults and contextual hints (October 31, 2026)

Four focused Windows GNU/Wine cases passed: `menu.transparency` defaults and
compatibility, hint source without binding/capture changes, highlighting after
Left even with transparent backgrounds, and the previous pixel case made
explicitly opaque. Test and production builds had no warnings. The EUR GUI with
simulated XInput verified keyboard/pad alternation, distinct sidebar focus,
unclipped labels at 854×480 and 640×480, keyboard use while a pad remained held,
and initial 10% without a file key. Captures were inspected. The copy's config
and bindings were identical; complete state, frame, clock, guest input and the
three private originals remained unchanged. Local evidence:
`target/menu-context-controls/`. Linux, real DirectInput/Gilrs, native Windows,
physical controllers and USA were unverified by these checks. No benchmark or
repeat of save/ROM validation.

<a id="verifica-dello-sblocco-31-ottobre-2026"></a>

## Unlock validation (October 31, 2026)

Windows GNU/Wine: one parser case and 97 menu cases passed, with one optional
export test ignored. Pre-existing Debug-path tests used an explicitly unlocked
fixture; three new cases used the actual locked constructor and checked
visibility/entry points, sequence/persistence and reset/exclusivity. The first
run had one numbered-save test still lacking the unlock; it was corrected and
rerun alone, retaining the 96 successful results. Final corrections were only
test fixtures and did not alter production code. The eight affected host-selector
cases and Workbench-draft case also passed under Wine without repeating the
entire runtime suite.

Production and test builds had no warnings. The EUR GUI with simulated XInput
verified hidden screens, absence of Debug options in Saves, unlocking on the
eighth direction with keyboard and pad, writing `debug = true`, reopening
unlocked and hiding again with `debug = false` on restarting the same copy.
Images were inspected; complete state/frame/clock/input and the three private
originals remained unchanged. Local evidence: `target/menu-debug-unlock/`.
Linux, native Windows, physical pads, DirectInput/Gilrs and USA were unverified
by these checks. No benchmark, guest step or new personal ROM/save test.
