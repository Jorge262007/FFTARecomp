"""Behavioral checks for benchmark rejection and input/output separation."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('benchmark', Path(__file__).parents[1] / 'benchmark_windows_runtime.py')
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


class BenchmarkTests(unittest.TestCase):
    def log(self, capture=True):
        rows = [f'FFTA_BENCH_SCENE frame=1 guest_sha256={"a" * 64} sprite_requested=true',
                'FFTA_BENCH first=1 last=3 frames=2 seconds=0.004000 ms_per_frame=2.000000 width=284',
                'frames: 3 hash: 0123456789abcdef native_blocks: 50 fallback_steps: 100']
        if capture:
            rows += [f'FFTA_EQ frame={i} framebuffer={"b" * 64} oam={"c" * 64}' for i in range(1, 4)]
        return '\n'.join(rows)

    def test_requires_complete_unique_frame_capture(self):
        text = self.log()
        self.assertEqual(len(bench.parse_run(text, 3, 1, capture=True)['hashes']), 3)
        for broken in (text.rsplit('\n', 1)[0], text + '\n' + text.splitlines()[-1],
                       text.replace('last=3', 'last=2'), text.replace('first=1', 'first=0')):
            with self.subTest(broken=broken), self.assertRaises(ValueError):
                bench.parse_run(broken, 3, 1, capture=True)

    def test_timing_cannot_contain_hash_capture_or_zero_time(self):
        with self.assertRaises(ValueError):
            bench.parse_run(self.log(), 3, 1)
        with self.assertRaises(ValueError):
            bench.parse_run(self.log(False).replace('ms_per_frame=2.000000', 'ms_per_frame=0'), 3, 1)
        with self.assertRaises(ValueError):
            bench.parse_run(self.log(False).replace('seconds=0.004000', 'seconds=1e999'), 3, 1)

    def test_machine_state_and_warmup_divergence_are_rejected(self):
        text = self.log() + '\nFFTA_FINAL_STATE sha256=' + 'd' * 64
        reference = bench.parse_run(text, 3, 1, capture=True, verify_state=True)
        for wrong in (dict(reference, state='e' * 64), dict(reference, scene=('1', 'f' * 64, 'true'))):
            with self.assertRaises(ValueError):
                bench.assert_equivalent(reference, wrong)
        with self.assertRaises(ValueError):
            bench.parse_run(self.log(), 3, 1, capture=True, verify_state=True)

    def test_translation_changes_can_change_dispatch_counts(self):
        reference = bench.parse_run(self.log(), 3, 1, capture=True)
        different = dict(reference, native_blocks=300, fallback_steps=0)
        bench.assert_equivalent(reference, different)
        with self.assertRaises(ValueError):
            bench.assert_equivalent(reference, different, require_counts=True)

    def test_profile_must_be_complete_ordered_and_finite(self):
        line = 'FFTA_FRAME_TIMES samples=2 omitted=0 p50_ms=1 p95_ms=3 p99_ms=3 max_ms=3 over_budget=0 over_double_budget=0'
        text = self.log(False) + '\n' + line
        self.assertEqual(bench.parse_run(text, 3, 1, profile=True)['frame_times']['p99_ms'], 3)
        with self.assertRaises(ValueError):
            bench.parse_run(text + ' total_ms=5 mean_ms=2.5', 3, 1, profile=True)
        for broken in (text.replace('samples=2', 'samples=1'), text.replace('omitted=0', 'omitted=1'),
                       text.replace('p99_ms=3', 'p99_ms=nan'), text.replace('p99_ms=3', 'p99_ms=5'),
                       text.replace('over_double_budget=0', 'over_double_budget=1')):
            with self.subTest(broken=broken), self.assertRaises(ValueError):
                bench.parse_run(broken, 3, 1, profile=True)

    def test_ppu_profile_requires_complete_disjoint_finite_capture(self):
        meta = 'FFTA_PPU_COUNTS frames=2 scanlines=320 zoom=1 invalid=0'
        stages = []
        for name, value in [('hardware_scanlines', 0.5), ('zoom_rows', 0.2), ('core_other', 1.0)]:
            stages.append(f'FFTA_PPU_FRAME phase={name} samples=2 omitted=0 p50_ms={value} '
                          f'p95_ms={value} p99_ms={value} max_ms={value} over_budget=0 '
                          f'over_double_budget=0 total_ms={value * 2} mean_ms={value}')
        text = self.log(False) + '\n' + '\n'.join([meta, *stages])
        result = bench.parse_run(text, 3, 1, ppu_profile=True)['ppu']
        self.assertEqual(result['scanlines'], 320)
        self.assertEqual(result['stages']['zoom_rows']['total_ms'], 0.4)
        variants = [text.rsplit('\n', 1)[0], text + '\n' + meta, text + '\n' + stages[0],
                    text.replace('scanlines=320', 'scanlines=319'),
                    text.replace('zoom=1', 'zoom=3'), text.replace('invalid=0', 'invalid=1'),
                    text.replace('mean_ms=0.5', 'mean_ms=nan'),
                    text.replace('mean_ms=0.5', 'mean_ms=0.1'),
                    text.replace('mean_ms=0.5', 'other=0.5'),
                    text.replace('samples=2', 'samples=1'), text.replace('omitted=0', 'omitted=1'),
                    text.replace('samples=2', 'samples=2 samples=2'),
                    text.replace('seconds=0.004000 ms_per_frame=2.000000',
                                 'seconds=0.002000 ms_per_frame=1.000000')]
        for broken in variants:
            with self.subTest(broken=broken), self.assertRaises(ValueError):
                bench.parse_run(broken, 3, 1, ppu_profile=True)
        with self.assertRaises(ValueError):
            bench.parse_run(text, 3, 1)  # timing must not silently include PPU instrumentation

    def test_scanline_stage_partition_rejects_incomplete_or_nested_totals(self):
        names = ['backgrounds', 'objects', 'composition', 'ui_capture', 'ui_setup', 'scanline_other']
        rows = []
        for name, value in zip(names, [0.05] * 5 + [0.25]):
            rows.append(f'FFTA_PPU_STAGE phase={name} samples=2 omitted=0 p50_ms={value} '
                        f'p95_ms={value} p99_ms={value} max_ms={value} over_budget=0 '
                        f'over_double_budget=0 total_ms={2 * value} mean_ms={value}')
        text = '\n'.join(rows)
        self.assertEqual(set(bench.parse_scanline_stages(text, 2, 1.0)), set(names))
        self.assertIsNone(bench.parse_scanline_stages('', 2, 1.0))
        for broken in (text.rsplit('\n', 1)[0], text + '\n' + rows[0],
                       text.replace('samples=2', 'samples=1'), text.replace('phase=objects', 'phase=unknown')):
            with self.subTest(broken=broken), self.assertRaises(ValueError):
                bench.parse_scanline_stages(broken, 2, 1.0)
        for wrong_total in (0.8, 1.2):
            with self.assertRaises(ValueError):
                bench.parse_scanline_stages(text, 2, wrong_total)
        with self.assertRaises(ValueError):
            bench.parse_run(self.log(False) + '\n' + text, 3, 1)

    def test_snapshot_replay_preserves_guest_frame_origin_and_initial_identity(self):
        text = self.log().replace('frame=1 ', 'frame=101 ').replace('frame=2 ', 'frame=102 ').replace('frame=3 ', 'frame=103 ')
        text = text.replace('first=1 last=3', 'first=101 last=103').replace('frames: 3 ', 'frames: 103 ')
        initial = f'FFTA_REPLAY_STATE frame=100 machine_sha256={"d" * 64}'
        text = initial + '\n' + text
        reference = bench.parse_run(text, 3, 1, capture=True, snapshot=True)
        self.assertEqual(reference['start_frame'], 100)
        self.assertEqual(set(reference['hashes']), {101, 102, 103})
        self.assertEqual(reference['initial_state'], ('100', 'd' * 64))
        wrong = dict(reference, initial_state=('100', 'e' * 64))
        with self.assertRaises(ValueError):
            bench.assert_equivalent(reference, wrong)
        for broken in (text.replace('first=101', 'first=1'),
                       text.replace('frame=102 ', 'frame=2 '),
                       text.replace(initial + '\n', ''), text + '\n' + initial,
                       text.replace('STATE frame=100', 'STATE frame=99')):
            with self.subTest(broken=broken), self.assertRaises(ValueError):
                bench.parse_run(broken, 3, 1, capture=True, snapshot=True)
        with self.assertRaises(ValueError):
            bench.parse_run(text, 3, 1, capture=True)

    def test_snapshot_is_protected_from_output_and_conflicting_start_options(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            exe = root / 'recomp'; exe.write_bytes(b'\x7fELF')
            state = root / 'private.ffstate'; state.write_bytes(b'private snapshot')
            replay = root / 'idle.input'; replay.write_text('gba-input v1\n0 3ff\n')
            out = root / 'output'; out.mkdir()
            (out / 'candidate-equivalence.log').symlink_to(state)
            command = ['--baseline', str(exe), '--candidate', str(exe), '--rom', str(exe),
                       '--library', str(exe), '--out', str(out), '--state', str(state)]
            for options in (['--input', str(replay)], ['--demo'],
                            ['--input', str(replay), '--sav', str(state)]):
                with patch.object(bench.platform, 'system', return_value='Linux'), self.assertRaises(SystemExit):
                    bench.main(command + options)
            self.assertEqual(state.read_bytes(), b'private snapshot')
            self.assertFalse((out / 'summary.json').exists())

    def test_target_platform_uses_binary_format(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'recomp.exe'
            for magic, expected in [(b'\x7fELF', 'Linux'), (b'MZ00', 'Windows'),
                                    (b'\xcf\xfa\xed\xfe', 'Darwin')]:
                path.write_bytes(magic)
                self.assertEqual(bench.binary_platform(path), expected)

    def test_command_refuses_output_over_input_even_through_symlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            exe = root / 'recomp'; exe.write_bytes(b'\x7fELF')
            rom = root / 'private.gba'; rom.write_bytes(b'private fixture')
            out = root / 'output'; out.mkdir()
            (out / 'baseline-equivalence.log').symlink_to(rom)
            with patch.object(bench.platform, 'system', return_value='Linux'), self.assertRaises(SystemExit):
                bench.main(['--baseline', str(exe), '--candidate', str(exe), '--rom', str(rom),
                            '--library', str(exe), '--out', str(out), '--demo'])
            self.assertEqual(rom.read_bytes(), b'private fixture')
            self.assertFalse((out / 'summary.json').exists())

    def test_failure_replaces_stale_success_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            exe = root / 'recomp'; exe.write_bytes(b'\x7fELF')
            out = root / 'output'; out.mkdir()
            (out / 'summary.json').write_text('{"status":"passed"}')
            with patch.object(bench.platform, 'system', return_value='Linux'), \
                    patch.object(bench.subprocess, 'run', side_effect=ValueError('incomplete replay')), \
                    self.assertRaises(ValueError):
                bench.main(['--baseline', str(exe), '--candidate', str(exe), '--rom', str(exe),
                            '--library', str(exe), '--out', str(out), '--demo', '--frames', '3', '--warmup', '1'])
            self.assertIn('"status": "failed"', (out / 'summary.json').read_text())

    def test_decoded_mode_tracks_variant_with_same_binary_in_all_phases(self):
        for option, selected in [('--baseline-rom-data', 'baseline'), ('--candidate-rom-data', 'candidate')]:
            with self.subTest(option=option), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                exe = root / 'recomp'; exe.write_bytes(b'\x7fELF')
                calls = []

                def run(command, *, env, stdout, **kwargs):
                    label = Path(stdout.name).stem
                    calls.append((label, '--rom-data' in command))
                    stdout.write(self.log(env['FFTA_PATCH_VERIFY_FRAMES'] == '1'))
                    if env['FFTA_BENCH_FRAME_TIMES'] == '1':
                        stdout.write('\nFFTA_FRAME_TIMES samples=2 omitted=0 p50_ms=2 p95_ms=2 '
                                     'p99_ms=2 max_ms=2 over_budget=0 over_double_budget=0\n')

                with patch.object(bench.platform, 'system', return_value='Linux'), \
                        patch.object(bench.subprocess, 'run', side_effect=run):
                    summary = bench.main(['--baseline', str(exe), '--candidate', str(exe),
                                          '--rom', str(exe), '--library', str(exe),
                                          '--out', str(root / 'output'), '--demo', '--frames', '3',
                                          '--warmup', '1', '--runs', '2', '--profile', option])
                self.assertEqual(len(calls), 8)
                for label, enabled in calls:
                    self.assertEqual(enabled, label.startswith(selected + '-'))
                self.assertEqual(summary['decoded_resources'],
                                 {name: name == selected for name in ('baseline', 'candidate')})


if __name__ == '__main__':
    unittest.main()
