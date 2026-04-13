#!/usr/bin/env bash
# Generate docs/SUMMARY.md from Markdown files in docs/.
# Called by `just summary`, ci-dry-run, and CI workflows before any step that
# compiles test targets (cargo clippy/test/dylint --all-targets all trigger
# include_str! on docs/SUMMARY.md at compile time).
set -euo pipefail

docs="docs"
build_dir="$docs/book"
out="$docs/SUMMARY.md"

echo "# Summary" > "$out"
echo "" >> "$out"

# Find all .md files under docs/, excluding SUMMARY.md itself and the build output
while IFS= read -r f; do
    rel="${f#$docs/}"

    # Skip SUMMARY.md
    [ "$rel" = "SUMMARY.md" ] && continue

    # Skip files under the build output directory
    case "$f" in "$build_dir"/*) continue ;; esac

    # Derive the title from the first H1; fallback to filename
    title="$(grep -m1 '^# ' "$f" | sed 's/^# *//')"
    if [ -z "$title" ]; then
        base="$(basename "${f%.*}")"
        title="$(printf '%s\n' "$base" \
            | tr '._-' '   ' \
            | awk '{for(i=1;i<=NF;i++){ $i=toupper(substr($i,1,1)) substr($i,2) }}1')"
    fi

    # Indent two spaces per directory depth
    depth="$(awk -F'/' '{print NF-1}' <<<"$rel")"
    indent="$(printf '%*s' $((depth*2)) '')"

    echo "${indent}- [$title](${rel})" >> "$out"
done < <(find "$docs" -type f -name '*.md' -not -name 'SUMMARY.md' -not -path "$build_dir/*" | LC_ALL=C sort)

echo "Wrote $out"
