#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

rust_files=()
while IFS= read -r file; do
  rust_files+=("$file")
done < <(rg --files -g '*.rs' crates/*/src | sort)

exempt_files=(
  "crates/core/src/constants.rs"
  "crates/core/src/lib.rs"
  "crates/macros/src/bounded_value_macro.rs"
  "crates/macros/src/effect_handler_macro.rs"
  "crates/macros/src/effect_trait_macro.rs"
  "crates/macros/src/id_type_macro.rs"
  "crates/macros/src/lib.rs"
  "crates/macros/src/must_use_handle_macro.rs"
  "crates/macros/src/public_model_macro.rs"
  "crates/macros/src/support.rs"
  "crates/traits/src/hashing.rs"
  "crates/traits/src/lib.rs"
  "crates/traits/src/routing.rs"
)

pattern='#\[(effect_trait|effect_handler|id_type|bounded_value|must_use_handle|public_model)'

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
