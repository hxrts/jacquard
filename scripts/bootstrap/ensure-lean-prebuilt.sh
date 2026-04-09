#!/usr/bin/env bash
# Ensure the Lean verification package has its Mathlib olean cache hydrated.
#
# On first run: resolves the Mathlib revision compatible with the pinned Lean
# toolchain and writes lake-manifest.json, then fetches prebuilt oleans from
# https://cache.leanprover.community so Mathlib is never rebuilt from source.
#
# On subsequent runs: checks that the manifest and olean markers are present;
# re-fetches only if something is missing.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LEAN_DIR="${ROOT_DIR}/verification"

if [[ ! -f "${LEAN_DIR}/lakefile.lean" ]]; then
  echo "error: missing ${LEAN_DIR}/lakefile.lean" >&2
  exit 2
fi

if ! command -v lake >/dev/null 2>&1; then
  echo "error: lake not on PATH — run this script from inside 'nix develop'" >&2
  exit 2
fi

cd "${LEAN_DIR}"

# Step 1: generate or refresh lake-manifest.json so the mathlib revision is pinned.
if [[ ! -f "lake-manifest.json" ]]; then
  echo "== lake update (generating lake-manifest.json) =="
  lake update
else
  echo "OK   lake-manifest.json present"
fi

# Step 2: fetch prebuilt oleans from the Mathlib cache server.
# lake exe cache get downloads .olean files matching the exact mathlib commit
# pinned in lake-manifest.json; it never rebuilds mathlib from source.
MATHLIB_OLEAN_MARKER=".lake/packages/mathlib/.lake/build/lib/lean/Mathlib.olean"
if [[ ! -f "${MATHLIB_OLEAN_MARKER}" ]]; then
  echo "== lake exe cache get (fetching prebuilt Mathlib oleans) =="
  lake exe cache get
else
  echo "OK   Mathlib olean cache present at ${LEAN_DIR}/${MATHLIB_OLEAN_MARKER}"
fi

# Verify the marker is now present.
if [[ ! -f "${MATHLIB_OLEAN_MARKER}" ]]; then
  echo "error: Mathlib olean marker missing after cache fetch:" >&2
  echo "  ${LEAN_DIR}/${MATHLIB_OLEAN_MARKER}" >&2
  echo "hint: try 'lake exe cache get' manually inside verification/" >&2
  exit 1
fi

echo "== Lean prebuilt cache ready — run 'lake build Hello' to verify =="
