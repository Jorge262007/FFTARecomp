import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import types
import unittest
from unittest import mock
import zipfile

ROOT = Path(__file__).resolve().parents[2]


def module(path):
    spec = importlib.util.spec_from_file_location('android_build_test', path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


class AndroidDockerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='ffta android build ')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.driver = module(ROOT / 'AndroidPort/scripts/build_android_docker.py')

    def project(self, root=False):
        project = self.root / ('root tree' if root else 'standalone port')
        builder = 'scripts/android/build_android.py' if root else 'scripts/build_android.py'
        for name in ['Cargo.toml', 'Cargo.lock', builder, 'crates/gba-android/Cargo.toml',
                     'crates/gba-android/src/lib.rs', 'android/app/build.gradle.kts',
                     'android/gradlew', 'android/gradle/wrapper/gradle-wrapper.jar']:
            path = project / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'public source\r\n')
        return project, builder

    def test_context_selects_each_tree_filters_private_generated_and_linked_files(self):
        for root in (False, True):
            with self.subTest(root=root):
                project, builder = self.project(root)
                private = ['android/local.properties', 'android/app/build/old.apk',
                           'android/app/src/main/jniLibs/arm64-v8a/stale.so',
                           'android/app/src/main/assets/game.gba', 'crates/test/personal.sav',
                           'crates/test/key.keystore', 'crates/test/.env',
                           'crates/test/target/generated.rs', 'android/app/src/main/assets/licenses/tinycc/source.zip']
                for name in private:
                    path = project / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text('private input')
                if os.name == 'posix':
                    (project / 'crates/test/linked.rs').symlink_to(project / builder)
                before = {p: p.read_bytes() for p in project.rglob('*') if p.is_file()}
                context = self.root / f'context-{root}'
                script, apk, scope, metadata = self.driver.stage(project, context)
                self.assertEqual(script, builder)
                self.assertEqual(scope, 'root' if root else 'androidport')
                self.assertEqual(apk.startswith('Installer/'), root)
                self.assertTrue(set(private).isdisjoint(metadata['files']))
                self.assertNotIn('crates/test/linked.rs', metadata['files'])
                self.assertEqual((context / 'project/android/gradlew').read_bytes(), b'public source\n')
                self.assertEqual(before, {p: p.read_bytes() for p in project.rglob('*') if p.is_file()})

    def test_failure_propagates_docker_status_keeps_logs_and_never_mounts_checkout(self):
        project, _ = self.project()
        logs = self.root / 'logs'
        commands = []
        def fail(command, log):
            commands.append(command)
            log.write('retained failure diagnostics\n')
            return 17
        with mock.patch.object(self.driver.shutil, 'which', return_value='/fake/docker'), \
             mock.patch.object(self.driver, 'stream', side_effect=fail), \
             contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            status = self.driver.main(['--project-dir', str(project), '--output', str(self.root / 'export with spaces')], log_root=logs)
        self.assertEqual(status, 17)
        command = commands[0]
        self.assertNotIn('--mount', command)
        self.assertNotIn('--volume', command)
        self.assertNotEqual(command[-1], str(project))
        self.assertIn('retained failure diagnostics', next(logs.glob('*.log')).read_text())
        retained = list(logs.glob('*-context'))
        self.assertEqual(len(retained), 1)
        self.assertTrue((retained[0] / 'project/BUILD-SOURCE.json').is_file())

    def test_unsafe_output_and_missing_required_inputs_refuse_before_docker(self):
        for suffix in [',', '"', '\n', '\r']:
            with self.subTest(suffix=suffix), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as error:
                    self.driver.main(['--output', str(self.root / ('bad' + suffix)), '--dry-run'])
                self.assertEqual(error.exception.code, 2)
        project, _ = self.project()
        (project / 'android/gradle/wrapper/gradle-wrapper.jar').unlink()
        with self.assertRaisesRegex(ValueError, 'Missing public build input'):
            self.driver.stage(project, self.root / 'missing-context')

    def test_success_checks_actual_export_checksum(self):
        project, _ = self.project()
        output = self.root / 'export'
        def export(command, log):
            data = b'synthetic APK'
            (output / self.driver.APK_NAME).write_bytes(data)
            (output / 'SHA256.txt').write_text(hashlib.sha256(data).hexdigest() + '  ' + self.driver.APK_NAME)
            return 0
        with mock.patch.object(self.driver.shutil, 'which', return_value='/fake/docker'), \
             mock.patch.object(self.driver, 'stream', side_effect=export), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(self.driver.main(['--project-dir', str(project), '--output', str(output)], log_root=self.root / 'logs'), 0)

    def test_port_driver_still_works_after_copying_only_the_standalone_tree(self):
        context = self.root / 'copied-context'
        self.driver.stage(ROOT / 'AndroidPort', context)
        copied = context / 'project'
        shutil.copy2(ROOT / 'AndroidPort/Dockerfile.android', copied / 'Dockerfile.android')
        for name in ['build_android_docker.py', 'export_android.py']:
            shutil.copy2(ROOT / 'AndroidPort/scripts' / name, copied / 'scripts' / name)
        result = subprocess.run([sys.executable, str(copied / 'scripts/build_android_docker.py'), '--dry-run'],
                                cwd=self.root, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(str(copied), result.stdout)
        self.assertFalse((copied / 'dist').exists())


class NativeAndroidBuilderTests(unittest.TestCase):
    def test_windows_and_linux_commands_and_licensed_outputs_for_both_trees(self):
        for source in ['scripts/android/build_android.py', 'AndroidPort/scripts/build_android.py']:
            for windows in (False, True):
                with self.subTest(source=source, windows=windows), tempfile.TemporaryDirectory(prefix='ffta native builder ') as temporary:
                    root = Path(temporary)
                    builder = module(ROOT / source)
                    builder.ROOT = root
                    builder.os = types.SimpleNamespace(name='nt' if windows else 'posix', environ={})
                    builder.sys = types.SimpleNamespace(platform='win32' if windows else 'linux')
                    sdk = root / 'sdk'
                    host = 'windows-x86_64' if windows else 'linux-x86_64'
                    (sdk / 'ndk/28.2.13676358/toolchains/llvm/prebuilt' / host / 'bin').mkdir(parents=True)
                    cache = root / 'target/android-tcc'
                    cache.mkdir(parents=True)
                    archive = cache / 'source.zip'
                    with zipfile.ZipFile(archive, 'w') as files:
                        for name in ['COPYING', 'conftest.c', 'include/tccdefs.h', 'libtcc.c']:
                            files.writestr(f'tinycc-{builder.REV}/{name}', 'synthetic source')
                    builder.ARCHIVE_SHA256 = hashlib.sha256(archive.read_bytes()).hexdigest()
                    commands = []
                    def run(args, **kwargs):
                        args = [str(a) for a in args]
                        commands.append((args, kwargs))
                        if '-shared' in args:
                            Path(args[args.index('-o') + 1]).write_bytes(b'tcc')
                        if args[0] == 'cargo':
                            native = root / 'target/aarch64-linux-android/release/libgba_android.so'
                            native.parent.mkdir(parents=True)
                            native.write_bytes(b'rust native')
                        if ':app:assembleDebug' in args:
                            apk = root / 'android/app/build/outputs/apk/debug/app-debug.apk'
                            apk.parent.mkdir(parents=True)
                            apk.write_bytes(b'APK')
                    builder.run = run
                    with mock.patch.object(builder.shutil, 'which', return_value='gcc'), contextlib.redirect_stdout(io.StringIO()):
                        builder.build(sdk, '28.2.13676358')
                    clang = commands[2][0][0]
                    self.assertTrue(clang.endswith('clang.exe' if windows else 'clang'))
                    linker = commands[3][1]['env']['CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER']
                    self.assertTrue(linker.endswith('-clang.cmd' if windows else '-clang'))
                    gradle = commands[-1][0]
                    self.assertEqual(gradle[0].endswith('gradlew.bat'), windows)
                    self.assertIn(':app:testDebugUnitTest', gradle)
                    self.assertIn('--no-daemon', gradle)
                    self.assertTrue((root / 'android/app/src/main/assets/licenses/tinycc/source.zip').is_file())
                    destination = 'APK' if source.startswith('AndroidPort/') else 'Installer/Android'
                    self.assertEqual((root / destination / 'FFTARecomp-android-arm64-preview.apk').read_bytes(), b'APK')


class ApkExportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='ffta APK ')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.exporter = module(ROOT / 'AndroidPort/scripts/export_android.py')

    def apk(self, extra=None, machine=183):
        header = bytearray(64)
        header[:6] = b'\x7fELF\x02\x01'
        header[18:20] = machine.to_bytes(2, 'little')
        path = self.root / 'test.apk'
        with zipfile.ZipFile(path, 'w') as apk:
            for name in ['AndroidManifest.xml', 'classes.dex', 'assets/licenses/tinycc/source.zip',
                         'assets/licenses/tinycc/COPYING', 'assets/licenses/tinycc/config.h',
                         'assets/licenses/tinycc/build_android.py']:
                apk.writestr(name, 'fixture')
            for name in ['libgba_android.so', 'libtcc.so']:
                apk.writestr('lib/arm64-v8a/' + name, header)
            if extra:
                apk.writestr(extra, 'private')
        return path

    def test_arm64_and_licenses_are_required_and_private_payload_refuses(self):
        self.exporter.verify_apk(self.apk())
        for extra in ['assets/game.gba', 'assets/progress.sav', 'lib/x86/old.so']:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                self.exporter.verify_apk(self.apk(extra=extra))
        with self.assertRaisesRegex(ValueError, 'ARM64'):
            self.exporter.verify_apk(self.apk(machine=62))


if __name__ == '__main__':
    unittest.main()
