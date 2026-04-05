#!/usr/bin/env bash
# Validate CI runner prerequisites: required commands and resource limits.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

fail() {
  echo "ci-preflight: $*" >&2
  exit 1
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || fail "required command missing from PATH: $cmd"
}

format_gib() {
  local kib="$1"
  awk -v kib="$kib" 'BEGIN { printf "%.1f GiB", kib / 1024 / 1024 }'
}

# Disk space check
min_free_gib="${JACQUARD_CI_PREFLIGHT_MIN_FREE_GIB:-1}"
avail_kib="$(df -Pk "$repo_root" | awk 'NR==2 { print $4 }')"
min_kib="$((min_free_gib * 1024 * 1024))"
if [[ -z "$avail_kib" || "$avail_kib" -lt "$min_kib" ]]; then
  fail "need at least ${min_free_gib} GiB free; found $(format_gib "${avail_kib:-0}")"
fi

# Temp directory writable
tmp_root="${TMPDIR:-$repo_root/.tmp}"
mkdir -p "$tmp_root"
tmp_probe="$tmp_root/ci-preflight.$$"
: >"$tmp_probe" || fail "failed to write temp probe under $tmp_root"
rm -f "$tmp_probe"

# Required commands
for cmd in bash git rg rustc cargo just mdbook; do
  require_cmd "$cmd"
done

echo "ci-preflight: disk free $(format_gib "$avail_kib") (threshold ${min_free_gib}.0 GiB)"
echo "ci-preflight: temp dir writable at $tmp_root"
echo "ci-preflight: required toolchain commands present"

# ── CI / dry-run parity check ──────────────────────────────────────────
#
# Verify that ci-dry-run (justfile) and the CI workflow files cover the
# same set of check categories. The manifest lives in
# scripts/ci/check-manifest.txt. Each line is a check name. Both the
# justfile add_step names and the CI job step names must cover every
# manifest entry.

manifest="$repo_root/scripts/ci/check-manifest.txt"

if [[ ! -f "$manifest" ]]; then
  fail "missing check manifest at $manifest"
fi

justfile="$repo_root/justfile"
ci_yml_dir="$repo_root/.github/workflows"

# Extract add_step names from justfile (first quoted argument).
dry_run_names="$(
  grep 'add_step ' "$justfile" \
    | sed -E 's/.*add_step[[:space:]]+"([^"]+)".*/\1/' \
    | tr '[:upper:]' '[:lower:]' \
    | sort -u
)"

# Extract step name: values from CI workflow files.
ci_names="$(
  find "$ci_yml_dir" -name '*.yml' -exec grep -h '^\s*-\?\s*name:' {} + 2>/dev/null \
    | sed -E 's/^[[:space:]]*-?[[:space:]]*name:[[:space:]]*//' \
    | tr '[:upper:]' '[:lower:]' \
    | sort -u
)"

parity_ok=true

while IFS= read -r check; do
  [[ -z "$check" || "$check" == \#* ]] && continue

  lc_check="$(echo "$check" | tr '[:upper:]' '[:lower:]')"

  if ! echo "$dry_run_names" | grep -qi "$lc_check"; then
    echo "ci-preflight: manifest check missing from ci-dry-run: $check" >&2
    parity_ok=false
  fi

  if ! echo "$ci_names" | grep -qi "$lc_check"; then
    echo "ci-preflight: manifest check missing from CI workflows: $check" >&2
    parity_ok=false
  fi
done < "$manifest"

if [ "$parity_ok" = false ]; then
  fail "ci-dry-run and CI workflows have diverged from the check manifest"
fi

echo "ci-preflight: ci-dry-run and CI workflows match the check manifest"
