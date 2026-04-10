#!/usr/bin/env bash
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required for the host wasm toolchain path" >&2
  exit 1
fi

toolchain="stable"
target="wasm32-unknown-unknown"
toolchain_bin="$(dirname "$(rustup which --toolchain "$toolchain" cargo)")"

if ! rustup target list --toolchain "$toolchain" --installed | grep -Fxq "$target"; then
  rustup target add --toolchain "$toolchain" "$target"
fi

export PATH="$toolchain_bin:$PATH"
export RUSTUP_TOOLCHAIN="$toolchain"

exec "$@"
