#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
[[ $(uname -m) == x86_64 ]] || { echo "Linux bundle supports x86_64 only" >&2; exit 1; }
command -v lsb_release >/dev/null || {
    echo "Unsupported packaging environment. Use Ubuntu 22.04 targeting glibc 2.35." >&2
    exit 1
}
[[ $(lsb_release --id --short) == Ubuntu && $(lsb_release --release --short) == 22.04 ]] || {
    echo "Unsupported packaging environment. Use Ubuntu 22.04 targeting glibc 2.35." >&2
    exit 1
}
[[ $(getconf GNU_LIBC_VERSION) == "glibc 2.35" ]] || {
    echo "Unsupported packaging environment. Use Ubuntu 22.04 targeting glibc 2.35." >&2
    exit 1
}

cargo build --release --locked -p overmax-app --bin overmax-rs

stage=dist/overmax
archive=dist/overmax-linux-x86_64.tar.gz
rm -rf "$stage"
rm -f "$archive"
install -Dm755 target/release/overmax-rs "$stage/overmax"
install -Dm644 settings.json "$stage/settings.json"
install -Dm644 README.md "$stage/README.md"
mkdir "$stage/cache"
tar -czf "$archive" -C dist overmax
tar -tzf "$archive" overmax/overmax >/dev/null
sha256sum "$archive" > "$archive.sha256"
bash scripts/smoke-linux-bundle.sh "$archive" 2.35
OVERMAX_LINUX_UPDATE_SMOKE_ARCHIVE="../../$archive" cargo test --locked -p overmax-app --lib system::updater::linux::tests::release_tarball_installs_and_updates_without_touching_user_data -- --exact

echo "Created $archive"
