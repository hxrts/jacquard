#!/usr/bin/env bash
# crate-boundary.sh — Enforce the Jacquard crate dependency graph.
#
# Rules:
#   - jacquard-core must not depend on jacquard-traits, jacquard-mesh, or any runtime crate
#   - jacquard-traits must depend only on jacquard-core (plus jacquard-macros)
#   - neither core nor traits may depend on telltale-runtime
#   - jacquard-transport must not depend on jacquard-mesh or jacquard-router
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

fail() {
  echo "crate-boundary: $*" >&2
  exit 1
}

violations=0

check_no_dep() {
  local crate_toml="$1" forbidden="$2" label="$3"
  if grep -qE "^${forbidden}[[:space:]]*=" "$crate_toml" 2>/dev/null; then
    echo "crate-boundary: $label depends on $forbidden (forbidden)" >&2
    violations=$((violations + 1))
  fi
}

# jacquard-core must not depend on traits, mesh, router, simulator, or telltale-runtime
core_toml="crates/core/Cargo.toml"
for forbidden in jacquard-traits jacquard-mesh jacquard-router jacquard-simulator jacquard-transport telltale-runtime; do
  check_no_dep "$core_toml" "$forbidden" "jacquard-core"
done

# jacquard-traits must not depend on mesh, router, simulator, transport, or telltale-runtime
traits_toml="crates/traits/Cargo.toml"
for forbidden in jacquard-mesh jacquard-router jacquard-simulator jacquard-transport telltale-runtime; do
  check_no_dep "$traits_toml" "$forbidden" "jacquard-traits"
done

# jacquard-transport must not depend on mesh or router
if [ -f "crates/transport/Cargo.toml" ]; then
  transport_toml="crates/transport/Cargo.toml"
  for forbidden in jacquard-mesh jacquard-router; do
    check_no_dep "$transport_toml" "$forbidden" "jacquard-transport"
  done
fi

if [ "$violations" -gt 0 ]; then
  echo ""
  echo "crate-boundary: found $violations forbidden dependency edge(s)"
  exit 1
fi

echo "crate-boundary: dependency graph is valid"
