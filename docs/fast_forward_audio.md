# Desktop Fast audio

Fast Forward has three audio choices: `fast` follows actual guest audio
production up to the selected 2x/4x/8x multiplier, `silent` discards guest audio,
and `normal` plays the accelerated guest's faithful PCM at 1x from a bounded
memory queue. The menu describes this as delayed 1x audio: it can fall behind
the video and periodically skip old audio. It is not pitch-preserving time
stretch. See the queue policy below and [BUG-03](development/todo_bug_fix.md).

## Discarding an old audio timeline

Fast activation/deactivation, multiplier changes and audio-mode changes now
invalidate both the producer queues and the callback's signal history. Opening
the pause menu does the same, discarding any Normal backlog. The
session Fast latch is preserved and the effective speed is evaluated again on
resume. Returning to 1x primes the ordinary rings again.

The device-loss watchdog also replaces pending output, including the Normal
queue, before attempting reconnection. Load/Undo continues to rebuild the stream
and goes through the same queue invalidation; neither uses audio from the
previous guest timeline.

`AudioStreams::discard_pending` increments a host epoch under the existing
mutex. At its next callback, the consumer resets the faithful DAC/fractional
position, PSG/HLE sinc histories, FIFO impulses/integrator/event position, DC
filters and crossfade state. It reuses filter tables and storage, and ramps new
audio in through the existing approximately 20 ms volume slew. No allocation,
I/O, logging, extra lock or clock is added to the callback reset. Ordinary 1x
playback retains its existing DSP and queue steering.

This removes stale **application-side** audio. Samples already submitted to the
device/backend remain subject to its buffering; the host epoch cannot retract
them. The new audio starts at the current producer position; it is not replayed
from the discarded queues. Guest mix, timers, snapshot format and native
translation ABI are unchanged.

## Verification and remaining diagnosis

A synthetic test first reproduced output after all producer queues were
emptied (faithful peak 0.19975911 on a normalized f32 output). With consumer
invalidation, all five paths (faithful, PSG, HLE, FIFO and Normal) produce exact
zero after discard at multipliers 1/2/4/8. A second regression compares fresh
audio after successive transitions against an independently initialized
callback: exact samples and remaining queues at 44.1/48 kHz, including
Faithful/Enhanced/HLE/Normal/Silent, observation on/off and multiple resets
between callbacks.

Native release runtime validation passed 68 tests (4 optional ignored), then
the added fresh-history regression passed separately. The final Windows GNU
runtime suite passed 70 tests under Wine (4 optional ignored). The native
candidate build succeeded and the prior canonical runtime was restored by
hash; portable exports were not replaced.

Actual Linux CPAL 44.1 kHz callbacks were exercised in an idle battle with
Xvfb/llvmpipe and native translation, separately for Faithful and Enhanced.
Hold/release, toggle, pause/resume and clean window close passed; inspected
captures show Fast remains latched after pause and returns to approximately
59.7 INT/EXT when disabled. The original slot bytes were unchanged. The test
used volume zero: this checks the real callback and lifecycle, not audible
quality. An obsolete launcher assertion in the helper failed after Faithful
had already exited successfully; its completed result was retained and only
the unrun Enhanced case followed the helper correction.

Those runs also reproduce a distinct consumption mismatch: at requested 4x,
captures show only 121.7/128.2 INT and 30.4/32.1 EXT. Faithful/Enhanced recorded
497/499 Fast callbacks, all with faithful source holds; about 47% of PSG/HLE
requests were empty. Enhanced's configured PSG/HLE targets were affected too.
Neither run reported producer drops or callbacks exceeding their period.
Normal-playing intervals had zero faithful holds (207/205 callbacks), and
paused mixed queues stayed empty. This supports underproduction relative to
the nominal 4x consumer on this CPU-rendered setup, rather than a growing
backlog in the reproduced case. It does not establish the user's hardware
behavior or a measured audio-to-video latency. Logs, inputs and captures are
local under ignored `target/fast-audio-reset/`.

