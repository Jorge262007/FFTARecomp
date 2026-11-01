#!/usr/bin/env python3
"""Build the maintained AndroidPort preview using the shared Docker entry point."""
from pathlib import Path
import runpy

ROOT = Path(__file__).resolve().parents[2]

if __name__ == '__main__':
    driver = runpy.run_path(str(ROOT / 'AndroidPort/scripts/build_android_docker.py'))
    raise SystemExit(driver['main'](default_project=ROOT / 'AndroidPort',
                                   default_output=ROOT / 'dist/docker-android',
                                   log_root=ROOT / 'target/build-logs'))
