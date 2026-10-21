# TinyCC used by FFTARecomp

This folder accompanies the replaceable `tcc/libtcc.dll` (Windows x86_64)
or `tcc/libtcc.so` (Linux x86_64). No ROM, game-generated code or save is here.

TinyCC revision: `43c7708b85681a2fd4451c8a541af4494a8919b2`, the same source
revision as the Android port. `source.zip` contains complete upstream source;
its SHA256 is checked by `build.py`. See `../Licenses/TinyCC/COPYING` and source-file notices
for the LGPL terms. This folder must accompany the binary distribution, or
be offered as an equivalent source download from the same release location.

The upstream source is unchanged. `build.py` supplies the platform config,
generates the upstream predefined-header table and links `desktop_abi.c`.
That project-owned adapter exports `ffta_tcc_abi()` returning 1, to describe
the one-argument `tcc_relocate` interface. It is not a binary hash check.
Windows also links TinyCC's own `win32/lib/chkstk.S` stack-frame helper.

## Rebuild or replace

Developers need Python 3.9+ and GCC or Clang. Players need neither.

Windows: `python TinyCC/build.py --cc C:/path/to/gcc.exe`

Linux: `python3 TinyCC/build.py --platform linux`

Cross-compile Windows from Linux: use `--platform windows
--cc x86_64-w64-mingw32-gcc --host-cc gcc` and choose a Windows output DLL path
with `--output`. The build-time `c2str` helper must run on the build host;
`--host-cc` compiles that helper while `--cc` builds the target library.
The Windows Docker builder wires these flags automatically.

Copy the resulting library into `tcc/` beside `recomp`, or set
`FFTA_TCC_LIBRARY` to its absolute path. Compatible modified libraries are
accepted. Preserve the documented TinyCC API/ABI, the adapter entry point
and the Windows helper if building for Windows. Binary compatibility is
the modifier's responsibility. You may debug and reverse engineer the
integration to make a modified compatible library work.

`GBA_RECOMP_CC` explicitly selects an external compiler instead. Its cache
is separate; the usual GCC shared libraries still load through the OS.

Standalone notice texts are grouped in `../Licenses/TinyCC/`. Desktop bundles
carry that folder as `payload/Licenses/TinyCC/`; the source archive retains its
original upstream license files.
