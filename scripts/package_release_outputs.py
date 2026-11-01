"""Package current builds into Installer, the canonical release output directory."""
from pathlib import Path
import argparse
import hashlib
import json
import shutil
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--skip-android', action='store_true')
    args = parser.parse_args()
    output = ROOT / 'Installer'
    payload = output / 'payload'
    for source, target in [('installer.exe', output / 'Installer.exe'),
                           ('recomp.exe', payload / 'recomp.exe'),
                           ('ffta-play.exe', payload / 'FFTARecomp.exe'),
                           ('tcc/libtcc.dll', payload / 'tcc/libtcc.dll')]:
        copy(ROOT / 'target/release' / source, target)
    copy(ROOT / 'gamedb.sqlite', payload / 'gamedb.sqlite')
    windows = output / 'FFTARecomp-windows-x86_64.zip'
    files = [output / 'Installer.exe', payload / 'recomp.exe', payload / 'FFTARecomp.exe',
             payload / 'gamedb.sqlite', payload / 'tcc/libtcc.dll']
    with zipfile.ZipFile(windows, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(files):
            if path.suffix.lower() in {'.gba', '.sav', '.sps', '.state'}:
                raise RuntimeError(f'Personal game data in payload: {path}')
            archive.write(path, path.relative_to(output).as_posix())
    linux = output / 'Linux/FFTAInstaller-linux-x86_64.tar.gz'
    deck = output / 'SteamDeck/FFTAInstaller-SteamDeck-linux-x86_64.tar.gz'
    copy(output / 'Linux' / deck.name, deck)
    artifacts = [windows, linux, deck]
    if not args.skip_android:
        apk = output / 'Android/FFTARecomp-android-arm64.apk'
        copy(ROOT / 'AndroidPort/android/app/build/outputs/apk/debug/app-debug.apk', apk)
        for target in [ROOT / 'AndroidPort/APK/FFTARecomp-android-arm64-preview.apk']:
            copy(apk, target)
            (target.parent / 'SHA256.txt').write_text(hashlib.sha256(target.read_bytes()).hexdigest() + '  ' + target.name + '\n', encoding='utf-8')
        (apk.parent / 'SHA256.txt').write_text(hashlib.sha256(apk.read_bytes()).hexdigest() + '  ' + apk.name + '\n', encoding='utf-8')
        artifacts.append(apk)
    rows = []
    for path in artifacts:
        rows.append({'file': path.relative_to(output).as_posix(), 'bytes': path.stat().st_size,
                     'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
    (output / 'SHA256SUMS.txt').write_text(''.join(row['sha256'] + '  ' + row['file'] + '\n' for row in rows), encoding='utf-8')
    (output / 'BUILD-MANIFEST.json').write_text(json.dumps({'version': '3.1', 'artifacts': rows}, indent=2), encoding='utf-8')
    print(json.dumps(rows, indent=2))

if __name__ == '__main__':
    main()
