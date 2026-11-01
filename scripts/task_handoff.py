#!/usr/bin/env python3
"""Check the bounded operational handoff or publish a guarded memory update."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile

HANDOFF = 'docs/development/TASK_HANDOFF.md'
ARCHIVE = 'docs/development/TASK_COMPLETED_ARCHIVE.md'
MANIFEST = 'docs/development/TASK_HANDOFF_MIGRATION.json'
SECTIONS = (
    'Stato operativo', 'Task in esecuzione o sospesi',
    'Task incompleti e criteri aperti', 'Blocchi, dipendenze e risorse condivise',
    'PASSAGGIO DI TESTIMONE',
)
STATES = {'attivo', 'sospeso', 'interrotto', 'fallito', 'in_attesa'}
OUTCOMES = {'parziale', 'non_iniziato', 'integrato_non_verificato'}
BEGIN = b'<!-- legacy-handoff-begin -->\n'
END = b'<!-- legacy-handoff-end -->'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def validate_handoff(data):
    if len(data) > 20 * 1024:
        raise ValueError('Operational handoff exceeds 20 KiB; compact closed details')
    text = data.decode('utf-8')
    if re.findall(r'^## (.+)$', text, re.M) != list(SECTIONS):
        raise ValueError('Handoff sections must follow the five-section contract; handover is last')
    final = text.split('## PASSAGGIO DI TESTIMONE\n', 1)[1]
    if re.search(r'^#{1,6} ', final, re.M):
        raise ValueError('No sections may follow the final handover')
    fields = ('Task', 'Ultimo risultato verificato', 'Da completare', 'Errori o impedimenti',
              'Prossima azione', 'Riferimenti')
    final_lines = [line for line in final.splitlines() if line.strip()]
    if len(final_lines) != len(fields) or any(
            not re.fullmatch(r'- ' + re.escape(field) + r': .+', line)
            for field, line in zip(fields, final_lines)):
        raise ValueError('Final handover must contain only the six ordered fields (one line each)')
    tasks = {}
    for block in text.split('\n### ')[1:]:
        # A task ends at either another task or the next main section.
        block = block.split('\n## ', 1)[0]
        task_id = block.splitlines()[0]
        if not re.fullmatch(r'[A-Z][A-Z0-9-]+', task_id) or task_id in tasks:
            raise ValueError('Invalid or duplicate task ID: ' + task_id)
        state = re.findall(r'^- Stato: (.+)$', block, re.M)
        outcome = re.findall(r'^- Esito: (.+)$', block, re.M)
        if len(state) != 1 or state[0] not in STATES or len(outcome) != 1 or outcome[0] not in OUTCOMES:
            raise ValueError('Task needs one explicit open state and outcome: ' + task_id)
        for field in ('Verificato', 'Manca', 'Ripresa'):
            if not re.search(r'^- ' + field + r': .+', block, re.M):
                raise ValueError(f'{task_id} needs {field}')
        tasks[task_id] = {'state': state[0], 'outcome': outcome[0]}
    if not tasks:
        raise ValueError('No operational tasks recorded')
    return tasks


def validate_archive(data, manifest):
    start = data.index(BEGIN) + len(BEGIN)
    end = start + manifest['original_bytes']
    payload = data[start:end]
    if data[end:end + len(END)] != END or digest(payload) != manifest['original_sha256']:
        raise ValueError('Original handoff history changed or was truncated')
    cursor = 0
    for section in manifest['sections']:
        if section['start_byte'] != cursor:
            raise ValueError('Historical section coverage has a gap or overlap')
        cursor = section['end_byte']
        if digest(payload[section['start_byte']:cursor]) != section['sha256']:
            raise ValueError('Historical section hash mismatch')
    if cursor != len(payload):
        raise ValueError('Historical section coverage is incomplete')


def validate_queue_archive(handoff, archive, manifest):
    tasks = validate_handoff(handoff)
    closed = {x['id'] for x in manifest['closed_outcomes']}
    appendix = archive.split(END, 1)[1].decode('utf-8')
    closed.update(re.findall(r'^### ([A-Z][A-Z0-9-]+)$', appendix, re.M))
    for task in set(tasks) & closed:
        block = handoff.decode('utf-8').split('\n### ' + task + '\n', 1)[1].split('\n##', 1)[0]
        if not re.search(r'^- Riapertura: \S.+', block, re.M):
            raise ValueError('Closed task requires a concrete reopening reason: ' + task)
    return tasks, closed


def check(root):
    handoff = (root / HANDOFF).read_bytes()
    manifest = json.loads((root / MANIFEST).read_text(encoding='utf-8'))
    archive = (root / ARCHIVE).read_bytes()
    validate_archive(archive, manifest)
    tasks, closed = validate_queue_archive(handoff, archive, manifest)
    return {'bytes': (root / HANDOFF).stat().st_size, 'tasks': len(tasks),
            'active': sum(t['state'] == 'attivo' for t in tasks.values()),
            'partial': sum(t['outcome'] in ('parziale', 'integrato_non_verificato') for t in tasks.values()),
            'archived_outcomes': len(closed)}


def atomic_write(path, data):
    fd, name = tempfile.mkstemp(prefix='.' + path.name + '-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def validate_transition(old_handoff, handoff, appended):
    old_tasks = validate_handoff(old_handoff)
    new_tasks = validate_handoff(handoff)
    history = appended.decode('utf-8')
    for task in old_tasks.keys() - new_tasks.keys():
        match = re.search(r'^### ' + re.escape(task) + r'\n(.*?)(?=^### |\Z)', history, re.M | re.S)
        if not match:
            raise ValueError('Removed task needs an appended terminal outcome: ' + task)
        block = match[1]
        if not re.search(r'^- Esito: (completato|respinto|chiuso)$', block, re.M):
            raise ValueError('Partial tasks must stay operational: ' + task)
        for field in ('Criteri verificati', 'Prove', 'Limiti'):
            if not re.search(r'^- ' + field + r': \S.+', block, re.M):
                raise ValueError('Terminal outcome needs ' + field + ': ' + task)


def publish(root, handoff, archive, expected_handoff, expected_archive):
    """Cooperative writers share a lock and both preimages; archive is append-only.

    Archive first means a crash can leave duplicate historical context but cannot
    remove it from both files. This does not lock editors that ignore the protocol.
    """
    validate_handoff(handoff)
    manifest = json.loads((root / MANIFEST).read_text(encoding='utf-8'))
    validate_archive(archive, manifest)
    validate_queue_archive(handoff, archive, manifest)
    lock = root / 'docs/development/.task-handoff.lock'
    fd = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        os.write(fd, f'pid={os.getpid()}\n'.encode())
        old_handoff = (root / HANDOFF).read_bytes()
        old_archive = (root / ARCHIVE).read_bytes()
        if digest(old_handoff) != expected_handoff or digest(old_archive) != expected_archive:
            raise ValueError('Memory changed since inspection; re-read and merge the newer update')
        if not archive.startswith(old_archive):
            raise ValueError('Archive updates must preserve the existing bytes as an exact prefix')
        validate_transition(old_handoff, handoff, archive[len(old_archive):])
        atomic_write(root / ARCHIVE, archive)
        if (root / HANDOFF).read_bytes() != old_handoff:
            raise ValueError('Handoff changed during publication; archive preserved, re-read and merge')
        atomic_write(root / HANDOFF, handoff)
    finally:
        os.close(fd)
        lock.unlink()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('check')
    update = sub.add_parser('update', help='Publish reviewed proposals; never decides task acceptance')
    update.add_argument('--handoff', type=Path, required=True)
    update.add_argument('--archive', type=Path, required=True)
    update.add_argument('--expected-handoff', required=True)
    update.add_argument('--expected-archive', required=True)
    args = parser.parse_args(argv)
    try:
        root = args.root.resolve()
        if args.command == 'update':
            publish(root, args.handoff.read_bytes(), args.archive.read_bytes(),
                    args.expected_handoff, args.expected_archive)
        print('Task handoff: ' + json.dumps(check(root), sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError, UnicodeError) as exc:
        parser.exit(1, f'Task handoff: {exc}\n')


if __name__ == '__main__':
    main()
