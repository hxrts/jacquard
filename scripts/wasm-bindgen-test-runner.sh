#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lockfile="$repo_root/Cargo.lock"

wasm_bindgen_version="$(
  awk '
    /name = "wasm-bindgen"/ {
      getline;
      if ($0 ~ /^version = /) {
        gsub(/version = "|"/, "", $0);
        print $0;
        exit;
      }
    }
  ' "$lockfile"
)"

if [ -z "$wasm_bindgen_version" ]; then
  echo "failed to resolve wasm-bindgen version from $lockfile" >&2
  exit 1
fi

cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/jacquard/wasm-bindgen-cli/$wasm_bindgen_version"
runner="$cache_root/bin/wasm-bindgen-test-runner"

need_install=true
if [ -x "$runner" ]; then
  if "$runner" --version 2>/dev/null | grep -Fq "$wasm_bindgen_version"; then
    need_install=false
  fi
fi

if [ "$need_install" = true ]; then
  rustup run stable cargo install \
    --root "$cache_root" \
    --locked \
    --version "$wasm_bindgen_version" \
    wasm-bindgen-cli
fi

exec "$runner" "$@"
