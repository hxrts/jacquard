# CLAUDE.md

Jacquard is a deterministic multi-engine routing system built around explicit router ownership, host bridges, and choreographic protocol support where an engine needs it. It uses Telltale for session types and choreography macros inside pathway, while the shared runtime model stays bridge-and-driver based rather than effect-stream based.

Jacquard is fully deterministic. No floating-point types, host-dependent ordering, or ambient randomness in routing or protocol state. Use typed time effects (`Tick`, `DurationMs`, `OrderStamp`, `RouteEpoch`) rather than raw wall-clock APIs or ad hoc `u64` timestamp fields.

See [Crate Architecture](docs/999_crate_architecture.md) for the dependency graph, cross-crate invariants, ownership rules, purity model, and extension boundary.

## Development

Enter the dev shell with `nix develop` or direnv (`direnv allow`).

```
just check          # cargo check --workspace
just build          # cargo build --workspace
just test           # cargo test --workspace
just lint           # cargo clippy --workspace --all-targets -- -D warnings
just fmt            # toolkit-owned nightly rustfmt policy
just fmt-check      # toolkit-owned nightly rustfmt policy with --check
just wasm-check     # build jacquard-pathway and jacquard-reference-client for wasm32-unknown-unknown
just wasm-test-reference-client # run the reference-client wasm integration test under wasm-bindgen-test
just book           # build mdbook docs (default recipe when running bare `just`)
just ci-dry-run     # run all CI checks locally (format, clippy, tests, toolkit/policy, dylint)
just install-hooks  # enable .githooks/pre-commit
./scripts/toolkit-shell.sh <command> [args...]
./scripts/toolkit-shell.sh toolkit-xtask check <name> --repo-root . --config policy/toolkit.toml
./scripts/toolkit-shell.sh toolkit-install-dylint
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint <lint-name> <cargo-dylint args...>
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --lint-path ./policy/lints/<lint-name> <cargo-dylint args...>
cargo run --manifest-path policy/xtask/Cargo.toml -- check <name>
cargo run --manifest-path policy/xtask/Cargo.toml -- pre-commit
```

Run a single test: `cargo test -p <crate> <test_name>`

## Crate rules

`core` defines what exists. `traits` defines what components are allowed to do. `core` must not grow behavioral traits. All cross-crate behavioral interfaces belong in `traits`. `core` and `traits` must remain runtime-free.

The routing pipeline flows: `observation → estimate → fact → candidate → admission → materialization → publication`. Only the first three stages live in the shared world model. Candidate production and above happen through router and engine contracts.

`adapter` exists for transport-neutral adapter support primitives only:
- bounded raw-ingress mailbox helpers
- unresolved/resolved peer bookkeeping
- in-flight claim ownership guards

`adapter` must not grow:
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
- Transport-specific endpoint authoring belongs in transport-owned profile crates, not in `core`, `adapter`, or the mem profile crates.

`macros` owns syntax-local code generation and annotation-site validation. The flake-input `rust-toolkit` dependency owns portable nightly compiler-backed policy checks and generic fast-path checks. `policy/lints/` and `policy/xtask` own Jacquard-specific policy used by `just`, CI, and the pre-commit hook. Do not hide broad policy in generic proc macros when the rule belongs in an explicit lint or xtask check.

`jacquard-field` owns field-private posterior state, mean-field compression, regime/posture control state, and continuation scoring. Like pathway and batman, field-private choreography may supply only observational evidence into the deterministic local controller — canonical route publication remains router-owned.

`jacquard-simulator` is the scenario/replay harness above the shared boundaries. It reuses reference-client bridge ownership and round advancement; it does not maintain a simulator-only stack.

The canonical host wiring reference is `crates/reference-client/tests/e2e_multi_layer_routing.rs`.

## Policy checks

Run generic policy checks with `./scripts/toolkit-shell.sh toolkit-xtask check <name> --repo-root . --config policy/toolkit.toml` and Jacquard-specific checks with `cargo run --manifest-path policy/xtask/Cargo.toml -- check <name>`. Key categories:

- **Boundary**: `adapter-boundary`, `crate-boundary`, `engine-service-boundary`, `pathway-async-boundary`, `transport-authoring-boundary`, `transport-ownership-boundary`, `reference-bridge-boundary`
- **Routing invariants**: `routing-invariants`, `fail-closed-ordering`, `router-round-boundary`, `pathway-choreography` (pass `--validate` to run fixtures)
- **Docs**: `docs-link-check`, `docs-semantic-drift`, `invariant-specs`, `no-scratch-refs-in-rust`
- **Surface / DX**: `dx-surface`, `surface-classification`, `trait-purity`, `result-must-use`, `proof-bearing-actions`, `proc-macro-scope`, `ownership-invariants`
- **Model**: `no-usize-in-models`, `checkpoint-namespacing`, `test-boundaries`

For nightly compiler-backed lint parity, use the toolkit wrappers for portable lints and the repo nightly shell for Jacquard-specific lints:

```bash
./scripts/toolkit-shell.sh toolkit-install-dylint
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint trait_purity --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --lint-path ./policy/lints/model_policy --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --lint-path ./policy/lints/routing_invariants --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint trait_must_use --all -- --all-targets
./scripts/toolkit-shell.sh toolkit-dylint --repo-root . --toolkit-lint naked_map_err --all -- --all-targets
```

## Test layout

Unit tests co-locate with the module they cover. Higher-level tests go in `tests/` subdirectories by type (`integration/`, `regression/`, `property/`).

- `jacquard-core`: type invariants, canonical encoding, boundedness, deterministic ordering, content-addressing stability.
- `jacquard-traits`: compile-only surface checks, trait-object and generic-boundary tests.
- `jacquard-adapter`: transport-neutral ingress mailbox, peer-directory, and claim-guard helpers with explicit ownership semantics.
- `jacquard-pathway`: deterministic candidate production, admission/materialization, commitment tracking, forwarding, repair, topology-change, observation handling.
- `jacquard-router`: control-plane selection, ownership, capability enforcement, canonical handle issuance, lease expiry, explicit ingress, synchronous round advancement, fallback legality, adaptive-profile derivation.
- `jacquard-reference-client`: host-side bridge composition of router + pathway/batman + in-memory profiles for end-to-end tests.

## Telltale dependency

Telltale crates are pinned from crates.io through the workspace `[workspace.dependencies]` table (`telltale`, `telltale-types`, `telltale-macros`, `telltale-runtime`, currently `12.0.0`). Individual crates import them via `{ workspace = true }`.

## long-block-exception

The `model_policy` dylint caps function bodies at 60 source lines (measured
from the opening brace to the closing brace, inclusive). Long bodies usually
mean a helper should be extracted. When a body legitimately needs to stay
long — e.g. a match statement that mirrors a shared enum one-to-one, or a
fixture constructor that assembles a full world sample — add an exception
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
