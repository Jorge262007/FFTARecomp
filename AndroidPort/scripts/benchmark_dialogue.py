"""Debug APK replay benchmark; never uses or overwrites the normal game save.

Requires the user's benchmark.input and benchmark.sav already in app files/.
Run from frame zero; the replay's state is cumulative. Uses imported ffta.gba.
Example: python scripts/benchmark_dialogue.py zoom6 --adb C:/platform-tools/adb.exe
"""
import argparse
import csv
import io
from pathlib import Path
import math
import statistics
import subprocess
import time

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('tag')
p.add_argument('--adb', default='adb')
p.add_argument('--serial')
p.add_argument('--frames', type=int, default=13100)
p.add_argument('--zoom', type=int, choices=range(7), default=6)
p.add_argument('--device-aspect', action='store_true', help='Use the connected display landscape aspect instead of the square reference')
p.add_argument('--slow-cpu', action='store_true', help='Reference: leave affinity to Android')
p.add_argument('--reference-frame', action='store_true', help='Reference: disable frame-work elimination')
p.add_argument('--reference-ui', action='store_true', help='Reference: old UI pixel extractor')
p.add_argument('--reference-dialogue', action='store_true', help='Reference: disable native-row reuse and ordered UI merging')
a = p.parse_args()
if not 600 <= a.frames <= 30000:
    p.error('frames must be between 600 and 30000')
if Path(a.tag).name != a.tag or a.tag in ('.', '..'):
    p.error('tag must be a single directory name')
out = Path(__file__).resolve().parents[1] / 'target' / ('dialogue-' + a.tag)
out.mkdir(parents=True, exist_ok=True)
adb = [a.adb] + (['-s', a.serial] if a.serial else [])
pkg = 'com.fftarecomp.port'


def call(*args, check=True):
    result = subprocess.run(adb + list(args), capture_output=True)
    if check and result.returncode:
        raise RuntimeError(result.stderr.decode(errors='replace'))
    return result.stdout


def shell(*args):
    return call('shell', *args).decode(errors='replace')


save = call('exec-out', 'run-as', pkg, 'cat', 'files/ffta.sav')
(out / 'normal-save-backup.sav').write_bytes(save)
shell('am', 'force-stop', pkg)
shell('run-as', pkg, 'rm', '-f', 'cache/benchmark.csv', 'cache/benchmark-camera.csv',
      'cache/benchmark-hashes.txt', 'cache/benchmark-characters.csv')
side = 256 * 2 ** (a.zoom / 6)
ex, ey = math.ceil((side - 240) / 2), math.ceil((side - 160) / 2)
if a.device_aspect:
    import re
    width,height=map(int,re.findall(r'(\d+)x(\d+)',shell('wm','size'))[-1])
    ratio=max(width,height)/min(width,height)
    factor=2**(a.zoom/6)
    ex=math.ceil((160*ratio*factor-240)/2);ey=math.ceil((160*factor-160)/2)
print('Device' if a.device_aspect else 'Square', 'viewport zoom', a.zoom, 'render', 240 + 2 * ex, 160 + 2 * ey, flush=True)
extras = ['--ei', 'ffta_benchmark_frames', str(a.frames),
          '--ei', 'ffta_benchmark_extra_x', str(ex), '--ei', 'ffta_benchmark_extra_y', str(ey)]
for name, value in [('ffta_benchmark', True), ('ffta_benchmark_clock', False),
                    ('ffta_benchmark_metrics', False), ('ffta_benchmark_fill',a.device_aspect), ('ffta_benchmark_profile_ui', True),
                    ('ffta_benchmark_fast_cpu', not a.slow_cpu),
                    ('ffta_benchmark_reference_frame', a.reference_frame),
                    ('ffta_benchmark_reference_dialogue', a.reference_dialogue),
                    ('ffta_benchmark_reference_ui', a.reference_ui)]:
    extras += ['--ez', name, str(value).lower()]
print(shell('am', 'start', '-n', pkg + '/.MainActivity', *extras), flush=True)
started, last = time.monotonic(), -1
while time.monotonic() - started < 1800:
    text = call('shell', 'run-as', pkg, 'cat', 'cache/benchmark.csv', check=False).decode(errors='replace')
    rows = list(csv.DictReader(io.StringIO(text)))
    frame = int(rows[-1]['frame']) if rows else 0
    if frame // 600 != last // 600:
        print('frame', frame, 'elapsed', round(time.monotonic() - started, 1), flush=True)
    last = frame
    if frame==0 and time.monotonic()-started>15 and not shell('pidof',pkg).strip():
        raise RuntimeError('Benchmark process exited before its first frame; inspect app crash log')
    hashes = call('shell', 'run-as', pkg, 'cat', 'cache/benchmark-hashes.txt', check=False).decode(errors='replace')
    if frame == a.frames and hashes.rstrip().endswith('COMPLETE'):
        break
    time.sleep(2)
else:
    raise RuntimeError('Replay timed out')
for name in ['benchmark.csv', 'benchmark-camera.csv', 'benchmark-hashes.txt', 'performance.txt', 'benchmark-loop.csv']:
    (out / name).write_bytes(call('exec-out', 'run-as', pkg, 'cat', 'cache/' + name))
(out / 'screen.png').write_bytes(call('exec-out', 'screencap', '-p'))
assert save == call('exec-out', 'run-as', pkg, 'cat', 'files/ffta.sav'), 'Normal save changed'
rows = list(csv.DictReader((out / 'benchmark-camera.csv').open()))
parts = [('panel active', [r for r in rows if r['ui_active'] == '1'])]
if a.frames == 13100:
    parts.append(('final replay segment (not necessarily moving camera)', rows[-1201:]))
moving=[row for previous,row in zip(rows,rows[1:])
        if (row['hofs'],row['vofs'])!=(previous['hofs'],previous['vofs']) and row['hofs']!='-1']
if moving:parts.append(('camera actually moving',moving))
for label, part in parts:
    if not part:
        continue
    print(label, len(part), 'samples')
    for key in ['frame_ms', 'raster_ms', 'zoom_ms', 'ui_capture_ms', 'ui_setup_ms', 'ui_export_ms']:
        values = sorted(float(r[key]) for r in part)
        print(key, 'mean', round(statistics.mean(values), 4),
              'p95', round(values[int(.95 * (len(values) - 1))], 4))
print('Normal save unchanged; results', out)