## Following actual production in Fast

`fast_audio_rate::Clock` fixes that second verified cause. The frontend counts
produced faithful PCM pairs while Fast audio is active; the callback compares
their progress with output device frames. It needs no new wall-clock calls,
locks, I/O or callback allocation. The normal-play path does not count production.

Production is smoothed over approximately 200 ms. Queue feedback restores a
60 ms reserve after each complete device callback, with approximately 250 ms
correction time. One common speed slews per output sample over approximately
100 ms, and drives faithful PCM, PSG, HLE and both FIFO consumers. Their
independent queue steering is disabled only in Fast, so sources do not chase
different clocks. Effective speed is bounded to 0.05x through the requested
multiplier plus the existing 2% clock tolerance (at most 8.16x). It can follow
less than 1x production when the machine is heavily loaded. Guest execution,
frame pacing, timers and the generated translation remain independent.

Before starting, the consumer accumulates the reserve and one callback's source
demand. A discontinuity that exhausts this budget re-primes with explicit
silence and clears the old signal history instead of repeating stale source
samples. New output uses the existing volume ramp. This is recovery from
missing production, not a promise of continuous sound through an arbitrary
host stall. Normal and Silent retain their separate policies.

Fractional speeds use continuously blended conservative sinc cutoffs rather
than casting the speed to an integer and potentially choosing the 1x filter.
Tables for 1x through 8.25x in eighth steps are prepared when the stream is
created; both chosen cutoffs stay below the current output Nyquist. The two
grid resamplers add approximately 1.4 MiB of coefficient storage per stream;
the callback does no filter construction. FIFO timing uses the same effective
speed and retains its existing band-limited impulse renderer.

With `FFTA_PROFILE=1`, `FFTA_FAST_AUDIO` reports ready and waiting output frames,
rebuffer events, effective/production rates and maximum estimated queue delay.
It shares the existing guest-frame capture gate and context separation. A
zero underrun count must be assessed alongside waiting frames: priming silence
is explicitly visible and cannot be counted as successful source playback.

## Effective-rate verification

The ROM-free callback regression first fails on the old nominal4x consumer
with 536641 steady-state faithful holds while production runs at2x. The fixed
callback produces audible synthetic samples throughout the measured steady
interval with zero holds for Faithful and Enhanced. Clock tests cover72
combinations of 44.1/48/96 kHz,256/2048-frame buffers, requested2x/4x/8x and
actual0.5x/1x/1.75x/full speed: no steady waiting, no depleted queue, less than
200 ms estimated backlog and less than5% final speed error. Another test covers
4x->1x->3x->stopped->2x, reset and empty callbacks. Fractional-rate sine tests
check the passband and attenuated above-Nyquist tones. A real callback test
also verifies that a forced rebuffer emits silence and records waiting frames.

Final native runtime tests pass74cases (4optional ignored), with the final
empty-buffer clock refinement also checked by the2targeted clock tests.
Windows GNU builds without warnings and passes75runtime tests under Wine
(4optional ignored). Native and Windows production builds succeed; prior
canonical binaries were restored by hash and portable exports were not replaced.

The actual Linux CPAL44.1kHz/native-translation/Xvfb idle-battle check selects
Fast4x and measures guest frames9400:11400 after startup. Faithful357callbacks
and Enhanced359callbacks have zero faithful holds, PSG/HLE empty reads, producer
drops, over-period callbacks, waiting frames or rebuffer events. Mean effective
speed2.2032/2.1906 matches measured production2.2030/2.1906; maximum estimated
source-queue delay is107.3/108.0ms. All measured device frames are ready output,
so this result is not obtained by replacing starvation with silence. Slot bytes
are unchanged, normal1x returns to about59.7INT/EXT, and window closes are clean.
Enhanced's pause/resume keeps Fast active. A short dedicated Faithful
pause/Return-resume check confirms130.0INT/32.5EXT after resume, then59.7/59.7
after disabling Fast; it resolves an ambiguous1x capture from the first sequence.
Its imported helper failed an obsolete launcher assertion only after asserting
the runtime's clean exit; the completed captures/slot result were retained.
Tests run at volume zero: source consumption/lifecycle is verified, audibility
and physical speaker latency are not inferred. Local evidence is under ignored
`target/fast-audio-rate/`.

