#!/usr/bin/env bash
# no-usize-in-models.sh — Reject usize in stored/protocol type fields.
#
# The style guide requires explicitly-sized integers (u8, u16, u32, u64)
# in all stored formats and protocol types. This script greps for usize
# fields in public structs and enums across core and traits source.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

search_dirs=(
  crates/core/src
  crates/traits/src
)

existing_dirs=()
for d in "${search_dirs[@]}"; do
  [ -d "$d" ] && existing_dirs+=("$d")
done

if [ ${#existing_dirs[@]} -eq 0 ]; then
  echo "no-usize-in-models: no source directories found; skipping"
  exit 0
fi

# Match usize appearing as a field type, generic parameter, or tuple field
# in struct/enum definitions. Exclude test modules and comments.
hits=()
while IFS= read -r match; do
  [ -z "$match" ] && continue
  # Skip test modules
  case "$match" in
    *"#[cfg(test)]"*|*"#[test]"*) continue ;;
  esac
  hits+=("$match")
done < <(rg --no-heading -n '\busize\b' "${existing_dirs[@]}" \
  --glob '*.rs' \
  --glob '!**/tests/**' \
  --glob '!**/test_*' \
  2>/dev/null | grep -v '^\s*//' || true)

if [ ${#hits[@]} -gt 0 ]; then
  printf 'no-usize-in-models: found usize in model source files:\n' >&2
  printf '  %s\n' "${hits[@]}" >&2
  echo ""
  echo "no-usize-in-models: use explicitly-sized integers (u8, u16, u32, u64) instead"
  exit 1
fi

echo "no-usize-in-models: no usize found in model source files"
