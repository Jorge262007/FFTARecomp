#!/usr/bin/env python3
"""Package desktop portable runtimes, selecting only distributable files."""
import argparse
from pathlib import Path
import tempfile
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def main(default_platform="windows"):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', choices=('windows', 'linux'), default=default_platform)
    parser.add_argument('--runtime-dir', type=Path, default=ROOT / 'target/release')
    parser.add_argument('--output', type=Path, default=None)
    args = parser.parse_args()
    linux = args.platform == 'linux'
    extension = '.tar.gz' if linux else '.zip'
    if args.output is None:
        args.output = ROOT / ('dist/FFTARecomp-' + args.platform + '-portable' + extension)
    launcher = 'FFTARecomp' if linux else 'FFTARecomp.exe'
    runtime = 'recomp' if linux else 'recomp.exe'
    library = 'libtcc.so' if linux else 'libtcc.dll'
    files = {
        launcher: args.runtime_dir / ('ffta-play' if linux else 'ffta-play.exe'),
        runtime: args.runtime_dir / runtime,
        'tcc/' + library: args.runtime_dir / 'tcc' / library,
        'gamedb.sqlite': ROOT / 'gamedb.sqlite',
    }
    for name in ('COPYING', 'README.md', 'build.py', 'desktop_abi.c', 'libtcc.h', 'source.zip'):
        files['TinyCC/' + name] = ROOT / 'TinyCC' / name
    for path in (ROOT / 'Licenses').rglob('*'):
        if path.is_file():
            files[path.relative_to(ROOT).as_posix()] = path
    for name in ('portable', 'docker', 'fps_overlay', 'menu_shortcuts', 'save_states', 'save_backups', 'rom_data', 'rom_data_runtime'):
        files['docs/' + name + '.md'] = ROOT / 'docs' / (name + '.md')
    for name, source in files.items():
        if source.is_symlink() or not source.is_file():
            raise SystemExit(f'Missing regular package input: {source} ({name})')
    prefix = 'FFTARecomp-portable'
    readme = '''FFTARecomp portable - {platform} x86_64
Estrai tutta la cartella in una posizione scrivibile e avvia {launch}.
Al primo avvio scegli la tua ROM FFTA USA/Europa: viene copiata come ffta.gba.
Puoi anche copiarla manualmente accanto agli eseguibili prima di avviare.
Nessuna ROM o BIOS e incluso nel pacchetto. Nessuna installazione richiesta.

Tutti i dati dell'app restano nella cartella Data. Per spostare il gioco,
copia TUTTA la cartella, inclusi Data, ffta.gba e portable.txt, a gioco chiuso.
Per importare progressi: a gioco chiuso copia il tuo .sav in Data/saves/ffta.sav.
Se esiste gia, conservane prima una copia. Dettagli: docs/portable.md.
Menu: Esc oppure Start+Back/Select sul controller; combo configurabili in Input.
Savestate: menu Save states (9 slot con anteprima e annulla).
Backup del .sav: menu Save backups (le ultime 2 copie create all'avvio).
Per errori di avvio consulta play-error.log nella cartella principale.
'''.format(platform=args.platform, launch='./FFTARecomp' if linux else launcher)
    if linux:
        readme += '\nRichiede Ubuntu/Linux x86_64 con sessione grafica, ALSA e libudev.\n'
        readme += 'Non richiede Wine. Se il desktop chiede cosa fare, scegli Esegui.\n'
    generated = {
        'portable.txt': b'FFTARecomp portable mode. Keep beside both executables.\n',
        'LEGGIMI.txt': readme.encode('utf-8'),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    # Stage only selected inputs. Existing personal Data is never copied.
    with tempfile.TemporaryDirectory(dir=args.output.parent) as staging:
        directory = Path(staging) / prefix
        directory.mkdir()
        for name, source in files.items():
            dest = directory / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(source.read_bytes())
            dest.chmod(0o755 if name in (launcher, runtime) else 0o644)
        for name, content in generated.items():
            (directory / name).write_bytes(content)
        for name in ('config', 'cache', 'saves'):
            (directory / 'Data' / name).mkdir(parents=True)
        archive = Path(staging) / ('portable' + extension)
        if linux:
            with tarfile.open(archive, 'w:gz') as bundle:
                bundle.add(directory, arcname=prefix, filter=clean_tar_metadata)
            with tarfile.open(archive, 'r:gz') as bundle:
                for name, source in files.items():
                    if bundle.extractfile(prefix + '/' + name).read() != source.read_bytes():
                        raise SystemExit(f'Archive input mismatch: {name}')
                for name in (launcher, runtime):
                    if bundle.getmember(prefix + '/' + name).mode != 0o755:
                        raise SystemExit(f'Missing executable permission: {name}')
        else:
            with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
                for path in sorted(directory.rglob('*')):
                    bundle.write(path, prefix + '/' + path.relative_to(directory).as_posix())
            with zipfile.ZipFile(archive) as bundle:
                if bundle.testzip() is not None:
                    raise SystemExit('Archive CRC validation failed')
                for name, source in files.items():
                    if bundle.read(prefix + '/' + name) != source.read_bytes():
                        raise SystemExit(f'Archive input mismatch: {name}')
        archive.replace(args.output)
    print(f'Created {args.output} ({args.output.stat().st_size:,} bytes)')


def clean_tar_metadata(info):
    info.uid = info.gid = 0
    info.uname = info.gname = ''
    return info


if __name__ == '__main__':
    main()