An earlier run overlapping Windows compilation is retained as variable-load
evidence, not a steady-play pass:0source holds/empty PSG-HLE reads, but3rebuffers
and13171waiting frames as production ranges roughly0.52x..2.25x. This makes
the recovery limit visible. Only the failed steady-load criterion was repeated
after compilation ended; existing baseline and passing suites were reused.

These checks establish the consumption correction. Native Windows hardware,
audible transition quality and recognizable audio/video event latency remain
separate validation. The later Normal queue correction below is independent
of the effective-rate fix and does not implement pitch-preserving stretch.

## Bounded delayed Normal audio

Normal keeps at most 2 seconds of stereo i16 PCM at 65536 Hz: 131072 pairs,
512 KiB, reserved once on producer setup. It always consumes at 1x through the
faithful path, including when Enhanced is configured. On overflow, it replaces
the old backlog with the latest 250 ms and crossfades from the old read head
over about 10 ms. This intentionally skips old sounds rather than allowing
delay to grow throughout a Fast session. A fixed 2620-byte tail holds the old
samples, including an unfinished earlier fade; both channels share the fade.

There is no temporary audio file, file growth, callback disk read, allocation
or error logging on consumption. The existing mutex protects the queue and
the transition epoch still discards it on mode/speed/pause/load/device changes.
Legacy `ff.audio=normal` remains valid; the menu labels the choice `1X` and
explains `UP TO 2S DELAY`. The 2-second budget bounds application PCM, not the
device/backend buffers or physical speaker latency.

With `FFTA_PROFILE=1`, `FFTA_NORMAL_AUDIO` reports per-context callback counts,
maximum queue pairs, queue limit, resync count and skipped pairs. Counters use
the existing frame gate and epoch-aware baselines, excluding earlier jumps.

Four queue tests cover stereo/order/reuse/invalid input, overflow fades,
oversized/repeated bursts during a fade and fixed storage over 6000 producer
ticks at each of 2x/4x/8x (93.75 seconds of 1x consumption per speed). A real
callback regression checks exact observed/unobserved output, audible synthetic
samples, 1x consumption, zero holds and silent discard. Native release suites
pass 78 runtime and 33 menu tests; Windows GNU/Wine passes 79 runtime and
33 menu tests (4 optional runtime tests ignored on each). Both production
builds are warning-free. Existing canonical binaries were restored by hash;
portable exports were not replaced.

Actual Linux CPAL 44.1 kHz/native/Xvfb idle-battle Normal-at-Fast4x gates
9400:11400 pass with Faithful 365 / Enhanced-configured 376 callbacks. Maximum
queues are 130287/130783 pairs, below the 131072 limit; each resynchronizes
10 times with zero faithful holds, producer drops or over-period callbacks.
Enhanced sources remain unrouted as required by Normal. Menu selection and
persistence, pause/resume, Fast-off return to approximately 59.7 INT/EXT,
unchanged slot bytes, absent audio spool files and clean closes are verified.
Captures were inspected. The first harness attempt paused at the root menu
due to an extra Escape and never reached the capture gate; it remains setup
failure evidence, not an audio result. Only the unmeasured gates were repeated
after correcting that helper. Logs and manifests are retained under ignored
`target/normal-audio-bounds/`.

These live checks use volume zero and do not establish audible fade quality,
audio/video event offsets, native Windows hardware or physical reconnection.
