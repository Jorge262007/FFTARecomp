#!/usr/bin/env python3
"""Build the private offline root homepage from existing local index pages."""
import argparse
from datetime import datetime, timezone
import html
from html.parser import HTMLParser
import os
from pathlib import Path
import re
import sys
import tempfile
from urllib.parse import quote, unquote

SKIP_DIRS = {'.git', '.hg', '.svn', 'node_modules', '__pycache__', '.gradle', '.cxx'}
MAX_ENTRIES = 1_000_000
MAX_REPORTS = 5_000
GROUPS = {
    'coverage': 'ROM coverage',
    'graphics': 'Graphics inspector',
    'ui': 'Menu, text and input',
    'audio': 'Audio diagnostics',
    'code': 'Code and fallback',
    'issues': 'Issue reports',
    'gameplay': 'Tasks, scripts and units',
    'artifacts': 'Sessions and artifacts',
    'recent': 'Recent events',
    'campaigns': 'Targeted campaigns',
    'comparisons': 'Comparisons',
    'other': 'Other indexes',
}


class Title(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.active = False
        self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag == 'title':
            self.active = True

    def handle_endtag(self, tag):
        if tag == 'title':
            self.active = False

    def handle_data(self, data):
        if self.active:
            self.parts.append(data)


def discover(root):
    """Do not follow links or open ROM/save/JSON payloads; bound title reads."""
    reports = []
    pending = [root]
    visited = 0
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                visited += 1
                if visited > MAX_ENTRIES:
                    raise ValueError('Scan exceeds 1,000,000 entries; previous homepage preserved.')
                if entry.is_symlink():
                    continue
                if entry.is_dir(follow_symlinks=False):
                    if entry.name not in SKIP_DIRS:
                        pending.append(Path(entry.path))
                    continue
                if entry.name.casefold() not in ('index.html', 'index.htm') or not entry.is_file(follow_symlinks=False):
                    continue
                path = Path(entry.path)
                if path == root / 'index.html':
                    continue
                relative = path.relative_to(root).as_posix()
                parts = Path(relative).parts
                parser = Title()
                with path.open('rb') as source:
                    parser.feed(source.read(65_536).decode('utf-8', errors='replace'))
                title = ' '.join(''.join(parser.parts).split())[:240] or 'HTML index'
                group = 'other'
                for sidecar, kind in [('coverage.json', 'coverage'), ('graphics.json', 'graphics'), ('ui.json', 'ui'), ('audio.json', 'audio'), ('code.json', 'code'), ('gameplay.json', 'gameplay'), ('recent.json', 'recent'), ('campaign.json', 'campaigns'), ('issue.json', 'issues'), ('plan.json', 'artifacts'), ('comparison.json', 'comparisons')]:
                    if (path.parent / sidecar).is_file():
                        group = kind
                        break
                if relative == 'docs/development/rom-coverage/index.html':
                    name = 'Primary ROM coverage map'
                elif group in ('graphics', 'ui', 'audio', 'code', 'gameplay', 'recent', 'issues') and 'Data' in parts and parts.index('Data') > 0:
                    name = parts[parts.index('Data') - 1]
                elif group == 'comparisons' and path.parent.parent.name == 'comparisons':
                    name = path.parent.parent.parent.name
                else:
                    name = path.parent.name
                reports.append(dict(path=relative, href=quote(relative, safe='/'), title=title,
                                    name=name, group=group, mtime=entry.stat(follow_symlinks=False).st_mtime))
                if len(reports) > MAX_REPORTS:
                    raise ValueError('More than 5,000 index pages; previous homepage preserved.')
    return sorted(reports, key=lambda item: (-item['mtime'], item['path']))


def stamp(seconds):
    return datetime.fromtimestamp(seconds, timezone.utc).isoformat(timespec='seconds')


def render(reports, template, root=None):
    # A portable bundle intentionally omits repository-only evidence/state.
    # Show guide links only when their public document exists at this root.
    if root is not None:
        def guides(match):
            body = re.sub(r'<a href="([^"]+)">.*?</a>',
                          lambda a: a.group(0) if (root / unquote(a.group(1))).is_file() else '',
                          match.group(2))
            return match.group(1) + body + match.group(3)
        template = re.sub(r'(<div class="guides">)(.*?)(</div>)', guides, template, count=1, flags=re.S)

    esc = html.escape
    shortcuts = []
    for group in ('coverage', 'graphics', 'comparisons'):
        choices = [item for item in reports if item['group'] == group]
        if not choices:
            continue
        selected = next((item for item in choices if item['path'] == 'docs/development/rom-coverage/index.html'), choices[0])
        qualifier = 'Primary coverage map' if selected['path'] == 'docs/development/rom-coverage/index.html' else 'Newest file'
        shortcuts.append(f'<a class="shortcut" href="{esc(selected["href"])}"><span>{GROUPS[group]}</span>'
                         f'<strong>{esc(selected["name"])}</strong><small>{qualifier} <span aria-hidden="true">↗</span></small></a>')
    sections = []
    filters = [f'<button type="button" class="filter active" data-filter="all" aria-pressed="true">All <span>{len(reports)}</span></button>']
    for group, label in GROUPS.items():
        items = [item for item in reports if item['group'] == group]
        if not items:
            continue
        filters.append(f'<button type="button" class="filter" data-filter="{group}" aria-pressed="false">{label} <span>{len(items)}</span></button>')
        rows = []
        for item in items:
            date = stamp(item['mtime'])
            rows.append(f'<a class="report" href="{esc(item["href"])}">'
                        f'<div class="report-name"><strong>{esc(item["name"])}</strong><span>{esc(item["title"])}</span></div>'
                        f'<code>{esc(item["path"])}</code><time datetime="{date}">{date[:10]}</time>'
                        '<span class="arrow" aria-hidden="true">↗</span></a>')
        sections.append(f'<section class="report-group" data-group="{group}" aria-labelledby="heading-{group}">'
                        f'<h3 id="heading-{group}">{label} <span>{len(items)}</span></h3>{"".join(rows)}</section>')
    if not sections:
        sections.append('<p class="empty">No index found. Generate a report, then refresh this homepage.</p>')
    values = {'@@SHORTCUTS@@': ''.join(shortcuts), '@@FILTERS@@': ''.join(filters),
              '@@REPORTS@@': ''.join(sections), '@@COUNT@@': str(len(reports)),
              '@@UPDATED@@': datetime.now(timezone.utc).isoformat(timespec='seconds')}
    for key in values:
        if template.count(key) != 1:
            raise ValueError(f'Template requires exactly one {key}')
    # Match only the original template, never tokens inside discovered titles/paths.
    return re.sub(r'@@(?:SHORTCUTS|FILTERS|REPORTS|COUNT|UPDATED)@@',
                  lambda match: values[match.group()], template)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--stdout', action='store_true', help='Emit HTML for a readonly Docker mount; host publishes the page')
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    try:
        template = Path(__file__).with_suffix('.html').read_text(encoding='utf-8')
        output = render(discover(root), template, root)
        if args.stdout:
            sys.stdout.buffer.write(output.encode('utf-8'))
        else:
            fd, name = tempfile.mkstemp(prefix='.report-home-', suffix='.tmp', dir=root)
            try:
                with os.fdopen(fd, 'w', encoding='utf-8', newline='\n') as destination:
                    destination.write(output)
                os.replace(name, root / 'index.html')
            finally:
                if os.path.exists(name):
                    os.unlink(name)
            print(f'Homepage: {root / "index.html"}')
        return 0
    except (OSError, ValueError) as error:
        print(f'Homepage: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
