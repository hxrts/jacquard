#!/usr/bin/env bash
# docs-semantic-drift.sh — Detect stale backtick references in docs.
#
# Checks CLAUDE.md and docs/*.md for: unknown just recipes, missing file
# paths, unresolved PascalCase identifiers, unknown workspace crate names,
# unresolved qualified symbols, deprecated identifiers, and version drift.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

# ── Temp Directory ────────────────────────────────────────────────────
TMPDIR_DRIFT="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_DRIFT"' EXIT

ERRORS_FILE="$TMPDIR_DRIFT/errors"
touch "$ERRORS_FILE"

# ── Collect Doc Files ─────────────────────────────────────────────────
DOC_FILES=()
[[ -f CLAUDE.md ]] && DOC_FILES+=(CLAUDE.md)
while IFS= read -r f; do
    DOC_FILES+=("$f")
done < <(find docs -name '*.md' -type f -not -path 'docs/book/*' 2>/dev/null | sort)

# ── Cargo Metadata ────────────────────────────────────────────────────
CARGO_META="$TMPDIR_DRIFT/cargo_meta.json"
cargo metadata --no-deps --format-version 1 > "$CARGO_META"

# Workspace package names (hyphenated and underscored)
jq -r '.packages[].name' "$CARGO_META" | sort -u > "$TMPDIR_DRIFT/pkg_names"
sed 's/-/_/g' "$TMPDIR_DRIFT/pkg_names" > "$TMPDIR_DRIFT/pkg_names_underscore"
cat "$TMPDIR_DRIFT/pkg_names" "$TMPDIR_DRIFT/pkg_names_underscore" | sort -u > "$TMPDIR_DRIFT/crate_tokens"

# Package versions: name<TAB>version
jq -r '.packages[] | "\(.name)\t\(.version)"' "$CARGO_META" > "$TMPDIR_DRIFT/pkg_versions"

# ── Just Recipes ──────────────────────────────────────────────────────
just --summary | tr ' ' '\n' | sort -u > "$TMPDIR_DRIFT/just_recipes"

# ── Build Identifier Set ──────────────────────────────────────────────
{
    find crates -name '*.rs' -type f -exec grep -ohE '\b[A-Za-z_][A-Za-z0-9_]*\b' {} + 2>/dev/null || true
} | sort -u > "$TMPDIR_DRIFT/repo_identifiers"

# ── Skip Identifiers ──────────────────────────────────────────────────
# Well-known types, traits, keywords that should not trigger warnings.
cat > "$TMPDIR_DRIFT/skip_identifiers" <<'SKIP'
String
Vec
Option
Result
Box
Arc
Rc
Mutex
HashMap
HashSet
BTreeMap
BTreeSet
PathBuf
Path
Ok
Err
Some
None
Self
Sized
Send
Sync
Clone
Copy
Debug
Display
Default
Drop
Eq
Ord
Hash
Iterator
Future
Pin
From
Into
AsRef
Deref
PartialEq
PartialOrd
Serialize
Deserialize
Error
Read
Write
PhantomData
Infallible
README
SUMMARY
TODO
FIXME
NOTE
WARNING
IMPORTANT
API
CLI
CI
CD
PR
OS
IO
UUID
HTTP
HTTPS
URL
JSON
CBOR
TOML
YAML
WASM
BFT
CRDT
BLE
GPS
GATT
QUIC
MTU
Alice
Bob
Client
Server
Worker
Coordinator
Done
Active
Closed
Faulted
Admitted
Blocked
Failure
Full
Ack
Commit
Abort
Cancel
Retry
Ping
Pong
SKIP
sort -u "$TMPDIR_DRIFT/skip_identifiers" -o "$TMPDIR_DRIFT/skip_identifiers"

# ── External Prefixes ─────────────────────────────────────────────────
# Qualified paths starting with these are from external crates.
cat > "$TMPDIR_DRIFT/external_prefixes" <<'EXT'
std
core
alloc
serde
serde_json
tokio
futures
uuid
blake3
thiserror
tracing
proc_macro2
telltale
EXT

