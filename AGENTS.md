# AGENTS.md

Jacquard is a deterministic multi-engine routing system built around explicit router ownership, host bridges, and choreographic protocol support where an engine needs it. It uses Telltale for session types and choreography macros inside pathway, while the shared runtime model stays bridge-and-driver based rather than effect-stream based.

Jacquard is fully deterministic. No floating-point types, host-dependent ordering, or ambient randomness in routing or protocol state. Use typed time effects (`Tick`, `DurationMs`, `OrderStamp`, `RouteEpoch`) rather than raw wall-clock APIs or ad hoc `u64` timestamp fields.

See [Crate Architecture](docs/999_crate_architecture.md) for the dependency graph, cross-crate invariants, ownership rules, purity model, and extension boundary.

## Development

Enter the dev shell with `nix develop` or direnv (`direnv allow`).

```bash
just check          # cargo check --workspace
just build          # cargo build --workspace
just test           # cargo test --workspace
just lint           # cargo clippy --workspace --all-targets -- -D warnings
just fmt            # toolkit-owned nightly rustfmt policy
just fmt-check      # toolkit-owned nightly rustfmt policy with --check
just lean-style     # toolkit-owned Lean source-style policy over verification/
just lean-check     # lean-style, lean setup, then lake build
just no-std-check   # check portable no_std crates on host and thumbv7em-none-eabihf
just wasm-check     # build jacquard-pathway and jacquard-reference-client for wasm32-unknown-unknown
just wasm-test-reference-client # run the reference-client wasm integration test under wasm-bindgen-test
just book           # build mdbook docs (default recipe when running bare `just`)
just ci-dry-run     # run all CI checks locally (format, clippy, tests, toolkit/policy, dylint)
just install-hooks  # enable .githooks/pre-commit
just tuning-smoke   # run the small tuning matrix without generating the report
just tuning-local   # run the full local tuning matrix and generate the analysis report
just tuning-report <dir> # regenerate CSVs, plots, and the PDF from an existing artifact dir
just report-sanity  # validate generated analysis report artifacts without rerunning the matrix
just benchmark-audit # run the benchmark-audit regression surface without the full matrix
just docs-links     # validate external docs links (markdown-link-check)
just docs-link-check # validate internal docs link integrity via the toolkit xtask
just docs-semantic-drift # detect stale backtick references in docs
./scripts/toolkit-shell.sh <command> [args...]
./scripts/toolkit-shell.sh toolkit-xtask check <name> --repo-root . --config toolkit/toolkit.toml
./scripts/toolkit-shell.sh toolkit-install-dylint
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint <lint-name> <cargo-dylint args...>
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --lint-path ./toolkit/lints/<lint-name> <cargo-dylint args...>
cargo xtask check <name>
cargo xtask pre-commit
```

Run a single test: `cargo test -p <crate> <test_name>`

## Crate rules

`core` defines what exists. `traits` defines what components are allowed to do. `core` must not grow behavioral traits. All cross-crate behavioral interfaces belong in `traits`. `core` and `traits` must remain runtime-free.

The routing pipeline flows: `observation → estimate → fact → candidate → admission → materialization → publication`. Only the first three stages live in the shared world model. Candidate production and above happen through router and engine contracts.

`jacquard-host-support` exists for transport-neutral host support primitives only:
- bounded raw-ingress mailbox helpers
- unresolved/resolved peer bookkeeping
- in-flight claim ownership guards

`jacquard-host-support` must not grow:
- world-model vocabulary that belongs in `core`
- capability or driver traits that belong in `traits`
- transport-specific protocol logic or endpoint constructors
- Jacquard time or ordering assignment

Transport ownership is split deliberately:

- `TransportSenderEffects` is the shared synchronous send capability.
- `TransportDriver` is the host-owned ingress and supervision surface.
- Routers and engines must not own transport streams or assign `Tick`.
- Routers consume explicit ingress through router-owned ingestion APIs and advance synchronously through `advance_round`.
- Host bridges own ingress draining, batching, and time attachment.
- Transport-specific endpoint authoring belongs in transport-owned profile crates, not in `core`, `jacquard-host-support`, `jacquard-cast-support`, or the mem profile crates.

