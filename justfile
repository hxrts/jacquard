default: book
    @just --list

toolkit_shell_cmd := "./scripts/toolkit-shell.sh"
toolkit_cmd := "./scripts/toolkit-shell.sh toolkit-xtask"
toolkit_dylint := "./scripts/toolkit-shell.sh toolkit-dylint --repo-root ."
install_dylint_cmd := "./scripts/toolkit-shell.sh toolkit-install-dylint"
policy_cmd := "cargo xtask"
fmt_cmd := "./scripts/toolkit-shell.sh toolkit-fmt"

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

# format code (uses the toolkit-owned nightly rustfmt policy)
fmt:
    {{fmt_cmd}} --all

# check formatting (uses the toolkit-owned nightly rustfmt policy)
fmt-check:
    {{fmt_cmd}} --all -- --check

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
    add_step "Format Check"               "{{fmt_cmd}} --all -- --check"
    add_step "Clippy"                     "cargo clippy --workspace --all-targets -- -D warnings"
    add_step "Tests"                      "cargo test --workspace"
    add_step "Docs Links"                 "npx --yes markdown-link-check -q -c .github/config/markdown-link-check.json docs"
    add_step "Docs Link Check"            "{{toolkit_cmd}} check docs-link-check --repo-root . --config policy/toolkit.toml"
    add_step "Proc Macro Scope"           "{{toolkit_cmd}} check proc-macro-scope --repo-root . --config policy/toolkit.toml"
    add_step "Test Boundaries"            "{{toolkit_cmd}} check test-boundaries --repo-root . --config policy/toolkit.toml"
    add_step "Trait Purity"               "{{policy_cmd}} check trait-purity"
    add_step "Crate Boundary"             "{{policy_cmd}} check crate-boundary"
    add_step "Adapter Boundary"           "{{policy_cmd}} check adapter-boundary"
    add_step "DX Surface"                "{{policy_cmd}} check dx-surface"
    add_step "Transport Authoring Boundary" "{{policy_cmd}} check transport-authoring-boundary"
    add_step "Transport Ownership Boundary" "{{policy_cmd}} check transport-ownership-boundary"
    add_step "Router Round Boundary"     "{{policy_cmd}} check router-round-boundary"
    add_step "Reference Bridge Boundary" "{{policy_cmd}} check reference-bridge-boundary"
    add_step "Simulator Boundary"        "{{policy_cmd}} check simulator-boundary"
    add_step "Ownership Invariants"       "{{policy_cmd}} check ownership-invariants"
    add_step "No usize in Models"         "{{policy_cmd}} check no-usize-in-models"
    add_step "Result Must Use"            "{{toolkit_cmd}} check result-must-use --repo-root . --config policy/toolkit.toml"
    add_step "Proof Bearing Actions"      "{{policy_cmd}} check proof-bearing-actions"
    add_step "Surface Classification"     "{{policy_cmd}} check surface-classification"
    add_step "Rust Style Guide"           "{{policy_cmd}} check rust-style-guide"
    add_step "Checkpoint Namespacing"     "{{policy_cmd}} check checkpoint-namespacing"
    add_step "Engine Service Boundary"    "{{policy_cmd}} check engine-service-boundary"
    add_step "Invariant Specs"            "{{policy_cmd}} check invariant-specs"
    add_step "Fail-Closed Ordering"       "{{policy_cmd}} check fail-closed-ordering"
    add_step "No Scratch Refs in Rust"    "{{policy_cmd}} check no-scratch-refs-in-rust"
    add_step "Pathway Async Boundary"     "{{policy_cmd}} check pathway-async-boundary"
    add_step "Pathway Choreography"          "{{policy_cmd}} check pathway-choreography"
    add_step "Pathway Choreography Validate" "{{policy_cmd}} check pathway-choreography --validate"
    add_step "Routing Invariants"         "{{policy_cmd}} check routing-invariants"
    add_step "Routing Invariants Validate" "{{policy_cmd}} check routing-invariants --validate"
    add_step "Install cargo-dylint"       "{{install_dylint_cmd}}"
    add_step "Dylint Trait Purity"        "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --toolkit-lint trait_purity --all -- --all-targets"
    add_step "Dylint Model Policy"        "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --lint-path ./policy/lints/model_policy --all -- --all-targets"
    add_step "Dylint Routing Invariants"  "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --lint-path ./policy/lints/routing_invariants --all -- --all-targets"
    add_step "Dylint Trait Must Use"      "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --toolkit-lint trait_must_use --all -- --all-targets"
    add_step "Dylint Naked Map Err"       "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --toolkit-lint naked_map_err --all -- --all-targets"
    add_step "Docs Semantic Drift"        "{{toolkit_cmd}} check docs-semantic-drift --repo-root . --config policy/toolkit.toml"
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

