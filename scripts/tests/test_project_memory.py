import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('memory', Path(__file__).parents[1] / 'project_memory.py')
memory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(memory)


class MemoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('config', 'user.name', 'Test fixture')
        self.git('commit', '--allow-empty', '-qm', 'Initial fixture')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args]).decode().strip()

    def command(self, *args):
        return memory.main(['--root', str(self.root), *args])

    def record(self, *extra):
        return self.command('record', '--date', '2026-10-20', '--slug', 'test-change',
                            '--title', 'Test change', '--kind', 'fixed', '--change', 'Correct behavior',
                            '--file', 'crates/example.rs', *extra)

    def test_record_render_and_stale_detection(self):
        self.record()
        before = (self.root / 'CHANGELOG.md').read_bytes()
        self.command('render')
        self.assertEqual(before, (self.root / 'CHANGELOG.md').read_bytes())
        self.command('check', '--metadata-only')
        (self.root / 'CHANGELOG.md').write_text('stale')
        with self.assertRaises(SystemExit):
            self.command('check', '--metadata-only')

    def test_no_overwrite_or_unsafe_paths(self):
        self.record()
        with self.assertRaises(SystemExit):
            self.record()
        for path in ('../secret', '/tmp/file', 'target/output.rs', 'rom/private.gba', 'slot.ffstate', 'foo/./bar', 'foo\\bar'):
            with self.subTest(path=path), self.assertRaises(ValueError):
                memory.safe_path(path)

    def test_coverage_requires_changed_record_and_supports_ranges(self):
        self.record()
        self.git('add', '.')
        self.git('commit', '-qm', 'Record old behavior')
        base = self.git('rev-parse', 'HEAD')
        (self.root / 'crates').mkdir()
        (self.root / 'crates/example.rs').write_text('changed')
        with self.assertRaises(ValueError):
            memory.check_coverage(self.root, memory.read_records(self.root), 'HEAD')
        path = next((self.root / 'docs/changes').glob('*.json'))
        r = json.loads(path.read_text())
        r['changes'].append('New correction verified')
        path.write_text(json.dumps(r))
        self.command('render')
        self.command('check')
        self.git('add', '.')
        self.git('commit', '-qm', 'Implement correction')
        self.command('check', '--base', base)

    def test_invalid_date_and_missing_reference_leave_no_record(self):
        with self.assertRaises(SystemExit):
            self.record('--reference', 'docs/missing.md')
        self.assertFalse((self.root / 'docs/changes').exists())
        with self.assertRaises(SystemExit):
            self.record('--date', '2026-02-30')
        with self.assertRaises(ValueError):
            memory.validate(dict(schema=1))

    def test_deleted_files_need_coverage_and_doc_only_edits_do_not(self):
        self.record()
        (self.root / 'crates').mkdir()
        source = self.root / 'crates/example.rs'
        source.write_text('old')
        self.git('add', '.')
        self.git('commit', '-qm', 'Existing implementation')
        (self.root / 'README.md').write_text('Documentation only')
        self.command('check')
        source.unlink()
        with self.assertRaises(ValueError):
            memory.check_coverage(self.root, memory.read_records(self.root), 'HEAD')
        path = next((self.root / 'docs/changes').glob('*.json'))
        r = json.loads(path.read_text())
        r['changes'] = ['Remove the obsolete implementation']
        path.write_text(json.dumps(r))
        self.command('render')
        self.git('add', '.')
        self.command('check')


if __name__ == '__main__':
    unittest.main()
