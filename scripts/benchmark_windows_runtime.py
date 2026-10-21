"""Compare native Linux/macOS or Windows/Wine runtimes on one ROM translation.

Historical filename retained for existing commands. Verification, uninstrumented
work timing and optional frame-time diagnostics run separately. Saves are read-only.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys


HASH = r'[0-9a-f]{64}'
FLOAT = r'(?:[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)'


def digest(path):
    checksum = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            checksum.update(chunk)
    return checksum.hexdigest()


def binary_platform(path):
    with path.open('rb') as source:
        magic = source.read(4)
    if magic.startswith(b'MZ'):
        return 'Windows'
    if magic == b'\x7fELF':
        return 'Linux'
    if magic in (b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf',
                 b'\xce\xfa\xed\xfe', b'\xfe\xed\xfa\xce', b'\xca\xfe\xba\xbe'):
        return 'Darwin'
    raise ValueError(f'unrecognized executable format: {path}')


def parse_profile_stats(row, samples, require_totals=False):
    try:
        parts = row.split()
        fields = dict(part.split('=', 1) for part in parts)
        if len(fields) != len(parts):
            raise ValueError('duplicate profile field')
        stats = {k: int(fields[k]) for k in
                 ('samples', 'omitted', 'over_budget', 'over_double_budget')}
        stats.update({k: float(fields[k]) for k in
                      ('p50_ms', 'p95_ms', 'p99_ms', 'max_ms')})
        if require_totals or 'total_ms' in fields or 'mean_ms' in fields:
            stats.update({k: float(fields[k]) for k in ('total_ms', 'mean_ms')})
    except (ValueError, KeyError) as error:
        raise ValueError('invalid frame-time diagnostics') from error
    ordered = [stats[k] for k in ('p50_ms', 'p95_ms', 'p99_ms', 'max_ms')]
    if (stats['samples'] != samples or stats['omitted'] != 0
            or not all(math.isfinite(v) and v >= 0 for v in ordered)
            or ordered != sorted(ordered)
            or not 0 <= stats['over_double_budget'] <= stats['over_budget'] <= samples):
        raise ValueError('incomplete or inconsistent frame-time diagnostics')
    if 'total_ms' in stats:
        total, mean = stats['total_ms'], stats['mean_ms']
        tolerance = 0.001 + samples * 0.000001  # six-decimal report rounding
        if (not all(math.isfinite(v) and v >= 0 for v in (total, mean))
                or abs(total - mean * samples) > tolerance
                or mean > stats['max_ms'] + 0.000001):
            raise ValueError('inconsistent profile totals')
    return stats


def parse_scanline_stages(text, samples, scanline_total):
    if not re.search(r'^FFTA_PPU_STAGE ', text, re.M):
        return None  # Older/coarse-only runtime.
    rows = re.findall(r'^FFTA_PPU_STAGE phase=(\w+) (.+)$', text, re.M)
    names = {'backgrounds', 'objects', 'composition', 'ui_capture', 'ui_setup', 'scanline_other'}
    if len(rows) != len(names) or {name for name, _ in rows} != names:
        raise ValueError('missing or duplicate scanline-stage diagnostics')
    stages = {name: parse_profile_stats(row, samples, require_totals=True) for name, row in rows}
    if abs(sum(stage['total_ms'] for stage in stages.values()) - scanline_total) > 0.01:
        raise ValueError('scanline-stage durations do not partition scanline time')
    return stages


def parse_run(text, frames, warmup, capture=False, verify_state=False, profile=False, ppu_profile=False, snapshot=False):
    starts = re.findall(rf'^FFTA_REPLAY_STATE frame=(\d+) machine_sha256=({HASH})$', text, re.M)
    if snapshot:
        if len(starts) != 1:
            raise ValueError('missing or duplicate initial snapshot identity')
    elif re.search(r'^FFTA_REPLAY_STATE ', text, re.M):
        raise ValueError('snapshot unexpectedly loaded')
    start = int(starts[0][0]) if snapshot else 0
    ends = re.findall(r'^frames: (\d+) hash: ([0-9a-f]{16}) '
                      r'native_blocks: (\d+) fallback_steps: (\d+)$', text, re.M)
    timings = re.findall(r'^FFTA_BENCH first=(\d+) last=(\d+) frames=(\d+) '
                         rf'seconds=({FLOAT}) ms_per_frame=({FLOAT}) width=(\d+)$', text, re.M)
    scenes = re.findall(rf'^FFTA_BENCH_SCENE frame=(\d+) guest_sha256=({HASH}) '
                        r'sprite_requested=(true|false)$', text, re.M)
    if len(ends) != 1 or len(timings) != 1 or len(scenes) != 1:
        raise ValueError('missing or duplicate run/timing/scene summary')
    end, timing, scene = ends[0], timings[0], scenes[0]
    if (int(end[0]) != start + frames or tuple(map(int, timing[:3])) !=
            (start + warmup, start + frames, frames - warmup) or int(scene[0]) != start + warmup):
        raise ValueError('incomplete run or incorrect measured interval')
    seconds, ms = float(timing[3]), float(timing[4])
    if not all(math.isfinite(v) and v > 0 for v in (seconds, ms)):
        raise ValueError('nonfinite or nonpositive timing')
    if abs(ms * (frames - warmup) - seconds * 1000) > 0.001 + (frames - warmup) * 0.000001:
        raise ValueError('inconsistent timing totals')
    hashes = {}
    for frame, fb, oam in re.findall(
            rf'^FFTA_EQ frame=(\d+) framebuffer=({HASH}) oam=({HASH})$', text, re.M):
        frame = int(frame)
        if frame in hashes:
            raise ValueError(f'duplicate frame capture: {frame}')
        hashes[frame] = (fb, oam)
    if capture and set(hashes) != set(range(start + 1, start + frames + 1)):
        raise ValueError('incomplete frame capture')
    if not capture and hashes:
        raise ValueError('frame hashing unexpectedly enabled during timing')
    states = re.findall(rf'^FFTA_FINAL_STATE sha256=({HASH})$', text, re.M)
    if verify_state and len(states) != 1:
        raise ValueError('runtime does not provide exactly one final machine-state hash')
    stats = None
    if profile:
        rows = re.findall(r'^FFTA_FRAME_TIMES (.+)$', text, re.M)
        if len(rows) != 1:
            raise ValueError('runtime does not provide frame-time diagnostics')
        stats = parse_profile_stats(rows[0], frames - warmup)
        if stats.get('total_ms', 0) > seconds * 1000 + 0.01:
            raise ValueError('frame-time durations exceed the measured interval')
    ppu = None
    if ppu_profile:
        counts = re.findall(r'^FFTA_PPU_COUNTS frames=(\d+) scanlines=(\d+) zoom=(\d+) invalid=(\d+)$', text, re.M)
        rows = re.findall(r'^FFTA_PPU_FRAME phase=(\w+) (.+)$', text, re.M)
        names = {'hardware_scanlines', 'zoom_rows', 'core_other'}
        if (len(counts) != 1 or len(rows) != 3 or {name for name, _ in rows} != names):
            raise ValueError('missing or duplicate PPU diagnostics')
        measured, scanlines, zoom, invalid = map(int, counts[0])
        if measured != frames - warmup or scanlines != measured * 160 or zoom > measured or invalid:
            raise ValueError('incomplete or inconsistent PPU capture')
        stages = {name: parse_profile_stats(row, measured, require_totals=True) for name, row in rows}
        if sum(stage['total_ms'] for stage in stages.values()) > seconds * 1000 + 0.01:
            raise ValueError('PPU/core stage durations exceed the measured interval')
        ppu = {'frames': measured, 'scanlines': scanlines, 'zoom': zoom, 'stages': stages,
               'scanline_stages': parse_scanline_stages(text, measured, stages['hardware_scanlines']['total_ms'])}
    elif re.search(r'^FFTA_PPU_(?:COUNTS|FRAME|STAGE) ', text, re.M):
        raise ValueError('PPU profiling unexpectedly enabled during timing')
    return {'frames': int(end[0]), 'start_frame': start,
            'initial_state': starts[0] if snapshot else None, 'framebuffer': end[1],
            'native_blocks': int(end[2]), 'fallback_steps': int(end[3]),
            'ms_per_frame': ms, 'hashes': hashes, 'scene': scene,
            'width': int(timing[5]), 'state': states[0] if states else None,
            'frame_times': stats, 'ppu': ppu}


def assert_equivalent(reference, candidate, require_counts=False):
    for key in ('frames', 'start_frame', 'initial_state', 'framebuffer', 'scene', 'width'):
        if reference[key] != candidate[key]:
            raise ValueError(f'divergent {key}')
    if reference['hashes'] and candidate['hashes'] != reference['hashes']:
        raise ValueError('framebuffer/OAM differ during replay')
    if reference['state'] is not None and candidate['state'] != reference['state']:
        raise ValueError('serialized machine state differs')
    if require_counts and any(reference[k] != candidate[k] for k in
                              ('native_blocks', 'fallback_steps')):
        raise ValueError('execution counts differ')


def atomic_json(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + '\n', encoding='utf-8')
    temporary.replace(path)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('baseline', 'candidate', 'rom', 'library', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    inputs = parser.add_mutually_exclusive_group(required=True)
    inputs.add_argument('--input', type=Path)
    inputs.add_argument('--demo', action='store_true', help='deterministic introductory Start/A taps')
    initial = parser.add_mutually_exclusive_group()
    initial.add_argument('--sav', type=Path)
    initial.add_argument('--state', type=Path, help='resume desktop slot or raw snapshot; controls are relative to it')
    parser.add_argument('--frames', type=int, default=6000)
    parser.add_argument('--warmup', type=int, default=120)
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--widescreen', default='16:9')
    parser.add_argument('--zoom', help='headless zoom schedule, e.g. 0:0,1000:4')
    parser.add_argument('--scenario', help='descriptive workload name; no inferred AI coverage')
    parser.add_argument('--timeout', type=float, default=300, help='maximum seconds per process')
    parser.add_argument('--wine')
    parser.add_argument('--wine-prefix', type=Path)
    parser.add_argument('--verify-state', action='store_true', help='require full serialized final-state hashes')
    parser.add_argument('--baseline-rom-data', action='store_true', help='enable --rom-data only for the baseline runtime')
    parser.add_argument('--candidate-rom-data', action='store_true', help='enable --rom-data only for the candidate runtime')
    parser.add_argument('--profile', action='store_true', help='separate instrumented frame-time runs')
    parser.add_argument('--ppu-profile', action='store_true', help='also measure CPU scanlines/zoom/core remainder; implies --profile')
    parser.add_argument('--require-execution-counts', action='store_true',
                        help='also require dispatch counts to match (same translation/dispatch)')
    args = parser.parse_args(argv)
    if args.state and args.demo:
        parser.error('--state needs an explicit --input replay, not the introductory --demo')
    args.profile = args.profile or args.ppu_profile
    if (args.frames <= args.warmup or args.warmup < 0 or args.runs < 1
            or not math.isfinite(args.timeout) or args.timeout <= 0):
        parser.error('require frames > warmup >= 0, runs >= 1 and positive finite timeout')
    protected = {name: path.resolve() for name in
                 ('baseline', 'candidate', 'rom', 'library', 'input', 'sav', 'state')
                 if (path := getattr(args, name)) is not None}
    for name, path in protected.items():
        if not path.is_file():
            parser.error(f'file not found ({name}): {path}')
    try:
        target = binary_platform(args.baseline)
        if binary_platform(args.candidate) != target:
            raise ValueError('baseline and candidate target different platforms')
        if args.wine and target != 'Windows':
            raise ValueError('--wine requires Windows executables')
        if not args.wine and target != platform.system():
            raise ValueError(f'{target} binaries cannot run natively on {platform.system()}')
    except ValueError as error:
        parser.error(str(error))
    if args.wine_prefix and not args.wine:
        parser.error('--wine-prefix requires --wine')
    args.out = args.out.resolve()
    reserved = ['summary.json', 'summary.json.tmp', 'demo.input']
    reserved += [f'{name}-{kind}.log' for name in ('baseline', 'candidate')
                 for kind in ['equivalence', 'profile'] + [str(i + 1) for i in range(args.runs)]]
    if set(protected.values()) & {(args.out / name).resolve() for name in reserved}:
        parser.error('output files would overwrite a benchmark input')
    args.out.mkdir(parents=True, exist_ok=True)
    atomic_json(args.out / 'summary.json', {'status': 'running'})
    original_hashes = {name: digest(path) for name, path in protected.items()}
    if args.demo:
        args.input = args.out / 'demo.input'
        rows, previous = ['gba-input v1'], None
        for frame in range(args.frames + 1):
            phase, keys = frame % 180, 0x3ff
            if frame >= 240:
                if phase < 10:
                    keys &= ~(1 << 3)
                elif 60 <= phase < 70:
                    keys &= ~1
            if keys != previous:
                rows.append(f'{frame} {keys:03x}')
                previous = keys
        args.input.write_text('\n'.join(rows) + '\n', encoding='utf-8')
    env = os.environ.copy()
    env['FFTA_BENCH_LIBRARY'] = str(args.library.resolve())
    env['FFTA_WIDESCREEN'] = args.widescreen
    env['FFTA_BENCH_FROM'] = str(args.warmup)
    env['FFTA_ZOOM_LEVEL'] = args.zoom or '0'
    # Do not silently mix the diagnostic cost-dispatch path into normal timing.
    env.pop('RECOMP_COST_FROM', None)
    if args.wine:
        env['WINEDEBUG'] = '-all'
        if args.wine_prefix:
            env['WINEPREFIX'] = str(args.wine_prefix.resolve())

    def run(label, exe, capture=False, profile_times=False):
        run_env = env.copy()
        run_env['FFTA_PATCH_VERIFY_FRAMES'] = '1' if capture else '0'
        run_env['FFTA_BENCH_STATE'] = '1' if args.verify_state else '0'
        run_env['FFTA_BENCH_FRAME_TIMES'] = '1' if profile_times else '0'
        run_env['FFTA_BENCH_PPU'] = '1' if profile_times and args.ppu_profile else '0'
        command = ([args.wine] if args.wine else []) + [
            str(exe.resolve()), 'runc', str(args.rom.resolve()),
            '--frames', str(args.frames), '--input', str(args.input.resolve())]
        if args.sav:
            command += ['--sav', str(args.sav.resolve())]
        if args.state:
            command += ['--state', str(args.state.resolve())]
        variant = label.split('-', 1)[0]
        if getattr(args, variant + '_rom_data'):
            command.append('--rom-data')
        log = args.out / f'{label}.log'
        with log.open('w', encoding='utf-8') as output:
            subprocess.run(command, env=run_env, stdout=output, stderr=subprocess.STDOUT,
                           check=True, timeout=args.timeout)
        result = parse_run(log.read_text(encoding='utf-8'), args.frames, args.warmup,
                           capture, args.verify_state, profile_times, profile_times and args.ppu_profile, snapshot=args.state is not None)
        print(f'{label}: {result["ms_per_frame"]:.6f} ms/frame', flush=True)
        return result

    def compare(reference, candidate):
        assert_equivalent(reference, candidate, args.require_execution_counts)

    try:
        reference = run('baseline-equivalence', args.baseline, capture=True)
        candidate = run('candidate-equivalence', args.candidate, capture=True)
        compare(reference, candidate)
        print(f'Equivalent framebuffer/OAM on all {args.frames} frames.', flush=True)
        samples = {'baseline': [], 'candidate': []}
        counts = {}
        # Capture hashes only in verification; timing runs compare the final state.
        timed_reference = dict(reference, hashes={})
        for i in range(args.runs):
            order = [('baseline', args.baseline), ('candidate', args.candidate)]
            if i % 2:
                order.reverse()
            for name, exe in order:
                result = run(f'{name}-{i + 1}', exe)
                compare(timed_reference, result)
                samples[name].append(result['ms_per_frame'])
                counts[name] = {k: result[k] for k in ('native_blocks', 'fallback_steps')}
        profiles = {}
        if args.profile:
            for name, exe in [('baseline', args.baseline), ('candidate', args.candidate)]:
                result = run(f'{name}-profile', exe, profile_times=True)
                compare(timed_reference, result)
                profiles[name] = {'frame_times': result['frame_times'], 'ppu': result['ppu'],
                                  'instrumented_ms_per_frame': result['ms_per_frame'],
                                  'overhead_vs_median_percent':
                                      (result['ms_per_frame'] / statistics.median(samples[name]) - 1) * 100}
        before, after = (statistics.median(samples[k]) for k in ('baseline', 'candidate'))
        summary = {
            'status': 'passed', 'schema': 2,
            'platform': target + (' via Wine' if args.wine else ' native'),
            'host': {'system': platform.system(), 'release': platform.release(),
                     'machine': platform.machine()},
            'scenario': args.scenario or ('intro-demo' if args.demo else 'user-replay'),
            'frames': args.frames, 'start_frame': reference['start_frame'],
            'initial_state': reference['initial_state'],
            'warmup': args.warmup, 'widescreen': args.widescreen,
            'zoom': env['FFTA_ZOOM_LEVEL'], 'runs': args.runs,
            'decoded_resources': {'baseline': args.baseline_rom_data, 'candidate': args.candidate_rom_data},
            'framebuffer_oam_equal': True, 'full_final_state_equal': args.verify_state,
            'final_framebuffer': reference['framebuffer'], 'final_state_sha256': reference['state'],
            'execution_counts': counts, 'execution_counts_required': args.require_execution_counts,
            'samples_ms_per_frame': samples, 'median_baseline_ms': before,
            'median_candidate_ms': after, 'emulation_time_reduction_percent': (1 - after / before) * 100,
            'frame_time_profiles': profiles,
            'note': 'Headless work timing, excludes the window/audio device. AI coverage requires an identified AI-turn replay. Profile overhead is a separate single-run estimate, not a speedup.',
            'sha256': {**original_hashes, 'input': digest(args.input)},
        }
        for name, path in protected.items():
            if digest(path) != original_hashes[name]:
                raise ValueError(f'benchmark input changed: {name}')
        atomic_json(args.out / 'summary.json', summary)
        return summary
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        atomic_json(args.out / 'summary.json', {'status': 'failed', 'error': str(error)})
        raise


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f'Benchmark failed: {error}', file=sys.stderr)
        sys.exit(1)
