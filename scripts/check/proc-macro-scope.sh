#!/usr/bin/env bash
# Verify every non-exempt crate source file uses at least one jacquard proc macro.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

rust_files=()
while IFS= read -r file; do
  rust_files+=("$file")
done < <(find crates -path '*/src/*.rs' -type f | sort)

exempt_files=(
  "crates/core/src/lib.rs"
  "crates/core/src/base/mod.rs"
  "crates/core/src/base/constants.rs"
  "crates/core/src/model/mod.rs"
  "crates/core/src/routing/mod.rs"
  "crates/macros/src/lib.rs"
  "crates/macros/src/model/bounded_value.rs"
  "crates/macros/src/model/id_type.rs"
  "crates/macros/src/model/mod.rs"
  "crates/macros/src/model/must_use_handle.rs"
  "crates/macros/src/model/public_model.rs"
  "crates/macros/src/support/attrs.rs"
  "crates/macros/src/support/derives.rs"
  "crates/macros/src/support/mod.rs"
  "crates/macros/src/support/parsing.rs"
  "crates/macros/src/support/validation.rs"
  "crates/macros/src/traits/effect_handler.rs"
  "crates/macros/src/traits/effect_trait.rs"
  "crates/macros/src/traits/mod.rs"
  "crates/macros/src/traits/purity.rs"
  "crates/traits/src/hashing.rs"
  "crates/traits/src/lib.rs"
  "crates/traits/src/routing.rs"
)

pattern='#\[(effect_trait|effect_handler|id_type|bounded_value|must_use_handle|public_model|purity)'

is_exempt() {
  local file="$1"
  shift

  for exempt_file in "$@"; do
    if [[ "$file" == "$exempt_file" ]]; then
      return 0
    fi
  done

  return 1
}

missing=()
for file in "${rust_files[@]}"; do
  if is_exempt "$file" "${exempt_files[@]}"; then
    continue
  fi

  if ! rg -q "$pattern" "$file"; then
    missing+=("$file")
  fi
done

stale=()
for file in "${exempt_files[@]}"; do
  if [[ ! -f "$file" ]]; then
    stale+=("$file")
  fi
done

if (( ${#stale[@]} > 0 )); then
  printf 'stale proc-macro exemptions:\n' >&2
  printf '  %s\n' "${stale[@]}" >&2
  exit 1
fi

if (( ${#missing[@]} > 0 )); then
  printf 'missing proc-macro file coverage:\n' >&2
  printf '  %s\n' "${missing[@]}" >&2
  exit 1
fi

printf 'proc-macro file coverage is maximal for crate source files\n'
