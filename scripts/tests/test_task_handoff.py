import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('handoff', Path(__file__).parents[1] / 'task_handoff.py')
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)


def handoff(ids=('TASK-ONE', 'TASK-TWO'), legacy=False):
    text = '# Task handoff\n\n'
    sections = h.LEGACY_SECTIONS if legacy else h.SECTIONS
    for section in sections:
        text += '## ' + section + '\n\n'
        if section == sections[1]:
            for task in ids:
                if legacy:
                    text += (f'### {task}\n\n- Stato: sospeso\n- Esito: parziale\n'
                             '- Verificato: accepted scope\n- Manca: missing scope\n- Ripresa: explicit request\n\n')
                else:
                    text += (f'### {task}\n\n- State: suspended\n- Outcome: partial\n'
                             '- Verified: accepted scope\n- Missing: missing scope\n- Resume: explicit request\n\n')
        if section == sections[-1]:
            for field in h.LEGACY_FINAL_FIELDS if legacy else h.FINAL_FIELDS:
                text += '- ' + field + ': recorded\n'
    return text.encode()


class HandoffTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        (self.root / 'docs/development').mkdir(parents=True)
        legacy = b'original history\n'
        self.archive = h.BEGIN + legacy + h.END + b'\n'
        manifest = dict(original_bytes=len(legacy), original_sha256=h.digest(legacy),
                        sections=[dict(start_byte=0, end_byte=len(legacy), sha256=h.digest(legacy))],
                        closed_outcomes=[])
        (self.root / h.MANIFEST).write_text(json.dumps(manifest))
        (self.root / h.ARCHIVE).write_bytes(self.archive)
        (self.root / h.HANDOFF).write_bytes(handoff())

    def publish(self, new=None, archive=None, expected_handoff=None, expected_archive=None):
        h.publish(self.root, handoff() if new is None else new,
                  self.archive if archive is None else archive,
                  h.digest(handoff()) if expected_handoff is None else expected_handoff,
                  h.digest(self.archive) if expected_archive is None else expected_archive)

    def test_bounded_final_handover_and_open_states(self):
        self.assertEqual(h.check(self.root)['tasks'], 2)
        for invalid in (handoff() + b'\n## Chronology\n', handoff() + b'\nObsolete next action\n',
                        handoff() + b'x' * 20480,
                        handoff(('TASK-ONE', 'TASK-ONE')),
                        handoff().replace(b'State: suspended', b'State: completed'),
                        handoff().replace(b'- Missing:', b'- Other:')):
            with self.subTest(invalid=invalid[:40]), self.assertRaises(ValueError):
                h.validate_handoff(invalid)

    def test_lossless_history_and_full_section_coverage(self):
        manifest = json.loads((self.root / h.MANIFEST).read_text())
        for invalid in (self.archive.replace(b'original', b'changed!'), self.archive[:-10]):
            with self.assertRaises(ValueError):
                h.validate_archive(invalid, manifest)
        manifest['sections'][0]['start_byte'] = 1
        with self.assertRaises(ValueError):
            h.validate_archive(self.archive, manifest)

    def test_stale_preimage_for_either_file_refuses_without_mutation(self):
        for args in ({'expected_handoff': 'stale'}, {'expected_archive': 'stale'}):
            with self.assertRaises(ValueError):
                self.publish(**args)
            self.assertEqual((self.root / h.HANDOFF).read_bytes(), handoff())
            self.assertEqual((self.root / h.ARCHIVE).read_bytes(), self.archive)
        self.assertFalse((self.root / 'docs/development/.task-handoff.lock').exists())

    def test_another_writer_lock_is_preserved(self):
        lock = self.root / 'docs/development/.task-handoff.lock'
        lock.write_text('other owner')
        with self.assertRaises(FileExistsError):
            self.publish()
        self.assertEqual(lock.read_text(), 'other owner')

    def test_open_task_cannot_disappear_without_reviewed_closure(self):
        for tail in (b'', b'\n### TASK-ONE\n- Outcome: partial\n'):
            with self.assertRaises(ValueError):
                self.publish(new=handoff(('TASK-TWO',)), archive=self.archive + tail)
        self.assertEqual((self.root / h.HANDOFF).read_bytes(), handoff())

    def test_archive_is_append_only_and_closure_publishes(self):
        with self.assertRaises(ValueError):
            self.publish(archive=b'new preface\n' + self.archive)
        tail = (b'\n### TASK-ONE\n\n- Outcome: completed\n'
                b'- Verified criteria: acceptance checked\n- Evidence: docs/changes/example.json\n'
                b'- Limits: platform coverage recorded\n')
        self.publish(new=handoff(('TASK-TWO',)), archive=self.archive + tail)
        self.assertEqual(h.check(self.root)['tasks'], 1)
        self.assertEqual((self.root / h.ARCHIVE).read_bytes(), self.archive + tail)

    def test_failed_second_write_keeps_history_and_old_queue(self):
        real_write = h.atomic_write
        def fail_handoff(path, data):
            if path.name == 'TASK_HANDOFF.md':
                raise OSError('interrupted publication')
            real_write(path, data)
        with patch.object(h, 'atomic_write', side_effect=fail_handoff), self.assertRaises(OSError):
            self.publish(archive=self.archive + b'\nretained evidence\n')
        self.assertEqual((self.root / h.HANDOFF).read_bytes(), handoff())
        self.assertEqual((self.root / h.ARCHIVE).read_bytes(), self.archive + b'\nretained evidence\n')

    def test_closed_task_needs_concrete_reopening_reason(self):
        manifest = json.loads((self.root / h.MANIFEST).read_text())
        manifest['closed_outcomes'] = [{'id': 'TASK-ONE'}]
        (self.root / h.MANIFEST).write_text(json.dumps(manifest))
        with self.assertRaises(ValueError):
            h.check(self.root)
        (self.root / h.HANDOFF).write_bytes(handoff().replace(
            b'### TASK-ONE\n', b'### TASK-ONE\n- Reopening: new contradictory evidence\n'))
        self.assertEqual(h.check(self.root)['tasks'], 2)

    def test_legacy_queue_migrates_to_english_without_changing_tasks_or_history(self):
        legacy = handoff(legacy=True)
        (self.root / h.HANDOFF).write_bytes(legacy)
        self.assertEqual(h.validate_handoff(legacy), h.validate_handoff(handoff()))
        self.publish(expected_handoff=h.digest(legacy))
        self.assertEqual((self.root / h.HANDOFF).read_bytes(), handoff())
        self.assertEqual((self.root / h.ARCHIVE).read_bytes(), self.archive)

    def test_legacy_terminal_fields_and_reopening_remain_compatible(self):
        tail = (b'\n### TASK-ONE\n- Esito: completato\n- Criteri verificati: checked\n'
                b'- Prove: recorded\n- Limiti: unchanged\n')
        self.publish(new=handoff(('TASK-TWO',)), archive=self.archive + tail)
        reopened = handoff().replace(b'### TASK-ONE\n', b'### TASK-ONE\n- Riapertura: contradictory evidence\n')
        h.validate_queue_archive(reopened, self.archive + tail, {'closed_outcomes': []})

    def test_duplicate_legacy_and_english_fields_refuse_ambiguous_publication(self):
        for field in (b'- Stato: sospeso\n', b'- Verificato: duplicate\n'):
            with self.assertRaises(ValueError):
                h.validate_handoff(handoff().replace(b'- State: suspended\n', b'- State: suspended\n' + field))
        tail = (b'\n### TASK-ONE\n- Outcome: completed\n- Esito: parziale\n'
                b'- Verified criteria: checked\n- Evidence: recorded\n- Limits: unchanged\n')
        with self.assertRaises(ValueError):
            self.publish(new=handoff(('TASK-TWO',)), archive=self.archive + tail)


if __name__ == '__main__':
    unittest.main()
