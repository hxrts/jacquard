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
    cargo clippy --workspace --all-targets -- -D warnings

# format code (uses nightly rustfmt for unstable rustfmt.toml options)
fmt:
    nix develop ./nix/nightly --command cargo-fmt-nightly --all

# check formatting (uses nightly rustfmt for unstable rustfmt.toml options)
fmt-check:
    nix develop ./nix/nightly --command cargo-fmt-nightly --all -- --check

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
    tmp_theme_dir="/tmp/mdbook-theme-gen"
    mkdir -p theme
    rm -rf "$tmp_theme_dir"
    mdbook init "$tmp_theme_dir" --theme <<< $'n\n' > /dev/null 2>&1
    test -f "$tmp_theme_dir/theme/index.hbs"
    sed 's|<script async src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.1/MathJax.js?config=TeX-AMS-MML_HTMLorMML"></script>|<script>window.MathJax = { tex2jax: { inlineMath: [["$","$"],["\\\\(","\\\\)"]], displayMath: [["$$","$$"],["\\\\[","\\\\]"]], processEscapes: true } };</script>\n        <script async src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.1/MathJax.js?config=TeX-AMS-MML_HTMLorMML"></script>|' "$tmp_theme_dir/theme/index.hbs" > theme/index.hbs
    rm -rf "$tmp_theme_dir"

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

# run all CI checks locally
ci-dry-run:
    #!/usr/bin/env bash
    set -euo pipefail
    export CARGO_INCREMENTAL=0
    export CARGO_TERM_COLOR=always
    GREEN='\033[0;32m' RED='\033[0;31m' NC='\033[0m'
    exit_code=0
    current=0
    STEPS=()
    FAILURES=()
    run_id="$(date +%Y%m%d-%H%M%S)"
    log_root="${PWD}/artifacts/ci-dry-run/${run_id}"
    mkdir -p "$log_root"

    add_step() {
        local name="$1" cmd="$2"
        STEPS+=("${name}:::${cmd}")
    }

    slugify() {
        printf '%s' "$1" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-'
    }

    run_step() {
        local name="$1" cmd="$2" slug log_path start_ts end_ts duration
        current=$((current + 1))
        slug="$(slugify "$name")"
        log_path="$(printf '%s/%02d-%s.log' "$log_root" "$current" "$slug")"
        printf "[%d/%d] %s... " "$current" "$total" "$name"
        start_ts="$(date +%s)"
        if bash -lc "$cmd" >"$log_path" 2>&1; then
            end_ts="$(date +%s)"
            duration=$((end_ts - start_ts))
            echo -e "${GREEN}OK${NC} (${duration}s)"
        else
            end_ts="$(date +%s)"
            duration=$((end_ts - start_ts))
            echo -e "${RED}FAIL${NC} (${duration}s)"
            echo "  log: $log_path"
            tail -n 30 "$log_path" | sed 's/^/    /'
            FAILURES+=("$name")
            exit_code=1
        fi
    }

    add_step "Preflight"                  "./scripts/preflight.sh"
    add_step "Format Check"               "nix develop ./nix/nightly --command cargo-fmt-nightly --all -- --check"
    add_step "Clippy"                     "cargo clippy --workspace --all-targets -- -D warnings"
    add_step "Tests"                      "cargo test --workspace"
    add_step "Docs Link Check"            "cargo xtask check docs-link-check"
    add_step "Proc Macro Scope"           "cargo xtask check proc-macro-scope"
    add_step "Test Boundaries"            "cargo xtask check test-boundaries"
    add_step "Trait Purity"               "cargo xtask check trait-purity"
    add_step "Crate Boundary"             "cargo xtask check crate-boundary"
    add_step "Ownership Invariants"       "cargo xtask check ownership-invariants"
    add_step "No usize in Models"         "cargo xtask check no-usize-in-models"
    add_step "Result Must Use"            "cargo xtask check result-must-use"
    add_step "Proof Bearing Actions"      "cargo xtask check proof-bearing-actions"
    add_step "Surface Classification"     "cargo xtask check surface-classification"
    add_step "Checkpoint Namespacing"     "cargo xtask check checkpoint-namespacing"
    add_step "Engine Service Boundary"    "cargo xtask check engine-service-boundary"
    add_step "Invariant Specs"            "cargo xtask check invariant-specs"
    add_step "Fail-Closed Ordering"       "cargo xtask check fail-closed-ordering"
    add_step "No Scratch Refs in Rust"    "cargo xtask check no-scratch-refs-in-rust"
    add_step "Pathway Choreography"          "cargo xtask check pathway-choreography"
    add_step "Pathway Choreography Validate" "cargo xtask check pathway-choreography --validate"
    add_step "Routing Invariants"         "cargo xtask check routing-invariants"
    add_step "Routing Invariants Validate" "cargo xtask check routing-invariants --validate"
    add_step "Install cargo-dylint"       "nix develop ./nix/nightly --command install-dylint"
    add_step "Dylint Trait Purity"        "nix develop ./nix/nightly --command cargo dylint --path lints/trait_purity --all -- --all-targets"
    add_step "Dylint Model Policy"        "nix develop ./nix/nightly --command cargo dylint --path lints/model_policy --all -- --all-targets"
    add_step "Dylint Routing Invariants"  "nix develop ./nix/nightly --command cargo dylint --path lints/routing_invariants --all -- --all-targets"
    add_step "Dylint Trait Must Use"      "nix develop ./nix/nightly --command cargo dylint --path lints/trait_must_use --all -- --all-targets"
    add_step "Dylint Naked Map Err"       "nix develop ./nix/nightly --command cargo dylint --path lints/naked_map_err --all -- --all-targets"
    add_step "Docs Semantic Drift"        "cargo xtask check docs-semantic-drift"
    add_step "Docs Build"                 "just book"

    total=${#STEPS[@]}
    echo "CI Dry Run"
    echo "=========="
    echo "Logs: $log_root"
    echo ""

    for step in "${STEPS[@]}"; do
        name="${step%%:::*}"
        cmd="${step#*:::}"
        run_step "$name" "$cmd"
    done

    echo ""
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}All CI checks passed${NC}"
    else
        echo "Failed:"
        for failure in "${FAILURES[@]}"; do
            echo "  - $failure"
        done
        echo -e "${RED}Some CI checks failed${NC}"
        exit 1
    fi

# fast environment sanity checks
ci-preflight:
    ./scripts/preflight.sh

# validate docs link integrity
docs-link-check:
    cargo xtask check docs-link-check

# detect stale backtick references in docs
docs-semantic-drift:
    cargo xtask check docs-semantic-drift

# enforce unit-test / integration-test boundary rules
test-boundaries:
    cargo xtask check test-boundaries

# enforce crate-level ownership documentation requirements
ownership-invariants:
    cargo xtask check ownership-invariants

# enforce routing correctness invariants
routing-invariants:
    cargo xtask check routing-invariants

# validate routing-invariant checks against seeded fixtures
routing-invariants-validate:
    cargo xtask check routing-invariants --validate

# enter nightly shell for dylint and rustc_private lints (run install-dylint once inside)
nightly-shell:
    nix develop ./nix/nightly

# install git hooks
install-hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed. Pre-commit checks will run automatically."
