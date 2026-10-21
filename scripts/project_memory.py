#!/usr/bin/env python3
"""Maintain evidence-based project change records using only the standard library."""
import argparse
import datetime
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile

KINDS = ('added', 'changed', 'fixed', 'performance', 'docs')
PREFIXES = ('crates/', 'AndroidPort/', 'scripts/', 'TinyCC/', 'android/', 'assets/', 'Installer/', 'tools/', '.github/workflows/', '.cursor/skills/')
ROOT_FILES = {'AGENTS.md', 'Cargo.toml', 'Cargo.lock', 'build.rs', 'build.sh', 'build.ps1', 'build-all.ps1'}


def safe_path(value):
    if not isinstance(value, str) or not value or '\\' in value:
        raise ValueError(f'Invalid repository path: {value!r}')
    p = PurePosixPath(value)
    if p.is_absolute() or '..' in p.parts or str(p) != value:
        raise ValueError(f'Invalid repository path: {value!r}')
    if p.parts[0] in ('target', 'dist', '.git') or p.suffix.lower() in ('.gba', '.sav', '.input', '.ffstate'):
        raise ValueError(f'Private/generated data cannot be recorded: {value}')
    return value


def validate(record, filename=None):
    required = {'schema', 'date', 'slug', 'title', 'kind', 'platforms', 'changes', 'validation', 'limitations', 'files', 'references'}
    if not isinstance(record, dict) or set(record) != required:
        raise ValueError('Record fields must be: ' + ', '.join(sorted(required)))
    if record['schema'] != 1 or record['kind'] not in KINDS:
        raise ValueError('Unsupported schema or kind')
    if datetime.date.fromisoformat(record['date']).isoformat() != record['date']:
        raise ValueError('Date must use YYYY-MM-DD')
    if not re.fullmatch(r'[a-z0-9]+(?:-[a-z0-9]+)*', record['slug']):
        raise ValueError('Slug must use lowercase words separated by hyphens')
    if not isinstance(record['title'], str) or not record['title'].strip() or '\n' in record['title']:
        raise ValueError('Title is required')
    for field in ('platforms', 'changes', 'validation', 'limitations', 'files', 'references'):
        values = record[field]
        if not isinstance(values, list) or any(not isinstance(v, str) or not v.strip() or '\n' in v for v in values):
            raise ValueError(f'{field} must contain nonempty single-line strings')
        if len(values) != len(set(values)):
            raise ValueError(f'Duplicate values in {field}')
    if not record['changes'] or not record['files']:
        raise ValueError('Changes and files are required')
    for value in record['files'] + record['references']:
        safe_path(value)
    expected = f"{record['date']}-{record['slug']}.json"
    if filename and filename != expected:
        raise ValueError(f'Record filename must be {expected}')
    return expected


def read_records(root):
    records = []
    for path in sorted((root / 'docs/changes').glob('*.json'), reverse=True):
        record = json.loads(path.read_text(encoding='utf-8'))
        validate(record, path.name)
        for ref in record['references']:
            if not (root / ref).is_file():
                raise ValueError(f'Missing reference: {ref}')
        records.append((path.relative_to(root).as_posix(), record))
    return records


def render(records):
    lines = ['# Changelog', '', 'Unreleased development changes. Generated from `docs/changes/*.json`.',
             'Edit the structured records, then run `python3 scripts/project_memory.py render`.', '']
    for path, r in records:
        lines += [f"## {r['date']} — {r['title']}", '', f"Type: {r['kind']}. Platforms: {', '.join(r['platforms']) or 'project-wide'}.", '']
        for field, label in (('changes', 'Changes'), ('validation', 'Validation'), ('limitations', 'Limitations')):
            if r[field]:
                lines += [f'**{label}**', ''] + [f'- {v}' for v in r[field]] + ['']
        if r['references']:
            lines += ['Documentation: ' + ', '.join(f'[{Path(p).name}]({p})' for p in r['references']) + '.', '']
        lines += [f'[Change record]({path})', '']
    return '\n'.join(lines)


def atomic_write(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8', newline='\n') as out:
            out.write(content)
        os.replace(temp, path)
    finally:
        if os.path.exists(temp):
            os.unlink(temp)


def git_paths(root, base):
    def run(*args):
        return subprocess.check_output(['git', '-C', str(root), *args]).decode().split('\0')
    return set(filter(None, run('diff', '--name-only', '--no-renames', '-z', base, '--') +
                      run('ls-files', '--others', '--exclude-standard', '-z')))


def check_coverage(root, records, base):
    changed = git_paths(root, base)
    covered = {p for name, r in records if name in changed for p in r['files']}
    missing = sorted(p for p in changed if (p.startswith(PREFIXES) or p in ROOT_FILES) and p not in covered and '__pycache__/' not in p)
    if missing:
        raise ValueError('Changed implementation files need a new or updated docs/changes record:\n' + '\n'.join(missing))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    sub = parser.add_subparsers(dest='command', required=True)
    rec = sub.add_parser('record', help='Add a factual record and regenerate the changelog; never overwrites')
    for field in ('date', 'slug', 'title'):
        rec.add_argument('--' + field, required=True)
    rec.add_argument('--kind', choices=KINDS, required=True)
    for flag in ('platform', 'change', 'validation', 'limitation', 'file', 'reference'):
        rec.add_argument('--' + flag, action='append', default=[])
    ren = sub.add_parser('render')
    ren.add_argument('--check', action='store_true')
    check = sub.add_parser('check')
    check.add_argument('--base', default='HEAD')
    check.add_argument('--metadata-only', action='store_true', help='Validate records and generated output without Git coverage')
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        records = read_records(root)
        if args.command == 'record':
            r = dict(schema=1, date=args.date, slug=args.slug, title=args.title, kind=args.kind,
                     platforms=args.platform, changes=args.change, validation=args.validation,
                     limitations=args.limitation, files=args.file, references=args.reference)
            name = validate(r)
            path = root / 'docs/changes' / name
            if path.exists():
                raise ValueError(f'Record already exists: {path}; edit it explicitly')
            for ref in r['references']:
                if not (root / ref).is_file():
                    raise ValueError(f'Missing reference: {ref}')
            atomic_write(path, json.dumps(r, indent=2, ensure_ascii=False) + '\n')
            records = read_records(root)
        output = render(records)
        destination = root / 'CHANGELOG.md'
        if args.command == 'check' or (args.command == 'render' and args.check):
            if not destination.exists() or destination.read_text(encoding='utf-8') != output:
                raise ValueError('CHANGELOG.md is stale; run python3 scripts/project_memory.py render')
            if args.command == 'check' and not args.metadata_only:
                check_coverage(root, records, args.base)
        else:
            atomic_write(destination, output)
        print('Project memory: OK')
        return 0
    except (ValueError, TypeError, KeyError, OSError, subprocess.CalledProcessError) as exc:
        parser.exit(1, f'Project memory: {exc}\n')


if __name__ == '__main__':
    raise SystemExit(main())