# validate external docs links
docs-links:
    npx --yes markdown-link-check -q -c .github/config/markdown-link-check.json docs

# validate internal docs link integrity
docs-link-check:
    {{toolkit_cmd}} check docs-link-check --repo-root . --config policy/toolkit.toml

# detect stale backtick references in docs
docs-semantic-drift:
    {{toolkit_cmd}} check docs-semantic-drift --repo-root . --config policy/toolkit.toml

# enforce unit-test / integration-test boundary rules
test-boundaries:
    {{toolkit_cmd}} check test-boundaries --repo-root . --config policy/toolkit.toml

# enforce crate-level ownership documentation requirements
ownership-invariants:
    {{policy_cmd}} check ownership-invariants

# enforce routing correctness invariants
routing-invariants:
    {{policy_cmd}} check routing-invariants

# enforce mechanized Rust style-guide rules
rust-style-guide:
    {{policy_cmd}} check rust-style-guide

# validate routing-invariant checks against seeded fixtures
routing-invariants-validate:
    {{policy_cmd}} check routing-invariants --validate

# enter the pinned toolkit shell for nightly formatter and dylint commands
toolkit-shell:
    {{toolkit_shell_cmd}} bash -lc 'exec "${SHELL:-bash}" -l'

# backwards-compatible alias for the toolkit shell
nightly-shell: toolkit-shell

install-dylint:
    {{install_dylint_cmd}}

# Publish workspace crates to crates.io and cut a release tag.
# Usage:
#   just release <version> [dry_run] [skip_ci] [no_tag] [push] [allow_dirty] [no_require_main]
# Example:
#   just release 0.3.0 true true true false true false   # dry-run + skip ci + no-tag + allow dirty
release \
  version="" \
  dry_run="false" \
  skip_ci="false" \
  no_tag="false" \
  push="false" \
  allow_dirty="false" \
  no_require_main="false":
    #!/usr/bin/env bash
    set -euo pipefail
    args=()
    if [ -n "{{version}}" ]; then
      args+=(--version "{{version}}")
    fi
    if [ "{{dry_run}}" = "true" ]; then
      args+=(--dry-run)
    fi
    if [ "{{skip_ci}}" = "true" ]; then
      args+=(--skip-ci)
    fi
    if [ "{{no_tag}}" = "true" ]; then
      args+=(--no-tag)
    fi
    if [ "{{push}}" = "true" ]; then
      args+=(--push)
    fi
    if [ "{{allow_dirty}}" = "true" ]; then
      args+=(--allow-dirty)
    fi
    if [ "{{no_require_main}}" = "true" ]; then
      args+=(--no-require-main)
    fi
    ./scripts/release-publish.sh "${args[@]}"

# Hydrate the Mathlib olean cache so lean-build never rebuilds Mathlib from source.
# Must be run once from inside `nix develop` before lean-build will work.
lean-setup:
    ./scripts/lean-prebuilt.sh

# Build the Lean verification package (requires lean-setup to have been run once).
lean-build:
    cd verification && lake build

# install git hooks
install-hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed. Pre-commit checks will run automatically."