# ── Planned Crate Names ───────────────────────────────────────────────
# Crates documented in the architecture but not yet in the workspace.
# Remove entries as each crate is created.
cat > "$TMPDIR_DRIFT/planned_crates" <<'PLANNED'
jacquard-mesh
jacquard-router
jacquard-transport
jacquard-simulator
PLANNED
cat "$TMPDIR_DRIFT/planned_crates" >> "$TMPDIR_DRIFT/crate_tokens"
sed 's/-/_/g' "$TMPDIR_DRIFT/planned_crates" >> "$TMPDIR_DRIFT/crate_tokens"
sort -u "$TMPDIR_DRIFT/crate_tokens" -o "$TMPDIR_DRIFT/crate_tokens"

# ── Deprecated Identifiers ────────────────────────────────────────────
# name<TAB>reason. Add entries as identifiers are removed or renamed.
cat > "$TMPDIR_DRIFT/deprecated_identifiers" <<'DEP'
DEP

# ── Helpers ───────────────────────────────────────────────────────────

in_set() {
    grep -qFx "$1" "$2" 2>/dev/null
}

looks_like_path() {
    local s="$1"
    case "$s" in
        CLAUDE.md|Cargo.toml|justfile) return 0 ;;
        docs/*|crates/*|scripts/*|lints/*|nix/*|.github/*|work/*) return 0 ;;
        *) return 1 ;;
    esac
}

normalized_symbol_tail() {
    local snippet="$1"
    local last="${snippet##*::}"
    if [[ "$last" =~ ^([A-Za-z_][A-Za-z0-9_]*) ]]; then
        echo "${BASH_REMATCH[1]}"
    fi
}

# ── Scan Backtick Code Spans ──────────────────────────────────────────
for doc_file in "${DOC_FILES[@]}"; do
    line_no=0
    in_code_block=0
    while IFS= read -r line; do
        line_no=$((line_no + 1))

        # Track fenced code blocks to skip them
        if [[ "$line" =~ ^'```' ]]; then
            if [[ $in_code_block -eq 0 ]]; then
                in_code_block=1
            else
                in_code_block=0
            fi
            continue
        fi
        [[ $in_code_block -eq 1 ]] && continue

        # Extract all backtick code spans from this line
        rest="$line"
        while [[ "$rest" == *'`'* ]]; do
            rest="${rest#*\`}"
            if [[ "$rest" != *'`'* ]]; then
                break
            fi
            snippet="${rest%%\`*}"
            rest="${rest#*\`}"

            [[ -z "$snippet" ]] && continue

            # Trim whitespace
            snippet="${snippet#"${snippet%%[![:space:]]*}"}"
            snippet="${snippet%"${snippet##*[![:space:]]}"}"
            [[ -z "$snippet" ]] && continue

            # ── Check deprecated identifiers ────────────────────
            if [[ -s "$TMPDIR_DRIFT/deprecated_identifiers" ]]; then
                dep_reason=""
                while IFS=$'\t' read -r dep_name dep_msg; do
                    if [[ "$snippet" == "$dep_name" ]]; then
                        dep_reason="$dep_msg"
                        break
                    fi
                done < "$TMPDIR_DRIFT/deprecated_identifiers"
                if [[ -n "$dep_reason" ]]; then
                    echo "$doc_file:$line_no: deprecated identifier \`$snippet\` ($dep_reason)" >> "$ERRORS_FILE"
                    continue
                fi
            fi

            # ── just recipe check ───────────────────────────────
            if [[ "$snippet" == "just "* ]]; then
                read -ra parts <<< "$snippet"
                if [[ ${#parts[@]} -ge 2 && "${parts[1]}" != -* ]]; then
                    if ! in_set "${parts[1]}" "$TMPDIR_DRIFT/just_recipes"; then
                        echo "$doc_file:$line_no: unknown just recipe \`${parts[1]}\`" >> "$ERRORS_FILE"
                    fi
                fi
                continue
            fi

            # ── File path check ─────────────────────────────────
            if looks_like_path "$snippet"; then
                if [[ "$snippet" == *'*'* ]]; then
                    continue
                fi
                if [[ ! -e "$ROOT_DIR/$snippet" ]]; then
                    echo "$doc_file:$line_no: missing referenced path \`$snippet\`" >> "$ERRORS_FILE"
                fi
                continue
            fi

            # ── Workspace crate token check ─────────────────────
            if in_set "$snippet" "$TMPDIR_DRIFT/crate_tokens"; then
                continue
            fi
            if [[ "$snippet" =~ ^jacquard(-[a-z0-9]+)?$ ]]; then
                echo "$doc_file:$line_no: unknown workspace crate \`$snippet\`" >> "$ERRORS_FILE"
                continue
            fi

            # ── Qualified symbol check (contains ::) ────────────
            if [[ "$snippet" == *'::'* ]]; then
                if [[ "$snippet" =~ [[:space:]\{\}\(\),] ]]; then
                    continue
                fi
                head="${snippet%%::*}"
                if in_set "$head" "$TMPDIR_DRIFT/external_prefixes"; then
                    continue
                fi
                symbol="$(normalized_symbol_tail "$snippet")"
                if [[ -n "$symbol" ]] && ! in_set "$symbol" "$TMPDIR_DRIFT/repo_identifiers" && ! in_set "$symbol" "$TMPDIR_DRIFT/skip_identifiers"; then
                    echo "$doc_file:$line_no: unresolved repo-local symbol tail \`$snippet\`" >> "$ERRORS_FILE"
                fi
                continue
            fi

            # ── PascalCase identifier check ─────────────────────
            if [[ "$snippet" =~ ^[A-Z][A-Za-z0-9_]+$ ]]; then
                if in_set "$snippet" "$TMPDIR_DRIFT/skip_identifiers"; then
                    continue
                fi
                # Skip ALL_CAPS constants
                if [[ "$snippet" =~ ^[A-Z][A-Z0-9_]+$ ]]; then
                    continue
                fi
                if ! in_set "$snippet" "$TMPDIR_DRIFT/repo_identifiers"; then
                    echo "$doc_file:$line_no: unresolved type or identifier \`$snippet\`" >> "$ERRORS_FILE"
                fi
            fi

        done
    done < "$doc_file"
done

# ── Crate Version Accuracy ────────────────────────────────────────────
# Compare version strings in docs against actual workspace versions.

for vpath in "${DOC_FILES[@]}"; do
    line_no=0
    while IFS= read -r vline; do
        line_no=$((line_no + 1))

        crate_name=""
        declared_version=""

        if [[ "$vline" =~ ^[[:space:]]*jacquard(-[a-z0-9]+)?[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
            crate_name="jacquard${BASH_REMATCH[1]}"
            declared_version="${BASH_REMATCH[2]}"
        elif [[ "$vline" =~ ^[[:space:]]*jacquard(-[a-z0-9]+)?[[:space:]]*=[[:space:]]*\{.*version[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
            crate_name="jacquard${BASH_REMATCH[1]}"
            declared_version="${BASH_REMATCH[2]}"
        fi

        if [[ -n "$crate_name" && -n "$declared_version" ]]; then
            expected_version=""
            while IFS=$'\t' read -r pkg_name pkg_ver; do
                if [[ "$pkg_name" == "$crate_name" ]]; then
                    expected_version="$pkg_ver"
                    break
                fi
            done < "$TMPDIR_DRIFT/pkg_versions"

            if [[ -n "$expected_version" && "$declared_version" != "$expected_version" ]]; then
                echo "$vpath:$line_no: \`$crate_name\` version \`$declared_version\` does not match workspace version \`$expected_version\`" >> "$ERRORS_FILE"
            fi
        fi
    done < "$vpath"
done

# ── Report ────────────────────────────────────────────────────────────
if [[ -s "$ERRORS_FILE" ]]; then
    echo "docs-semantic-drift: failures found:" >&2
    cat "$ERRORS_FILE" >&2
    exit 1
fi

echo "docs-semantic-drift: passed"
