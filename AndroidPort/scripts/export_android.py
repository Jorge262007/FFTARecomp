"""Validate the ARM64 APK and export only the APK, checksum and build/test evidence."""
import hashlib
import json
from pathlib import Path
import shutil
import sys
import zipfile

APK_NAME = 'FFTARecomp-android-arm64-preview.apk'


def verify_apk(path):
    with zipfile.ZipFile(path) as apk:
        names = set(apk.namelist())
        required = {'AndroidManifest.xml', 'classes.dex',
                    'lib/arm64-v8a/libgba_android.so', 'lib/arm64-v8a/libtcc.so',
                    'assets/licenses/tinycc/source.zip', 'assets/licenses/tinycc/COPYING',
                    'assets/licenses/tinycc/config.h', 'assets/licenses/tinycc/build_android.py'}
        if not required <= names:
            raise ValueError(f'Incomplete APK: {sorted(required - names)}')
        for name in names:
            entry = Path(name)
            if entry.suffix.lower() in {'.gba', '.agb', '.sav', '.ffstate', '.input', '.keystore', '.jks'} or entry.name.lower() == 'gba_bios.bin':
                raise ValueError(f'Private data must not be packaged: {name}')
            if name.startswith('lib/') and not name.startswith('lib/arm64-v8a/'):
                raise ValueError(f'Unexpected ABI: {name}')
        for name in ['lib/arm64-v8a/libgba_android.so', 'lib/arm64-v8a/libtcc.so']:
            header = apk.read(name)[:64]
            if len(header) < 64 or header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != 183:
                raise ValueError(f'Not an ARM64 ELF library: {name}')
        damaged = apk.testzip()
        if damaged:
            raise ValueError(f'Corrupt APK entry: {damaged}')


def export(apk, destination):
    verify_apk(apk)
    destination.mkdir(parents=True, exist_ok=True)
    output = destination / APK_NAME
    shutil.copy2(apk, output)
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    (destination / 'SHA256.txt').write_text(f'{digest}  {APK_NAME}\n', encoding='utf-8')
    source = Path('BUILD-SOURCE.json')
    shutil.copy2(source, destination / source.name)
    metadata = json.loads(source.read_text())
    (destination / 'BUILD-INFO.txt').write_text(
        f"Android ARM64 preview (debug-signed)\nSource SHA256: {metadata['source_sha256']}\n"
        f"Kotlin unit tests required before export\nAPK SHA256: {digest}\n", encoding='utf-8')
    tests = Path('android/app/build/test-results/testDebugUnitTest')
    results = sorted(tests.glob('TEST-*.xml'))
    if not results:
        raise ValueError('No Kotlin unit-test results; refusing to report a verified build')
    (destination / 'tests').mkdir(exist_ok=True)
    for result in results:
        shutil.copy2(result, destination / 'tests' / result.name)


if __name__ == '__main__':
    export(Path(sys.argv[1]), Path(sys.argv[2]))
