# Desktop fullscreen

Open **Video > Fullscreen (F11)** and use Left/Right or click its value.
F11 uses the same operation during gameplay and while the pause menu is open;
holding it does not repeatedly toggle. Fullscreen is borderless on the current
monitor. No display mode or exclusive refresh is requested.

`video.fullscreen = true|false` in `av.cfg` persists the choice, default false
for existing configs. The preference applies after the window is created on
the next launch. Paths use existing installed/portable storage; see
[portable settings](portable.md). Window geometry itself is not persisted
between sessions. Deterministic GUI benchmarks ignore this preference.

Winit/the window manager retain the pre-fullscreen normal placement and
maximized state for restoration in the same session. If the former placement
no longer has a usable intersection with an available monitor, the host moves
it to an available current/primary monitor, bounds an oversized normal window
and preserves maximized mode. This fallback uses physical monitor coordinates,
including negative desktop origins; positioning remains unavailable on
backends such as Wayland that do not expose it.

The menu follows the state reported by winit. A pending request gets up to two
seconds for compositor acknowledgement before the preference is reconciled
with the reported result; a stale previous state cannot immediately undo a new
request. F11 can cancel a pending request. Backends that do not report external
window-manager fullscreen changes cannot provide that synchronization; native
Windows/macOS/Wayland and physical hotplug coverage remain separate.

Fullscreen uses existing resize/DPI/cursor/presenter handling. It does not
change aspect ratio, game zoom, emulation speed, audio queues, guest state or
video temporal history. During menu resizing, both GPU upload and software
sampling use the dimensions of the completed canvas, scaling it to the latest
window size; they do not reinterpret old pixels with a new window stride.

## Validation

Native release suites pass 28 input/config, 35 menu and 81 runtime tests;
Windows GNU/Wine passes 28/35/82 (4 optional runtime tests ignored per target).
Both production builds are warning-free. Tests cover old/default configs,
roundtrip, menu/mouse toggles and scrolling, delayed acknowledgement/rejection,
pending cancellation, external reported state and negative/removed monitor
coordinates with oversized-window fallback. Idle reconciliation does not read
a clock. The final resize correction passes both runtime suites again; the
unchanged config/menu suites were reused.

Actual Linux CPAL/native-translation/Xvfb/Openbox checks verify held F11 toggles
once, restoration of a 640x360 client at the same coordinates, menu/F11 sync,
fullscreen at restart, maximized/unmaximized geometry, VSync On and fullscreen
minimize/restore. Private slot bytes stay unchanged and closes are clean.
Inspected captures show intact game/menu content and no GPU fallback.
Windows GNU under Wine/Openbox passes the same two-run lifecycle and menu
checks, with a 640x360 client restored at the same coordinates and the same
maximized client size of 1200x779. This GUI uses the interpreter to reuse the
existing private state without producing another native translation fixture;
it does not establish native Windows behavior or performance. Menu/startup/
windowed captures were inspected. Both slots stay unchanged and exits are zero.

An additional native software-rendering check uses process-local unavailable
Vulkan/GL driver paths while retaining normal EGL vendors. The GL surface
cannot configure and the existing fallback activates. Held F11, menu/F11 sync,
fullscreen canvas, 640x360 restoration, unchanged slot and clean close all pass;
software captures were inspected. An earlier forced empty EGL-vendor setup
panicked in wgpu initialization before the window became usable; it was not
counted as a presentation result and only that unmeasured fallback gate was
retried with valid vendors. No production environment switch was added.

The first live run exposed a menu resize race: GPU upload interpreted a
640x360 canvas as the new 1200x800 window, overran the upload source and the
software fallback then panicked on the same wrong stride. The corrected paths
use the completed canvas dimensions. That negative candidate/source/log is
retained separately. Later pointer/focus setup failures selected the wrong
root menu row; already passed F11/geometry evidence was retained and only
unmeasured menu/restart criteria followed the helper correction. Local evidence
is under ignored `target/fullscreen-menu/`.

Physical monitor removal, DPI/refresh changes, controller hardware and native
Windows/macOS/Wayland remain unverified. Winit cannot report every external WM
fullscreen change on every backend; no new platform-specific window hooks were
added. Reuse the previously completed PERF-04 monitor/minimize matrix rather
than treating this feature as a reason to repeat it.
