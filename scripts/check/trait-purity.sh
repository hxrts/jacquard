#!/usr/bin/env bash
# trait-purity.sh — Enforce purity annotations on public traits in jacquard-traits.
#
# Every public trait in crates/traits/src must carry #[purity(...)] or
# #[effect_trait]. This script greps for unannotated traits and fails CI
# if any are found. When run inside the nightly shell (nix develop
# ./nix/nightly) with cargo-dylint installed, it also runs the companion
# Dylint lint library at lints/trait_purity for deeper receiver-shape checks.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

fail() {
  echo "trait-purity: $*" >&2
  exit 1
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || fail "required command missing from PATH: $cmd"
}

require_cmd rg
require_cmd awk

trait_files=()
while IFS= read -r file; do
  trait_files+=("$file")
done < <(rg --files crates/traits/src -g '*.rs' | sort)

missing_markers=()

for file in "${trait_files[@]}"; do
  while IFS= read -r trait_name; do
    missing_markers+=("$file:$trait_name")
  done < <(
    awk '
      BEGIN {
        pending = 0;
      }
      /^[[:space:]]*#\[(purity|effect_trait)(\(|\])/ {
        pending = 1;
        next;
      }
      /^[[:space:]]*\/\/\// || /^[[:space:]]*$/ {
        next;
      }
      /^[[:space:]]*pub trait[[:space:]]+/ {
        if (!pending) {
          name = $0;
          sub(/^[[:space:]]*pub trait[[:space:]]+/, "", name);
          sub(/[:<{( ].*$/, "", name);
          if (name != "Sealed" && name != "EffectDefinition" && name != "HandlerDefinition") {
            print name;
          }
        }
        pending = 0;
        next;
      }
      {
        pending = 0;
      }
    ' "$file"
  )
done

if (( ${#missing_markers[@]} > 0 )); then
  printf 'trait-purity: public traits missing #[purity(...)] or #[effect_trait]:\n' >&2
  printf '  %s\n' "${missing_markers[@]}" >&2
  exit 1
fi

echo "trait-purity: all public traits are annotated"

if ! rustc --version | rg -q nightly; then
  echo "trait-purity: current Rust toolchain is not nightly; skipping Dylint run"
  echo "trait-purity: use 'nix develop ./nix/nightly' for the nightly lint shell"
  exit 0
fi

if command -v cargo-dylint >/dev/null 2>&1; then
  echo "trait-purity: running cargo dylint with local Jacquard lint libraries"
  cargo dylint --path lints/trait_purity --all -- --all-targets
  cargo dylint --path lints/model_policy --all -- --all-targets
else
  echo "trait-purity: cargo-dylint not installed; skipping Dylint run"
  echo "trait-purity: in the nightly shell, run 'install-dylint' once to enable the lint pass"
fi
