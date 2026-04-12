#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

sanitize_path() {
  perl -e '
    my $path = $ENV{PATH} // q();
    my $home = $ENV{HOME} // q();
    my $cargo_home = $ENV{CARGO_HOME} // ($home eq q() ? q() : "$home/.cargo");
    my @drop = grep { $_ ne q() } (
      $home eq q() ? q() : "$home/.cargo/bin",
      $cargo_home eq q() ? q() : "$cargo_home/bin",
    );
    my %drop = map { $_ => 1 } @drop;
    my @parts = grep { $_ ne q() && !$drop{$_} } split(/:/, $path, -1);
    print join(":", @parts);
  '
}

ensure_writable_toolkit_root() {
  local toolkit_root="${TOOLKIT_ROOT:-}"
  local cache_root cache_key writable_root
  if [ -z "$toolkit_root" ] || [ -w "$toolkit_root/xtask/Cargo.lock" ]; then
    printf '%s' "$toolkit_root"
    return
  fi

  cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/jacquard/toolkit-roots"
  cache_key="$(basename "$toolkit_root" | tr -cs 'A-Za-z0-9._-' '_')"
  writable_root="$cache_root/$cache_key"
  if [ ! -d "$writable_root" ]; then
    mkdir -p "$cache_root"
    cp -R "$toolkit_root" "$writable_root"
    chmod -R u+w "$writable_root"
  fi
  printf '%s' "$writable_root"
}

run_sanitized() {
  local sanitized_path
  sanitized_path="$(sanitize_path)"
  env \
    -u CARGO \
    -u RUSTC \
    -u RUSTDOC \
    -u RUSTUP_TOOLCHAIN \
    PATH="$sanitized_path" \
    "$@"
}

if [ "${1:-}" = "--inside-nix" ]; then
  shift
  if [ -z "${IN_NIX_SHELL:-}" ] || [ -z "${TOOLKIT_ROOT:-}" ] || ! command -v toolkit-xtask >/dev/null 2>&1; then
    echo "toolkit-shell.sh: --inside-nix requires the toolkit nix shell" >&2
    exit 1
  fi
  export TOOLKIT_ROOT
  TOOLKIT_ROOT="$(ensure_writable_toolkit_root)"
  run_sanitized "$@"
  exit $?
fi

if [ "${JACQUARD_TOOLKIT_SHELL_ACTIVE:-}" = "1" ] \
  && [ -n "${IN_NIX_SHELL:-}" ] \
  && [ -n "${TOOLKIT_ROOT:-}" ] \
  && command -v toolkit-xtask >/dev/null 2>&1; then
  export TOOLKIT_ROOT
  TOOLKIT_ROOT="$(ensure_writable_toolkit_root)"
  run_sanitized "$@"
  exit $?
fi

sanitized_path="$(sanitize_path)"
env \
  -u CARGO \
  -u RUSTC \
  -u RUSTDOC \
  -u RUSTUP_TOOLCHAIN \
  -u TOOLKIT_ROOT \
  -u IN_NIX_SHELL \
  PATH="$sanitized_path" \
  JACQUARD_TOOLKIT_SHELL_ACTIVE=1 \
  nix develop --command \
  "$repo_root/scripts/toolkit-shell.sh" --inside-nix "$@"
