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

# run the small tuning matrix without generating the analysis report
tuning-smoke:
    nix develop --command cargo run -p jacquard-simulator --bin tuning_matrix -- smoke
    @echo "Tuning smoke artifacts: artifacts/analysis/smoke/latest"

# run the full local tuning matrix and generate the analysis report
tuning-local:
    nix develop --command cargo run -p jacquard-simulator --bin tuning_matrix -- local
    @echo "Tuning local artifacts: artifacts/analysis/local/latest"
    @echo "Report: artifacts/analysis/local/latest/router-tuning-report.pdf"

# regenerate CSVs, plots, and the analysis report for one existing tuning artifact directory
tuning-report artifact_dir='artifacts/analysis/local/latest':
    #!/usr/bin/env bash
    set -euo pipefail
    nix develop --command python3 -m analysis.report "{{artifact_dir}}"

# validate generated analysis report artifacts without rerunning the matrix
report-sanity artifact_dir='artifacts/analysis/local/latest':
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -d "{{artifact_dir}}/report" ] || [ "$(basename "{{artifact_dir}}")" = "report" ]; then
      nix develop --command python3 -m analysis.sanity "{{artifact_dir}}"
    else
      echo "report-sanity: skipped; {{artifact_dir}}/report does not exist"
    fi

# run the benchmark-audit regression surface without the full tuning matrix
benchmark-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo test -p jacquard-simulator comparison_families_document_activation_rounds_and_active_windows
    cargo test -p jacquard-simulator mixed_comparison_high_loss_prefers_the_next_hop_engine_that_keeps_the_route_up
    cargo test -p jacquard-simulator mixed_comparison_partial_observability_is_not_masked_by_batman_bellman
    cargo test -p jacquard-simulator mixed_comparison_concurrent_family_records_a_real_engine_handoff
    cargo test -p jacquard-simulator comparison_connected_high_loss_is_seed_stable_under_scripted_hooks
    cargo test -p jacquard-simulator congestion_cascade_tracks_cluster_coverage_separately_from_node_coverage
    cargo test -p jacquard-simulator adversarial_observation_reports_non_zero_leakage_for_broad_baseline
    cargo test -p jacquard-simulator bounded_state_classifies_regions
    cargo test -p jacquard-simulator energy_starved_relay_separates_conservative_and_broad_profiles
    nix develop --command python3 -m unittest analysis.tests.test_scoring analysis.tests.test_sanity

# format code (uses the toolkit-owned nightly rustfmt policy)
fmt:
    {{fmt_cmd}} --all

# check formatting (uses the toolkit-owned nightly rustfmt policy)
fmt-check:
    {{fmt_cmd}} --all -- --check

# verify Pathway and the reference client compile for wasm32
wasm-check:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v rustup >/dev/null 2>&1; then
      ./scripts/wasm-host-toolchain.sh cargo build --lib --target wasm32-unknown-unknown -p jacquard-pathway
      ./scripts/wasm-host-toolchain.sh cargo build --lib --target wasm32-unknown-unknown -p jacquard-reference-client
    elif command -v nix >/dev/null 2>&1; then
      nix develop --command just wasm-check
    else
      echo "wasm-check requires either rustup or nix" >&2
      exit 1
    fi

# verify the shared portable crates compile without std
no-std-check:
    cargo check -p jacquard-core --no-default-features
    cargo check -p jacquard-traits --no-default-features
    cargo check -p jacquard-cast-support --no-default-features
    cargo check -p jacquard-mercator --no-default-features
    cargo check -p jacquard-router --no-default-features

# execute the wasm reference-client integration test under wasm-bindgen-test
wasm-test-reference-client:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v rustup >/dev/null 2>&1 && command -v node >/dev/null 2>&1; then
      ./scripts/wasm-host-toolchain.sh cargo test --target wasm32-unknown-unknown -p jacquard-reference-client --test wasm_smoke
    elif command -v nix >/dev/null 2>&1; then
      nix develop --command just wasm-test-reference-client
    else
      echo "wasm-test-reference-client requires rustup + node or nix" >&2
      exit 1
    fi

# Generate docs/SUMMARY.md from Markdown files in docs/
summary:
    ./scripts/gen-summary.sh

