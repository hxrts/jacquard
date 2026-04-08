# CLAUDE.md

Jacquard is an adaptive mesh routing system built on choreographic protocols. It uses Telltale for session types, choreography macros, and the effect-based runtime.

Jacquard is fully deterministic. No floating-point types, host-dependent ordering, or ambient randomness in routing or protocol state. Use typed time effects (`Tick`, `DurationMs`, `OrderStamp`, `RouteEpoch`) rather than raw wall-clock APIs or ad hoc `u64` timestamp fields.

See [Crate Architecture](docs/999_crate_architecture.md) for the dependency graph, cross-crate invariants, ownership rules, purity model, and extension boundary.

## Development

Enter the dev shell with `nix develop` or direnv (`direnv allow`).

```
just check          # cargo check --workspace
just build          # cargo build --workspace
just test           # cargo test --workspace
just lint           # cargo clippy --workspace -- -D warnings
just fmt            # cargo fmt --all
just book           # build mdbook docs (default recipe)
just ci-dry-run     # run all CI checks locally
just install-hooks  # enable .githooks/pre-commit
cargo xtask check <name>   # run one policy/doc check directly
cargo xtask pre-commit     # run the staged-file pre-commit lane manually
```

Run a single test: `cargo test -p <crate> <test_name>`

## Crate rules

`core` defines what exists. `traits` defines what components are allowed to do. `core` must not grow behavioral traits. All cross-crate behavioral interfaces belong in `traits`. `core` and `traits` must remain runtime-free.

`macros` owns syntax-local code generation and annotation-site validation. `lints/` owns nightly compiler-backed policy checks. `crates/xtask` owns the stable fast-path workspace checks used by `just`, CI, and the pre-commit hook. Do not hide broad policy in generic proc macros when the rule belongs in an explicit lint or xtask check.

Run individual policy checks with `cargo xtask check <crate-boundary|docs-link-check|docs-semantic-drift|no-usize-in-models|ownership-invariants|proc-macro-scope|routing-invariants|trait-purity>`.

For routing-invariant fixture validation, use:

```bash
cargo xtask check routing-invariants --validate
```

For nightly compiler-backed lint parity, use the nightly shell, run `install-dylint`, and then run:

```bash
cargo dylint --path lints/trait_purity --all -- --all-targets
cargo dylint --path lints/model_policy --all -- --all-targets
cargo dylint --path lints/routing_invariants --all -- --all-targets
```

## Test layout

Unit tests co-locate with the module they cover. Higher-level tests go in `tests/` subdirectories by type (`integration/`, `regression/`, `property/`).

### Testing focus by crate

- `jacquard-core`: type invariants, canonical encoding, boundedness, deterministic ordering, content-addressing stability.
- `jacquard-traits`: compile-only surface checks, trait-object and generic-boundary tests.
- `jacquard-mesh`: deterministic candidate production, admission/materialization, commitment tracking, forwarding, repair, topology-change, observation handling.
- `jacquard-router`: control-plane selection, ownership, capability enforcement, canonical handle issuance, lease expiry, fallback legality, anti-entropy, adaptive-profile derivation.
- `jacquard-mem-node-profile`: deterministic node-profile and node-state builders with no routing-engine knowledge.
- `jacquard-mem-link-profile`: in-memory link-profile, carrier, retention, and runtime-effect adapters with no routing semantics.
- `jacquard-reference-client`: host-side composition of router + mesh + in-memory profiles for end-to-end tests.
- `jacquard-simulator`: scenario execution, replay, checkpoint/resume, regression scenarios across sparse, dense, partitioned, and adversarial settings.

## Telltale dependency

Telltale crates are pinned from crates.io through the workspace `[workspace.dependencies]` table (`telltale`, `telltale-types`, `telltale-macros`, `telltale-runtime`, currently `11.3.0`). Individual crates import them via `{ workspace = true }`.

## long-block-exception

Some test helpers and fixture functions use `executor::block_on(async { ... })` or
similar blocking constructs. Where Clippy or a lint warns about blocking in async
context, a `// long-block-exception: <reason>` comment suppresses the warning.

Rules for when this is permitted:
1. The block is in a test-only context (behind `#[cfg(test)]` or in `tests/`)
2. The blocking call is a known-safe `executor::block_on` on a test fixture
3. The reason clearly explains why non-blocking is impractical here

Do not use `long-block-exception` in production code paths.
