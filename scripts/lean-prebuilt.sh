#!/usr/bin/env bash
# Ensure the Lean verification package has its Mathlib olean cache hydrated.
#
# Telltale (the direct dependency) pins Mathlib to a specific git commit.
# lake exe cache get downloads prebuilt .olean files from
# cache.leanprover.community keyed by that commit — Mathlib is never
# rebuilt from source when the cache entry exists.
#
# Iris and Paco come through Telltale and compile once into .lake/packages/.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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

# Step 1: generate or refresh lake-manifest.json.
# This pins the exact commit of telltale (and transitively mathlib/iris/paco).
if [[ ! -f "lake-manifest.json" ]]; then
  echo "== lake update (generating lake-manifest.json) =="
  lake update
else
  echo "OK   lake-manifest.json present"
fi

mathlib_dep_type="$(
  perl -MJSON::PP -e '
    my $path = shift;
    open my $fh, "<", $path or die "failed to open $path: $!";
    local $/;
    my $manifest = decode_json(<$fh>);
    for my $pkg (@{$manifest->{packages} // []}) {
      next unless ($pkg->{name} // q()) eq "mathlib";
      print $pkg->{type} // q();
      exit 0;
    }
    exit 1;
  ' "lake-manifest.json" 2>/dev/null || true
)"

# Step 2: fetch prebuilt Mathlib oleans from cache.leanprover.community.
# The cache key is the exact mathlib commit resolved transitively through
# telltale and pinned in lake-manifest.json.
MATHLIB_MARKER=".lake/packages/mathlib/.lake/build/lib/lean/Mathlib.olean"
if [[ "${mathlib_dep_type}" == "path" ]]; then
  echo "OK   mathlib is supplied by a local path dependency; skipping cache fetch"
elif [[ ! -f "${MATHLIB_MARKER}" ]]; then
  echo "== lake exe cache get (fetching prebuilt Mathlib oleans) =="
  lake exe cache get
else
  echo "OK   Mathlib olean cache present"
fi

if [[ "${mathlib_dep_type}" != "path" && ! -f "${MATHLIB_MARKER}" ]]; then
  echo "error: Mathlib olean marker missing after cache fetch" >&2
  echo "  expected: ${LEAN_DIR}/${MATHLIB_MARKER}" >&2
  echo "hint: try 'lake exe cache get' manually inside verification/" >&2
  exit 1
fi

echo "== Lean prebuilt cache ready — run 'just lean-build' to compile verification =="
