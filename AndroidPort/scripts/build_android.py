"""Build Android ARM64 Rust + separately loaded TinyCC, then package the APK.

Windows/Linux host: Android SDK/NDK, Rust aarch64-linux-android, Java, and a host
C compiler (gcc/clang) are required. All downloaded/generated code stays in target.
"""
from pathlib import Path
import argparse
import hashlib
import io
import os
import sys
import shutil
import subprocess
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
REV = '43c7708b85681a2fd4451c8a541af4494a8919b2'
ARCHIVE_SHA256 = 'f660447b16d6e0efe27ba8a9122aa45ee950f84b574eeb8dd1e86bc8ad34c0ea'


def run(args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, cwd=ROOT, **kwargs)


def build(sdk, ndk, native_only=False):
    sdk = Path(sdk).resolve()
    ndk_root = sdk / 'ndk' / ndk
    if os.name == 'nt':
        host, executable, launcher = 'windows-x86_64', '.exe', '.cmd'
    elif sys.platform.startswith('linux'):
        host, executable, launcher = 'linux-x86_64', '', ''
    else:
        raise RuntimeError('Native Android builds support Windows/Linux; use Docker on other hosts')
    bin_dir = ndk_root / f'toolchains/llvm/prebuilt/{host}/bin'
    if not bin_dir.is_dir():
        raise RuntimeError(f'NDK toolchain not found: {bin_dir}')
    cache = ROOT / 'target/android-tcc'
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / 'source.zip'
    if not archive.exists():
        print(f'Downloading TinyCC {REV}', flush=True)
        archive.write_bytes(urllib.request.urlopen(
            f'https://codeload.github.com/TinyCC/tinycc/zip/{REV}', timeout=60).read())
    data = archive.read_bytes()
    if hashlib.sha256(data).hexdigest() != ARCHIVE_SHA256:
        raise RuntimeError('TinyCC archive checksum mismatch; refusing to compile')
    source = cache / f'tinycc-{REV}'
    if not source.is_dir():
        with zipfile.ZipFile(io.BytesIO(data)) as files:
            for name in files.namelist():
                target = (cache / name).resolve()
                if not target.is_relative_to(cache.resolve()):
                    raise RuntimeError('Invalid archive path')
            files.extractall(cache)
    config = '''#define TCC_VERSION "0.9.28-43c7708"
#define TCC_TARGET_ARM64 1
#define TCC_TARGET_ANDROID 1
#define CONFIG_TCCDIR ""
#define CONFIG_TCC_SYSINCLUDEPATHS ""
#define CONFIG_TCC_LIBPATHS ""
#define CONFIG_TCC_CRTPREFIX ""
#define CONFIG_TCC_ELFINTERP "/system/bin/linker64"
#define CONFIG_TCC_PREDEFS 1
'''
    (source / 'config.h').write_text(config)
    host_cc = shutil.which('gcc') or shutil.which('clang')
    if not host_cc:
        raise RuntimeError('A host gcc or clang is required to build TinyCC c2str')
    run([host_cc, '-DC2STR', source / 'conftest.c', '-o', cache / ('c2str' + executable)])
    run([cache / ('c2str' + executable), source / 'include/tccdefs.h', source / 'tccdefs_.h'])
    libraries = ROOT / 'android/app/src/main/jniLibs/arm64-v8a'
    libraries.mkdir(parents=True, exist_ok=True)
    run([bin_dir / ('clang' + executable), '--target=aarch64-linux-android24', '-shared', '-fPIC', '-O2',
         '-Wl,-z,max-page-size=16384', '-Wl,-soname,libtcc.so', '-o', libraries / 'libtcc.so',
         source / 'libtcc.c', '-ldl', '-lm'])
    env = os.environ.copy()
    env['CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER'] = str(bin_dir / ('aarch64-linux-android24-clang' + launcher))
    run(['cargo', 'build', '--release', '--locked', '-p', 'gba-android',
         '--target', 'aarch64-linux-android'], env=env)
    native = ROOT / 'target/aarch64-linux-android/release/libgba_android.so'
    shutil.copy2(native, libraries / native.name)
    # Supply complete upstream source + build settings with the replaceable LGPL library.
    notices = ROOT / 'android/app/src/main/assets/licenses/tinycc'
    notices.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source / 'COPYING', notices / 'COPYING')
    shutil.copy2(archive, notices / 'source.zip')
    (notices / 'config.h').write_text(config)
    shutil.copy2(Path(__file__), notices / 'build_android.py')
    (notices / 'SOURCE.txt').write_text(
        f'TinyCC {REV}\nhttps://github.com/TinyCC/tinycc/tree/{REV}\n'
        'LGPL-2.1-or-later, dynamically loaded as libtcc.so.\n'
        'Unmodified upstream source is in source.zip; build settings are alongside it.\n')
    for name in ('LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY_NOTICES.md'):
        source_notice = ROOT / 'Licenses' / name
        if source_notice.is_file():
            shutil.copy2(source_notice, notices.parent / name)
    if native_only:
        return
    (ROOT / 'android/local.properties').write_text('sdk.dir=' + str(sdk).replace('\\', '/') + '\n')
    gradle = [ROOT / 'android/gradlew.bat'] if os.name == 'nt' else ['bash', ROOT / 'android/gradlew']
    run([*gradle, '-p', ROOT / 'android', ':app:assembleDebug',
         ':app:testDebugUnitTest', '--no-daemon', '--console=plain'], env=env)
    apk = ROOT / 'android/app/build/outputs/apk/debug/app-debug.apk'
    destination = ROOT / 'APK/FFTARecomp-android-arm64-preview.apk'
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(apk, destination)
    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    (destination.parent / 'SHA256.txt').write_text(digest + '  ' + destination.name + '\n', encoding='utf-8')
    print(f'APK: {destination}\nSHA256: {digest}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--sdk', default=os.environ.get('ANDROID_SDK_ROOT') or os.environ.get('ANDROID_HOME'))
    parser.add_argument('--ndk', default='28.2.13676358')
    parser.add_argument('--native-only', action='store_true')
    args = parser.parse_args()
    if not args.sdk:
        parser.error('--sdk or ANDROID_HOME/ANDROID_SDK_ROOT is required')
    build(args.sdk, args.ndk, args.native_only)