`macros` owns syntax-local code generation and annotation-site validation. The flake-input `toolkit` dependency owns portable nightly compiler-backed policy checks and generic fast-path checks. `toolkit/lints/` and `toolkit/xtask` own Jacquard-specific policy used by `just`, CI, and the pre-commit hook. Do not hide broad policy in generic proc macros when the rule belongs in an explicit lint or xtask check.

DualTide is the canonical home for the migrated research engine, paper, and theorem boundary. Jacquard keeps only legacy report compatibility fields needed to read historical `analysis/` and `analysis_2/` artifacts.

`jacquard-batman-bellman` is the enhanced BATMAN engine using local Bellman-Ford over a gossip-merged topology graph with TQ enrichment and a bootstrap shortcut. `jacquard-batman-classic` is the spec-faithful BATMAN IV engine with OGM-carried TQ, TTL-bounded propagation, and echo-only bidirectionality. `jacquard-babel` implements RFC 8966 with bidirectional ETX link cost, additive metric, and a feasibility distance table for loop-free route selection.

`jacquard-olsrv2` implements an OLSRv2-class proactive link-state engine with deterministic MPR election and TC-style topology flooding. `jacquard-scatter` is the bounded deferred-delivery diffusion engine and publishes opaque viability claims. `jacquard-mercator` is a hybrid corridor routing engine skeleton under active development and has no dedicated `docs/` entry yet.

`jacquard-simulator` is the scenario and replay harness above the shared boundaries. It reuses reference-client bridge ownership and round advancement rather than maintaining a simulator-only stack. The `tuning_matrix` binary runs experiment suites and generates analysis reports via `python3 -m analysis.report`. Artifacts land under `artifacts/analysis/{suite}/{timestamp}/` with a `latest` symlink.

The `analysis/` directory contains a Python pipeline (polars + Altair + reportlab) that reads simulator artifacts and generates a PDF report with per-engine recommendations, transition metrics, failure boundaries, cross-engine comparisons, and diffusion analysis.

Canonical host wiring examples live in `crates/reference-client/tests/e2e_pathway_shared_network.rs`, `crates/reference-client/tests/e2e_batman_pathway_handoff.rs`, `crates/reference-client/tests/e2e_olsrv2_shared_network.rs`, `crates/reference-client/tests/e2e_olsrv2_pathway_handoff.rs`, and the shared scenarios under `crates/testkit/src/reference_client_scenarios.rs`.

## Documentation layout

`docs/` is organized so every file belongs to one of three categories. Specs (100s through 300s and 999) describe shared shape and contracts. Implementation specs (400s) describe per-engine and per-profile behavior. Guides (500s) walk a 3rd-party developer through using the system. Start at `docs/101_introduction.md` for orientation, `docs/503_client_assembly.md` for the fastest library-use path, and `docs/502_running_experiments.md` for the analytical path.

## Policy checks

Run generic policy checks with `./scripts/toolkit-shell.sh toolkit-xtask check <name> --repo-root . --config toolkit/toolkit.toml` and Jacquard-specific checks with `cargo xtask check <name>`. Key categories:

- **Boundary**: `adapter-boundary`, `crate-boundary`, `engine-service-boundary`, `pathway-async-boundary`, `transport-authoring-boundary`, `transport-ownership-boundary`, `reference-bridge-boundary`
- **Routing invariants**: `routing-invariants`, `fail-closed-ordering`, `router-round-boundary`, `pathway-choreography` (pass `--validate` to run fixtures)
- **Docs**: `docs-link-check`, `docs-semantic-drift`, `invariant-specs`, `no-scratch-refs-in-rust`
- **Surface / DX**: `dx-surface`, `surface-classification`, `trait-purity`, `result-must-use`, `proof-bearing-actions`, `proc-macro-scope`, `ownership-invariants`, `long-file`
- **Model**: `no-usize-in-models`, `checkpoint-namespacing`, `test-boundaries`

For nightly compiler-backed lint parity, use the toolkit wrappers for portable lints and the repo nightly shell for Jacquard-specific lints:

```bash
./scripts/toolkit-shell.sh toolkit-install-dylint
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint trait_purity --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --lint-path ./toolkit/lints/model_policy --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --lint-path ./toolkit/lints/routing_invariants --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint trait_must_use --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint naked_map_err --all -- --all-targets
```

