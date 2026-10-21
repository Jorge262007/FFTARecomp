"""The distributable must never inherit private data beside selected inputs."""
import importlib.util
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

SPEC = importlib.util.spec_from_file_location(
    'portable_packager', Path(__file__).resolve().parents[1] / 'package_ffta_portable.py')
package = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(package)


class PortableResourceTests(unittest.TestCase):
    def test_both_archives_include_resource_help_and_exclude_private_neighbors(self):
        for platform in ('linux', 'windows'):
            with self.subTest(platform=platform), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                runtime = root / 'runtime'
                selected = ['ffta-play', 'recomp', 'tcc/libtcc.so'] if platform == 'linux' else [
                    'ffta-play.exe', 'recomp.exe', 'tcc/libtcc.dll']
                for name in selected:
                    path = runtime / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(('synthetic distributable: ' + name).encode())
                for name in ['gamedb.sqlite', 'Licenses/notice.txt'] + [
                        'TinyCC/' + name for name in
                        ('COPYING', 'README.md', 'build.py', 'desktop_abi.c', 'libtcc.h', 'source.zip')] + [
                        'docs/' + name + '.md' for name in
                        ('portable', 'docker', 'fps_overlay', 'menu_shortcuts', 'save_states',
                         'save_backups', 'rom_data', 'rom_data_runtime')]:
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text('synthetic package document: ' + name)
                # Markers exercise accidental recursive-copy regressions at
                # both roots. No ROM, extracted asset or save bytes are used.
                marker = b'PRIVATE-FIXTURE-MUST-STAY-OUT'
                for parent in (root, runtime):
                    for name in ('ffta.gba', 'personal.sav', 'scene.ffstate', 'replay.input',
                                 'out/generated.c', 'Data/resources/source/v1/generation/manifest.toml',
                                 'Data/resources/source/v1/generation/obj-dimensions.toml',
                                 'Data/resources/source/v1/e3-d2/generation/bg-descriptors.toml',
                                 'resources/bg-descriptors.toml',
                                 'Data/resources/source/v1/e4-d3/generation/bg-descriptors.json',
                                 'resources/bg-descriptors.json',
                                 'Data/resources/source/v1/e5-d4/generation/bg-base-palettes.json',
                                 'resources/bg-base-palettes.json',
                                 'resources/obj-dimensions.toml', 'Data/cache/native.so'):
                        path = parent / name
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_bytes(marker)
                output = root / ('result.tar.gz' if platform == 'linux' else 'result.zip')
                with patch.object(package, 'ROOT', root), patch.object(sys, 'argv', [
                        'package', '--platform', platform, '--runtime-dir', str(runtime),
                        '--output', str(output)]):
                    package.main()
                if platform == 'linux':
                    with tarfile.open(output) as archive:
                        contents = {item.name: archive.extractfile(item).read()
                                    for item in archive.getmembers() if item.isfile()}
                else:
                    with zipfile.ZipFile(output) as archive:
                        contents = {name: archive.read(name) for name in archive.namelist()
                                    if not name.endswith('/')}
                names = {name.removeprefix('FFTARecomp-portable/') for name in contents}
                self.assertIn('docs/rom_data.md', names)
                self.assertIn('docs/rom_data_runtime.md', names)
                self.assertFalse(any(name.startswith(('Data/', 'resources/', 'out/')) for name in names))
                self.assertFalse(any(name.endswith(('.gba', '.sav', '.ffstate', '.input')) for name in names))
                self.assertFalse(any(marker in content for content in contents.values()))
                binary = 'recomp' if platform == 'linux' else 'recomp.exe'
                self.assertEqual(contents['FFTARecomp-portable/' + binary], (runtime / binary).read_bytes())


if __name__ == '__main__':
    unittest.main()
