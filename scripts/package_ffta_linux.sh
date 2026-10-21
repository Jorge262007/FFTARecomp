#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $(uname -s) != Linux ]]; then
    printf 'This script requires Linux.\n' >&2
    exit 1
fi
cargo build --locked --release -p recomp -p ffta-play
python3 TinyCC/build.py --platform linux
python3 scripts/package_ffta_portable.py --platform linux
