#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dry_run=0

usage() {
  printf 'Usage: %s [--dry-run]\n' "$(basename "$0")"
  printf '\nRemove generated build, plugin, dependency-cache, and temporary files.\n'
}

case "${1:-}" in
  '') ;;
  --dry-run) dry_run=1 ;;
  --help|-h)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

paths=(
  "$root/target"
  "$root/.cargo-home"
  "$root/.linux-build-deps"
  "$root/plugins"
  "$root/tmp"
  "$root/Cargo.lock"
)

for path in "${paths[@]}"; do
  if [[ ! -e "$path" && ! -L "$path" ]]; then
    continue
  fi

  if ((dry_run)); then
    printf 'Would remove %s\n' "${path#"$root/"}"
  else
    rm -rf -- "$path"
    printf 'Removed %s\n' "${path#"$root/"}"
  fi
done

printf '%s\n' "Only source and build configuration files remain."