For Lean verification work, use `just lean-style` for targeted source-policy
checking and `nix develop --command just lean-check` for the full local style +
build path. The CI lane currently blocks on Lean style only; full Lean build is
still local-only while the verification package depends on sibling path
checkouts.

## Test layout

Unit tests co-locate with the module they cover. Higher-level tests go in `tests/` subdirectories by type (`integration/`, `regression/`, `property/`).

- `jacquard-core`: type invariants, canonical encoding, boundedness, deterministic ordering, content-addressing stability.
- `jacquard-traits`: compile-only surface checks, trait-object and generic-boundary tests.
- `jacquard-host-support`: transport-neutral ingress mailbox, peer-directory, and claim-guard helpers with explicit ownership semantics.
- `jacquard-cast-support`: deterministic bounded unicast, multicast, and broadcast evidence helpers shared by transport profiles and cast-capable engines.
- `jacquard-pathway`: deterministic candidate production, admission/materialization, commitment tracking, forwarding, repair, topology-change, observation handling.
- `jacquard-router`: control-plane selection, ownership, capability enforcement, canonical handle issuance, lease expiry, explicit ingress, synchronous round advancement, fallback legality, adaptive-profile derivation.
- `jacquard-reference-client`: host-side bridge composition of router + pathway/batman + in-memory profiles for end-to-end tests.
- `jacquard-batman-bellman`: next-hop ranking, TQ derivation with enrichment, Bellman-Ford path computation, gossip integration, bootstrap transition, and router integration.
- `jacquard-batman-classic`: spec-faithful OGM-carried TQ, echo-only bidirectionality, receive-window occupancy, and router integration.
- `jacquard-babel`: ETX link cost, additive metric, feasibility distance table, seqno ordering, and router integration.
- `jacquard-olsrv2`: HELLO-driven neighbor learning, deterministic MPR election, TC flooding, SPF derivation, and router integration.
- `jacquard-scatter`: retained-message expiry, replication budgets, opportunistic forwarding, and bounded custody handoff.
- `jacquard-mercator`: corridor evidence accumulation and planner skeleton (engine under active development).
- `jacquard-macros`: proc-macro compile checks and trybuild UI regression tests for annotation contracts.
- `jacquard-mem-link-profile`: transport, retention, and runtime-effect adapter integration.
- `jacquard-mem-node-profile`: node profile and capability modeling.
- `jacquard-testkit`: shared scenario helpers consumed by the simulator and reference-client test suites.
- `jacquard-simulator`: scenario smoke tests (all seven engines), composition tests, regression tests, tuning parameter sweeps, and replay round-trip.

## Telltale dependency

Telltale crates are pinned from crates.io through the workspace `[workspace.dependencies]` table (`telltale`, `telltale-types`, `telltale-macros`, `telltale-runtime`, `telltale-search`, `telltale-simulator`). Individual crates import them via `{ workspace = true }`.

## long-block-exception

The `model_policy` dylint caps function bodies at 60 source lines (measured
from the opening brace to the closing brace, inclusive). Long bodies usually
mean a helper should be extracted. When a body legitimately needs to stay
long, for example a match statement that mirrors a shared enum one-to-one or a
fixture constructor that assembles a full world sample, add an exception
marker directly above the `fn` signature:

```rust
// long-block-exception: <non-empty reason>
fn assemble_world() -> Configuration {
    // ... > 60 lines ...
}
```

Blank lines, doc comments, and `#[attr]` attributes between the marker and
the signature are allowed. The reason text must be non-empty so every
exception stays auditable in code review. Prefer splitting the function
first; reach for the exception marker only when extraction would obscure
the mapping.

## long-file-exception

The `long-file` xtask check caps each `.rs` file under `crates/` at 800
total source lines. Oversized files should be split into submodules
grouped by concern (types, pure reducers, effectful entry points,
protocol/choreography, per-object state) rather than by size. Two
escape hatches exist:

- a `// long-file-exception: <non-empty reason>` comment inside the
  first 40 lines of the file, for deliberate long-lived exceptions, or
- a `[[toolkit.exemptions.long_file]]` entry in `toolkit/toolkit.toml`
  with a `path` and `reason`, for inherited oversized files tracked
  for a future split.

Prefer splitting the file first; reach for the marker only when
splitting would obscure the code. Every toml exemption is tracked tech
debt — delete the entry once the file is split below the cap.
