#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="${1:-x86_64-unknown-linux-gnu}"
destination="$root/plugins"

case "$target" in
  x86_64-unknown-linux-gnu) platform="linux" ;;
  x86_64-pc-windows-gnu) platform="windows" ;;
  *)
    printf 'Unsupported target: %s\n' "$target" >&2
    exit 2
    ;;
esac

export CARGO_HOME="$root/.cargo-home"
rm -rf "$root/target/bundled/StereoDelay.vst3"
cargo xtask bundle stereo-delay --release --target "$target"

mkdir -p "$destination/$platform"
rm -rf "$destination/$platform/StereoDelay.vst3"
cp -a "$root/target/bundled/StereoDelay.vst3" "$destination/$platform/"

printf 'Created %s/%s/StereoDelay.vst3\n' "$destination" "$platform"
