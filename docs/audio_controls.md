# Desktop audio controls

Open the pause menu with Escape, then **Audio**. Volume, Mute and Enhanced
audio apply live. Use Left/Right on the selected row (keyboard or gamepad), or
click its value. Mute is independent of Volume: turning it on preserves the
chosen level, including volume changes made while muted. Turning it off
returns to that level unless Fast Silent or an audio fallback still requires
silence. Volume zero remains separate from Mute On.

`av.cfg` stores `audio.mute = true|false`, default false for old/missing configs,
and `audio.volume = 0..1`. The same audio configuration is applied at stream
startup and after menu edits, so saved volume and mute apply before the first
callback. Portable settings use `Data/config/av.cfg`; installed settings use
the existing platform config directory (Windows `%APPDATA%\FFTARecomp`). See
[portable storage](portable.md).

User mute only changes final output gain. It does not stop the device stream,
source consumption, guest timing or frame pacing, and does not clear queues or
filter history. It survives pause/load/Fast timeline resets. Temporary Fast
Silent/fallback mute has separate state and cannot be undone by changing the
user toggle. Faithful, Enhanced, Fast and delayed Normal share the same final
gain; see [Fast audio](fast_forward_audio.md) for those modes' queue policies.

Gain uses the existing exponential slew with a roughly 20 ms time constant.
A mute tail below -100 dB is snapped to exact zero (about 231 ms from full
volume); unmute slews toward the retained level. Audio-only menu changes do
not rebuild video temporal history. No guest state or save format changes.

## Validation

Config tests cover old/default/invalid values, case-insensitive true, roundtrip
and retained volume. Menu tests cover shared keyboard/gamepad navigation,
mouse toggles, value display and volume edits while muted. The actual callback
test compares muted and reference consumption/queues/counters through startup,
mute/unmute, volume edits, temporary mute and discard for Faithful, Enhanced,
Fast and Normal. It checks smooth gain on both transitions, exact settled
silence, audible synthetic output after unmute and return to the edited level.

Native release suites pass 27 input/config, 34 menu and 79 runtime tests
(4 optional runtime ignored); the final gain-curve assertions pass separately.
Windows GNU production/tests compile warning-free and Wine passes 27 config,
34 menu and 80 runtime tests (4 optional runtime ignored). Native production
build passes; existing canonical binaries are restored by hash. These tests
use synthetic sound and do not establish physical hardware latency or audible
transition quality on native Windows. No portable exports are updated.

The isolated native GUI uses actual CPAL 44.1 kHz, native translation and
Xvfb/llvmpipe. Inspected captures show Mute On at 20%, a live edit to 30% and
Enhanced On, then the same saved values after a clean restart. Unmute is
checked after setting Volume to zero, keeping the live test quiet. Both runs
load the existing private slot, exercise Fast4x and return to about 59.7
INT/EXT, retain unchanged slot bytes and close cleanly. Playing171/170 and
Fast108/108 callbacks continue with zero faithful holds/drops/over-period calls;
priming/pause remain separate and can have empty-source holds. This is menu,
persistence and lifecycle evidence, not an audible hardware-quality check.
Local manifests, logs and captures: ignored `target/audio-mute/`.
