#!/usr/bin/env python3
"""Build the ARM64 preview with Docker/Buildx; no host SDK, Java or Rust needed."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = Path(__file__).resolve().parent
APK_NAME = 'FFTARecomp-android-arm64-preview.apk'
SKIP_DIRS = {'.git', '.gradle', '.kotlin', '.idea', '.agents', '.codex', '.aws',
             '__pycache__', 'target', 'build', 'Data', 'rom', 'ROMs', 'jniLibs', 'node_modules'}
SOURCE_SUFFIXES = {'.rs', '.toml', '.lock', '.py', '.c', '.h', '.inc', '.wgsl', '.tsv', '.json',
                   '.kt', '.kts', '.xml', '.properties', '.pro', '.png', '.webp', '.svg',
                   '.ttf', '.md', '.txt'}


def project_layout(project):
    if (project / 'scripts/build_android.py').is_file():
        return 'scripts/build_android.py', 'APK/' + APK_NAME, 'androidport'
    if (project / 'scripts/android/build_android.py').is_file():
        return 'scripts/android/build_android.py', 'Installer/Android/' + APK_NAME, 'root'
    raise ValueError(f'No supported Android builder under {project}')


def source_allowed(relative):
    if any(part in SKIP_DIRS or part.startswith('.') for part in relative.parts):
        return False
    if relative.name in {'local.properties', 'gba_bios.bin', 'shared-save-path.txt'}:
        return False
    # These files are generated from pinned source by the native builder.
    if relative.as_posix().startswith('android/app/src/main/assets/licenses/'):
        return False
    if relative.as_posix() == 'android/gradle/wrapper/gradle-wrapper.jar':
        return True
    return relative.suffix.lower() in SOURCE_SUFFIXES or relative.as_posix() == 'android/gradlew'


def stage(project, context):
    """Copy public working-tree sources only; never bind-mount or modify the checkout."""
    script, apk, scope = project_layout(project)
    output = context / 'project'
    output.mkdir(parents=True)
    paths = [project / 'Cargo.toml', project / 'Cargo.lock', project / script]
    for directory in ['crates', 'android', 'Licenses']:
        for parent, dirs, files in os.walk(project / directory, followlinks=False):
            dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS and not d.startswith('.') and not (Path(parent) / d).is_symlink())
            paths.extend(Path(parent) / name for name in sorted(files))
    entries = {}
    for path in paths:
        relative = path.relative_to(project)
        if path.is_symlink() or not path.is_file() or not source_allowed(relative):
            continue
        destination = output / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        data = path.read_bytes()
        if relative.as_posix() == 'android/gradlew':
            data = data.replace(b'\r\n', b'\n')
        destination.write_bytes(data)
        entries[relative.as_posix()] = hashlib.sha256(data).hexdigest()
    for required in ['Cargo.toml', 'Cargo.lock', script, 'crates/gba-android/Cargo.toml',
                     'android/app/build.gradle.kts', 'android/gradlew',
                     'android/gradle/wrapper/gradle-wrapper.jar']:
        if required not in entries:
            raise ValueError(f'Missing public build input: {project / required}')
    digest = hashlib.sha256(json.dumps(entries, sort_keys=True).encode()).hexdigest()
    metadata = {'project': scope, 'source_sha256': digest, 'files': entries}
    (output / 'BUILD-SOURCE.json').write_text(json.dumps(metadata, indent=2) + '\n', encoding='utf-8')
    shutil.copyfile(ROOT / 'Dockerfile.android', context / 'Dockerfile')
    shutil.copyfile(SCRIPTS / 'export_android.py', context / 'export_android.py')
    return script, apk, scope, metadata


def stream(command, log):
    with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, encoding='utf-8', errors='replace') as process:
        for line in process.stdout:
            print(line, end='', flush=True)
            log.write(line)
            log.flush()
        return process.wait()


def main(argv=None, *, default_project=None, default_output=None, log_root=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--project-dir', type=Path, default=default_project or ROOT,
                        help='Android source tree; defaults to this self-contained port')
    parser.add_argument('--output', type=Path, default=default_output or ROOT / 'dist/docker-android',
                        help='APK, checksums and build/test evidence destination')
    parser.add_argument('--dry-run', action='store_true', help='Validate public inputs and show the build command without Docker')
    args = parser.parse_args(argv)
    project = args.project_dir.resolve()
    output = args.output.resolve()
    if any(ch in str(output) for ch in ',"\r\n'):
        parser.error('output must not contain commas, double quotes or newlines (Buildx CSV syntax)')
    if not args.dry_run and not shutil.which('docker'):
        parser.error('Docker with Buildx is required')
    try:
        with tempfile.TemporaryDirectory(prefix='ffta-android-context-') as temporary:
            context = Path(temporary)
            script, apk, scope, metadata = stage(project, context)
            command = ['docker', 'buildx', 'build', '--platform', 'linux/amd64',
                       '--file', str(context / 'Dockerfile'), '--target', 'export',
                       '--build-arg', f'BUILD_SCRIPT={script}', '--build-arg', f'APK_PATH={apk}',
                       '--build-arg', f'CACHE_SCOPE={scope}', '--output', f'type=local,dest={output}',
                       '--progress', 'plain', str(context)]
            print(f"Android project: {project}\nPublic source files: {len(metadata['files'])}\nSource SHA256: {metadata['source_sha256']}", flush=True)
            if args.dry_run:
                print(json.dumps(command, indent=2))
                return 0
            output.mkdir(parents=True, exist_ok=True)
            logs = Path(log_root or project / 'target/build-logs')
            logs.mkdir(parents=True, exist_ok=True)
            timestamp = datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')
            with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', prefix=f'android-{timestamp}-',
                                             suffix='.log', dir=logs, delete=False) as log:
                print(f'Android Docker build log: {log.name}', flush=True)
                log.write(json.dumps({'command': command, 'source': metadata}) + '\n')
                status = stream(command, log)
                if status:
                    failed_context = logs / (Path(log.name).stem + '-context')
                    try:
                        shutil.copytree(context, failed_context)
                        log.write(f'Failed public build context retained: {failed_context}\n')
                        print(f'Failed public build context retained: {failed_context}', file=sys.stderr)
                    except OSError as error:
                        log.write(f'Could not retain failed context: {error}\n')
                        print(f'Could not retain failed context: {error}', file=sys.stderr)
                    print(f'Android build failed (exit {status}). Log retained: {log.name}', file=sys.stderr)
                    return status
            artifact = output / APK_NAME
            actual = hashlib.sha256(artifact.read_bytes()).hexdigest()
            expected = (output / 'SHA256.txt').read_text().split()[0]
            if actual != expected:
                raise ValueError('Exported APK checksum mismatch')
            print(f'APK: {artifact}\nSHA256: {actual}', flush=True)
            return 0
    except (OSError, ValueError) as error:
        print(f'Android Docker build: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
