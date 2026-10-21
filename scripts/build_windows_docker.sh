#!/usr/bin/env bash
# Build and export Windows artifacts without installing a host toolchain.
set -euo pipefail
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if (( $# > 1 )); then
    printf 'Usage: %s [output-directory]\n' "$0" >&2
    exit 2
fi
output_dir=${1:-"$repo_dir/dist/docker-windows"}
# Buildx parses --output as CSV. Do not silently truncate a path at a comma.
if [[ "$output_dir" == *','* || "$output_dir" == *$'\n'* ]]; then
    printf 'The output directory must not contain commas or newlines.\n' >&2
    exit 2
fi
command -v docker >/dev/null || { printf 'Docker with Buildx is required.\n' >&2; exit 1; }
mkdir -p -- "$output_dir"
docker buildx build \
    --platform linux/amd64 \
    --file "$repo_dir/Dockerfile.windows" \
    --target export \
    --output "type=local,dest=$output_dir" \
    --progress plain \
    "$repo_dir"
printf 'Portable archive and BUILD-INFO.txt exported to %s\n' "$output_dir"
