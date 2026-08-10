#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
archive=${1:-dist/overmax-linux-x86_64.tar.gz}
baseline=${2:-2.35}
checksum="$archive.sha256"

[[ -f "$archive" ]] || { echo "Missing Linux bundle: $archive" >&2; exit 1; }
[[ -f "$checksum" ]] || { echo "Missing Linux bundle checksum: $checksum" >&2; exit 1; }
sha256sum --check "$checksum"

smoke_dir=$(mktemp -d)
trap 'rm -rf "$smoke_dir"' EXIT
tar -xzf "$archive" -C "$smoke_dir"

install_dir="$smoke_dir/overmax"
binary="$install_dir/overmax"
[[ -x "$binary" ]]
[[ -f "$install_dir/settings.json" ]]
[[ -f "$install_dir/README.md" ]]
[[ -d "$install_dir/cache" ]]
"$binary" --version | grep -Eq '^overmax [0-9]'
! ldd "$binary" | grep -q 'not found'

highest_glibc=$(
    readelf --version-info "$binary" |
        sed -n 's/.*Name: GLIBC_\([0-9.]*\).*/\1/p' |
        sort -Vu |
        tail -n 1
)
[[ -n "$highest_glibc" ]]
[[ "$(printf '%s\n%s\n' "$highest_glibc" "$baseline" | sort -V | tail -n 1)" == "$baseline" ]] || {
    echo "Bundle requires GLIBC_$highest_glibc, above GLIBC_$baseline" >&2
    exit 1
}

printf 'Linux bundle smoke passed (max GLIBC_%s)\n' "$highest_glibc"
