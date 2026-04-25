#!/usr/bin/env bash
# Validate CI runner prerequisites: required commands and resource limits.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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
# Every enabled toolkit policy in toolkit/toolkit.toml must appear as a step
# name in both the justfile ci-dry-run (add_step) and at least one
# .github/workflows/*.yml file. Case-insensitive substring match.

humanize_check_name() {
  local raw="$1"
  local result="" part
  IFS='_' read -r -a parts <<<"$raw"
  for part in "${parts[@]}"; do
    if [[ -n "$part" ]]; then
      result+="$(printf '%s' "$part" | awk '{print toupper(substr($0, 1, 1)) substr($0, 2)}') "
    fi
  done
  printf '%s' "${result% }"
}

load_enabled_toolkit_checks() {
  local config_path="$1"
  awk '
    BEGIN {
      in_rust_base = 0
      rust_base_enabled = 0
      rust_base_docs = 0
    }

    function flush_section() {
      if (section == "") {
        return
      }
      if (seen_enabled != 1) {
        printf "missing-enabled:%s\n", section
      } else if (enabled == "true") {
        print section
      } else {
        printf "disabled:%s\n", section
      }
      section = ""
      seen_enabled = 0
      enabled = ""
    }

    function emit_rust_base_bundle_checks() {
      if (rust_base_enabled != 1) {
        return
      }
      print "proc_macro_scope"
      print "result_must_use"
      print "test_boundaries"
      print "workspace_hygiene"
      print "crate_root_policy"
      print "ignored_result"
      print "unsafe_boundary"
      print "bool_param"
      print "must_use_public_return"
      print "assert_shape"
      print "drop_side_effects"
      print "recursion_guard"
      print "naming_units"
      print "limit_constant"
      print "public_type_width"
      print "dependency_policy"
      print "workflow_actions"
      if (rust_base_docs == 1) {
        print "docs_link_check"
        print "docs_semantic_drift"
        print "text_formatting"
      }
    }

    /^\[checks\.[^.][^.]*\]$/ {
      in_rust_base = 0
      flush_section()
      section = $0
      sub(/^\[checks\./, "", section)
      sub(/\]$/, "", section)
      next
    }

    /^\[bundles\.rust_base\]$/ {
      in_rust_base = 1
      flush_section()
      next
    }

    /^\[/ {
      in_rust_base = 0
      flush_section()
      next
    }

    in_rust_base && /^[[:space:]]*enabled[[:space:]]*=/ {
      rust_base_enabled = ($0 ~ /=[[:space:]]*true([[:space:]]|$)/) ? 1 : 0
      next
    }

    in_rust_base && /^[[:space:]]*docs_roots[[:space:]]*=/ {
      rust_base_docs = ($0 ~ /\[[^]]*[^[:space:],][^]]*\]/) ? 1 : 0
      next
    }

    section != "" && /^[[:space:]]*enabled[[:space:]]*=/ {
      seen_enabled = 1
      enabled = ($0 ~ /=[[:space:]]*true([[:space:]]|$)/) ? "true" : "false"
      next
    }

    END {
      flush_section()
      emit_rust_base_bundle_checks()
    }
  ' "$config_path"
}

toolkit_config_ok=true
enabled_toolkit_checks=()
while IFS= read -r item; do
  case "$item" in
    missing-enabled:*)
      echo "ci-preflight: toolkit check missing explicit enabled=true: ${item#*:}" >&2
      toolkit_config_ok=false
      ;;
    disabled:*)
      echo "ci-preflight: toolkit check disabled in toolkit/toolkit.toml: ${item#*:}" >&2
      toolkit_config_ok=false
      ;;
    "")
      ;;
    *)
      enabled_toolkit_checks+=("$item")
      ;;
  esac
done < <(load_enabled_toolkit_checks "$repo_root/toolkit/toolkit.toml")

if [ "$toolkit_config_ok" = false ]; then
  fail "all Jacquard toolkit checks must be enabled by default"
fi

checks=(
  "Format Check"
  "Clippy"
  "Tests"
  "Wasm Check"
  "Wasm Reference Client Test"
  "Docs Links"
  "Trait Purity"
  "Crate Boundary"
  "No usize in Models"
  "Docs Build"
)

for check in "${enabled_toolkit_checks[@]}"; do
  checks+=("$(humanize_check_name "$check")")
done

justfile="$repo_root/justfile"
ci_yml_dir="$repo_root/.github/workflows"

dry_run_names="$(
  grep 'add_step ' "$justfile" \
    | sed -E 's/.*add_step[[:space:]]+"([^"]+)".*/\1/' \
    | tr '[:upper:]' '[:lower:]' \
    | sort -u
)"

ci_names="$(
  find "$ci_yml_dir" -name '*.yml' -exec grep -h '^\s*-\?\s*name:' {} + 2>/dev/null \
    | sed -E 's/^[[:space:]]*-?[[:space:]]*name:[[:space:]]*//' \
    | tr '[:upper:]' '[:lower:]' \
    | sort -u
)"

parity_ok=true

for check in "${checks[@]}"; do
  lc_check="$(echo "$check" | tr '[:upper:]' '[:lower:]')"

  if ! echo "$dry_run_names" | grep -qi "$lc_check"; then
    echo "ci-preflight: check missing from ci-dry-run: $check" >&2
    parity_ok=false
  fi

  if ! echo "$ci_names" | grep -qi "$lc_check"; then
    echo "ci-preflight: check missing from CI workflows: $check" >&2
    parity_ok=false
  fi
done

if [ "$parity_ok" = false ]; then
  fail "ci-dry-run and CI workflows have diverged"
fi

echo "ci-preflight: ci-dry-run and CI workflows are in sync"
