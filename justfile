default: book
    @just --list

# check workspace compiles
check:
    cargo check --workspace

# build all crates
build:
    cargo build --workspace

# run all tests
test:
    cargo test --workspace

# run clippy lints
lint:
    cargo clippy --workspace -- -D warnings

# format code
fmt:
    cargo fmt --all

# check formatting
fmt-check:
    cargo fmt --all -- --check

# Generate docs/SUMMARY.md from Markdown files in docs/ and subfolders
summary:
    #!/usr/bin/env bash
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

# Generate transient build assets (mermaid, mathjax theme override)
_gen-assets:
    #!/usr/bin/env bash
    set -euo pipefail
    mdbook-mermaid install . > /dev/null 2>&1 || true
    # Patch mermaid-init.js with null guards for mdbook 0.5.x theme buttons
    sed -i.bak 's/document\.getElementById(\(.*\))\.addEventListener/const el = document.getElementById(\1); if (el) el.addEventListener/' mermaid-init.js && rm -f mermaid-init.js.bak
    # Generate theme/index.hbs with MathJax v2 inline $ config injected before MathJax loads
    mkdir -p theme
    mdbook init --theme /tmp/mdbook-theme-gen <<< $'n\n' > /dev/null 2>&1
    sed 's|<script async src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.1/MathJax.js?config=TeX-AMS-MML_HTMLorMML"></script>|<script>window.MathJax = { tex2jax: { inlineMath: [["$","$"],["\\\\(","\\\\)"]], displayMath: [["$$","$$"],["\\\\[","\\\\]"]], processEscapes: true } };</script>\n        <script async src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.1/MathJax.js?config=TeX-AMS-MML_HTMLorMML"></script>|' /tmp/mdbook-theme-gen/theme/index.hbs > theme/index.hbs
    rm -rf /tmp/mdbook-theme-gen

# Clean transient build assets
_clean-assets:
    rm -f mermaid-init.js mermaid.min.js
    rm -rf theme

# Build the book after regenerating the summary
book: summary _gen-assets
    mdbook build && just _clean-assets

# Serve locally with live reload
serve: summary _gen-assets
    #!/usr/bin/env bash
    trap 'just _clean-assets' EXIT
    mdbook serve --open
    exit 1

# install git hooks
install-hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed. Pre-commit checks will run automatically."
