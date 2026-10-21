"""Build the replaceable desktop libtcc, using Android's exact source revision.

Requires a host GCC/Clang (developers only). For cross-Linux on Windows,
pass --zig PATH. No compiler toolchain is needed on the player's machine.
"""
from pathlib import Path
import argparse
import hashlib
import io
import os
import shutil
import subprocess
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parent
REV = '43c7708b85681a2fd4451c8a541af4494a8919b2'
SHA256 = 'f660447b16d6e0efe27ba8a9122aa45ee950f84b574eeb8dd1e86bc8ad34c0ea'

def run(args):
    subprocess.run(list(map(str, args)), check=True)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', choices=['windows', 'linux'], default='windows' if os.name == 'nt' else 'linux')
    parser.add_argument('--cc', default=os.environ.get('CC') or shutil.which('gcc') or shutil.which('clang'))
    parser.add_argument('--host-cc', help='Compiler for the executable build helper (defaults to --cc)')
    parser.add_argument('--zig', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--cache', type=Path, default=HERE.parent / 'target/desktop-tcc')
    args = parser.parse_args()
    if not args.cc and os.name == 'nt':
        args.cc = next((str(p) for p in (Path('E:/msys64/mingw64/bin/gcc.exe'), Path('C:/msys64/mingw64/bin/gcc.exe')) if p.is_file()), None)
    if not args.cc:
        raise SystemExit('Building TinyCC requires a host GCC or Clang (--cc PATH)')
    archive = HERE / 'source.zip'
    if not archive.exists():
        previous = HERE.parent / 'target/android-tcc/source.zip'
        data = previous.read_bytes() if previous.exists() else urllib.request.urlopen(
            f'https://codeload.github.com/TinyCC/tinycc/zip/{REV}', timeout=60).read()
        if hashlib.sha256(data).hexdigest() != SHA256:
            raise SystemExit('TinyCC source checksum mismatch')
        archive.write_bytes(data)
    data = archive.read_bytes()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise SystemExit('TinyCC source checksum mismatch')
    cache = args.cache.resolve() / args.platform
    cache.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(data)) as files:
        for name in files.namelist():
            if not (cache / name).resolve().is_relative_to(cache):
                raise SystemExit('Invalid TinyCC archive path')
        files.extractall(cache)
    source = cache / f'tinycc-{REV}'
    config = '#define TCC_VERSION "0.9.28-43c7708"\n#define TCC_TARGET_X86_64 1\n'
    if args.platform == 'windows':
        config += '#define TCC_TARGET_PE 1\n'
    else:
        config += '#define CONFIG_TCC_ELFINTERP "/lib64/ld-linux-x86-64.so.2"\n'
    config += ('#define CONFIG_TCCDIR ""\n#define CONFIG_TCC_SYSINCLUDEPATHS ""\n'
               '#define CONFIG_TCC_LIBPATHS ""\n#define CONFIG_TCC_CRTPREFIX ""\n'
               '#define CONFIG_TCC_PREDEFS 1\n')
    (source / 'config.h').write_text(config)
    shutil.copy2(source / 'COPYING', HERE / 'COPYING')
    shutil.copy2(source / 'libtcc.h', HERE / 'libtcc.h')
    converter = cache / ('c2str.exe' if os.name == 'nt' else 'c2str')
    run([args.host_cc or args.cc, '-DC2STR', source / 'conftest.c', '-o', converter])
    run([converter, source / 'include/tccdefs.h', source / 'tccdefs_.h'])
    default_output = HERE.parent / 'target/release/tcc' / ('libtcc.dll' if args.platform == 'windows' else 'libtcc.so')
    output = (args.output or default_output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    compiler = [args.zig, 'cc', '-target', 'x86_64-linux-gnu.2.17'] if args.zig else [args.cc]
    if args.platform == 'windows':
        flags = ['-DLIBTCC_AS_DLL', '-static-libgcc', '-Wl,--export-all-symbols']
        extra = [source / 'win32/lib/chkstk.S']
    else:
        flags, extra = ['-fPIC'], ['-ldl', '-lm']
    run([*compiler, '-shared', '-O2', '-I' + str(source), *flags,
         source / 'libtcc.c', HERE / 'desktop_abi.c', *extra, '-o', output])
    print(f'TINYCC_BUILT platform={args.platform} bytes={output.stat().st_size} path={output}', flush=True)

if __name__ == '__main__':
    main()