# Generate transient build assets (mermaid, mathjax theme override)
_gen-assets:
    #!/usr/bin/env bash
    set -euo pipefail
    mdbook-mermaid install . > /dev/null
    test -f mermaid-init.js
    test -f mermaid.min.js
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
    add_step "Generate Summary"           "./scripts/gen-summary.sh"
    add_step "Clippy"                     "cargo clippy --workspace --all-targets -- -D warnings"
    add_step "Tests"                      "cargo test --workspace"
    add_step "Benchmark Audit"           "just benchmark-audit"
    add_step "Report Sanity"             "just report-sanity"
    add_step "Lean Style"                 "just lean-style"
    add_step "Wasm Check"                 "just wasm-check"
    add_step "Wasm Reference Client Test" "just wasm-test-reference-client"
    add_step "Docs Links"                 "npx --yes markdown-link-check -q -c .github/config/markdown-link-check.json docs"
    add_step "Docs Link Check"            "{{toolkit_cmd}} check docs-link-check --repo-root . --config toolkit/toolkit.toml"
    add_step "Proc Macro Scope"           "{{toolkit_cmd}} check proc-macro-scope --repo-root . --config toolkit/toolkit.toml"
    add_step "Crate Root Policy"          "{{toolkit_cmd}} check crate-root-policy --repo-root . --config toolkit/toolkit.toml"
    add_step "Ignored Result"             "{{toolkit_cmd}} check ignored-result --repo-root . --config toolkit/toolkit.toml"
    add_step "Unsafe Boundary"            "{{toolkit_cmd}} check unsafe-boundary --repo-root . --config toolkit/toolkit.toml"
    add_step "Bool Param"                 "{{toolkit_cmd}} check bool-param --repo-root . --config toolkit/toolkit.toml"
    add_step "Must Use Public Return"     "{{toolkit_cmd}} check must-use-public-return --repo-root . --config toolkit/toolkit.toml"
    add_step "Assert Shape"               "{{toolkit_cmd}} check assert-shape --repo-root . --config toolkit/toolkit.toml"
    add_step "Drop Side Effects"          "{{toolkit_cmd}} check drop-side-effects --repo-root . --config toolkit/toolkit.toml"
    add_step "Recursion Guard"            "{{toolkit_cmd}} check recursion-guard --repo-root . --config toolkit/toolkit.toml"
    add_step "Naming Units"               "{{toolkit_cmd}} check naming-units --repo-root . --config toolkit/toolkit.toml"
    add_step "Limit Constant"             "{{toolkit_cmd}} check limit-constant --repo-root . --config toolkit/toolkit.toml"
    add_step "Public Type Width"          "{{toolkit_cmd}} check public-type-width --repo-root . --config toolkit/toolkit.toml"
    add_step "Dependency Policy"          "{{toolkit_cmd}} check dependency-policy --repo-root . --config toolkit/toolkit.toml"
    add_step "Test Boundaries"            "{{toolkit_cmd}} check test-boundaries --repo-root . --config toolkit/toolkit.toml"
    add_step "Lean Escape Hatches"        "{{toolkit_cmd}} check lean-escape-hatches --repo-root . --config toolkit/toolkit.toml"
    add_step "Text Formatting"            "{{toolkit_cmd}} check text-formatting --repo-root . --config toolkit/toolkit.toml"
    add_step "Workspace Hygiene"          "{{toolkit_cmd}} check workspace-hygiene --repo-root . --config toolkit/toolkit.toml"
    add_step "Workflow Actions"           "{{toolkit_cmd}} check workflow-actions --repo-root . --config toolkit/toolkit.toml"
    add_step "Trait Purity"               "{{policy_cmd}} check trait-purity"
    add_step "Annotation Semantics"       "{{policy_cmd}} check annotation-semantics"
    add_step "Crate Boundary"             "{{policy_cmd}} check crate-boundary"
    add_step "Adapter Boundary"           "{{policy_cmd}} check adapter-boundary"
    add_step "DX Surface"                "{{policy_cmd}} check dx-surface"
    add_step "DRY Code"                  "{{policy_cmd}} check dry-code"
    add_step "Transport Authoring Boundary" "{{policy_cmd}} check transport-authoring-boundary"
    add_step "Transport Ownership Boundary" "{{policy_cmd}} check transport-ownership-boundary"
    add_step "Router Round Boundary"     "{{policy_cmd}} check router-round-boundary"
    add_step "Reference Bridge Boundary" "{{policy_cmd}} check reference-bridge-boundary"
    add_step "Simulator Boundary"        "{{policy_cmd}} check simulator-boundary"
    add_step "Ownership Invariants"       "{{policy_cmd}} check ownership-invariants"
    add_step "No usize in Models"         "{{policy_cmd}} check no-usize-in-models"
    add_step "Result Must Use"            "{{toolkit_cmd}} check result-must-use --repo-root . --config toolkit/toolkit.toml"
    add_step "Proof Bearing Actions"      "{{policy_cmd}} check proof-bearing-actions"
    add_step "Surface Classification"     "{{policy_cmd}} check surface-classification"
    add_step "Rust Style Guide"           "{{policy_cmd}} check rust-style-guide"
    add_step "Field Code Map"             "{{policy_cmd}} check field-code-map"
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
    add_step "Dylint Model Policy"        "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --lint-path ./toolkit/lints/model_policy --all -- --all-targets"
    add_step "Dylint Routing Invariants"  "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --lint-path ./toolkit/lints/routing_invariants --all -- --all-targets"
    add_step "Dylint Trait Must Use"      "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --toolkit-lint trait_must_use --all -- --all-targets"
    add_step "Dylint Naked Map Err"       "env CARGO_INCREMENTAL=0 {{toolkit_dylint}} --toolkit-lint naked_map_err --all -- --all-targets"
    add_step "Docs Semantic Drift"        "{{toolkit_cmd}} check docs-semantic-drift --repo-root . --config toolkit/toolkit.toml"
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
    {{toolkit_cmd}} check docs-link-check --repo-root . --config toolkit/toolkit.toml

# detect stale backtick references in docs
docs-semantic-drift:
    {{toolkit_cmd}} check docs-semantic-drift --repo-root . --config toolkit/toolkit.toml

# enforce unit-test / integration-test boundary rules
test-boundaries:
    {{toolkit_cmd}} check test-boundaries --repo-root . --config toolkit/toolkit.toml

# enforce crate-level ownership documentation requirements
ownership-invariants:
    {{policy_cmd}} check ownership-invariants

# enforce routing correctness invariants
routing-invariants:
    {{policy_cmd}} check routing-invariants

# enforce mechanized Rust style-guide rules
rust-style-guide:
    {{policy_cmd}} check rust-style-guide

dry-code:
    {{policy_cmd}} check dry-code

field-code-map:
    {{policy_cmd}} check field-code-map

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

# Run the generic Lean source-style policy over the verification tree.
lean-style:
    {{toolkit_cmd}} check lean-style --repo-root . --config toolkit/toolkit.toml

# Build the Lean verification package (requires lean-setup to have been run once).
lean-build:
    cd verification && lake build

# Run Lean source-style policy, hydrate the cache if needed, then build.
lean-check:
    just lean-style
    just lean-setup
    cd verification && lake build

# install git hooks
install-hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed. Pre-commit checks will run automatically."
