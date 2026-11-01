#!/usr/bin/env bash
# The archive is private and never modifies the source saves or release ZIP.
set -euo pipefail
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
exec python3 "$repo_dir/scripts/export_debug_assets.py" "$@"
