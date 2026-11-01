#!/usr/bin/env python3
"""Export current project sources for a manual GitHub upload (never the .git history)."""
from pathlib import Path, PurePosixPath
import argparse
import hashlib
import json
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED_DIRS = {
    '.git', '.gradle', '.cxx', '.idea', '.vscode', '__pycache__',
    'target', 'build', 'Saves', 'APK', 'dist', 'out', 'stage', 'jniLibs',
}
EXCLUDED_SUFFIXES = {
    '.gba', '.sav', '.sps', '.state', '.bak', '.log', '.input', '.ppm',
    '.apk', '.exe', '.dll', '.so', '.dylib', '.o', '.a', '.bin', '.pyc',
    '.zip', '.gz', '.tar', '.7z', '.mp4', '.webm',
}

def include(name):
    path = PurePosixPath(name)
    if path.is_absolute() or '..' in path.parts or not path.parts:
        return False
    if any(part in EXCLUDED_DIRS for part in path.parts):
        return False
    if name.startswith(('Installer/payload/', 'Installer/Linux/', 'Installer/Android/',
                        'Installer/Licenses/', 'docs/bugs/')):
        return False
    if path.name in {'local.properties', '.env', 'av.cfg', 'input.cfg', 'shader-characters.cfg'}:
        return False
    if path.name.startswith(('ffta-input-diagnostic', 'ffta-input-diag')):
        return False
    if any(marker in path.name.lower() for marker in ('.sav.', '.autosave.', '.previews')):
        return False
    # Corresponding TinyCC source must accompany its LGPL distribution.
    if name == 'TinyCC/source.zip':
        return True
    return path.suffix.lower() not in EXCLUDED_SUFFIXES

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--name', default='FFTARecomp-source-clean')
    args = parser.parse_args()
    if not args.name or Path(args.name).name != args.name or args.name in {'.', '..'}:
        parser.error('--name must be a simple directory name')
    destination = ROOT / 'dist' / args.name
    archive = destination.with_suffix('.zip')
    if destination.exists() or archive.exists():
        parser.error(f'Output already exists: {destination}. Use a new --name.')
    raw = subprocess.check_output(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=ROOT
    )
    names = sorted(set(raw.decode('utf-8').split('\0')))
    candidates = [(name, ROOT / name) for name in names if include(name)]
    files = [(name, path) for name, path in candidates if path.is_file()]
    # Validate before writing: no symlinks outside the checkout and no renamed GBA dumps.
    for name, path in files:
        if path.is_symlink() or not path.resolve().is_relative_to(ROOT):
            raise RuntimeError(f'Unsafe source path: {name}')
        with path.open('rb') as stream:
            header = stream.read(192)
        if len(header) == 192 and header[4:16] == bytes.fromhex('24ffae51699aa2213d84820a'):
            raise RuntimeError(f'ROM content must not be published: {name}')
    required = {'Cargo.toml', 'Cargo.lock', 'TinyCC/source.zip', 'AndroidPort/Cargo.toml',
                'Licenses/LICENSE-MIT', 'Licenses/LICENSE-APACHE'}
    missing = required - {name for name, _ in files}
    if missing:
        raise RuntimeError(f'Missing source/license files: {sorted(missing)}')
    destination.mkdir(parents=True)
    manifest = []
    for name, path in files:
        copied = destination / name
        copied.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, copied)
        manifest.append({'path': name, 'sha256': hashlib.sha256(copied.read_bytes()).hexdigest()})
    with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
        for row in manifest:
            output.write(destination / row['path'], row['path'])
    (ROOT / 'dist' / (args.name + '-manifest.json')).write_text(
        json.dumps(manifest, indent=2), encoding='utf-8'
    )
    print(f'Folder: {destination}\nArchive: {archive}\nFiles: {len(manifest)}\nSize: {archive.stat().st_size:,} bytes')

if __name__ == '__main__':
    main()
